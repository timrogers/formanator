//! Model Context Protocol (MCP) server, exposing Forma operations as tools to
//! MCP clients such as Claude Desktop or VS Code's MCP integration. Modelled
//! on the corresponding implementation in [`timrogers/litra-rs`].

use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};

use anyhow::Result;
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt, handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters, model::*, schemars, tool, tool_handler, tool_router,
    transport::stdio,
};

use crate::claims::{ClaimInput, claim_input_to_create_options};
use crate::cli::McpArgs;
use crate::commands::login::parse_emailed_forma_magic_link;
use crate::config::{Config, get_access_token, read_config, store_config};
use crate::forma::{
    ClaimsFilter, create_claim, exchange_id_and_tk_for_access_token, get_benefits_with_categories,
    get_claims_list, request_magic_link,
};

const LOGIN_REQUIRED_MESSAGE: &str = "You aren't logged in to Forma. Use the `login_start` MCP tool to request a magic link, then call `login_complete` with the link from your email.";

#[derive(serde::Deserialize, schemars::JsonSchema, Default)]
pub struct ListBenefitsParams {}

#[derive(serde::Deserialize, schemars::JsonSchema, Default)]
pub struct AuthStatusParams {}

#[derive(serde::Deserialize, schemars::JsonSchema)]
pub struct LoginStartParams {
    /// Email address for the Forma account to log in to.
    pub email: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
pub struct LoginCompleteParams {
    /// The magic link emailed by Forma.
    #[serde(rename = "magicLink")]
    pub magic_link: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema, Default)]
pub struct ListClaimsParams {
    /// Optional filter for claim status. Currently supports `in_progress`.
    pub filter: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
pub struct CreateClaimParams {
    /// The amount to claim, e.g. "25.99".
    pub amount: String,
    /// The merchant / vendor name.
    pub merchant: String,
    /// The purchase date in YYYY-MM-DD format.
    #[serde(rename = "purchaseDate")]
    pub purchase_date: String,
    /// A short description of the purchase.
    pub description: String,
    /// One or more file paths to receipt images / PDFs.
    #[serde(rename = "receiptPath")]
    pub receipt_path: Vec<String>,
    /// The Forma benefit name to claim against.
    pub benefit: String,
    /// The Forma category name (or alias) for the claim.
    pub category: String,
}

#[derive(Clone)]
pub struct FormanatorMcpServer {
    #[allow(dead_code)]
    tool_router: ToolRouter<FormanatorMcpServer>,
    access_token: Arc<RwLock<Option<String>>>,
}

impl Default for FormanatorMcpServer {
    fn default() -> Self {
        Self::new(None)
    }
}

#[tool_router]
impl FormanatorMcpServer {
    pub fn new(access_token: Option<String>) -> Self {
        Self {
            tool_router: Self::tool_router(),
            access_token: Arc::new(RwLock::new(access_token.filter(|token| !token.is_empty()))),
        }
    }

    fn has_access_token(&self) -> Result<bool, McpError> {
        self.access_token
            .read()
            .map(|token| token.as_ref().is_some_and(|token| !token.is_empty()))
            .map_err(|_| McpError::internal_error("Authentication state lock is poisoned", None))
    }

    fn set_access_token(&self, access_token: String) -> Result<(), McpError> {
        let mut token = self
            .access_token
            .write()
            .map_err(|_| McpError::internal_error("Authentication state lock is poisoned", None))?;
        *token = Some(access_token);
        Ok(())
    }

    fn token(&self) -> Result<String, McpError> {
        self.access_token
            .read()
            .map_err(|_| McpError::internal_error("Authentication state lock is poisoned", None))?
            .clone()
            .filter(|token| !token.is_empty())
            .ok_or_else(|| McpError::invalid_request(LOGIN_REQUIRED_MESSAGE, None))
    }

    fn complete_login(&self, magic_link: &str) -> Result<(), McpError> {
        let (id, tk) = parse_emailed_forma_magic_link(magic_link)
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
        let access_token = exchange_id_and_tk_for_access_token(&id, &tk)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        store_access_token(&access_token)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        self.set_access_token(access_token)
    }

    #[tool(
        description = "Check whether Formanator is logged in to Forma",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn auth_status(
        &self,
        Parameters(_params): Parameters<AuthStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        let authenticated = self.has_access_token()?;
        let json = serde_json::to_string_pretty(&serde_json::json!({
            "authenticated": authenticated,
            "message": if authenticated {
                "Formanator has a Forma access token."
            } else {
                LOGIN_REQUIRED_MESSAGE
            },
        }))
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(
        description = "Start logging in to Forma by emailing a magic link",
        annotations(
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn login_start(
        &self,
        Parameters(params): Parameters<LoginStartParams>,
    ) -> Result<CallToolResult, McpError> {
        request_magic_link(&params.email)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(
            "Forma has emailed a magic link. Call `login_complete` with the magic link from that email.",
        )]))
    }

