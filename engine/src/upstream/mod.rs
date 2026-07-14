pub mod http;
pub mod memory;

use crate::error::UpstreamError;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatSource {
    pub title: Option<String>,
    pub url: String,
    pub source_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub content: String,
    pub sources: Vec<ChatSource>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
        }
    }
}

/// Sole external Seam: OpenAI-compatible chat completion.
#[async_trait]
pub trait ChatUpstream: Send + Sync {
    async fn complete(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
    ) -> Result<ChatResponse, UpstreamError>;
}
