# Xsearch leaf handoff

Load this reference when the parent dispatches one or more routes. Each child task must be self-contained; a child is not required to have the xsearch skill installed or loaded.

## Parent handoff template

Include all of the following in the child task:

```text
You are an xsearch leaf. Execute exactly one assigned route and return one JSON note. Do not create hypotheses, routes, or child agents.

Executable: <resolved absolute xsearch path>
Platform: <macos-linux | windows>
Host tool timeout: <at least 600 seconds, or the configured XSEARCH_TIMEOUT when higher>
Route: <complete route object JSON>

Run the executable once with: --json <route.query> <route.Q>
Read the xsearch.run.v1 receipt, then its manifest_path. Select only the item files needed to summarize this route. Do not read report_path wholesale.
Return one result object matching the contract below. Do not return artifact bodies or free-form prose.
```

The parent replaces every angle-bracket placeholder before dispatch.

## Execution

macOS/Linux:

```bash
"<resolved absolute xsearch path>" --json "<route.query>" <route.Q>
```

Windows PowerShell:

```powershell
& "<resolved absolute xsearch path>" --json "<route.query>" <route.Q>
```

Use the executable path supplied by the parent. This host tool call contains exactly one route command and has its own timeout of at least 600 seconds. Use a separate tool call for every other route.

## Result contract

Allowed values:

| Field | Values |
| --- | --- |
| `status` | `ok`, `failed`, `timeout`, `cancelled`, `malformed` |
| `info_yield` | `ok`, `empty`, `refused`, `thin`, `failed` |
| `confidence` | `low`, `medium`, `high` |

Successful note example:

```json
{
  "route_id": "R1",
  "round": 1,
  "status": "ok",
  "error": null,
  "query_used": "Rust current stable release from official announcements",
  "Q": 5,
  "manifest_path": "/home/user/.cache/xsearch/runs/example/manifest.json",
  "findings": ["The official release announcement names the current stable version."],
  "supports": ["H1"],
  "weakens": [],
  "conflicts": [],
  "gaps": [],
  "new_entities": [],
  "next_route_candidates": [],
  "links": [{"title": "Rust release announcement", "url": "https://blog.rust-lang.org/"}],
  "info_yield": "ok",
  "confidence": "high"
}
```

Failure note example:

```json
{
  "route_id": "R2",
  "round": 1,
  "status": "failed",
  "error": "upstream request timed out",
  "query_used": "Rust stable channel cross-check",
  "Q": 5,
  "manifest_path": null,
  "findings": [],
  "supports": [],
  "weakens": [],
  "conflicts": [],
  "gaps": ["This evidence route did not complete."],
  "new_entities": [],
  "next_route_candidates": [],
  "links": [],
  "info_yield": "failed",
  "confidence": "low"
}
```

Keep `findings` to 3–7 concise signals when evidence exists. Empty and refused responses are observations about retrieval yield, not proof that no answer exists. Every URL must come from the route artifacts.

## Parent fallback for malformed output

Extract these fields once: `route_id`, `query_used`, `findings`, `conflicts`, `gaps`, and `links`. If extraction succeeds, fill omitted fields conservatively. If it fails, write a failure note with `status: malformed`, `manifest_path: null`, `info_yield: failed`, and `confidence: low`. A malformed note never proposes follow-up routes.

**Leaf complete when:** one process produced a receipt and structured note, or one terminal failure note records why it did not.
