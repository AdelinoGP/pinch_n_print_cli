# Slicer Report (HTML)

Opt-in debugging artifact emitted by `slicer-host` when `--report <PATH.html>`
is passed. Captures per-layer / per-stage / per-module timing, host-side
memory accounting, and the DAG-derived explanation of which modules ran
serially and why.

## CLI

```bash
slicer-host run --model … --module-dir … --output … \
    --report /tmp/slicer-report.html \
    [--report-verbose]      # per-layer-per-module rows; off by default
```

When `--report` is absent: no allocator counters are incremented, no
collector is installed, the pipeline runs through `run_pipeline_with_raw_config`
exactly as it did before this feature existed. Cost: one relaxed atomic
load per allocation (from the global `AccountingAllocator` wrapper) plus
inlined-to-nothing `NoopInstrumentation` calls at each bracket point.

## What the report shows

- **Header**: model path, total wall-clock, layer count, module-call count,
  peak host memory in bytes, threads observed, peak concurrent layers.
- **Phase Totals**: PrePass / PerLayer (sum of per-layer wall-clock) / PostPass.
- **Per-Module Aggregate (per-layer tier)**: by module id — calls, total ms,
  mean, p95, peak host Δ.
- **Per-Layer table**: one row per layer with duration, worker thread,
  stages count, modules count, host bytes delta, host bytes peak.
- **Per-Stage Aggregate**: every stage that ran, with tier color-coded.
- **Parallelism Gantt (SVG)**: one row per rayon worker thread, showing
  which layers were processed on which thread and when.
- **Serial Edges**: collapsible `<details>` per stage with rows like
  `module-a → module-b  (IrWriteRead: PerimeterIR.regions.walls)` —
  the answer to "why couldn't these run in parallel?".

The HTML is a single self-contained file (~150 KB on a 1000-layer print;
larger with `--report-verbose`). No external assets, one inline `<style>`
and a tiny inline `<script>`.

## Architecture

- `slicer_host::instrumentation` (`src/instrumentation.rs`) — the
  `PipelineInstrumentation` trait, `Phase` / `TierKind` / `EdgeReason` /
  `SerialEdge` types, `NoopInstrumentation`, and the
  `compute_serial_edges_*` helpers.
- `slicer_host::report` (`src/report/`) — the consumer side:
  - `allocator.rs` — `AccountingAllocator<A: GlobalAlloc>` with a thread-local
    scope stack and a global `enable()` flag.
  - `model.rs` — `Report`, `LayerRecord`, `StageRecord`, `ModuleRecord`,
    `ParallelismRecord`.
  - `collector.rs` — `Collector` impl `PipelineInstrumentation`. Uses a
    thread-local scope stack so rayon workers don't contend on a Mutex
    per bracket — only finalized records cross the lock.
  - `render.rs` — `format!`-based HTML, inline CSS, inline SVG Gantt.

Hook points: `pipeline.rs::run_pipeline_with_instrumentation` brackets
each phase; `layer_executor.rs::execute_single_layer` brackets layer /
stage / module boundaries. The pipeline does *not* fire per-stage or
per-module brackets inside `prepass.rs` or `postpass.rs` — those phases
get phase-level totals only (see v1 limitations below).

## Global allocator contract

`slicer-host` installs `AccountingAllocator<System>` as its
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

1. **Prepass / Postpass have phase-level brackets only.** Per-stage and
   per-module data is not collected for those phases (they typically
   account for under 10% of slice time; the per-layer tier — the
   dominant cost — has full granularity). Lifting this requires
   refactoring `execute_prepass_with_builtins_configured` (550 LOC) and
   `execute_postpass` (273 LOC) to expose per-stage execution.
2. **WASM linear-memory deltas are zero.** wasmtime's typed component-model
   bindings (`WasmRuntimeDispatcher`) do not expose `memory.data_size()`
   the way classic modules do. The host allocator still captures
   wasmtime's *host-side* bookkeeping; what's missing is the linear
   memory the guest sees. Wiring this in requires either dropping to
   untyped instantiation or threading a memory export through every
   world's bindgen.
3. **Serial-edge reasons at runtime cover only `IrWriteRead`.**
   `CompiledModule` does not carry `requires_modules`, so the runtime
   helper (`compute_serial_edges_from_compiled`) cannot emit
   `EdgeReason::ExplicitRequires` reasons. Topological order in
   `stage.modules` still reflects explicit-requires dependencies — the
   report just doesn't label them with that reason. The
   `LoadedModule`-side helper (`compute_serial_edges_for_stage`)
   handles both reasons; future work could plumb `requires_modules`
   into `CompiledModule` or thread `LoadedModule`s through to the
   collector at plan-freeze time.
4. **Phase markers don't include claim conflicts.** The existing DAG
   builder doesn't produce claim-conflict edges (claims block plan
   validation entirely). If the validation model evolves to allow
   claim-induced ordering, add an `EdgeReason::ClaimConflict` variant.

## Test coverage

`crates/slicer-host/tests/slicer_report_html_tdd.rs` exercises the
collector directly via the trait surface and asserts the HTML contains
every expected section, stage id, and reason label. No real WASM, no
mesh, no pipeline — fast, deterministic, runs in <1s.

## Benchmarks

`crates/slicer-host/benches/pipeline.rs` measures the instrumentation
overhead (Noop vs Collector) so regressions in the report stack don't
silently tax the no-report path.
