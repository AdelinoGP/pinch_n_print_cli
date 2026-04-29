# Implementation Plan: 31a_support-geometry-prepass-and-layer-height

## Execution Rules

- One atomic step at a time.
- All steps map to `TASK-163` (architecture foundation portion).
- TDD: write the failing host-side tests before changing the planner.
- All open questions resolved: Q1 (accumulator approach for support layer boundaries), Q2 (intermediate model-resolution layers for `support_top_z_distance`, `global_support_layer_index = u32::MAX` sentinel), Q3 (sentinel = 0.0 for "use model layer height"), Q4 (SupportGeometryIR is Tier-1-only, does not survive into Tier 2).

## Steps

### Step 1: Discovery — read LayerPlanIR and understand support boundary computation

- Task IDs: `TASK-163`
- Objective: Confirm `LayerPlanIR.layers` shape and how to walk it to determine support layer boundaries. Understand the `effective_layer_height` field. Read `docs/02_ir_schemas.md` for `LayerPlanIR` and the `support-planner` module's current manifest to confirm no `SupportGeometryIR` read is declared yet.
- Precondition: Packet 30 closed.
- Postcondition: Engineer can sketch `support_layer_boundaries(layers, support_height_mm) -> Vec<SupportLayerBoundary>` signature and explain how catch-up layers are handled.
- Files expected to change: none.
- Authoritative docs: `docs/02_ir_schemas.md` (LayerPlanIR), `docs/01_system_architecture.md` (Tier 1 prepass).
- OrcaSlicer refs: none (this is a ModularSlicer innovation).
- Verification: `git status` clean.
- Context cost: S
- Exit condition: Engineer can describe support layer boundary computation from memory.

### Step 2: Confirm Q1, Q2, Q3 resolutions (already resolved)

- Task IDs: `TASK-163`
- Objective: Record that Q1 (accumulator approach), Q2 (intermediate model-resolution layers, `global_support_layer_index = u32::MAX` sentinel), Q3 (sentinel = 0.0), and Q4 (Tier-1-only) are already resolved.
- Precondition: Step 1.
- Postcondition: `design.md` Open Questions section shows all resolved.
- Files expected to change: none (documentation only).
- Verification: `grep -n 'Q1 (resolved)' .ralph/specs/31a_support-geometry-prepass-and-layer-height/design.md` returns 1 match; same for Q2, Q3, Q4.
- Context cost: S
- Exit condition: All Q resolutions confirmed in design.md.

### Step 3: Add `SupportGeometryIR` to slicer-ir

- Task IDs: `TASK-163`
- Objective: Add `SupportGeometryKey` struct and `SupportGeometryIR` struct to `crates/slicer-ir/src/slice_ir.rs`. Include `support_layer_height_mm` and `support_top_z_distance_mm` fields on the IR. Re-export from `crates/slicer-ir/src/lib.rs`.
- Precondition: Step 2 (Q1/Q2/Q3/Q4 resolved).
- Postcondition: `cargo build -p slicer-ir 2>&1 | tail -5` exits 0.
- Files expected to change: `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-ir/src/lib.rs`.
- Verification: `grep -nE 'pub struct SupportGeometryIR' crates/slicer-ir/src/slice_ir.rs && grep -nE 'SupportGeometryIR' crates/slicer-ir/src/lib.rs` returns ≥2 matches.
- Context cost: S
- Exit condition: Build green; type reachable.

### Step 5: Add Blackboard slot and accessor for SupportGeometryIR

- Task IDs: `TASK-163`
- Objective: Add `BlackboardPrepassSlot::SupportGeometry` to `crates/slicer-host/src/blackboard.rs`. Add `commit_support_geometry(&self, ir: Arc<SupportGeometryIR>)` and `fn support_geometry(&self) -> Option<Arc<SupportGeometryIR>>`.
- Precondition: Step 3.
- Postcondition: `cargo build -p slicer-host 2>&1 | tail -5` exits 0.
- Files expected to change: `crates/slicer-host/src/blackboard.rs`.
- Verification: `grep -nE 'SupportGeometry' crates/slicer-host/src/blackboard.rs` returns ≥3 matches.
- Context cost: S
- Exit condition: Build green; slot accessible.

