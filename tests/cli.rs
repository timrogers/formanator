//! End-to-end tests that drive the compiled `formanator` binary as a child
//! process. Network-dependent commands are pointed at a local mock HTTP server
//! via the `FORMANATOR_API_BASE` environment variable, and the config file is
//! redirected to a temporary directory via `HOME` so the tests don't touch the
//! user's real `~/.formanator.toml`.

use std::process::Command;

use assert_cmd::prelude::*;
use httpmock::prelude::*;
use predicates::prelude::*;
use predicates::str::contains;
use serial_test::serial;

#[path = "common/mod.rs"]
mod common;
use common::{fixture, make_fake_receipt};

const TOKEN: &str = "test-access-token-abc123";

/// Start a fresh mock HTTP server and prepare a `formanator` Command that's
/// already wired up to talk to it, with a clean temporary HOME and no
/// auto-inherited Forma credentials.
fn cli_with_server() -> (MockServer, Command, tempfile::TempDir) {
    let server = MockServer::start();
    let home = tempfile::tempdir().expect("tempdir");
    let config_path = home.path().join(".formanator.toml");
    let mut cmd = Command::cargo_bin("formanator").expect("binary built");
    // Keep the test environment isolated from any developer config.
    cmd.env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        // On Unix, HOME is used by dirs::home_dir(). On Windows the dirs crate
        // calls SHGetKnownFolderPath (a Win32 API) which ignores env vars, so
        // we override the full config path directly instead.
        .env("HOME", home.path())
        .env("FORMANATOR_CONFIG_PATH", &config_path)
        // Reset any color output so predicate matching is reliable.
        .env("NO_COLOR", "1")
        .env("FORMANATOR_API_BASE", server.base_url())
        // Don't perform real network update checks during tests.
        .env("FORMANATOR_DISABLE_UPDATE_CHECK", "1");
    // On Windows, `env_clear` strips `SystemRoot`, which the Winsock provider
    // needs to locate its DLLs (%SystemRoot%\System32). Without it spawned
    // subprocesses fail with WSAPROVIDERFAILEDINIT (os error 10106).
    #[cfg(windows)]
    if let Some(v) = std::env::var_os("SystemRoot") {
        cmd.env("SystemRoot", v);
    }
    (server, cmd, home)
}

// ---------------------------------------------------------------------------
// Plain CLI behaviour (no network)
// ---------------------------------------------------------------------------

#[test]
fn help_lists_all_subcommands() {
    Command::cargo_bin("formanator")
        .unwrap()
        .env("FORMANATOR_DISABLE_UPDATE_CHECK", "1")
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("login"))
        .stdout(contains("benefits"))
        .stdout(contains("categories"))
        .stdout(contains("list-claims"))
        .stdout(contains("submit-claim"))
        .stdout(contains("generate-template-csv"))
        .stdout(contains("submit-claims-from-csv"))
        .stdout(contains("submit-claims-from-directory"))
        .stdout(contains("validate-csv"));
}

#[test]
fn version_prints_crate_version() {
    Command::cargo_bin("formanator")
        .unwrap()
        .env("FORMANATOR_DISABLE_UPDATE_CHECK", "1")
        .arg("--version")
        .assert()
        .success()
        .stdout(contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn unknown_subcommand_fails() {
    Command::cargo_bin("formanator")
        .unwrap()
        .env("FORMANATOR_DISABLE_UPDATE_CHECK", "1")
        .arg("definitely-not-a-command")
        .assert()
        .failure();
}

#[test]
fn generate_template_csv_writes_the_template_to_a_fresh_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("claims.csv");
    Command::cargo_bin("formanator")
        .unwrap()
        .env("FORMANATOR_DISABLE_UPDATE_CHECK", "1")
        .args(["generate-template-csv", "--output-path"])
        .arg(&path)
        .assert()
        .success()
        .stdout(contains("Wrote template CSV"));
    let on_disk = std::fs::read_to_string(&path).unwrap();
    let expected = fixture("template.csv");
    assert_eq!(on_disk, expected);
}

#[test]
fn generate_template_csv_refuses_to_overwrite_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("claims.csv");
    std::fs::write(&path, "existing").unwrap();
    Command::cargo_bin("formanator")
        .unwrap()
        .env("FORMANATOR_DISABLE_UPDATE_CHECK", "1")
        .args(["generate-template-csv", "--output-path"])
        .arg(&path)
        .assert()
        .failure()
        .stderr(contains("already exists"));
    // Original contents preserved.
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "existing");
}

