# Implementation Plan: 31b_support-planner-algorithmic-parity

## Execution Rules

- One atomic step at a time.
- All steps map to `TASK-163` (algorithmic portion).
- TDD: write failing tests before changing the planner.
- No WIT change in this packet — only `support-planner` module rebuilds (31a already extended the WIT).
- All open questions resolved: Q2 (raft = signed `global_layer_index` `i32`); Q3 (both branch count ±10% **and** Hausdorff ≤ 0.5mm must hold).

## Steps

### Step 1: Discovery — read OrcaSlicer references and 31a architectural contract

- Task IDs: `TASK-163`
- Objective: Re-read OrcaSlicer `TreeSupport.cpp` lines 720–800 (radius taper), 1460–1700 (interface), 1913 (`support_interface_top_layers` use site), 2625–2860 (`drop_nodes` propagation including `max_move_distance` and `wall_count`). Confirm `SupportGeometryView` shape from 31a and understand how the planner receives coarse support outlines at support resolution.
- Precondition: Packet 31a closed.
- Postcondition: Engineer can name the four config keys, the radius-taper formula, the wall-count multiplier, and the raft Z convention from memory. Can explain how `SupportGeometryView` feeds the avoidance build at support resolution.
- Files expected to change: none.
- Authoritative docs: `docs/01`–`docs/05`, `docs/08`, `docs/09`.
- OrcaSlicer refs: as above.
- Verification: `git status` clean.
- Context cost: S
- Exit condition: Engineer can sketch how `SupportGeometryView` entries (coarse outlines at support resolution, plus intermediate model-resolution layers near column tops) flow into the avoidance build and propagation loop.

### Step 2: Confirm Q2 and Q3 resolutions (already resolved)

- Task IDs: `TASK-163`
- Objective: Record that Q2 (raft Z convention = signed `global_layer_index` `i32`) and Q3 (both branch count ±10% and Hausdorff ≤ 0.5mm must hold) are already resolved.
- Precondition: Step 1.
- Postcondition: `design.md` Open Questions section shows all resolved.
- Files expected to change: none (documentation only).
- Verification: `grep -n 'Q2 (resolved)' .ralph/specs/31b_support-planner-algorithmic-parity/design.md && grep -n 'Q3 (resolved)' .ralph/specs/31b_support-planner-algorithmic-parity/design.md` returns 2 matches.
- Context cost: S
- Exit condition: Q2 and Q3 resolutions confirmed in design.md.

### Step 3: Update `support-planner.toml` config schema

- Task IDs: `TASK-163`
- Objective: Rewrite `[config.schema]` in `support-planner.toml`: drop `support_branch_angle_deg`, `support_branch_merge_distance_mm`, `support_max_branches_per_layer`, `line_width`; add `tree_support_branch_angle` (float, default 45.0, min 0.0, max 75.0), `tree_support_branch_diameter` (float, default 5.0, min 0.5, max 20.0), `tree_support_branch_diameter_angle` (float, default 5.0, min 0.0, max 90.0), `tree_support_branch_distance` (float, default 1.0, min 0.1, max 10.0), `tree_support_wall_count` (int, default 1, min 1, max 10), `support_raft_layers` (int, default 0, min 0, max 20), `support_interface_top_layers` (int, default 2, min 0, max 10), `support_interface_bottom_layers` (int, default -1, min -1, max 10), `tree_support_interface_spacing_mm` (float, default 0.4, min 0.1, max 2.0). Bounds on `tree_support_branch_diameter_angle` (max 90.0) and `support_raft_layers` (>= 0) per negative ACs.
- Precondition: Step 2.
- Postcondition: AC-1 python3 grep passes.
- Files expected to change: `modules/core-modules/support-planner/support-planner.toml`.
- Authoritative docs: `docs/03_wit_and_manifest.md`.
- Verification: AC-1's python3 command exits 0.
- Context cost: S
- Exit condition: Config schema correct; module loads with new keys.

### Step 4: Add failing TDD tests

- Task IDs: `TASK-163`
- Objective: Create `crates/slicer-host/tests/prepass_support_generation_orca_parity_tdd.rs` with 8 tests (5 positive + 3 negative). Tests compile against existing SDK (no new WIT — 31a already added `SupportGeometryView` to the export signature).
- Precondition: Step 3.
- Postcondition: Compile clean; positive ACs fail (planner not yet consuming `SupportGeometryView` for avoidance); negative ACs pass against host-side config validation.
- Files expected to change: `crates/slicer-host/tests/prepass_support_generation_orca_parity_tdd.rs` (new).
- Verification: `cargo test -p slicer-host --test prepass_support_generation_orca_parity_tdd -- --test-threads=1 2>&1 | tail -20`.
- Context cost: M
- Exit condition: TDD scaffolding red-green-correct.

