//! Subcommand handlers. Each module exposes a `run(args)` function that the
//! [`crate::main`] dispatcher calls.

pub mod benefits;
pub mod categories;
pub mod generate_template_csv;
pub mod list_claims;
pub mod login;
pub mod submit_claim;
pub mod submit_claims_from_csv;
pub mod submit_claims_from_directory;
pub mod validate_csv;
