use crate::types::{InfoStatus, InfoStatusCounts, Metadata, Report};
use serde::Serialize;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct RunReceipt {
    pub schema: &'static str,
    pub run_id: String,
    pub manifest_path: String,
    pub report_path: String,
    pub item_count: u32,
    pub source_count: usize,
    pub duration_ms: u64,
    pub info_status_counts: InfoStatusCounts,
    pub full_report_bytes: u64,
    pub next_action: &'static str,
}

impl RunReceipt {
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn to_human(&self) -> String {
        let counts = &self.info_status_counts;
        format!(
            "xsearch complete\n  searches : {} ({} ok, {} thin, {} empty, {} refused)\n  sources  : {}\n  elapsed  : {:.2}s\n  manifest : {}\n  report   : {}\n  next     : read the manifest, then only the item files needed",
            self.item_count,
            counts.ok,
            counts.thin,
            counts.empty,
            counts.refused,
            self.source_count,
            self.duration_ms as f64 / 1_000.0,
            self.manifest_path,
            self.report_path,
        )
    }
}

#[derive(Debug, Serialize)]
struct RunManifest {
    schema: &'static str,
    run_id: String,
    report_path: String,
    items: Vec<ItemEntry>,
    metadata: Metadata,
}

#[derive(Debug, Serialize)]
struct ItemEntry {
    index: u32,
    sub_question: String,
    success: bool,
    info_status: InfoStatus,
    body_chars: usize,
    url_count: usize,
    item_path: String,
}

pub fn default_artifact_root() -> PathBuf {
    if let Some(path) = std::env::var_os("XSEARCH_ARTIFACT_DIR") {
        return PathBuf::from(path);
    }
    if let Some(path) = std::env::var_os("XSEARCH_LOG_DIR") {
        return PathBuf::from(path);
    }
    if let Some(path) = std::env::var_os("XDG_CACHE_HOME") {
        return PathBuf::from(path).join("xsearch/runs");
    }
    #[cfg(windows)]
    if let Some(path) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(path).join("xsearch/runs");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".cache/xsearch/runs");
    }
    if let Some(home) = std::env::var_os("USERPROFILE") {
        return PathBuf::from(home).join(".cache/xsearch/runs");
    }
    std::env::temp_dir().join("xsearch/runs")
}

pub fn persist_report(report: &Report, root: &Path) -> io::Result<RunReceipt> {
    let run_id = format!(
        "{}-{}",
        chrono::Utc::now().format("%Y%m%dT%H%M%S%.3fZ"),
        std::process::id()
    );
    let run_dir = root.join(&run_id);
    let items_dir = run_dir.join("items");
    fs::create_dir_all(&items_dir)?;
    set_private_dir(&run_dir)?;
    set_private_dir(&items_dir)?;

    let run_dir = fs::canonicalize(&run_dir)?;
    let report_path = run_dir.join("report.json");
    let manifest_path = run_dir.join("manifest.json");
    let full_json = report
        .to_json_pretty()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    write_private(&report_path, full_json.as_bytes())?;

    let mut entries = Vec::with_capacity(report.structured.items.len());
    for item in &report.structured.items {
        let item_path = items_dir.join(format!("{:03}.json", item.index));
        let item_json = serde_json::to_vec_pretty(item)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        write_private(&item_path, &item_json)?;
        entries.push(ItemEntry {
            index: item.index,
            sub_question: item.sub_question.clone(),
            success: item.success,
            info_status: item.info_status,
            body_chars: item.body.chars().count(),
            url_count: item.urls.len(),
            item_path: display_path(&item_path),
        });
    }

    let manifest = RunManifest {
        schema: "xsearch.manifest.v1",
        run_id: run_id.clone(),
        report_path: display_path(&report_path),
        items: entries,
        metadata: report.metadata.clone(),
    };
    let manifest_json = serde_json::to_vec_pretty(&manifest)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    write_private(&manifest_path, &manifest_json)?;

    Ok(RunReceipt {
        schema: "xsearch.run.v1",
        run_id,
        manifest_path: display_path(&manifest_path),
        report_path: display_path(&report_path),
        item_count: report.metadata.actual_sub_queries,
        source_count: report.structured.deduped_urls.len(),
        duration_ms: report.metadata.duration_ms,
        info_status_counts: report.structured.info_status_counts.clone(),
        full_report_bytes: full_json.len() as u64,
        next_action:
            "Read manifest_path, then read only the item_path files needed for the answer.",
    })
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    fs::write(path, bytes)?;
    set_private_file(path)
}

#[cfg(unix)]
fn set_private_dir(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
fn set_private_dir(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn set_private_file(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn set_private_file(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{InfoStatusCounts, Item, Metadata, StructuredV1};

    #[test]
    fn persists_full_report_and_item_index_without_truncation() {
        let body = "evidence ".repeat(4_000);
        let report = Report {
            structured: StructuredV1 {
                schema: "xsearch.retrieval.v1",
                items: vec![Item {
                    index: 1,
                    sub_question: "Which evidence answers the question?".into(),
                    success: true,
                    body: body.clone(),
                    title: None,
                    snippets: Vec::new(),
                    urls: vec!["https://example.com/source".into()],
                    info_status: InfoStatus::Ok,
                }],
                deduped_urls: Vec::new(),
                info_status_counts: InfoStatusCounts {
                    ok: 1,
                    empty: 0,
                    refused: 0,
                    thin: 0,
                },
            },
            metadata: Metadata {
                duration_ms: 1,
                model: "test".into(),
                requested_max_query_plan: 1,
                actual_sub_queries: 1,
                success_count: 1,
                failure_count: 0,
                empty_count: 0,
                refused_count: 0,
                thin_count: 0,
                ok_count: 1,
                timestamp: "2026-01-01T00:00:00Z".into(),
                artifacts_schema: "v1",
            },
        };
        let root = std::env::temp_dir().join(format!(
            "xsearch-artifact-test-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));

        let receipt = persist_report(&report, &root).unwrap();
        let receipt_json = receipt.to_json_pretty().unwrap();
        let stored_report = fs::read_to_string(&receipt.report_path).unwrap();
        let manifest = fs::read_to_string(&receipt.manifest_path).unwrap();
        let item_path = root.join(&receipt.run_id).join("items/001.json");
        let stored_item = fs::read_to_string(item_path).unwrap();

        assert!(receipt_json.len() < 1_500);
        assert!(receipt.to_human().contains("xsearch complete"));
        assert!(!receipt_json.contains(&body));
        assert!(stored_report.contains(&body));
        assert!(stored_item.contains(&body));
        assert!(!manifest.contains(&body));

        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn default_root_uses_local_appdata_on_windows() {
        let local = std::env::temp_dir().join("xsearch-localappdata-test");
        std::env::remove_var("XSEARCH_ARTIFACT_DIR");
        std::env::remove_var("XSEARCH_LOG_DIR");
        std::env::remove_var("XDG_CACHE_HOME");
        std::env::set_var("LOCALAPPDATA", &local);

        assert_eq!(default_artifact_root(), local.join("xsearch/runs"));

        std::env::remove_var("LOCALAPPDATA");
    }
}
