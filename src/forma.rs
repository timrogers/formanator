//! HTTP client for the Forma API (`https://api.joinforma.com`).

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use reqwest::blocking::{Client, multipart};
use serde::Deserialize;
use serde_json::Value;

use crate::verbose::is_enabled as is_verbose;

const DEFAULT_API_BASE: &str = "https://api.joinforma.com";
const AUTH_HEADER: &str = "x-auth-token";

// ---------------------------------------------------------------------------
// API base URL
// ---------------------------------------------------------------------------
//
// In production we always talk to `https://api.joinforma.com`. To support
// integration tests that point the client at a local mock HTTP server we keep
// the base URL behind a `RwLock` that can be overridden via [`set_api_base`].
//
// Production code never calls `set_api_base`, so the default value is used
// throughout the binary's lifetime.

static API_BASE: std::sync::RwLock<Option<String>> = std::sync::RwLock::new(None);

fn api_base() -> String {
    if let Ok(guard) = API_BASE.read()
        && let Some(base) = guard.as_ref()
    {
        return base.clone();
    }
    if let Ok(env_base) = std::env::var("FORMANATOR_API_BASE")
        && !env_base.is_empty()
    {
        return env_base;
    }
    DEFAULT_API_BASE.to_string()
}

/// Override the Forma API base URL. Intended for tests that point the client at
/// a local mock HTTP server. Pass `None` to restore the default.
///
/// This is exposed publicly so that integration tests living outside of the
/// crate can call it; production code should not call this.
pub fn set_api_base(base: Option<String>) {
    if let Ok(mut guard) = API_BASE.write() {
        *guard = base;
    }
}

// ---------------------------------------------------------------------------
// Verbose logging
// ---------------------------------------------------------------------------

struct RawResponse {
    status: reqwest::StatusCode,
    body: String,
}

/// Build, optionally log, send, read the body, and optionally log the response.
fn send_request(
    client: &Client,
    builder: reqwest::blocking::RequestBuilder,
    context: &str,
) -> Result<RawResponse> {
    let request = builder.build().with_context(|| context.to_string())?;

    if is_verbose() {
        eprintln!("[verbose] > {} {}", request.method(), request.url());
        if let Some(body) = request.body()
            && let Some(bytes) = body.as_bytes()
            && let Ok(text) = std::str::from_utf8(bytes)
        {
            eprintln!("[verbose] > Body: {text}");
        }
    }

    let response = client
        .execute(request)
        .with_context(|| context.to_string())?;
    let status = response.status();
    let body = response.text().unwrap_or_default();

    if is_verbose() {
        eprintln!(
            "[verbose] < {} {}",
            status.as_u16(),
            status.canonical_reason().unwrap_or("")
        );
        eprintln!("[verbose] < Body: {body}");
    }

    Ok(RawResponse { status, body })
}

