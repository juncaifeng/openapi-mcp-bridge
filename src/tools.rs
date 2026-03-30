use crate::state::{State, Tool};
use anyhow::Result;
use reqwest::Method;
use serde_json::Value;

pub async fn execute(state: &State, tool: &Tool, args: Value) -> Result<Value> {
    let client = &state.client;
    let config = &state.config;

    let base_url = config.base_url.as_ref().map(|s| s.as_str()).unwrap_or("");
    let url = format!("{}{}", base_url.trim_end_matches('/'), tool.path);

    let method = match tool.method.as_str() {
        "GET" => Method::GET,
        "POST" => Method::POST,
        "PUT" => Method::PUT,
        "DELETE" => Method::DELETE,
        "PATCH" => Method::PATCH,
        _ => anyhow::bail!("Unsupported method: {}", tool.method),
    };

    let mut req = client.request(method.clone(), &url);

    if let Some(token) = &config.auth_token {
        req = req.bearer_auth(token);
    }

    if let Some(api_key) = &config.api_key {
        req = req.header("X-API-Key", api_key);
    }

    if let Some(extra_headers) = &config.extra_headers {
        for header_line in extra_headers.split('\n') {
            let header_line = header_line.trim();
            if header_line.is_empty() {
                continue;
            }
            if let Some((key, value)) = header_line.split_once(':') {
                req = req.header(key.trim(), value.trim());
            }
        }
    }

    // Handle parameters based on HTTP method
    match method {
        Method::GET | Method::DELETE => {
            // GET and DELETE: parameters as query string
            if let Value::Object(obj) = &args {
                if !obj.is_empty() {
                    req = req.query(obj);
                }
            }
        }
        Method::POST | Method::PUT | Method::PATCH => {
            // POST/PUT/PATCH: parameters as JSON body
            if !args.is_null() {
                req = req.json(&args);
            }
        }
        _ => {}
    }

    let resp = req.send().await?;
    let status = resp.status();
    let text = resp.text().await?;

    if !status.is_success() {
        anyhow::bail!("API error {}: {}", status, text);
    }

    match serde_json::from_str::<Value>(&text) {
        Ok(json) => Ok(json),
        Err(_) => Ok(Value::String(text)),
    }
}
