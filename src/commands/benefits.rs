use anyhow::Result;
use colored::Colorize;
use tabled::{Table, Tabled, settings::Style};

use crate::cli::BenefitsArgs;
use crate::config::resolve_access_token;
use crate::forma::get_benefits;
use crate::verbose;

#[derive(Tabled)]
struct Row {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Remaining Amount")]
    remaining: String,
}

pub fn run(args: BenefitsArgs) -> Result<()> {
    verbose::set(args.verbose);
    let access_token = resolve_access_token(args.access_token.as_deref())?;
    let benefits = get_benefits(&access_token)?;

    if benefits.is_empty() {
        println!("{}", "No benefits found.".yellow());
        return Ok(());
    }

    let rows: Vec<Row> = benefits
        .into_iter()
        .map(|b| Row {
            name: b.name,
            remaining: format!("{} {}", b.remaining_amount, b.remaining_amount_currency),
        })
        .collect();

    let mut table = Table::new(rows);
    table.with(Style::sharp());
    println!("{table}");
    Ok(())
}
