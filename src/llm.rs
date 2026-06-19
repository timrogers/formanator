//! LLM-powered inference for claim metadata. Supports two providers:
//!
//! 1. The OpenAI API, spoken via the `async-openai` crate (the OpenAI
//!    chat-completions protocol).
//! 2. The GitHub Copilot CLI, via the `github-copilot-sdk` crate. This is the
//!    default when no OpenAI API key has been provided.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use async_openai::Client as OpenAiClient;
use async_openai::config::OpenAIConfig;
use async_openai::types::chat::{
    ChatCompletionRequestMessage, ChatCompletionRequestMessageContentPartImageArgs,
    ChatCompletionRequestMessageContentPartTextArgs, ChatCompletionRequestUserMessageArgs,
    ChatCompletionRequestUserMessageContent, ChatCompletionRequestUserMessageContentPart,
    CreateChatCompletionRequestArgs, ImageDetail, ImageUrlArgs,
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use github_copilot_sdk::types::{Attachment, MessageOptions, SessionConfig};
use github_copilot_sdk::{CliProgram, Client as CopilotClient, ClientOptions};
use regex::Regex;
use serde::Deserialize;

use crate::forma::BenefitWithCategories;
use crate::verbose::is_enabled as is_verbose;

const OPENAI_BASE: &str = "https://api.openai.com/v1";
const OPENAI_MODEL: &str = "gpt-4o";

// Base-URL override for the LLM API. Production code never sets this; the
// integration tests in `tests/llm_api.rs` and `tests/cli.rs` use it to point
// the OpenAI-compatible client at a local mock HTTP server instead of the
// real OpenAI endpoint.
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
    client: OpenAiClient<OpenAIConfig>,
    model: &'static str,
    api_base: String,
}

fn resolve_api_config(openai_api_key: Option<&str>) -> Result<ApiConfig> {
    let openai = openai_api_key.filter(|s| !s.is_empty());

    let (base, key, model) = if let Some(key) = openai {
        (OPENAI_BASE, key, OPENAI_MODEL)
    } else {
        bail!("You must specify an OpenAI API key.")
    };

    let base = llm_api_base_override().unwrap_or_else(|| base.to_string());
    let config = OpenAIConfig::new().with_api_base(&base).with_api_key(key);
    Ok(ApiConfig {
        client: OpenAiClient::with_config(config),
        model,
        api_base: base,
    })
}

/// The inference backend selected for a request.
enum Provider {
    /// OpenAI, spoken over the OpenAI chat-completions API.
    OpenAiCompatible(Box<ApiConfig>),
    /// The GitHub Copilot CLI, with an optional explicit path to the binary.
    Copilot { cli_path: Option<PathBuf> },
}

/// Decide which inference provider to use. An OpenAI API key, when present,
/// takes precedence (handled by [`resolve_api_config`]). Otherwise we fall
/// back to the GitHub Copilot CLI.
fn resolve_provider(
    openai_api_key: Option<&str>,
    copilot_cli_path: Option<&Path>,
) -> Result<Provider> {
    let has_openai = openai_api_key.is_some_and(|s| !s.is_empty());

    if has_openai {
        Ok(Provider::OpenAiCompatible(Box::new(resolve_api_config(
            openai_api_key,
        )?)))
    } else {
        Ok(Provider::Copilot {
            cli_path: copilot_cli_path.map(Path::to_path_buf),
        })
    }
}

fn run_blocking<F: std::future::Future<Output = Result<String>>>(future: F) -> Result<String> {
    use std::sync::OnceLock;
    use tokio::runtime::{Handle, Runtime};

    // If we're already inside a Tokio runtime, reuse its handle rather than
    // building a new runtime (which would either panic or waste resources).
    if let Ok(handle) = Handle::try_current() {
        return tokio::task::block_in_place(|| handle.block_on(future));
    }

    // Otherwise build a single shared runtime lazily and reuse it across
    // calls. Building a fresh runtime per LLM request adds substantial
    // overhead in bulk flows (e.g. submitting a directory of receipts).
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    let runtime = match RUNTIME.get() {
        Some(rt) => rt,
        None => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .context("Failed to build Tokio runtime for LLM call")?;
            RUNTIME.get_or_init(|| rt)
        }
    };
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

// ---------------------------------------------------------------------------
// GitHub Copilot CLI inference
// ---------------------------------------------------------------------------

