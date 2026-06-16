---
status: implemented
packet: 73_support-geometry-normalization
task_ids:
  - TASK-163c
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 73_support-geometry-normalization

## Goal

Give the `run-support-geometry` WIT export the same shape as its sibling prepass stages — `config: config-view` in, a `support-geometry-output` resource, `result<_, module-error>` out — so the tree-support planner finally receives real config and its fatals propagate instead of being swallowed.

## Scope Boundaries

This packet normalizes one WIT export and the host/macro glue that feeds it. The SDK trait (`PrepassModule::run_support_geometry`) and the `support-planner` module body are **already** in the normalized shape and do not change; the defect lives only at the WIT boundary, where the macro injects an empty `ConfigView` and discards the module's `Result`. Fixing it is a deliberate behavior change — support config takes effect and planner fatals surface — so re-baselining affected fixtures and adding a regression test are in scope. It depends on packet 72 (it edits the relocated `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit`).

## Prerequisites and Blockers

- Depends on: `72_wit-single-source-unification` (the canonical `world-prepass.wit` must exist at `crates/slicer-schema/wit/`; the shared `module-error` is reused here).
- Unblocks: nothing downstream in this plan.
- Activation blockers: packet 72 must be `implemented` (this packet edits a file 72 creates). If 72 is not yet done, keep this `draft`.

## Acceptance Criteria

- **AC-1. Given** `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit`, **when** inspecting the `run-support-geometry` export, **then** it accepts a `config: config-view` parameter, writes through a `resource support-geometry-output` exposing `push-support-plan-entry`, and returns `result<_, module-error>` (no bare record return). | `bash -c 'rg -U -q "run-support-geometry: func\([^)]*config: config-view[^)]*\) -> result<_, module-error>" crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit && rg -q "resource support-geometry-output" crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit; echo EXIT=$?'`
- **AC-2. Given** a slice configured with `support_raft_layers = 2` and one supported region, **when** the support-geometry stage runs through the host→guest dispatch, **then** the committed `SupportPlanIR` contains raft entries with negative `global_layer_index` values `-1` and `-2` (config now reaches the guest). | `cargo test -p slicer-runtime --test support_geometry_config_normalization_tdd -- raft_layers_config_is_honored`
- **AC-3. Given** `crates/slicer-macros/src/lib.rs`, **when** grepping the support-geometry glue, **then** the empty-`ConfigView` injection and the `let _ = out;` error-swallow are gone and the support arm propagates via `__slicer_error_out`. | `bash -c '! rg -q "run-support-geometry has no config-view parameter" crates/slicer-macros/src/lib.rs && ! rg -q "Ignore error from run_support_geometry" crates/slicer-macros/src/lib.rs; echo EXIT=$?'`
- **AC-4. Given** `crates/slicer-runtime/src/wit_host.rs` and `dispatch.rs`, **when** grepping, **then** the record-stash `push_support_geometry_result` is removed and the dispatch arm passes a config handle to `call_run_support_geometry`. | `bash -c '! rg -q "push_support_geometry_result" crates/slicer-runtime/src/wit_host.rs crates/slicer-runtime/src/dispatch.rs; echo EXIT=$?'`
- **AC-5. Given** `crates/slicer-sdk/src/traits.rs` and `modules/core-modules/support-planner/src/lib.rs`, **when** inspecting `run_support_geometry`, **then** both keep the existing signature (`output: &mut SupportGeometryOutput`, `config: &ConfigView`, `-> Result<(), ModuleError>`) — unchanged by this packet. | `bash -c 'rg -q "output: &mut SupportGeometryOutput" crates/slicer-sdk/src/traits.rs && rg -q "fn run_support_geometry" modules/core-modules/support-planner/src/lib.rs; echo EXIT=$?'`
- **AC-6. Given** freshly built guests, **when** the existing support-geometry integration test runs with default config (support enabled), **then** support geometry is still committed (no regression to the enabled path). | `cargo test -p slicer-runtime --test prepass_support_geometry_tdd`
- **AC-7. Given** the edited WIT and glue, **when** the freshness check runs after a rebuild, **then** it reports no `STALE:` guests. | `cargo xtask build-guests --check`

