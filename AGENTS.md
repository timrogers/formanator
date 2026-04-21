# AGENTS.md

## Repository overview

- `formanator` is a Rust CLI for working with Forma claims, with optional MCP server support.
- `src/main.rs` is the binary entry point and dispatches subcommands.
- `src/cli.rs` defines the Clap CLI surface.
- `src/commands/` contains one handler per subcommand.
- `src/lib.rs` exposes shared modules such as Forma API access, claim handling, config, prompts, LLM integration, and MCP support.
- The default feature set includes `mcp`; the crate targets Rust edition 2024 and has an MSRV of Rust 1.88.

## Working in this repository

- Keep changes small and focused on the requested task.
- Follow existing Rust patterns and keep command wiring in `src/cli.rs`, `src/main.rs`, and `src/commands/`.
- Update `README.md` when user-facing behavior or developer workflow changes.
- Avoid changing dependencies or feature flags unless the task requires it.

## Build, test, and validation

Use the same checks that GitHub Actions uses:

1. Run pre-commit for formatting, linting, and repository hygiene:

   ```bash
   pre-commit run --all-files
   ```

   This covers the configured hook set, including `cargo fmt --all`, `cargo check --locked --workspace --all-features --all-targets`, `cargo clippy --locked --workspace --all-features --all-targets -- -D warnings`, codespell, and standard file checks.

2. Run the test command used by the build workflow:

   ```bash
   cargo test --locked --all-features
   ```

3. Run the release build used by the build workflow:

   ```bash
   cargo build --release --locked
   ```

## GitHub Actions alignment

- `.github/workflows/pre-commit.yml` runs the pre-commit checks on pushes and pull requests.
- `.github/workflows/build_and_release.yml` runs `cargo test --locked --all-features --target=<target>` and `cargo build --release --locked --target=<target>` across Linux, macOS, and Windows targets on every push.
- Tags matching `v*` additionally trigger GitHub release creation and Cargo publishing steps.
