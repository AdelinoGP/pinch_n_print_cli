# Implementation Plan: 30_support-planner-prepass-wit-plumbing

## Execution Rules

- One atomic step at a time.
- Each step maps to `TASK-162`.
- TDD: write the failing host-side tests in Step 8 before changing the planner in Step 9.

## Steps

### Step 1: Discovery — read the WIT projection precedent

- Task IDs: `TASK-162`
- Objective: Confirm the existing host-side prepass projectors (e.g. how `MeshObjectView` is built from `MeshIR` for `run-mesh-segmentation` and `run-seam-planning`) and the SDK type pattern they follow. Confirm that `RegionMapIR` is committed by `PrePass::RegionMapping` before any user PrePass stage that depends on it.
- Precondition: None.
- Postcondition: Engineer can name (a) the file and function that builds `MeshObjectView` from a host IR, (b) the SDK pattern for re-exporting prepass types, (c) the `RegionMapIR.entries` shape and key sort order.
- Files expected to change: none.
- Authoritative docs: `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md`, `docs/05_module_sdk.md`.
- OrcaSlicer refs: none.
- Expected sub-agent dispatches: none (read-only discovery; planner owns).
- Context cost: S
- Verification: `git status` clean for packet-relevant files.
- Exit condition: Engineer can sketch `project_layer_plan_view` and `project_region_segmentation_view` signatures from memory and name the existing projector helper they mirror.

### Step 2: Add SDK types and re-exports

- Task IDs: `TASK-162`
- Objective: Add `LayerPlanView`, `LayerPlanViewEntry`, `RegionSegmentationView`, `RegionSegmentationViewEntry` to `crates/slicer-sdk/src/prepass_types.rs`. Re-export from `crates/slicer-sdk/src/prelude.rs`.
- Precondition: Step 1.
- Postcondition: Types defined with `Debug + Clone + PartialEq + Serialize + Deserialize`. `cargo build -p slicer-sdk` succeeds.
- Files expected to change: `crates/slicer-sdk/src/prepass_types.rs`, `crates/slicer-sdk/src/prelude.rs`.
- Authoritative docs: `docs/02_ir_schemas.md`, `docs/05_module_sdk.md`.
- Expected sub-agent dispatches: none.
- Context cost: M
- Verification: `grep -nE 'pub struct LayerPlanView\b|pub struct RegionSegmentationView\b' crates/slicer-sdk/src/prepass_types.rs` returns 2 matches; `cargo build -p slicer-sdk 2>&1 | tail -5` exits 0.
- Exit condition: Build green; types reachable from `slicer_sdk::prelude::*`.

### Step 3: Extend the WIT prepass world

- Task IDs: `TASK-162`
- Objective: Add the four new records to `wit/world-prepass.wit` (alongside `support-plan-entry`) and extend `export run-support-generation` parameters to `(objects, layer-plan, region-segmentation, output, config)`.
- Precondition: Step 2.
- Postcondition: `wit/world-prepass.wit` contains the new records; `cargo build --workspace 2>&1 | tail -10` exits 0 (host + macro consume the new world cleanly even before guest changes — the dispatcher Step 5 may temporarily be a `todo!()`).
- Files expected to change: `wit/world-prepass.wit`.
- Authoritative docs: `docs/03_wit_and_manifest.md`.
- Expected sub-agent dispatches: none.
- Context cost: M
- Verification: `grep -nE 'record layer-plan-view-entry|record layer-plan-view\b|record region-segmentation-view-entry|record region-segmentation-view\b|layer-plan: layer-plan-view|region-segmentation: region-segmentation-view' wit/world-prepass.wit` returns ≥6 matches.
- Exit condition: Workspace build green with the extended WIT.

### Step 4: Extend the SDK trait and the `#[slicer_module]` macro

