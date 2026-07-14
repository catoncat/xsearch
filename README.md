# xsearch

Turn a third-party OpenAI-compatible Grok model endpoint into a one-shot structured search CLI and Agent Skill.

No official xAI API account is required. `xsearch` uses the retrieval behavior available through your configured model proxy, splits research into an exact query plan, runs it concurrently, and returns stable JSON with sources and quality signals.

## Install

### macOS and Linux

```bash
curl -fsSL https://raw.githubusercontent.com/catoncat/xsearch/main/install.sh | bash
```

### Windows PowerShell

```powershell
irm https://raw.githubusercontent.com/catoncat/xsearch/main/install.ps1 | iex
```

The installer detects the platform, downloads the latest prebuilt binary, verifies its SHA-256 checksum, and installs both the CLI and `SKILL.md` under `~/.agents/skills/xsearch`.

Rust and Cargo are **not required** for installation.

Supported release targets:

- macOS: Apple Silicon and Intel
- Linux: x86_64 and ARM64
- Windows: x86_64

Set `XSEARCH_INSTALL_DIR` to override the installation directory. Set `XSEARCH_VERSION`, for example `v0.1.1`, to install a specific release.

## Configure

The installer creates a config file on first install:

```text
macOS/Linux: ~/.config/xsearch/config.toml
Windows:     %APPDATA%\xsearch\config.toml
```

Edit it for your third-party Grok model proxy:

```toml
api_url = "https://your-grok-proxy.example/v1"
model = "grok-4.3-fast"
```

Prefer supplying credentials through the environment:

```bash
export XSEARCH_API_KEY='your-provider-key'
```

A file-based `api_key` is also supported. Keep the file private and never commit it.

Configuration priority is: built-in defaults, config file, then `XSEARCH_*` environment variables.

## Verify

```bash
~/.agents/skills/xsearch/bin/xsearch --version
~/.agents/skills/xsearch/bin/xsearch "What happened today? Include sources." 3
```

A terminal shows a short aligned receipt. Scripts and Agents receive the same receipt as JSON; use `--json` to force it explicitly.

On Windows:

```powershell
~\.agents\skills\xsearch\bin\xsearch.exe --version
```

## What it does

- Accepts a natural-language query and requested query-plan size `Q`.
- Splits the request into exactly `Q` distinct sub-questions.
- Runs sub-searches concurrently through the configured model endpoint.
- Preserves upstream search sources when the endpoint returns them.
- Stores complete results as private local artifacts instead of flooding Agent context.
- Returns a small receipt pointing to a manifest and per-item result files.
- Filters uncited candidates and deduplicates URLs.
- Separates transport success from result quality with `info_status`.
- Supports multi-angle Agent workflows through [`SKILL.md`](./SKILL.md).
- Runs as a one-shot process: no MCP server, daemon, or listening port.

The hard output invariant is:

```text
requested_max_query_plan == actual_sub_queries == items.length
```

## Context-efficient results

Default stdout is a small `xsearch.run.v1` receipt:

```json
{
  "schema": "xsearch.run.v1",
  "run_id": "20260714T120000.000Z-1234",
  "manifest_path": "/home/user/.cache/xsearch/runs/.../manifest.json",
  "report_path": "/home/user/.cache/xsearch/runs/.../report.json",
  "item_count": 3,
  "source_count": 12,
  "next_action": "Read manifest_path, then read only the item_path files needed for the answer."
}
```

Artifacts are stored with private permissions:

```text
macOS/Linux: ~/.cache/xsearch/runs/<run-id>/
Windows:     %LOCALAPPDATA%\xsearch\runs\<run-id>\

  manifest.json       item index and selection metadata
  items/001.json      one complete sub-search result
  items/002.json
  report.json         complete xsearch.retrieval.v1 result
```

Agents should read the manifest first and load only relevant item files. No evidence is truncated; the filesystem acts as external working memory. Use `--full` only when intentionally sending the complete report to stdout. Set `XSEARCH_ARTIFACT_DIR` to move the artifact store.

`success` reports whether an upstream call completed. `info_status` reports whether its body was useful; these are intentionally separate.

## Build from source

Rust is only needed for development or unsupported platforms:

```bash
git clone https://github.com/catoncat/xsearch.git
cd xsearch
./scripts/install.sh
```

Development checks:

```bash
cd engine
cargo fmt --check
cargo test --locked
cargo check --locked
```

No live endpoint is needed for unit tests. The engine uses an injectable in-memory upstream in tests.

## Security

API URLs, keys, model names, and logs remain local runtime configuration. Release installers verify downloaded binaries against the published `checksums.txt`. See [`SECURITY.md`](./SECURITY.md) for reporting and credential-handling guidance.

## License

MIT