    #[tool(
        description = "Complete logging in to Forma with the emailed magic link",
        annotations(
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn login_complete(
        &self,
        Parameters(params): Parameters<LoginCompleteParams>,
    ) -> Result<CallToolResult, McpError> {
        self.complete_login(&params.magic_link)?;
        Ok(CallToolResult::success(vec![Content::text(
            "You are now logged in to Forma. The access token has been stored locally.",
        )]))
    }

    #[tool(
        description = "List all available Forma benefits with their categories and remaining balances",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_benefits_with_categories(
        &self,
        Parameters(_params): Parameters<ListBenefitsParams>,
    ) -> Result<CallToolResult, McpError> {
        let token = self.token()?;
        let benefits = get_benefits_with_categories(&token)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let json = serde_json::to_string_pretty(&benefits)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(
        description = "List claims in your Forma account with optional filtering",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_claims(
        &self,
        Parameters(params): Parameters<ListClaimsParams>,
    ) -> Result<CallToolResult, McpError> {
        let token = self.token()?;
        let filter = match params.filter.as_deref() {
            None => None,
            Some("in_progress") => Some(ClaimsFilter::InProgress),
            Some(other) => {
                return Err(McpError::invalid_params(
                    format!("Invalid filter value '{other}'. Currently supported: in_progress"),
                    None,
                ));
            }
        };
        let claims = get_claims_list(&token, filter)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let json = serde_json::to_string_pretty(&claims)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(
        description = "Create a new Forma claim",
        annotations(
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn create_claim(
        &self,
        Parameters(params): Parameters<CreateClaimParams>,
    ) -> Result<CallToolResult, McpError> {
        let token = self.token()?;
        let claim = ClaimInput {
            benefit: params.benefit,
            category: params.category,
            amount: params.amount,
            merchant: params.merchant,
            purchase_date: params.purchase_date,
            description: params.description,
            receipt_path: params.receipt_path.into_iter().map(PathBuf::from).collect(),
        };
        let opts = claim_input_to_create_options(&claim, &token)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        match create_claim(&opts) {
            Ok(()) => Ok(CallToolResult::success(vec![Content::text(
                "Claim created successfully",
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e.to_string())])),
        }
    }
}

#[tool_handler]
impl ServerHandler for FormanatorMcpServer {
    fn get_info(&self) -> ServerInfo {
        let mut implementation =
            Implementation::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        implementation.title = Some("Formanator".to_owned());
        implementation.website_url =
            Some("https://github.com/timrogers/formanator-rust".to_owned());

        let mut server = ServerInfo::new(ServerCapabilities::builder().enable_tools().build());
        server.protocol_version = ProtocolVersion::V_2025_03_26;
        server.server_info = implementation;
        server.instructions = None;
        server
    }
}

fn resolve_initial_access_token(explicit: Option<&str>) -> Result<Option<String>> {
    if let Some(token) = explicit.filter(|token| !token.is_empty()) {
        return Ok(Some(token.to_string()));
    }
    get_access_token()
}

fn store_access_token(access_token: &str) -> Result<()> {
    let last_update_check_timestamp = read_config()?.and_then(|c| c.last_update_check_timestamp);
    store_config(&Config {
        access_token: access_token.to_string(),
        email: None,
        last_update_check_timestamp,
    })
}

pub fn run(args: McpArgs) -> Result<()> {
    let access_token = resolve_initial_access_token(args.access_token.as_deref())?;

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        tracing::info!("Starting Formanator MCP server");
        let service = FormanatorMcpServer::new(access_token)
            .serve(stdio())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to start MCP server: {e}"))?;
        service
            .waiting()
            .await
            .map_err(|e| anyhow::anyhow!("MCP server error: {e}"))?;
        Ok::<(), anyhow::Error>(())
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::ffi::{OsStr, OsString};

    use httpmock::prelude::*;
    use serial_test::serial;

    use super::*;
    use crate::{forma::set_api_base, keychain};

    const TOKEN: &str = "mcp-login-token";

    struct EnvVarGuard {
        key: &'static str,
        original: Option<OsString>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: impl AsRef<OsStr>) -> Self {
            let original = std::env::var_os(key);
            unsafe {
                std::env::set_var(key, value);
            }
            Self { key, original }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            unsafe {
                match &self.original {
                    Some(value) => std::env::set_var(self.key, value),
                    None => std::env::remove_var(self.key),
                }
            }
        }
    }

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

    fn result_text(result: CallToolResult) -> String {
        let value = serde_json::to_value(result).expect("tool result serializes");
        value["content"][0]["text"]
            .as_str()
            .expect("tool result includes text content")
            .to_string()
    }

    fn magic_link(id: &str, tk: &str) -> String {
        let inner = format!("https://api.joinforma.com/client/auth/v2/login/magic?id={id}&tk={tk}");
        let encoded = url::form_urlencoded::byte_serialize(inner.as_bytes()).collect::<String>();
        format!("https://joinforma.page.link/?link={encoded}")
    }

    #[test]
    fn rejects_forma_tools_without_access_token() {
        let server = FormanatorMcpServer::new(None);
        let err = server.token().expect_err("token should be required");

        assert!(err.message.contains("login_start"), "{err:?}");
        assert!(err.message.contains("login_complete"), "{err:?}");
    }

    #[test]
    fn resolve_initial_access_token_prefers_explicit_token() {
        let token = resolve_initial_access_token(Some("from-mcp-arg")).expect("should resolve");

        assert_eq!(token, Some("from-mcp-arg".to_string()));
    }

    #[tokio::test]
    async fn auth_status_reports_unauthenticated_without_access_token() {
        let server = FormanatorMcpServer::new(None);
        let result = server
            .auth_status(Parameters(AuthStatusParams {}))
            .await
            .expect("auth status should succeed");
        let text = result_text(result);
        let status: serde_json::Value = serde_json::from_str(&text).expect("valid JSON status");

        assert_eq!(status["authenticated"], false);
        assert!(
            status["message"]
                .as_str()
                .expect("message")
                .contains("login_start")
        );
    }

    #[test]
    fn token_returns_stored_value() {
        let server = FormanatorMcpServer::new(Some("my-token".to_string()));
        assert_eq!(server.token().expect("token should be present"), "my-token");
    }

    #[test]
    fn has_access_token_returns_true_with_token() {
        let server = FormanatorMcpServer::new(Some("tok".to_string()));
        assert!(server.has_access_token().expect("no lock error"));
    }

    #[test]
    fn has_access_token_returns_false_without_token() {
        let server = FormanatorMcpServer::new(None);
        assert!(!server.has_access_token().expect("no lock error"));
    }

    #[tokio::test]
    async fn auth_status_reports_authenticated_with_access_token() {
        let server = FormanatorMcpServer::new(Some("my-token".to_string()));
        let result = server
            .auth_status(Parameters(AuthStatusParams {}))
            .await
            .expect("auth status should succeed");
        let text = result_text(result);
        let status: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");

        assert_eq!(status["authenticated"], true);
        assert!(
            status["message"]
                .as_str()
                .expect("message")
                .contains("access token"),
            "unexpected message: {}",
            status["message"]
        );
    }

    #[tokio::test]
    async fn list_claims_rejects_invalid_filter() {
        let server = FormanatorMcpServer::new(Some("tok".to_string()));
        let err = server
            .list_claims(Parameters(ListClaimsParams {
                filter: Some("bogus".to_string()),
            }))
            .await
            .expect_err("invalid filter should return an error");

        assert!(
            err.message.contains("in_progress"),
            "error should mention 'in_progress': {err:?}"
        );
    }

    #[test]
    fn resolve_initial_access_token_treats_empty_string_as_none() {
        // An empty string should be treated as absent and fall through to the
        // config/keychain lookup. We can't easily assert the exact return value
        // here since it depends on the environment, but we can assert the
        // function returns Ok (it only errors if config I/O fails).
        let result = resolve_initial_access_token(Some(""));
        assert!(result.is_ok(), "should not error for an empty token");
        // The returned value must not be Some("").
        if let Ok(Some(t)) = result {
            assert!(!t.is_empty(), "returned token must not be empty");
        }
    }

    #[serial]
    #[test]
    fn login_complete_stores_token_and_updates_server_state() {
        let _mock_keychain = EnvVarGuard::set("FORMANATOR_USE_MOCK_KEYCHAIN", "1");
        keychain::init();
        let config_dir = tempfile::tempdir().expect("temp config dir");
        let config_path = config_dir.path().join(".formanator.toml");
        let _config_path = EnvVarGuard::set("FORMANATOR_CONFIG_PATH", &config_path);

        let server = MockServer::start();
        let _api_base = ApiBaseGuard::new(&server.base_url());
        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/client/auth/v2/login/magic")
                .query_param("id", "the-id")
                .query_param("tk", "the-tk")
                .query_param("return_token", "true");
            then.status(200).body(
                serde_json::json!({
                    "success": true,
                    "data": {
                        "auth_token": TOKEN
                    }
                })
                .to_string(),
            );
        });

        let mcp_server = FormanatorMcpServer::new(None);
        mcp_server
            .complete_login(&magic_link("the-id", "the-tk"))
            .expect("login should complete");

        mock.assert();
        assert_eq!(mcp_server.token().expect("token should be stored"), TOKEN);
    }
}
