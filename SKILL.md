---
name: xsearch
description: "Web search and online research. Use xsearch for every request that needs current public information: simple online facts, latest developments, official-source verification, comparisons, research, multi-angle surveys, or explicit web/Grok search."
---

# xsearch

Turn a natural-language question into a **fan-out ledger** of independent searches, then fan in to a sourced answer.

## Resolve the leaf

Before searching, resolve the executable and verify `--version` exits `0`.

macOS/Linux:

```bash
test -x "${XSEARCH_BIN:-$HOME/.agents/skills/xsearch/bin/xsearch}" || \
  curl -fsSL https://raw.githubusercontent.com/catoncat/xsearch/main/install.sh | bash
"${XSEARCH_BIN:-$HOME/.agents/skills/xsearch/bin/xsearch}" --version
```

Windows PowerShell:

```powershell
$bin = if ($env:XSEARCH_BIN) { $env:XSEARCH_BIN } else { "$HOME\.agents\skills\xsearch\bin\xsearch.exe" }
if (-not (Test-Path $bin)) { irm https://raw.githubusercontent.com/catoncat/xsearch/main/install.ps1 | iex }
& $bin --version
```

For setup, config, artifact fields, or failures, read `references/runtime.md`.

## Choose the role

- **Parent:** you own the user request. Follow the parent process below.
- **Leaf:** your task already contains a self-contained xsearch leaf handoff. Follow that handoff once and return its result. Leaf mode is terminal: one route, one process, one note.

## Ledger invariant

`N x Q` means N ledger routes, each executing one independent xsearch process with plan size Q. A larger Q is deeper retrieval inside one route; it never replaces another route.

Each route maps to one host child ID or tool-call ID. Keep route commands separate: shell separators, chained commands, and multi-route shell scripts are not fan-out.

Every ledger entry has `route_id`, `round`, `query`, `Q`, `host_call_id`, `status`, and eventually `manifest_path` or `error`. A round is complete when all N entries are terminal. It is fully successful only when all N have distinct host call IDs and receipts/manifests.

## Parent process

### 1. Form the question

Rewrite the request as a self-contained goal: entity + current time boundary + dimension. Resolve an X-Y mismatch when the stated query differs from the likely goal. Keep unverified user claims neutral.

Ask one short preference question only when different answers would materially change what gets searched. “Just search” waives the question, not fan-out. Explicit “one call”, “do not split”, or “return the raw result” selects direct mode.

**Complete when:** the goal is searchable without hidden conversation context.

### 2. Choose scope and hypotheses

Write 2–6 competing hypotheses or evidence aspects, each with evidence that would support or weaken it.

| Scope | Signal | First round |
| --- | --- | --- |
| Narrow | one entity and one factual goal | **2 routes x Q5** |
| Medium | 2–3 dimensions, objects, or explanations | **3–4 routes x Q5–10** |
| Wide | landscape, controversy, multi-hop, or 4+ dimensions | **4–6 routes x Q10–20** |
| Direct | user explicitly requests one call/no split/raw result | **1 route x Q5** |

Cap a round at 8 routes and respect runtime limits. A simple question is narrow, not automatically direct.

**Complete when:** scope, N, Q, and the support/weaken conditions are explicit.

### 3. Open the ledger

Create all route objects before dispatch. Each contains:

- `route_id`, `route_name`, `round`, `query`, `Q`
- `target_dimension`, `output_goal`
- `targets_hypotheses`, `decision_value`
- a compact `context_pack` when local context is needed

Routes differ by dimension, time window, entity set, evidence class, or decision goal. Merge paraphrase duplicates and keep at most one broad survey route. Every hypothesis/aspect must be covered by at least one route.

**Complete when:** the ledger contains exactly N unique, covered, dispatchable routes.

### 4. Dispatch the round

Read `references/leaf.md`. For every route, build a self-contained child task from its handoff template, including the resolved executable path, route object, command, and result contract.

Use the host's strongest available execution lane:

1. N isolated child/task agents submitted concurrently; or
2. N separate CLI tool calls submitted in the same assistant turn/batch; or
3. N separate sequential tool calls only when concurrency is unavailable.

One child or tool call owns exactly one route. Set each host call timeout to at least the runtime timeout (default 600 seconds). Submit the entire concurrent round before waiting and record each distinct host call ID as `running`.

**Complete when:** the ledger has N distinct host call IDs submitted before collection begins, or a terminal dispatch error for an unsubmitted route.

### 5. Close the ledger

Collect one terminal note per route. A host failure, timeout, or cancellation becomes a parent-authored failure note. For malformed child output, apply the single extraction fallback in `references/leaf.md`; a failed extraction becomes `status: malformed` and cannot seed expansion.

A completed process must provide a distinct `xsearch.run.v1` receipt and `manifest_path`. Keep artifact bodies in their files; select only the item files needed for that route.

**Complete when:** all N ledger entries are terminal and successful entries have distinct host call IDs, run IDs, and manifest paths.

### 6. Fan in and decide

Score each usable route on information gain, conflict resolution, novelty, actionability, redundancy, and uncertainty. Merge mutually supporting claims; label single-route claims. Put genuine conflicts and unknowns in separate sections. Dedupe URLs and prefer primary/official evidence.

Open one follow-up round only when a new entity, unresolved conflict, missing mechanism, or high-value gap could change the answer. Use the best 1–3 candidates, assign new IDs (`F1`, `F2`, ...), and repeat sections 3–5 with `round: 2`. Failed or malformed notes do not seed follow-up routes.

Stop when proposed routes are paraphrases, results repeat, or the question is answered. Respond with conclusions first, then important conflicts/gaps and a few useful links. Keep N/Q terminology internal unless the user asks about execution.

**Complete when:** the final answer is backed by the closed ledger, preserves material conflicts/gaps, and does not dump leaf transcripts.
