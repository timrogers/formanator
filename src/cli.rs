//! Clap-based CLI definition. Each subcommand is wired up to a handler in the
//! [`crate::commands`] module by [`crate::main`].

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "formanator",
    version,
    about = "Submit Forma <https://joinforma.com> benefit claims from the command line.",
    long_about = "Submit Forma <https://joinforma.com> benefit claims from the command line.\n\nSupports automatic inference of claim details from receipts via OpenAI or GitHub Models."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Connect Formanator to your Forma account with a magic link.
    Login(LoginArgs),
    /// List benefits in your Forma account and their remaining balances.
    Benefits(BenefitsArgs),
    /// List categories available for a Forma benefit.
    Categories(CategoriesArgs),
    /// List claims in your Forma account and their current status.
    ListClaims(ListClaimsArgs),
    /// Submit a single claim for a Forma benefit.
    SubmitClaim(SubmitClaimArgs),
    /// Generate a template CSV for submitting multiple claims at once.
    GenerateTemplateCsv(GenerateTemplateCsvArgs),
    /// Submit multiple claims from a CSV.
    SubmitClaimsFromCsv(SubmitClaimsFromCsvArgs),
    /// Submit claims for every receipt found in a directory.
    SubmitClaimsFromDirectory(SubmitClaimsFromDirectoryArgs),
    /// Validate a completed claims CSV before submitting it.
    ValidateCsv(ValidateCsvArgs),
    /// Run Formanator as a Model Context Protocol (MCP) server over stdio.
    #[cfg(feature = "mcp")]
    Mcp(McpArgs),
}

#[derive(Debug, clap::Args)]
pub struct LoginArgs {
    /// Provide a magic link directly instead of opening the browser.
    #[arg(long)]
    pub magic_link: Option<String>,
    /// Log HTTP requests and responses to stderr for debugging.
    #[arg(long)]
    pub verbose: bool,
}

#[derive(Debug, clap::Args)]
pub struct BenefitsArgs {
    /// Access token used to authenticate with Forma.
    #[arg(long, env = "FORMANATOR_ACCESS_TOKEN")]
    pub access_token: Option<String>,
    /// Log HTTP requests and responses to stderr for debugging.
    #[arg(long)]
    pub verbose: bool,
}

#[derive(Debug, clap::Args)]
pub struct CategoriesArgs {
    /// The benefit to list categories for.
    #[arg(long)]
    pub benefit: String,
    /// Access token used to authenticate with Forma.
    #[arg(long, env = "FORMANATOR_ACCESS_TOKEN")]
    pub access_token: Option<String>,
    /// Log HTTP requests and responses to stderr for debugging.
    #[arg(long)]
    pub verbose: bool,
}

#[derive(Debug, clap::Args)]
pub struct ListClaimsArgs {
    /// Filter claims by status (currently supports: in_progress).
    #[arg(long)]
    pub filter: Option<String>,
    /// Access token used to authenticate with Forma.
    #[arg(long, env = "FORMANATOR_ACCESS_TOKEN")]
    pub access_token: Option<String>,
    /// Log HTTP requests and responses to stderr for debugging.
    #[arg(long)]
    pub verbose: bool,
}

