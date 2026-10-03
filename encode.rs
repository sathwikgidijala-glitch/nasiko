use crate::schema::{render_signature, validate_schema_for_compaction};
use crate::{CompactTools, JsonSchema, Result, ToolCompactError, ToolDef};
use std::collections::HashSet;

fn validate_tools(tools: &[ToolDef]) -> Result<()> {
    if tools.is_empty() {
        return Err(ToolCompactError::EmptyToolList);
    }
    let mut seen = HashSet::new();
    for tool in tools {
        if tool.name.is_empty()
            || !tool
                .name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
            || tool.name.starts_with(|c: char| c.is_ascii_digit())
        {
            return Err(ToolCompactError::InvalidToolName {
                name: tool.name.clone(),
            });
        }
        if !seen.insert(&tool.name) {
            return Err(ToolCompactError::DuplicateToolName {
                name: tool.name.clone(),
            });
        }
        validate_schema_for_compaction(&tool.name, &tool.parameters.value)?;
    }
    Ok(())
}

pub fn encode_tools(tools: &[ToolDef]) -> Result<CompactTools> {
    validate_tools(tools)?;
    let mut text = String::from("<tools>\n");
    for tool in tools {
        text.push_str(&tool.name);
        text.push_str(&render_signature(&tool.parameters.value));
        if let Some(description) = &tool.description {
            if !description.trim().is_empty() {
                text.push_str(" - ");
                text.push_str(description.trim());
            }
        }
        text.push('\n');
    }
    text.push_str(
        "</tools>\n\nTo call a tool, emit exactly:\n<<call tool_name {\"argument\":\"value\"}>>",
    );
    Ok(CompactTools {
        text,
        tools: tools.to_vec(),
    })
}

pub fn encode_tool(tool: &ToolDef) -> Result<String> {
    Ok(encode_tools(std::slice::from_ref(tool))?.text)
}

pub fn decode_tools(compact: &CompactTools) -> Result<Vec<ToolDef>> {
    validate_tools(&compact.tools)?;
    Ok(compact.tools.clone())
}

#[allow(dead_code)]
fn _schema(_schema: JsonSchema) {}
