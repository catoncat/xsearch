//! Config load: built-in defaults < config file < environment.
//!
//! File (first found):
//! - `$XSEARCH_CONFIG`
//! - `$XDG_CONFIG_HOME/xsearch/config.toml` (or `~/.config/xsearch/config.toml`)
//! - same paths with `.json`
//!
//! Env always wins over file for the same key.

use crate::types::EngineOptions;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct FileConfig {
    #[serde(alias = "api_url")]
    pub api_url: Option<String>,
    #[serde(alias = "api_key")]
    pub api_key: Option<String>,
    #[serde(alias = "model", alias = "search_model")]
    pub model: Option<String>,
    #[serde(alias = "analysis_model")]
    pub analysis_model: Option<String>,
    #[serde(alias = "timeout_secs", alias = "timeout")]
    pub timeout_secs: Option<u64>,
    #[serde(alias = "max_q")]
    pub max_q: Option<u32>,
    #[serde(alias = "log_dir")]
    pub log_dir: Option<String>,
}

/// Fully resolved settings for the CLI (and tests that want file/env behavior).
#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub api_url: Option<String>,
    pub api_key: Option<String>,
    pub options: EngineOptions,
    pub log_dir: Option<String>,
    /// Which file was loaded, if any (for diagnostics).
    pub loaded_file: Option<PathBuf>,
}

impl Default for ResolvedConfig {
    fn default() -> Self {
        Self {
            api_url: None,
            api_key: None,
            options: EngineOptions::default(),
            log_dir: None,
            loaded_file: None,
        }
    }
}

fn config_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(p) = std::env::var("XSEARCH_CONFIG") {
        if !p.is_empty() {
            paths.push(PathBuf::from(p));
        }
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
    if let Some(base) = base {
        let dir = base.join("xsearch");
        paths.push(dir.join("config.toml"));
        paths.push(dir.join("config.json"));
    }
    paths
}

fn parse_file(path: &PathBuf, raw: &str) -> Result<FileConfig, String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ext == "json" {
        serde_json::from_str(raw).map_err(|e| format!("parse {}: {e}", path.display()))
    } else {
        // default: toml (also if no/unknown extension when forced via XSEARCH_CONFIG)
        toml::from_str(raw).map_err(|e| format!("parse {}: {e}", path.display()))
    }
}

fn apply_file(cfg: &mut ResolvedConfig, file: FileConfig) {
    if let Some(v) = file.api_url.filter(|s| !s.is_empty()) {
        cfg.api_url = Some(v);
    }
    if let Some(v) = file.api_key.filter(|s| !s.is_empty()) {
        cfg.api_key = Some(v);
    }
    if let Some(v) = file.model.filter(|s| !s.is_empty()) {
        cfg.options.search_model = v.clone();
        // analysis follows search unless file sets analysis_model
        if file.analysis_model.is_none() {
            cfg.options.analysis_model = v;
        }
    }
    if let Some(v) = file.analysis_model.filter(|s| !s.is_empty()) {
        cfg.options.analysis_model = v;
    }
    if let Some(v) = file.timeout_secs {
        cfg.options.timeout_secs = v;
    }
    if let Some(v) = file.max_q {
        cfg.options.max_q = v;
    }
    if let Some(v) = file.log_dir.filter(|s| !s.is_empty()) {
        cfg.log_dir = Some(v);
    }
}

fn apply_env(cfg: &mut ResolvedConfig) {
    if let Ok(v) = std::env::var("XSEARCH_API_URL") {
        if !v.is_empty() {
            cfg.api_url = Some(v);
        }
    }
    if let Ok(v) = std::env::var("XSEARCH_API_KEY") {
        // allow empty to mean "clear file key"? Prefer: non-empty only overrides
        if !v.is_empty() {
            cfg.api_key = Some(v);
        }
    }
    if let Ok(v) = std::env::var("XSEARCH_MODEL") {
        if !v.is_empty() {
            cfg.options.search_model = v.clone();
            // if analysis not explicitly set via env, keep previous analysis or sync
            if std::env::var("XSEARCH_ANALYSIS_MODEL").is_err() {
                cfg.options.analysis_model = v;
            }
        }
    }
    if let Ok(v) = std::env::var("XSEARCH_ANALYSIS_MODEL") {
        if !v.is_empty() {
            cfg.options.analysis_model = v;
        }
    }
    if let Ok(v) = std::env::var("XSEARCH_TIMEOUT") {
        if let Ok(n) = v.parse() {
            cfg.options.timeout_secs = n;
        }
    }
    if let Ok(v) = std::env::var("XSEARCH_MAX_Q") {
        if let Ok(n) = v.parse() {
            cfg.options.max_q = n;
        }
    }
    if let Ok(v) = std::env::var("XSEARCH_LOG_DIR") {
        if !v.is_empty() {
            cfg.log_dir = Some(v);
        }
    }
}

