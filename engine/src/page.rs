//! Deep-read one URL: fetch → readable-article extraction → budgeted render.
//!
//! External seam: [`fetch_page`] (network, needs no API config).
//! [`extract_article`] and [`render_page`] are pure and unit-testable.
//! Format controls context, not network: the page is always fetched and
//! extracted in full; rendering decides how much reaches stdout.

use std::time::{Duration, Instant};

use dom_smoothie::Readability;
use thiserror::Error;

pub const DEFAULT_MAX_CHARS: usize = 500;
const FETCH_TIMEOUT_SECS: u64 = 60;
const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0 Safari/537.36";

#[derive(Debug, Error)]
pub enum PageError {
    #[error("invalid url (need http:// or https://): {0}")]
    InvalidUrl(String),
    #[error("fetch failed: {0}")]
    Network(String),
    #[error("fetch timed out after {FETCH_TIMEOUT_SECS}s")]
    Timeout,
    #[error("http {status}: {body}")]
    Http { status: u16, body: String },
    #[error("content extraction failed: {0}")]
    Extraction(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageFormat {
    /// Title + URL only.
    Compact,
    /// Title + URL + first `--max-chars` of the body.
    Snippet,
    /// Title + URL + complete body.
    Full,
}

impl PageFormat {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "compact" => Some(Self::Compact),
            "snippet" => Some(Self::Snippet),
            "full" => Some(Self::Full),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Snippet => "snippet",
            Self::Full => "full",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Page {
    /// Final URL after redirects.
    pub url: String,
    pub title: Option<String>,
    /// Extracted main content as Markdown.
    pub markdown: String,
    pub elapsed_ms: u64,
}

/// Fetch one URL and extract its readable main content.
///
/// Needs no API configuration; follows redirects; rejects non-http(s) URLs.
pub async fn fetch_page(url: &str) -> Result<Page, PageError> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(PageError::InvalidUrl(url.to_string()));
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(FETCH_TIMEOUT_SECS))
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| PageError::Network(e.to_string()))?;

    let started = Instant::now();
    let resp = client.get(url).send().await.map_err(|e| {
        if e.is_timeout() {
            PageError::Timeout
        } else {
            PageError::Network(e.to_string())
        }
    })?;
    let status = resp.status();
    let final_url = resp.url().to_string();
    let html = resp
        .text()
        .await
        .map_err(|e| PageError::Network(e.to_string()))?;
    if !status.is_success() {
        return Err(PageError::Http {
            status: status.as_u16(),
            body: html.chars().take(300).collect(),
        });
    }

