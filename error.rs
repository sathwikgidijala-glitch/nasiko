use thiserror::Error;

pub type Result<T> = std::result::Result<T, ToolCompactError>;

#[derive(Debug, Error)]
pub enum ToolCompactError {
    #[error("tool list is empty")]
    EmptyToolList,
    #[error("invalid tool name: {name}")]
    InvalidToolName { name: String },
    #[error("duplicate tool name: {name}")]
    DuplicateToolName { name: String },
    #[error("unsupported schema for tool `{tool}`: {reason}")]
    UnsupportedSchema { tool: String, reason: String },
    #[error("unknown tool: {name}")]
    UnknownTool { name: String },
    #[error("malformed tool call: {reason}")]
    MalformedCall { reason: String },
    #[error("unterminated tool call")]
    UnterminatedCall,
    #[error("invalid arguments for `{tool}`: {reason}")]
    InvalidArguments { tool: String, reason: String },
    #[error("invalid JSON in tool call: {0}")]
    InvalidJson(#[from] serde_json::Error),
}
