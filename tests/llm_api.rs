//! Integration tests for [`formanator::llm`].
//!
//! These tests drive the LLM inference entry points against a local mock
//! HTTP server (via [`httpmock`]) that impersonates the OpenAI-compatible
//! `/chat/completions` endpoint. Each test points the client at the mock
//! server through [`formanator::llm::LlmOptions::openai_base_url`], so no real
//! network requests are ever made.

use std::io::Write;

use formanator::forma::{Benefit, BenefitWithCategories, Category};
use formanator::llm::{LlmOptions, infer_all_from_receipt, infer_category_and_benefit};
use httpmock::prelude::*;
use serial_test::serial;

#[path = "common/mod.rs"]
mod common;
use common::fixture;

/// Build [`LlmOptions`] that use the OpenAI-compatible backend with a test API
/// key, pointed at the given mock-server base URL.
fn openai_options(base_url: &str) -> LlmOptions<'_> {
    LlmOptions {
        openai_api_key: Some("test-openai-key"),
        openai_base_url: Some(base_url),
        ..Default::default()
    }
}

/// A minimal valid-looking JPEG file at a fixed path. Returned as a
/// [`tempfile::NamedTempFile`] with the `.jpg` suffix so
/// [`infer_all_from_receipt`] skips its PDF-to-JPEG conversion branch.
fn fake_jpeg_receipt() -> tempfile::NamedTempFile {
    let mut f = tempfile::Builder::new()
        .suffix(".jpg")
        .tempfile()
        .expect("tempfile");
    // SOI + JFIF marker + EOI. Enough for the inference code to read it,
    // base64-encode it and post it as a data URL.
    f.write_all(&[
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0xFF, 0xD9,
    ])
    .expect("write fake jpeg");
    f
}

/// Build a synthetic set of benefits + categories that matches the
/// `benefit` / `category` values returned by the committed LLM fixtures.
fn fixture_benefits_with_categories() -> Vec<BenefitWithCategories> {
    let fra_id = "wallet-0001-aaaa-aaaa-aaaa-aaaaaaaaaaaa".to_string();
    let fra = BenefitWithCategories {
        benefit: Benefit {
            id: fra_id.clone(),
            name: "Flexible Reimbursement Account".to_string(),
            remaining_amount: 200.0,
            remaining_amount_currency: "GBP".to_string(),
        },
        categories: vec![
            // Raw subcategory (no alias) — the text-only fixture response
            // `"University Program"` matches this row via the alias field.
            Category {
                category_id: "cat-education".to_string(),
                category_name: "Education".to_string(),
                subcategory_name: "university_program".to_string(),
                subcategory_value: "university_program".to_string(),
                subcategory_alias: Some("University Program".to_string()),
                benefit_id: fra_id.clone(),
            },
            // A more descriptive alias for the same subcategory — the
            // vision fixture response `"Personal University Program & class
            // materials"` matches this row.
            Category {
                category_id: "cat-education".to_string(),
                category_name: "Education".to_string(),
                subcategory_name: "university_program".to_string(),
                subcategory_value: "university_program".to_string(),
                subcategory_alias: Some(
                    "Personal University Program & class materials".to_string(),
                ),
                benefit_id: fra_id.clone(),
            },
        ],
    };

    // A second benefit with a single unrelated category, to make sure the
    // LLM-response matching code discriminates by benefit correctly.
    let wellness_id = "wallet-0002-aaaa-aaaa-aaaa-aaaaaaaaaaaa".to_string();
    let wellness = BenefitWithCategories {
        benefit: Benefit {
            id: wellness_id.clone(),
            name: "Wellness and Lifestyle".to_string(),
            remaining_amount: 750.5,
            remaining_amount_currency: "GBP".to_string(),
        },
        categories: vec![Category {
            category_id: "cat-fitness".to_string(),
            category_name: "Fitness".to_string(),
            subcategory_name: "gym_membership".to_string(),
            subcategory_value: "gym_membership".to_string(),
            subcategory_alias: Some("Gym Membership".to_string()),
            benefit_id: wellness_id,
        }],
    };

    vec![fra, wellness]
}

// ---------------------------------------------------------------------------
// infer_category_and_benefit (text-only chat completion)
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn infer_category_and_benefit_resolves_llm_response_to_benefit() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/chat/completions")
            .header("authorization", "Bearer test-openai-key");
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("llm_category_inference_response.json"));
    });

    let bwcs = fixture_benefits_with_categories();
    let result = infer_category_and_benefit(
        "Open University",
        "MBA tuition fee",
        &bwcs,
        &openai_options(&server.base_url()),
    )
    .expect("infer_category_and_benefit should succeed");

    mock.assert();
    // The fixture's chat completion content is `"University Program"` — the
    // resolver should map that back to the FRA benefit.
    assert_eq!(result.category, "University Program");
    assert_eq!(result.benefit, "Flexible Reimbursement Account");
}

