# Agent CLI Debugging — `--instrument-stderr`, `dag`, and `diagnose`

This page is a practical guide for LLM agents (and any caller in a stateless
tool-call setting) that need to investigate a slow / failing slice or
introspect the static module DAG without launching a full slice.

The mechanisms are zero-dependency CLI extensions to `pnp_cli`:

| Capability                            | Command                             | Notes                                                  |
|---------------------------------------|-------------------------------------|--------------------------------------------------------|
| Live per-stage / per-module timing    | `pnp_cli slice --instrument-stderr` | Emits the instrumented event stream (schema `"1.2.0"`); composable with `--report`. |
| Sub-module cost attribution           | `pnp_cli slice --profile`           | Fuel per module *and per scope*; ranked table on stderr. |
| Re-read a profile capture             | `pnp_cli profile --from <jsonl>`    | No re-slice. Same table, from an existing capture.     |
| Stage / module / claim introspection  | `pnp_cli dag <subcommand>`          | Manifest TOML only — no WASM compilation.              |
| Manifest validation                   | `pnp_cli module diagnose`           | Structured JSON, exit codes 0 / 1 / 2.                 |

Spec: `docs/specs/_OLD/agent-cli-debugging.md` (superseded spec, retained for the design write-up).
Event contract: `09_progress_events.md`.
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
- `module_log` — one event per log record a module wrote through
  `slicer_sdk::host::log`, carrying `level` and `message` alongside the usual
  `module_id` / `stage` / `phase` / `layer_index`. Every occurrence is
  emitted, so `grep '"event":"module_log"' … | jq -s 'group_by(.message) | map({m: .[0].message, n: length})'`
  gives an exact hit count per message. The human-readable stderr channel
  (the `log` facade, filtered by `RUST_LOG`, default `warn`) instead collapses
  identical `(level, message)` pairs to one line and reports the suppressed
  counts once at slice end — use this stream, not that one, when you need
  frequencies.

The existing `phase_*` / `layer_*` / `module_error` / `slice_complete`
events still appear and are unchanged.

To produce both the live JSONL stream and the HTML report on one run,
pass both flags:

```
pnp_cli slice \
    --model resources/benchy.stl --module-dir modules/core-modules \
    --output /tmp/out.gcode --instrument-stderr --report /tmp/report.html
```

## Fuel profiling — `--profile`

`--instrument-stderr` ranks *modules*. `--profile` says what inside a module
costs, and does it with a signal that a noisy developer machine cannot corrupt.
Design and rationale: `docs/adr/0055-fuel-based-module-profiling.md`.

```
pnp_cli slice \
    --model resources/benchy.stl \
    --module-dir modules/core-modules \
    --output /tmp/out.gcode \
    --profile 2> /tmp/profile.jsonl
```

The ranked table goes to **stderr** at slice end (G-code owns stdout), after
the JSONL stream:

```
=== fuel profile (ADR-0055) ===
units: fuel = executed wasm instructions (deterministic; mark overhead costs no fuel)
total: 12,943,955,254 fuel across 15 module(s)

com.core.classic-perimeters                fuel  11.6%     1,500,901,794  calls 200
  polygon_ops::offset2_ex                    7.4%       961,313,415  calls 560
  polygon_ops::offset                        2.1%       275,727,712  calls 2200
  polygon_ops::clip_polygons                 0.8%        97,614,703  calls 880
  <module self>                              1.3%       166,245,964

units: wall-clock (native host built-ins are NOT fuel-metered)
total: 916.6 ms across 5 built-in(s)

host:slice                                 wall  61.6%          564.6 ms  calls 1832
  polygon_ops::offset                       56.1%          514.6 ms  calls 780
  polygon_ops::clip_polygons                 5.0%           45.7 ms  calls 1049
  <module self>                              0.0%           74.2 us
```

Reading it:

- **Two sections, two units, two denominators.** Guest modules are ranked by
  wasmtime fuel; host built-ins (`host:slice`, `host:shell_classification`,
  `host:paint_segmentation`, …) are native `slicer-core` code that wasmtime does
  not meter, so they can only be ranked by wall-clock. They share the
  `polygon_ops::*` vocabulary but are never mixed into one ranking — each
  section's percentages are shares of *its own* total.
- **Percentages decompose exactly.** A module's scope rows plus its
  `<module self>` row sum to the module's own percentage. `<module self>` is
  `module total − Σ scope self` — the module's own code plus anything it called
  that carries no marks. A large `<module self>` means the cost is somewhere
  that is not yet marked, not that the module is cheap.