#[test]
fn benefits_without_login_fails_with_helpful_message() {
    let home = tempfile::tempdir().unwrap();
    Command::cargo_bin("formanator")
        .unwrap()
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", home.path())
        .env("NO_COLOR", "1")
        .env("FORMANATOR_DISABLE_UPDATE_CHECK", "1")
        .arg("benefits")
        .assert()
        .failure()
        .stderr(contains("formanator login"));
}

#[test]
fn list_claims_rejects_unknown_filter() {
    let home = tempfile::tempdir().unwrap();
    Command::cargo_bin("formanator")
        .unwrap()
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", home.path())
        .env("FORMANATOR_ACCESS_TOKEN", TOKEN)
        .env("NO_COLOR", "1")
        .env("FORMANATOR_DISABLE_UPDATE_CHECK", "1")
        .args(["list-claims", "--filter", "bogus"])
        .assert()
        .failure()
        .stderr(contains("Invalid filter value"));
}

// ---------------------------------------------------------------------------
// Commands wired to a mock Forma API
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn benefits_command_renders_a_table_from_mock_profile_response() {
    let (server, mut cmd, _home) = cli_with_server();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/client/api/v3/settings/profile")
            .header("x-auth-token", TOKEN);
        then.status(200).body(fixture("profile_response.json"));
    });

    cmd.env("FORMANATOR_ACCESS_TOKEN", TOKEN)
        .arg("benefits")
        .assert()
        .success()
        .stdout(contains("Wellness and Lifestyle"))
        .stdout(contains("Learning"))
        .stdout(contains("Flexible Reimbursement Account"))
        .stdout(contains("GBP"))
        .stdout(contains("750.5"))
        // Ineligible wallets must not appear.
        .stdout(contains("Remote Life").not())
        .stdout(contains("New Hire Home Office").not());
    mock.assert();
}

#[test]
#[serial]
fn benefits_command_surfaces_friendly_error_for_invalid_token() {
    let (server, mut cmd, _home) = cli_with_server();
    server.mock(|when, then| {
        when.method(GET).path("/client/api/v3/settings/profile");
        then.status(401).body(fixture("error_invalid_jwt.json"));
    });

    cmd.env("FORMANATOR_ACCESS_TOKEN", TOKEN)
        .arg("benefits")
        .assert()
        .failure()
        .stderr(contains("Forma access token is invalid"));
}

#[test]
#[serial]
fn categories_command_lists_subcategories_for_a_benefit() {
    let (server, mut cmd, _home) = cli_with_server();
    server.mock(|when, then| {
        when.method(GET).path("/client/api/v3/settings/profile");
        then.status(200).body(fixture("profile_response.json"));
    });

    cmd.env("FORMANATOR_ACCESS_TOKEN", TOKEN)
        .args(["categories", "--benefit", "Learning"])
        .assert()
        .success()
        .stdout(contains("Personal Development"))
        .stdout(contains("Book"))
        .stdout(contains("Book (personal development)"))
        .stdout(contains("Skills Development"))
        // Categories from another benefit must not leak in.
        .stdout(contains("Athletic Clothing").not());
}

#[test]
#[serial]
fn list_claims_renders_pagination_results_as_a_table() {
    let (server, mut cmd, _home) = cli_with_server();
    server.mock(|when, then| {
        when.method(GET)
            .path("/client/api/v2/claims")
            .query_param("page", "0");
        then.status(200).body(fixture("claims_list_page0.json"));
    });
    server.mock(|when, then| {
        when.method(GET)
            .path("/client/api/v2/claims")
            .query_param("page", "1");
        then.status(200).body(fixture("claims_list_page1.json"));
    });

    cmd.env("FORMANATOR_ACCESS_TOKEN", TOKEN)
        .arg("list-claims")
        .assert()
        .success()
        .stdout(contains("Apple"))
        .stdout(contains("Amazon"))
        .stdout(contains("Open University"))
        // The "Payout Status" column is added when at least one claim has one.
        .stdout(contains("Payout Status"));
}