### Step 5: Implement avoidance + collision + wall-count + radius tapering in the planner

- Task IDs: `TASK-163`
- Objective: In `modules/core-modules/support-planner/src/lib.rs`:
  - Add `MAX_BRANCH_RADIUS = 6.0` constant.
  - Add `dist_to_top: u32` field on `PlannedSupportNode`.
  - Build per-support-layer `collision_polys = union(SupportGeometryView[L][object_id][region_id].outlines)` and `avoidance_polys = collision_polys.inflate(branch_radius + tree_support_branch_distance / 2)`.
  - Replace v1 propagation block: `max_move_distance = tan(branch_angle_rad) * effective_layer_height * tree_support_wall_count.max(1)`; move-pass clamps into `avoidance_polys`; drops + diagnoses nodes whose target lies inside `collision_polys`.
  - Per-emit radius `= clamp(branch_diameter / 2 + tan(diameter_angle_rad) * dist_to_top * effective_layer_height, branch_diameter / 2, MAX_BRANCH_RADIUS)`.
  - `Point3WithWidth.width = 2 * radius_mm`.
- Precondition: Step 4 (failing tests in place).
- Postcondition: ACs `radius_tapers_with_distance_to_top`, `avoidance_keeps_branches_inside_support_outline`, and `wall_count_scales_max_move_distance` pass. Negative AC `node_dropped_when_avoidance_rejects_all_moves` passes.
- Files expected to change: `modules/core-modules/support-planner/src/lib.rs`, `crates/slicer-helpers/src/geometry.rs` (helpers if missing).
- OrcaSlicer refs: `TreeSupport.cpp` 720–800, 2625–2860; `TreeModelVolumes.cpp`.
- Verification: targeted `cargo test` for those four tests.
- Context cost: M
- Exit condition: Algorithmic core green.

### Step 6: Implement raft + interface densification (path a — signed global_layer_index)

- Task IDs: `TASK-163`
- Objective: Implement Path (a) for raft: `SupportPlanEntry.global_layer_index` widened from `u32` to `i32`. Update `harvest_support_plan_ir` in `crates/slicer-host/src/dispatch.rs` to round-trip signed indices. Update tree-support's `support_plan_segments_for` to handle negative indices for raft matching (raft entries sorted by negative index, searched by `Z`).
  - In `support-planner/src/lib.rs`: for `support_raft_layers > 0`, prepend full-cross-section dense-fill raft entries at Z values `z_bed - (i+1) * raft_layer_height_mm` (raft_layer_height = `effective_layer_height` of layer 0). Each raft entry carries `global_layer_index = -1, -2, ..., -raft_layers`.
  - Interface-layer densification: track first/last touch per branch column; for top `support_interface_top_layers` and bottom `support_interface_bottom_layers` layers of each column, emit additional rectilinear scan-line dense fill at line spacing `tree_support_interface_spacing_mm`.
- Precondition: Step 5; Q2 resolved (signed `global_layer_index`).
- Postcondition: AC `raft_and_interface_layers_emit_expected_entry_count` passes; packet 31a regression suite green.
- Files expected to change: `crates/slicer-ir/src/slice_ir.rs` (`SupportPlanEntry.global_layer_index`: `u32` → `i32`, schema bump), `crates/slicer-host/src/dispatch.rs`, `modules/core-modules/tree-support/src/lib.rs`, `modules/core-modules/support-planner/src/lib.rs`.
- OrcaSlicer refs: `TreeSupport.cpp` 1460–1700, 1913, raft references.
- Verification: `cargo test -p slicer-host --test prepass_support_generation_orca_parity_tdd raft_and_interface_layers_emit_expected_entry_count -- --test-threads=1 2>&1 | tail -10`.
- Context cost: M
- Exit condition: Raft + interface ACs green; downstream consumers compile.

### Step 7: Wire config-validation negative cases

- Task IDs: `TASK-163`
- Objective: Ensure the host's config-schema validator (per `docs/03 §config schema validation`) rejects `tree_support_branch_diameter_angle = 80.0` and `support_raft_layers = -1` with documented error messages.
- Precondition: Step 3 (bounds already in config schema).
- Postcondition: Negative ACs `diameter_angle_out_of_range_rejects_load` and `negative_raft_layers_rejects_load` pass.
- Files expected to change: none.
- Verification: targeted `cargo test` for both tests.
- Context cost: S
- Exit condition: Config-validation negatives green.

### Step 8: Add Benchy parity golden + test (Q3 already resolved — both metrics must pass)

