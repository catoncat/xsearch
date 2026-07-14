use super::{ChatMessage, ChatResponse, ChatSource, ChatUpstream};
use crate::error::UpstreamError;
use async_trait::async_trait;
use std::sync::Mutex;

/// Scripted upstream for tests. Pops responses in order.
pub struct MemoryChatUpstream {
    responses: Mutex<Vec<Result<ChatResponse, UpstreamError>>>,
}

impl MemoryChatUpstream {
    pub fn new(responses: Vec<Result<ChatResponse, UpstreamError>>) -> Self {
        Self {
            responses: Mutex::new(responses),
        }
    }

    pub fn always(body: impl Into<String>) -> Self {
        // Unlimited-ish: refill not needed if tests use finite calls;
        // provide many clones for concurrent search.
        let body = body.into();
        let list = (0..64)
            .map(|_| {
                Ok(ChatResponse {
                    content: body.clone(),
                    sources: Vec::new(),
                })
            })
            .collect::<Vec<_>>();
        Self::new(list)
    }

    pub fn sequence(bodies: Vec<&str>) -> Self {
        Self::new(
            bodies
                .into_iter()
                .map(|body| {
                    Ok(ChatResponse {
                        content: body.to_string(),
                        sources: Vec::new(),
                    })
                })
                .collect(),
        )
    }

    pub fn always_with_sources(body: impl Into<String>, sources: Vec<ChatSource>) -> Self {
        let body = body.into();
        let list = (0..64)
            .map(|_| {
                Ok(ChatResponse {
                    content: body.clone(),
                    sources: sources.clone(),
                })
            })
            .collect::<Vec<_>>();
        Self::new(list)
    }
}

#[async_trait]
impl ChatUpstream for MemoryChatUpstream {
    async fn complete(
        &self,
        _model: &str,
        _messages: Vec<ChatMessage>,
    ) -> Result<ChatResponse, UpstreamError> {
        let mut guard = self
            .responses
            .lock()
            .map_err(|_| UpstreamError::Network("memory lock poisoned".into()))?;
        if guard.is_empty() {
            return Err(UpstreamError::EmptyModelOutput);
        }
        guard.remove(0)
    }
}
