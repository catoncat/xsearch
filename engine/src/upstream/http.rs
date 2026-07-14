use super::{ChatMessage, ChatResponse, ChatSource, ChatUpstream};
use crate::error::UpstreamError;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

pub struct HttpChatUpstream {
    client: Client,
    api_url: String,
    api_key: String,
}

impl HttpChatUpstream {
    pub fn new(api_url: String, api_key: String, timeout_secs: u64) -> Result<Self, UpstreamError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .map_err(|e| UpstreamError::Network(e.to_string()))?;
        Ok(Self {
            client,
            api_url: api_url.trim_end_matches('/').to_string(),
            api_key,
        })
    }

    pub fn from_env(timeout_secs: u64) -> Result<Self, UpstreamError> {
        let api_url = std::env::var("XSEARCH_API_URL").map_err(|_| {
            UpstreamError::Network(
                "missing API URL (set XSEARCH_API_URL or ~/.config/xsearch/config.toml)".into(),
            )
        })?;
        let api_key = std::env::var("XSEARCH_API_KEY").unwrap_or_default();
        Self::new(api_url, api_key, timeout_secs)
    }

    pub fn from_resolved(
        api_url: String,
        api_key: Option<String>,
        timeout_secs: u64,
    ) -> Result<Self, UpstreamError> {
        Self::new(api_url, api_key.unwrap_or_default(), timeout_secs)
    }
}

#[async_trait]
impl ChatUpstream for HttpChatUpstream {
    async fn complete(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
    ) -> Result<ChatResponse, UpstreamError> {
        let url = format!("{}/chat/completions", self.api_url);
        let body = json!({
            "model": model,
            "messages": messages.iter().map(|m| json!({
                "role": m.role,
                "content": m.content,
            })).collect::<Vec<_>>(),
            "stream": false,
        });

        let mut req = self.client.post(&url).json(&body);
        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }

        let resp = req.send().await.map_err(|e| {
            if e.is_timeout() {
                UpstreamError::Timeout
            } else {
                UpstreamError::Network(e.to_string())
            }
        })?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| UpstreamError::Network(e.to_string()))?;

        if !status.is_success() {
            return Err(UpstreamError::Http {
                status: status.as_u16(),
                body: text.chars().take(500).collect(),
            });
        }

        let value: Value = serde_json::from_str(&text)
            .map_err(|e| UpstreamError::Network(format!("invalid JSON: {e}")))?;

        let content = value
            .pointer("/choices/0/message/content")
            .and_then(|c| c.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                value
                    .pointer("/choices/0/delta/content")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_default();

        if content.trim().is_empty() {
            return Err(UpstreamError::EmptyModelOutput);
        }

        let sources = value
            .get("search_sources")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|source| {
                let url = source.get("url")?.as_str()?.trim();
                if url.is_empty() {
                    return None;
                }
                Some(ChatSource {
                    title: source
                        .get("title")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    url: url.to_string(),
                    source_type: source
                        .get("type")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                })
            })
            .collect();

        Ok(ChatResponse { content, sources })
    }
}
