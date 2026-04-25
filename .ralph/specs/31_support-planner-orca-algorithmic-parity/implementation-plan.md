# Implementation Plan: 31_support-planner-orca-algorithmic-parity

## Execution Rules

- One atomic step at a time.
- All steps map to `TASK-163`.
- Steps 4, 8, 11, and 14 cannot start until their corresponding open question (`design.md` Q3, Q4, Q2, Q5) is resolved.
- TDD where applicable: tests precede implementation in Steps 7, 9, 12, 14.

## Steps

### Step 1: Discovery — read OrcaSlicer references and packet-30 projector pattern

- Task IDs: `TASK-163`
- Objective: Read `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` lines 720–800 (radius taper), 1460–1700 (interface), 1913 (`support_interface_top_layers` use site), 2625–2860 (`drop_nodes` propagation including line 2634 `max_move_distance` and line 2632 `wall_count`). Read `TreeSupport.hpp` for `SupportNode` and `TreeSupportData` shapes. Re-read `.ralph/specs/30_support-planner-prepass-wit-plumbing/design.md` for the host projector pattern.
- Precondition: Packet 30 closed.
- Postcondition: Engineer can summarize each algorithmic limit's OrcaSlicer reference and sketch the eager outline cache approach.
- Files expected to change: none.
- Authoritative docs: `docs/01`–`docs/05`, `docs/08`, `docs/09`.
- OrcaSlicer refs: as above.
- Verification: `git status` clean.
- Exit condition: Engineer can name the four config keys, the radius-taper formula, the wall-count multiplier, and the raft Z convention from memory.

### Step 2: Resolve Q3 (built-in vs user module for `PrePass::SlicePreview`)

- Task IDs: `TASK-163`
- Objective: Decide whether `PrePass::SlicePreview` is host-built-in (`execute_prepass_with_builtins`) or a user prepass module under `modules/core-modules/slice-preview/`.
- Precondition: Step 1.
- Postcondition: `design.md` Q3 marked resolved with chosen path. Subsequent steps reflect the decision.
- Files expected to change: `.ralph/specs/31_support-planner-orca-algorithmic-parity/design.md` (Q3 status).
- Verification: `grep -n 'Q3 (resolved)' .ralph/specs/31_support-planner-orca-algorithmic-parity/design.md` returns 1 match.
- Exit condition: Q3 resolution recorded.

### Step 3: Add `SlicePreviewIR` to `slicer-ir`

- Task IDs: `TASK-163`
- Objective: Add `pub struct SlicePreviewIR { pub schema_version: SemVer, pub entries: HashMap<RegionKey, Vec<ExPolygon>> }` to `crates/slicer-ir/src/slice_ir.rs`. Re-export from `crates/slicer-ir/src/lib.rs`.
- Precondition: Step 2.
- Postcondition: `cargo build -p slicer-ir 2>&1 | tail -5` exits 0.
- Files expected to change: `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-ir/src/lib.rs`.
- Verification: `grep -nE 'pub struct SlicePreviewIR' crates/slicer-ir/src/slice_ir.rs && grep -nE 'SlicePreviewIR' crates/slicer-ir/src/lib.rs` returns ≥2 matches.
- Exit condition: Build green; type reachable.

### Step 4: Implement `PrePass::SlicePreview` (per Q3 resolution)

