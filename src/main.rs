use anyhow::Result;
use rmcp::ServiceExt;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let config = openapi_mcp_bridge::config::Config::from_env()?;
    let state = openapi_mcp_bridge::state::State::new(config).await?;
    let handler = openapi_mcp_bridge::Handler::new(state);

    tracing::info!("Starting MCP server");

    let service = handler.serve(rmcp::transport::stdio()).await.inspect_err(|e| {
        tracing::error!("serving error: {:?}", e);
    })?;

    service.waiting().await?;
    Ok(())
}
