//! Integration tests that drive the Forma API client (`formanator::forma`)
//! against a local mock HTTP server (via [`httpmock`]) using the JSON fixtures
//! committed under `tests/fixtures/`.
//!
//! The tests are serialised because they share a process-global API base URL
//! override (`formanator::forma::set_api_base`).

use std::path::PathBuf;

use formanator::forma::{
    ClaimsFilter, CreateClaimOptions, create_claim, exchange_id_and_tk_for_access_token,
    get_benefits, get_benefits_with_categories, get_categories_for_benefit_name, get_claims_list,
    request_magic_link, set_api_base,
};
use httpmock::prelude::*;
use serial_test::serial;

#[path = "common/mod.rs"]
mod common;
use common::{fixture, make_fake_receipt as fake_receipt};

const TOKEN: &str = "test-access-token-abc123";

/// RAII guard that sets the API base for the duration of a test and clears it
/// when dropped.
struct ApiBaseGuard;

impl ApiBaseGuard {
    fn new(base: &str) -> Self {
        set_api_base(Some(base.to_string()));
        Self
    }
}

impl Drop for ApiBaseGuard {
    fn drop(&mut self) {
        set_api_base(None);
    }
}

fn forma_server() -> (MockServer, ApiBaseGuard) {
    let server = MockServer::start();
    let guard = ApiBaseGuard::new(&server.base_url());
    (server, guard)
}

// ---------------------------------------------------------------------------
// get_benefits / get_categories_for_benefit_name / get_benefits_with_categories
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn get_benefits_returns_eligible_wallets_with_currency() {
    let (server, _guard) = forma_server();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/client/api/v3/settings/profile")
            .header("x-auth-token", TOKEN);
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("profile_response.json"));
    });

    let benefits = get_benefits(TOKEN).expect("should fetch benefits");
    mock.assert();

    // The fixture has 6 employee_wallets but only 3 are flagged as eligible;
    // the rest must be filtered out.
    assert_eq!(benefits.len(), 3, "ineligible wallets must be filtered out");
    let names: Vec<&str> = benefits.iter().map(|b| b.name.as_str()).collect();
    assert!(names.contains(&"Wellness and Lifestyle"));
    assert!(names.contains(&"Learning"));
    assert!(names.contains(&"Flexible Reimbursement Account"));
    assert!(!names.contains(&"Remote Life"));
    assert!(!names.contains(&"New Hire Home Office"));
    assert!(!names.contains(&"Gender Affirming HRA"));

    for b in &benefits {
        assert_eq!(b.remaining_amount_currency, "GBP");
    }
    let wellness = benefits
        .iter()
        .find(|b| b.name == "Wellness and Lifestyle")
        .unwrap();
    assert_eq!(wellness.id, "wallet-0002-aaaa-aaaa-aaaa-aaaaaaaaaaaa");
    assert!((wellness.remaining_amount - 750.5).abs() < 1e-9);
    let learning = benefits.iter().find(|b| b.name == "Learning").unwrap();
    assert!((learning.remaining_amount - 1250.0).abs() < 1e-9);
}

#[test]
#[serial]
fn get_benefits_returns_empty_when_no_eligible_wallets() {
    let (server, _guard) = forma_server();
    server.mock(|when, then| {
        when.method(GET).path("/client/api/v3/settings/profile");
        then.status(200)
            .body(fixture("profile_response_empty.json"));
    });

    let benefits = get_benefits(TOKEN).expect("should fetch");
    assert!(benefits.is_empty());
}

#[test]
#[serial]
fn get_benefits_surfaces_friendly_error_message_from_body() {
    let (server, _guard) = forma_server();
    server.mock(|when, then| {
        when.method(GET).path("/client/api/v3/settings/profile");
        then.status(401).body(fixture("error_invalid_jwt.json"));
    });

    let err = get_benefits(TOKEN).expect_err("should fail");
    let msg = format!("{err:#}");
    assert!(msg.contains("Forma access token is invalid"), "{msg}");
}

