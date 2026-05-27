---
name: debug-pipeline
description: Investigate a slow or failing slice and introspect the static module DAG using `slicer-host run --instrument-stderr`, `slicer-host dag <subcommand>`, and `slicer-host diagnose`. Trigger phrases include "which module is slow", "why does X depend on Y", "diagnose modules", "inspect the DAG", and "investigate slice timing".
---

# Debug the slicer pipeline

Three zero-WASM CLI surfaces back this skill. Pick the cheapest tool that
answers the question; `dag` and `diagnose` parse manifest TOMLs only and
respond in well under 100 ms.

Reference docs: `docs/specs/agent-cli-debugging.md`,
`docs/09_progress_events.md`, `docs/17_agent_debugging.md`.

## Tool selector

| Question                                  | Command                                             |
|-------------------------------------------|-----------------------------------------------------|
| "Which module is slow right now?"         | `run --instrument-stderr`, watch `module_complete`  |
| "Is WASM memory growing on layer N?"      | `run --instrument-stderr`, watch `wasm_peak_kb`     |
| "Why does stage X serialize to stage Y?"  | `dag stage <id>`                                    |
| "What depends on `<module>`?"             | `dag depends <module>`                              |
| "Is this manifest tree valid?"            | `diagnose`                                          |
| "What stages exist?"                      | `dag stages`                                        |
| "Who holds claim X? Are they swappable?"  | `dag claims`                                        |

## Live slice instrumentation

```
slicer-host run \
    --model resources/benchy.stl \
    --module-dir modules/core-modules \
    --output /tmp/out.gcode \
    --instrument-stderr 2> /tmp/events.jsonl
```

Composable with `--report`. New event types: `stage_start`,
`stage_complete`, `module_start`, `module_complete` (`elapsed_ms`,
`wasm_peak_kb`). Schema bumps to `"1.1.0"`.

Find the slowest module:

```
grep '"event":"module_complete"' /tmp/events.jsonl \
    | jq -s 'group_by(.module_id) | map({m: .[0].module_id, total: (map(.elapsed_ms) | add)}) | sort_by(.total) | reverse | .[0:5]'
```

## DAG introspection

All `dag` subcommands take `--module-dir <PATH>` (repeatable),
`--no-default-module-paths`, and optionally `--model <PATH>`.

- `slicer-host dag stages` — stages with tier, module count, claim count.
- `slicer-host dag stage <id>` — full detail; flat reason strings
  (`"ir_write_read: <path>"` or `"explicit_requires"`).
- `slicer-host dag depends <module-id>` — upstream + downstream global edges.
- `slicer-host dag claims` — claims with holders, requesters,
  `interchangeable`.

## Diagnose

```
slicer-host diagnose --module-dir modules/core-modules
```

Exit codes: `0` clean, `1` errors, `2` unreadable files.

## Output discipline

Report findings as one short paragraph + the specific event lines or
command output that proves them.