- **`calls` on a scope row** counts scope activations, not dispatches; `calls`
  on a module row counts dispatches.
- **`host:native`** is the fallback bucket for marked native geometry that no
  host built-in bracket claimed (e.g. reached from the per-layer tier). It is
  attributed honestly rather than credited to whichever built-in ran last.
- Rows and ties are sorted deterministically (cost descending, then id), so two
  runs of one workload render identically and `diff` is a valid A/B tool.

### The observer-effect contract (read before quoting a number)

- **Fuel ratios are exact.** A scope mark is a host call, and host calls burn no
  fuel, so measuring cannot perturb the fuel it measures. Fuel is also a
  deterministic instruction count: identical across runs and across machines,
  given identical guest inputs. An A/B on fuel is a comparison, not a hypothesis
  test.
- **Wall-clock under `--profile` is inflated** — every mark costs a guest→host
  transition and the marked primitives sit in hot loops. Treat the wall-clock
  section and the wall columns as *indicative only*: use them to see which
  scopes dominate, never as an absolute timing. **Absolute milliseconds come
  from a profiling-off run** (`--instrument-stderr` alone).
- Fuel metering costs throughput, which is why it rides its own flag: a plain
  `--instrument-stderr` run is completely unaffected, and so is a run with no
  flags. Without `--profile`, no `profile_summary` event is emitted and no
  event carries a `profile` or `profile_scopes` key.
- Fuel is deterministic *given identical guest inputs*. DEV-093 currently makes
  guest inputs vary run to run on a handful of layers, so whole-slice totals can
  drift slightly until that is fixed.

### `--profile-verbose`

Aggregation is the default: a 0.2 mm benchy emits thousands of
`module_complete` events, and hanging a scope array off each of them would force
every consumer to write a reducer before reading anything. `--profile-verbose`
is the opt-in for the opposite question — *which single call* was pathological —
and mirrors `--report-verbose`. It attaches one call's fold to each
`module_complete` as `profile_scopes`, and requires `--instrument-stderr` to be
visible (that is the tier which emits `module_complete` at all):

```
pnp_cli slice … --profile --profile-verbose --instrument-stderr 2> /tmp/p.jsonl
jq -c 'select(.profile_scopes.scopes | length > 0)
       | {m: .module_id, layer: .layer_index, fuel: .profile_scopes.call_fuel}' /tmp/p.jsonl \
  | sort -t: -k3 -rn | head
```

### JSONL surface (schema `1.5.0`)

Additive on top of `1.4.0`; a consumer that ignores unknown event types and
unknown keys is unaffected.

- `profile_summary` — exactly one per profiled slice, emitted at slice end and
  strictly **before** `slice_stats` / `slice_complete`, so a consumer that stops
  reading at `slice_complete` never misses it. Carries the whole run's fold in a
  `profile` object: `{fuel_total, wall_total_ns, modules: [{module_id, unit,
  calls, total_fuel, total_wall_ns, self_fuel, self_wall_ns, scopes: [{scope,
  scope_id, calls, self_fuel, total_fuel, self_wall_ns, total_wall_ns}]}]}`.
  `unit` is `"fuel"` or `"wall_ns"` — always branch on it before comparing rows.
- `profile_scopes` on `module_complete` — `--profile-verbose` only.

### `pnp_cli profile --from`

A profiled slice is expensive; re-running one to look at the numbers again is
the mistake this command exists to prevent. Same ergonomic as
`cargo xtask test --summary-from`: the work already landed on disk, so read it.

```
pnp_cli profile --from /tmp/profile.jsonl          # the ranked table
pnp_cli profile --from /tmp/profile.jsonl --json   # the payload, for jq
pnp_cli slice … --profile 2>&1 | pnp_cli profile --from -
```

It tolerates everything a stderr capture picks up alongside events (startup DAG
advisories, `env_logger` lines, blanks) and scans for the `profile_summary`
line. If a capture holds several runs, the last summary wins. A capture taken
without `--profile` fails with a message naming the missing flag rather than
rendering an empty table that would read as "nothing was slow".

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
`crates/slicer-scheduler/src/execution_plan.rs`).

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
6. If the wiring looks fine, the cost is intrinsic to the module. Re-run with
   `--profile` and read the module's scope rows: a dominant `polygon_ops::*`
   row points at clipper2 work, while a dominant `<module self>` says the cost
   is in the module's own code and no existing mark covers it.
7. To check whether a change helped, compare **fuel**, not milliseconds — it is
   exact and machine-independent, so a 3% improvement is visible where
   wall-clock noise would swallow it whole.