### Step 6: Implement `PrePass::SupportGeometry` built-in in prepass.rs

- Task IDs: `TASK-163`
- Objective: In `crates/slicer-host/src/prepass.rs::execute_prepass_with_builtins`, add `PrePass::SupportGeometry` computation:
  - Read `LayerPlanIR` and `MeshIR` from blackboard.
  - Compute support layer boundaries: walk `LayerPlanIR.layers` accumulating `effective_layer_height`; emit a support layer boundary when accumulated >= `support_layer_height_mm`. Catch-up layers count their full `effective_layer_height`.
  - For each support layer boundary Z, run plane-triangle intersection on `MeshIR` to collect polygons at that Z.
  - Union polygons per `(object_id, region_id)` to produce coarse outlines.
  - Add intermediate model-resolution outline layers at every model layer within `support_top_z_distance_mm` of column tops (these use `global_support_layer_index = u32::MAX` sentinel to mark them as model layers, not support layers).
  - Commit `SupportGeometryIR` to blackboard.
- Precondition: Steps 3 and 5.
- Postcondition: A unit test in `crates/slicer-host/src/prepass.rs::tests` (or the new test file) commits a `SupportGeometryIR` for a 2-layer fixture and asserts correct coarse outline count.
- Files expected to change: `crates/slicer-host/src/prepass.rs`.
- Verification: `cargo test -p slicer-host support_geometry 2>&1 | tail -15` passes.
- Context cost: M
- Exit condition: `SupportGeometryIR` reachable from a representative test.

### Step 7: Extend WIT prepass world with support-geometry-view

- Task IDs: `TASK-163`
- Objective: Add `record support-geometry-view-entry`, `record support-geometry-view`, and a `support-geometry: support-geometry-view` parameter to `export run-support-generation` in `wit/world-prepass.wit` (between `region-segmentation` and `output`).
- Precondition: Step 6.
- Postcondition: `cargo build --workspace 2>&1 | tail -10` exits 0 (with the dispatcher Step 9 temporarily passing an empty view).
- Files expected to change: `wit/world-prepass.wit`.
- Verification: `grep -nE 'record support-geometry-view-entry|record support-geometry-view\b|support-geometry: support-geometry-view' wit/world-prepass.wit` returns ≥3 matches.
- Context cost: S
- Exit condition: Workspace build green.

### Step 8: SDK + macro + projector wiring

- Task IDs: `TASK-163`
- Objective: Add `SupportGeometryView`, `SupportGeometryViewEntry` to `crates/slicer-sdk/src/prepass_types.rs`; re-export from `prelude.rs`. Extend `PrepassModule::run_support_generation` signature in `crates/slicer-sdk/src/traits.rs` to accept `&SupportGeometryView`. Extend `crates/slicer-macros/src/lib.rs` to thread the new arg. Add `crates/slicer-host/src/wit_host.rs::project_support_geometry_view` (deterministic ordering by `(global_support_layer_index, object_id, region_id)`) and pass it from the prepass dispatcher.
- Precondition: Step 7.
- Postcondition: All packet 28 + 30 tests still compile and pass.
- Files expected to change: `crates/slicer-sdk/src/prepass_types.rs`, `crates/slicer-sdk/src/prelude.rs`, `crates/slicer-sdk/src/traits.rs`, `crates/slicer-macros/src/lib.rs`, `crates/slicer-host/src/wit_host.rs`.
- Verification: `cargo build --workspace 2>&1 | tail -5` exits 0; packet 30 regression suite green.
- Context cost: M
- Exit condition: Packet 30 regression suite green.

### Step 9: Extend `required_slots` for `PrePass::SupportGeneration`

