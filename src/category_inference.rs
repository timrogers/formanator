//! Category and benefit inference with selectable providers.

use std::path::Path;

use anyhow::Result;

use crate::cli::CategoryProvider;
use crate::forma::BenefitWithCategories;
use crate::{llm, typesafe};

const DEFAULT_JEV_MIN_CONFIDENCE: f64 = 0.5;

#[derive(Debug, Clone)]
pub struct InferredCategoryAndBenefit {
    pub category: String,
    pub benefit: String,
}

pub struct CategoryInferenceOptions<'a> {
    pub provider: CategoryProvider,
    pub typesafe_api_key: Option<&'a str>,
    pub openai_api_key: Option<&'a str>,
    pub openai_base_url: Option<&'a str>,
    pub openai_model: Option<&'a str>,
    pub copilot_cli_path: Option<&'a Path>,
}

pub fn infer_category_and_benefit(
    merchant: &str,
    description: &str,
    benefits_with_categories: &[BenefitWithCategories],
    options: &CategoryInferenceOptions<'_>,
) -> Result<InferredCategoryAndBenefit> {
    if matches!(options.provider, CategoryProvider::Jev) {
        let decision = typesafe::infer_category_and_benefit(
            merchant,
            description,
            benefits_with_categories,
            options.typesafe_api_key,
        )?;

        if let Some(inferred) = decision.inferred
            && decision.confidence >= DEFAULT_JEV_MIN_CONFIDENCE
        {
            return Ok(inferred);
        }

        eprintln!(
            "Jev was uncertain about the category (confidence {:.2}); falling back to LLM inference.",
            decision.confidence
        );
    }

    llm::infer_category_and_benefit(
        merchant,
        description,
        benefits_with_categories,
        options.openai_api_key,
        options.openai_base_url,
        options.openai_model,
        options.copilot_cli_path,
    )
}
