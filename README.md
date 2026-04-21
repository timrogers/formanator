# Formanator 🤖

> [!NOTE]  
> 🦀 Formanator is now built with Rust and distributed through Homebrew and Crates.io! [v2.x](https://github.com/timrogers/formanator/releases/tag/v2.1.0), built in TypeScript and distributed through npm, is [still available](https://github.com/timrogers/formanator/releases/tag/v2.1.0).

Formanator allows you to:

* **Submit benefit claims to [Forma](https://www.joinforma.com/) and track progress from the command line**, either one-by-one or in bulk
* **Understand your Forma benefits and track and submit claims from any Model Context Protocol (MCP) client**, for example [Copilot CLI](https://github.com/features/copilot/cli), [Visual Studio Code](https://code.visualstudio.com/) or [Claude Code](https://code.claude.com/docs/en/overview)

With the power of large language models 🧠👀 - free of charge thanks to [GitHub Models](https://docs.github.com/en/github-models/use-github-models/prototyping-with-ai-models) - it can even **analyse your receipts and generate your claims automatically**.

![Screenshot of running `formanator` from a terminal](https://github.com/user-attachments/assets/e053efc8-f4cb-4ea1-8850-6c82d668bf29)

## Installation

### macOS or Linux via [Homebrew](https://brew.sh/)

```bash
brew tap timrogers/tap && brew install formanator
```

### macOS, Linux, or Windows via [Cargo](https://doc.rust-lang.org/cargo/), Rust's package manager

1. Install [Rust](https://www.rust-lang.org/tools/install) on your machine, if it isn't already installed.
1. Install the `formanator` crate by running `cargo install formanator`.
1. Run `formanator --help` to check that everything is working and see the available commands.

### macOS, Linux, or Windows via direct binary download

1. Download the [latest release](https://github.com/timrogers/formanator/releases/latest) for your platform. macOS, Linux, and Windows devices are supported.
1. Add the binary to your `PATH` (or `$PATH` on Unix-like systems), so you can execute it from your shell/terminal. For the best experience, call it `formanator` (or `formanator.exe` on Windows).
1. Run `formanator --help` to check that everything is working.

### From source

```bash
git clone https://github.com/timrogers/formanator
cd formanator
cargo install --path .
```

### Optional: PDF receipt support

To infer claim details for PDF receipts, you need to have [GraphicsMagick](http://www.graphicsmagick.org/) and [Ghostscript](https://www.ghostscript.com/) installed.

```bash
# macOS
brew install graphicsmagick ghostscript
```

## Usage

### Connecting to your Forma account

To get started, you'll need to connect Formanator to your Forma account:

1. Run `formanator login`.
2. Press Enter to open your browser to the Forma login page.
3. Enter your email address and request a magic link.
4. Copy the magic link from your email and paste it into the terminal.
5. You're logged in 🥳

The access token is stored in `~/.formanatorrc.json` (the same location used by the original Node.js implementation, so the two clients can share state).

### Configuring an LLM provider (optional, but recommended)

When submitting a claim you can either provide every detail manually or let an LLM infer them. Two providers are supported:

- **GitHub Models** — free, with a generous quota. Set the `GITHUB_TOKEN` environment variable to a GitHub Personal Access Token with **read access to GitHub Models**, or pass `--github-token`.
- **OpenAI** — billed to your OpenAI account. Set the `OPENAI_API_KEY` environment variable, or pass `--openai-api-key`.

If both are configured, Formanator prefers OpenAI.

### Submitting claims in bulk

#### Automatically submitting all receipts in a directory (recommended)

```bash
formanator submit-claims-from-directory --directory input/
```

All `.jpg`, `.jpeg`, `.png`, `.pdf` and `.heic` receipts in the directory will be analysed by the LLM. You'll be asked to confirm the inferred claim details for each receipt before it's submitted, and successfully-submitted receipts are moved into a `processed/` subdirectory.

#### Manually submitting receipts using a CSV template

1. Generate a template: `formanator generate-template-csv` (writes `claims.csv`).
2. Fill in one row per claim. If you've configured an LLM, you can leave `benefit` and `category` blank to have them inferred from the other fields, or leave every column except `receiptPath` blank to have all claim details inferred from the receipt. Comma-separate paths in the `receiptPath` column to attach multiple receipts.
3. Optionally validate up-front: `formanator validate-csv --input-path claims.csv`.
4. Submit: `formanator submit-claims-from-csv --input-path claims.csv`.

### Submitting a single claim

#### Option 1: Infer all claim details from the receipt (recommended)

```bash
formanator submit-claim --receipt-path receipt.jpg
```

Formanator will ask the LLM to extract the amount, merchant, purchase date, description, benefit and category, show you the result and ask you to confirm before submitting.

#### Option 2: Provide details manually, infer benefit and category

```bash
formanator submit-claim \
  --amount 2.28 \
  --merchant Amazon \
  --description "USB cable" \
  --purchase-date 2024-01-15 \
  --receipt-path USB.pdf
```

#### Option 3: Provide every detail manually

```bash
formanator submit-claim \
  --amount 2.28 \
  --merchant Amazon \
  --description "USB cable" \
  --purchase-date 2024-01-15 \
  --receipt-path USB.pdf \
  --benefit "Remote Life" \
  --category "Cables & Cords"
```

Use `formanator benefits` and `formanator categories --benefit <benefit>` to discover the valid values.

### Listing claims

```bash
formanator list-claims
formanator list-claims --filter in_progress
```

## Model Context Protocol (MCP) usage

Formanator can run as an MCP server over stdio so AI assistants can interact with your Forma account programmatically.

```jsonc
{
  "mcpServers": {
    "formanator": {
      "command": "/path/to/formanator",
      "args": ["mcp"]
    }
  }
}
```

The server exposes three tools:

- `list_benefits_with_categories` — list all benefits with their categories and remaining balances.
- `list_claims` — list claims, with optional filtering (currently only `in_progress`).
- `create_claim` — create a new claim.

You must be logged in (`formanator login`) before starting the MCP server.

To build a binary without MCP support (smaller binary, fewer dependencies):

```bash
cargo install formanator --no-default-features
```

## Development

```bash
cargo build              # build with default features (CLI + MCP)
cargo test --all-features
cargo clippy --all-features --all-targets -- -D warnings
cargo fmt --all
```