#[test]
#[serial]
fn infer_category_and_benefit_errors_when_llm_returns_unknown_category() {
    let server = MockServer::start();
    // Build a chat-completion response whose `content` is not in the
    // category list we pass in.
    let body = serde_json::json!({
        "id": "chatcmpl-test-bad-0001",
        "object": "chat.completion",
        "created": 1_745_000_000,
        "model": "gpt-4.1-2025-04-14",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "Total Nonsense"},
            "finish_reason": "stop",
        }],
        "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2},
    })
    .to_string();
    server.mock(|when, then| {
        when.method(POST).path("/chat/completions");
        then.status(200)
            .header("content-type", "application/json")
            .body(body);
    });

    let bwcs = fixture_benefits_with_categories();
    let err = infer_category_and_benefit(
        "Merchant",
        "Description",
        &bwcs,
        &openai_options(&server.base_url()),
    )
    .expect_err("should reject unknown category");
    assert!(
        format!("{err}").contains("wasn't a valid category"),
        "{err}"
    );
}

// ---------------------------------------------------------------------------
// infer_all_from_receipt (vision chat completion)
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn infer_all_from_receipt_parses_structured_json_response() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/chat/completions")
            .header("authorization", "Bearer test-openai-key");
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("llm_receipt_inference_response.json"));
    });

    let receipt = fake_jpeg_receipt();
    let bwcs = fixture_benefits_with_categories();
    let result = infer_all_from_receipt(receipt.path(), &bwcs, &openai_options(&server.base_url()))
        .expect("infer_all_from_receipt should succeed");

    mock.assert();
    assert_eq!(result.amount, "3670.00");
    assert_eq!(result.merchant, "Open University");
    assert_eq!(result.purchase_date, "2026-03-24");
    assert_eq!(result.description, "MBA module registration fee");
    assert_eq!(result.benefit, "Flexible Reimbursement Account");
    assert_eq!(
        result.category,
        "Personal University Program & class materials"
    );
}

#[test]
#[serial]
fn infer_all_from_receipt_rejects_invalid_date_format() {
    let server = MockServer::start();
    // The model returns a JSON payload whose shape is valid but whose
    // `purchaseDate` is not YYYY-MM-DD. The validator must reject it.
    let inner = serde_json::json!({
        "amount": "10.00",
        "merchant": "Open University",
        "purchaseDate": "24/03/2026",
        "description": "MBA module registration fee",
        "benefit": "Flexible Reimbursement Account",
        "category": "University Program",
    })
    .to_string();
    let body = serde_json::json!({
        "id": "chatcmpl-test-bad-date",
        "object": "chat.completion",
        "created": 1_745_000_000,
        "model": "gpt-4.1-2025-04-14",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": inner},
            "finish_reason": "stop",
        }],
        "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2},
    })
    .to_string();
    server.mock(|when, then| {
        when.method(POST).path("/chat/completions");
        then.status(200).body(body);
    });

    let receipt = fake_jpeg_receipt();
    let bwcs = fixture_benefits_with_categories();
    let err = infer_all_from_receipt(receipt.path(), &bwcs, &openai_options(&server.base_url()))
        .expect_err("should reject bad date");
    assert!(format!("{err}").contains("invalid date format"), "{err}");
}

#[test]
#[serial]
fn infer_all_from_receipt_strips_markdown_code_fences() {
    // Some models wrap the JSON in ```json ... ``` despite the prompt.
    // The parser is expected to strip those fences.
    let server = MockServer::start();
    let inner = "```json\n{\n  \"amount\": \"42.00\",\n  \"merchant\": \"Open University\",\n  \"purchaseDate\": \"2026-01-15\",\n  \"description\": \"Course fee\",\n  \"benefit\": \"Flexible Reimbursement Account\",\n  \"category\": \"University Program\"\n}\n```";
    let body = serde_json::json!({
        "id": "chatcmpl-test-fenced",
        "object": "chat.completion",
        "created": 1_745_000_000,
        "model": "gpt-4.1-2025-04-14",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": inner},
            "finish_reason": "stop",
        }],
        "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2},
    })
    .to_string();
    server.mock(|when, then| {
        when.method(POST).path("/chat/completions");
        then.status(200).body(body);
    });

    let receipt = fake_jpeg_receipt();
    let bwcs = fixture_benefits_with_categories();
    let result = infer_all_from_receipt(receipt.path(), &bwcs, &openai_options(&server.base_url()))
        .expect("should parse fenced JSON");
    assert_eq!(result.amount, "42.00");
    assert_eq!(result.category, "University Program");
}

// ---------------------------------------------------------------------------
// LLM key validation
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn infer_category_and_benefit_falls_back_to_copilot_and_fails_with_bogus_cli_path() {
    // With no OpenAI key, inference falls back to the GitHub Copilot CLI.
    // Pointing at a non-existent CLI binary must surface an error rather than
    // silently succeeding — this keeps the test hermetic even in environments
    // where a real `copilot` binary is on PATH.
    let bwcs = fixture_benefits_with_categories();
    let bogus = std::path::Path::new("/no/such/copilot-cli-binary");
    let options = LlmOptions {
        copilot_cli_path: Some(bogus),
        ..Default::default()
    };
    let err = infer_category_and_benefit("m", "d", &bwcs, &options)
        .expect_err("should fail to launch the bogus Copilot CLI");
    // Just assert that an error was produced; the exact message comes from the
    // SDK / OS and is not stable across platforms.
    assert!(!format!("{err}").is_empty(), "{err}");
}
