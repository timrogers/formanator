use anyhow::Result;
use tabled::{Table, Tabled, settings::Style};

use crate::cli::CategoriesArgs;
use crate::config::resolve_access_token;
use crate::forma::get_categories_for_benefit_name;
use crate::verbose;

#[derive(Tabled)]
struct Row {
    #[tabled(rename = "Parent Category")]
    parent: String,
    #[tabled(rename = "Category")]
    category: String,
}

pub fn run(args: CategoriesArgs) -> Result<()> {
    verbose::set(args.verbose);
    let access_token = resolve_access_token(args.access_token.as_deref())?;
    let categories = get_categories_for_benefit_name(&access_token, &args.benefit)?;

    let rows: Vec<Row> = categories
        .into_iter()
        .map(|c| Row {
            parent: c.category_name,
            category: c.subcategory_alias.unwrap_or(c.subcategory_name),
        })
        .collect();

    let mut table = Table::new(rows);
    table.with(Style::sharp());
    println!("{table}");
    Ok(())
}
