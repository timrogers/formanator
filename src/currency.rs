use std::str::FromStr;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use chrono::{NaiveDate, Utc};
use rust_decimal::{Decimal, RoundingStrategy};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ExchangeRate {
    date: String,
    base: String,
    quote: String,
    #[serde(with = "rust_decimal::serde::float")]
    rate: Decimal,
}

pub fn normalize_currency(currency: &str) -> Result<String> {
    let code = currency.trim().to_ascii_uppercase();
    if code.len() != 3 || !code.bytes().all(|b| b.is_ascii_uppercase()) {
        bail!("Currency must be a three-letter ISO 4217 code, e.g. PLN or EUR.");
    }
    Ok(code)
}

fn decimal_places(currency: &str) -> u32 {
    match currency {
        "BIF" | "CLP" | "DJF" | "GNF" | "ISK" | "JPY" | "KMF" | "KRW" | "PYG" | "RWF" | "UGX"
        | "UYI" | "VND" | "VUV" | "XAF" | "XOF" | "XPF" => 0,
        "BHD" | "IQD" | "JOD" | "KWD" | "LYD" | "OMR" | "TND" => 3,
        "CLF" | "UYW" => 4,
        _ => 2,
    }
}

pub fn validate_amount(amount: &str, currency: &str) -> Result<Decimal> {
    let (whole, fraction) = amount.split_once('.').unwrap_or((amount, ""));
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !fraction.bytes().all(|b| b.is_ascii_digit())
        || (amount.contains('.') && fraction.is_empty())
        || fraction.trim_end_matches('0').len() > decimal_places(currency) as usize
    {
        bail!("Amount must be a non-negative number with valid decimal places for {currency}.");
    }
    Decimal::from_str(amount).context("Amount is too large or invalid")
}

pub fn convert_claim_amount(
    amount: &str,
    source: &str,
    target: &str,
    purchase_date: &str,
    description: &str,
) -> Result<(String, String)> {
    let source = normalize_currency(source)?;
    let target = normalize_currency(target)?;
    let original = validate_amount(amount, &source)?;
    let precision = decimal_places(&target) as usize;
    if source == target {
        return Ok((format!("{original:.precision$}"), description.to_string()));
    }
    let date = NaiveDate::parse_from_str(purchase_date, "%Y-%m-%d")
        .context("Currency conversion requires a valid purchase date in YYYY-MM-DD format")?;
    if date > Utc::now().date_naive() {
        bail!("Cannot convert currency for a future purchase date.");
    }

    let base = std::env::var("FORMANATOR_EXCHANGE_RATE_API_BASE")
        .unwrap_or_else(|_| "https://api.frankfurter.dev".to_string());
    let rate: ExchangeRate = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()?
        .get(format!("{base}/v2/rate/{source}/{target}"))
        .query(&[("date", purchase_date)])
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .with_context(|| {
            format!("Could not fetch {source}/{target} exchange rate for {purchase_date}; claim not submitted")
        })?
        .json()
        .context("Invalid exchange rate response; claim not submitted")?;

    apply_rate(original, &source, &target, date, description, &rate)
}