- Task IDs: `TASK-163`
- Objective: Generate `resources/golden/benchy_tree_support_orca_branch_count.txt` (single integer: OrcaSlicer reference branch count) and `resources/golden/benchy_tree_support_orca_endpoints.txt` (newline-delimited `x,y,z` of OrcaSlicer reference branch endpoints) from a clean OrcaSlicer slice of `resources/test_models/benchy.stl` with `resources/test_config/benchy-tree-support.json`. Implement `benchy_orca_parity_within_tolerance` in the test file: asserts branch count within ±10% of the golden count AND endpoint Hausdorff distance ≤ 0.5mm (per `slicer_helpers::geometry::hausdorff_distance`). Either failure fails the test.
- Precondition: Steps 5–7.
- Postcondition: AC-6 passes.
- Files expected to change: `resources/golden/benchy_tree_support_orca_branch_count.txt` (new), `resources/golden/benchy_tree_support_orca_endpoints.txt` (new), `crates/slicer-host/tests/prepass_support_generation_orca_parity_tdd.rs` (extension).
- Verification: AC-6's targeted cargo test exits 0.
- Context cost: M
- Exit condition: Benchy parity green.

### Step 9: Remove v1 module-level doc bullets

- Task IDs: `TASK-163`
- Objective: Edit module-level docs in `modules/core-modules/support-planner/src/lib.rs` to remove bullets for limits 3–7 (avoidance, radius, raft/interface, wall-count, branch tuning). Limits 1–2 were removed by packet 30.
- Precondition: Steps 5–8.
- Postcondition: `grep -nE 'No avoidance|No radius tapering|No raft|wall-count-aware|branch_angle.*_diameter.*_distance' modules/core-modules/support-planner/src/lib.rs` returns 0 matches.
- Files expected to change: `modules/core-modules/support-planner/src/lib.rs`.
- Verification: the grep returns 0 matches.
- Context cost: S
- Exit condition: Doc bullets scrubbed.

### Step 10: Rebuild support-planner.wasm

- Task IDs: `TASK-163`
- Objective: Rebuild support-planner module. No other modules require rebuild (no WIT change in this packet — 31a already extended the WIT).
- Precondition: Steps 5–9 complete.
- Postcondition: `support-planner.wasm` rebuilt; `--check` reports it up to date.
- Files expected to change: `modules/core-modules/support-planner/wit-guest/target/` and the `.wasm` artifact.
- Verification: `bash modules/core-modules/build-core-modules.sh 2>&1 | tail -10 && bash modules/core-modules/build-core-modules.sh --check 2>&1 | grep -E 'support-planner.*up to date'`.
- Context cost: S
- Exit condition: Module rebuilt; no cascade to other modules.

### Step 11: Update `docs/07_implementation_status.md`

- Task IDs: `TASK-163`
- Objective: Update the `TASK-163` row to reference `31b_support-planner-algorithmic-parity` for the algorithmic portion.
- Precondition: Step 10.
- Postcondition: `docs/07` contains a row matching `TASK-163.*31b_support-planner-algorithmic-parity`.
- Files expected to change: `docs/07_implementation_status.md`.
- Verification: AC-8's grep.
- Context cost: S
- Exit condition: Backlog updated.

### Step 12: Packet completion gate

- Task IDs: `TASK-163`
- Objective: Run the focused matrix.
- Precondition: Steps 1–11.
- Postcondition: All gate commands exit 0.
- Files expected to change: none.
- Verification:
  ```
  cargo test -p slicer-host --test prepass_support_generation_tdd -- --test-threads=1 --nocapture 2>&1 | tail -10
  cargo test -p slicer-host --test prepass_support_generation_layer_plan_tdd -- --test-threads=1 --nocapture 2>&1 | tail -10
  cargo test -p slicer-host --test support_geometry_prepass_tdd -- --test-threads=1 --nocapture 2>&1 | tail -10
  cargo test -p slicer-host --test prepass_support_generation_orca_parity_tdd -- --test-threads=1 --nocapture 2>&1 | tail -10
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
- Every pipe-suffixed AC command from `packet.spec.md` re-run and green (8 ACs + 3 negatives = 11 commands).
- Q2 and Q3 resolved before activation. Q1 resolved by 31a.
- `docs/07_implementation_status.md` updated.
- Packets 28, 26, 30, 31a still `status: implemented`.

## Acceptance Ceremony

- Re-run every `|`-suffixed verification command from `packet.spec.md` and confirm green.
- Confirm all packet-level verification commands are green.
- Record any remaining packet-local risk (e.g., golden-fixture brittleness across OrcaSlicer version updates).
- Confirm no other active packet before marking `status: active` (per `.ralph/specs/README.md`).