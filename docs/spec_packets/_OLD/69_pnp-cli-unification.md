---
status: implemented
packet: pnp-cli-unification
task_ids:
  - TASK-213
---

# 69_pnp-cli-unification

## Goal

Replace the `slicer-cli` and `slicer-host` binaries with a single `pnp_cli` binary by renaming `crates/slicer-host` → `crates/slicer-runtime` (library only, no binary target), extracting a `slicer_runtime::run::run_slice()` library entry point, externalising the 8 synthetic host built-ins onto a `Producer` trait that flows through both the DAG validator and `dag_cli`, consolidating manifest-validation constants into `slicer-schema`, and creating a new `crates/pnp-cli/` binary crate that owns the noun-namespaced verb tree (`slice`, `module new|diagnose|config-schema`, `mesh repair|decimate|import`, `dag stages|stage|depends|claims`).

## Problem Statement

The workspace currently ships two binaries with overlapping surface area: `slicer-cli` (binary `slicer`, in `cli/slicer-cli/`) and `slicer-host` (binary `slicer-host`, in `crates/slicer-host/`). `slicer-cli run` shells out to `slicer-host run` after a partial duplicate of `slicer-host diagnose`'s manifest validation; `slicer-cli validate` ships its own copy of `VALID_STAGES`/`SUPPORTED_WIT_WORLDS`/`VALID_CONFIG_TYPES`/`RECOGNIZED_CLAIMS`/`VALID_SEVERITIES` parallel to `slicer-host::execution_plan::STAGE_ORDER`; `slicer-host::main()`'s `HostCommands::Run` arm is a 456-LOC god-function that inlines config parsing, mesh loading, an 8-row synthetic `LoadedModule` block to humour the DAG validator about host built-ins, and a 4-way instrumentation fork. The `crates/slicer-host` library has zero external importers (verified by `grep` across the workspace), so its name is a misnomer — it's the runtime library, not specifically "the host". The `dag_cli::run_dag_*` functions accept `&[LoadedModule]` and never see the synthetic host built-ins, so `slicer-host dag claims` lies by omission about who writes `MeshIR`/`SliceIR`/etc. This packet collapses both binaries into one (`pnp_cli`), renames the runtime crate honestly, externalises host built-ins behind a `Producer` trait that flows through both the DAG validator AND `dag_cli`, and removes the duplicate validator without leaving a transitional alias.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `./modules/core-modules/build-core-modules.sh --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- The crate rename `slicer-host` → `slicer-runtime` must land atomically (step 1). A partial rename leaves the workspace uncompilable. Use `cargo check --workspace` as the gate before proceeding past step 1.
- The library API extracted in step 4 (`run_slice`) MUST be byte-deterministic with the pre-refactor `HostCommands::Run` arm given identical inputs. The 4-way instrumentation fork is moved INSIDE `run_slice` as a private helper; the call site composition (Report-only / Progress-only / Composite / Noop) is unchanged.
- The `Producer` trait surface is the smallest projection of `LoadedModule` that `validate_startup_dag`, `build_intra_stage_dag`, and the 4 `dag_cli::run_dag_*` functions all need to consume. Adding fields to the trait surface that aren't read by any of these is forbidden — it expands the seam without callers.
- Manifest TOML schema, IR field paths, WIT world strings, scheduler claim semantics, and `wasm-tools component new` invocation shape are NOT touched. `slicer-schema`'s validator constants are pure values relocated from `cli/slicer-cli/src/cmd_validate.rs`; the values must match exactly.

## Data and Contract Notes

- **IR contracts**: unchanged. The 8 synthetic-row IR write paths (`MeshIR`, `SurfaceClassificationIR`, `RegionMapIR`, `SliceIR`, `SupportGeometryIR`, `PaintRegionIR`, `GCodeIR`) are preserved verbatim as the `ir_writes` field on the corresponding `BuiltinProducer` constants — these are observable in `pnp_cli dag claims` JSON.
- **WIT contracts**: unchanged. No `wit/**/*.wit` file is edited in this packet.
- **Manifest schema**: unchanged. The validator constants relocate but the values are identical. `slicer-schema`'s public API gains the constants but no other shape change.
- **Scheduler semantics**: unchanged. The DAG validator's input projection narrows (`LoadedModule` → `&dyn Producer`), but every field it actually reads is preserved, including the IR-schema-version compat fields used by `IrVersionCompatibility` pass.
- **Determinism**: `run_slice()` is byte-deterministic with the pre-refactor `HostCommands::Run` arm for identical `SliceRunOptions`. AC-3's test asserts non-empty output; the AC-2 smoke against benchy.stl provides the regression signal. If a downstream snapshot diff appears, the rename or the synthetic-row move dropped a field the validator reads.

## Locked Assumptions and Invariants

- `crates/slicer-host` has zero external library importers in the workspace (verified by `grep -rln 'slicer-host =\|use slicer_host::' --include='Cargo.toml' --include='*.rs'` returning only internal hits). The rename does not need to ripple outside the crate.
- The `Producer` trait's smallest valid surface is exactly `(id: &str, stage: &str, ir_writes: &[String], ir_reads: &[String], claims_holds: &[String], claims_requires: &[String], requires_modules: &[String], min_ir_schema: SemVer, max_ir_schema: SemVer)`. Any caller needing more is using a `LoadedModule`-specific field that doesn't apply to host built-ins (e.g., `wasm_path`) and must not be part of this trait.
- The 4-way instrumentation fork composition (`Composite(progress, report)` / `report-only` / `progress-only` / `noop`) is invariant — `run_slice` reproduces it inside its body. If the implementer notices a 5th case during the move, they have introduced a regression — stop and reconcile.
- `pnp_cli`'s clap definitions and dispatcher logic depend on `slicer-runtime` exporting `cli::HostCli` / `HostCommands` / `DagSubcommand` / `OutputFormat` / `HostRunOptions` (renamed `SliceRunOptions`) verbatim. The runtime crate must keep these public.
- `.cargo/config.toml` aliases generated by `pnp_cli module new` are scoped to the new module's directory; they do not interact with the workspace's own `.cargo/config.toml` (which is empty today — verified by `Glob`).

## Risks and Tradeoffs

- **Rename atomicity**: a partial rename in step 1 leaves the workspace uncompilable. Mitigation: run `cargo check --workspace` as the step-1 gate (delegated); do not progress past step 1 until green.
- **Producer trait shape drift**: the trait projection might be slightly off (a field needed by `validate_startup_dag::IrVersionCompatibility` that I missed). Mitigation: AC-3's `run_slice_api_tdd` exercises the validator end-to-end against benchy modules; a mis-projection surfaces as a test failure.
- **dag_cli signature change blast radius**: the 4 `run_dag_*` signature changes touch existing tests in `dag_cli.rs:400-600`. Mitigation: each test gets updated alongside the signature change in step 3; the test count stays the same.
- **CI gate ordering**: the doc/CI sweep (step 10) updates `.github/workflows/ci.yml`. If the implementer pushes the PR before step 10 lands, CI fails with `cargo test -p slicer-host: package not found`. Mitigation: step 11 (the gate) runs locally before push; step 10 lands before step 11.
- **Living-skill sweep scope creep**: the LOCATIONS dispatch in step 10 might return many matches in skill files. Mitigation: dispatch with a max-line bound (`head_limit: 50`) and triage; only edit files that emit CLI invocations.
