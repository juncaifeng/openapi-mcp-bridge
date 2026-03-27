use anyhow::Result;
use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(rename = "OPENAPI_SPEC_PATH")]
    pub spec_path: String,

    #[serde(rename = "API_BASE_URL")]
    pub base_url: Option<String>,

    #[serde(rename = "API_AUTH_TOKEN")]
    pub auth_token: Option<String>,

    #[serde(rename = "API_KEY")]
    pub api_key: Option<String>,

    #[serde(rename = "SKILLS_MD_PATH")]
    pub skills_path: Option<String>,

    #[serde(rename = "EXTRA_HEADERS")]
    pub extra_headers: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Config {
            spec_path: env::var("OPENAPI_SPEC_PATH")?,
            base_url: env::var("API_BASE_URL").ok(),
            auth_token: env::var("API_AUTH_TOKEN").ok(),
            api_key: env::var("API_KEY").ok(),
            skills_path: env::var("SKILLS_MD_PATH").ok(),
            extra_headers: env::var("EXTRA_HEADERS").ok(),
        })
    }
}
