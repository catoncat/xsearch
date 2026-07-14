# xsearch

`xsearch` turns a configured OpenAI-compatible Grok model endpoint into a one-shot, structured search CLI and an Agent Skill.

It is designed for third-party Grok model proxies. It does not require an official xAI API account or key. The upstream model/runtime may provide additional native retrieval capabilities; `xsearch` uses whatever retrieval behavior is available without making a specific internal tool part of its public contract.

## What it does

- Accepts a natural-language query and a requested query-plan size `Q`.
- Splits the request into exactly `Q` distinct sub-questions.
- Runs the sub-searches concurrently.
- Preserves upstream search sources when the endpoint returns them.
- Produces stable `xsearch.retrieval.v1` JSON with URLs, quality status, and metadata.
- Supports multi-angle Agent workflows through [`SKILL.md`](./SKILL.md).
- Runs as a one-shot process: no MCP server, daemon, or listening port.

## Requirements

- Rust toolchain for local installation.
- An OpenAI-compatible endpoint serving a Grok model.
- Endpoint credentials if required by that provider.

`xsearch` does not provide an endpoint or credentials. Model IDs and retrieval behavior vary between proxy providers.

## Install

```bash
git clone https://github.com/catoncat/xsearch.git
cd xsearch
./scripts/install.sh
```

The installer builds the Rust binary and installs the skill under:

```text
~/.agents/skills/xsearch/
```

Override the destination with `XSEARCH_INSTALL_DIR` or point the skill at another binary with `XSEARCH_BIN`.

## Configure

Configuration priority is: built-in defaults, config file, then environment variables.

```bash
mkdir -p ~/.config/xsearch
cp config.example.toml ~/.config/xsearch/config.toml
chmod 600 ~/.config/xsearch/config.toml
```

Edit the endpoint and model for your proxy. Prefer keeping credentials in the environment:

```bash
export XSEARCH_API_KEY='your-provider-key'
```

A file-only setup is also supported when needed:

```toml
api_url = "https://your-grok-proxy.example/v1"
api_key = "your-provider-key"
model = "grok-4.3-fast"
```

Never commit the real config file.

## Use

```bash
~/.agents/skills/xsearch/bin/xsearch \
  "Compare the latest public evidence about topic A and topic B" 5
```

Successful stdout is one JSON document:

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
    "requested_max_query_plan": 5,
    "actual_sub_queries": 5
  }
}
```

The hard count invariant is:

```text
requested_max_query_plan == actual_sub_queries == items.length
```

`success` reports whether an upstream call completed. `info_status` reports whether its body was useful; these are intentionally separate.

## Development

```bash
cd engine
cargo fmt --check
cargo test
cargo check
cargo build --release
```

No live endpoint is needed for unit tests. The engine uses an injectable in-memory upstream in tests.

## Security

API URLs, keys, model names, and logs are local runtime configuration. See [`SECURITY.md`](./SECURITY.md) for reporting and credential-handling guidance.

## License

MIT
