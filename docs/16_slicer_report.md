# Slicer Report (HTML)

Opt-in debugging artifact emitted by `pnp_cli` when `--report <PATH.html>`
is passed. Captures per-layer / per-stage / per-module timing, host-side
memory accounting, and the DAG-derived explanation of which modules ran
serially and why.

## Build configuration

The report subsystem is gated behind the default-enabled `report` Cargo feature on `slicer-runtime`. Build with `cargo build --no-default-features -p slicer-runtime` to omit it; the `--report` flag is then absent from `pnp_cli slice`.

## Related G-code artifacts

The slicer report does not include the G-code preamble itself (header lines,
thumbnail PNG block, config-dump block); those are emitted directly into the
`.gcode` file by `PostPass::GCodeEmit`. Packet 55 standardised the preamble
format. See `docs/01_system_architecture.md` GCodeEmit section for the contract.

Machine start / end G-code (`machine_start_gcode` / `machine_end_gcode` config
keys) is emitted by a designated finalization module before the first layer and
after the last layer. Macro expansion is documented in
`docs/03_wit_and_manifest.md` 'Machine start / end G-code emission' (packet 59).
The slicer report records these as `phase_start` / `phase_complete` markers but
does not include the literal G-code text.

## CLI

```bash
pnp_cli slice --model … --module-dir … --output … \
    --report /tmp/slicer-report.html \
    [--report-verbose]      # per-layer-per-module rows; off by default
```

When `--report` is absent: no allocator counters are incremented, no
collector is installed, the pipeline runs through `run_pipeline_with_raw_config`
exactly as it did before this feature existed. Cost: one relaxed atomic
load per allocation (from the global `AccountingAllocator` wrapper) plus
inlined-to-nothing `NoopInstrumentation` calls at each bracket point.

### Parent-directory creation and error semantics (Normative — Packet 65)

Parent directories for both `--output` and `--report` paths are created
automatically when missing, via `pnp_cli::io::write_with_parents`. The
two flags differ on what happens when directory creation fails:

- **`--output`** — a failure to create the parent directory causes the
  CLI to exit non-zero with a structured error. The slice does NOT
  proceed (no G-code, no partial output).
- **`--report`** — a failure to create the parent directory emits a
  `log::warn!` and the slice CONTINUES. G-code is still written to
  `--output`; the HTML report is skipped. The report is a debugging
  aid; a missing report directory should not block a successful slice.

Both flags accept `PathBuf` (Packet 65 normalised every path arg in the
`Slice` subcommand to `PathBuf`; no string-typed path arguments remain).

## What the report shows

- **Header**: model path, total wall-clock, layer count, module-call count,
  peak host memory in bytes, threads observed, peak concurrent layers.
- **Phase Totals**: PrePass / PerLayer / PostPass with two time columns:
  **Wall (ms)** — actual elapsed wall-clock for the phase bracket; and
  **Worker total (ms)** — aggregate thread time (sum of per-duration across
  all workers). For the PerLayer row, the worker total may exceed the wall
  value when layers run in parallel; for sequential phases (PrePass,
  PostPass) the two are identical. A note under the table explains the
  distinction.
- **Per-Module Aggregate (per-layer tier)**: by module id — calls, total ms,
  mean, p95, peak host Δ, peak WASM linear memory.
- **Per-Layer table**: one row per layer with duration, worker thread,
  stages count, modules count, host bytes delta, host bytes peak.
- **Per-Stage Aggregate**: every stage that ran, with tier color-coded.
- **Parallelism Gantt (SVG)**: one row per rayon worker thread, showing
  which layers were processed on which thread and when.
- **Serial Edges**: collapsible `<details>` per stage with rows like
  `module-a → module-b  (IrWriteRead: PerimeterIR.regions.walls)` —
  the answer to "why couldn't these run in parallel?". Auto-collapsed
  when there are more than 3 stages to keep initial scroll length compact.
- **Per-Layer-Per-Module (verbose)**: opt-in via `--report-verbose`.
  One row per module call: layer index, stage, module, duration, host
  peak Δ, WASM peak. Off by default because it scales as
  O(layers × stages × modules).

The HTML is a single self-contained file (~60–150 KB without
`--report-verbose`; can grow to MBs with it). No external assets,
one inline `<style>`, no JavaScript.

## LLM-readable JSON data block

Every slicer report embeds a `<script type="application/json" id="slicer-report-data">`
block containing a curated JSON summary of phase timing, per-module
aggregates, per-layer summaries, memory, and thread usage. The block is
invisible in visual rendering (no `<style>` or `display:none` targets the
`slicer-report-data` id; browsers ignore `<script>` tags with unrecognised
MIME types). It is intended for LLMs and automated analysis tools that
parse HTML structurally rather than scraping table markup.

Top-level keys:

- `total_wallclock_ms` (f64): total slice wall-clock in milliseconds
- `peak_host_memory_bytes` (u64): peak host allocator bytes
- `layer_count` (u32), `module_count` (u32), `threads_observed` (\[String\]),
  `max_layers_concurrent` (usize)
