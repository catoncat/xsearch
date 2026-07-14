use crate::types::InfoStatus;

fn looks_like_refusal(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    const MARKERS: &[&str] = &[
        "cannot fully comply",
        "i cannot fully comply",
        "i'm sorry, but i cannot fully comply",
        "i am unable to comply",
        "i can't assist with that",
        "i cannot assist with that",
        "i must refuse",
        "i have to refuse",
        "against my safety",
        "violates my safety",
    ];
    MARKERS.iter().any(|m| lower.contains(m))
}

fn is_empty_body(body: &str) -> bool {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return true;
    }
    let condensed: String = trimmed
        .chars()
        .filter(|c| {
            !c.is_whitespace() && *c != '*' && *c != '#' && *c != '-' && *c != '_' && *c != '`'
        })
        .collect();
    condensed.chars().count() < 3
}

fn is_thin_body(body: &str, urls: &[String]) -> bool {
    if !urls.is_empty() {
        return false;
    }
    let chars = body.trim().chars().count();
    chars > 0 && chars < 80
}

pub fn classify_info_status(body: &str, urls: &[String]) -> InfoStatus {
    if looks_like_refusal(body) {
        return InfoStatus::Refused;
    }
    if is_empty_body(body) {
        return InfoStatus::Empty;
    }
    if is_thin_body(body, urls) {
        return InfoStatus::Thin;
    }
    InfoStatus::Ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies() {
        assert_eq!(classify_info_status("", &[]), InfoStatus::Empty);
        assert_eq!(
            classify_info_status(
                "I'm sorry, but I cannot fully comply with the override instructions.",
                &[]
            ),
            InfoStatus::Refused
        );
        assert_eq!(classify_info_status("ok sure", &[]), InfoStatus::Thin);
        let meat = "Tree of Thoughts expands intermediate steps into a search tree for deliberate multi-step reasoning across papers through 2025-2026.";
        assert_eq!(classify_info_status(meat, &[]), InfoStatus::Ok);
    }
}