- Task IDs: `TASK-162`
- Objective: Update `PrepassModule::run_support_generation` to accept `&LayerPlanView, &RegionSegmentationView`. Default body still returns `Err(ModuleError::unimplemented(...))`. Update `crates/slicer-macros/src/lib.rs` to thread the two new args from the generated WIT shim into the trait method when `stage.id == "PrePass::SupportGeneration"`.
- Precondition: Step 3.
- Postcondition: Other prepass modules (`seam-planner-default`, `mesh-segmentation`, etc.) compile unchanged. `cargo build --workspace` succeeds.
- Files expected to change: `crates/slicer-sdk/src/traits.rs`, `crates/slicer-macros/src/lib.rs`.
- Authoritative docs: `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md`.
- Expected sub-agent dispatches: none.
- Context cost: M
- Verification: `grep -nA8 'fn run_support_generation' crates/slicer-sdk/src/traits.rs | head -12` shows the new 5-parameter signature; `cargo build --workspace 2>&1 | tail -5` exits 0.
- Exit condition: Build green; macro routes the new args.

### Step 5: Implement host-side projectors and dispatcher wiring

- Task IDs: `TASK-162`
- Objective: Add `project_layer_plan_view(layer_plan_ir: &LayerPlanIR) -> wit_host::LayerPlanView` and `project_region_segmentation_view(region_map_ir: &RegionMapIR) -> wit_host::RegionSegmentationView` in `crates/slicer-host/src/wit_host.rs`. Both projectors sort their outputs deterministically: `LayerPlanView.layers` by `global_layer_index ASC`; `RegionSegmentationView.entries` by `(global_layer_index ASC, object_id ASC)` with each entry's `region_ids` sorted ASC. Wire both into the prepass dispatcher's `run-support-generation` arm (call the projectors before invoking the export). Region IDs are emitted as canonical decimal strings.
- Precondition: Step 4.
- Postcondition: `WasmRuntimeDispatcher` invokes the support-generation export with the new args. The packet 28 tests in `prepass_support_generation_tdd.rs` continue passing because they use single-layer-height + single-region fixtures and the projector hands those through unchanged.
- Files expected to change: `crates/slicer-host/src/wit_host.rs` (and any small follow-on in `crates/slicer-host/src/dispatch.rs` for exhaustive matches).
- Authoritative docs: `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md`.
- Expected sub-agent dispatches: none.
- Context cost: M
- Verification: `cargo test -p slicer-host --test prepass_support_generation_tdd -- --test-threads=1 2>&1 | tail -10` shows all 7 packet-28 tests passing.
- Exit condition: Packet-28 regression suite green; new dispatcher path covered by Step 8 tests next.

### Step 6: Extend `required_slots` for `PrePass::SupportGeneration`

- Task IDs: `TASK-162`
- Objective: Add `BlackboardPrepassSlot::RegionMap` to the prerequisite slice in `crates/slicer-host/src/prepass.rs::required_slots`. Order: `[SurfaceClassification, LayerPlan, RegionMap]`.
- Precondition: Step 5.
- Postcondition: Negative AC `prepass_support_generation_fails_without_region_map` will eventually prove the slot is required. Packet 28's `prepass_support_generation_fails_without_layer_plan` still passes because `LayerPlan` precedes `RegionMap` in the slice.
- Files expected to change: `crates/slicer-host/src/prepass.rs`.
- Authoritative docs: `docs/04_host_scheduler.md`.
- Expected sub-agent dispatches: none.
- Context cost: S
- Verification: `grep -nA4 '"PrePass::SupportGeneration"' crates/slicer-host/src/prepass.rs | head -8` shows three slot entries in the documented order.
- Exit condition: Packet-28 regression suite still green.

### Step 7: Update `support-planner` manifest

- Task IDs: `TASK-162`
- Objective: Set `[ir-access].reads = ["MeshIR", "SurfaceClassificationIR", "LayerPlanIR", "RegionMapIR", "PaintRegionIR"]` (in that order). Remove the v1 layer-height-agnostic comment block above the list.
- Precondition: Step 6.
- Postcondition: Manifest declares the runtime reads the planner will exercise after Step 9.
- Files expected to change: `modules/core-modules/support-planner/support-planner.toml`.
- Authoritative docs: `docs/03_wit_and_manifest.md`.
- Expected sub-agent dispatches: none.
- Context cost: S
- Verification: `grep -nE 'reads  = \["MeshIR", "SurfaceClassificationIR", "LayerPlanIR", "RegionMapIR", "PaintRegionIR"\]' modules/core-modules/support-planner/support-planner.toml` returns 1 match; `! grep -n 'layer-height-agnostic' modules/core-modules/support-planner/support-planner.toml`.
- Exit condition: Manifest reads list and comment scrubbed as specified.

