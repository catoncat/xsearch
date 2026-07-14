---
name: xsearch
description: "Search the public web through a configured OpenAI-compatible Grok model endpoint. Use when the user wants online facts, latest public info, comparisons, research, multi-angle surveys, or web/Grok search. Once loaded or named, run retrieval by default; local evidence may refine and complement search."
---

# xsearch

Natural-language in → multi-angle retrieval → plain-language answer.

Execution is a **skill-local one-shot binary** (no MCP, no search daemon).

## Binary

### Bootstrap

Before the first search, resolve the binary path shown below. If it is missing, install the latest checksummed release, then verify `xsearch --version` exits `0`.

macOS/Linux bootstrap:

```bash
test -x "${XSEARCH_BIN:-$HOME/.agents/skills/xsearch/bin/xsearch}" || \
  curl -fsSL https://raw.githubusercontent.com/catoncat/xsearch/main/install.sh | bash
```

Windows PowerShell bootstrap:

```powershell
$bin = if ($env:XSEARCH_BIN) { $env:XSEARCH_BIN } else { "$HOME\.agents\skills\xsearch\bin\xsearch.exe" }
if (-not (Test-Path $bin)) {
  irm https://raw.githubusercontent.com/catoncat/xsearch/main/install.ps1 | iex
}
```

### Execute

macOS/Linux:

```bash
"${XSEARCH_BIN:-$HOME/.agents/skills/xsearch/bin/xsearch}" --json "<query>" [Q]
```

Windows PowerShell:

```powershell
$bin = if ($env:XSEARCH_BIN) { $env:XSEARCH_BIN } else { "$HOME\.agents\skills\xsearch\bin\xsearch.exe" }
& $bin --json "<query>" [Q]
```

- Pass `Q` when plan size matters (default **5**).
- **Stdout**: a small `xsearch.run.v1` receipt with `manifest_path` and `report_path`.
- Full results: private local artifacts; use `--full` only for explicit debugging.
- **Stderr**: errors/diagnostics. Exit `0` only when the receipt and artifacts are complete.
- Build source: `engine/`; run `scripts/install.sh` after cloning.

### Config (defaults < file < env)

First file found: `$XSEARCH_CONFIG`; otherwise `$XDG_CONFIG_HOME/xsearch`, `~/.config/xsearch`, or `%APPDATA%\xsearch` on Windows.
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

This is an orchestration contract, not a suggestion. A leaf is one `xsearch` process. `N x Q` means **N distinct leaves, each with plan size Q**.

### 0. Choose the role

- **Parent mode:** you own the user request. Follow sections 1–6 and create the route objects.
- **Leaf mode:** your task already contains one route object or explicitly says to execute one route. Skip sections 1–4, execute section 5 once, return its note, and stop.

A leaf never reloads this process as a new parent and never creates hypotheses, routes, or children.

### 1. Form the question

1. Rewrite the request into a self-contained goal: entity + current time boundary + dimension. Resolve the real goal when the wording shows an X–Y mismatch.
2. Ask one short preference question only when different answers would materially change the search. A user saying “just search” waives that question, not the multi-route default.
3. Keep unverified user claims neutral until retrieval supports them. Use local context to sharpen names and terms, but do not treat it as public evidence.

**Done when:** the goal is searchable without hidden conversation context.

### 2. Build hypotheses and choose scope

Before calling a leaf, write 2–6 competing hypotheses or aspects. Each must say what evidence would support or weaken it.

| Scope | Signal | First round |
| --- | --- | --- |
| Narrow | one entity and one factual goal | **2 routes x Q5** |
| Medium | 2–3 dimensions, objects, or explanations | **3–4 routes x Q5–10** |
| Wide | landscape, controversy, multi-hop, or 4+ dimensions | **4–6 routes x Q10–20** |
| Explicit direct | user says one call, no split, or raw result | **1 route x Q5** |

Hard cap: 8 routes. Runtime limits still apply. A simple question is **narrow**, not automatically direct.

### 3. Plan orthogonal routes

Create all route objects before dispatch. Each route has:

