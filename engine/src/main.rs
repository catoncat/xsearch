use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;
use xsearch::{
    default_artifact_root, load_resolved, persist_report, run_search, HttpChatUpstream,
    SearchRequest,
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
    println!();
    println!("output:");
    println!("  default  human receipt in a terminal; JSON receipt when piped");
    println!("  --json   print the artifact receipt as JSON");
    println!("  --human  print the aligned human receipt");
    println!("  --full   print the complete retrieval report to stdout");
    println!();
    println!("config (defaults < file < env):");
    println!("  file: $XSEARCH_CONFIG, ~/.config/xsearch, or %APPDATA%\\xsearch");
    println!("  env:  XSEARCH_API_URL, XSEARCH_API_KEY, XSEARCH_MODEL,");
    println!("        XSEARCH_ANALYSIS_MODEL, XSEARCH_TIMEOUT,");
    println!("        XSEARCH_ARTIFACT_DIR, XSEARCH_LOG_DIR");
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