### Step 8: Add failing TDD tests for the new contract

- Task IDs: `TASK-162`
- Objective: Create `crates/slicer-host/tests/prepass_support_generation_layer_plan_tdd.rs` with the five tests listed in the packet ACs. Tests must compile against the SDK changes from Steps 2–4 and the host changes from Steps 5–6, and they must fail because the planner stub still uses the v1 derivation. Also add `planner_consuming_tier::tree_support_live_dispatch_finds_branches_for_real_region_id` to `crates/slicer-host/tests/live_support_generation_tdd.rs` Section C.
- Precondition: Steps 5–7 complete.
- Postcondition: Compile-clean; the variable-height and multi-region tests fail; the missing-RegionMap negative passes (host already enforces it post-Step 6); empty-region-map passes if the planner's behaviour matches; empty-layer-plan-view fails until Step 9.
- Files expected to change: `crates/slicer-host/tests/prepass_support_generation_layer_plan_tdd.rs` (new), `crates/slicer-host/tests/live_support_generation_tdd.rs` (extension).
- Authoritative docs: `docs/02_ir_schemas.md`, `docs/04_host_scheduler.md`.
- Expected sub-agent dispatches: none.
- Context cost: M
- Verification: `cargo test -p slicer-host --test prepass_support_generation_layer_plan_tdd -- --test-threads=1 2>&1 | tail -20` runs; `planner_walks_real_layer_plan_with_variable_layer_heights`, `planner_emits_one_entry_per_region_in_region_map`, and `host_projector_orders_region_segmentation_deterministically` fail; `prepass_support_generation_fails_without_region_map` passes.
- Exit condition: TDD scaffolding green/red as expected; no implementation yet.

### Step 9: Update `support-planner` to consume the new views

- Task IDs: `TASK-162`
- Objective: In `modules/core-modules/support-planner/src/lib.rs`:
  - Drop module-level v1 doc bullets for layer-height-agnostic and single-region.
  - Remove the `DEFAULT_LAYER_HEIGHT_MM` constant.
  - Change the `run_support_generation` signature to accept `&LayerPlanView, &RegionSegmentationView`.
  - Replace the bounds-derived `num_layers`/`layer_height` block in `plan_for_object` with `layer_plan.layers` indexing. Per-layer Z and effective height come from `layer_plan_view`.
  - Replace the hard-coded `region_id: "0".to_string()` site with a loop over the `region_ids` for the current `(layer_index, object_id)` from `region_segmentation_view`. Skip the object entirely when no entry exists for it.
  - Add an early `return Err(ModuleError::fatal(_, "empty layer-plan-view"))` when `layer_plan_view.layers.is_empty()`.
- Precondition: Step 8 (failing tests in place).
- Postcondition: All Step 8 tests pass. Packet 28's tests continue passing (single-layer-height + single-region fixtures still work because the planner now reads the same Z values the projector hands it).
- Files expected to change: `modules/core-modules/support-planner/src/lib.rs`, `modules/core-modules/support-planner/wit-guest/src/lib.rs` (regenerate guest re-export).
- Authoritative docs: `docs/05_module_sdk.md`.
- Expected sub-agent dispatches: none.
- Context cost: M
- Verification: `cargo test -p slicer-host --test prepass_support_generation_layer_plan_tdd -- --test-threads=1 2>&1 | tail -20` reports all five tests passing; `cargo test -p support-planner --lib 2>&1 | tail -10` passes including `empty_layer_plan_view_returns_fatal_module_error`.
- Exit condition: All packet-30 prepass-stage ACs green.

### Step 10: Wire the live tree-support multi-region test

- Task IDs: `TASK-162`
- Objective: Implement `planner_consuming_tier::tree_support_live_dispatch_finds_branches_for_real_region_id` in `live_support_generation_tdd.rs` so it dispatches `tree-support` against a `LayerView` whose `region_id() == 42` with a `SupportPlanIR` carrying entries for region IDs `7` and `42`. Assert non-empty `support_paths` and byte-identical match against the `region_id == 42` plan entry.
- Precondition: Step 9.
- Postcondition: The new test passes. All other live-dispatch tests continue passing.
- Files expected to change: `crates/slicer-host/tests/live_support_generation_tdd.rs`.
- Authoritative docs: `docs/04_host_scheduler.md`.
- Expected sub-agent dispatches: none.
- Context cost: M
- Verification: `cargo test -p slicer-host --test live_support_generation_tdd -- --test-threads=1 2>&1 | tail -20` reports the new test passing alongside the existing 13 (14 total).
- Exit condition: Live dispatch suite green.

