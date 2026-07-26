mod bounded;
mod client;
mod error;
pub mod http;
mod parse;
mod sanitize;
mod types;
mod validate;

pub use client::{chat_completions_url, LlmClient, LlmClientConfig};
pub use error::{LlmError, Result as LlmResult};
pub use http::{HttpTransport, ReqwestTransport};
pub use parse::extract_json_text;
pub use types::ChatMessage;

#[cfg(test)]
pub use http::mock::{MockResponse, MockTransport};
