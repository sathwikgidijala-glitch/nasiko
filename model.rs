use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub struct ToolDef {
    pub name: String,
    pub description: Option<String>,
    pub parameters: JsonSchema,
}

#[derive(Clone, Debug, PartialEq)]
pub struct JsonSchema {
    pub value: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
}

impl ToolCall {
    pub fn arguments_json(&self) -> crate::Result<String> {
        serde_json::to_string(&self.arguments).map_err(crate::ToolCompactError::InvalidJson)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompactTools {
    pub text: String,
    pub tools: Vec<ToolDef>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DecodeItem {
    Text(String),
    ToolCall(ToolCall),
}
