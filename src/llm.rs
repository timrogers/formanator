//! LLM-powered inference for claim metadata. Supports both the OpenAI API and
//! the GitHub Models inference endpoint via the `async-openai` crate, which
//! speaks the OpenAI chat-completions protocol.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};
use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::types::chat::{
    ChatCompletionRequestMessage, ChatCompletionRequestMessageContentPartImageArgs,
    ChatCompletionRequestMessageContentPartTextArgs, ChatCompletionRequestUserMessageArgs,
    ChatCompletionRequestUserMessageContent, ChatCompletionRequestUserMessageContentPart,
    CreateChatCompletionRequestArgs, ImageDetail, ImageUrlArgs,
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use regex::Regex;
use serde::Deserialize;

use crate::forma::BenefitWithCategories;
use crate::verbose::is_enabled as is_verbose;

const OPENAI_BASE: &str = "https://api.openai.com/v1";
const OPENAI_MODEL: &str = "gpt-4o";
const GITHUB_MODELS_BASE: &str = "https://models.github.ai/inference";
const GITHUB_MODELS_MODEL: &str = "openai/gpt-4.1";

// Base-URL override for the LLM API. Production code never sets this; the
// integration tests in `tests/llm_api.rs` and `tests/cli.rs` use it to point
// the OpenAI-compatible client at a local mock HTTP server instead of the
// real OpenAI / GitHub Models endpoints.
static LLM_API_BASE: std::sync::RwLock<Option<String>> = std::sync::RwLock::new(None);

/// Override the LLM API base URL used for OpenAI-compatible chat-completions
/// calls. Passing `None` clears any previous override. This is exposed
/// publicly so that integration tests can call it; production code should
/// never do so.
pub fn set_llm_api_base(base: Option<String>) {
    if let Ok(mut guard) = LLM_API_BASE.write() {
        *guard = base;
    }
}

fn llm_api_base_override() -> Option<String> {
    LLM_API_BASE.read().ok().and_then(|g| g.clone())
}

/// Resolved configuration for an OpenAI-compatible API call.
struct ApiConfig {
    client: Client<OpenAIConfig>,
    model: &'static str,
    api_base: String,
}

fn resolve_api_config(
    openai_api_key: Option<&str>,
    github_token: Option<&str>,
) -> Result<ApiConfig> {
    let openai = openai_api_key.filter(|s| !s.is_empty());
    let github = github_token.filter(|s| !s.is_empty());

    if openai.is_some() && github.is_some() {
        eprintln!(
            "Warning: You have provided both an OpenAI API key and a GitHub token. Defaulting to using OpenAI."
        );
    }

    let (base, key, model) = if let Some(key) = openai {
        (OPENAI_BASE, key, OPENAI_MODEL)
    } else if let Some(key) = github {
        (GITHUB_MODELS_BASE, key, GITHUB_MODELS_MODEL)
    } else {
        bail!("You must either specify a GitHub token or an OpenAI API key.")
    };

    let base = llm_api_base_override().unwrap_or_else(|| base.to_string());
    let config = OpenAIConfig::new().with_api_base(&base).with_api_key(key);
    Ok(ApiConfig {
        client: Client::with_config(config),
        model,
        api_base: base,
    })
}

fn run_blocking<F: std::future::Future<Output = Result<String>>>(future: F) -> Result<String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("Failed to build Tokio runtime for LLM call")?;
    runtime.block_on(future)
}

async fn call_chat_completion(
    config: &ApiConfig,
    messages: Vec<ChatCompletionRequestMessage>,
) -> Result<String> {
    let request = CreateChatCompletionRequestArgs::default()
        .model(config.model)
        .messages(messages)
        .build()
        .context("Failed to build chat completions request")?;

    if is_verbose() {
        eprintln!("[verbose] > POST {}/chat/completions", config.api_base);
        match serde_json::to_string(&request) {
            Ok(body) => eprintln!("[verbose] > Body: {body}"),
            Err(err) => eprintln!("[verbose] > Body: <failed to serialize: {err}>"),
        }
    }

    let response = config
        .client
        .chat()
        .create(request)
        .await
        .context("Failed to call chat completions endpoint")?;

    if is_verbose() {
        match serde_json::to_string(&response) {
            Ok(body) => eprintln!("[verbose] < Body: {body}"),
            Err(err) => eprintln!("[verbose] < Body: <failed to serialize: {err}>"),
        }
    }

    response
        .choices
        .into_iter()
        .next()
        .and_then(|c| c.message.content)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| anyhow!("LLM returned an empty response."))
}

fn user_text_message(text: String) -> Result<ChatCompletionRequestMessage> {
    let message = ChatCompletionRequestUserMessageArgs::default()
        .content(ChatCompletionRequestUserMessageContent::Text(text))
        .build()
        .context("Failed to build user message")?;
    Ok(ChatCompletionRequestMessage::User(message))
}

