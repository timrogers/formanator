//! Model Context Protocol (MCP) server, exposing Forma operations as tools to
//! MCP clients such as Claude Desktop or VS Code's MCP integration. Modelled
//! on the corresponding implementation in [`timrogers/litra-rs`].

use std::path::PathBuf;

use anyhow::Result;
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt, handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters, model::*, schemars, tool, tool_handler, tool_router,
    transport::stdio,
};

use crate::claims::{ClaimInput, claim_input_to_create_options};
use crate::cli::McpArgs;
use crate::config::resolve_access_token;
use crate::forma::{ClaimsFilter, create_claim, get_benefits_with_categories, get_claims_list};

#[derive(serde::Deserialize, schemars::JsonSchema, Default)]
pub struct ListBenefitsParams {}

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
}

impl Default for FormanatorMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl FormanatorMcpServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    fn token() -> Result<String, McpError> {
        resolve_access_token(None).map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    #[tool(
        description = "List all available Forma benefits with their categories and remaining balances",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_benefits_with_categories(
        &self,
        Parameters(_params): Parameters<ListBenefitsParams>,
    ) -> Result<CallToolResult, McpError> {
        let token = Self::token()?;
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
        let token = Self::token()?;
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
        let token = Self::token()?;
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

pub fn run(args: McpArgs) -> Result<()> {
    let _ = resolve_access_token(args.access_token.as_deref())?;

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
        let service = FormanatorMcpServer::new()
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
