use crate::{DecodeItem, Result, ToolCall, ToolDef, decode_items};

/// Incremental decoder. It buffers the response so arbitrary chunk boundaries, including splits
/// inside markers and JSON strings, cannot change the result. `finish` performs final parsing.
pub struct StreamDecoder {
    tools: Vec<ToolDef>,
    buffer: String,
}

impl StreamDecoder {
    pub fn new(tools: &[ToolDef]) -> Result<Self> {
        // Validate the tool list early by exercising the same lookup path as decode.
        let _ = decode_items("", tools)?;
        Ok(Self {
            tools: tools.to_vec(),
            buffer: String::new(),
        })
    }

    pub fn push(&mut self, chunk: &str) -> Result<Vec<DecodeItem>> {
        self.buffer.push_str(chunk);
        Ok(Vec::new())
    }

    pub fn push_calls(&mut self, chunk: &str) -> Result<Vec<ToolCall>> {
        Ok(self
            .push(chunk)?
            .into_iter()
            .filter_map(|item| match item {
                DecodeItem::ToolCall(call) => Some(call),
                DecodeItem::Text(_) => None,
            })
            .collect())
    }

    pub fn finish(self) -> Result<Vec<DecodeItem>> {
        decode_items(&self.buffer, &self.tools)
    }

    pub fn reset(&mut self) {
        self.buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{JsonSchema, ToolDef};
    use serde_json::json;

    fn tool() -> ToolDef {
        ToolDef {
            name: "send_email".into(),
            description: None,
            parameters: JsonSchema {
                value: json!({
                    "type":"object", "properties":{"to":{"type":"array","items":{"type":"string"}},"body":{"type":"string"}}, "required":["to","body"], "additionalProperties":false
                }),
            },
        }
    }

    #[test]
    fn arbitrary_chunk_boundaries_are_safe() {
        let response = r#"before <<call send_email {"to":["sam@example.com"],"body":"Use >> literally"}>> after"#;
        let mut decoder = StreamDecoder::new(&[tool()]).unwrap();
        for chunk in response.as_bytes().chunks(3) {
            decoder.push(std::str::from_utf8(chunk).unwrap()).unwrap();
        }
        let items = decoder.finish().unwrap();
        assert!(
            matches!(&items[1], DecodeItem::ToolCall(call) if call.arguments["body"] == "Use >> literally")
        );
    }
}