fn user_text_and_image_message(
    text: String,
    image_data_url: String,
) -> Result<ChatCompletionRequestMessage> {
    let text_part = ChatCompletionRequestMessageContentPartTextArgs::default()
        .text(text)
        .build()
        .context("Failed to build text content part")?;
    let image_part = ChatCompletionRequestMessageContentPartImageArgs::default()
        .image_url(
            ImageUrlArgs::default()
                .url(image_data_url)
                .detail(ImageDetail::High)
                .build()
                .context("Failed to build image URL")?,
        )
        .build()
        .context("Failed to build image content part")?;

    let message = ChatCompletionRequestUserMessageArgs::default()
        .content(ChatCompletionRequestUserMessageContent::Array(vec![
            ChatCompletionRequestUserMessageContentPart::Text(text_part),
            ChatCompletionRequestUserMessageContentPart::ImageUrl(image_part),
        ]))
        .build()
        .context("Failed to build user message")?;
    Ok(ChatCompletionRequestMessage::User(message))
}

// ---------------------------------------------------------------------------
// Category / benefit inference (text-only)
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct InferredCategoryAndBenefit {
    pub category: String,
    pub benefit: String,
}

pub fn infer_category_and_benefit(
    merchant: &str,
    description: &str,
    benefits_with_categories: &[BenefitWithCategories],
    openai_api_key: Option<&str>,
    github_token: Option<&str>,
) -> Result<InferredCategoryAndBenefit> {
    let config = resolve_api_config(openai_api_key, github_token)?;

    let valid_categories: Vec<String> = benefits_with_categories
        .iter()
        .flat_map(|b| {
            b.categories.iter().map(|c| {
                c.subcategory_alias
                    .clone()
                    .unwrap_or_else(|| c.subcategory_name.clone())
            })
        })
        .collect();

    let prompt = format!(
        "Your job is to predict the category for an expense claim based on the name of the merchant and a description of what was purchased. You should give a single, specific answer without any extra words or punctuation.\n\nHere are the possible categories:\n\n{}\n\nPlease predict the category for the following claim:\n\nMerchant: {}\nDescription: {}",
        valid_categories.join("\n"),
        merchant,
        description,
    );

    let messages = vec![user_text_message(prompt)?];
    let response = run_blocking(call_chat_completion(&config, messages))?;
    let trimmed = response.trim().to_string();

    // Find the matching category to derive the benefit name.
    let categories_with_benefits: Vec<(String, String, String)> = benefits_with_categories
        .iter()
        .flat_map(|b| {
            b.categories.iter().map(move |c| {
                (
                    b.benefit.name.clone(),
                    c.subcategory_alias
                        .clone()
                        .unwrap_or_else(|| c.subcategory_name.clone()),
                    c.subcategory_name.clone(),
                )
            })
        })
        .collect();

    let matched = categories_with_benefits
        .iter()
        .find(|(_, alias_or_name, name)| alias_or_name == &trimmed || name == &trimmed)
        .ok_or_else(|| {
            anyhow!("The LLM returned a response that wasn't a valid category: {trimmed}")
        })?;

    Ok(InferredCategoryAndBenefit {
        category: trimmed,
        benefit: matched.0.clone(),
    })
}

// ---------------------------------------------------------------------------
// Receipt inference (vision)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct ReceiptInferenceResult {
    pub amount: String,
    pub merchant: String,
    #[serde(rename = "purchaseDate")]
    pub purchase_date: String,
    pub description: String,
    pub category: String,
    pub benefit: String,
}

