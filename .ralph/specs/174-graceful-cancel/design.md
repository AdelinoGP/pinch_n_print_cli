# Design: 174-graceful-cancel

## Controlling Code Paths

- Primary code path: `pnp_cli::main` slice arm (`crates/pnp-cli/src/main.rs:354-439`, builds `SliceRunOptions` at :409, writes output only after `run_slice` returns `Ok` at :425-433) → `slicer_runtime::run_slice` (`crates/slicer-runtime/src/run.rs:285`, owns the `ProgressChannel` and emits lifecycle events) → `run_pipeline_fork` (`run.rs:194`) → `run_pipeline_core` (`crates/slicer-runtime/src/pipeline.rs:299`, phase brackets) → `execute_per_layer_with_instrumentation` (`crates/slicer-runtime/src/layer_executor.rs:189`, rayon `par_iter` over `plan.global_layers` at :201-215 calling `execute_single_layer`).
- Neighboring tests/fixtures: `crates/pnp-cli/tests/slice_instrumentation_fork_tdd.rs` (assert_cmd pattern, `resources/regression_wedge.stl`, `modules/core-modules`); `crates/slicer-runtime/tests/unit/main.rs` (mod-list aggregator for the `unit` binary); `crates/slicer-runtime/tests/integration/progress_events_tdd.rs` (event-shape assertions that must stay green).
- OrcaSlicer comparison: none — cancellation is a PNP/fork process contract with no canonical Orca counterpart; the orca-delegation snippet is deliberately omitted.

## Architecture Constraints

- Host-side only: no file under `modules/`, `crates/slicer-schema/wit/`, `slicer-sdk`, `slicer-macros`, or `slicer-ir` is touched, so the guest-WASM staleness gate is not triggered by this packet's edits (the pnp-cli e2e test still needs previously built guests on disk).
- No geometry or mm/unit conversion anywhere in this packet; the coordinate-system checklist does not apply.
- Progress-event schema rules (`docs/09_progress_events.md` §Compatibility): additive event ⇒ minor bump only; `slice_complete`/`module_error` must never be dropped — on cancel no `slice_complete` is emitted at all (mirroring the existing fatal-abort sequence, which also ends without `slice_complete`).
- Event-type serialization is `snake_case` (`progress_events.rs:66` `#[serde(rename_all = "snake_case")]`): variant `Cancelled` ⇒ wire string `cancelled`.
- Config keys snake_case: this packet adds no config key (the cancel switch is the CLI flag `--cancel-on-stdin-eof`, kebab-case like every existing flag).

## Code Change Surface

- Selected approach: cooperative-flag cancellation. The CLI owns an `Arc<AtomicBool>`; OS signal handler and optional stdin watcher both set it; the runtime checks it at phase boundaries and per layer; the CLI, not the runtime error type, decides "this failure was a cancel" by re-reading the flag after `run_slice` returns `Err`. This keeps `SliceRunError` (an opaque `String` newtype) untouched.
- Exact symbols:
  - `SliceRunOptions.cancel_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>` — new field (`run.rs:46-80`); `None` ⇒ never cancellable (all existing behaviour preserved).
  - `PipelineConfig.cancel_flag: Option<Arc<AtomicBool>>` — new field (`pipeline.rs:51`); populated from opts at the `PipelineConfig` literal (`run.rs:624`); destructured in `run_pipeline_core` (`pipeline.rs:305`) and checked (Relaxed load) before the PrePass, PerLayer, and PostPass phase starts.
  - `execute_per_layer_with_instrumentation` gains a `cancel_flag: Option<&AtomicBool>` parameter (`layer_executor.rs:189`); inside the `par_iter` closure (`layer_executor.rs:203`) a set flag returns `Err(LayerExecutionError::Cancelled)` instead of calling `execute_single_layer`. All other callers pass `None` (grep-verified caller set is grounded in Step 2; `execute_captured_stages` is untouched).
  - `LayerExecutionError::Cancelled` — new unit variant (`layer_executor.rs:58` enum, plus its `Display` arm at :100); flows through the existing `From<LayerExecutionError> for PipelineError` (`pipeline.rs:130`).
  - `ProgressEventType::Cancelled` — new variant (`progress_events.rs:67` enum); `ProgressEvent::cancelled(slice_id: String, timestamp_ms: u64) -> ProgressEvent` constructor emitting required fields `schema_version`, `event`, `timestamp_ms`, `slice_id` (same builder pattern as `ProgressEvent::phase_start` at :185-196).
  - `PROGRESS_EVENT_SCHEMA_VERSION` / `_INSTRUMENTED` (live `"1.2.0"` at `progress_events.rs:35` — `1.2.0` is already consumed by the `slice_stats` row) — bumped one minor above the live value at implementation time; never take the `slice_stats` row's version (see Data and Contract Notes).
  - `run_slice` (`run.rs:285`): on the pipeline-error path, if `opts.cancel_flag` is set, record `ProgressEvent::cancelled(...)` on `channel.sink` before returning `Err`.
  - `pnp-cli/src/main.rs`: new `--cancel-on-stdin-eof` bool arg on the `Slice` variant (:54-87); slice arm creates the flag, calls `ctrlc::set_handler` (once; ignore `MultipleHandlers`-style double-init in tests via `try` semantics), spawns the stdin watcher thread when the flag arg is set (`std::io::Read::read` loop on `std::io::stdin()` until `Ok(0)`), passes `Some(flag)` in `SliceRunOptions`; on `Err` from `run_slice` with flag set: best-effort `std::fs::remove_file` of the output path, `eprintln` a one-line notice, `std::process::exit(130)`. `pub const EXIT_CODE_CANCELLED: i32 = 130;` in `main.rs`.
  - `crates/pnp-cli/Cargo.toml`: add `ctrlc = "3"`.