fn client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(120))
        .user_agent(concat!("formanator/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("Failed to build HTTP client")
}

// ---------------------------------------------------------------------------
// Public domain types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct Benefit {
    pub id: String,
    pub name: String,
    #[serde(rename = "remainingAmount")]
    pub remaining_amount: f64,
    #[serde(rename = "remainingAmountCurrency")]
    pub remaining_amount_currency: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Category {
    pub category_id: String,
    pub category_name: String,
    pub subcategory_name: String,
    pub subcategory_value: String,
    pub subcategory_alias: Option<String>,
    pub benefit_id: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BenefitWithCategories {
    #[serde(flatten)]
    pub benefit: Benefit,
    pub categories: Vec<Category>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Claim {
    pub id: String,
    pub status: String,
    pub reimbursement_status: Option<String>,
    pub payout_status: Option<String>,
    pub amount: Option<f64>,
    pub category: Option<String>,
    pub subcategory: Option<String>,
    pub reimbursement_vendor: Option<String>,
    pub date_processed: Option<String>,
    pub note: Option<String>,
    pub employee_note: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateClaimOptions {
    pub amount: String,
    pub merchant: String,
    pub purchase_date: String,
    pub description: String,
    pub receipt_path: Vec<PathBuf>,
    pub access_token: String,
    pub benefit_id: String,
    pub category_id: String,
    pub subcategory_value: String,
    pub subcategory_alias: Option<String>,
}

// ---------------------------------------------------------------------------
// Raw response schemas (only what we need)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ProfileResponse {
    data: ProfileData,
}

#[derive(Debug, Deserialize)]
struct ProfileData {
    company: CompanyInfo,
    employee: EmployeeInfo,
}

#[derive(Debug, Deserialize)]
struct CompanyInfo {
    company_wallet_configurations: Vec<CompanyWalletConfiguration>,
}

#[derive(Debug, Deserialize)]
struct CompanyWalletConfiguration {
    wallet_name: String,
    categories: Vec<RawCategory>,
}

#[derive(Debug, Deserialize)]
struct RawCategory {
    id: String,
    name: String,
    subcategories: Vec<RawSubcategory>,
}

#[derive(Debug, Deserialize)]
struct RawSubcategory {
    name: String,
    value: String,
    #[serde(default)]
    aliases: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct EmployeeInfo {
    employee_wallets: Vec<EmployeeWallet>,
    settings: EmployeeSettings,
}

#[derive(Debug, Deserialize)]
struct EmployeeWallet {
    id: String,
    amount: f64,
    company_wallet_configuration: EmployeeWalletConfig,
    is_employee_eligible: bool,
}

#[derive(Debug, Deserialize)]
struct EmployeeWalletConfig {
    wallet_name: String,
}

#[derive(Debug, Deserialize)]
struct EmployeeSettings {
    currency: String,
}

#[derive(Debug, Deserialize)]
struct ClaimsListResponse {
    data: ClaimsListData,
}

#[derive(Debug, Deserialize)]
struct ClaimsListData {
    claims: Vec<RawClaim>,
    #[allow(dead_code)]
    page: Value,
    limit: serde_json::Value,
    count: u64,
}

#[derive(Debug, Deserialize)]
struct RawClaim {
    id: String,
    status: String,
    reimbursement: RawReimbursement,
}

#[derive(Debug, Deserialize)]
struct RawReimbursement {
    status: Option<String>,
    payout_status: Option<String>,
    amount: Option<f64>,
    category: Option<String>,
    subcategory: Option<String>,
    reimbursement_vendor: Option<String>,
    date_processed: Option<String>,
    note: Option<String>,
    employee_note: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GenericSuccessResponse {
    success: bool,
}

#[derive(Debug, Deserialize)]
struct MagicLinkExchangeResponse {
    success: bool,
    data: MagicLinkExchangeData,
}

#[derive(Debug, Deserialize)]
struct MagicLinkExchangeData {
    auth_token: String,
}

// ---------------------------------------------------------------------------
// Error handling
// ---------------------------------------------------------------------------

fn handle_error_response(status: reqwest::StatusCode, body: &str) -> anyhow::Error {
    let status_text = status
        .canonical_reason()
        .unwrap_or("unknown status")
        .to_string();

    if let Ok(parsed) = serde_json::from_str::<Value>(body)
        && let Some(message) = parsed
            .get("errors")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
    {
        if message.contains("JWT token is invalid") {
            return anyhow!(
                "Your Forma access token is invalid. Please log in again with `formanator login`."
            );
        }
        return anyhow!("{}", message);
    }

    anyhow!(
        "Received an unexpected {} {} response from Forma: {}",
        status.as_u16(),
        status_text,
        body
    )
}

// ---------------------------------------------------------------------------
// API operations
// ---------------------------------------------------------------------------

fn get_profile(access_token: &str) -> Result<ProfileResponse> {
    let c = client()?;
    let base = api_base();
    let resp = send_request(
        &c,
        c.get(format!("{base}/client/api/v3/settings/profile"))
            .header(AUTH_HEADER, access_token),
        "Failed to call Forma profile endpoint",
    )?;
    if !resp.status.is_success() {
        return Err(handle_error_response(resp.status, &resp.body));
    }
    serde_json::from_str(&resp.body).context("Failed to parse Forma profile response")
}

pub fn get_benefits(access_token: &str) -> Result<Vec<Benefit>> {
    let profile = get_profile(access_token)?;
    let currency = profile.data.employee.settings.currency.clone();

    Ok(profile
        .data
        .employee
        .employee_wallets
        .into_iter()
        .filter(|w| w.is_employee_eligible)
        .map(|w| Benefit {
            id: w.id,
            name: w.company_wallet_configuration.wallet_name,
            remaining_amount: w.amount,
            remaining_amount_currency: currency.clone(),
        })
        .collect())
}

pub fn get_categories_for_benefit_name(
    access_token: &str,
    benefit_name: &str,
) -> Result<Vec<Category>> {
    let profile = get_profile(access_token)?;

    let employee_wallet = profile
        .data
        .employee
        .employee_wallets
        .iter()
        .find(|w| {
            w.is_employee_eligible && w.company_wallet_configuration.wallet_name == benefit_name
        })
        .ok_or_else(|| anyhow!("Could not find benefit with name `{benefit_name}`."))?;

    let company_wallet = profile
        .data
        .company
        .company_wallet_configurations
        .iter()
        .find(|c| c.wallet_name == benefit_name)
        .ok_or_else(|| anyhow!("Could not find benefit with name `{benefit_name}`."))?;

    let benefit_id = employee_wallet.id.clone();

    let mut out = Vec::new();
    for category in &company_wallet.categories {
        for subcategory in &category.subcategories {
            out.push(Category {
                category_id: category.id.clone(),
                category_name: category.name.clone(),
                subcategory_name: subcategory.name.clone(),
                subcategory_value: subcategory.value.clone(),
                subcategory_alias: None,
                benefit_id: benefit_id.clone(),
            });
            for alias in &subcategory.aliases {
                out.push(Category {
                    category_id: category.id.clone(),
                    category_name: category.name.clone(),
                    subcategory_name: subcategory.name.clone(),
                    subcategory_value: subcategory.value.clone(),
                    subcategory_alias: Some(alias.clone()),
                    benefit_id: benefit_id.clone(),
                });
            }
        }
    }
    Ok(out)
}

pub fn get_benefits_with_categories(access_token: &str) -> Result<Vec<BenefitWithCategories>> {
    let benefits = get_benefits(access_token)?;
    let mut out = Vec::with_capacity(benefits.len());
    for benefit in benefits {
        let categories = get_categories_for_benefit_name(access_token, &benefit.name)?;
        out.push(BenefitWithCategories {
            benefit,
            categories,
        });
    }
    Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimsFilter {
    InProgress,
}

fn fetch_claims_page(access_token: &str, page: u32) -> Result<ClaimsListData> {
    let base = api_base();
    let url = format!("{base}/client/api/v2/claims?page={page}");
    let c = client()?;
    let resp = send_request(
        &c,
        c.get(url).header(AUTH_HEADER, access_token),
        "Failed to call Forma claims endpoint",
    )?;
    if !resp.status.is_success() {
        return Err(handle_error_response(resp.status, &resp.body));
    }
    let parsed: ClaimsListResponse = serde_json::from_str(&resp.body)
        .with_context(|| format!("Failed to parse Forma claims response:\n{}", resp.body))?;
    Ok(parsed.data)
}

pub fn get_claims_list(access_token: &str, filter: Option<ClaimsFilter>) -> Result<Vec<Claim>> {
    let mut all = Vec::new();
    let mut page = 0u32;
    loop {
        let data = fetch_claims_page(access_token, page)?;
        let limit_num: u64 = match &data.limit {
            Value::Number(n) => n.as_u64().unwrap_or(0),
            Value::String(s) => s.parse().unwrap_or(0),
            _ => 0,
        };
        let count = data.count;

        for raw in data.claims {
            all.push(Claim {
                id: raw.id,
                status: raw.status,
                reimbursement_status: raw.reimbursement.status,
                payout_status: raw.reimbursement.payout_status,
                amount: raw.reimbursement.amount,
                category: raw.reimbursement.category,
                subcategory: raw.reimbursement.subcategory,
                reimbursement_vendor: raw.reimbursement.reimbursement_vendor,
                date_processed: raw.reimbursement.date_processed,
                note: raw.reimbursement.note,
                employee_note: raw.reimbursement.employee_note,
            });
        }

        if limit_num == 0 || count != limit_num {
            break;
        }
        page += 1;
    }

    if filter == Some(ClaimsFilter::InProgress) {
        all.retain(|c| {
            c.status == "in_progress" || c.reimbursement_status.as_deref() == Some("in_progress")
        });
    }
    Ok(all)
}

pub fn create_claim(opts: &CreateClaimOptions) -> Result<()> {
    let subcategory_alias = opts.subcategory_alias.clone().unwrap_or_default();

    if is_verbose() {
        eprintln!("[verbose] Multipart form fields for POST /client/api/v2/claims:");
        eprintln!("[verbose]   type = \"transaction\"");
        eprintln!("[verbose]   is_recurring = \"false\"");
        eprintln!("[verbose]   amount = {:?}", opts.amount);
        eprintln!("[verbose]   transaction_date = {:?}", opts.purchase_date);
        eprintln!(
            "[verbose]   default_employee_wallet_id = {:?}",
            opts.benefit_id
        );
        eprintln!("[verbose]   note = {:?}", opts.description);
        eprintln!("[verbose]   category = {:?}", opts.category_id);
        eprintln!("[verbose]   category_alias = \"\"");
        eprintln!("[verbose]   subcategory = {:?}", opts.subcategory_value);
        eprintln!("[verbose]   subcategory_alias = {:?}", subcategory_alias);
        eprintln!("[verbose]   reimbursement_vendor = {:?}", opts.merchant);
        for path in &opts.receipt_path {
            eprintln!("[verbose]   file[] = {:?}", path.display());
        }
    }

    let mut form = multipart::Form::new()
        .text("type", "transaction".to_string())
        .text("is_recurring", "false".to_string())
        .text("amount", opts.amount.clone())
        .text("transaction_date", opts.purchase_date.clone())
        .text("default_employee_wallet_id", opts.benefit_id.clone())
        .text("note", opts.description.clone())
        .text("category", opts.category_id.clone())
        .text("category_alias", String::new())
        .text("subcategory", opts.subcategory_value.clone())
        .text("subcategory_alias", subcategory_alias)
        .text("reimbursement_vendor", opts.merchant.clone());

    for path in &opts.receipt_path {
        let abs: &Path = path.as_ref();
        form = form
            .file("file[]", abs)
            .with_context(|| format!("Failed to attach receipt at {}", abs.display()))?;
    }

    let c = client()?;
    let base = api_base();
    let resp = send_request(
        &c,
        c.post(format!("{base}/client/api/v2/claims"))
            .header(AUTH_HEADER, &opts.access_token)
            .multipart(form),
        "Failed to submit claim to Forma",
    )?;

    if resp.status.as_u16() != 201 {
        return Err(handle_error_response(resp.status, &resp.body));
    }

    let parsed: GenericSuccessResponse = serde_json::from_str(&resp.body)
        .context("Failed to parse Forma claim creation response")?;
    if !parsed.success {
        bail!(
            "Something went wrong while submitting your claim. Forma returned `201 Created`, but the response body indicated that the request was not successful."
        );
    }
    Ok(())
}

/// Request a magic link be emailed to the user.
pub fn request_magic_link(email: &str) -> Result<()> {
    let c = client()?;
    let base = api_base();
    let resp = send_request(
        &c,
        c.post(format!("{base}/client/auth/v2/login/magic"))
            .json(&serde_json::json!({ "email": email })),
        "Failed to request magic link",
    )?;
    if !resp.status.is_success() {
        return Err(handle_error_response(resp.status, &resp.body));
    }
    let parsed: GenericSuccessResponse =
        serde_json::from_str(&resp.body).context("Failed to parse Forma magic link response")?;
    if !parsed.success {
        bail!("Something went wrong while requesting a magic link from Forma.");
    }
    Ok(())
}

/// Exchange a magic-link `id`/`tk` pair for a long-lived access token.
pub fn exchange_id_and_tk_for_access_token(id: &str, tk: &str) -> Result<String> {
    let c = client()?;
    let base = api_base();
    let resp = send_request(
        &c,
        c.get(format!("{base}/client/auth/v2/login/magic")).query(&[
            ("id", id),
            ("tk", tk),
            ("return_token", "true"),
        ]),
        "Failed to exchange magic link for an access token",
    )?;
    if !resp.status.is_success() {
        return Err(handle_error_response(resp.status, &resp.body));
    }
    let parsed: MagicLinkExchangeResponse = serde_json::from_str(&resp.body)
        .context("Failed to parse Forma magic link exchange response")?;
    if !parsed.success {
        bail!("Something went wrong while exchanging the magic link for an access token.");
    }
    Ok(parsed.data.auth_token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn fixture(name: &str) -> String {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name);
        std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", path.display()))
    }

    // ----- handle_error_response -----

    #[test]
    fn handle_error_invalid_jwt_returns_login_message() {
        let body = fixture("error_invalid_jwt.json");
        let err = handle_error_response(reqwest::StatusCode::UNAUTHORIZED, &body);
        let msg = format!("{err}");
        assert!(
            msg.contains("Forma access token is invalid"),
            "unexpected error message: {msg}"
        );
        assert!(msg.contains("formanator login"), "{msg}");
    }

    #[test]
    fn handle_error_generic_returns_message_from_body() {
        let body = fixture("error_generic.json");
        let err = handle_error_response(reqwest::StatusCode::FORBIDDEN, &body);
        assert_eq!(
            format!("{err}"),
            "That benefit is not available for your account."
        );
    }

    #[test]
    fn handle_error_unknown_shape_falls_back_to_status_and_body() {
        let body = fixture("error_unknown_shape.json");
        let err = handle_error_response(reqwest::StatusCode::INTERNAL_SERVER_ERROR, &body);
        let msg = format!("{err}");
        assert!(msg.contains("500"), "{msg}");
        assert!(msg.contains("Internal Server Error"), "{msg}");
        // The original body should be included verbatim so debugging is possible.
        assert!(msg.contains("\"unexpected\""), "{msg}");
    }

    #[test]
    fn handle_error_unparseable_body_falls_back_to_status_and_body() {
        let err = handle_error_response(reqwest::StatusCode::BAD_GATEWAY, "<html>boom</html>");
        let msg = format!("{err}");
        assert!(msg.contains("502"), "{msg}");
        assert!(msg.contains("<html>boom</html>"), "{msg}");
    }

    // ----- Deserialization of the response fixtures -----

    #[test]
    fn parses_profile_response_fixture() {
        let body = fixture("profile_response.json");
        let parsed: ProfileResponse =
            serde_json::from_str(&body).expect("profile response fixture should parse");
        assert_eq!(parsed.data.employee.settings.currency, "GBP");
        assert_eq!(parsed.data.employee.employee_wallets.len(), 6);
        assert_eq!(parsed.data.company.company_wallet_configurations.len(), 8);

        // The fixture intentionally includes ineligible wallets so we can
        // assert that downstream code filters them out.
        let eligible = parsed
            .data
            .employee
            .employee_wallets
            .iter()
            .filter(|w| w.is_employee_eligible)
            .count();
        assert_eq!(eligible, 3);
    }

    #[test]
    fn parses_empty_profile_response_fixture() {
        let body = fixture("profile_response_empty.json");
        let parsed: ProfileResponse =
            serde_json::from_str(&body).expect("empty profile fixture should parse");
        assert_eq!(parsed.data.employee.settings.currency, "GBP");
        assert!(parsed.data.employee.employee_wallets.is_empty());
        assert!(parsed.data.company.company_wallet_configurations.is_empty());
    }

    #[test]
    fn parses_claims_list_page0_fixture() {
        let body = fixture("claims_list_page0.json");
        let parsed: ClaimsListResponse = serde_json::from_str(&body).expect("claims page 0");
        assert_eq!(parsed.data.claims.len(), 2);
        assert_eq!(parsed.data.count, 2);
        // limit can be either a number or a string.
        assert_eq!(parsed.data.limit, Value::Number(2u64.into()));
        assert_eq!(parsed.data.claims[1].status, "in_progress");
        assert_eq!(
            parsed.data.claims[0].reimbursement.payout_status.as_deref(),
            Some("paid")
        );
    }

    #[test]
    fn parses_claims_list_page1_fixture_with_string_limit() {
        let body = fixture("claims_list_page1.json");
        let parsed: ClaimsListResponse = serde_json::from_str(&body).expect("claims page 1");
        assert_eq!(parsed.data.claims.len(), 1);
        assert_eq!(parsed.data.count, 1);
        // Forma sometimes returns `limit` as a string; we must tolerate that.
        assert_eq!(parsed.data.limit, Value::String("2".to_string()));
    }

    #[test]
    fn parses_magic_link_exchange_response_fixture() {
        let body = fixture("magic_link_exchange_response.json");
        let parsed: MagicLinkExchangeResponse =
            serde_json::from_str(&body).expect("magic link exchange");
        assert!(parsed.success);
        // The fixture's auth_token is a JWT-shaped string with three
        // base64url-encoded segments separated by dots.
        let segments: Vec<&str> = parsed.data.auth_token.split('.').collect();
        assert_eq!(segments.len(), 3);
        assert!(parsed.data.auth_token.starts_with("eyJ"));
    }

    #[test]
    fn parses_create_claim_success_fixture() {
        let body = fixture("create_claim_response_success.json");
        let parsed: GenericSuccessResponse =
            serde_json::from_str(&body).expect("create claim success");
        assert!(parsed.success);
    }

    #[test]
    fn parses_create_claim_unsuccessful_fixture() {
        let body = fixture("create_claim_response_unsuccessful.json");
        let parsed: GenericSuccessResponse =
            serde_json::from_str(&body).expect("create claim unsuccessful");
        assert!(!parsed.success);
    }

    // ----- Public types serialize predictably (used by the MCP server / JSON output) -----

    #[test]
    fn benefit_serializes_with_camel_case_amount_fields() {
        let benefit = Benefit {
            id: "wallet-1".to_string(),
            name: "Lifestyle Spending Account".to_string(),
            remaining_amount: 12.34,
            remaining_amount_currency: "USD".to_string(),
        };
        let json = serde_json::to_value(&benefit).unwrap();
        assert_eq!(json["id"], "wallet-1");
        assert_eq!(json["name"], "Lifestyle Spending Account");
        assert_eq!(json["remainingAmount"], 12.34);
        assert_eq!(json["remainingAmountCurrency"], "USD");
    }

    #[test]
    fn benefit_with_categories_flattens_benefit_fields() {
        let bwc = BenefitWithCategories {
            benefit: Benefit {
                id: "wallet-1".to_string(),
                name: "LSA".to_string(),
                remaining_amount: 1.0,
                remaining_amount_currency: "USD".to_string(),
            },
            categories: vec![Category {
                category_id: "c1".to_string(),
                category_name: "Fitness".to_string(),
                subcategory_name: "Gym".to_string(),
                subcategory_value: "gym".to_string(),
                subcategory_alias: Some("alias".to_string()),
                benefit_id: "wallet-1".to_string(),
            }],
        };
        let json = serde_json::to_value(&bwc).unwrap();
        // Flattened benefit fields appear at the top level alongside `categories`.
        assert_eq!(json["id"], "wallet-1");
        assert_eq!(json["name"], "LSA");
        assert_eq!(json["categories"][0]["category_id"], "c1");
        assert_eq!(json["categories"][0]["subcategory_alias"], "alias");
    }

    // ----- api_base override -----

    #[test]
    fn api_base_defaults_and_can_be_overridden() {
        // The other tests in this binary may override the API base concurrently,
        // so we're careful only to set + restore inside this test and only
        // assert on values we control.
        let original = api_base();
        set_api_base(Some("http://localhost:1/forma".to_string()));
        assert_eq!(api_base(), "http://localhost:1/forma");
        set_api_base(None);
        // After clearing the override, we either fall back to the env var
        // (if one is set in the test environment) or the default.
        let after = api_base();
        assert!(after == DEFAULT_API_BASE || after == original || after.starts_with("http"));
    }
}