pub fn infer_all_from_receipt(
    receipt_path: &Path,
    benefits_with_categories: &[BenefitWithCategories],
    openai_api_key: Option<&str>,
    github_token: Option<&str>,
) -> Result<ReceiptInferenceResult> {
    let config = resolve_api_config(openai_api_key, github_token)?;

    let image_path = convert_to_image_if_needed(receipt_path)?;
    let image_b64 = encode_image_to_base64(&image_path)?;
    let data_url = format!("data:image/jpeg;base64,{image_b64}");

    let valid_categories: Vec<String> = benefits_with_categories
        .iter()
        .flat_map(|b| {
            b.categories.iter().map(|c| {
                c.subcategory_alias
                    .clone()
                    .unwrap_or_else(|| c.subcategory_name.clone())
            })
        })
        .collect();
    let valid_benefits: Vec<String> = benefits_with_categories
        .iter()
        .map(|b| b.benefit.name.clone())
        .collect();

    let valid_benefits_list = valid_benefits
        .iter()
        .map(|b| format!("- `{b}`"))
        .collect::<Vec<_>>()
        .join("\n");
    let valid_categories_list = valid_categories
        .iter()
        .map(|c| format!("- `{c}`"))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
        "Your job is to analyze a receipt image and extract ALL required information for an expense claim. You must return a JSON object with the following fields:\n\n- amount: The total amount (e.g., \"25.99\")\n- merchant: The name of the merchant/store\n- purchaseDate: The date in YYYY-MM-DD format\n- description: A brief description of what was purchased\n- benefit: The most appropriate benefit category from the valid benefits list. Only benefits from the provided list are valid.\n- category: The most appropriate category from the valid categories list. Only categories from the provided list are valid.\n\nValid benefits:\n{valid_benefits_list}\n\nValid categories:\n{valid_categories_list}\n\nReturn ONLY a valid JSON object with these exact field names. Do not include any other text or formatting. Do not wrap the JSON object in a markdown code block syntax.",
    );

    let messages = vec![user_text_and_image_message(prompt, data_url)?];
    let raw = run_blocking(call_chat_completion(&config, messages))?;

    // Strip markdown code fences if the model added them despite the prompt.
    let cleaned = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let parsed: ReceiptInferenceResult = serde_json::from_str(cleaned)
        .with_context(|| format!("Failed to parse LLM response as JSON: {raw}"))?;

    // Validate benefit
    let matching_benefit = benefits_with_categories
        .iter()
        .find(|b| b.benefit.name == parsed.benefit)
        .ok_or_else(|| {
            anyhow!(
                "The LLM returned a benefit that wasn't valid: {}",
                parsed.benefit
            )
        })?;

    // Validate category for that benefit
    let valid = matching_benefit.categories.iter().any(|c| {
        c.subcategory_alias.as_deref() == Some(parsed.category.as_str())
            || c.subcategory_name == parsed.category
    });
    if !valid {
        bail!(
            "The LLM returned a category that wasn't valid for the benefit: {}",
            parsed.category
        );
    }

    let date_re = Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
    if !date_re.is_match(&parsed.purchase_date) {
        bail!(
            "The LLM returned an invalid date format: {}. Expected YYYY-MM-DD.",
            parsed.purchase_date
        );
    }
    let amount_re = Regex::new(r"^\d+(\.\d{1,2})?$").unwrap();
    if !amount_re.is_match(&parsed.amount) {
        bail!(
            "The LLM returned an invalid amount format: {}. Expected up to two decimals.",
            parsed.amount
        );
    }
    if parsed.merchant.trim().is_empty() {
        bail!("The LLM returned an empty merchant name.");
    }
    if parsed.description.trim().is_empty() {
        bail!("The LLM returned an empty description.");
    }

    Ok(parsed)
}

// ---------------------------------------------------------------------------
// Receipt → image conversion
// ---------------------------------------------------------------------------

fn convert_to_image_if_needed(receipt_path: &Path) -> Result<PathBuf> {
    let ext = receipt_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if ext != "pdf" {
        return Ok(receipt_path.to_path_buf());
    }

    // Convert the first page of the PDF to a JPEG using GraphicsMagick (which
    // delegates to Ghostscript). This mirrors the upstream `pdf2pic` setup.
    let tmp_dir = std::env::temp_dir();
    let stem = receipt_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("receipt");
    let output = tmp_dir.join(format!("formanator-{stem}-{}.jpg", std::process::id()));

    let status = Command::new("gm")
        .args(["convert", "-density", "100", "-resize", "2000x2000"])
        .arg(format!("{}[0]", receipt_path.display()))
        .arg(&output)
        .status();

    match status {
        Ok(s) if s.success() && output.exists() => Ok(output),
        Ok(s) => Err(anyhow!(
            "Failed to convert PDF receipt at {} to a JPEG: `gm convert` exited with {}. Please ensure GraphicsMagick and Ghostscript are installed (e.g. `brew install graphicsmagick ghostscript` on macOS, or `apt install graphicsmagick ghostscript` on Debian/Ubuntu), or use a JPEG/PNG receipt instead.",
            receipt_path.display(),
            s
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(anyhow!(
            "Failed to convert PDF receipt at {}: the GraphicsMagick `gm` command was not found on your PATH. Please install GraphicsMagick and Ghostscript (e.g. `brew install graphicsmagick ghostscript` on macOS, or `apt install graphicsmagick ghostscript` on Debian/Ubuntu), or use a JPEG/PNG receipt instead.",
            receipt_path.display()
        )),
        Err(e) => Err(anyhow!(
            "Failed to invoke `gm convert` to convert PDF receipt at {}: {e}. Please ensure GraphicsMagick and Ghostscript are installed, or use a JPEG/PNG receipt instead.",
            receipt_path.display()
        )),
    }
}

fn encode_image_to_base64(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read image file at {}", path.display()))?;
    Ok(BASE64.encode(bytes))
}
