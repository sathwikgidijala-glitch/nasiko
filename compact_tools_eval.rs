//! Offline compact tool schema evaluation.
//!
//! EVAL_SET=/tmp/compact-tools-eval.json OUT=/tmp/out.jsonl \
//! cargo run --release -p nasiko-llm-router --example compact_tools_eval
use nasiko_tool_compact::{
    DecodeItem, JsonSchema, StreamDecoder, ToolCompactError, ToolDef, decode_calls, encode_tools,
};
use serde_json::{Value, json};
use std::io::Write;

fn tool_from_value(value: &Value) -> ToolDef {
    let parameters = value["function"]["parameters"].clone();
    ToolDef {
        name: value["function"]["name"]
            .as_str()
            .or_else(|| value["name"].as_str())
            .expect("tool name")
            .into(),
        description: value["function"]["description"]
            .as_str()
            .or_else(|| value["description"].as_str())
            .map(str::to_owned),
        parameters: JsonSchema {
            value: if parameters.is_null() {
                json!({"type":"object","properties":{}})
            } else {
                parameters
            },
        },
    }
}

fn selected_tools(all: &[ToolDef], names: Option<&Value>) -> Vec<ToolDef> {
    let Some(names) = names.and_then(Value::as_array) else {
        return all.to_vec();
    };
    all.iter()
        .filter(|tool| {
            names
                .iter()
                .any(|name| name.as_str() == Some(tool.name.as_str()))
        })
        .cloned()
        .collect()
}

fn call_json(call: nasiko_tool_compact::ToolCall) -> Value {
    json!({"name": call.name, "arguments": call.arguments})
}

fn decoded_json(items: Vec<DecodeItem>) -> Value {
    json!({
        "calls": items.into_iter().filter_map(|item| match item {
            DecodeItem::ToolCall(call) => Some(call_json(call)),
            DecodeItem::Text(_) => None,
        }).collect::<Vec<_>>()
    })
}

fn error_json(error: &ToolCompactError) -> Value {
    let code = match error {
        ToolCompactError::UnknownTool { .. } => "unknown_tool",
        ToolCompactError::InvalidArguments { .. }
        | ToolCompactError::MalformedCall { .. }
        | ToolCompactError::UnterminatedCall
        | ToolCompactError::InvalidJson(_) => "invalid_arguments",
        _ => "invalid_arguments",
    };
    json!({"error": code})
}

fn render_expected_calls(expected: &[Value]) -> String {
    expected
        .iter()
        .filter_map(|item| {
            Some(format!(
                "<<call {} {}>>",
                item["name"].as_str()?,
                serde_json::to_string(&item["arguments"]).ok()?
            ))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() {
    let path = std::env::var("EVAL_SET").expect("set EVAL_SET to the eval JSON path");
    let out_path = std::env::var("OUT").unwrap_or_else(|_| "compact-tools-out.jsonl".into());
    let data: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("read EVAL_SET"))
        .expect("valid eval JSON");
    let all_tools = data["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .map(tool_from_value)
        .collect::<Vec<_>>();
    let mut out = std::io::BufWriter::new(std::fs::File::create(out_path).expect("create OUT"));

    for case in data["cases"].as_array().expect("cases array") {
        let id = case["id"].as_str().expect("case id");
        let tools = selected_tools(&all_tools, case.get("tools"));
        let compact = encode_tools(&tools).expect("encode tools");
        let expected = case["expected"].as_array().cloned().unwrap_or_default();
        let rendered_calls = render_expected_calls(&expected);
        let roundtrip_calls = decode_calls(&rendered_calls, &tools)
            .expect("rendered calls must decode")
            .into_iter()
            .map(call_json)
            .collect::<Vec<_>>();
        let mut decoder = StreamDecoder::new(&tools).expect("decoder");
        for chunk in rendered_calls.as_bytes().chunks(3) {
            decoder
                .push(std::str::from_utf8(chunk).expect("utf8"))
                .expect("stream chunk");
        }
        let decoded = match decoder.finish() {
            Ok(items) => decoded_json(items),
            Err(error) => error_json(&error),
        };
        let compact_request = json!({
            "messages": case.get("messages").cloned().unwrap_or_else(|| json!([])),
            "tools": Value::Null,
            "compact_tools": compact.text,
        });
        let line = json!({
            "id": id,
            "compact_request": compact_request,
            "compacted": true,
            "rendered_calls": rendered_calls,
            "roundtrip_calls": roundtrip_calls,
            "decoded": decoded,
        });
        writeln!(out, "{line}").expect("write OUT");
    }

    for case in data["decoder_cases"].as_array().into_iter().flatten() {
        let id = case["id"].as_str().expect("decoder case id");
        let tools = selected_tools(&all_tools, case.get("tools"));
        let mut decoder = StreamDecoder::new(&tools).expect("decoder");
        let mut push_error = None;
        if let Some(chunks) = case["chunks"].as_array() {
            for chunk in chunks.iter().filter_map(Value::as_str) {
                if let Err(error) = decoder.push(chunk) {
                    push_error = Some(error);
                    break;
                }
            }
        }
        let decoded = match push_error {
            Some(error) => error_json(&error),
            None => match decoder.finish() {
                Ok(items) => decoded_json(items),
                Err(error) => error_json(&error),
            },
        };
        writeln!(out, "{}", json!({"id": id, "decoded": decoded})).expect("write decoder output");
    }
    out.flush().expect("flush OUT");
}
