use anyhow::{Result, bail};
use colored::Colorize;

use crate::claims::{ClaimInput, claim_input_to_create_options, read_claims_from_csv};
use crate::cli::ValidateCsvArgs;
use crate::config::resolve_access_token;
use crate::forma::get_benefits_with_categories;
use crate::verbose;

pub fn run(args: ValidateCsvArgs) -> Result<()> {
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
    if benefits.is_empty() {
        bail!("Your account does not have any benefits, so claims cannot be validated.");
    }

    let total = claims.len();
    for (index, claim) in claims.into_iter().enumerate() {
        let row_number = index + 2;
        println!(
            "Validating claim {}/{} (row {row_number})",
            index + 1,
            total
        );

        let result = (|| -> Result<()> {
            let benefit_category_empty = claim.benefit.is_empty() && claim.category.is_empty();
            let other_details_empty = claim.amount.is_empty()
                && claim.merchant.is_empty()
                && claim.purchase_date.is_empty()
                && claim.description.is_empty();

            if !claim.benefit.is_empty() && !claim.category.is_empty() {
                claim_input_to_create_options(&claim, &access_token).map(|_| ())
            } else if benefit_category_empty && other_details_empty {
                if claim.receipt_path.is_empty() {
                    bail!(
                        "Row leaves every column blank except `receiptPath`, but `receiptPath` is also empty. Provide at least one receipt path so claim details can be inferred at submission time."
                    );
                }
                for path in &claim.receipt_path {
                    if !path.exists() {
                        bail!("Receipt path '{}' does not exist.", path.display());
                    }
                }
                println!(
                    "{}",
                    format!(
                        "Claim {}/{} (row {row_number}) leaves every column except `receiptPath` blank. All claim details will be inferred from the receipt at submission time.",
                        index + 1,
                        total
                    )
                    .yellow()
                );
                Ok(())
            } else {
                let placeholder = benefits
                    .iter()
                    .find(|b| !b.categories.is_empty())
                    .ok_or_else(|| anyhow::anyhow!(
                        "Your account does not have any benefits with categories, so claims with a missing benefit/category cannot be validated."
                    ))?;
                let placeholder_benefit = &placeholder.benefit.name;
                let placeholder_category = &placeholder.categories[0].subcategory_name;
                let test_claim = ClaimInput {
                    benefit: placeholder_benefit.clone(),
                    category: placeholder_category.clone(),
                    ..claim.clone()
                };
                claim_input_to_create_options(&test_claim, &access_token).map(|_| ())?;
                println!(
                    "{}",
                    format!(
                        "Claim {}/{} (row {row_number}) doesn't have a benefit and/or category. This will have to be inferred when the claims are submitted.",
                        index + 1,
                        total
                    )
                    .yellow()
                );
                Ok(())
            }
        })();

        match result {
            Ok(()) => println!(
                "{}",
                format!("Validated claim {}/{} (row {row_number})", index + 1, total).green()
            ),
            Err(e) => eprintln!(
                "{}",
                format!(
                    "Error validating claim {}/{}: {e} (row {row_number})",
                    index + 1,
                    total
                )
                .red()
            ),
        }
    }

    Ok(())
}