fn apply_rate(
    amount: Decimal,
    source: &str,
    target: &str,
    purchase_date: NaiveDate,
    description: &str,
    rate: &ExchangeRate,
) -> Result<(String, String)> {
    let rate_date = NaiveDate::parse_from_str(&rate.date, "%Y-%m-%d")
        .context("Exchange rate has an invalid date")?;
    if rate.base != source
        || rate.quote != target
        || rate.rate <= Decimal::ZERO
        || rate_date > purchase_date
        || (purchase_date - rate_date).num_days() > 7
    {
        bail!("Exchange rate has an unexpected currency, date, or value; claim not submitted.");
    }
    let precision = decimal_places(target) as usize;
    let converted = amount
        .checked_mul(rate.rate)
        .context("Converted amount is too large")?
        .round_dp_with_strategy(precision as u32, RoundingStrategy::MidpointAwayFromZero);
    if amount > Decimal::ZERO && converted.is_zero() {
        bail!("Converted amount rounds to zero; claim not submitted.");
    }
    let converted = format!("{converted:.precision$}");
    let description = format!(
        "{description}\nCurrency conversion: {amount} {source} -> {converted} {target}; \
         purchase date {purchase_date}; 1 {source} = {} {target} \
         (Frankfurter, rate date {}).",
        rate.rate, rate.date
    );
    Ok((converted, description))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_with_decimal_rounding_and_discloses_original_amount_and_dates() {
        let date = NaiveDate::from_ymd_opt(2026, 9, 27).unwrap();
        let rate = ExchangeRate {
            date: "2026-09-25".into(),
            base: "PLN".into(),
            quote: "EUR".into(),
            rate: Decimal::from_str("0.2352941176").unwrap(),
        };
        let (amount, note) = apply_rate(
            Decimal::from_str("4.25").unwrap(),
            "PLN",
            "EUR",
            date,
            "Book",
            &rate,
        )
        .unwrap();
        assert_eq!(amount, "1.00");
        assert!(note.starts_with("Book\nCurrency conversion: 4.25 PLN -> 1.00 EUR"));
        assert!(note.contains("purchase date 2026-09-27"));
        assert!(note.contains("Frankfurter, rate date 2026-09-25"));

        for (target, value, expected) in [
            ("EUR", "1.005", "1.01"),
            ("JPY", "150.5", "151"),
            ("KWD", "0.3005", "0.301"),
        ] {
            let rate = ExchangeRate {
                quote: target.into(),
                rate: Decimal::from_str(value).unwrap(),
                date: rate.date.clone(),
                base: rate.base.clone(),
            };
            assert_eq!(
                apply_rate(Decimal::ONE, "PLN", target, date, "Book", &rate)
                    .unwrap()
                    .0,
                expected
            );
        }
    }

    #[test]
    fn rejects_bad_rates_and_dates() {
        let date = NaiveDate::from_ymd_opt(2026, 9, 27).unwrap();
        for (base, quote, rate_date, value) in [
            ("EUR", "PLN", "2026-09-25", "4.25"),
            ("PLN", "EUR", "2026-09-28", "0.23"),
            ("PLN", "EUR", "2026-09-01", "0.23"),
            ("PLN", "EUR", "invalid", "0.23"),
            ("PLN", "EUR", "2026-09-25", "0"),
            ("PLN", "EUR", "2026-09-25", "-1"),
            ("PLN", "EUR", "2026-09-25", "0.00001"),
        ] {
            let rate = ExchangeRate {
                base: base.into(),
                quote: quote.into(),
                date: rate_date.into(),
                rate: Decimal::from_str(value).unwrap(),
            };
            assert!(apply_rate(Decimal::ONE, "PLN", "EUR", date, "Book", &rate).is_err());
        }
    }

    #[test]
    fn same_currency_needs_no_rate_and_keeps_description() {
        assert_eq!(
            convert_claim_amount("4.2", "eur", "EUR", "2026-09-25", "Book").unwrap(),
            ("4.20".into(), "Book".into())
        );
        for code in ["", "€", "EU", "EURO", "../EUR"] {
            assert!(normalize_currency(code).is_err());
        }
        for amount in ["-1", "NaN", "1e3", "1.234", "1.", ".25"] {
            assert!(validate_amount(amount, "EUR").is_err());
        }
        assert!(validate_amount("150.00", "JPY").is_ok());
        assert!(validate_amount("150.5", "JPY").is_err());
        assert!(validate_amount("1.234", "KWD").is_ok());
        assert!(convert_claim_amount("4.25", "PLN", "EUR", "2026-02-30", "Book").is_err());
        assert!(convert_claim_amount("4.25", "PLN", "EUR", "9999-01-01", "Book").is_err());
    }
}
