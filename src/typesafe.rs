//! TypeSafe System One API client for Jev category inference.

use std::collections::BTreeMap;
use std::sync::RwLock;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use reqwest::StatusCode;
use reqwest::blocking::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::category_inference::{CategoryInferenceSource, InferredCategoryAndBenefit};
use crate::forma::BenefitWithCategories;
use crate::verbose::is_enabled as is_verbose;

const DEFAULT_API_BASE: &str = "https://api.typesafe.ai";
const DEFAULT_MODEL: &str = "jev-latest";
const MAX_CHOICE_OPTIONS: usize = 255;
const MAX_RETRIES: usize = 2;

static API_BASE: RwLock<Option<String>> = RwLock::new(None);

#[doc(hidden)]
pub fn set_api_base(base: Option<String>) {
    if let Ok(mut guard) = API_BASE.write() {
        *guard = base;
    }
}

fn api_base() -> String {
    API_BASE
        .read()
        .ok()
        .and_then(|guard| guard.clone())
        .unwrap_or_else(|| DEFAULT_API_BASE.to_string())
}

#[derive(Debug)]
pub struct JevCategoryDecision {
    pub inferred: Option<InferredCategoryAndBenefit>,
    pub confidence: f64,
}

#[derive(Serialize)]
struct SystemOneRequest {
    state: Value,
    model: &'static str,
    questions: BTreeMap<&'static str, ChoiceQuestion>,
}

#[derive(Serialize)]
struct ChoiceQuestion {
    #[serde(rename = "type")]
    question_type: &'static str,
    instructions: &'static str,
    criteria: BTreeMap<String, Value>,
}

#[derive(Deserialize)]
struct SystemOneResponse {
    answers: BTreeMap<String, ChoiceAnswer>,
}

#[derive(Deserialize)]
struct ChoiceAnswer {
    #[serde(rename = "type")]
    answer_type: String,
    choice: String,
    confidence: f64,
}

struct CategoryOption {
    benefit: String,
    category: String,
}

pub fn infer_category_and_benefit(
    merchant: &str,
    description: &str,
    benefits_with_categories: &[BenefitWithCategories],
    api_key: Option<&str>,
) -> Result<JevCategoryDecision> {
    let api_key = api_key
        .filter(|key| !key.trim().is_empty())
        .ok_or_else(|| {
            anyhow!(
                "You must set TYPESAFE_API_KEY or pass --typesafe-api-key when using --category-provider=jev."
            )
        })?;

    let mut options = BTreeMap::new();
    let mut criteria = BTreeMap::new();
    for (index, benefit) in benefits_with_categories.iter().enumerate() {
        for (category_index, category) in benefit.categories.iter().enumerate() {
            let key = format!("category_{index}_{category_index}");
            let category_name = category
                .subcategory_alias
                .clone()
                .unwrap_or_else(|| category.subcategory_name.clone());
            criteria.insert(
                key.clone(),
                json!({
                    "benefit": benefit.benefit.name,
                    "category": category_name,
                }),
            );
            options.insert(
                key,
                CategoryOption {
                    benefit: benefit.benefit.name.clone(),
                    category: category_name,
                },
            );
        }
    }

    if options.is_empty() {
        bail!("Forma returned no categories for Jev to choose from.");
    }
    if options.len() + 1 > MAX_CHOICE_OPTIONS {
        bail!(
            "Jev supports at most {} category choices, but Forma returned {}.",
            MAX_CHOICE_OPTIONS - 1,
            options.len()
        );
    }

    criteria.insert(
        "no_match".to_string(),
        json!(
            "None of the listed benefit and category pairs appropriately describes the purchase."
        ),
    );

    let request = SystemOneRequest {
        state: json!({
            "merchant": merchant,
            "description": description,
        }),
        model: DEFAULT_MODEL,
        questions: BTreeMap::from([(
            "category",
            ChoiceQuestion {
                question_type: "choice",
                instructions: "Which Forma benefit and category pair best describes this expense claim?",
                criteria,
            },
        )]),
    };

    let base = api_base();
    let url = format!("{}/v1/systemone", base.trim_end_matches('/'));
    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .user_agent(concat!("formanator/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("Failed to build the TypeSafe API client")?;

    let response = send_with_retries(&client, &url, api_key, &request)?;
    let status = response.status();
    let body = response.text().unwrap_or_default();

    if is_verbose() {
        eprintln!(
            "[verbose] < TypeSafe {} {}",
            status.as_u16(),
            status.canonical_reason().unwrap_or("")
        );
        eprintln!("[verbose] < Body: {body}");
    }

    if !status.is_success() {
        bail!("TypeSafe API returned HTTP {}: {}", status.as_u16(), body);
    }

    let parsed: SystemOneResponse =
        serde_json::from_str(&body).context("Failed to parse the TypeSafe API response")?;
    let answer = parsed
        .answers
        .get("category")
        .ok_or_else(|| anyhow!("TypeSafe API response did not include the category answer"))?;
    if answer.answer_type != "choice" {
        bail!(
            "TypeSafe API returned an unexpected answer type: {}",
            answer.answer_type
        );
    }
    if !(0.0..=1.0).contains(&answer.confidence) {
        bail!(
            "TypeSafe API returned an invalid confidence value: {}",
            answer.confidence
        );
    }

    let inferred = if answer.choice == "no_match" {
        None
    } else {
        let selected = options.get(&answer.choice).ok_or_else(|| {
            anyhow!(
                "TypeSafe API selected an unknown category option: {}",
                answer.choice
            )
        })?;
        Some(InferredCategoryAndBenefit {
            category: selected.category.clone(),
            benefit: selected.benefit.clone(),
            source: CategoryInferenceSource::Jev,
        })
    };

    Ok(JevCategoryDecision {
        inferred,
        confidence: answer.confidence,
    })
}

fn send_with_retries(
    client: &Client,
    url: &str,
    api_key: &str,
    request: &SystemOneRequest,
) -> Result<Response> {
    for attempt in 0..=MAX_RETRIES {
        if is_verbose() {
            eprintln!("[verbose] > POST {url}");
            match serde_json::to_string(request) {
                Ok(body) => eprintln!("[verbose] > Body: {body}"),
                Err(error) => {
                    eprintln!("[verbose] > Body: <failed to serialize: {error}>")
                }
            }
        }

        let response = client
            .post(url)
            .bearer_auth(api_key)
            .json(request)
            .send()
            .context("Failed to call the TypeSafe API")?;

        let status = response.status();
        if (status != StatusCode::TOO_MANY_REQUESTS && status.as_u16() != 529)
            || attempt == MAX_RETRIES
        {
            return Ok(response);
        }

        thread::sleep(Duration::from_millis(250 * (1 << attempt)));
    }

    unreachable!("retry loop always returns a response")
}