- Task IDs: `TASK-163`
- Objective: Either (a) add a built-in computation to `crates/slicer-host/src/prepass.rs::execute_prepass_with_builtins` that computes plane-triangle intersections per layer per region and commits `SlicePreviewIR`, OR (b) ship `modules/core-modules/slice-preview/` with manifest + `wit-guest/` + `src/lib.rs` doing the same. In either case, add `BlackboardPrepassSlot::SlicePreview`, `commit_slice_preview`, and `slice_preview()` accessor in `crates/slicer-host/src/blackboard.rs`. Add `PrepassStageOutput::SlicePreview(Arc<SlicePreviewIR>)` and the `commit_stage_output` arm.
- Precondition: Step 3 complete; Q3 resolved.
- Postcondition: A unit test in `crates/slicer-host/src/prepass.rs::tests` (or the slice-preview module's tests) commits a `SlicePreviewIR` for a 2-layer cube and asserts `entries.len() == 2 * 1 * 1` (2 layers × 1 object × 1 region).
- Files expected to change: `crates/slicer-host/src/prepass.rs`, `crates/slicer-host/src/blackboard.rs`, optionally `modules/core-modules/slice-preview/*`.
- Verification: `cargo test -p slicer-host slice_preview 2>&1 | tail -15` passes.
- Exit condition: `SlicePreviewIR` reachable from a representative test.

### Step 5: Extend WIT prepass world with `slice-preview-view`

- Task IDs: `TASK-163`
- Objective: Add `record slice-preview-view-entry`, `record slice-preview-view`, and an additional `slice-preview: slice-preview-view` parameter to `export run-support-generation` in `wit/world-prepass.wit` (between `region-segmentation` and `output`).
- Precondition: Step 4.
- Postcondition: `cargo build --workspace 2>&1 | tail -10` exits 0 (with the dispatcher Step 6 temporarily passing an empty view).
- Files expected to change: `wit/world-prepass.wit`.
- Verification: `grep -nE 'record slice-preview-view-entry|record slice-preview-view\b|slice-preview: slice-preview-view' wit/world-prepass.wit` returns ≥3 matches.
- Exit condition: Workspace build green.

### Step 6: SDK + macro + projector wiring

- Task IDs: `TASK-163`
- Objective: Add `SlicePreviewView`, `SlicePreviewViewEntry` to `crates/slicer-sdk/src/prepass_types.rs`; re-export from `prelude.rs`. Extend `PrepassModule::run_support_generation` signature in `crates/slicer-sdk/src/traits.rs` to accept `&SlicePreviewView`. Extend `crates/slicer-macros/src/lib.rs` to thread the new arg. Add `crates/slicer-host/src/wit_host.rs::project_slice_preview_view` (deterministic ordering by `(global_layer_index, object_id, region_id)`) and pass it from the prepass dispatcher.
- Precondition: Step 5.
- Postcondition: All packet 28 + 30 tests still compile and pass.
- Files expected to change: `crates/slicer-sdk/src/prepass_types.rs`, `crates/slicer-sdk/src/prelude.rs`, `crates/slicer-sdk/src/traits.rs`, `crates/slicer-macros/src/lib.rs`, `crates/slicer-host/src/wit_host.rs`.
- Verification: `cargo test -p slicer-host --test prepass_support_generation_layer_plan_tdd -- --test-threads=1 2>&1 | tail -10` reports all packet-30 tests passing; `cargo build --workspace 2>&1 | tail -5` exits 0.
- Exit condition: Packet 30 regression suite green.

### Step 7: Extend `required_slots` for `PrePass::SupportGeneration`

- Task IDs: `TASK-163`
- Objective: Add `BlackboardPrepassSlot::SlicePreview` to the prerequisite slice in `crates/slicer-host/src/prepass.rs::required_slots` after `RegionMap`.
- Precondition: Step 6.
- Postcondition: Negative AC `prepass_support_generation_fails_without_slice_preview` will eventually prove the slot is required.
- Files expected to change: `crates/slicer-host/src/prepass.rs`.
- Verification: `grep -nA5 '"PrePass::SupportGeneration"' crates/slicer-host/src/prepass.rs | head -8` shows four slot entries in order.
- Exit condition: Packet 28 + 30 regression suites still green.

### Step 8: Resolve Q4 (avoidance inflation amount)

- Task IDs: `TASK-163`
- Objective: Decide between fixed `branch_radius + 0.4 mm` and config-driven `branch_radius + tree_support_branch_distance / 2`.
- Precondition: Step 7.
- Postcondition: `design.md` Q4 marked resolved.
- Files expected to change: `.ralph/specs/31_support-planner-orca-algorithmic-parity/design.md`.
- Verification: `grep -n 'Q4 (resolved)' .ralph/specs/31_support-planner-orca-algorithmic-parity/design.md` returns 1 match.
- Exit condition: Q4 recorded; the avoidance code in Step 10 has its formula.

### Step 9: Add failing TDD tests covering the new ACs

- Task IDs: `TASK-163`
- Objective: Create `crates/slicer-host/tests/prepass_support_generation_orca_parity_tdd.rs` with the nine tests named in the ACs (5 positive + 4 negative). The Benchy parity test is allowed to fail until Step 14.
- Precondition: Step 8.
- Postcondition: Compile clean; positive ACs related to radius taper, avoidance, raft+interface, wall-count fail; negatives mostly pass against the host-side prereq enforcement and config-validation paths.
- Files expected to change: `crates/slicer-host/tests/prepass_support_generation_orca_parity_tdd.rs`.
- Verification: `cargo test -p slicer-host --test prepass_support_generation_orca_parity_tdd -- --test-threads=1 2>&1 | tail -20`.
- Exit condition: Test scaffolding red-green-correct.

### Step 10: Implement avoidance + collision + wall-count move + radius tapering in the planner

- Task IDs: `TASK-163`
- Objective: In `modules/core-modules/support-planner/src/lib.rs`:
  - Add `MAX_BRANCH_RADIUS = 6.0` constant.
  - Add `dist_to_top: u32` field on `PlannedSupportNode`.
  - Replace the v1 propagation block:
    - Per layer, build avoidance + collision polygons from `slice_preview_view`.
    - Compute `max_move_distance = tan(branch_angle_rad) * effective_layer_height * tree_support_wall_count.max(1)`.
    - Move-pass clamps each node into avoidance polygons; drops + diagnoses any node whose target lies inside collision polygons.
    - Per-emit radius `= clamp(branch_diameter / 2 + tan(diameter_angle_rad) * dist_to_top * effective_layer_height, branch_diameter / 2, MAX_BRANCH_RADIUS)`.
    - `Point3WithWidth.width = 2 * radius_mm`.
- Precondition: Step 9.
- Postcondition: ACs `radius_tapers_with_distance_to_top`, `avoidance_keeps_branches_inside_layer_outline`, and `wall_count_scales_max_move_distance` pass. Negative AC `node_dropped_when_avoidance_rejects_all_moves` passes.
- Files expected to change: `modules/core-modules/support-planner/src/lib.rs`, `crates/slicer-helpers/src/geometry.rs` (helpers if missing).
- OrcaSlicer refs: `TreeSupport.cpp` 720–800, 2625–2860; `TreeModelVolumes.cpp`.
- Verification: Targeted `cargo test -p slicer-host --test prepass_support_generation_orca_parity_tdd -- --test-threads=1 2>&1 | tail -20` for those four tests.
- Exit condition: Algorithmic core green.

### Step 11: Resolve Q2 (raft Z convention) and implement raft + interface densification

- Task IDs: `TASK-163`
- Objective: Resolve Q2 (signed `global_layer_index` vs separate `raft_layers` field on `SupportPlanIR`). Implement the chosen path:
  - Update `crates/slicer-ir/src/slice_ir.rs::SupportPlanIR` and bump `schema_version` to `1.1.0`.
  - Update host harvester (`crates/slicer-host/src/dispatch.rs::harvest_support_plan_ir`) to round-trip the new shape.
  - Update tree-support's `support_plan_segments_for` to handle raft entries (matching by raft index when present).
  - Implement raft prefix entry emission in `support-planner/src/lib.rs`: for `support_raft_layers > 0`, prepend full-cross-section dense-fill entries.
  - Implement interface-layer densification: track first/last touch per branch column; for the top `support_interface_top_layers` and bottom `support_interface_bottom_layers` layers of each column, emit additional rectilinear scan-line dense fill at line spacing `tree_support_interface_spacing_mm`.
- Precondition: Step 10; Q2 resolved.
- Postcondition: AC `raft_and_interface_layers_emit_expected_entry_count` passes; packet 28 + 30 regression suites green.
- Files expected to change: `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-host/src/dispatch.rs`, `modules/core-modules/tree-support/src/lib.rs`, `modules/core-modules/support-planner/src/lib.rs`.
- OrcaSlicer refs: `TreeSupport.cpp` 1460–1700, 1913, raft references throughout.
- Verification: `cargo test -p slicer-host --test prepass_support_generation_orca_parity_tdd raft_and_interface_layers_emit_expected_entry_count -- --test-threads=1 2>&1 | tail -10`.
- Exit condition: Raft + interface ACs green; downstream consumers compile.

### Step 12: Update `support-planner.toml` config schema

- Task IDs: `TASK-163`
- Objective: Replace the four v1 keys (`support_branch_angle_deg`, `support_branch_merge_distance_mm`, `support_max_branches_per_layer`, `line_width`) with the nine new keys (`tree_support_branch_angle`, `tree_support_branch_diameter`, `tree_support_branch_diameter_angle`, `tree_support_branch_distance`, `tree_support_wall_count`, `support_raft_layers`, `support_interface_top_layers`, `support_interface_bottom_layers`, `tree_support_interface_spacing_mm`) in `support-planner.toml`. Also add `"SlicePreviewIR"` to `[ir-access].reads`.
- Precondition: Steps 10–11.
- Postcondition: AC-4 grep test passes; module loads with new config keys.
- Files expected to change: `modules/core-modules/support-planner/support-planner.toml`.
- Verification: AC-4's python3 grep command exits 0.
- Exit condition: Config schema correct.

### Step 13: Wire config-validation negative cases

- Task IDs: `TASK-163`
- Objective: Ensure the host's existing config-schema validator (per `docs/03 §config schema validation`) rejects `tree_support_branch_diameter_angle = 80.0` (above max) and `support_raft_layers = -1` with the documented error messages. Add the negative tests `diameter_angle_out_of_range_rejects_load` and `negative_raft_layers_rejects_load` in `prepass_support_generation_orca_parity_tdd.rs`.
- Precondition: Step 12.
- Postcondition: Both negative ACs pass.
- Files expected to change: `crates/slicer-host/tests/prepass_support_generation_orca_parity_tdd.rs` (extension); host config validator only if missing the bound checks.
- Verification: targeted `cargo test` for both tests.
- Exit condition: Config-validation negatives green.

### Step 14: Resolve Q5 and add Benchy parity golden + test

- Task IDs: `TASK-163`
- Objective: Resolve Q5 (parity tolerance). Generate `resources/golden/benchy_tree_support_orca_branch_count.txt` and `resources/golden/benchy_tree_support_orca_endpoints.txt` from a clean OrcaSlicer slice of `resources/test_models/benchy.stl` with `resources/test_config/benchy-tree-support.json`. Implement `benchy_orca_parity_within_tolerance` reading both goldens and asserting branch-count + Hausdorff distance.
- Precondition: Steps 10–13.
- Postcondition: AC-9 passes.
- Files expected to change: `resources/golden/benchy_tree_support_orca_branch_count.txt` (new), `resources/golden/benchy_tree_support_orca_endpoints.txt` (new), `crates/slicer-host/tests/prepass_support_generation_orca_parity_tdd.rs` (extension).
- Verification: AC-9's targeted cargo test exits 0.
- Exit condition: Benchy parity green.

### Step 15: Remove v1 module-level doc bullets

- Task IDs: `TASK-163`
- Objective: Edit the module-level docs in `modules/core-modules/support-planner/src/lib.rs` to remove the bullets for limits 3–7 (avoidance, radius, raft/interface, wall-count, branch tuning). Limits 1–2 were already removed by packet 30.
- Precondition: Steps 10–14.
- Postcondition: Module-level docs reflect only the genuinely-deferred items (heap-MST, soluble interface, geometry-aware multi-region branch separation).
- Files expected to change: `modules/core-modules/support-planner/src/lib.rs`.
- Verification: `grep -nE 'No avoidance|No radius tapering|No raft|wall-count-aware|branch_angle.*_diameter.*_distance' modules/core-modules/support-planner/src/lib.rs` returns 0 matches.
- Exit condition: Doc bullets scrubbed.

### Step 16: Rebuild every prepass `.wasm`

- Task IDs: `TASK-163`
- Objective: Cascade rebuild after the WIT change. Verify `--check` reports every artifact up to date.
- Precondition: Steps 11–15 complete.
- Postcondition: Every `.wasm` rebuilt; `--check` clean.
- Files expected to change: every `modules/core-modules/*/wit-guest/target/` and `.wasm` artifacts.
- Verification: `bash modules/core-modules/build-core-modules.sh 2>&1 | tail -10 && bash modules/core-modules/build-core-modules.sh --check 2>&1 | grep -E 'STALE'` returns no `STALE` matches.
- Exit condition: Cascade clean.

### Step 17: Update `docs/07_implementation_status.md`

- Task IDs: `TASK-163`
- Objective: Append the `TASK-163` row from `requirements.md` under Workstream 3.
- Precondition: Step 16.
- Postcondition: `docs/07` contains exactly one row matching `^- \[.\] TASK-163 .*31_support-planner-orca-algorithmic-parity`.
- Files expected to change: `docs/07_implementation_status.md`.
- Verification: AC-11's grep.
- Exit condition: Backlog updated.

### Step 18: Packet completion gate

- Task IDs: `TASK-163`
- Objective: Run the focused matrix.
- Precondition: Steps 1–17.
- Postcondition: All gate commands exit 0.
- Files expected to change: none.
- Verification:
  ```
  cargo test -p slicer-host --test prepass_support_generation_tdd -- --test-threads=1 --nocapture 2>&1 | tail -10
  cargo test -p slicer-host --test prepass_support_generation_layer_plan_tdd -- --test-threads=1 --nocapture 2>&1 | tail -10
  cargo test -p slicer-host --test prepass_support_generation_orca_parity_tdd -- --test-threads=1 --nocapture 2>&1 | tail -10
  cargo test -p slicer-host --test live_support_generation_tdd -- --test-threads=1 --nocapture 2>&1 | tail -10
  cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_with_support_enabled -- --test-threads=1 --nocapture 2>&1 | tail -10
  cargo test -p support-planner --lib 2>&1 | tail -10
  bash modules/core-modules/build-core-modules.sh --check 2>&1 | tail -10
  cargo build --workspace 2>&1 | tail -5
  cargo clippy --workspace -- -D warnings 2>&1 | tail -5
  ```
- Exit condition: All commands exit 0; packet ready for `spec-review`.

## Packet Completion Gate

- All steps complete.
- Every step exit condition met.
- Every pipe-suffixed AC command from `packet.spec.md` re-run and green (11 ACs + 4 negatives = 15 commands).
- All four open questions (Q2, Q3, Q4, Q5) resolved before activation.
- `docs/07_implementation_status.md` updated.
- Packets 28, 26, 30 still `status: implemented`.

## Acceptance Ceremony

- Re-run every `|`-suffixed AC verification command and confirm green.
- Confirm all packet-level verification commands are green.
- Record any remaining packet-local risk (e.g., golden-fixture brittleness across OrcaSlicer version updates) before moving to `status: implemented`.
