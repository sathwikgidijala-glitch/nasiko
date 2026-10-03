//! Compact tool definitions and fail-closed tool-call decoding.
//!
//! The crate is provider-independent: it renders a compact prompt representation and decodes
//! calls back into structured values. Router integrations own conversion to provider IR types.

mod decode;
mod encode;
mod error;
mod model;
mod schema;
mod stream;

pub use decode::{decode_calls, decode_items};
pub use encode::{decode_tools, encode_tool, encode_tools};
pub use error::{Result, ToolCompactError};
pub use model::{CompactTools, DecodeItem, JsonSchema, ToolCall, ToolDef};
pub use stream::StreamDecoder;