- `phases` (object): `prepass` / `perlayer` / `postpass` each with
  `wall_ms` and `worker_total_ms`
- `module_aggregates` (\[object\]): each object has `module_id`, `calls`,
  `total_ms`, `mean_ms`, `p95_ms`, `peak_host_delta_bytes`, `wasm_peak_bytes`
- `per_layer_summary` (\[object\]): each object has `layer_index`, `z_mm`,
  `duration_ms`, `worker`, `stages`, `modules`, `host_delta_bytes`,
  `host_peak_bytes`

The JSON block exists for all report states, including empty / zero-layer
runs where arrays are empty and counts are zero.

## Architecture

- `slicer_runtime::instrumentation` (`src/instrumentation.rs`) — the
  `PipelineInstrumentation` trait, `Phase` / `TierKind` / `EdgeReason` /
  `SerialEdge` types, `NoopInstrumentation`, and the
  `compute_serial_edges_*` helpers.
- `slicer_runtime::report` (`src/report/`) — the consumer side:
  - `allocator.rs` — `AccountingAllocator<A: GlobalAlloc>` with a thread-local
    scope stack and a global `enable()` flag.
  - `model.rs` — `Report`, `LayerRecord`, `StageRecord`, `ModuleRecord`,
    `ParallelismRecord`. Only `PhaseWallTimes` derives `Serialize`; the
    other model structs are deliberately serde-free (see "LlmReport
    curation pattern" below).
  - `collector.rs` — `Collector` impl `PipelineInstrumentation`. Uses a
    thread-local scope stack so rayon workers don't contend on a Mutex
    per bracket — only finalized records cross the lock.
    Phase wall-clock timestamps live in `phase_*_wall_ns: Mutex<u64>`
    fields on the collector (PrePass / PerLayer / PostPass — Packet 66).
    All phase callbacks are single-threaded (main thread only), so the
    Mutex is uncontended; it is used for consistency with the rest of
    the collector's locking pattern, not for synchronisation.
  - `render.rs` — `format!`-based HTML, inline CSS, inline SVG Gantt.
    Also defines `LlmReport`, a render-private curation struct selecting
    a subset of `Report` fields for the JSON data block (Packet 66 —
    avoids forcing `serde::Serialize` onto every model struct or onto
    transitive types like `TierKind` / `SerialEdge`). The curation
    approach is deliberate; reviewers should not propose deriving
    `Serialize` on model.rs structs to "simplify" the JSON path.

Hook points: `pipeline.rs::run_pipeline_with_instrumentation` brackets
each phase; `layer_executor.rs::execute_single_layer` brackets layer /
stage / module boundaries for per-layer; `prepass.rs` and `postpass.rs`
have `_with_instrumentation` variants that bracket per-stage and
per-module for those tiers, including host built-ins.

## Global allocator contract

`pnp_cli` (via `slicer-runtime`) installs `AccountingAllocator<System>` as its
`#[global_allocator]`:

```rust
#[global_allocator]
static ALLOC: AccountingAllocator<std::alloc::System> =
    AccountingAllocator::new(std::alloc::System);
```

Downstream packagers who want a different allocator (jemalloc, mimalloc)
must wrap it the same way:

```rust
#[global_allocator]
static ALLOC: AccountingAllocator<MimallocOrWhatever> =
    AccountingAllocator::new(MimallocOrWhatever);
```

Adding a second `#[global_allocator]` anywhere in the workspace will
cause a link-time conflict. There is currently no other global allocator
declared anywhere in this workspace.

## v1 limitations

These are deliberate tradeoffs for the initial implementation. None affect
correctness; they bound the level of detail the report can surface.

1. **Phase markers don't include claim conflicts.** The existing DAG
   builder doesn't produce claim-conflict edges (claims block plan
   validation entirely). If the validation model evolves to allow
   claim-induced ordering, add an `EdgeReason::ClaimConflict` variant.

## WASM linear-memory sampling

Each per-call `wasmtime::Store` installs a `MemTracker` (in
`crates/slicer-runtime/src/wit_host.rs`) as its `ResourceLimiter`. The
limiter records every `memory.grow` notification (including the initial
instantiation grow) and surfaces two values per dispatch:

- `wasm_initial_bytes` — linear-memory size right after instantiation
  but before the export call runs (the module's static baseline).
- `wasm_peak_bytes` — highwater observed across the whole dispatch.

The per-call `(initial, peak)` is stashed in a thread-local read by
`LayerStageRunner::last_wasm_mem_sample` (default impl returns `(0, 0)`
for non-wasm runners), then handed to `on_module_end`. Test mocks and
host built-ins leave the WASM columns blank without any extra wiring.

## Test coverage

`crates/slicer-runtime/tests/slicer_report_html_tdd.rs` exercises the
collector directly via the trait surface and asserts the HTML contains
every expected section, stage id, and reason label. No real WASM, no
mesh, no pipeline — fast, deterministic, runs in <1s.

## Benchmarks

`crates/slicer-runtime/benches/pipeline.rs` measures the instrumentation
overhead (Noop vs Collector) so regressions in the report stack don't
silently tax the no-report path.
