use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::Value;

#[derive(Clone)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub path: String,
    pub method: String,
    pub schema: Value,
}

pub struct State {
    pub tools: Arc<RwLock<Vec<Tool>>>,
    pub config: crate::config::Config,
    pub client: reqwest::Client,
}

impl State {
    pub async fn new(config: crate::config::Config) -> anyhow::Result<Self> {
        tracing::info!("Loading OpenAPI spec from: {}", config.spec_path);
        let spec = crate::openapi::load_spec(&config.spec_path).await?;
        tracing::info!("Successfully loaded OpenAPI spec");

        let tools = crate::openapi::extract_tools(&spec);
        tracing::info!("Extracted {} tools from OpenAPI spec", tools.len());

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            tools: Arc::new(RwLock::new(tools)),
            config,
            client,
        })
    }
}