- Task IDs: `TASK-163`
- Objective: Add `SupportGeometry` to the prerequisite slice in `crates/slicer-host/src/prepass.rs::required_slots` after `RegionMap`. Order: `[SurfaceClassification, LayerPlan, RegionMap, SupportGeometry]`.
- Precondition: Step 8.
- Postcondition: Negative AC `prepass_support_generation_fails_without_support_geometry` will eventually prove the slot is required.
- Files expected to change: `crates/slicer-host/src/prepass.rs`.
- Verification: `grep -nA5 '"PrePass::SupportGeneration"' crates/slicer-host/src/prepass.rs | head -8` shows four slot entries.
- Context cost: S
- Exit condition: Packet 30 regression suite still green.

### Step 10: Add failing TDD tests for the new contract

- Task IDs: `TASK-163`
- Objective: Create `crates/slicer-host/tests/support_geometry_prepass_tdd.rs` with the tests named in the ACs (5 positive + 3 negative). Tests must compile against the SDK and host changes from Steps 4–9.
- Precondition: Steps 8–9.
- Postcondition: Compile clean; positive ACs fail (planner not yet consuming `SupportGeometryView`); negative ACs pass against the host-side prereq enforcement and config-validation paths.
- Files expected to change: `crates/slicer-host/tests/support_geometry_prepass_tdd.rs` (new).
- Verification: `cargo test -p slicer-host --test support_geometry_prepass_tdd -- --test-threads=1 2>&1 | tail -20`.
- Context cost: M
- Exit condition: TDD scaffolding red-green-correct.

### Step 11: Update `support-planner.toml` manifest + config schema

- Task IDs: `TASK-163`
- Objective: Add `"SupportGeometryIR"` to `[ir-access].reads`. Add `support_layer_height_mm` (float, default 0.0, min 0.05, max 1.0) and `support_top_z_distance_mm` (float, default 0.0, min 0.0, max 5.0) to `[config.schema]`.
- Precondition: Steps 9–10.
- Postcondition: Manifest reads list and config schema correct; AC-5 and AC-7 grep tests pass.
- Files expected to change: `modules/core-modules/support-planner/support-planner.toml`.
- Authoritative docs: `docs/03_wit_and_manifest.md`.
- Verification: `grep -nE '"SupportGeometryIR"' modules/core-modules/support-planner/support-planner.toml` returns 1 match; `grep -nE 'support_layer_height_mm|support_top_z_distance_mm' modules/core-modules/support-planner/support-planner.toml` returns 2 matches.
- Context cost: S
- Exit condition: Manifest correct.

### Step 12: Update `tree-support.toml` config schema

- Task IDs: `TASK-163`
- Objective: Add `support_layer_height_mm` and `support_top_z_distance_mm` to `modules/core-modules/tree-support/tree-support.toml [config.schema]` with the same defaults and ranges. The tree-support module does not read `SupportGeometryIR` — it falls back to grid-MST path — but it needs the config keys so the user can set support layer height consistently.
- Precondition: Step 11.
- Postcondition: AC-6 grep passes.
- Files expected to change: `modules/core-modules/tree-support/tree-support.toml`.
- Verification: `grep -nE 'support_layer_height_mm|support_top_z_distance_mm' modules/core-modules/tree-support/tree-support.toml` returns 2 matches.
- Context cost: S
- Exit condition: Tree-support manifest correct.

### Step 13: Implement support interpolation in the planner

- Task IDs: `TASK-163`
- Objective: In `modules/core-modules/support-planner/src/lib.rs`, add support interpolation logic:
  - Read `SupportGeometryView` (via WIT arg from Step 8).
  - In the propagation loop, use coarse `SupportGeometryView` outlines for collision (no per-model-layer collision in this packet — that comes in 31b).
  - When emitting `SupportPlanEntry` records, interpolate to model resolution near column tops: for each support layer, if model's top layers are within `support_top_z_distance_mm`, emit additional entries at model layer Z values with interpolated outline data.
  - Each emitted `SupportPlanEntry` carries model-layer Z and effective height (not support layer indices).
