use thiserror::Error;

#[derive(Debug, Error)]
pub enum SearchError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("config: {0}")]
    Config(String),
    #[error("upstream: {0}")]
    Upstream(String),
    #[error("all sub-searches failed")]
    AllSearchesFailed,
    #[error("internal: {0}")]
    Internal(String),
}

#[derive(Debug, Error)]
pub enum UpstreamError {
    #[error("invalid endpoint: {0}")]
    InvalidEndpoint(String),
    #[error("http {status}: {body}")]
    Http { status: u16, body: String },
    #[error("network: {0}")]
    Network(String),
    #[error("timeout")]
    Timeout,
    #[error("empty model output")]
    EmptyModelOutput,
}

impl From<UpstreamError> for SearchError {
    fn from(value: UpstreamError) -> Self {
        SearchError::Upstream(value.to_string())
    }
}