#[derive(Debug, clap::Args)]
pub struct SubmitClaimArgs {
    /// The benefit you are claiming for. Optional when using LLM inference.
    #[arg(long)]
    pub benefit: Option<String>,
    /// The category of the claim. Optional when using LLM inference.
    #[arg(long)]
    pub category: Option<String>,
    /// The amount of the claim. Optional when using full receipt inference.
    #[arg(long)]
    pub amount: Option<String>,
    /// The name of the merchant. Optional when using full receipt inference.
    #[arg(long)]
    pub merchant: Option<String>,
    /// The date of purchase in YYYY-MM-DD format. Optional when using full receipt inference.
    #[arg(long)]
    pub purchase_date: Option<String>,
    /// The description of the claim. Optional when using full receipt inference.
    #[arg(long)]
    pub description: Option<String>,
    /// The path of the receipt. Pass multiple times to attach multiple files.
    #[arg(long, required = true, num_args = 1..)]
    pub receipt_path: Vec<PathBuf>,
    /// Access token used to authenticate with Forma.
    #[arg(long, env = "FORMANATOR_ACCESS_TOKEN")]
    pub access_token: Option<String>,
    /// OpenAI API key used to infer claim details. Defaults to the `OPENAI_API_KEY` environment variable.
    #[arg(long, env = "OPENAI_API_KEY")]
    pub openai_api_key: Option<String>,
    /// GitHub token used to infer claim details via GitHub Models. Defaults to the `GITHUB_TOKEN` environment variable.
    #[arg(long, env = "GITHUB_TOKEN")]
    pub github_token: Option<String>,
    /// Run through the entire flow without actually submitting the claim.
    #[arg(long)]
    pub dry_run: bool,
    /// Log HTTP requests and responses to stderr for debugging.
    #[arg(long)]
    pub verbose: bool,
}

#[derive(Debug, clap::Args)]
pub struct GenerateTemplateCsvArgs {
    /// The path to write the CSV template to.
    #[arg(long, default_value = "claims.csv")]
    pub output_path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct SubmitClaimsFromCsvArgs {
    /// The path to the CSV to read claims from.
    #[arg(long)]
    pub input_path: PathBuf,
    /// Access token used to authenticate with Forma.
    #[arg(long, env = "FORMANATOR_ACCESS_TOKEN")]
    pub access_token: Option<String>,
    /// OpenAI API key used to infer claim details for rows that leave columns blank.
    #[arg(long, env = "OPENAI_API_KEY")]
    pub openai_api_key: Option<String>,
    /// GitHub token used to infer claim details via GitHub Models.
    #[arg(long, env = "GITHUB_TOKEN")]
    pub github_token: Option<String>,
    /// Run through the entire flow without actually submitting the claims.
    #[arg(long)]
    pub dry_run: bool,
    /// Log HTTP requests and responses to stderr for debugging.
    #[arg(long)]
    pub verbose: bool,
}

#[derive(Debug, clap::Args)]
pub struct SubmitClaimsFromDirectoryArgs {
    /// Directory containing receipt files to process. Supported file types: JPEG, PNG, PDF, HEIC.
    #[arg(long)]
    pub directory: PathBuf,
    /// Directory to move successfully processed receipts to. Defaults to `<directory>/processed`.
    #[arg(long)]
    pub processed_directory: Option<PathBuf>,
    /// Access token used to authenticate with Forma.
    #[arg(long, env = "FORMANATOR_ACCESS_TOKEN")]
    pub access_token: Option<String>,
    /// OpenAI API key used to infer claim details from receipts.
    #[arg(long, env = "OPENAI_API_KEY")]
    pub openai_api_key: Option<String>,
    /// GitHub token used to infer claim details via GitHub Models.
    #[arg(long, env = "GITHUB_TOKEN")]
    pub github_token: Option<String>,
    /// Run through the entire flow without actually submitting the claims.
    #[arg(long)]
    pub dry_run: bool,
    /// Log HTTP requests and responses to stderr for debugging.
    #[arg(long)]
    pub verbose: bool,
}

#[derive(Debug, clap::Args)]
pub struct ValidateCsvArgs {
    /// The path to the CSV to read claims from.
    #[arg(long)]
    pub input_path: PathBuf,
    /// Access token used to authenticate with Forma.
    #[arg(long, env = "FORMANATOR_ACCESS_TOKEN")]
    pub access_token: Option<String>,
    /// Log HTTP requests and responses to stderr for debugging.
    #[arg(long)]
    pub verbose: bool,
}

#[cfg(feature = "mcp")]
#[derive(Debug, clap::Args)]
pub struct McpArgs {
    /// Access token used to authenticate with Forma.
    #[arg(long, env = "FORMANATOR_ACCESS_TOKEN")]
    pub access_token: Option<String>,
}
