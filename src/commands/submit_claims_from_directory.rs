use std::fs;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use chrono::Utc;
use colored::Colorize;

use crate::claims::{ClaimInput, claim_input_to_create_options};
use crate::cli::SubmitClaimsFromDirectoryArgs;
use crate::config::resolve_access_token;
use crate::forma::{create_claim, get_benefits_with_categories};
use crate::llm::{infer_all_from_receipt, is_provider_error};
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

const PROGRESS_WIDTH: usize = 30;

/// Draw a single-line progress bar, rewriting it in place. Falls back to one
/// plain line per receipt when stdout isn't a terminal, so piped output and CI
/// logs stay readable.
fn draw_progress(done: usize, total: usize, label: &str) {
    let mut stdout = std::io::stdout();
    if !stdout.is_terminal() {
        println!("Analyzing receipt {done}/{total}: {label}");
        return;
    }
    // \r returns to the start of the line and \x1b[K clears whatever the
    // previous, possibly longer, filename left behind.
    print!("\r\x1b[K{}", progress_line(done, total, label));
    let _ = stdout.flush();
}

fn progress_line(done: usize, total: usize, label: &str) -> String {
    let filled = (PROGRESS_WIDTH * done.min(total) / total.max(1)).min(PROGRESS_WIDTH);
    let bar = format!(
        "{}{}",
        "\u{2588}".repeat(filled),
        "\u{2591}".repeat(PROGRESS_WIDTH - filled)
    );
    format!("[{}] {done}/{total} {label}", bar.cyan())
}

fn finish_progress() {
    if std::io::stdout().is_terminal() {
        println!();
    }
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

    // Every receipt is analysed up front so all the confirmation prompts
    // happen together, without waiting for the LLM in between.
    println!(
        "{}",
        format!(
            "Analyzing {} receipt(s) before review...",
            receipt_files.len()
        )
        .cyan()
    );
    let mut analyzed = Vec::new();
    for (index, receipt_file) in receipt_files.iter().enumerate() {
        draw_progress(
            index + 1,
            receipt_files.len(),
            &receipt_file
                .file_name()
                .unwrap_or_default()
                .to_string_lossy(),
        );
        let result = infer_all_from_receipt(
            receipt_file,
            &benefits,
            args.openai_api_key.as_deref(),
            args.openai_base_url.as_deref(),
            args.openai_model.as_deref(),
            args.copilot_cli_path.as_deref(),
        );
        // Nothing has been submitted yet, so a dead provider means bailing
        // out now rather than making the user sit through the rest.
        if let Err(error) = &result
            && is_provider_error(error)
        {
            finish_progress();
            return Err(result.unwrap_err()).context(
                "Aborted before reviewing any claims because the receipts could not be analysed",
            );
        }
        analyzed.push(result);
    }
    finish_progress();

    let mut processed = 0usize;
    let mut skipped = 0usize;

    for (index, (receipt_file, inferred)) in receipt_files.iter().zip(analyzed).enumerate() {
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
            let inferred = inferred?;

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
                eprintln!("{}", format!("❌ Error processing {filename}: {e:#}").red());
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
    use super::{PROGRESS_WIDTH, progress_line};

    fn blocks(line: &str) -> (usize, usize) {
        (
            line.matches('\u{2588}').count(),
            line.matches('\u{2591}').count(),
        )
    }

    #[test]
    fn progress_line_fills_in_proportion_and_never_overflows() {
        assert_eq!(blocks(&progress_line(0, 4, "a.jpg")), (0, PROGRESS_WIDTH));
        assert_eq!(
            blocks(&progress_line(2, 4, "a.jpg")),
            (PROGRESS_WIDTH / 2, PROGRESS_WIDTH / 2)
        );
        assert_eq!(blocks(&progress_line(4, 4, "a.jpg")), (PROGRESS_WIDTH, 0));
        // Degenerate inputs must not panic or overflow the bar.
        assert_eq!(blocks(&progress_line(0, 0, "a.jpg")), (0, PROGRESS_WIDTH));
        assert_eq!(blocks(&progress_line(9, 4, "a.jpg")), (PROGRESS_WIDTH, 0));
    }

    #[test]
    fn progress_line_shows_the_count_and_the_receipt_name() {
        let line = progress_line(3, 7, "receipt.jpg");
        assert!(line.contains("3/7"), "{line}");
        assert!(line.contains("receipt.jpg"), "{line}");
    }
}
