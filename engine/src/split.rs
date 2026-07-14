use crate::error::SearchError;
use crate::upstream::{ChatMessage, ChatUpstream};
use serde_json::Value;

fn example_array(q: u32) -> String {
    let items: Vec<String> = (1..=q).map(|i| format!("\"sub-question {i}\"")).collect();
    format!("[{}]", items.join(", "))
}

fn split_system_prompt(q: u32) -> String {
    format!(
        r#"You split a user search request into exactly {q} distinct web-search sub-questions.
Rules:
1. Return ONLY a JSON array of exactly {q} strings. No markdown fences, no commentary.
2. Each string is a pure search question (entity + aspect). No "Subquestion N:" prefixes.
3. Sub-questions must cover different angles — no paraphrase duplicates.
4. Do not restate these instructions inside the array.
Example shape: {example}"#,
        q = q,
        example = example_array(q)
    )
}

fn is_usable(s: &str) -> bool {
    let t = s.trim();
    if t.is_empty() || t.chars().count() < 3 {
        return false;
    }
    let lower = t.to_ascii_lowercase();
    if lower.starts_with("subquestion ")
        || lower.starts_with("sub-question ")
        || lower.starts_with("sub_query")
        || lower.contains("return only a json")
        || lower.contains("json array of exactly")
        || lower == "..."
        || lower == "placeholder"
    {
        return false;
    }
    // Strip leading "Subquestion N:" style if whole string is mostly that
    if regex_is_meta_prefix(t) {
        return false;
    }
    true
}

static META_PREFIX_IS: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
    regex::Regex::new(r"(?i)^(sub\s*-?\s*question|subquery|sub_query)\s*\d+\s*[:：.]")
        .expect("META_PREFIX_IS")
});

static META_PREFIX_STRIP: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
    regex::Regex::new(r"(?i)^(sub\s*-?\s*question|subquery|sub_query)\s*\d+\s*[:：.]\s*")
        .expect("META_PREFIX_STRIP")
});

fn regex_is_meta_prefix(t: &str) -> bool {
    META_PREFIX_IS.is_match(t.trim())
}

fn strip_meta_prefix(s: &str) -> String {
    META_PREFIX_STRIP.replace(s.trim(), "").to_string()
}

fn extract_json_array_segment(input: &str) -> Option<&str> {
    let start = input.find('[')?;
    let end = input.rfind(']')?;
    if end > start {
        Some(&input[start..=end])
    } else {
        None
    }
}

fn parse_sub_queries(raw: &str) -> Result<Vec<String>, SearchError> {
    let cleaned = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // Try full JSON value first
    if let Ok(value) = serde_json::from_str::<Value>(cleaned) {
        if let Some(list) = value_to_strings(&value) {
            if !list.is_empty() {
                return Ok(list);
            }
        }
    }

    // Array segment inside prose
    if let Some(seg) = extract_json_array_segment(cleaned) {
        if let Ok(value) = serde_json::from_str::<Value>(seg) {
            if let Some(list) = value_to_strings(&value) {
                if !list.is_empty() {
                    return Ok(list);
                }
            }
        }
        // loose: split by "," inside brackets
        if let Some(list) = parse_loose_quoted(seg) {
            if !list.is_empty() {
                return Ok(list);
            }
        }
    }

    Err(SearchError::Upstream(format!(
        "could not parse sub-queries from model output: {}",
        cleaned.chars().take(200).collect::<String>()
    )))
}