- `route_id`, `route_name`, `query`, `Q`
- `target_dimension`, `output_goal`
- `targets_hypotheses`, `decision_value`
- a compact `context_pack` when needed

Routes must differ materially by dimension, time window, entity set, evidence class, or decision goal. Merge paraphrase duplicates before execution. Keep at most one broad survey route.

**Done when:** exactly N distinct route objects exist and each has one concrete query.

### 4. Dispatch a real fan-out

1. When the host has isolated child/task agents, dispatch **N children concurrently**: one child = one route = one leaf.
2. Otherwise use the host’s parallel tool/process facility to run **N independent CLI calls in one batch**.
3. Only when neither form of concurrency exists may routes run sequentially; they remain N separate calls.
4. Dispatch the whole round before waiting. A child never creates more routes or children.
5. One large-Q call never substitutes for multiple routes.
6. After the host reports a route as failed, timed out, cancelled, or malformed, the parent creates its failure note and continues fan-in; do not wait forever for a missing child.

Do not claim fan-out until the route ledger contains N terminal notes and records which have receipts/manifests. If fewer than N succeeded, report a partial fan-out; do not silently relabel it successful.

### 5. Execute one leaf

For each route:

1. Run exactly one `xsearch --json "<query>" Q`; allow at most one targeted follow-up after fan-in.
2. Read the receipt, then `manifest_path`. Select only the item files needed for this route; do not load `report.json` wholesale.
3. Return a short structured note, never the artifact bodies:

```json
{
  "route_id": "R1",
  "round": 1,
  "status": "ok|failed|timeout|cancelled|malformed",
  "error": null,
  "query_used": "...",
  "Q": 5,
  "manifest_path": "... or null",
  "findings": ["3–7 concise signals"],
  "supports": ["H1"],
  "weakens": ["H2"],
  "conflicts": [],
  "gaps": [],
  "new_entities": [],
  "next_route_candidates": [],
  "links": [{"title": "...", "url": "https://..."}],
  "info_yield": "ok|empty|refused|thin|failed",
  "confidence": "low|medium|high"
}
```

A failed, empty, or refused route still gets a note. If the child cannot return one, the parent synthesizes it from the terminal host error with `manifest_path: null`, empty findings, and `info_yield: failed`. Confidence describes the written findings, not result volume.

If a child returns malformed prose, the parent makes one best-effort extraction of `route_id`, `query_used`, `findings`, `conflicts`, `gaps`, and `links`. If that fails, record `status: malformed`, `confidence: low`, and never expand from that note.

### 6. Fan in, score, and decide

1. Collect one note for every planned route, including failures.
2. Score routes qualitatively on `info_gain`, `conflict_resolution`, `novelty`, `actionability`, `redundancy`, and `uncertainty`.
3. Merge mutually supporting claims; label single-route claims. Put genuine conflicts and unknowns in separate sections.
4. Dedupe URLs across notes and leaf `deduped_urls`. Prefer primary/official evidence; never invent a link.
5. Expand only when a new entity, unresolved conflict, missing mechanism, or high-value gap could change the answer. Run at most one follow-up round using the best 1–3 candidates.
6. For that round, create new route objects with unique IDs (`F1`, `F2`, …), dispatch them by section 4, collect section 5 notes with `round: 2`, and merge them into the same route ledger before answering.
7. Stop when new routes would be paraphrases, results repeat, or the user’s question is answered. Never expand from failed or malformed notes.
8. Answer in plain language with conclusions first, then conflicts/gaps and a few useful links. Do not expose N/Q terminology unless the user asks about execution.

**Done when:** the answer is backed by a verifiable route count, reports important conflicts/gaps, and does not dump leaf transcripts.

---

## Guardrails

- Never invent URLs or turn an empty route into a world-level claim.
- The binary enforces **count Q**, not route diversity. The parent owns hypotheses, routes, dispatch, and fan-in.
- Default: receipt → manifest → selected item files. Keep full artifacts out of parent context.
- No MCP client and no local search daemon required.
- Logging only via `XSEARCH_LOG_DIR`; this skill does not own dogfood scoring.
