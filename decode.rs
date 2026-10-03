use crate::schema::validate_arguments;
use crate::{DecodeItem, Result, ToolCall, ToolCompactError, ToolDef};
use serde_json::Value;
use std::collections::HashMap;

const OPEN: &str = "<<call";

pub fn decode_calls(text: &str, tools: &[ToolDef]) -> Result<Vec<ToolCall>> {
    Ok(decode_items(text, tools)?
        .into_iter()
        .filter_map(|item| match item {
            DecodeItem::ToolCall(call) => Some(call),
            DecodeItem::Text(_) => None,
        })
        .collect())
}

pub fn decode_items(text: &str, tools: &[ToolDef]) -> Result<Vec<DecodeItem>> {
    let lookup = tool_lookup(tools)?;
    let mut items = Vec::new();
    let mut cursor = 0;
    while let Some(relative) = text[cursor..].find(OPEN) {
        let start = cursor + relative;
        if start > cursor {
            items.push(DecodeItem::Text(text[cursor..start].to_owned()));
        }
        let (call, end) = parse_call(&text[start..], &lookup)?;
        items.push(DecodeItem::ToolCall(call));
        cursor = start + end;
    }
    if cursor < text.len() {
        items.push(DecodeItem::Text(text[cursor..].to_owned()));
    }
    Ok(items)
}

fn tool_lookup<'a>(tools: &'a [ToolDef]) -> Result<HashMap<&'a str, &'a ToolDef>> {
    let mut map = HashMap::new();
    for tool in tools {
        if map.insert(tool.name.as_str(), tool).is_some() {
            return Err(ToolCompactError::DuplicateToolName {
                name: tool.name.clone(),
            });
        }
    }
    Ok(map)
}

fn parse_call(input: &str, tools: &HashMap<&str, &ToolDef>) -> Result<(ToolCall, usize)> {
    let rest = input
        .strip_prefix(OPEN)
        .ok_or_else(|| ToolCompactError::MalformedCall {
            reason: "missing opening marker".into(),
        })?;
    let rest =
        rest.strip_prefix(char::is_whitespace)
            .ok_or_else(|| ToolCompactError::MalformedCall {
                reason: "expected whitespace after marker".into(),
            })?;
    let name_end =
        rest.find(char::is_whitespace)
            .ok_or_else(|| ToolCompactError::MalformedCall {
                reason: "missing tool name or JSON arguments".into(),
            })?;
    let name = &rest[..name_end];
    let tool = tools
        .get(name)
        .ok_or_else(|| ToolCompactError::UnknownTool { name: name.into() })?;
    let after_name = rest[name_end..].trim_start();
    let json_start = input.len() - after_name.len();
    let close = find_close(after_name)?;
    let json_text = after_name[..close].trim();
    let arguments: Value =
        serde_json::from_str(json_text).map_err(|e| ToolCompactError::MalformedCall {
            reason: e.to_string(),
        })?;
    if !arguments.is_object() {
        return Err(ToolCompactError::InvalidArguments {
            tool: name.into(),
            reason: "arguments must be a JSON object".into(),
        });
    }
    validate_arguments(name, &tool.parameters.value, &arguments)?;
    Ok((
        ToolCall {
            name: name.into(),
            arguments,
        },
        json_start + close + 2,
    ))
}

fn find_close(input: &str) -> Result<usize> {
    let bytes = input.as_bytes();
    let mut in_string = false;
    let mut escaped = false;
    for i in 0..bytes.len().saturating_sub(1) {
        let b = bytes[i];
        if in_string {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_string = false;
            }
        } else if b == b'"' {
            in_string = true;
        } else if b == b'>' && bytes[i + 1] == b'>' {
            return Ok(i);
        }
    }
    if in_string {
        return Err(ToolCompactError::UnterminatedCall);
    }
    Err(ToolCompactError::UnterminatedCall)
}