## Negative Test Cases

- **AC-N1. Given** a slice configured with `support_enabled = false`, **when** the support-geometry stage runs through dispatch, **then** the committed `SupportPlanIR` has zero entries (config now disables the planner; previously the empty `ConfigView` forced the `enabled = true` default and it always ran). | `cargo test -p slicer-runtime --test support_geometry_config_normalization_tdd -- support_disabled_emits_no_plan`
- **AC-N2. Given** a support-geometry run whose layer-plan view is empty, **when** the planner returns `Err(ModuleError::fatal(1, "empty layer-plan-view"))`, **then** dispatch surfaces a `DispatchError` (the error is no longer swallowed). | `cargo test -p slicer-runtime --test support_geometry_config_normalization_tdd -- planner_fatal_surfaces_as_dispatch_error`

## Verification

Gate commands only (full matrix in `requirements.md` §Verification Commands):

- `cargo check --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo test -p slicer-runtime --test support_geometry_config_normalization_tdd`

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — the prepass world contract. **Delegate a SUMMARY** of the prepass-world / `run-support-geometry` section.
- `docs/02_ir_schemas.md` — `SupportPlanIR` / `support-plan-entry` field names. Delegate; load only the `SupportPlanIR` section.
- `docs/04_host_scheduler.md` — prepass dispatch + error→`DispatchError` path. Delegate; consult for AC-N2 only.

## Doc Impact Statement (Required)

This packet changes a WIT export contract, so `none` is not eligible.

- `docs/03_wit_and_manifest.md` §"PrePass world — run-support-geometry" (new `config-view` param, output resource, `result<_, module-error>`) — `rg -q 'run-support-geometry' docs/03_wit_and_manifest.md`
- `docs/07_implementation_status.md` — note the WIT-boundary config/error fix under the support-geometry line (distinct from TASK-166's RegionMapIR-layer work) — `rg -q 'run-support-geometry' docs/07_implementation_status.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

## Deviations

- **[Step 1 / AC-1 path]** Specified edits to `crates/slicer-schema/wit/world-prepass.wit`; implemented at `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit`, with the stale path corrected across all 5 packet docs. Reason: packet 72's nested-package umbrella moved the file; the authored path predated 72's final structure.
- **[Step 3 / push validation — audit concern investigated, RETAINED]** A spec-audit flagged the empty-`object_id`/`region_id` rejection in `push_support_plan_entry` as unrequested and potentially harmful (fatal `code: 11`). Verification against the named mirror target disproved that: `seam-planning-output` and `mesh-analysis-output` carry the identical validation, with the stated reason that an empty id *"would corrupt the RegionKey construction in the harvest helper"* — which `harvest_support_plan_ir` shares. Rejecting an empty id fails loud rather than silently mis-keying support geometry, so the validation is the correct, consistent prepass pattern and is **retained** (it correctly mirrors `seam-planning`, as the packet specified — not a deviation). The rejection branch lacks dedicated test coverage, a pre-existing gap shared by all prepass output builders; out of scope here.
- **[Step 4 scope — packet-72 remediation]** Also edited `tests/{live_seam_path,pipeline,z_envelope_contract}_tdd.rs`. These are **packet-72 generated-path fallout** (`world_layer::geometry`→`types::geometry`, `world_layer::ir_handles`→`ir_handles::ir_handles`) that left those test targets uncompilable; `cargo check --workspace` without `--all-targets` hid it from packet 72's gate. Path-only, no logic change. Re-attributed to packet 72's Deviations; gate hardened to `--all-targets`.
- **[task attribution]** Packet retargeted from `TASK-166` (RegionMapIR config) to `TASK-163c` (support-geometry cluster) — the WIT-boundary fix is topically support-geometry, not RegionMapIR. `task_ids`, `task-map.md`, and the `requirements.md` Problem Statement updated to match the `docs/07` entry.
