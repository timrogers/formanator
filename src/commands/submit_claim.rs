use anyhow::{Result, bail};
use colored::Colorize;

use crate::claims::{ClaimInput, claim_input_to_create_options};
use crate::cli::SubmitClaimArgs;
use crate::config::resolve_access_token;
use crate::forma::{create_claim, get_benefits_with_categories};
use crate::llm::{infer_all_from_receipt, infer_category_and_benefit};
use crate::prompt::prompt;
use crate::verbose;

pub fn run(args: SubmitClaimArgs) -> Result<()> {
    verbose::set(args.verbose);
    let access_token = resolve_access_token(args.access_token.as_deref())?;

    let SubmitClaimArgs {
        benefit,
        category,
        amount,
        merchant,
        purchase_date,
        description,
        receipt_path,
        openai_api_key,
        github_token,
        dry_run,
        verbose: _,
        ..
    } = args;

    if receipt_path.is_empty() {
        bail!("You must specify at least one --receipt-path.");
    }

    let has_all_manual = if let (
        Some(benefit),
        Some(category),
        Some(amount),
        Some(merchant),
        Some(purchase_date),
        Some(description),
    ) = (
        benefit.clone(),
        category.clone(),
        amount.clone(),
        merchant.clone(),
        purchase_date.clone(),
        description.clone(),
    ) {
        Some(ClaimInput {
            benefit,
            category,
            amount,
            merchant,
            purchase_date,
            description,
            receipt_path: receipt_path.clone(),
        })
    } else {
        None
    };
    let has_some_manual = benefit.is_some()
        || category.is_some()
        || amount.is_some()
        || merchant.is_some()
        || purchase_date.is_some()
        || description.is_some();
    let has_llm_key = openai_api_key.is_some() || github_token.is_some();

    if let Some(claim) = has_all_manual {
        let opts = claim_input_to_create_options(&claim, &access_token)?;
        if dry_run {
            println!("{}", "Dry run: skipping claim submission.".yellow());
        } else {
            create_claim(&opts)?;
        }
    } else if !has_some_manual && has_llm_key {
        // Full receipt inference mode
        let benefits = get_benefits_with_categories(&access_token)?;
        let inferred = infer_all_from_receipt(
            &receipt_path[0],
            &benefits,
            openai_api_key.as_deref(),
            github_token.as_deref(),
        )?;

        println!(
            "{}",
            "The LLM inferred the following details from your receipt:".cyan()
        );
        println!("Amount: {}", inferred.amount.magenta());
        println!("Merchant: {}", inferred.merchant.magenta());
        println!("Purchase Date: {}", inferred.purchase_date.magenta());
        println!("Description: {}", inferred.description.magenta());
        println!("Benefit: {}", inferred.benefit.magenta());
        println!("Category: {}", inferred.category.magenta());
        println!();
        println!(
            "If these details look correct, hit Enter to proceed. If not, press Ctrl+C to end your session."
        );
        let _ = prompt("> ")?;

        let claim = ClaimInput {
            benefit: inferred.benefit,
            category: inferred.category,
            amount: inferred.amount,
            merchant: inferred.merchant,
            purchase_date: inferred.purchase_date,
            description: inferred.description,
            receipt_path,
        };
        let opts = claim_input_to_create_options(&claim, &access_token)?;
        if dry_run {
            println!("{}", "Dry run: skipping claim submission.".yellow());
        } else {
            create_claim(&opts)?;
        }
    } else if has_llm_key
        && let (Some(merchant), Some(description), Some(amount), Some(purchase_date)) = (
            merchant.clone(),
            description.clone(),
            amount.clone(),
            purchase_date.clone(),
        )
    {
        // Legacy mode: infer benefit and category only.
        let benefits = get_benefits_with_categories(&access_token)?;
        let inferred = infer_category_and_benefit(
            &merchant,
            &description,
            &benefits,
            openai_api_key.as_deref(),
            github_token.as_deref(),
        )?;

        println!(
            "The LLM inferred that you should claim using the {} benefit and {} category. If that seems right, hit Enter. If not, press Ctrl+C to end your session.",
            inferred.benefit.magenta(),
            inferred.category.magenta(),
        );
        let _ = prompt("> ")?;

        let claim = ClaimInput {
            benefit: inferred.benefit,
            category: inferred.category,
            amount,
            merchant,
            purchase_date,
            description,
            receipt_path,
        };
        let opts = claim_input_to_create_options(&claim, &access_token)?;
        if dry_run {
            println!("{}", "Dry run: skipping claim submission.".yellow());
        } else {
            create_claim(&opts)?;
        }
    } else {
        bail!(
            "You must either provide all claim details (--benefit, --category, --amount, --merchant, --purchase-date, --description), or provide an OpenAI API key or GitHub token with either: (1) just a receipt for full inference, or (2) all details except --benefit and --category to infer them."
        );
    }

    println!("{}", "Claim submitted successfully ✅".green());
    Ok(())
}
