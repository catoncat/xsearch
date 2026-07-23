# Xsearch runtime reference

Load this reference only for first-use setup, configuration, artifact interpretation, or failures.

## Configuration

Resolution order is defaults, config file, then environment variables.

Config file: `$XSEARCH_CONFIG`; otherwise `$XDG_CONFIG_HOME/xsearch/config.toml`, `~/.config/xsearch/config.toml`, or `%APPDATA%\xsearch\config.toml` on Windows.

| Key / environment variable | Required | Meaning |
| --- | --- | --- |
| `api_url` / `XSEARCH_API_URL` | one form required | OpenAI-compatible base URL |
| `api_key` / `XSEARCH_API_KEY` | when upstream requires it | Prefer environment variables for secrets |
| `model` / `XSEARCH_MODEL` | no | Search model; default `grok-4.3-fast` |
| `analysis_model` / `XSEARCH_ANALYSIS_MODEL` | no | Query-splitting model; defaults to search model |
| `timeout_secs` / `XSEARCH_TIMEOUT` | no | Timeout in seconds; default 600 |
| `max_q` / `XSEARCH_MAX_Q` | no | Per-process plan limit; default and hard ceiling 20 |
| `XSEARCH_ARTIFACT_DIR` | no | Artifact root; defaults to the user cache directory |
| `log_dir` / `XSEARCH_LOG_DIR` | no | Compatibility override for the artifact root |

On binary failure, report stderr and leave the route failed. Never invent a result.

## Receipt and artifacts

Successful stdout is a small `xsearch.run.v1` receipt containing `run_id`, `manifest_path`, `report_path`, `item_count`, and yield counts. Stderr carries diagnostics. Exit code `0` means the receipt and artifacts completed.

The receipt points to:

- `manifest.json`: item index with sub-question, yield status, body size, URL count, and `item_path`
- `items/NNN.json`: one complete sub-search result per file
- `report.json`: complete `xsearch.retrieval.v1` report

Normal route reading order is receipt → manifest → selected item files. `report.json` and `--full` are for explicit debugging.

The full report preserves:

- `structured.items[]`: `index`, `sub_question`, `success`, `body`, `urls[]`, and `info_status`
- `structured.deduped_urls[]`: URL, source sub-query IDs, first rank, and occurrence count
- `success`: whether the upstream call completed
- `info_status`: `ok`, `empty`, `refused`, `thin`, or `failed`; this describes yield, not truth

Failed upstream calls use `info_status: failed`, never contribute to `ok`, and never contribute URLs. The engine runs at most four sub-searches concurrently per process.

The engine guarantees `requested_max_query_plan == actual_sub_queries == len(items)`. It guarantees Q count, not semantic diversity between parent routes.

## Deep-read (extract)

`xsearch extract <url>` fetches one page and renders its readable main content as Markdown. It needs no API configuration and no route ledger.

- `--format compact|snippet|full` — render budget; default `full`. `compact` is title+URL only, `snippet` adds the first `--max-chars` of body.
- `--max-chars N` — snippet budget, default 500.
- The output header reports elapsed seconds, format, and the exact UTF-8 KB of the rendered body.
- Fetch timeout is 60 seconds. Exit `0` means fetched and extracted; fetch/extraction failures print to stderr with exit `1`; usage errors exit `2`.

## Source build

Repository builds use `engine/`. After cloning, run `scripts/install.sh`. End users installed through the Skill bootstrap do not need a Rust toolchain.
