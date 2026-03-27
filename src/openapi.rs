use anyhow::{Result, Context};
use openapiv3::OpenAPI;
use crate::state::Tool;
use serde_json::json;

pub async fn load_spec(path: &str) -> Result<OpenAPI> {
    tracing::info!("Fetching OpenAPI spec from: {}", path);

    let content = if path.starts_with("http://") || path.starts_with("https://") {
        reqwest::get(path).await?
            .text()
            .await
            .with_context(|| format!("Failed to fetch spec from {}", path))?
    } else {
        tokio::fs::read_to_string(path).await
            .with_context(|| format!("Failed to read spec file: {}", path))?
    };

    tracing::debug!("Spec content length: {} bytes", content.len());

    let spec: OpenAPI = if path.ends_with(".yaml") || path.ends_with(".yml") {
        serde_yaml::from_str(&content)
            .with_context(|| "Failed to parse YAML OpenAPI spec")?
    } else {
        // Try parsing directly first
        match serde_json::from_str(&content) {
            Ok(spec) => {
                tracing::info!("Successfully parsed OpenAPI spec");
                spec
            }
            Err(e) => {
                // If that fails, try to fix common issues
                tracing::warn!("Direct parsing failed: {}, attempting to fix common issues", e);
                let fixed_content = fix_common_json_issues(&content)?;
                let spec = serde_json::from_str(&fixed_content)
                    .with_context(|| "Failed to parse OpenAPI spec even after fixes")?;
                tracing::info!("Successfully parsed OpenAPI spec after applying fixes");
                spec
            }
        }
    };

    Ok(spec)
}

fn fix_common_json_issues(content: &str) -> Result<String> {
    // Parse as generic JSON value first
    let mut value: serde_json::Value = serde_json::from_str(content)
        .with_context(|| "Failed to parse as JSON")?;

    // Recursively fix common issues
    fix_value(&mut value);

    // Convert back to string
    Ok(serde_json::to_string(&value)?)
}

fn fix_value(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            // Only fix boolean fields in specific contexts
            // In OpenAPI, 'required' can be either:
            // - A boolean (in Parameter objects)
            // - An array of strings (in Schema objects)

            // Check if this looks like a parameter object (has "in" field)
            let is_parameter = map.contains_key("in") && map.contains_key("name");

            if is_parameter {
                // For parameters, fix boolean fields
                for key in &["required", "deprecated"] {
                    if let Some(v) = map.get_mut(*key) {
                        if let serde_json::Value::Number(n) = v {
                            let bool_val = n.as_f64().unwrap_or(0.0) != 0.0;
                            tracing::debug!("Converting parameter field '{}' from {} to {}", key, n, bool_val);
                            *v = serde_json::Value::Bool(bool_val);
                        }
                    }
                }
            } else {
                // For other objects (like schemas), fix only non-required boolean fields
                for key in &["deprecated", "nullable", "readOnly", "writeOnly",
                             "uniqueItems", "exclusiveMinimum", "exclusiveMaximum", "allowEmptyValue"] {
                    if let Some(v) = map.get_mut(*key) {
                        if let serde_json::Value::Number(n) = v {
                            let bool_val = n.as_f64().unwrap_or(0.0) != 0.0;
                            tracing::debug!("Converting schema field '{}' from {} to {}", key, n, bool_val);
                            *v = serde_json::Value::Bool(bool_val);
                        }
                    }
                }
            }

            // Recurse into all values
            for v in map.values_mut() {
                fix_value(v);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                fix_value(v);
            }
        }
        _ => {}
    }
}

pub fn extract_tools(spec: &OpenAPI) -> Vec<Tool> {
    let mut tools = Vec::new();

    for (path, path_item) in spec.paths.iter() {
        let path_item = match path_item {
            openapiv3::ReferenceOr::Item(item) => item,
            _ => continue,
        };

        if let Some(op) = &path_item.get {
            if let Some(t) = make_tool(path, "GET", op) {
                tools.push(t);
            }
        }
        if let Some(op) = &path_item.post {
            if let Some(t) = make_tool(path, "POST", op) {
                tools.push(t);
            }
        }
    }

    tools
}

fn make_tool(path: &str, method: &str, op: &openapiv3::Operation) -> Option<Tool> {
    let name = op.operation_id.clone()
        .unwrap_or_else(|| format!("{}{}", method.to_lowercase(), path.replace('/', "_")));

    let description = op.summary.clone()
        .or_else(|| op.description.clone())
        .unwrap_or_else(|| format!("{} {}", method, path));

    let schema = json!({"type": "object", "properties": {}});

    Some(Tool {
        name,
        description,
        path: path.to_string(),
        method: method.to_string(),
        schema,
    })
}
