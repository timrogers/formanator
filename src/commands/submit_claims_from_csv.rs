use anyhow::{Result, bail};
use colored::Colorize;

use crate::claims::{claim_input_to_create_options, read_claims_from_csv};
use crate::cli::SubmitClaimsFromCsvArgs;
use crate::config::resolve_access_token;
use crate::forma::{create_claim, get_benefits_with_categories};
use crate::llm::{infer_all_from_receipt, infer_category_and_benefit};
use crate::verbose;

pub fn run(args: SubmitClaimsFromCsvArgs) -> Result<()> {
    verbose::set(args.verbose);
    let access_token = resolve_access_token(args.access_token.as_deref())?;

    if !args.input_path.exists() {
        bail!("File '{}' doesn't exist.", args.input_path.display());
    }

    let claims = read_claims_from_csv(&args.input_path)?;
    if claims.is_empty() {
        bail!("Your CSV doesn't seem to contain any claims. Have you filled out the template?");
    }

    let benefits = get_benefits_with_categories(&access_token)?;
    let total = claims.len();

    for (index, mut claim) in claims.into_iter().enumerate() {
        let row_number = index + 2;
        println!(
            "Submitting claim {}/{} (row {row_number})",
            index + 1,
            total
        );

        let result = (|| -> Result<()> {
            let has_llm_key = args.openai_api_key.is_some() || args.github_token.is_some();
            let benefit_category_empty = claim.benefit.is_empty() && claim.category.is_empty();
            let other_details_empty = claim.amount.is_empty()
                && claim.merchant.is_empty()
                && claim.purchase_date.is_empty()
                && claim.description.is_empty();
            let other_details_all_filled = !claim.amount.is_empty()
                && !claim.merchant.is_empty()
                && !claim.purchase_date.is_empty()
                && !claim.description.is_empty();

            if !claim.benefit.is_empty() && !claim.category.is_empty() {
                let opts = claim_input_to_create_options(&claim, &access_token)?;
                if args.dry_run {
                    println!("{}", "Dry run: skipping claim submission.".yellow());
                    Ok(())
                } else {
                    create_claim(&opts)
                }
            } else if has_llm_key && benefit_category_empty && other_details_empty {
                if claim.receipt_path.is_empty() {
                    anyhow::bail!(
                        "To infer all claim details from the receipt, you must provide at least one path in the `receiptPath` column."
                    );
                }
                let inferred = infer_all_from_receipt(
                    &claim.receipt_path[0],
                    &benefits,
                    args.openai_api_key.as_deref(),
                    args.github_token.as_deref(),
                )?;
                println!("Inferred amount: {}", inferred.amount);
                println!("Inferred merchant: {}", inferred.merchant);
                println!("Inferred purchase date: {}", inferred.purchase_date);
                println!("Inferred description: {}", inferred.description);
                println!("Inferred benefit: {}", inferred.benefit);
                println!("Inferred category: {}", inferred.category);
                claim.benefit = inferred.benefit;
                claim.category = inferred.category;
                claim.amount = inferred.amount;
                claim.merchant = inferred.merchant;
                claim.purchase_date = inferred.purchase_date;
                claim.description = inferred.description;
                let opts = claim_input_to_create_options(&claim, &access_token)?;
                if args.dry_run {
                    println!("{}", "Dry run: skipping claim submission.".yellow());
                    Ok(())
                } else {
                    create_claim(&opts)
                }
            } else if has_llm_key && benefit_category_empty && other_details_all_filled {
                let inferred = infer_category_and_benefit(
                    &claim.merchant,
                    &claim.description,
                    &benefits,
                    args.openai_api_key.as_deref(),
                    args.github_token.as_deref(),
                )?;
                claim.benefit = inferred.benefit;
                claim.category = inferred.category;
                println!("Inferred benefit: {}", claim.benefit);
                println!("Inferred category: {}", claim.category);
                let opts = claim_input_to_create_options(&claim, &access_token)?;
                if args.dry_run {
                    println!("{}", "Dry run: skipping claim submission.".yellow());
                    Ok(())
                } else {
                    create_claim(&opts)
                }
            } else if has_llm_key {
                anyhow::bail!(
                    "To use LLM inference, a row must either leave every column except `receiptPath` blank (full receipt inference), or fill every column except `benefit` and `category` (benefit/category inference only)."
                );
            } else {
                anyhow::bail!(
                    "You must either fill out the `benefit` and `category` columns, or specify an OpenAI API key or GitHub token."
                );
            }
        })();

        match result {
            Ok(()) => println!(
                "{}",
                format!(
                    "Successfully submitted claim {}/{} (row {row_number})",
                    index + 1,
                    total
                )
                .green()
            ),
            Err(e) => eprintln!(
                "{}",
                format!(
                    "Error submitting claim {}/{}: {e} (row {row_number})",
                    index + 1,
                    total
                )
                .red()
            ),
        }
    }

    Ok(())
}
