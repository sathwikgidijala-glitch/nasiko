use crate::{Result, ToolCompactError};
use serde_json::Value;

pub(crate) fn validate_schema_for_compaction(tool: &str, schema: &Value) -> Result<()> {
    let Some(object) = schema.as_object() else {
        return Err(ToolCompactError::UnsupportedSchema {
            tool: tool.into(),
            reason: "root schema must be an object".into(),
        });
    };
    if object.get("type").and_then(Value::as_str) != Some("object") {
        return Err(ToolCompactError::UnsupportedSchema {
            tool: tool.into(),
            reason: "root schema must have type=object".into(),
        });
    }
    for key in object.keys() {
        if !matches!(
            key.as_str(),
            "type" | "properties" | "required" | "additionalProperties" | "description"
        ) {
            return Err(ToolCompactError::UnsupportedSchema {
                tool: tool.into(),
                reason: format!("unsupported root keyword `{key}`"),
            });
        }
    }
    if let Some(properties) = object.get("properties") {
        let Some(properties) = properties.as_object() else {
            return Err(ToolCompactError::UnsupportedSchema {
                tool: tool.into(),
                reason: "properties must be an object".into(),
            });
        };
        for (name, child) in properties {
            validate_value_schema(tool, name, child)?;
        }
    }
    Ok(())
}

fn validate_value_schema(tool: &str, path: &str, schema: &Value) -> Result<()> {
    let Some(object) = schema.as_object() else {
        return Err(ToolCompactError::UnsupportedSchema {
            tool: tool.into(),
            reason: format!("schema for `{path}` must be an object"),
        });
    };
    for key in object.keys() {
        if !matches!(
            key.as_str(),
            "type"
                | "description"
                | "format"
                | "enum"
                | "items"
                | "properties"
                | "required"
                | "additionalProperties"
        ) {
            return Err(ToolCompactError::UnsupportedSchema {
                tool: tool.into(),
                reason: format!("unsupported keyword `{key}` at `{path}`"),
            });
        }
    }
    if let Some(items) = object.get("items") {
        validate_value_schema(tool, &format!("{path}[]"), items)?;
    }
    if let Some(properties) = object.get("properties") {
        let Some(properties) = properties.as_object() else {
            return Err(ToolCompactError::UnsupportedSchema {
                tool: tool.into(),
                reason: format!("properties at `{path}` must be an object"),
            });
        };
        for (name, child) in properties {
            validate_value_schema(tool, &format!("{path}.{name}"), child)?;
        }
    }
    Ok(())
}

pub(crate) fn render_signature(schema: &Value) -> String {
    let props = schema.get("properties").and_then(Value::as_object);
    let required = schema.get("required").and_then(Value::as_array);
    let is_required = |name: &str| {
        required
            .map(|r| r.iter().any(|v| v.as_str() == Some(name)))
            .unwrap_or(false)
    };
    let fields = props
        .map(|props| {
            props
                .iter()
                .map(|(name, child)| {
                    let suffix = if is_required(name) { "" } else { "?" };
                    format!("{name}{suffix}:{}", render_type(child))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    format!("({})", fields.join(", "))
}

pub(crate) fn render_type(schema: &Value) -> String {
    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        let rendered = values
            .iter()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect::<Vec<_>>();
        if !rendered.is_empty() {
            return rendered.join("|");
        }
    }
    match schema.get("type").and_then(Value::as_str) {
        Some("array") => format!(
            "{}[]",
            schema
                .get("items")
                .map(render_type)
                .unwrap_or_else(|| "any".into())
        ),
        Some("object") => "object".into(),
        Some("integer") => "integer".into(),
        Some("number") => "number".into(),
        Some("boolean") => "boolean".into(),
        Some("string") => match schema.get("format").and_then(Value::as_str) {
            Some(format) => format.to_owned(),
            None => "string".into(),
        },
        Some("null") => "null".into(),
        Some(other) => other.into(),
        None => "any".into(),
    }
}

pub(crate) fn validate_arguments(tool: &str, schema: &Value, arguments: &Value) -> Result<()> {
    validate_value(tool, "$", schema, arguments)
}

fn validate_value(tool: &str, path: &str, schema: &Value, value: &Value) -> Result<()> {
    let fail = |reason: String| {
        Err(ToolCompactError::InvalidArguments {
            tool: tool.into(),
            reason: format!("{path}: {reason}"),
        })
    };
    match schema.get("type").and_then(Value::as_str) {
        Some("object") => {
            let Some(object) = value.as_object() else {
                return fail("expected object".into());
            };
            if let Some(required) = schema.get("required").and_then(Value::as_array) {
                for field in required.iter().filter_map(Value::as_str) {
                    if !object.contains_key(field) {
                        return fail(format!("missing required field `{field}`"));
                    }
                }
            }
            let properties = schema.get("properties").and_then(Value::as_object);
            if schema.get("additionalProperties").and_then(Value::as_bool) == Some(false) {
                for key in object.keys() {
                    if !properties.is_some_and(|p| p.contains_key(key)) {
                        return fail(format!("unknown field `{key}`"));
                    }
                }
            }
            if let Some(properties) = properties {
                for (key, child) in properties {
                    if let Some(actual) = object.get(key) {
                        validate_value(tool, &format!("{path}.{key}"), child, actual)?;
                    }
                }
            }
        }
        Some("string") => {
            if !value.is_string() {
                return fail("expected string".into());
            }
        }
        Some("integer") => {
            if value.as_i64().is_none() && value.as_u64().is_none() {
                return fail("expected integer".into());
            }
        }
        Some("number") => {
            if !value.is_number() {
                return fail("expected number".into());
            }
        }
        Some("boolean") => {
            if !value.is_boolean() {
                return fail("expected boolean".into());
            }
        }
        Some("array") => {
            let Some(array) = value.as_array() else {
                return fail("expected array".into());
            };
            if let Some(items) = schema.get("items") {
                for (i, item) in array.iter().enumerate() {
                    validate_value(tool, &format!("{path}[{i}]"), items, item)?;
                }
            }
        }
        Some("null") => {
            if !value.is_null() {
                return fail("expected null".into());
            }
        }
        Some(other) => return fail(format!("unsupported type `{other}`")),
        None => {}
    }
    if let Some(enums) = schema.get("enum").and_then(Value::as_array)
        && !enums.iter().any(|candidate| candidate == value)
    {
        return fail("value is not in enum".into());
    }
    Ok(())
}