#[test]
#[serial]
fn list_claims_in_progress_filter_returns_only_matching_claims() {
    let (server, mut cmd, _home) = cli_with_server();
    server.mock(|when, then| {
        when.method(GET).path("/client/api/v2/claims");
        then.status(200)
            .body(fixture("claims_list_in_progress.json"));
    });

    cmd.env("FORMANATOR_ACCESS_TOKEN", TOKEN)
        .args(["list-claims", "--filter", "in_progress"])
        .assert()
        .success()
        // ip_c1 (top-level in_progress) and ip_c2 (reimbursement in_progress)
        // must both appear; ip_c3 (completed) must be filtered out.
        .stdout(contains("Open University"))
        .stdout(contains("Peloton"))
        .stdout(contains("Grab").not());
}

#[test]
#[serial]
fn submit_claim_dry_run_resolves_ids_without_posting_a_claim() {
    let (server, mut cmd, _home) = cli_with_server();
    let profile = server.mock(|when, then| {
        when.method(GET).path("/client/api/v3/settings/profile");
        then.status(200).body(fixture("profile_response.json"));
    });
    // No mock for the create endpoint; --dry-run must not POST.

    let receipt = make_fake_receipt();

    cmd.env("FORMANATOR_ACCESS_TOKEN", TOKEN)
        .args([
            "submit-claim",
            "--benefit",
            "Learning",
            "--category",
            "Book (personal development)",
            "--amount",
            "9.99",
            "--merchant",
            "Local Bookshop",
            "--purchase-date",
            "2024-02-03",
            "--description",
            "Personal development book",
            "--receipt-path",
        ])
        .arg(receipt.path())
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(contains("Dry run"))
        .stdout(contains("Claim submitted successfully"));
    profile.assert();
}

#[test]
#[serial]
fn submit_claim_submits_a_full_multipart_request_to_the_mock_server() {
    let (server, mut cmd, _home) = cli_with_server();
    server.mock(|when, then| {
        when.method(GET).path("/client/api/v3/settings/profile");
        then.status(200).body(fixture("profile_response.json"));
    });
    let create = server.mock(|when, then| {
        when.method(POST)
            .path("/client/api/v2/claims")
            .header("x-auth-token", TOKEN)
            .body_includes("name=\"amount\"")
            .body_includes("9.99")
            .body_includes("name=\"reimbursement_vendor\"")
            .body_includes("Local Bookshop")
            .body_includes("name=\"default_employee_wallet_id\"")
            .body_includes("wallet-0003-aaaa-aaaa-aaaa-aaaaaaaaaaaa");
        then.status(201)
            .body(fixture("create_claim_response_success.json"));
    });
    let receipt = make_fake_receipt();

    cmd.env("FORMANATOR_ACCESS_TOKEN", TOKEN)
        .args([
            "submit-claim",
            "--benefit",
            "Learning",
            "--category",
            "Book (personal development)",
            "--amount",
            "9.99",
            "--merchant",
            "Local Bookshop",
            "--purchase-date",
            "2024-02-03",
            "--description",
            "Personal development book",
            "--receipt-path",
        ])
        .arg(receipt.path())
        .assert()
        .success()
        .stdout(contains("Claim submitted successfully"));
    create.assert();
}

#[test]
#[serial]
fn submit_claim_without_required_args_or_llm_key_fails_with_explanation() {
    let (_server, mut cmd, _home) = cli_with_server();
    let receipt = make_fake_receipt();
    cmd.env("FORMANATOR_ACCESS_TOKEN", TOKEN)
        .args(["submit-claim", "--receipt-path"])
        .arg(receipt.path())
        .assert()
        .failure()
        .stderr(contains("OpenAI API key").or(contains("GitHub token")));
}

