use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;

use formanator::cli::{Cli, Command};
use formanator::commands;

fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Login(args) => commands::login::run(args),
        Command::Benefits(args) => commands::benefits::run(args),
        Command::Categories(args) => commands::categories::run(args),
        Command::ListClaims(args) => commands::list_claims::run(args),
        Command::SubmitClaim(args) => commands::submit_claim::run(args),
        Command::GenerateTemplateCsv(args) => commands::generate_template_csv::run(args),
        Command::SubmitClaimsFromCsv(args) => commands::submit_claims_from_csv::run(args),
        Command::SubmitClaimsFromDirectory(args) => {
            commands::submit_claims_from_directory::run(args)
        }
        Command::ValidateCsv(args) => commands::validate_csv::run(args),
        #[cfg(feature = "mcp")]
        Command::Mcp(args) => formanator::mcp::run(args),
    }
}

fn main() -> ExitCode {
    formanator::update_check::print_update_notification();
    let cli = Cli::parse();
    match dispatch(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}", format!("{e:#}").red());
            ExitCode::FAILURE
        }
    }
}
