use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InfoStatus {
    Ok,
    Empty,
    Refused,
    Thin,
}

impl InfoStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            InfoStatus::Ok => "ok",
            InfoStatus::Empty => "empty",
            InfoStatus::Refused => "refused",
            InfoStatus::Thin => "thin",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Item {
    pub index: u32,
    pub sub_question: String,
    pub success: bool,
    pub body: String,
    pub title: Option<String>,
    pub snippets: Vec<String>,
    pub urls: Vec<String>,
    pub info_status: InfoStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct DedupedUrl {
    pub url: String,
    pub title: Option<String>,
    pub source_subquery_ids: Vec<u32>,
    pub first_rank: u32,
    pub occurrence_count: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct InfoStatusCounts {
    pub ok: u32,
    pub empty: u32,
    pub refused: u32,
    pub thin: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct StructuredV1 {
    pub schema: &'static str,
    pub items: Vec<Item>,
    pub deduped_urls: Vec<DedupedUrl>,
    pub info_status_counts: InfoStatusCounts,
}

#[derive(Debug, Clone, Serialize)]
pub struct Metadata {
    pub duration_ms: u64,
    pub model: String,
    pub requested_max_query_plan: u32,
    pub actual_sub_queries: u32,
    pub success_count: u32,
    pub failure_count: u32,
    pub empty_count: u32,
    pub refused_count: u32,
    pub thin_count: u32,
    pub ok_count: u32,
    pub timestamp: String,
    pub artifacts_schema: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub structured: StructuredV1,
    pub metadata: Metadata,
}

impl Report {
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[derive(Debug, Clone)]
pub struct SearchRequest {
    pub query: String,
    pub q: u32,
}

#[derive(Debug, Clone)]
pub struct EngineOptions {
    pub search_model: String,
    pub analysis_model: String,
    pub timeout_secs: u64,
    pub max_q: u32,
}

impl Default for EngineOptions {
    fn default() -> Self {
        Self {
            search_model: "grok-4.3-fast".into(),
            analysis_model: "grok-4.3-fast".into(),
            timeout_secs: 600,
            max_q: 100,
        }
    }
}
