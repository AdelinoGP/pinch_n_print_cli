---
status: implemented
packet: layer-module-preparation_05_preparation-runtime-lifecycle
task_ids:
  - TASK-574
---

# layer-module-preparation_05_preparation-runtime-lifecycle

## Goal

Automatically prepare exactly the eligible frozen module owners from final committed PrePass products, enforce readiness before their ordinary/anchored consumers, and dispose print-private plans after all calls complete.

## Problem Statement

The optional preparation transport is not production activation. A print must freeze the artifact actually selected by existing override precedence, derive eligible work from the shared selection authority, prepare each participating owner once using final committed products, and keep immutable results available through every applicable ordinary and anchored invocation. The runtime must not accidentally prepare an unused module, call a prepared module with no meaningful work, use a different native artifact, expose half-written data, or free owners while calls remain in flight.

## Architecture Constraints

- Lifecycle is a late **PrePass closure**, not another ordinary module stage or barrier. Ordinary `from_config`, instance pool mode, serial-module constraints, arena run-to-completion and existing anchored capability-derived host closure stay authoritative. No synthetic entity commit is converted into module dispatch.
- Avoid widening existing `SliceRunOptions`, `SliceOutcome`, `PipelineConfig`, `PipelineOutput`, `PrepassContext`, `LiveModuleBinding`, `LoadedModule`, `CompiledModuleStatic`, `NativeStageEntry`, `PreparationExportSchema`, IR/config/WIT envelopes or ProgressEvent. New sidecars/private companions and forwarding functions contain this packet's blast radius.
- New `PipelineError::Preparation { module_id: ModuleId, failure: Arc<PreparationFailure> }` preserves failure identity and conveys **required dependency abort**. Existing exhaustive `PipelineError` handling is `fmt::Display` in `crates/slicer-runtime/src/pipeline.rs`; surveyed tests in `tests/integration/pipeline_tdd.rs` have catch-all arms. Add variant and Display together. Cancellation keeps the existing `PipelineError::LayerExecution(LayerExecutionError::Cancelled)` path. Do not repair the documented ordinary nonfatal mapping limitation.
- No existing struct field, config contract, WIT signature or schema-version bump is planned. Adding any would violate the bounded steps: first rederive complete literal sites, version-assertion fallout and cross-boundary SDK/macros/native/WASM/build consumers, split the <=3-edit steps, and reapprove scope. New watched test structs use FRU or justified exhaustive waiver; fixture-only config keys are snake_case. Existing manifest spelling is not mass-renamed.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Data and Contract Notes

No WIT/IR/version changes. All new metadata-only public shapes and private forwarding-driver shape belong to packet.spec. Existing struct literal blast radius is empty by design; new types have no prior sites. `PipelineError` Display is in the same edit step as its new variant. The production schema dependency move, both fixture dev-dependencies and guest manifests are explicitly inventoried; no reliance on dev-feature unification. Config keys remain snake_case.

## Locked Assumptions and Invariants

Same selected artifact for preparation/ordinary calls; owner immutable and print-private; all Ready before required consumers; no prepare/call for prepared no-work; plain empty-target semantics unchanged; final committed inputs only; no stage barrier; selected Layer::AnchoredEvents reads differ from synthetic host commits; print scope outlives both paths; caller joins before Busy-free disposal; original ModuleError preserved; decoder stays module-owned; capture is metadata-only with actual production read accounting; #06 alone owns typed diagnostic snapshots; legacy lightning unchanged.

## Risks and Tradeoffs

Factoring existing pipeline variants must preserve raw config/instrumentation differences. Nondefault/non-region ownership can be lost through fill-only keys. Names/lengths/typed selection diagnostics must never expose opaque bytes or let the host infer plan semantics; ordinary module outputs provide that independent proof. Synthetic commits are lifetime controls, never prepared-consumer evidence. Embedded missing-Ready testing is narrower than activation proof. Aggregate M requires bounded step contracts.
