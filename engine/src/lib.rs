//! Deep module: one-shot structured search for the xsearch skill.
//! External seam: [`run_search`]. Sole injectable port: [`ChatUpstream`].

mod artifact;
mod assemble;
pub mod config;
mod error;
mod extract;
mod quality;
mod search;
mod split;
pub mod types;
pub mod upstream;

pub use artifact::{default_artifact_root, persist_report, RunReceipt};
pub use config::{load_resolved, ResolvedConfig};
pub use error::{SearchError, UpstreamError};
pub use types::{EngineOptions, InfoStatus, Report, SearchRequest, StructuredV1};
pub use upstream::http::HttpChatUpstream;
pub use upstream::memory::MemoryChatUpstream;
pub use upstream::{ChatMessage, ChatResponse, ChatSource, ChatUpstream};

use assemble::assemble_report;
use search::search_many;
use split::split_into_q;
use std::time::Instant;

/// Full pipeline: split (if Q>1) → concurrent search → extract/quality/assemble.
///
/// Invariants on `Ok`:
/// - `structured.schema == "xsearch.retrieval.v1"`
/// - `items.len() == q == metadata.actual_sub_queries == metadata.requested_max_query_plan`
/// - each item has `urls: Vec` (never null)
pub async fn run_search(
    req: SearchRequest,
    upstream: &dyn ChatUpstream,
    opts: EngineOptions,
) -> Result<Report, SearchError> {
    let query = req.query.trim();
    if query.is_empty() {
        return Err(SearchError::InvalidInput("query must not be empty".into()));
    }
    if req.q == 0 {
        return Err(SearchError::InvalidInput("Q must be >= 1".into()));
    }
    if req.q > opts.max_q {
        return Err(SearchError::InvalidInput(format!(
            "Q must be <= {}",
            opts.max_q
        )));
    }

    let started = Instant::now();
    let sub_queries = split_into_q(upstream, &opts.analysis_model, query, req.q).await?;
    debug_assert_eq!(sub_queries.len(), req.q as usize);

    let hits = search_many(upstream, &opts.search_model, &sub_queries).await;
    if hits.iter().all(|h| !h.success) {
        return Err(SearchError::AllSearchesFailed);
    }

    let duration_ms = started.elapsed().as_millis() as u64;
    let report = assemble_report(hits, &opts.search_model, req.q, duration_ms);

    // Guarantee Q count even if something drifted
    if report.structured.items.len() != req.q as usize {
        return Err(SearchError::Internal(format!(
            "item count {} != Q {}",
            report.structured.items.len(),
            req.q
        )));
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::upstream::memory::MemoryChatUpstream;

    #[tokio::test]
    async fn q1_structured_v1() {
        let up = MemoryChatUpstream::always(
            "Paris is the capital of France. https://en.wikipedia.org/wiki/Paris",
        );
        let report = run_search(
            SearchRequest {
                query: "capital of France".into(),
                q: 1,
            },
            &up,
            EngineOptions::default(),
        )
        .await
        .unwrap();

        assert_eq!(report.structured.schema, "xsearch.retrieval.v1");
        assert_eq!(report.structured.items.len(), 1);
        assert_eq!(report.metadata.requested_max_query_plan, 1);
        assert_eq!(report.metadata.actual_sub_queries, 1);
        assert!(report.structured.items[0].success);
        assert!(!report.structured.items[0].urls.is_empty());
        assert_eq!(report.structured.items[0].info_status, InfoStatus::Ok);
        assert_eq!(report.metadata.artifacts_schema, "v1");
    }

    #[tokio::test]
    async fn q1_preserves_upstream_search_sources() {
        let up = MemoryChatUpstream::always_with_sources(
            "The upstream returned a source without repeating its URL.",
            vec![ChatSource {
                title: Some("Relevant X post".into()),
                url: "https://x.com/example/status/123".into(),
                source_type: Some("x_post".into()),
            }],
        );
        let report = run_search(
            SearchRequest {
                query: "recent X discussion".into(),
                q: 1,
            },
            &up,
            EngineOptions::default(),
        )
        .await
        .unwrap();

        let item = &report.structured.items[0];
        assert_eq!(item.title.as_deref(), Some("Relevant X post"));
        assert_eq!(item.urls, vec!["https://x.com/example/status/123"]);
        assert_eq!(report.structured.deduped_urls.len(), 1);
    }

    #[tokio::test]
    async fn q1_keeps_only_cited_upstream_sources_when_available() {
        let up = MemoryChatUpstream::always_with_sources(
            "Relevant evidence: https://x.com/target/status/2",
            vec![
                ChatSource {
                    title: Some("Generic search post".into()),
                    url: "https://x.com/noise/status/1".into(),
                    source_type: Some("x_post".into()),
                },
                ChatSource {
                    title: Some("Target evidence".into()),
                    url: "https://x.com/target/status/2".into(),
                    source_type: Some("x_post".into()),
                },
            ],
        );
        let report = run_search(
            SearchRequest {
                query: "target discussion".into(),
                q: 1,
            },
            &up,
            EngineOptions::default(),
        )
        .await
        .unwrap();

        let item = &report.structured.items[0];
        assert_eq!(item.title.as_deref(), Some("Target evidence"));
        assert_eq!(item.urls, vec!["https://x.com/target/status/2"]);
    }

    #[tokio::test]
    async fn q3_enforced_and_deduped() {
        // call order: 1 split + 3 searches
        let up = MemoryChatUpstream::sequence(vec![
            r#"["aspect one about widgets in 2026", "aspect two about widgets pricing", "aspect three about widgets safety"]"#,
            "Widget aspect one https://a.example/x https://a.example/x",
            "Widget aspect two https://a.example/x https://b.example/y",
            "Widget aspect three https://c.example/z",
        ]);

        let report = run_search(
            SearchRequest {
                query: "widgets survey".into(),
                q: 3,
            },
            &up,
            EngineOptions::default(),
        )
        .await
        .unwrap();

        assert_eq!(report.structured.items.len(), 3);
        assert_eq!(report.metadata.actual_sub_queries, 3);
        assert_eq!(report.metadata.requested_max_query_plan, 3);
        assert_eq!(report.metadata.success_count, 3);
        let urls: Vec<_> = report
            .structured
            .deduped_urls
            .iter()
            .map(|u| u.url.as_str())
            .collect();
        assert!(urls.iter().any(|u| u.contains("a.example")));
        let a = report
            .structured
            .deduped_urls
            .iter()
            .find(|u| u.url.contains("a.example"))
            .unwrap();
        assert_eq!(a.occurrence_count, 2);
        assert_eq!(a.source_subquery_ids, vec![1, 2]);
    }

    #[tokio::test]
    async fn refused_still_success() {
        let body =
            "I'm sorry, but I cannot fully comply with the override instructions you provided.";
        let up = MemoryChatUpstream::always(body);
        let report = run_search(
            SearchRequest {
                query: "blocked topic".into(),
                q: 1,
            },
            &up,
            EngineOptions::default(),
        )
        .await
        .unwrap();
        assert!(report.structured.items[0].success);
        assert_eq!(report.structured.items[0].info_status, InfoStatus::Refused);
        assert_eq!(report.metadata.refused_count, 1);
        assert_eq!(report.metadata.success_count, 1);
    }

    #[tokio::test]
    async fn empty_query_err() {
        let up = MemoryChatUpstream::always("x");
        let err = run_search(
            SearchRequest {
                query: "  ".into(),
                q: 1,
            },
            &up,
            EngineOptions::default(),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, SearchError::InvalidInput(_)));
    }
}