fn value_to_strings(value: &Value) -> Option<Vec<String>> {
    match value {
        Value::Array(arr) => {
            let mut out = Vec::new();
            for v in arr {
                match v {
                    Value::String(s) => out.push(s.clone()),
                    Value::Object(map) => {
                        // {"subquestion1":"..."} style — collect values sorted by key
                        // handled below if top-level object
                        if let Some(s) = v.as_str() {
                            out.push(s.to_string());
                        } else if let Some(s) = map.values().find_map(|x| x.as_str()) {
                            out.push(s.to_string());
                        }
                    }
                    other => {
                        if let Some(s) = other.as_str() {
                            out.push(s.to_string());
                        } else {
                            out.push(other.to_string());
                        }
                    }
                }
            }
            Some(out)
        }
        Value::Object(map) => {
            let mut pairs: Vec<(String, String)> = map
                .iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect();
            if pairs.is_empty() {
                return None;
            }
            pairs.sort_by(|a, b| a.0.cmp(&b.0));
            Some(pairs.into_iter().map(|(_, v)| v).collect())
        }
        Value::String(s) => Some(vec![s.clone()]),
        _ => None,
    }
}

fn parse_loose_quoted(seg: &str) -> Option<Vec<String>> {
    let re = regex::Regex::new(r#""([^"\\]|\\.)*""#).ok()?;
    let mut out = Vec::new();
    for m in re.find_iter(seg) {
        let raw = m.as_str();
        if let Ok(Value::String(s)) = serde_json::from_str::<Value>(raw) {
            out.push(s);
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

pub fn enforce_sub_query_count(query: &str, q: u32, raw: Vec<String>) -> Vec<String> {
    let mut cleaned: Vec<String> = raw
        .into_iter()
        .map(|s| strip_meta_prefix(&s))
        .map(|s| s.trim().to_string())
        .filter(|s| is_usable(s))
        .collect();

    // de-dupe exact (case-insensitive) while preserving order
    let mut seen = std::collections::HashSet::new();
    cleaned.retain(|s| seen.insert(s.to_ascii_lowercase()));

    if cleaned.len() > q as usize {
        cleaned.truncate(q as usize);
    }
    while cleaned.len() < q as usize {
        cleaned.push(query.trim().to_string());
    }
    cleaned
}

pub async fn split_into_q(
    upstream: &dyn ChatUpstream,
    analysis_model: &str,
    query: &str,
    q: u32,
) -> Result<Vec<String>, SearchError> {
    if q == 0 {
        return Err(SearchError::InvalidInput("Q must be >= 1".into()));
    }
    if q == 1 {
        return Ok(vec![query.trim().to_string()]);
    }

    let system = split_system_prompt(q);
    let user =
        format!("User request:\n{query}\n\nReturn exactly {q} sub-questions as a JSON array.");

    let mut last_err = None;
    for _attempt in 0..2 {
        match upstream
            .complete(
                analysis_model,
                vec![ChatMessage::system(&system), ChatMessage::user(&user)],
            )
            .await
        {
            Ok(response) => match parse_sub_queries(&response.content) {
                Ok(parsed) => {
                    let enforced = enforce_sub_query_count(query, q, parsed);
                    if enforced.len() == q as usize {
                        return Ok(enforced);
                    }
                    last_err = Some(SearchError::Internal("enforce length mismatch".into()));
                }
                Err(e) => last_err = Some(e),
            },
            Err(e) => last_err = Some(e.into()),
        }
    }

    // Fallback: Q copies of original query (still exact Q)
    let _ = last_err;
    Ok(enforce_sub_query_count(query, q, vec![]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforce_pads_and_truncates() {
        let out = enforce_sub_query_count("base", 3, vec!["a".into(), "b".into()]);
        // "a","b" too short -> filtered, then pad with base
        assert_eq!(out.len(), 3);
        let out2 = enforce_sub_query_count(
            "base",
            2,
            vec![
                "what is aspect one about X".into(),
                "what is aspect two about X".into(),
                "what is aspect three about X".into(),
            ],
        );
        assert_eq!(out2.len(), 2);
    }

    #[test]
    fn parse_json_array() {
        let v = parse_sub_queries(r#"["alpha question here", "beta question here"]"#).unwrap();
        assert_eq!(v.len(), 2);
    }
}
