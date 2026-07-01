use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use chrono::Utc;
use colored::Colorize;

use crate::claims::{ClaimInput, claim_input_to_create_options};
use crate::cli::SubmitClaimsFromDirectoryArgs;
use crate::config::resolve_access_token;
use crate::forma::{create_claim, get_benefits_with_categories};
use crate::llm::infer_all_from_receipt;
use crate::prompt::prompt;
use crate::verbose;

const SUPPORTED_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "pdf", "heic"];

fn is_supported_receipt(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let lower = e.to_ascii_lowercase();
            SUPPORTED_EXTENSIONS.contains(&lower.as_str())
        })
        .unwrap_or(false)
}

fn list_receipt_files(directory: &Path) -> Result<Vec<PathBuf>> {
    if !directory.exists() {
        bail!("Directory '{}' does not exist.", directory.display());
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && is_supported_receipt(&path) {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn move_to_processed(source: &Path, processed_dir: &Path) -> Result<()> {
    fs::create_dir_all(processed_dir)?;
    let filename = source
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Receipt path has no filename: {}", source.display()))?;
    let mut destination = processed_dir.join(filename);
    if destination.exists() {
        let timestamp = Utc::now().format("%Y-%m-%dT%H-%M-%S-%3f").to_string();
        let ext = source.extension().and_then(|s| s.to_str()).unwrap_or("");
        let stem = source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("receipt");
        let new_name = if ext.is_empty() {
            format!("{stem}-{timestamp}")
        } else {
            format!("{stem}-{timestamp}.{ext}")
        };
        destination = processed_dir.join(new_name);
    }
    fs::rename(source, &destination)?;
    println!(
        "{}",
        format!("Moved processed receipt to: {}", destination.display()).blue()
    );
    Ok(())
}

pub fn run(args: SubmitClaimsFromDirectoryArgs) -> Result<()> {
    verbose::set(args.verbose);
    let access_token = resolve_access_token(args.access_token.as_deref())?;

    let processed_directory = args
        .processed_directory
        .clone()
        .unwrap_or_else(|| args.directory.join("processed"));

    let receipt_files = list_receipt_files(&args.directory)?;
    if receipt_files.is_empty() {
        println!(
            "{}",
            format!(
                "No supported receipt files found in directory: {}",
                args.directory.display()
            )
            .yellow()
        );
        println!(
            "{}",
            format!(
                "Supported file types: {}",
                SUPPORTED_EXTENSIONS
                    .iter()
                    .map(|e| format!(".{e}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
            .yellow()
        );
        return Ok(());
    }

    println!(
        "{}",
        format!("Found {} receipt file(s) to process:", receipt_files.len()).green()
    );
    for (i, file) in receipt_files.iter().enumerate() {
        println!(
            "  {}. {}",
            i + 1,
            file.file_name().unwrap_or_default().to_string_lossy()
        );
    }
    println!();

    let benefits = get_benefits_with_categories(&access_token)?;
    let mut processed = 0usize;
    let mut skipped = 0usize;

    for (index, receipt_file) in receipt_files.iter().enumerate() {
        let filename = receipt_file
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        println!();
        println!(
            "{}",
            format!(
                "--- Processing receipt {}/{}: {filename} ---",
                index + 1,
                receipt_files.len()
            )
            .cyan()
        );

        let outcome = (|| -> Result<bool> {
            println!("Analyzing receipt...");
            let inferred = infer_all_from_receipt(
                receipt_file,
                &benefits,
                args.openai_api_key.as_deref(),
                args.openai_base_url.as_deref(),
                args.openai_model.as_deref(),
                args.copilot_cli_path.as_deref(),
            )?;

            println!("{}", "\nInferred claim details:".green());
            println!("  Amount: {}", inferred.amount.yellow());
            println!("  Merchant: {}", inferred.merchant.yellow());
            println!("  Purchase Date: {}", inferred.purchase_date.yellow());
            println!("  Description: {}", inferred.description.yellow());
            println!("  Benefit: {}", inferred.benefit.yellow());
            println!("  Category: {}", inferred.category.yellow());

            println!(
                "\n{}",
                "Do you want to submit this claim? Enter Y to proceed or N to skip:".white()
            );
            let response = prompt("> ")?.trim().to_ascii_lowercase();
            if response == "y" || response == "yes" {
                println!("Submitting claim...");
                let claim = ClaimInput {
                    benefit: inferred.benefit,
                    category: inferred.category,
                    amount: inferred.amount,
                    merchant: inferred.merchant,
                    purchase_date: inferred.purchase_date,
                    description: inferred.description,
                    receipt_path: vec![receipt_file.clone()],
                };
                let opts = claim_input_to_create_options(&claim, &access_token)?;
                if args.dry_run {
                    println!("{}", "Dry run: skipping claim submission.".yellow());
                } else {
                    create_claim(&opts)?;
                }
                println!(
                    "{}",
                    format!("✅ Claim submitted successfully for {filename}").green()
                );
                if let Err(e) = move_to_processed(receipt_file, &processed_directory) {
                    eprintln!(
                        "{}",
                        format!(
                            "Warning: Could not move file {} to processed directory: {e}",
                            receipt_file.display()
                        )
                        .red()
                    );
                    eprintln!(
                        "{}",
                        "The claim was submitted successfully, but the file was not moved.".red()
                    );
                }
                Ok(true)
            } else {
                println!("{}", format!("Skipped {filename}").yellow());
                Ok(false)
            }
        })();

        match outcome {
            Ok(true) => processed += 1,
            Ok(false) => skipped += 1,
            Err(e) => {
                eprintln!("{}", format!("❌ Error processing {filename}: {e}").red());
                skipped += 1;
            }
        }
    }

    println!();
    println!("{}", "--- Summary ---".green());
    println!("Processed successfully: {}", processed.to_string().green());
    println!("Skipped: {}", skipped.to_string().yellow());
    println!("Total files: {}", receipt_files.len());
    if processed > 0 {
        println!(
            "{}",
            format!(
                "Processed receipts moved to: {}",
                processed_directory.display()
            )
            .blue()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    // ---------------------------------------------------------------------------
    // is_supported_receipt
    // ---------------------------------------------------------------------------

    #[test]
    fn is_supported_receipt_accepts_known_extensions() {
        for ext in &["jpg", "jpeg", "png", "pdf", "heic"] {
            let path = Path::new("receipt").with_extension(ext);
            assert!(is_supported_receipt(&path), ".{ext} should be supported");
        }
    }

    #[test]
    fn is_supported_receipt_is_case_insensitive() {
        for ext in &["JPG", "PDF", "HEIC", "PNG"] {
            let path = Path::new("receipt").with_extension(ext);
            assert!(is_supported_receipt(&path), ".{ext} should be supported");
        }
    }

    #[test]
    fn is_supported_receipt_rejects_unknown_extensions() {
        for ext in &["txt", "csv", "rs", "toml", "doc"] {
            let path = Path::new("file").with_extension(ext);
            assert!(
                !is_supported_receipt(&path),
                ".{ext} should not be supported"
            );
        }
    }

    #[test]
    fn is_supported_receipt_rejects_no_extension() {
        assert!(!is_supported_receipt(Path::new("receipt")));
        assert!(!is_supported_receipt(Path::new("some/path/file")));
    }

    // ---------------------------------------------------------------------------
    // list_receipt_files
    // ---------------------------------------------------------------------------

    #[test]
    fn list_receipt_files_returns_sorted_matching_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("b.pdf"), b"").unwrap();
        fs::write(dir.path().join("a.jpg"), b"").unwrap();
        fs::write(dir.path().join("c.txt"), b"").unwrap();

        let files = list_receipt_files(dir.path()).unwrap();
        assert_eq!(files.len(), 2, "only jpg and pdf should be listed");
        assert_eq!(files[0].file_name().unwrap(), "a.jpg");
        assert_eq!(files[1].file_name().unwrap(), "b.pdf");
    }

    #[test]
    fn list_receipt_files_ignores_subdirectories() {
        let dir = tempfile::tempdir().unwrap();
        // A sub-directory that happens to have a receipt-like name.
        fs::create_dir(dir.path().join("subdir.jpg")).unwrap();
        fs::write(dir.path().join("real.png"), b"").unwrap();

        let files = list_receipt_files(dir.path()).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_name().unwrap(), "real.png");
    }

    #[test]
    fn list_receipt_files_errors_on_missing_directory() {
        let err = list_receipt_files(Path::new("/nonexistent/path/xyz")).unwrap_err();
        assert!(
            format!("{err}").contains("does not exist"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn list_receipt_files_returns_empty_for_no_matching_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("notes.txt"), b"").unwrap();

        let files = list_receipt_files(dir.path()).unwrap();
        assert!(files.is_empty());
    }

    // ---------------------------------------------------------------------------
    // move_to_processed
    // ---------------------------------------------------------------------------

    #[test]
    fn move_to_processed_moves_file_to_destination() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("receipt.jpg");
        fs::write(&source, b"data").unwrap();
        let processed = dir.path().join("processed");

        move_to_processed(&source, &processed).unwrap();

        assert!(!source.exists(), "source should have been moved");
        assert!(
            processed.join("receipt.jpg").exists(),
            "destination should exist"
        );
    }

    #[test]
    fn move_to_processed_creates_destination_directory() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("r.png");
        fs::write(&source, b"img").unwrap();
        let processed = dir.path().join("a").join("b").join("processed");

        move_to_processed(&source, &processed).unwrap();

        assert!(processed.join("r.png").exists());
    }

    #[test]
    fn move_to_processed_adds_timestamp_on_collision() {
        let dir = tempfile::tempdir().unwrap();
        let processed = dir.path().join("processed");

        // First file.
        let source1 = dir.path().join("receipt.jpg");
        fs::write(&source1, b"first").unwrap();
        move_to_processed(&source1, &processed).unwrap();

        // Second file with the same name.
        let source2 = dir.path().join("receipt.jpg");
        fs::write(&source2, b"second").unwrap();
        move_to_processed(&source2, &processed).unwrap();

        // The original destination exists, and a timestamped variant was created.
        assert!(
            processed.join("receipt.jpg").exists(),
            "original destination missing"
        );
        let entries: Vec<_> = fs::read_dir(&processed)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(entries.len(), 2, "expected two files in processed dir");
        // The second file should have a name containing a timestamp (hyphen-separated digits).
        let names: Vec<String> = entries
            .iter()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        let has_timestamped = names.iter().any(|n| n != "receipt.jpg");
        assert!(
            has_timestamped,
            "expected a timestamped collision file; got: {names:?}"
        );
    }
}
