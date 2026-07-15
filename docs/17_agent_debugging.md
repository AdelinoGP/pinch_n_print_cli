# Agent CLI Debugging — `--instrument-stderr`, `dag`, and `diagnose`

This page is a practical guide for LLM agents (and any caller in a stateless
tool-call setting) that need to investigate a slow / failing slice or
introspect the static module DAG without launching a full slice.

The mechanisms are zero-dependency CLI extensions to `pnp_cli`:

| Capability                            | Command                             | Notes                                                  |
|---------------------------------------|-------------------------------------|--------------------------------------------------------|
| Live per-stage / per-module timing    | `pnp_cli slice --instrument-stderr` | Bumps event schema to `"1.1.0"`; composable with `--report`. |
| Stage / module / claim introspection  | `pnp_cli dag <subcommand>`          | Manifest TOML only — no WASM compilation.              |
| Manifest validation                   | `pnp_cli module diagnose`           | Structured JSON, exit codes 0 / 1 / 2.                 |

Spec: `docs/specs/agent-cli-debugging.md`.
Event contract: `docs/09_progress_events.md`.
Geometry-stage visualization: `docs/19_visual_debug.md`. Use it independently
when the question is where a visible toolpath defect first appears; this guide
remains the surface for timing, DAG, and manifest diagnosis.

## Live slice instrumentation

```
pnp_cli slice \
    --model resources/benchy.stl \
    --module-dir modules/core-modules \
    --output /tmp/out.gcode \
    --instrument-stderr 2> /tmp/events.jsonl
```

Then `tail -f /tmp/events.jsonl` and grep for the event of interest. New
events emitted under the flag:

- `stage_start` / `stage_complete` — `elapsed_ms` on complete.
- `module_start` / `module_complete` — `elapsed_ms` and `wasm_peak_kb`
  (ceiling-rounded KiB; `0` for host built-ins) on complete.

The existing `phase_*` / `layer_*` / `module_error` / `slice_complete`
events still appear and are unchanged.

To produce both the live JSONL stream and the HTML report on one run,
pass both flags:

```
pnp_cli slice \
    --model resources/benchy.stl --module-dir modules/core-modules \
    --output /tmp/out.gcode --instrument-stderr --report /tmp/report.html
```

## DAG introspection

All `dag` subcommands take `--module-dir <PATH>` (repeatable),
`--no-default-module-paths`, and optionally `--model <PATH>` (for
attaching per-object context to the output). They never compile WASM and
respond in well under 100 ms regardless of module count.

### `dag stages`

Every stage with its tier, module count, and distinct claim count.

```
pnp_cli dag stages --module-dir modules/core-modules --no-default-module-paths
```

### `dag stage <id>`

Full detail for one stage — every module's claims, IR access masks,
`requires_modules`, and config keys, plus the intra-stage serial edges
with flattened reasons (`"ir_write_read: <path>"` or `"explicit_requires"`).

```
pnp_cli dag stage "Layer::Infill" --module-dir modules/core-modules
```

Stage ids are the canonical scheduler ids (with `PrePass::`, `Layer::`,
`PostPass::` prefixes — same as `STAGE_ORDER` in
`crates/slicer-runtime/src/execution_plan.rs`).

### `dag depends <module-id>`

Upstream and downstream edges for a single module, computed across the
full module set so edges that cross stage boundaries are visible (each
edge carries `from_stage` and `to_stage`).

```
pnp_cli dag depends "com.core.gyroid-infill" --module-dir modules/core-modules
```

`--model <PATH>` attaches the model's object ids to the output's
`object_ids` field for downstream correlation.

### `dag claims`

Every claim with its holders, requesters, and an `interchangeable` flag
that is `true` when more than one module declares the same claim in
`claims.holds` (the scheduler picks one holder per region; multiple
holders make them interchangeable).

```
pnp_cli dag claims --module-dir modules/core-modules
```

## Diagnose

Run the manifest-loading and DAG-validation passes against a module
tree and emit `{pass, modules_loaded, stages, diagnostics: [...]}` to
stdout. Exit codes:

- `0` — `pass: true`, no errors.
- `1` — at least one `error`-level diagnostic. This includes an unreadable
  `--module-dir` root (nonexistent, permission denied, not a directory):
  that root is skipped and reported as an `error`-level diagnostic naming
  it, not a hard failure — other roots are still scanned.
- `2` — a malformed manifest **file** inside an otherwise-readable root
  (bad TOML, schema violation, missing companion `.wasm`); `load_modules_from_roots`
  returned `LoadError`.

```
pnp_cli module diagnose --module-dir modules/core-modules
```

## Worked example — find a slow module

1. Run with `--instrument-stderr` and redirect stderr to a file.
2. `grep '"event":"module_complete"' /tmp/events.jsonl | jq -s 'group_by(.module_id) | map({m: .[0].module_id, total: (map(.elapsed_ms) | add)})'`
3. Pick the module with the highest total `elapsed_ms`.
4. `pnp_cli dag depends <that-module-id> --module-dir modules/core-modules`
   to see what feeds it and what it feeds.
5. `pnp_cli dag stage <its-stage> --module-dir modules/core-modules`
   to see config keys, IR access, and intra-stage edges.
6. If the wiring looks fine, the cost is intrinsic to the module — file an
   issue against that module's owners, or investigate config that drives
   its work (e.g. `density`, `pattern`).
