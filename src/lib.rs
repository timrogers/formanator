//! `formanator` library crate. The CLI binary in `src/main.rs` wires this
//! together with [Clap](https://docs.rs/clap/) and the various subcommand
//! handlers in [`commands`].

pub mod claims;
pub mod cli;
pub mod commands;
pub mod config;
pub mod forma;
pub mod llm;
#[cfg(feature = "mcp")]
pub mod mcp;
pub mod prompt;
pub mod verbose;
