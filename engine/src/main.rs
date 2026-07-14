use std::path::PathBuf;
use std::process::ExitCode;
use xsearch::{load_resolved, run_search, HttpChatUpstream, SearchRequest};

fn usage() -> ! {
    eprintln!("usage: xsearch \"<query>\" [Q]");
    eprintln!();
    eprintln!("config (defaults < file < env):");
    eprintln!("  file: $XSEARCH_CONFIG or ~/.config/xsearch/config.toml|.json");
    eprintln!("  env:  XSEARCH_API_URL, XSEARCH_API_KEY, XSEARCH_MODEL,");
    eprintln!("        XSEARCH_ANALYSIS_MODEL, XSEARCH_TIMEOUT, XSEARCH_LOG_DIR");
    eprintln!("  example: vendor/skills/xsearch/config.example.toml");
    std::process::exit(2);
}

fn maybe_log(report_json: &str, log_dir: Option<&str>) {
    let Some(dir) = log_dir.filter(|s| !s.is_empty()) else {
        return;
    };
    let path = PathBuf::from(dir);
    if let Err(e) = std::fs::create_dir_all(&path) {
        eprintln!("xsearch: log dir: {e}");
        return;
    }
    let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
    let file = path.join(format!("xsearch-{ts}.json"));
    if let Err(e) = std::fs::write(&file, report_json) {
        eprintln!("xsearch: log write: {e}");
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        usage();
    }

    let query = args.remove(0);
    let q: u32 = if args.is_empty() {
        5
    } else {
        match args[0].parse() {
            Ok(n) => n,
            Err(_) => usage(),
        }
    };

    let cfg = match load_resolved() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("xsearch: config: {e}");
            return ExitCode::from(2);
        }
    };

    let api_url = match cfg.require_api_url() {
        Ok(u) => u,
        Err(e) => {
            eprintln!("xsearch: {e}");
            return ExitCode::from(2);
        }
    };

    let upstream = match HttpChatUpstream::from_resolved(
        api_url,
        cfg.api_key.clone(),
        cfg.options.timeout_secs,
    ) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("xsearch: {e}");
            return ExitCode::from(2);
        }
    };

    match run_search(
        SearchRequest {
            query: query.clone(),
            q,
        },
        &upstream,
        cfg.options.clone(),
    )
    .await
    {
        Ok(report) => match report.to_json_pretty() {
            Ok(json) => {
                maybe_log(&json, cfg.log_dir.as_deref());
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("xsearch: serialize: {e}");
                ExitCode::from(1)
            }
        },
        Err(e) => {
            eprintln!("xsearch: {e}");
            ExitCode::from(1)
        }
    }
}
