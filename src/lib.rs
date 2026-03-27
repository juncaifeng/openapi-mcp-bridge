pub mod config;
pub mod state;
pub mod openapi;
pub mod tools;

use state::State;
use rmcp::{model::*, ServerHandler, ErrorData, RoleServer};

pub struct Handler {
    state: State,
}

impl Handler {
    pub fn new(state: State) -> Self {
        Self { state }
    }
}

impl ServerHandler for Handler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            ..Default::default()
        }
    }

    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParam>,
        _: rmcp::service::RequestContext<RoleServer>,
    ) -> std::result::Result<ListToolsResult, ErrorData> {
        tracing::info!("list_tools called");
        let tools = self.state.tools.read().await.clone();
        tracing::info!("Returning {} tools", tools.len());
        let mcp_tools: Vec<Tool> = tools.into_iter().map(|t| {
            Tool {
                name: t.name.into(),
                description: Some(t.description.into()),
                input_schema: std::sync::Arc::new(t.schema.as_object().cloned().unwrap_or_default()),
                output_schema: None,
                title: None,
                annotations: None,
                icons: None,
            }
        }).collect();

        Ok(ListToolsResult {
            tools: mcp_tools,
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        req: CallToolRequestParam,
        _: rmcp::service::RequestContext<RoleServer>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let tools = self.state.tools.read().await.clone();
        let tool = tools.iter().find(|t| t.name == req.name)
            .cloned()
            .ok_or_else(|| ErrorData::invalid_params("Tool not found", None))?;

        let args = req.arguments.unwrap_or_default();
        let args_value = serde_json::Value::Object(args);

        match tools::execute(&self.state, &tool, args_value).await {
            Ok(result) => Ok(CallToolResult {
                content: vec![Content::text(result.to_string())],
                structured_content: None,
                is_error: Some(false),
                meta: None,
            }),
            Err(e) => Ok(CallToolResult {
                content: vec![Content::text(format!("Error: {}", e))],
                structured_content: None,
                is_error: Some(true),
                meta: None,
            }),
        }
    }
}
