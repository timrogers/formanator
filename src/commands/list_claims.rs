use anyhow::{Result, bail};
use tabled::{builder::Builder, settings::Style};

use crate::cli::ListClaimsArgs;
use crate::config::resolve_access_token;
use crate::forma::{ClaimsFilter, get_claims_list};
use crate::verbose;

pub fn run(args: ListClaimsArgs) -> Result<()> {
    verbose::set(args.verbose);
    let access_token = resolve_access_token(args.access_token.as_deref())?;

    let filter = match args.filter.as_deref() {
        None => None,
        Some("in_progress") => Some(ClaimsFilter::InProgress),
        Some(other) => {
            bail!("Invalid filter value '{other}'. Currently supported filters: in_progress")
        }
    };

    let claims = get_claims_list(&access_token, filter)?;

    let has_payout = claims.iter().any(|c| c.payout_status.is_some());

    let mut builder = Builder::default();
    let mut header = vec![
        "Reimbursement Vendor".to_string(),
        "Employee Note".to_string(),
        "Amount".to_string(),
        "Category".to_string(),
        "Subcategory".to_string(),
        "Status".to_string(),
        "Reimbursement Status".to_string(),
    ];
    if has_payout {
        header.push("Payout Status".to_string());
    }
    header.push("Date Processed".to_string());
    header.push("Note".to_string());
    builder.push_record(header);

    for claim in &claims {
        let mut row = vec![
            claim.reimbursement_vendor.clone().unwrap_or_default(),
            claim.employee_note.clone().unwrap_or_default(),
            claim.amount.map(|a| a.to_string()).unwrap_or_default(),
            claim.category.clone().unwrap_or_default(),
            claim.subcategory.clone().unwrap_or_default(),
            claim.status.clone(),
            claim.reimbursement_status.clone().unwrap_or_default(),
        ];
        if has_payout {
            row.push(claim.payout_status.clone().unwrap_or_default());
        }
        row.push(claim.date_processed.clone().unwrap_or_default());
        row.push(claim.note.clone().unwrap_or_default());
        builder.push_record(row);
    }

    let mut table = builder.build();
    table.with(Style::sharp());
    println!("{table}");
    Ok(())
}