/// Load defaults, then first readable config file, then env overrides.
pub fn load_resolved() -> Result<ResolvedConfig, String> {
    let mut cfg = ResolvedConfig::default();

    for path in config_candidates() {
        if !path.is_file() {
            continue;
        }
        let raw = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let file = parse_file(&path, &raw)?;
        apply_file(&mut cfg, file);
        cfg.loaded_file = Some(path);
        break;
    }

    apply_env(&mut cfg);
    Ok(cfg)
}

impl ResolvedConfig {
    /// Require api_url; human-readable hint listing env + file paths.
    pub fn require_api_url(&self) -> Result<String, String> {
        if let Some(url) = self.api_url.as_ref().filter(|s| !s.is_empty()) {
            return Ok(url.clone());
        }
        let mut msg = String::from(
            "missing API URL\n  set XSEARCH_API_URL, or put api_url in a config file:\n",
        );
        for p in config_candidates() {
            msg.push_str(&format!("    - {}\n", p.display()));
        }
        msg.push_str("  see vendor/skills/xsearch/config.example.toml");
        Err(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // serialize env mutations in tests
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn env_overrides_defaults() {
        let _g = ENV_LOCK.lock().unwrap();
        // clear relevant
        for k in [
            "XSEARCH_API_URL",
            "XSEARCH_API_KEY",
            "XSEARCH_MODEL",
            "XSEARCH_ANALYSIS_MODEL",
            "XSEARCH_TIMEOUT",
            "XSEARCH_LOG_DIR",
            "XSEARCH_CONFIG",
        ] {
            std::env::remove_var(k);
        }
        std::env::set_var("XSEARCH_API_URL", "http://example.test/v1");
        std::env::set_var("XSEARCH_MODEL", "my-model");
        std::env::set_var("XSEARCH_TIMEOUT", "42");
        let cfg = load_resolved().unwrap();
        assert_eq!(cfg.api_url.as_deref(), Some("http://example.test/v1"));
        assert_eq!(cfg.options.search_model, "my-model");
        assert_eq!(cfg.options.analysis_model, "my-model");
        assert_eq!(cfg.options.timeout_secs, 42);
        std::env::remove_var("XSEARCH_API_URL");
        std::env::remove_var("XSEARCH_MODEL");
        std::env::remove_var("XSEARCH_TIMEOUT");
    }

    #[test]
    fn file_then_env() {
        let _g = ENV_LOCK.lock().unwrap();
        for k in [
            "XSEARCH_API_URL",
            "XSEARCH_API_KEY",
            "XSEARCH_MODEL",
            "XSEARCH_ANALYSIS_MODEL",
            "XSEARCH_TIMEOUT",
            "XSEARCH_LOG_DIR",
            "XSEARCH_CONFIG",
        ] {
            std::env::remove_var(k);
        }
        let dir = std::env::temp_dir().join(format!("xsearch-cfg-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("config.toml");
        fs::write(
            &path,
            r#"
api_url = "http://from-file/v1"
model = "file-model"
timeout_secs = 99
"#,
        )
        .unwrap();
        std::env::set_var("XSEARCH_CONFIG", path.to_str().unwrap());
        std::env::set_var("XSEARCH_MODEL", "env-model");
        let cfg = load_resolved().unwrap();
        assert_eq!(cfg.api_url.as_deref(), Some("http://from-file/v1"));
        assert_eq!(cfg.options.search_model, "env-model");
        assert_eq!(cfg.options.timeout_secs, 99);
        assert_eq!(cfg.loaded_file.as_ref(), Some(&path));
        std::env::remove_var("XSEARCH_CONFIG");
        std::env::remove_var("XSEARCH_MODEL");
        let _ = fs::remove_dir_all(&dir);
    }
}