#[test]
#[serial]
fn submit_claims_from_csv_dry_run_runs_against_the_mock_server() {
    let (server, mut cmd, _home) = cli_with_server();
    server.mock(|when, then| {
        when.method(GET).path("/client/api/v3/settings/profile");
        then.status(200).body(fixture("profile_response.json"));
    });

    let receipt = make_fake_receipt();
    let dir = tempfile::tempdir().unwrap();
    let csv_path = dir.path().join("claims.csv");
    let body = format!(
        "benefit,category,merchant,amount,description,purchaseDate,receiptPath\n\
         Learning,Book (personal development),Local Bookshop,9.99,Monthly,2024-02-03,{}\n",
        receipt.path().display()
    );
    std::fs::write(&csv_path, body).unwrap();

    cmd.env("FORMANATOR_ACCESS_TOKEN", TOKEN)
        .args(["submit-claims-from-csv", "--input-path"])
        .arg(&csv_path)
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(contains("Dry run"))
        .stdout(contains("Successfully submitted claim"));
    // `dir` lives until the end of the function, so its temporary files are
    // cleaned up after the assertion completes.
    drop(dir);
}

#[test]
#[serial]
fn validate_csv_reports_per_row_validation_status() {
    let (server, mut cmd, _home) = cli_with_server();
    // validate-csv calls profile multiple times: once via
    // get_benefits_with_categories and again per row via
    // claim_input_to_create_options. Configure a single mock that handles all
    // calls.
    server.mock(|when, then| {
        when.method(GET).path("/client/api/v3/settings/profile");
        then.status(200).body(fixture("profile_response.json"));
    });

    let receipt = make_fake_receipt();
    let dir = tempfile::tempdir().unwrap();
    let csv_path = dir.path().join("claims.csv");
    let body = format!(
        "benefit,category,merchant,amount,description,purchaseDate,receiptPath\n\
         Learning,Book,Local Bookshop,25.99,Personal development book,2024-01-02,{}\n\
         Wellness and Lifestyle,Athletic Clothing,Sportswear Co,30.00,Running shorts,2024-01-03,{}\n",
        receipt.path().display(),
        receipt.path().display()
    );
    std::fs::write(&csv_path, body).unwrap();

    cmd.env("FORMANATOR_ACCESS_TOKEN", TOKEN)
        .args(["validate-csv", "--input-path"])
        .arg(&csv_path)
        .assert()
        .success()
        .stdout(contains("Validating claim 1/2"))
        .stdout(contains("Validated claim 1/2"))
        .stdout(contains("Validating claim 2/2"))
        .stdout(contains("Validated claim 2/2"));
}

#[test]
#[serial]
fn validate_csv_errors_on_missing_input_file() {
    let (_server, mut cmd, _home) = cli_with_server();
    cmd.env("FORMANATOR_ACCESS_TOKEN", TOKEN)
        .args(["validate-csv", "--input-path", "/no/such/file.csv"])
        .assert()
        .failure()
        .stderr(contains("doesn't exist"));
}

#[test]
#[serial]
fn login_with_magic_link_writes_config_to_home() {
    let (server, mut cmd, home) = cli_with_server();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/client/auth/v2/login/magic")
            .query_param("id", "abc123")
            .query_param("tk", "xyz789")
            .query_param("return_token", "true");
        then.status(200)
            .body(fixture("magic_link_exchange_response.json"));
    });

    let inner = "https://api.joinforma.com/client/auth/v2/login/magic?id=abc123&tk=xyz789";
    let encoded = url::form_urlencoded::byte_serialize(inner.as_bytes()).collect::<String>();
    let outer = format!("https://joinforma.page.link/?link={encoded}");

    cmd.args(["login", "--magic-link", &outer])
        .assert()
        .success()
        .stdout(contains("logged in"));
    mock.assert();

    // The CLI should have persisted the access token returned by the mock
    // server into the config path we pointed it at via FORMANATOR_CONFIG_PATH.
    let config_path = home.path().join(".formanator.toml");
    let saved: toml::Value =
        toml::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
    assert_eq!(
        saved["access_token"].as_str(),
        Some(common::FIXTURE_AUTH_TOKEN)
    );
}

#[test]
#[serial]
fn login_with_invalid_magic_link_fails_without_writing_config() {
    let (_server, mut cmd, home) = cli_with_server();
    cmd.args(["login", "--magic-link", "not a magic link"])
        .assert()
        .failure();
    assert!(
        !home.path().join(".formanator.toml").exists(),
        "no config file should have been written"
    );
}