#[test]
#[serial]
fn get_categories_for_benefit_name_expands_aliases() {
    let (server, _guard) = forma_server();
    server.mock(|when, then| {
        when.method(GET).path("/client/api/v3/settings/profile");
        then.status(200).body(fixture("profile_response.json"));
    });

    let cats = get_categories_for_benefit_name(TOKEN, "Learning").expect("should fetch");

    // The Learning wallet in the fixture has 4 parent categories totalling
    // 15 subcategories. 8 of those subcategories declare an alias, each of
    // which produces an additional row in the flattened output:
    //   15 base rows + 8 alias rows = 23 rows.
    assert_eq!(cats.len(), 23);

    // Every row in this benefit must reference the Learning wallet id.
    for c in &cats {
        assert_eq!(c.benefit_id, "wallet-0003-aaaa-aaaa-aaaa-aaaaaaaaaaaa");
    }

    // The original (non-aliased) row exists for every subcategory.
    assert!(
        cats.iter()
            .any(|c| c.subcategory_name == "Book" && c.subcategory_alias.is_none())
    );
    // Aliases produce additional rows with the alias populated.
    let aliases: Vec<&str> = cats
        .iter()
        .filter_map(|c| c.subcategory_alias.as_deref())
        .collect();
    assert!(aliases.contains(&"Book (personal development)"));
    assert!(aliases.contains(&"Personal Class Material/Supplies/Equipment"));
    assert!(aliases.contains(&"Personal University Program & class materials"));
}

#[test]
#[serial]
fn get_categories_for_benefit_name_errors_for_unknown_benefit() {
    let (server, _guard) = forma_server();
    server.mock(|when, then| {
        when.method(GET).path("/client/api/v3/settings/profile");
        then.status(200).body(fixture("profile_response.json"));
    });

    let err = get_categories_for_benefit_name(TOKEN, "Nonexistent").expect_err("should fail");
    assert!(format!("{err}").contains("Could not find benefit"));
}

#[test]
#[serial]
fn get_benefits_with_categories_combines_each_benefit_with_its_categories() {
    let (server, _guard) = forma_server();
    server.mock(|when, then| {
        when.method(GET).path("/client/api/v3/settings/profile");
        then.status(200).body(fixture("profile_response.json"));
    });

    let combined = get_benefits_with_categories(TOKEN).expect("should fetch");
    assert_eq!(combined.len(), 3);
    let wellness = combined
        .iter()
        .find(|b| b.benefit.name == "Wellness and Lifestyle")
        .unwrap();
    // Wellness and Lifestyle: 53 base subcategories + 4 alias rows = 57.
    assert_eq!(wellness.categories.len(), 57);
    let learning = combined
        .iter()
        .find(|b| b.benefit.name == "Learning")
        .unwrap();
    assert_eq!(learning.categories.len(), 23);
    assert!(
        learning
            .categories
            .iter()
            .any(|c| c.subcategory_name == "Book")
    );
}

// ---------------------------------------------------------------------------
// get_claims_list (pagination + filtering)
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn get_claims_list_paginates_until_a_partial_page() {
    let (server, _guard) = forma_server();
    let page0 = server.mock(|when, then| {
        when.method(GET)
            .path("/client/api/v2/claims")
            .query_param("page", "0");
        then.status(200).body(fixture("claims_list_page0.json"));
    });
    let page1 = server.mock(|when, then| {
        when.method(GET)
            .path("/client/api/v2/claims")
            .query_param("page", "1");
        then.status(200).body(fixture("claims_list_page1.json"));
    });

    let claims = get_claims_list(TOKEN, None).expect("should fetch");
    page0.assert();
    page1.assert();
    assert_eq!(claims.len(), 3);
    let ids: Vec<&str> = claims.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(
        ids,
        vec![
            "c1aa1111-1111-4111-8111-111111111111",
            "c2aa2222-2222-4222-8222-222222222222",
            "c3aa3333-3333-4333-8333-333333333333",
        ],
    );

    // Field projection from the nested `reimbursement` object.
    let in_progress_claim = claims
        .iter()
        .find(|c| c.id == "c2aa2222-2222-4222-8222-222222222222")
        .unwrap();
    assert_eq!(
        in_progress_claim.reimbursement_status.as_deref(),
        Some("in_progress")
    );
    assert_eq!(in_progress_claim.payout_status, None);
    assert_eq!(in_progress_claim.amount, Some(23.99));
    assert_eq!(
        in_progress_claim.reimbursement_vendor.as_deref(),
        Some("Amazon")
    );
}

