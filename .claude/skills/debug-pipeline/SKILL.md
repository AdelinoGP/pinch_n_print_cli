---
name: debug-pipeline
description: Investigate a slow or failing slice and introspect the static module DAG using `pnp_cli slice --instrument-stderr`, `pnp_cli dag <subcommand>`, and `pnp_cli module diagnose`. Use when the user asks "which module is slow", "why does X depend on Y", "diagnose modules", "inspect the DAG", "investigate slice timing", "what wires X to Y", or describes a perf regression in the slicer pipeline.
type: anthropic-skill
version: "1.0"
metadata:
  internal: true
---

# Debug the slicer pipeline

The `pnp_cli` binary exposes two zero-WASM CLI surfaces and one
instrumented run mode for live debugging. Pick the cheapest tool that
answers the question — `dag` and `module diagnose` never compile WASM, so they
return in well under 100 ms even on large module sets.

Spec: `docs/specs/agent-cli-debugging.md`.
Event contract: `docs/09_progress_events.md` (instrumented stream section).
Agent guide: `docs/17_agent_debugging.md`.

---

## Step 1 — Pick the right tool

| Question                                  | Tool                                                    |
|-------------------------------------------|---------------------------------------------------------|
| "Which module is slow right now?"         | `pnp_cli slice --instrument-stderr`, watch `module_complete`|
| "Is WASM memory growing on layer N?"      | `pnp_cli slice --instrument-stderr`, watch `wasm_peak_kb`   |
| "Why does stage X serialize to stage Y?"  | `pnp_cli dag stage <id>`                                |
| "What depends on `<module>`?"             | `pnp_cli dag depends <module>`                          |
| "Is this manifest tree valid?"            | `pnp_cli module diagnose`                               |
| "What stages exist?"                      | `pnp_cli dag stages`                                    |
| "Who holds claim X? Are they swappable?"  | `pnp_cli dag claims`                                    |

Never run a full slice just to read the DAG — use the `dag` subcommands.

---

## Step 2 — Live timing

```
pnp_cli slice \
    --model resources/benchy.stl \
    --module-dir modules/core-modules \
    --output /tmp/out.gcode \
    --instrument-stderr 2> /tmp/events.jsonl
```

Stream-read `/tmp/events.jsonl`. Match on the `event` field:
`stage_start`, `module_complete` (`elapsed_ms`, `wasm_peak_kb`),
`stage_complete`, plus the existing `phase_*` / `layer_*` events.

Composable with `--report` — both will run, both will emit.

---

## Step 3 — Find the slow module

```
grep '"event":"module_complete"' /tmp/events.jsonl \
    | jq -s 'group_by(.module_id) | map({m: .[0].module_id, total: (map(.elapsed_ms) | add)}) | sort_by(.total) | reverse | .[0:5]'
```

Then drill in:

```
pnp_cli dag depends "<that-module-id>" --module-dir modules/core-modules
pnp_cli dag stage "<its-stage>"          --module-dir modules/core-modules
```

If the wiring looks fine, the cost is intrinsic to the module: investigate
its config keys (visible in `dag stage` output) or open an issue against
the module owners.

---

## Step 4 — Validate the module tree

```
pnp_cli module diagnose --module-dir modules/core-modules
```

Exit codes: `0` clean, `1` errors, `2` unreadable files. JSON shape:
`{pass, modules_loaded, stages, diagnostics: [{level, file, field?, message}]}`.

---

## Output discipline

Report findings as one short paragraph + the specific event lines or
command output that proves them. Don't paste whole JSONL streams — pick
the ~5 events that matter.
