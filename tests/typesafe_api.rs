//! Integration tests for Jev category inference through the TypeSafe API.

use formanator::category_inference::{
    CategoryInferenceOptions, CategoryInferenceSource, infer_category_and_benefit,
};
use formanator::cli::CategoryProvider;
use formanator::forma::{Benefit, BenefitWithCategories, Category};
use formanator::llm::set_llm_api_base;
use formanator::typesafe::{infer_category_and_benefit as infer_with_jev, set_api_base};
use httpmock::prelude::*;
use serial_test::serial;

#[path = "common/mod.rs"]
mod common;
use common::fixture;

struct ApiBaseGuard;

impl ApiBaseGuard {
    fn new(typesafe_base: &str, llm_base: Option<&str>) -> Self {
        set_api_base(Some(typesafe_base.to_string()));
        set_llm_api_base(llm_base.map(str::to_string));
        Self
    }
}

impl Drop for ApiBaseGuard {
    fn drop(&mut self) {
        set_api_base(None);
        set_llm_api_base(None);
    }
}

fn fixture_benefits_with_categories() -> Vec<BenefitWithCategories> {
    let learning_id = "learning-wallet".to_string();
    let wellness_id = "wellness-wallet".to_string();

    vec![
        BenefitWithCategories {
            benefit: Benefit {
                id: learning_id.clone(),
                name: "Flexible Reimbursement Account".to_string(),
                remaining_amount: 200.0,
                remaining_amount_currency: "GBP".to_string(),
            },
            categories: vec![Category {
                category_id: "education".to_string(),
                category_name: "Education".to_string(),
                subcategory_name: "university_program".to_string(),
                subcategory_value: "university_program".to_string(),
                subcategory_alias: Some("University Program".to_string()),
                benefit_id: learning_id,
            }],
        },
        BenefitWithCategories {
            benefit: Benefit {
                id: wellness_id.clone(),
                name: "Wellness and Lifestyle".to_string(),
                remaining_amount: 500.0,
                remaining_amount_currency: "GBP".to_string(),
            },
            categories: vec![Category {
                category_id: "fitness".to_string(),
                category_name: "Fitness".to_string(),
                subcategory_name: "gym_membership".to_string(),
                subcategory_value: "gym_membership".to_string(),
                subcategory_alias: Some("Gym Membership".to_string()),
                benefit_id: wellness_id,
            }],
        },
    ]
}

#[test]
#[serial]
fn jev_maps_a_choice_back_to_the_exact_benefit_and_category() {
    let server = MockServer::start();
    let _guard = ApiBaseGuard::new(&server.base_url(), None);
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v1/systemone")
            .header("authorization", "Bearer test-typesafe-key")
            .json_body(serde_json::json!({
                "state": {
                    "merchant": "Open University",
                    "description": "MBA tuition fee"
                },
                "model": "jev-latest",
                "questions": {
                    "category": {
                        "type": "choice",
                        "instructions": "Which Forma benefit and category pair best describes this expense claim?",
                        "criteria": {
                            "category_0_0": {
                                "benefit": "Flexible Reimbursement Account",
                                "category": "University Program"
                            },
                            "category_1_0": {
                                "benefit": "Wellness and Lifestyle",
                                "category": "Gym Membership"
                            },
                            "no_match": "None of the listed benefit and category pairs appropriately describes the purchase."
                        }
                    }
                }
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(serde_json::json!({
                "model": "jev-1.13.0",
                "answers": {
                    "category": {
                        "type": "choice",
                        "choice": "category_0_0",
                        "confidence": 0.92,
                        "probabilities": {
                            "category_0_0": 0.92,
                            "category_1_0": 0.07,
                            "no_match": 0.01
                        }
                    }
                },
                "usage": {"input_tokens": 100, "output_tokens": 10}
            }));
    });

    let result = infer_with_jev(
        "Open University",
        "MBA tuition fee",
        &fixture_benefits_with_categories(),
        Some("test-typesafe-key"),
    )
    .expect("Jev inference should succeed");

    mock.assert();
    assert_eq!(result.confidence, 0.92);
    let inferred = result.inferred.expect("category should match");
    assert_eq!(inferred.benefit, "Flexible Reimbursement Account");
    assert_eq!(inferred.category, "University Program");
    assert_eq!(inferred.source, CategoryInferenceSource::Jev);
}

#[test]
#[serial]
fn low_confidence_jev_result_falls_back_to_the_configured_llm() {
    let typesafe_server = MockServer::start();
    let llm_server = MockServer::start();
    let _guard = ApiBaseGuard::new(&typesafe_server.base_url(), Some(&llm_server.base_url()));

    let jev_mock = typesafe_server.mock(|when, then| {
        when.method(POST).path("/v1/systemone");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(serde_json::json!({
                "model": "jev-1.13.0",
                "answers": {
                    "category": {
                        "type": "choice",
                        "choice": "category_1_0",
                        "confidence": 0.25,
                        "probabilities": {
                            "category_0_0": 0.35,
                            "category_1_0": 0.40,
                            "no_match": 0.25
                        }
                    }
                },
                "usage": {"input_tokens": 100, "output_tokens": 10}
            }));
    });
    let llm_mock = llm_server.mock(|when, then| {
        when.method(POST).path("/chat/completions");
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("llm_category_inference_response.json"));
    });

    let result = infer_category_and_benefit(
        "Open University",
        "MBA tuition fee",
        &fixture_benefits_with_categories(),
        &CategoryInferenceOptions {
            provider: CategoryProvider::Jev,
            typesafe_api_key: Some("test-typesafe-key"),
            openai_api_key: Some("test-openai-key"),
            openai_base_url: None,
            openai_model: None,
            copilot_cli_path: None,
        },
    )
    .expect("low-confidence Jev result should fall back to the LLM");

    jev_mock.assert();
    llm_mock.assert();
    assert_eq!(result.benefit, "Flexible Reimbursement Account");
    assert_eq!(result.category, "University Program");
    assert_eq!(result.source, CategoryInferenceSource::Llm);
}

#[test]
#[serial]
fn jev_requires_an_api_key() {
    let server = MockServer::start();
    let _guard = ApiBaseGuard::new(&server.base_url(), None);

    let error = infer_with_jev(
        "Merchant",
        "Description",
        &fixture_benefits_with_categories(),
        None,
    )
    .expect_err("missing API key should fail");

    assert!(format!("{error}").contains("TYPESAFE_API_KEY"), "{error}");
}

#[test]
#[serial]
fn jev_can_report_that_no_category_matches() {
    let server = MockServer::start();
    let _guard = ApiBaseGuard::new(&server.base_url(), None);
    server.mock(|when, then| {
        when.method(POST).path("/v1/systemone");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(serde_json::json!({
                "model": "jev-1.13.0",
                "answers": {
                    "category": {
                        "type": "choice",
                        "choice": "no_match",
                        "confidence": 0.91,
                        "probabilities": {
                            "category_0_0": 0.04,
                            "category_1_0": 0.05,
                            "no_match": 0.91
                        }
                    }
                },
                "usage": {"input_tokens": 100, "output_tokens": 10}
            }));
    });

    let result = infer_with_jev(
        "Unknown Merchant",
        "Unclear purchase",
        &fixture_benefits_with_categories(),
        Some("test-typesafe-key"),
    )
    .expect("no-match is a valid Jev response");

    assert_eq!(result.confidence, 0.91);
    assert!(result.inferred.is_none());
}
