use std::fs;

use anyhow::{Context, Result, bail};
use colored::Colorize;

use crate::cli::GenerateTemplateCsvArgs;

const TEMPLATE_CSV: &str =
    "benefit,category,merchant,amount,description,purchaseDate,receiptPath\n";

pub fn run(args: GenerateTemplateCsvArgs) -> Result<()> {
    if args.output_path.exists() {
        bail!(
            "File '{}' already exists. Please delete it first, or set a different `--output-path`.",
            args.output_path.display()
        );
    }
    fs::write(&args.output_path, TEMPLATE_CSV).with_context(|| {
        format!(
            "Failed to write template CSV to {}",
            args.output_path.display()
        )
    })?;
    println!(
        "{}",
        format!("Wrote template CSV to {}", args.output_path.display()).green()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_matches_expected_headers() {
        // The template CSV must have headers in the same shape that the CSV
        // reader expects. Keep this in sync with `crate::claims::EXPECTED_HEADERS`.
        let header_line = TEMPLATE_CSV.lines().next().unwrap();
        let mut headers: Vec<&str> = header_line.split(',').collect();
        headers.sort();
        let mut expected = vec![
            "category",
            "benefit",
            "amount",
            "merchant",
            "purchaseDate",
            "description",
            "receiptPath",
        ];
        expected.sort();
        assert_eq!(headers, expected);
    }

    #[test]
    fn template_matches_committed_fixture() {
        // A copy of the template lives under `tests/fixtures/template.csv` so
        // integration tests can compare against it. Keep them in sync.
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("template.csv");
        let on_disk = std::fs::read_to_string(&path).expect("read template fixture");
        assert_eq!(on_disk, TEMPLATE_CSV);
    }

    #[test]
    fn writes_template_to_a_fresh_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("claims.csv");
        let args = GenerateTemplateCsvArgs {
            output_path: path.clone(),
        };
        run(args).expect("should write");
        let contents = std::fs::read_to_string(&path).unwrap();
        assert_eq!(contents, TEMPLATE_CSV);
    }

    #[test]
    fn refuses_to_overwrite_an_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("claims.csv");
        std::fs::write(&path, "existing").unwrap();
        let args = GenerateTemplateCsvArgs {
            output_path: path.clone(),
        };
        let err = run(args).expect_err("should refuse");
        let msg = format!("{err}");
        assert!(msg.contains("already exists"), "{msg}");
        // The original contents must be preserved.
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "existing");
    }
}
