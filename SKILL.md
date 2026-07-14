---
name: xsearch
description: "Search the public web through a configured OpenAI-compatible Grok model endpoint. Use when the user wants online facts, latest public info, comparisons, research, multi-angle surveys, or web/Grok search. Once loaded or named, run retrieval by default; local evidence may refine and complement search."
---

# xsearch

Natural-language in → multi-angle retrieval → plain-language answer.

Execution is a **skill-local one-shot binary** (no MCP, no search daemon).

## Binary

```bash
"${XSEARCH_BIN:-$HOME/.agents/skills/xsearch/bin/xsearch}" --json "<query>" [Q]
```

- Pass `Q` when plan size matters (default **5**).
- **Stdout**: a small `xsearch.run.v1` receipt with `manifest_path` and `report_path`.
- Full results: private local artifacts; use `--full` only for explicit debugging.
- **Stderr**: errors/diagnostics. Exit `0` only when the receipt and artifacts are complete.
- Build source: `engine/`; run `scripts/install.sh` after cloning.

### Config (defaults < file < env)

First file found: `$XSEARCH_CONFIG`, else `~/.config/xsearch/config.toml` or `config.json`.
Example: `config.example.toml`.

| Key / Env | Required | Meaning |
| --- | --- | --- |
| `api_url` / `XSEARCH_API_URL` | **yes** (one of) | OpenAI-compatible base |
| `api_key` / `XSEARCH_API_KEY` | if upstream needs it | Prefer **env** for secrets |
| `model` / `XSEARCH_MODEL` | no | default `grok-4.3-fast` |
| `analysis_model` / `XSEARCH_ANALYSIS_MODEL` | no | defaults to search model |
| `timeout_secs` / `XSEARCH_TIMEOUT` | no | seconds (default 600) |
| `XSEARCH_ARTIFACT_DIR` | no | root for receipt artifacts; defaults to the user cache directory |
| `log_dir` / `XSEARCH_LOG_DIR` | no | compatibility override for the artifact root |

Env overrides file for the same field. On binary failure: report stderr; do not invent results.

### Artifact output

The receipt points to:

- `manifest.json`: item index with sub-question, status, body size, URL count, and `item_path`.
- `items/NNN.json`: one complete sub-search result per file.
- `report.json`: complete `xsearch.retrieval.v1` report.

The full report preserves:

- `structured.items[]`: `index`, `sub_question`, `success`, `body`, `urls[]`, `info_status` (`ok|empty|refused|thin`), …
- `structured.deduped_urls[]`: url, sources, first_rank, occurrence_count
- `success` = upstream call ok; **`info_status` = body yield** (not truth)
- Q hard guarantee: `requested_max_query_plan == actual_sub_queries == len(items)`

---

## Process

This section is the algorithm. Follow it every run.

### 1. Form

1. **Rewrite** into a self-contained query: entity + time + dimension. State the real goal (watch X–Y).
2. **Confirm** scope/budget/preference with the host ask/choice UI when needed — not a product-specific tool name. User “just search / don’t ask” → shorten confirmation.
3. Treat confirmation as **preference**, not verified entity facts. Unverified attributes from the user stay out of every angle query until retrieval supports them; rewrite neutrally and disambiguate entities as a normal step.
4. If local docs/code/config already name entities or terms, read them as **material for rewrite** (and later merge). Default is **parallel** with search; you may sharpen first then search when that clearly helps.
5. **Done when:** rewritten goal is explicit, and either confirmation is settled or the user waived it.

### 2. Angles

| Rule | Do |
| --- | --- |
| Default | **≥3 orthogonal angles**, then fan-in |
| Cap | **8** |
| Fewer than 3 | only if user forbids expansion or host cannot multi-run |
| Never | one large-Q call described as “multiple angles” |

When entity/time/disposition boundaries are still unstable, start with a **small-Q** pass, then widen N/Q.

Budget hints (not a program gate): floor ≥3×Q5; light 3×5; mid 3–4×5–10; wide 4–6×10–20.

**Done when:** angle list is orthogonal, each has one concrete query string, and N/Q are chosen.

### 3. Execute (one angle → one leaf)

1. One angle → exactly one `bin/xsearch --json` (at most one follow-up). No nested fan-out.
2. Read the receipt, then `manifest_path`; do not read `report_path` wholesale.
3. Select item files by sub-question, `info_status`, body size, and URL count. Read only the `item_path` files needed for this angle.
4. Let the model use any retrieval capabilities available upstream; native tools are optional implementation details.
5. Return a **short note** to the parent — not artifact contents.

| Field | Meaning |
| --- | --- |
| `angle` | short name |
| `query` | what was searched |
| `Q` | plan size |
| `findings` | 3–7 bullets; empty/refused are facts |
| `conflicts` / `gaps` | when present |
| `links` | `{title,url}` or `missing_link` |
| `info_yield` | `ok` \| `empty` \| `refused` \| `thin` |
| `confidence` | trust in **the findings you wrote**, not volume of material |

**Done when:** every planned angle has a note (including empty/failed).

### 4. Fan-in

1. Collect every note.
2. Merge only claims that cohere; mark single-source claims.
3. On the same claim (who / when / what outcome / source class), **surface conflicts in their own section** — never silently prefer one `ok` body.
4. Grade sources in the narrative: primary/official vs secondary vs single-source rumor; do not promote weaker grades.
5. Dedupe URLs via notes + leaf `deduped_urls`; never invent URLs.
6. Answer in plain language; few links. Drop paraphrase-only angles at merge.

The binary enforces **count Q**, not semantic diversity.

**Done when:** final answer states agreements, open conflicts/gaps, and graded sources without dumping leaf transcripts.

---

## Guardrails

- Never invent URLs.
- Default: receipt → manifest → selected item files. Never load `report.json` wholesale into the parent context.
- No MCP client and no local search daemon required.
- Logging only via `XSEARCH_LOG_DIR`; this skill does not own dogfood scoring.