/// Search `PATH` for the `copilot` CLI binary. The SDK's auto-resolution does
/// not scan `PATH` (only `COPILOT_CLI_PATH` and the bundled binary), so we do
/// it ourselves to support a "just works" experience when the CLI is installed.
fn detect_copilot_cli() -> Option<PathBuf> {
    let names: &[&str] = if cfg!(windows) {
        &["copilot.exe", "copilot"]
    } else {
        &["copilot"]
    };
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        for name in names {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Run a single-shot prompt through the GitHub Copilot CLI and return the
/// assistant's final text response. An optional base64-encoded image is sent
/// as an attachment for vision inference.
async fn copilot_complete(
    cli_path: Option<&Path>,
    prompt: String,
    image: Option<(String, String)>,
) -> Result<String> {
    let program = match cli_path {
        Some(path) => CliProgram::Path(path.to_path_buf()),
        None => detect_copilot_cli().map_or(CliProgram::Resolve, CliProgram::Path),
    };

    if is_verbose() {
        eprintln!("[verbose] > GitHub Copilot CLI inference (program: {program:?})");
    }

    let mut options = ClientOptions::default();
    options.program = program;

    let client = CopilotClient::start(options).await.context(
        "Failed to start the GitHub Copilot CLI. Ensure the `copilot` CLI is installed and on your PATH, pass --copilot-cli-path, or set COPILOT_CLI_PATH.",
    )?;

    let result = copilot_run_session(&client, prompt, image).await;

    // Always attempt to shut the CLI process down cleanly, regardless of
    // whether the request succeeded.
    let _ = client.stop().await;

    result
}

async fn copilot_run_session(
    client: &CopilotClient,
    prompt: String,
    image: Option<(String, String)>,
) -> Result<String> {
    let session = client
        .create_session(SessionConfig::default().approve_all_permissions())
        .await
        .context("Failed to create a GitHub Copilot session")?;

    let message = MessageOptions::new(prompt).with_wait_timeout(Duration::from_secs(120));
    let message = match image {
        Some((data, mime_type)) => message.with_attachments(vec![Attachment::Blob {
            data,
            mime_type,
            display_name: Some("receipt".to_string()),
        }]),
        None => message,
    };

    let event = session
        .send_and_wait(message)
        .await
        .context("The GitHub Copilot inference request failed")?;

    let _ = session.disconnect().await;

    let response = event
        .and_then(|e| {
            e.data
                .get("content")
                .and_then(|c| c.as_str())
                .map(str::to_string)
        })
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| anyhow!("GitHub Copilot returned an empty response."))?;

    if is_verbose() {
        eprintln!("[verbose] < GitHub Copilot response: {response}");
    }

    Ok(response)
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
    copilot_cli_path: Option<&Path>,
) -> Result<InferredCategoryAndBenefit> {
    let provider = resolve_provider(openai_api_key, copilot_cli_path)?;

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

    let response = match &provider {
        Provider::OpenAiCompatible(config) => {
            let messages = vec![user_text_message(prompt)?];
            run_blocking(call_chat_completion(config, messages))?
        }
        Provider::Copilot { cli_path } => {
            run_blocking(copilot_complete(cli_path.as_deref(), prompt, None))?
        }
    };
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
    copilot_cli_path: Option<&Path>,
) -> Result<ReceiptInferenceResult> {
    let provider = resolve_provider(openai_api_key, copilot_cli_path)?;

    let image_path = convert_to_image_if_needed(receipt_path)?;
    let image_b64 = encode_image_to_base64(&image_path)?;
    let mime_type = image_mime_type(&image_path);
    // If we converted the receipt to a temporary JPEG, remove it now that
    // it's been encoded so we don't leak files into the temp directory.
    if image_path != receipt_path {
        let _ = std::fs::remove_file(&image_path);
    }

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

    let raw = match &provider {
        Provider::OpenAiCompatible(config) => {
            let data_url = format!("data:{mime_type};base64,{image_b64}");
            let messages = vec![user_text_and_image_message(prompt, data_url)?];
            run_blocking(call_chat_completion(config, messages))?
        }
        Provider::Copilot { cli_path } => run_blocking(copilot_complete(
            cli_path.as_deref(),
            prompt,
            Some((image_b64, mime_type)),
        ))?,
    };

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
    //
    // Use a uniquely-named temp file (via the `tempfile` crate) so concurrent
    // or repeated conversions (e.g. multiple receipts with the same filename
    // stem in a bulk flow) don't overwrite each other's output.
    let stem = receipt_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("receipt");
    let named = tempfile::Builder::new()
        .prefix(&format!("formanator-{stem}-"))
        .suffix(".jpg")
        .rand_bytes(12)
        .tempfile()
        .context("Failed to create temporary file for converted PDF receipt")?;
    // Keep the path but drop the file handle so `gm convert` can write to it.
    let (_file, output) = named
        .keep()
        .context("Failed to persist temporary file for converted PDF receipt")?;

    let status = Command::new("gm")
        .args(["convert", "-density", "100", "-resize", "2000x2000"])
        .arg(format!("{}[0]", receipt_path.display()))
        .arg(&output)
        .status();

    match status {
        Ok(s) if s.success() && output.exists() => Ok(output),
        Ok(s) => {
            let _ = std::fs::remove_file(&output);
            Err(anyhow!(
                "Failed to convert PDF receipt at {} to a JPEG: `gm convert` exited with {}. Please ensure GraphicsMagick and Ghostscript are installed (e.g. `brew install graphicsmagick ghostscript` on macOS, or `apt install graphicsmagick ghostscript` on Debian/Ubuntu), or use a JPEG/PNG receipt instead.",
                receipt_path.display(),
                s
            ))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let _ = std::fs::remove_file(&output);
            Err(anyhow!(
                "Failed to convert PDF receipt at {}: the GraphicsMagick `gm` command was not found on your PATH. Please install GraphicsMagick and Ghostscript (e.g. `brew install graphicsmagick ghostscript` on macOS, or `apt install graphicsmagick ghostscript` on Debian/Ubuntu), or use a JPEG/PNG receipt instead.",
                receipt_path.display()
            ))
        }
        Err(e) => {
            let _ = std::fs::remove_file(&output);
            Err(anyhow!(
                "Failed to invoke `gm convert` to convert PDF receipt at {}: {e}. Please ensure GraphicsMagick and Ghostscript are installed, or use a JPEG/PNG receipt instead.",
                receipt_path.display()
            ))
        }
    }
}

fn encode_image_to_base64(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read image file at {}", path.display()))?;
    Ok(BASE64.encode(bytes))
}

/// Best-effort MIME type for an image based on its file extension, defaulting
/// to `image/jpeg` (PDF receipts are converted to JPEG before this is called).
fn image_mime_type(path: &Path) -> String {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("png") => "image/png",
        Some("heic") => "image/heic",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        _ => "image/jpeg",
    }
    .to_string()
}
