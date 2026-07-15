use crate::assemble::SearchHit;
use crate::upstream::{ChatMessage, ChatUpstream};
use futures::{stream, StreamExt};

const MAX_CONCURRENT_SEARCHES: usize = 4;

fn search_system_prompt() -> &'static str {
    r#"You are a web research assistant. Answer the user's question with factual, current public information using the retrieval capabilities available in this runtime.

Rewrite the request into concrete evidence needs, search before answering, and prefer primary sources. For X research, exact terms and semantic discovery may complement each other when the runtime supports them. Treat any native search tools as optional implementation details.

Keep a result only when it names the target entity or directly establishes the requested relationship through an official source, quoted context, or thread connection. Generic topical overlap is noise. Cite every retained result with a concrete returned https URL. Never invent a URL or claim retrieval that did not occur. Be concise but complete."#
}

pub async fn search_one(upstream: &dyn ChatUpstream, model: &str, sub_query: &str) -> SearchHit {
    let result = upstream
        .complete(
            model,
            vec![
                ChatMessage::system(search_system_prompt()),
                ChatMessage::user(sub_query),
            ],
        )
        .await;

    match result {
        Ok(response) => SearchHit {
            sub_question: sub_query.to_string(),
            success: true,
            body: response.content,
            sources: response.sources,
        },
        Err(e) => SearchHit {
            sub_question: sub_query.to_string(),
            success: false,
            body: e.to_string(),
            sources: Vec::new(),
        },
    }
}

/// Concurrent search; output order matches input order.
pub async fn search_many(
    upstream: &dyn ChatUpstream,
    model: &str,
    sub_queries: &[String],
) -> Vec<SearchHit> {
    stream::iter(sub_queries)
        .map(|query| search_one(upstream, model, query))
        .buffered(MAX_CONCURRENT_SEARCHES)
        .collect()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_keeps_native_tools_optional() {
        let prompt = search_system_prompt();
        assert!(prompt.contains("retrieval capabilities available"));
        assert!(prompt.contains("optional implementation details"));
        assert!(prompt.contains("exact terms and semantic discovery"));
        assert!(prompt.contains("Generic topical overlap is noise"));
        assert!(!prompt.contains("x_keyword_search"));
        assert!(!prompt.contains("x_semantic_search"));
    }
}