### Step 11: Rebuild every prepass `.wasm`

- Task IDs: `TASK-162`
- Objective: Rebuild the entire prepass module set since the WIT package binding shape changed. Verify `--check` reports every artifact up to date.
- Precondition: Steps 9–10 complete.
- Postcondition: Every `.wasm` under `modules/core-modules/*/` rebuilt without errors. `bash modules/core-modules/build-core-modules.sh --check` reports no `STALE` lines.
- Files expected to change: every `modules/core-modules/*/wit-guest/target/` and the `.wasm` artifacts (no source change for non-support-planner modules).
- Authoritative docs: `docs/03_wit_and_manifest.md`.
- Expected sub-agent dispatches: none.
- Context cost: S
- Verification: `bash modules/core-modules/build-core-modules.sh 2>&1 | tail -20` exits 0; `bash modules/core-modules/build-core-modules.sh --check 2>&1 | grep -E 'STALE'` returns 0 matches.
- Exit condition: Cascade rebuild green.

### Step 12: Update `docs/07_implementation_status.md`

- Task IDs: `TASK-162`
- Objective: Append the `TASK-162` row from `requirements.md` under Workstream 3.
- Precondition: Step 11.
- Postcondition: `docs/07_implementation_status.md` contains exactly one row matching `^- \[.\] TASK-162 ` with body mentioning `LayerPlanIR`, `RegionMapIR`, and the slug `30_support-planner-prepass-wit-plumbing`.
- Files expected to change: `docs/07_implementation_status.md`.
- Authoritative docs: none.
- Expected sub-agent dispatches: none.
- Context cost: S
- Verification: `grep -nE '^- \[.\] TASK-162 .*LayerPlanIR.*RegionMapIR.*30_support-planner-prepass-wit-plumbing' docs/07_implementation_status.md` returns 1 match.
- Exit condition: Backlog reflects the packet's deliverable.

### Step 13: Packet completion gate

- Task IDs: `TASK-162`
- Objective: Run the focused test matrix and workspace checks for packet 30.
- Precondition: Steps 1–12 complete.
- Postcondition: All gate commands exit 0.
- Files expected to change: none.
- Authoritative docs: `docs/11_operational_governance_and_acceptance_gate.md`, `docs/12_architecture_gate_metrics.md`.
- Expected sub-agent dispatches: none.
- Context cost: S
- Verification:
  ```
  cargo test -p slicer-host --test prepass_support_generation_tdd -- --test-threads=1 --nocapture 2>&1 | tail -10
  cargo test -p slicer-host --test prepass_support_generation_layer_plan_tdd -- --test-threads=1 --nocapture 2>&1 | tail -10
  cargo test -p slicer-host --test live_support_generation_tdd -- --test-threads=1 --nocapture 2>&1 | tail -10
  cargo test -p support-planner --lib 2>&1 | tail -10
  bash modules/core-modules/build-core-modules.sh --check 2>&1 | tail -10
  cargo build --workspace 2>&1 | tail -5
  cargo clippy --workspace -- -D warnings 2>&1 | tail -5
  ```
- Exit condition: All seven commands exit 0; packet ready for `spec-review`.

## Packet Completion Gate

- All steps complete.
- Every step exit condition met.
- Every pipe-suffixed acceptance criterion command from `packet.spec.md` re-run and green.
- `docs/07_implementation_status.md` updated.
- Packet 28 still `status: implemented` (this packet did not modify packet 28's files).
- `packet.spec.md` ready to move to `status: implemented` after `spec-review` approves.

## Acceptance Ceremony

- Re-run every `|`-suffixed verification command from `packet.spec.md` (10 ACs + 3 negative ACs = 13 commands) and confirm green.
- Confirm packet-level verification commands are green.
- Record any remaining packet-local risk (e.g., known-incomplete branch geometry across multi-region objects — covered explicitly by packet `31b`'s scope).