#[test]
#[serial]
fn get_claims_list_in_progress_filter() {
    let (server, _guard) = forma_server();
    server.mock(|when, then| {
        when.method(GET).path("/client/api/v2/claims");
        then.status(200)
            .body(fixture("claims_list_in_progress.json"));
    });

    let claims = get_claims_list(TOKEN, Some(ClaimsFilter::InProgress)).expect("should fetch");
    let ids: Vec<&str> = claims.iter().map(|c| c.id.as_str()).collect();
    // ip_c1: top-level status in_progress; ip_c2: reimbursement.status in_progress.
    assert!(ids.contains(&"ip1a1111-1111-4111-8111-111111111111"));
    assert!(ids.contains(&"ip2a2222-2222-4222-8222-222222222222"));
    // ip_c3 is fully completed and must be filtered out.
    assert!(!ids.contains(&"ip3a3333-3333-4333-8333-333333333333"));
    assert_eq!(claims.len(), 2);
}

#[test]
#[serial]
fn get_claims_list_propagates_friendly_error() {
    let (server, _guard) = forma_server();
    server.mock(|when, then| {
        when.method(GET).path("/client/api/v2/claims");
        then.status(403).body(fixture("error_generic.json"));
    });

    let err = get_claims_list(TOKEN, None).expect_err("should fail");
    assert!(
        format!("{err}").contains("That benefit is not available"),
        "{err}"
    );
}

// ---------------------------------------------------------------------------
// request_magic_link / exchange_id_and_tk_for_access_token
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn request_magic_link_posts_email_and_succeeds() {
    let (server, _guard) = forma_server();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/client/auth/v2/login/magic")
            .header("content-type", "application/json")
            .json_body(serde_json::json!({ "email": "user@example.com" }));
        then.status(200).body(r#"{"success":true}"#);
    });

    request_magic_link("user@example.com").expect("should succeed");
    mock.assert();
}

#[test]
#[serial]
fn request_magic_link_fails_when_response_says_unsuccessful() {
    let (server, _guard) = forma_server();
    server.mock(|when, then| {
        when.method(POST).path("/client/auth/v2/login/magic");
        then.status(200).body(r#"{"success":false}"#);
    });
    let err = request_magic_link("user@example.com").expect_err("should fail");
    assert!(
        format!("{err}").contains("requesting a magic link"),
        "{err}"
    );
}

#[test]
#[serial]
fn exchange_id_and_tk_returns_auth_token_from_response() {
    let (server, _guard) = forma_server();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/client/auth/v2/login/magic")
            .query_param("id", "the-id")
            .query_param("tk", "the-tk")
            .query_param("return_token", "true");
        then.status(200)
            .body(fixture("magic_link_exchange_response.json"));
    });

    let token = exchange_id_and_tk_for_access_token("the-id", "the-tk").expect("should exchange");
    mock.assert();
    assert_eq!(token, common::FIXTURE_AUTH_TOKEN);
}

#[test]
#[serial]
fn exchange_id_and_tk_propagates_invalid_jwt_error() {
    let (server, _guard) = forma_server();
    server.mock(|when, then| {
        when.method(GET).path("/client/auth/v2/login/magic");
        then.status(401).body(fixture("error_invalid_jwt.json"));
    });
    let err = exchange_id_and_tk_for_access_token("x", "y").expect_err("should fail");
    assert!(
        format!("{err}").contains("Forma access token is invalid"),
        "{err}"
    );
}

