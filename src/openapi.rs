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
    let mut name_counter: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for (path, path_item) in spec.paths.iter() {
        let path_item = match path_item {
            openapiv3::ReferenceOr::Item(item) => item,
            _ => continue,
        };

        if let Some(op) = &path_item.get {
            if let Some(t) = make_tool(path, "GET", op, &mut name_counter) {
                tools.push(t);
            }
        }
        if let Some(op) = &path_item.post {
            if let Some(t) = make_tool(path, "POST", op, &mut name_counter) {
                tools.push(t);
            }
        }
        if let Some(op) = &path_item.put {
            if let Some(t) = make_tool(path, "PUT", op, &mut name_counter) {
                tools.push(t);
            }
        }
        if let Some(op) = &path_item.delete {
            if let Some(t) = make_tool(path, "DELETE", op, &mut name_counter) {
                tools.push(t);
            }
        }
        if let Some(op) = &path_item.patch {
            if let Some(t) = make_tool(path, "PATCH", op, &mut name_counter) {
                tools.push(t);
            }
        }
    }

    tools
}

fn make_tool(
    path: &str,
    method: &str,
    op: &openapiv3::Operation,
    name_counter: &mut std::collections::HashMap<String, usize>,
) -> Option<Tool> {
    // Extract resource name from path (first segment after /)
    let resource = path
        .split('/')
        .filter(|s| !s.is_empty() && !s.starts_with('{'))
        .next()
        .unwrap_or("api");

    // Get operation info
    let op_id = op.operation_id.as_deref().unwrap_or("unknown");
    let tags = op.tags.first().map(|s| s.as_str()).unwrap_or(resource);

    // Generate a descriptive name: method_resource_operation
    // Examples:
    //   GET /terms/{id} -> get_terms_detail
    //   POST /terms -> post_terms_create
    //   GET /categories/tree -> get_categories_tree
    let base_name = format!(
        "{}_{}_{}",
        method.to_lowercase(),
        resource.to_lowercase().replace('-', "_"),
        op_id.to_lowercase().replace('-', "_")
    );

    // Handle duplicates: if this name was seen before, append _2, _3, etc.
    let name = {
        let count = name_counter.entry(base_name.clone()).or_insert(0);
        *count += 1;
        if *count == 1 {
            base_name
        } else {
            format!("{}_{}", base_name, count)
        }
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

    // Handle request body for POST/PUT/PATCH
    if let Some(body) = &op.request_body {
        if let openapiv3::ReferenceOr::Item(body_data) = body {
            // Look for application/json content
            if let Some(json_content) = body_data.content.get("application/json") {
                if let Some(schema_ref) = &json_content.schema {
                    // Convert schema to JSON and extract properties
                    if let Ok(schema_json) = serde_json::to_value(schema_ref) {
                        if let Some(props) = schema_json.get("properties").and_then(|p| p.as_object()) {
                            for (prop_name, prop_value) in props {
                                properties.insert(prop_name.clone(), prop_value.clone());
                            }
                        }
                        // Extract required fields
                        if let Some(req_fields) = schema_json.get("required").and_then(|r| r.as_array()) {
                            for req_field in req_fields {
                                if let Some(field_name) = req_field.as_str() {
                                    if !required_params.contains(&field_name.to_string()) {
                                        required_params.push(field_name.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
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
