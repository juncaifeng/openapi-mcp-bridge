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
    // Generate a descriptive name using operationId or path-based name
    let name = if let Some(op_id) = &op.operation_id {
        // If operationId is too simple (e.g., "tree"), enhance it with path info
        if op_id.len() < 5 || !op_id.contains('_') {
            // Create a more descriptive name from path
            let path_parts: Vec<&str> = path.split('/')
                .filter(|p| !p.is_empty() && !p.starts_with('{'))
                .collect();
            if path_parts.is_empty() {
                format!("{}_{}", method.to_lowercase(), op_id)
            } else {
                format!("{}_{}", path_parts.join("_"), op_id)
            }
        } else {
            op_id.clone()
        }
    } else {
        // Fallback to method + path
        format!("{}{}", method.to_lowercase(), path.replace('/', "_"))
    };

    let description = op.summary.clone()
        .or_else(|| op.description.clone())
        .unwrap_or_else(|| format!("{} {}", method, path));

    // Extract parameters schema
    let mut properties = serde_json::Map::new();
    let mut required_params = Vec::new();

    for param_ref in &op.parameters {
        let param = match param_ref {
            openapiv3::ReferenceOr::Item(p) => p,
            _ => continue,
        };

        let (param_name, is_required) = match param {
            openapiv3::Parameter::Query { parameter_data, .. } |
            openapiv3::Parameter::Path { parameter_data, .. } |
            openapiv3::Parameter::Header { parameter_data, .. } => {
                (&parameter_data.name, parameter_data.required)
            }
            _ => continue,
        };

        if is_required {
            required_params.push(param_name.clone());
        }

        // Convert OpenAPI parameter to JSON Schema
        let mut schema = serde_json::Map::new();
        schema.insert("type".to_string(), serde_json::json!("string"));
        schema.insert("description".to_string(), serde_json::json!(format!("Parameter {}", param_name)));

        properties.insert(param_name.clone(), serde_json::Value::Object(schema));
    }

    let schema = if properties.is_empty() {
        json!({"type": "object", "properties": {}})
    } else {
        let mut schema_map = serde_json::Map::new();
        schema_map.insert("type".to_string(), serde_json::json!("object"));
        schema_map.insert("properties".to_string(), serde_json::Value::Object(properties));
        if !required_params.is_empty() {
            schema_map.insert("required".to_string(), serde_json::json!(required_params));
        }
        serde_json::Value::Object(schema_map)
    };

    Some(Tool {
        name,
        description,
        path: path.to_string(),
        method: method.to_string(),
        schema,
    })
}
