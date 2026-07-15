use crate::extract::extract_urls_from_text;
use crate::quality::classify_info_status;
use crate::types::{
    DedupedUrl, InfoStatus, InfoStatusCounts, Item, Metadata, Report, StructuredV1,
};
use crate::upstream::ChatSource;
use std::collections::HashMap;

pub struct SearchHit {
    pub sub_question: String,
    pub success: bool,
    pub body: String,
    pub sources: Vec<ChatSource>,
}

fn select_urls(sources: &[ChatSource], body: &str) -> Vec<String> {
    let cited_urls = extract_urls_from_text(body);
    if sources.is_empty() {
        return cited_urls;
    }

    let selected: Vec<String> = cited_urls
        .iter()
        .filter_map(|cited| {
            sources
                .iter()
                .find(|source| source.url.eq_ignore_ascii_case(cited))
                .map(|source| source.url.clone())
        })
        .collect();

    if selected.is_empty() {
        sources.iter().map(|source| source.url.clone()).collect()
    } else {
        selected
    }
}

fn select_title(sources: &[ChatSource], urls: &[String]) -> Option<String> {
    urls.iter().find_map(|url| {
        sources
            .iter()
            .find(|source| source.url.eq_ignore_ascii_case(url))
            .and_then(|source| source.title.clone())
    })
}

pub fn assemble_report(
    hits: Vec<SearchHit>,
    model: &str,
    requested_q: u32,
    duration_ms: u64,
) -> Report {
    let mut items = Vec::with_capacity(hits.len());
    for (i, hit) in hits.into_iter().enumerate() {
        let (urls, info_status, title) = if hit.success {
            let urls = select_urls(&hit.sources, &hit.body);
            let info_status = classify_info_status(&hit.body, &urls);
            let title = select_title(&hit.sources, &urls);
            (urls, info_status, title)
        } else {
            (Vec::new(), InfoStatus::Failed, None)
        };
        items.push(Item {
            index: (i + 1) as u32,
            sub_question: hit.sub_question,
            success: hit.success,
            body: hit.body,
            title,
            snippets: Vec::new(),
            urls,
            info_status,
        });
    }

    let deduped_urls = build_deduped_urls(&items);
    let mut counts = InfoStatusCounts {
        ok: 0,
        empty: 0,
        refused: 0,
        thin: 0,
        failed: 0,
    };
    for item in &items {
        match item.info_status {
            InfoStatus::Ok => counts.ok += 1,
            InfoStatus::Empty => counts.empty += 1,
            InfoStatus::Refused => counts.refused += 1,
            InfoStatus::Thin => counts.thin += 1,
            InfoStatus::Failed => counts.failed += 1,
        }
    }

    let success_count = items.iter().filter(|i| i.success).count() as u32;
    let failure_count = items.iter().filter(|i| !i.success).count() as u32;
    let actual = items.len() as u32;

    Report {
        structured: StructuredV1 {
            schema: "xsearch.retrieval.v1",
            items,
            deduped_urls,
            info_status_counts: counts.clone(),
        },
        metadata: Metadata {
            duration_ms,
            model: model.to_string(),
            requested_max_query_plan: requested_q,
            actual_sub_queries: actual,
            success_count,
            failure_count,
            empty_count: counts.empty,
            refused_count: counts.refused,
            thin_count: counts.thin,
            ok_count: counts.ok,
            timestamp: chrono::Utc::now().to_rfc3339(),
            artifacts_schema: "v1",
        },
    }
}

fn build_deduped_urls(items: &[Item]) -> Vec<DedupedUrl> {
    struct Acc {
        url: String,
        title: Option<String>,
        source_subquery_ids: Vec<u32>,
        first_rank: u32,
        first_item_index: u32,
    }

    let mut by_key: HashMap<String, Acc> = HashMap::new();
    for item in items {
        for (rank0, url) in item.urls.iter().enumerate() {
            let key = url.to_ascii_lowercase();
            let rank = (rank0 + 1) as u32;
            if let Some(entry) = by_key.get_mut(&key) {
                if !entry.source_subquery_ids.contains(&item.index) {
                    entry.source_subquery_ids.push(item.index);
                }
                if entry.title.is_none() {
                    entry.title = item.title.clone();
                }
            } else {
                by_key.insert(
                    key,
                    Acc {
                        url: url.clone(),
                        title: item.title.clone(),
                        source_subquery_ids: vec![item.index],
                        first_rank: rank,
                        first_item_index: item.index,
                    },
                );
            }
        }
    }

    let mut out: Vec<(u32, DedupedUrl)> = by_key
        .into_values()
        .map(|mut a| {
            a.source_subquery_ids.sort_unstable();
            a.source_subquery_ids.dedup();
            let occurrence_count = a.source_subquery_ids.len() as u32;
            (
                a.first_item_index,
                DedupedUrl {
                    url: a.url,
                    title: a.title,
                    source_subquery_ids: a.source_subquery_ids,
                    first_rank: a.first_rank,
                    occurrence_count,
                },
            )
        })
        .collect();

    out.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(a.1.first_rank.cmp(&b.1.first_rank))
            .then(a.1.url.cmp(&b.1.url))
    });
    out.into_iter().map(|(_, u)| u).collect()
}
