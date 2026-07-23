use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;
use xsearch::{
    default_artifact_root, fetch_page, load_resolved, persist_report, render_page, run_search,
    HttpChatUpstream, PageFormat, SearchRequest, DEFAULT_MAX_CHARS,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum OutputMode {
    Auto,
    Json,
    Human,
    Full,
}

fn print_usage() {
    println!("usage: xsearch [--json|--human|--full] \"<query>\" [Q]");
    println!("       xsearch extract <url> [--format compact|snippet|full] [--max-chars N]");
    println!();
    println!("output:");
    println!("  default  human receipt in a terminal; JSON receipt when piped");
    println!("  --json   print the artifact receipt as JSON");
    println!("  --human  print the aligned human receipt");
    println!("  --full   print the complete retrieval report to stdout");
    println!();
    println!("extract:");
    println!("  deep-read one URL (readable content as Markdown); default --format full,");
    println!("  --max-chars {DEFAULT_MAX_CHARS}; needs no API configuration");
    println!();
    println!("config (defaults < file < env):");
    println!("  file: $XSEARCH_CONFIG, ~/.config/xsearch, or %APPDATA%\\xsearch");
    println!("  env:  XSEARCH_API_URL, XSEARCH_API_KEY, XSEARCH_MODEL,");
    println!("        XSEARCH_ANALYSIS_MODEL, XSEARCH_TIMEOUT,");
    println!("        XSEARCH_ARTIFACT_DIR, XSEARCH_LOG_DIR");
}

fn extract_usage_error() -> ! {
    eprintln!("usage: xsearch extract <url> [--format compact|snippet|full] [--max-chars N]");
    std::process::exit(2);
}

/// `xsearch extract <url>`: deep-read one page; standalone (no API config).
async fn run_extract(args: &[String]) -> ExitCode {
    let mut format = PageFormat::Full;
    let mut max_chars = DEFAULT_MAX_CHARS;
    let mut url: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        if arg == "--format" {
            i += 1;
            format = match args.get(i).and_then(|v| PageFormat::parse(v)) {
                Some(f) => f,
                None => extract_usage_error(),
            };
        } else if let Some(v) = arg.strip_prefix("--format=") {
            format = match PageFormat::parse(v) {
                Some(f) => f,
                None => extract_usage_error(),
            };
        } else if arg == "--max-chars" {
            i += 1;
            max_chars = match args.get(i).and_then(|v| v.parse().ok()) {
                Some(n) => n,
                None => extract_usage_error(),
            };
        } else if let Some(v) = arg.strip_prefix("--max-chars=") {
            max_chars = match v.parse() {
                Ok(n) => n,
                Err(_) => extract_usage_error(),
            };
        } else if arg.starts_with("--") {
            extract_usage_error();
        } else if url.is_none() {
            url = Some(arg.to_string());
        } else {
            extract_usage_error();
        }
        i += 1;
    }

    let url = match url {
        Some(u) => u,
        None => extract_usage_error(),
    };

    match fetch_page(&url).await {
        Ok(page) => {
            println!("{}", render_page(&page, format, max_chars));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("xsearch: {error}");
            ExitCode::from(1)
        }
    }
}

fn usage_error() -> ! {
    eprintln!("usage: xsearch [--json|--human|--full] \"<query>\" [Q]");
    eprintln!("try 'xsearch --help' for output and configuration options");
    std::process::exit(2);
}

fn parse_output_mode(args: &mut Vec<String>) -> OutputMode {
    let mut mode = OutputMode::Auto;
    let mut positional = Vec::with_capacity(args.len());

    for arg in args.drain(..) {
        let requested = match arg.as_str() {
            "--json" => Some(OutputMode::Json),
            "--human" => Some(OutputMode::Human),
            "--full" => Some(OutputMode::Full),
            _ => None,
        };
        if let Some(requested) = requested {
            if mode != OutputMode::Auto {
                usage_error();
            }
            mode = requested;
        } else {
            positional.push(arg);
        }
    }

    *args = positional;
    mode
}

fn artifact_root(configured_log_dir: Option<&str>) -> PathBuf {
    if std::env::var_os("XSEARCH_ARTIFACT_DIR").is_some() {
        return default_artifact_root();
    }
    configured_log_dir
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(default_artifact_root)
}

#[tokio::main]
async fn main() -> ExitCode {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        usage_error();
    }
    if args.len() == 1 && matches!(args[0].as_str(), "-h" | "--help") {
        print_usage();
        return ExitCode::SUCCESS;
    }
    if args.len() == 1 && matches!(args[0].as_str(), "-V" | "--version") {
        println!("xsearch {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    if args[0] == "extract" {
        return run_extract(&args[1..]).await;
    }

    let output_mode = parse_output_mode(&mut args);
    if args.is_empty() || args.len() > 2 || args[0].starts_with("--") {
        usage_error();
    }

    let query = args.remove(0);
    let q: u32 = if args.is_empty() {
        5
    } else {
        match args[0].parse() {
            Ok(number) => number,
            Err(_) => usage_error(),
        }
    };

    let config = match load_resolved() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("xsearch: config: {error}");
            return ExitCode::from(2);
        }
    };

    let api_url = match config.require_api_url() {
        Ok(url) => url,
        Err(error) => {
            eprintln!("xsearch: {error}");
            return ExitCode::from(2);
        }
    };

    let upstream = match HttpChatUpstream::from_resolved(
        api_url,
        config.api_key.clone(),
        config.options.timeout_secs,
    ) {
        Ok(upstream) => upstream,
        Err(error) => {
            eprintln!("xsearch: {error}");
            return ExitCode::from(2);
        }
    };

    let report = match run_search(
        SearchRequest { query, q },
        &upstream,
        config.options.clone(),
    )
    .await
    {
        Ok(report) => report,
        Err(error) => {
            eprintln!("xsearch: {error}");
            return ExitCode::from(1);
        }
    };

    if output_mode == OutputMode::Full {
        return match report.to_json_pretty() {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("xsearch: serialize: {error}");
                ExitCode::from(1)
            }
        };
    }

    let root = artifact_root(config.log_dir.as_deref());
    let receipt = match persist_report(&report, &root) {
        Ok(receipt) => receipt,
        Err(error) => {
            eprintln!("xsearch: artifact: {error}");
            return ExitCode::from(1);
        }
    };

    let human = output_mode == OutputMode::Human
        || (output_mode == OutputMode::Auto && std::io::stdout().is_terminal());
    if human {
        println!("{}", receipt.to_human());
        ExitCode::SUCCESS
    } else {
        match receipt.to_json_pretty() {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("xsearch: serialize receipt: {error}");
                ExitCode::from(1)
            }
        }
    }
}