// ---------------------------------------------------------------------------
// create_claim
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn create_claim_posts_multipart_form_and_succeeds_on_201() {
    let (server, _guard) = forma_server();
    let receipt = fake_receipt();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/client/api/v2/claims")
            .header("x-auth-token", TOKEN)
            // reqwest sets a multipart content-type with a boundary; assert the
            // textual form fields appear in the body — that's enough to be
            // confident we're constructing the multipart correctly.
            .body_includes("name=\"type\"")
            .body_includes("transaction")
            .body_includes("name=\"amount\"")
            .body_includes("25.99")
            .body_includes("name=\"transaction_date\"")
            .body_includes("2024-01-02")
            .body_includes("name=\"default_employee_wallet_id\"")
            .body_includes("wallet-lsa-1")
            .body_includes("name=\"category\"")
            .body_includes("cat-fitness")
            .body_includes("name=\"subcategory\"")
            .body_includes("gym_membership")
            .body_includes("name=\"reimbursement_vendor\"")
            .body_includes("FitClub")
            .body_includes("name=\"file[]\"");
        then.status(201)
            .body(fixture("create_claim_response_success.json"));
    });

    let opts = CreateClaimOptions {
        amount: "25.99".to_string(),
        merchant: "FitClub".to_string(),
        purchase_date: "2024-01-02".to_string(),
        description: "Monthly gym membership".to_string(),
        receipt_path: vec![receipt.path().to_path_buf()],
        access_token: TOKEN.to_string(),
        benefit_id: "wallet-lsa-1".to_string(),
        category_id: "cat-fitness".to_string(),
        subcategory_value: "gym_membership".to_string(),
        subcategory_alias: Some("Gym".to_string()),
    };
    create_claim(&opts).expect("should succeed");
    mock.assert();
}

#[test]
#[serial]
fn create_claim_fails_when_response_indicates_unsuccessful() {
    let (server, _guard) = forma_server();
    let receipt = fake_receipt();
    server.mock(|when, then| {
        when.method(POST).path("/client/api/v2/claims");
        then.status(201)
            .body(fixture("create_claim_response_unsuccessful.json"));
    });

    let opts = CreateClaimOptions {
        amount: "10.00".to_string(),
        merchant: "M".to_string(),
        purchase_date: "2024-01-01".to_string(),
        description: "D".to_string(),
        receipt_path: vec![receipt.path().to_path_buf()],
        access_token: TOKEN.to_string(),
        benefit_id: "wallet-lsa-1".to_string(),
        category_id: "cat-fitness".to_string(),
        subcategory_value: "gym_membership".to_string(),
        subcategory_alias: None,
    };
    let err = create_claim(&opts).expect_err("should fail");
    assert!(format!("{err}").contains("201 Created"), "{err}");
}

#[test]
#[serial]
fn create_claim_propagates_friendly_error_for_non_201_response() {
    let (server, _guard) = forma_server();
    let receipt = fake_receipt();
    server.mock(|when, then| {
        when.method(POST).path("/client/api/v2/claims");
        then.status(422).body(fixture("error_generic.json"));
    });

    let opts = CreateClaimOptions {
        amount: "10.00".to_string(),
        merchant: "M".to_string(),
        purchase_date: "2024-01-01".to_string(),
        description: "D".to_string(),
        receipt_path: vec![receipt.path().to_path_buf()],
        access_token: TOKEN.to_string(),
        benefit_id: "wallet-lsa-1".to_string(),
        category_id: "cat-fitness".to_string(),
        subcategory_value: "gym_membership".to_string(),
        subcategory_alias: None,
    };
    let err = create_claim(&opts).expect_err("should fail");
    assert!(
        format!("{err}").contains("That benefit is not available"),
        "{err}"
    );
}