- Dependency choice (recorded per plan): `ctrlc` 3.x (MIT OR Apache-2.0). On Windows it registers `SetConsoleCtrlHandler`, covering `CTRL_C_EVENT` and `CTRL_BREAK_EVENT`; on unix it handles SIGINT (the `termination` feature would add SIGTERM/CTRL_CLOSE — not enabled; out of scope). Chosen over hand-rolled `SetConsoleCtrlHandler`/`sigaction` FFI (platform-split unsafe code for no gain) and over `tokio::signal` (no async runtime in pnp-cli). `[FWD]` below pins the CTRL_BREAK verification obligation.
- Rejected alternatives: (a) making `run_slice` return a structured error enum with a `Cancelled` variant — larger API break across every `SliceRunError` consumer for information the CLI already holds in the flag; (b) unconditional stdin-EOF cancel (fork ticket shape) — cancels every `< /dev/null` / CI invocation, so gated behind `--cancel-on-stdin-eof`; (c) checkpoint in `run.rs` per the plan text — falsified: `run.rs:369` is a paint-scan loop; the execution loop is in `layer_executor.rs`; (d) killing rayon workers mid-layer — not cooperative, corrupts instrumentation brackets.

## Files in Scope (read + edit)

Five files exceed the 3-primary guidance; justified: the flag must cross three architectural layers (CLI → run → pipeline → layer executor) plus the event module. Steps keep ≤3 edits each.

- `crates/pnp-cli/src/main.rs` - role: flag creation, handlers, watcher, exit code; expected change: slice-arm additions + new arg.
- `crates/pnp-cli/Cargo.toml` - role: `ctrlc` dependency; expected change: one line.
- `crates/slicer-runtime/src/run.rs` - role: `SliceRunOptions` field + cancelled-event emission + `PipelineConfig` population; expected change: three bounded edits.
- `crates/slicer-runtime/src/pipeline.rs` - role: `PipelineConfig` field + pre-phase checks + parameter pass-through; expected change: bounded edits in `run_pipeline_core`.
- `crates/slicer-runtime/src/layer_executor.rs` - role: `Cancelled` variant + per-layer checkpoint; expected change: enum arm + closure guard + signature.
- `crates/slicer-runtime/src/progress_events.rs` - role: `Cancelled` event type + constructor + version bump; expected change: additive.
- Tests: `crates/slicer-runtime/tests/unit/cancel_flag_tdd.rs` (new) + `tests/unit/main.rs` (one `mod` line); `crates/pnp-cli/tests/slice_cancel_tdd.rs` (new, standalone — pnp-cli tests have no aggregator).

## Read-Only Context

- `crates/slicer-runtime/src/run.rs` - lines `46-91`, `194-300`, `600-650` only - purpose: options struct, fork/channel shape, `PipelineConfig` literal.
- `crates/slicer-runtime/src/layer_executor.rs` - lines `58-140`, `189-240` only - purpose: error enum shape, loop body.
- `crates/slicer-runtime/src/progress_events.rs` - lines `20-70`, `180-230` only - purpose: version constants, event-type enum, constructor pattern.
- `crates/pnp-cli/tests/slice_instrumentation_fork_tdd.rs` - lines `1-110` only - purpose: assert_cmd + fixture + module-dir pattern to clone.
- `docs/09_progress_events.md` - lines `95-168` only - purpose: ordering rules, compatibility, version table.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - no parity surface in this packet; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `.ralph/specs/169-time-estimator-slice-stats/**` - coordinate via the docs/09 version table only; never edit another packet's files
- `crates/slicer-runtime/src/layer_executor.rs` beyond the listed ranges (notably `execute_captured_stages`, :938+) - untouched tap path
- `modules/**`, `crates/slicer-schema/wit/**` - untouched; do not browse