- Precondition: Step 10 (failing tests in place).
- Postcondition: AC-8, AC-9, AC-10 pass. The planner emits at model resolution even when support resolution is coarser.
- Files expected to change: `modules/core-modules/support-planner/src/lib.rs`.
- Context cost: M
- Exit condition: Support interpolation tests pass.

### Step 14: Wire config-validation negative cases

- Task IDs: `TASK-163`
- Objective: Ensure the host's existing config-schema validator rejects `support_layer_height_mm = 0.03` (below min) and `support_top_z_distance_mm = -0.5` (negative) with documented error messages.
- Precondition: Steps 11–12.
- Postcondition: Negative ACs `support_layer_height_below_minimum_rejects_load` and `negative_support_top_z_distance_rejects_load` pass.
- Files expected to change: none (bounds in config schema validated by host at module load time).
- Verification: targeted `cargo test` for both tests.
- Context cost: S
- Exit condition: Config-validation negatives green.

### Step 15: Rebuild all prepass .wasm artifacts

- Task IDs: `TASK-163`
- Objective: Cascade rebuild after the WIT change. Verify `--check` reports every `.wasm` up to date.
- Precondition: Steps 11–14 complete.
- Postcondition: Every `.wasm` rebuilt; `--check` clean.
- Files expected to change: every `modules/core-modules/*/wit-guest/target/` and `.wasm` artifacts.
- Verification: `bash modules/core-modules/build-core-modules.sh 2>&1 | tail -10 && bash modules/core-modules/build-core-modules.sh --check 2>&1 | grep -E 'STALE'` returns no `STALE` matches.
- Context cost: S
- Exit condition: Cascade clean.

### Step 16: Update `docs/07_implementation_status.md`

- Task IDs: `TASK-163`
- Objective: Append the `TASK-163` row from `requirements.md` under Workstream 3.
- Precondition: Step 15.
- Postcondition: `docs/07` contains exactly one row matching `^- \[.\] TASK-163 .*31a_support-geometry-prepass-and-layer-height`.
- Files expected to change: `docs/07_implementation_status.md`.
- Verification: AC-12's grep.
- Context cost: S
- Exit condition: Backlog updated.

### Step 17: Packet completion gate

- Task IDs: `TASK-163`
- Objective: Run the focused matrix.
- Precondition: Steps 1–16.
- Postcondition: All gate commands exit 0.
- Files expected to change: none.
- Verification:
  ```
  cargo test -p slicer-host --test prepass_support_generation_tdd -- --test-threads=1 --nocapture 2>&1 | tail -10
  cargo test -p slicer-host --test prepass_support_generation_layer_plan_tdd -- --test-threads=1 --nocapture 2>&1 | tail -10
  cargo test -p slicer-host --test support_geometry_prepass_tdd -- --test-threads=1 --nocapture 2>&1 | tail -10
  cargo test -p slicer-host --test live_support_generation_tdd -- --test-threads=1 --nocapture 2>&1 | tail -10
  cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_with_support_enabled -- --test-threads=1 --nocapture 2>&1 | tail -10
  cargo test -p support-planner --lib 2>&1 | tail -10
  cargo build --workspace 2>&1 | tail -5
  cargo clippy --workspace -- -D warnings 2>&1 | tail -5
  ```
- Context cost: S
- Exit condition: All commands exit 0; packet ready for `spec-review`.

## Packet Completion Gate

- All steps complete.
- Every step exit condition met.
- Every pipe-suffixed AC command from `packet.spec.md` re-run and green (12 ACs + 3 negatives = 15 commands).
- All three open questions (Q1, Q2, Q3) resolved before activation.
- `docs/07_implementation_status.md` updated.
- Packets 28, 26, 30 still `status: implemented`.

## Acceptance Ceremony

- Re-run every `|`-suffixed verification command from `packet.spec.md` and confirm green.
- Confirm all packet-level verification commands are green.
- Confirm no other active packet before marking `status: active` (per `.ralph/specs/README.md`).
- Record that this packet is the architectural foundation for packet `31b` (which adds avoidance/collision, radius tapering, wall-count, raft, interface densification).