    let article = extract_article(&html, &final_url)?;
    Ok(Page {
        url: final_url,
        title: article.title,
        markdown: article.markdown,
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

pub(crate) struct Article {
    title: Option<String>,
    markdown: String,
}

/// Pure: HTML → readable article (title + Markdown body).
pub(crate) fn extract_article(html: &str, url: &str) -> Result<Article, PageError> {
    let mut readability = Readability::new(html, Some(url), None)
        .map_err(|e| PageError::Extraction(e.to_string()))?;
    let article = readability
        .parse()
        .map_err(|e| PageError::Extraction(e.to_string()))?;
    let markdown = htmd::HtmlToMarkdown::builder()
        .build()
        .convert(&article.content)
        .map_err(|e| PageError::Extraction(e.to_string()))?;
    let title = article.title.trim();
    let title = if title.is_empty() {
        None
    } else {
        Some(title.to_string())
    };
    Ok(Article { title, markdown })
}

/// Pure: render a fetched page under the payload budget.
///
/// Layout mirrors the search discipline: metadata header (elapsed, format,
/// exact UTF-8 KB), `#1 title + url`, body per format, and a next-step hint
/// for compact/snippet.
pub fn render_page(page: &Page, format: PageFormat, max_chars: usize) -> String {
    let title = page.title.as_deref().unwrap_or("(untitled)");
    let mut body = format!("#1  {title}\n    {}", page.url);
    match format {
        PageFormat::Compact => {
            body.push_str(
                "\n\n(use --format snippet for a content preview, --format full for complete content)",
            );
        }
        PageFormat::Snippet => {
            let clipped = clip(page.markdown.trim(), max_chars);
            if !clipped.is_empty() {
                body.push_str("\n    ");
                body.push_str(&clipped.replace('\n', "\n    "));
            }
            body.push_str(&format!(
                "\n\n(use --format full to read the complete page: {})",
                page.url
            ));
        }
        PageFormat::Full => {
            let full = page.markdown.trim();
            if !full.is_empty() {
                body.push_str("\n\n");
                body.push_str(full);
            }
        }
    }

    let size_kb = body.len() as f64 / 1024.0;
    let secs = page.elapsed_ms as f64 / 1000.0;
    format!(
        "## extract \"{}\" — 1 result, {:.1}s, {}, {:.2} KB\n\n{}",
        page.url,
        secs,
        format.as_str(),
        size_kb,
        body
    )
}

/// UTF-8-safe character clip with ellipsis.
fn clip(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let clipped: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{}…", clipped.trim_end())
    } else {
        clipped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"<html>
      <head><title>My Post - Example Blog</title></head>
      <body>
        <nav><a href="/">Home</a><a href="/about">About</a></nav>
        <article>
          <h1>My Post</h1>
          <p>The point release fixes an LLVM miscompilation.</p>
          <pre><code>rustup update stable</code></pre>
        </article>
        <footer>Copyright 2026. Subscribe to our newsletter.</footer>
      </body>
    </html>"#;

    fn page(markdown: &str) -> Page {
        Page {
            url: "https://example.com/post".into(),
            title: Some("My Post".into()),
            markdown: markdown.into(),
            elapsed_ms: 1234,
        }
    }

    #[test]
    fn extract_drops_chrome_and_keeps_content() {
        let article = extract_article(FIXTURE, "https://example.com/post").unwrap();
        assert!(article.markdown.contains("LLVM miscompilation"));
        assert!(article.markdown.contains("rustup update stable"));
        assert!(!article.markdown.contains("Subscribe to our newsletter"));
        assert!(!article.markdown.contains("About"));
    }

    #[test]
    fn render_compact_has_no_body_and_points_to_snippet() {
        let out = render_page(&page("Body text here."), PageFormat::Compact, 500);
        assert!(out.contains("#1  My Post"));
        assert!(out.contains("https://example.com/post"));
        assert!(!out.contains("Body text here."));
        assert!(out.contains("--format snippet"));
        assert!(out.contains("compact"));
    }

    #[test]
    fn render_snippet_respects_budget_and_points_to_full() {
        let long = "x".repeat(900);
        let out = render_page(&page(&long), PageFormat::Snippet, 500);
        assert!(out.contains('…'));
        assert!(out.contains("--format full"));
        let body_start = out.find("#1").unwrap();
        let body = &out[body_start..];
        // title + url + <=500 clipped chars + hint; must stay well under the full body
        assert!(body.len() < 900);
    }

    #[test]
    fn render_full_keeps_everything_without_hint() {
        let long = "word ".repeat(300);
        let out = render_page(&page(&long), PageFormat::Full, 500);
        assert!(out.contains(&long.trim()[..50]));
        assert!(!out.contains("--format"));
        assert!(out.contains("full,"));
    }

    #[test]
    fn render_header_reports_format_and_size() {
        let out = render_page(&page("Body."), PageFormat::Full, 500);
        assert!(out.starts_with("## extract \"https://example.com/post\" — 1 result, 1.2s, full,"));
        assert!(out.contains("KB"));
    }

    #[test]
    fn render_untitled_falls_back() {
        let mut p = page("Body.");
        p.title = None;
        let out = render_page(&p, PageFormat::Compact, 500);
        assert!(out.contains("#1  (untitled)"));
    }

    #[test]
    fn clip_is_char_safe_and_marks_truncation() {
        let text = "中文字符串".repeat(200);
        let clipped = clip(&text, 100);
        assert!(clipped.ends_with('…'));
        assert!(clipped.chars().count() <= 101);
        assert_eq!(clip("short", 100), "short");
    }

    #[test]
    fn format_parse_roundtrip() {
        assert_eq!(PageFormat::parse("compact"), Some(PageFormat::Compact));
        assert_eq!(PageFormat::parse("snippet"), Some(PageFormat::Snippet));
        assert_eq!(PageFormat::parse("full"), Some(PageFormat::Full));
        assert_eq!(PageFormat::parse("verbose"), None);
        assert_eq!(PageFormat::Full.as_str(), "full");
    }
}