## Expected Sub-Agent Dispatches

- Question: list every in-tree call site of `execute_per_layer_with_instrumentation` and every `SliceRunOptions { ... }` struct literal (file:line each); scope: `crates/`; return: `LOCATIONS`; purpose: Step 1/2 signature-change blast radius.
- Question: does `ctrlc` 3.x's Windows handler fire for `CTRL_BREAK_EVENT` as well as `CTRL_C_EVENT` without the `termination` feature? (check the crate's `windows.rs` handler routine in the vendored registry source or docs.rs); scope: docs.rs/`ctrlc` source via web or registry cache; return: `FACT`; purpose: Step 3 handler wiring; on NO, enable the feature or add a direct `SetConsoleCtrlHandler` shim and record the deviation in this packet's closure notes.
- Question: current live value of `PROGRESS_EVENT_SCHEMA_VERSION` and whether the docs/09 table row after 1.2.0 exists yet; scope: `crates/slicer-runtime/src/progress_events.rs`, `docs/09_progress_events.md`; return: `FACT`; purpose: Step 4 version-bump computation at implementation time.

## Data and Contract Notes

- IR/manifest contracts: none touched.
- WIT boundary: none.
- Progress-event contract: `cancelled` is additive; required fields `schema_version`, `event`, `timestamp_ms`, `slice_id`. Emitted at most once, only on the cancel path, never followed by `slice_complete` (parallel to the documented fatal-abort sequence). Version target is computed from the live constant at implementation time; the live constant is already `1.2.0` (`progress_events.rs:35`), consumed by the `slice_stats` row — a row `docs/09_progress_events.md` attributes to `pinch_n_print_studio` T-096, with packet 169 implementing it. This packet takes the next free minor above the live constant (`1.3.0` as of this grounding; re-verify via the Step-1 FACT dispatch) and never takes the `slice_stats` row's version. ACs deliberately assert the event, not a version literal.
- Process contract (fork-facing): cancel triggers = OS signal, or stdin EOF iff `--cancel-on-stdin-eof`; acknowledgement = `cancelled` JSONL on stderr (when progress events are enabled; with `--no-progress-events` the only signals are the exit code and absent output); exit code 130; `--output` path guaranteed absent.
- Determinism/scheduler: the checkpoint reads the flag with `Ordering::Relaxed`; layers already scheduled may finish (cancel latency ≤ in-flight layer batch). No change to layer ordering or module scheduling when the flag is unset.

## Locked Assumptions and Invariants

- The CLI writes the output file only after `run_slice` returns `Ok` (main.rs:425-433); therefore no partial G-code file can exist on cancel — AC-3 asserts absence, and the `remove_file` is defensive only. Any future streaming writer must revisit this packet's contract.
- Exit code 130 is the sole cancellation exit code and is documented in `slice --help`.
- Stdin-EOF cancel is opt-in forever (flag-gated); flagless behaviour with closed stdin is locked by AC-4.
- `cancel_flag: None` reproduces today's behaviour bit-for-bit (AC-N1 guards the `Some`-but-unset case).

## Risks and Tradeoffs

- `ctrlc` CTRL_BREAK coverage is asserted by the crate's Windows implementation but verified by dispatch before wiring ([FWD] below); the automated e2e test drives the stdin-EOF path (deterministic cross-platform), not real console events — signal delivery itself is manually verified once per platform at the acceptance ceremony and recorded in closure notes.
- Signature change to `execute_per_layer_with_instrumentation` touches its callers; blast radius bounded by the Step-2 LOCATIONS dispatch.
- Shared-file contention with draft packet 169 (`progress_events.rs`, docs/09 table): additive edits in different regions; second-lander rebases the version row.
- A layer stuck inside a WASM module cannot be interrupted; accepted (out of scope) and stated in the fork-facing contract.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2, runtime plumbing across three files)
- Highest-risk dispatch and required return format: call-site/struct-literal blast radius — `LOCATIONS` (≤20 entries); reject prose summaries.

## Open Questions

- `[FWD]` Verify via the Step-3 FACT dispatch that `ctrlc` 3.x (default features) handles `CTRL_BREAK_EVENT` on Windows; if not, enable the crate feature that does or add a minimal `SetConsoleCtrlHandler` shim in `main.rs` — either way the AC surface is unchanged.
- `[FWD]` Exact schema version literal for the `cancelled` row (next free minor above the live constant at implementation time; never `1.2.0`). Implementer computes it per the Data and Contract Notes rule and writes both the constant and the docs/09 row.
