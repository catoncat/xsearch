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

The installer creates this file on first install:

```text
~/.config/xsearch/config.toml
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

On Windows:

```powershell
~\.agents\skills\xsearch\bin\xsearch.exe --version
```

## What it does

- Accepts a natural-language query and requested query-plan size `Q`.
- Splits the request into exactly `Q` distinct sub-questions.
- Runs sub-searches concurrently through the configured model endpoint.
- Preserves upstream search sources when the endpoint returns them.
- Filters uncited candidates and deduplicates URLs.
- Separates transport success from result quality with `info_status`.
- Supports multi-angle Agent workflows through [`SKILL.md`](./SKILL.md).
- Runs as a one-shot process: no MCP server, daemon, or listening port.

The hard output invariant is:

```text
requested_max_query_plan == actual_sub_queries == items.length
```

Successful stdout is one `xsearch.retrieval.v1` JSON document:

```json
{
  "structured": {
    "schema": "xsearch.retrieval.v1",
    "items": [],
    "deduped_urls": [],
    "info_status_counts": {
      "ok": 0,
      "empty": 0,
      "refused": 0,
      "thin": 0
    }
  },
  "metadata": {
    "requested_max_query_plan": 3,
    "actual_sub_queries": 3
  }
}
```

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