#[test]
#[serial]
fn create_claim_errors_when_receipt_is_missing() {
    let (_server, _guard) = forma_server();
    let opts = CreateClaimOptions {
        amount: "10.00".to_string(),
        merchant: "M".to_string(),
        purchase_date: "2024-01-01".to_string(),
        description: "D".to_string(),
        receipt_path: vec![PathBuf::from("/nonexistent/receipt.jpg")],
        access_token: TOKEN.to_string(),
        benefit_id: "wallet-lsa-1".to_string(),
        category_id: "cat-fitness".to_string(),
        subcategory_value: "gym_membership".to_string(),
        subcategory_alias: None,
    };
    let err = create_claim(&opts).expect_err("should fail");
    let msg = format!("{err:#}");
    assert!(msg.contains("Failed to attach receipt"), "{msg}");
}

// ---------------------------------------------------------------------------
// claim_input_to_create_options (also exercises the profile endpoint)
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn claim_input_to_create_options_resolves_benefit_and_subcategory_alias() {
    let (server, _guard) = forma_server();
    server.mock(|when, then| {
        when.method(GET).path("/client/api/v3/settings/profile");
        then.status(200).body(fixture("profile_response.json"));
    });
    let receipt = fake_receipt();
    let claim = formanator::claims::ClaimInput {
        benefit: "Learning".to_string(),
        category: "Book (personal development)".to_string(),
        amount: "9.99".to_string(),
        merchant: "Local Bookshop".to_string(),
        purchase_date: "2024-02-03".to_string(),
        description: "Personal development book".to_string(),
        receipt_path: vec![receipt.path().to_path_buf()],
    };
    let opts =
        formanator::claims::claim_input_to_create_options(&claim, TOKEN).expect("should resolve");
    assert_eq!(opts.benefit_id, "wallet-0003-aaaa-aaaa-aaaa-aaaaaaaaaaaa");
    assert_eq!(opts.category_id, "personal_development");
    assert_eq!(opts.subcategory_value, "book");
    assert_eq!(
        opts.subcategory_alias.as_deref(),
        Some("Book (personal development)")
    );
}

#[test]
#[serial]
fn claim_input_to_create_options_errors_for_unknown_category() {
    let (server, _guard) = forma_server();
    server.mock(|when, then| {
        when.method(GET).path("/client/api/v3/settings/profile");
        then.status(200).body(fixture("profile_response.json"));
    });
    let receipt = fake_receipt();
    let claim = formanator::claims::ClaimInput {
        benefit: "Learning".to_string(),
        category: "Bogus Category".to_string(),
        amount: "9.99".to_string(),
        merchant: "X".to_string(),
        purchase_date: "2024-02-03".to_string(),
        description: "X".to_string(),
        receipt_path: vec![receipt.path().to_path_buf()],
    };
    let err =
        formanator::claims::claim_input_to_create_options(&claim, TOKEN).expect_err("should fail");
    assert!(format!("{err}").contains("No category"));
}

#[test]
#[serial]
fn claim_input_to_create_options_errors_for_invalid_amount() {
    let (server, _guard) = forma_server();
    server.mock(|when, then| {
        when.method(GET).path("/client/api/v3/settings/profile");
        then.status(200).body(fixture("profile_response.json"));
    });
    let receipt = fake_receipt();
    let claim = formanator::claims::ClaimInput {
        benefit: "Learning".to_string(),
        category: "Book".to_string(),
        amount: "9.999".to_string(),
        merchant: "X".to_string(),
        purchase_date: "2024-02-03".to_string(),
        description: "X".to_string(),
        receipt_path: vec![receipt.path().to_path_buf()],
    };
    let err =
        formanator::claims::claim_input_to_create_options(&claim, TOKEN).expect_err("should fail");
    assert!(format!("{err}").contains("Amount"));
}

#[test]
#[serial]
fn api_base_override_can_be_cleared_and_reinstalled() {
    // Sanity check that the guard pattern restores state. After this block the
    // override must be cleared so that other tests can install their own mocks.
    {
        let _g = ApiBaseGuard::new("http://example.invalid");
        // Just exercise the call paths; nothing should panic.
    }
    set_api_base(Some("http://example2.invalid".to_string()));
    set_api_base(None);
}
