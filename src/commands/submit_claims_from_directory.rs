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

    if args.openai_api_key.is_none() && args.github_token.is_none() {
        bail!(
            "You must provide either an OpenAI API key (--openai-api-key) or GitHub token (--github-token) to infer claim details from receipts."
        );
    }

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
                args.github_token.as_deref(),
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
