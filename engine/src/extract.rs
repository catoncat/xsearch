use once_cell::sync::Lazy;
use regex::Regex;

static URL_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"https?://[^\s)\]>]+").expect("URL_PATTERN"));

fn normalize_extracted_url(raw: &str) -> String {
    let mut url = raw.trim().to_string();
    while url
        .chars()
        .last()
        .is_some_and(|c| matches!(c, '.' | ',' | ';' | ':' | ')' | ']' | '>' | '"' | '\''))
    {
        url.pop();
    }
    url
}

/// Soft-extract http(s) URLs; de-dupe case-insensitively within one body.
pub fn extract_urls_from_text(text: &str) -> Vec<String> {
    let mut urls = Vec::new();
    for m in URL_PATTERN.find_iter(text) {
        let candidate = normalize_extracted_url(m.as_str());
        if candidate.is_empty() {
            continue;
        }
        if urls
            .iter()
            .any(|existing: &String| existing.eq_ignore_ascii_case(&candidate))
        {
            continue;
        }
        urls.push(candidate);
    }
    urls
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_punct_and_dedupes() {
        let text =
            "see https://Example.com/a). also https://example.com/a and https://other.test/b,";
        let urls = extract_urls_from_text(text);
        assert_eq!(urls.len(), 2);
        assert_eq!(urls[0], "https://Example.com/a");
        assert_eq!(urls[1], "https://other.test/b");
    }
}
