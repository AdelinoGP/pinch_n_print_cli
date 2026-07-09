# Implementation Plan: 148-arachne-per-vertex-parity

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: Add `slicer-core` dependency (with `default-features=false`) and the 2 new config keys

- Task IDs:
  - none
- Objective: make the helpers from `slicer_core::perimeter_utils` reachable from the arachne module, and register the two new config keys in the arachne manifest.
- Precondition: `parity/arachne` is checked out at `182892ad`; cargo build is green; the 3 packet-1 red tests are red (3 passed, 12 red in `arachne_parity`).
- Postcondition: `cargo check -p arachne-perimeters` is green; `cargo build -p arachne-perimeters --target wasm32-unknown-unknown` is green (the `slicer-core` dep with `default-features=false` compiles to wasm); the arachne manifest TOML has two new sections; `cargo xtask build-guests --check` reports Fresh.
- Files allowed to read (with line-range hints when > 300 lines):
  - `modules/core-modules/arachne-perimeters/Cargo.toml` (26 lines, full)
  - `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` (204 lines, full)
  - `modules/core-modules/classic-perimeters/Cargo.toml` (≤ 30 lines, full — the precedent for the slicer-core dep)
  - `modules/core-modules/classic-perimeters/classic-perimeters.toml` (197 lines, lines 75-99 only — the precise_outer_wall + seam_candidate_angle_threshold_deg sections to confirm byte-for-byte)
- Files allowed to edit (≤ 3):
  - `modules/core-modules/arachne-perimeters/Cargo.toml`
  - `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/**` (no edits to slicer-core)
  - `modules/core-modules/arachne-perimeters/src/lib.rs` (no edits to source this step)
- Expected sub-agent dispatches:
  - "Read classic-perimeters.toml lines 75-99 and return the two new arachne sections as TOML; return SNIPPETS (verbatim, ≤ 30 lines)." — purpose: get the canonical exact-string entries.
  - "Run `cargo check -p arachne-perimeters 2>&1 | tee target/check.log`; return FACT (pass) or SNIPPETS (first 20 lines of error)." — purpose: confirm the new dep is reachable.
  - "Run `cargo build -p arachne-perimeters --target wasm32-unknown-unknown 2>&1 | tee target/wasm-build.log`; return FACT (pass) or SNIPPETS (first 20 lines of error)." — purpose: confirm the `default-features=false` pin compiles to wasm.
  - "Run `cargo xtask build-guests --check 2>&1 | tee target/guest-check.log`; return FACT (Fresh/STALE)." — purpose: confirm the manifest change is non-stale.
- Context cost: S
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` (delegate the `[config.schema]` section summary if needed; this file is > 300 lines)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp:1484-1489` — `precise_outer_wall` coBool provenance
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `seam_candidate_angle_threshold_deg` provenance
- Verification:
  - `cargo check -p arachne-perimeters --all-targets 2>&1 | tee target/check.log` — pass
  - `cargo build -p arachne-perimeters --target wasm32-unknown-unknown 2>&1 | tee target/wasm-build.log` — pass
  - `cargo xtask build-guests --check` — Fresh
- Exit condition: `cargo check` exit 0; `cargo build --target wasm32-unknown-unknown` exit 0; `xtask build-guests --check` exits 0 with `Fresh:` in the last 5 lines.

### Step 2: AC-1 — `boundary_type = ExteriorSurface` for `inset_idx == 0`

- Task IDs:
  - none
- Objective: replace the hardcoded `WallBoundaryType::Interior` at `arachne-perimeters/src/lib.rs:302` with a conditional that returns `ExteriorSurface` for the outer wall and `Interior` for inner walls.
- Precondition: Step 1 complete; the arachne module compiles.
- Postcondition: `arachne_parity_arachne_path_outer_wall_boundary_type_hardcoded_interior` is green.
- Files allowed to read (with line-range hints when > 300 lines):
  - `modules/core-modules/arachne-perimeters/src/lib.rs` lines 280-310 (the construction loop)
  - `crates/slicer-ir/src/slice_ir.rs` lines 1418-1428 (WallBoundaryType enum) — `rg 'pub enum WallBoundaryType' crates/slicer-ir/src/slice_ir.rs` is enough
- Files allowed to edit (≤ 3):
  - `modules/core-modules/arachne-perimeters/src/lib.rs`
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/**`, `crates/slicer-ir/src/**`, `crates/slicer-sdk/src/**` (read-only)
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test arachne_parity -- arachne_parity_arachne_path_outer_wall_boundary_type_hardcoded_interior 2>&1 | tee target/test-output.log`; return FACT (pass) or SNIPPETS (first 20 lines of failure)." — purpose: AC-1 green.
- Context cost: S
- Authoritative docs:
  - `docs/02_ir_schemas.md` §1418-1428 — delegate SUMMARY if implementer needs the enum variants
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:383` — `inset_idx` assignment
- Verification:
  - `cargo test -p slicer-runtime --test arachne_parity -- arachne_parity_arachne_path_outer_wall_boundary_type_hardcoded_interior` — pass
- Exit condition: AC-1 test green; no other test regressed (run the full `arachne_parity` file once and confirm the count of passing tests is now 4, not 3).

### Step 3: AC-2 + AC-3 — `LoopType::ThinWall` + `is_thin_wall` flag

- Task IDs:
  - none
- Objective: extend `classify_line` (lib.rs:206-214) to return `LoopType::ThinWall` for widened thin-wall beads, and wire `feature_flags[i].is_thin_wall = true` in the construction loop for those walls.
- Precondition: Step 2 complete.
- Postcondition: `arachne_parity_arachne_path_thin_wall_loop_type_never_emitted` and `arachne_parity_arachne_path_is_thin_wall_flag_never_set` are green.
- Files allowed to read (with line-range hints when > 300 lines):
  - `modules/core-modules/arachne-perimeters/src/lib.rs` lines 200-220 (classify_line) and 280-310 (construction loop)
  - `modules/core-modules/classic-perimeters/src/lib.rs` lines 760-800 (the ThinWall emission path; the canonical pattern)
- Files allowed to edit (≤ 3):
  - `modules/core-modules/arachne-perimeters/src/lib.rs`
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/beading/**` (the beading-strategy stack is correct; do not touch)
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test arachne_parity -- arachne_parity_arachne_path_thin_wall_loop_type_never_emitted 2>&1 | tee target/test-output.log`; return FACT (pass) or SNIPPETS (first 20 lines of failure)." — purpose: AC-2 green.
  - "Run `cargo test -p slicer-runtime --test arachne_parity -- arachne_parity_arachne_path_is_thin_wall_flag_never_set 2>&1 | tee target/test-output.log`; return FACT (pass) or SNIPPETS (first 20 lines of failure)." — purpose: AC-3 green.
- Context cost: S
- Authoritative docs:
  - `docs/02_ir_schemas.md` §1505-1516 — LoopType::ThinWall (delegate)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/WideningBeadingStrategy.cpp:27-77` — thin-wall widening
- Verification:
  - Both AC-2 and AC-3 tests green
- Exit condition: full `arachne_parity` count is now 6 passed (3 packet-1 + 2 this step + 1 from Step 2), 9 red.

### Step 4: AC-4 + AC-5 — per-vertex `is_bridge` and `overhang_quartile` from region data

- Task IDs:
  - none
- Objective: in the construction loop, populate `feature_flags[i].is_bridge` per-vertex via `point_in_any_polygon(pt, region.bridge_areas())`, and populate `path.points[i].overhang_quartile` per-vertex via the band lookup against `region.overhang_quartile_polygons()`.
- Precondition: Step 3 complete; `slicer-core` is a reachable dep from Step 1.
- Postcondition: `arachne_parity_arachne_path_is_bridge_flag_never_set` and `arachne_parity_arachne_path_overhang_quartile_hardcoded_none` are green.
- Files allowed to read (with line-range hints when > 300 lines):
  - `modules/core-modules/arachne-perimeters/src/lib.rs` lines 280-310
  - `crates/slicer-core/src/perimeter_utils.rs:608` (`point_in_any_polygon` signature) and `:316-331` (overhang-band lookup pattern in `expolygon_to_path3d`)
  - `crates/slicer-sdk/src/views.rs:388, 468` (accessor signatures)
  - `crates/slicer-ir/src/slice_ir.rs:1542-1558` (`Point3WithWidth.overhang_quartile` field)
- Files allowed to edit (≤ 3):
  - `modules/core-modules/arachne-perimeters/src/lib.rs`
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/perimeter_utils.rs` (the helper is correct; do not touch)
  - `crates/slicer-sdk/src/views.rs` (the accessor is correct; do not touch)
- Expected sub-agent dispatches:
  - "Summarize `crates/slicer-core/src/perimeter_utils.rs:316-331` (the overhang-band lookup pattern in `expolygon_to_path3d`); return SUMMARY ≤ 200 words." — purpose: confirm the canonical lookup shape.
  - "Run `cargo test -p slicer-runtime --test arachne_parity -- arachne_parity_arachne_path_is_bridge_flag_never_set 2>&1 | tee target/test-output.log`; return FACT (pass) or SNIPPETS (first 20 lines of failure)." — purpose: AC-4 green.
  - "Run `cargo test -p slicer-runtime --test arachne_parity -- arachne_parity_arachne_path_overhang_quartile_hardcoded_none 2>&1 | tee target/test-output.log`; return FACT (pass) or SNIPPETS (first 20 lines of failure)." — purpose: AC-5 green.
- Context cost: M (the band lookup is the largest single edit; the bridge lookup is trivial)
- Authoritative docs:
  - `docs/02_ir_schemas.md` §1520-1533 (WallFeatureFlags), §1542-1558 (Point3WithWidth) — delegate SUMMARY
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:675-678` — per-vertex is_bridge
  - `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:2113-2119` — overhang quartile population
- Verification:
  - AC-4 + AC-5 green
- Exit condition: full `arachne_parity` count is now 8 passed (3 packet-1 + 4 this packet so far), 7 red.

### Step 5: AC-6 + AC-7 — seam-candidate emission + `precise_outer_wall` registration

- Task IDs:
  - none
- Objective: in `run_perimeters`, after the `for wall in walls { output.push_wall_loop(wall)?; }` loop, iterate the outer region and call `generate_sharp_corner_seam_candidates(&region.polygons()[0].contour, region.z(), seam_candidate_angle_threshold_deg)`, then `output.push_seam_candidate(c.position, c.score)?` for each. The seam helper takes a `&slicer_ir::Polygon` (units-space input contour) — NOT `&wall.path` (mm-space `ExtrusionPath3D`); the call shape matches classic's `lib.rs:889-900` (`generate_sharp_corner_seam_candidates(&poly.contour, z, threshold_deg)`). Then verify the test for AC-7 (manifest presence of `precise_outer_wall` and `seam_candidate_angle_threshold_deg` keys) — Step 1 already added the manifest entries, so AC-7 should now be green on the test run. AC-6 needs the new code path; AC-7 just needs Step 1's manifest change to be present.
- Precondition: Step 4 complete; `slicer-core` and the two new config keys are reachable.
- Postcondition: `arachne_parity_arachne_path_seam_candidate_producer_missing` (rewritten as `arachne_parity_seam_candidate_tdd`) and `arachne_parity_arachne_path_precise_outer_wall_not_registered` (rewritten as `arachne_parity_precise_outer_wall_manifest_tdd`) are green.
- Files allowed to read (with line-range hints when > 300 lines):
  - `modules/core-modules/arachne-perimeters/src/lib.rs` lines 310-330 (the post-loop region)
  - `crates/slicer-core/src/perimeter_utils.rs:460` (`generate_sharp_corner_seam_candidates` signature)
  - `modules/core-modules/classic-perimeters/src/lib.rs:889-900` (the canonical seam-candidate emission; the `&poly.contour` call shape is the precedent)
- Files allowed to edit (≤ 3):
  - `modules/core-modules/arachne-perimeters/src/lib.rs`
  - `modules/core-modules/arachne-perimeters/tests/arachne_parity_seam_candidate_tdd.rs` (new file, AC-6)
  - `modules/core-modules/arachne-perimeters/tests/arachne_parity_precise_outer_wall_manifest_tdd.rs` (new file, AC-7)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/perimeter_utils.rs` (read-only; do not touch)
- Expected sub-agent dispatches:
  - "Run `cargo test -p arachne-perimeters --test arachne_parity_seam_candidate_tdd 2>&1 | tee target/test-output.log`; return FACT (pass) or SNIPPETS (first 20 lines of failure)." — purpose: AC-6 green.
  - "Run `cargo test -p arachne-perimeters --test arachne_parity_precise_outer_wall_manifest_tdd 2>&1 | tee target/test-output.log`; return FACT (pass) or SNIPPETS (first 20 lines of failure)." — purpose: AC-7 green.
- Context cost: M (importing a new helper, traversing the outer region, bridging types)
- Authoritative docs:
  - `docs/05_module_sdk.md` §"SliceRegionView accessors" — the `seam_candidate` field is consumed by `seam-placer`, not produced by it; this step produces for `seam-placer` to consume. No docs change needed.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:2093-2535` — seam candidate emission
- Verification:
  - AC-6 + AC-7 green
- Exit condition: full `arachne_parity` count is now 10 passed (3 packet-1 + 7 packet-148 arachne-path rewritten), 5 red (4 packet-149 + 1 D-104f).

### Step 6: AC-8 — `precise_outer_wall` beading-stack mechanism unit test + offset gating

- Task IDs:
  - none
- Objective: add a unit test in `arachne-perimeters/tests/precise_outer_wall_tdd.rs` (NEW) that sets up a region with `wall_sequence=InnerOuter` and `precise_outer_wall=true`, runs `ArachnePerimeters::run_perimeters`, and asserts the outermost wall's path lies at `-(ext_perimeter_width/2 - ext_perimeter_spacing/2)` from the input polygon boundary. **Mechanism rewrite**: in `arachne_params_from_config`, when `precise_outer_wall && wall_sequence == InnerOuter`, set `params.outer_wall_offset = -(ext_perimeter_width/2 - ext_perimeter_spacing/2)`; otherwise `params.outer_wall_offset = 0.0`. The inset is realized at the beading-stack level via the existing `ArachneParams.outer_wall_offset` plumbing (wired at `arachne-perimeters/src/lib.rs:157`; flows into `BeadingStrategyFactory::makeStrategy(..., outer_wall_offset, ...)` mirroring OrcaSlicer's `OuterWallInsetBeadingStrategy::compute` at `Arachne/BeadingStrategy/OuterWallInsetBeadingStrategy.cpp:44-60`). This is NOT a post-hoc `wall.path` mutation.
- Precondition: Step 5 complete.
- Postcondition: a new unit test in `arachne-perimeters/tests/precise_outer_wall_tdd.rs` is green (positive: `precise_outer_wall=true` → outer wall inset applied; negative: `precise_outer_wall=false` → no inset, AC-N2). The `outer_wall_offset` is gated on `precise_outer_wall && wall_sequence==InnerOuter`.
- Files allowed to read (with line-range hints when > 300 lines):
  - `modules/core-modules/arachne-perimeters/src/lib.rs` lines 106-197 (arachne_params_from_config), 157 (the existing `outer_wall_offset` wiring)
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/OuterWallInsetBeadingStrategy.cpp:44-60` (the canonical beading-stack mechanism; delegate via SUMMARY)
- Files allowed to edit (≤ 3):
  - `modules/core-modules/arachne-perimeters/src/lib.rs`
  - `modules/core-modules/arachne-perimeters/tests/precise_outer_wall_tdd.rs` (new file)
- Files explicitly out-of-bounds for this step:
  - `modules/core-modules/classic-perimeters/src/lib.rs` (read-only; classic's precise_outer_wall uses a different mechanism — medial-axis spacing, not the beading-stack — and is not the right precedent for the arachne path)
  - `crates/slicer-core/src/beading/*` (the beading strategy stack already consumes `outer_wall_offset`; no edits)
- Expected sub-agent dispatches:
  - "Summarize `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/OuterWallInsetBeadingStrategy.cpp:44-60`; return SUMMARY (≤ 200 words) of the beading-stack precise_outer_wall mechanism." — purpose: confirm the AC-8 offset formula mirrors OrcaSlicer.
  - "Run `cargo test -p arachne-perimeters --test precise_outer_wall_tdd 2>&1 | tee target/test-output.log`; return FACT (pass) or SNIPPETS (first 20 lines of failure)." — purpose: AC-8 green.
  - "Run `cargo test -p arachne-perimeters --test precise_outer_wall_tdd -- --skip precise_outer_wall_default_off 2>&1 | tee target/test-output.log` (or equivalent filter for the negative case); return FACT (no offset applied)." — purpose: AC-N2.
- Context cost: M (gating logic in `arachne_params_from_config`; the unit test exercises the beading-stack output via `output.wall_loops()[0].path.points` geometry)
- Authoritative docs:
  - none (this is implementation-only; the config key is registered in Step 1)
- Verification:
  - `cargo test -p arachne-perimeters --test precise_outer_wall_tdd` — pass (positive and negative cases)
- Exit condition: AC-8 + AC-N2 both green.

### Step 7: AC-1 through AC-5 unit tests — `arachne-parimeters/tests/` harness

- Task IDs:
  - none
- Objective: add the remaining 5 unit-test files in `arachne-perimeters/tests/` that drive `ArachnePerimeters::run_perimeters` natively and assert on real `WallLoop` output (the per-AC coverage that the rewritten `arachne_parity.rs` delegates to):
  - `arachne_parity_outer_wall_boundary_type_tdd.rs` (AC-1)
  - `arachne_parity_thin_wall_loop_type_tdd.rs` (AC-2)
  - `arachne_parity_is_thin_wall_flag_tdd.rs` (AC-3)
  - `arachne_parity_is_bridge_flag_tdd.rs` (AC-4)
  - `arachne_parity_overhang_quartile_tdd.rs` (AC-5)
  - Each builds a `SliceRegionView` via `SliceRegionViewBuilder` (with `set_bridge_areas(...)` for AC-4 and `set_overhang_quartile_polygons(...)` for AC-5 as needed), a `ConfigView` via `ConfigViewBuilder`, constructs `PerimeterOutputBuilder::new()`, calls `ArachnePerimeters.run_perimeters(0, &[region], &PaintRegionLayerView::new(0), &mut output, &config)`, and asserts on `output.wall_loops()`. The harness pattern is identical to `modules/core-modules/classic-perimeters/tests/classic_perimeters_tdd.rs:1-50`.
- Precondition: Steps 1-6 complete (the guest module code is in; the per-vertex overrides for `boundary_type`, `is_bridge`, `is_thin_wall`, `overhang_quartile` are wired; the seam emission is wired; `outer_wall_offset` gating is wired).
- Postcondition: 5 new unit tests in `arachne-perimeters/tests/` are green; the runtime `output.wall_loops()` fields match the AC assertions.
- Files allowed to read:
  - `modules/core-modules/classic-perimeters/tests/classic_perimeters_tdd.rs:1-50` (the harness precedent)
  - `crates/slicer-sdk/src/test_support/fixtures.rs` (`SliceRegionViewBuilder`, `ConfigViewBuilder`)
  - `crates/slicer-sdk/src/test_support/capture.rs` (the test-only `PerimeterOutputCapture` if needed)
  - `crates/slicer-ir/src/polygon_predicate.rs:41-47` (`point_in_polygon_winding`, `point_in_contour_winding` — wasm-compatible helpers used by the per-vertex band lookup)
- Files allowed to edit (≤ 6):
  - `modules/core-modules/arachne-perimeters/tests/arachne_parity_outer_wall_boundary_type_tdd.rs` (new)
  - `modules/core-modules/arachne-perimeters/tests/arachne_parity_thin_wall_loop_type_tdd.rs` (new)
  - `modules/core-modules/arachne-perimeters/tests/arachne_parity_is_thin_wall_flag_tdd.rs` (new)
  - `modules/core-modules/arachne-perimeters/tests/arachne_parity_is_bridge_flag_tdd.rs` (new)
  - `modules/core-modules/arachne-perimeters/tests/arachne_parity_overhang_quartile_tdd.rs` (new)
- Files explicitly out-of-bounds for this step:
  - `modules/core-modules/arachne-perimeters/src/lib.rs` (read-only; the implementation is already in from Steps 1-6)
  - the manifests (read-only; Step 1 already set them)
- Expected sub-agent dispatches:
  - "For each of the 5 arachne-path ACs, run `cargo test -p arachne-perimeters --test <test_name> 2>&1 | tee target/test-output.log`; return FACT (pass) or SNIPPETS (first 20 lines of failure)." — purpose: per-AC unit test verification.
  - "Read `modules/core-modules/classic-perimeters/tests/classic_perimeters_tdd.rs:1-50`; return SNIPPETS (verbatim, ≤ 30 lines) of the harness pattern (SliceRegionViewBuilder + ConfigViewBuilder + PerimeterOutputBuilder::new() + run_perimeters call)." — purpose: confirm the harness shape.
- Context cost: M (5 new test files, each a small variation on the harness pattern; parallelizable to a sub-agent)
- Authoritative docs:
  - none (the harness pattern is established by the classic module's existing tests)
- OrcaSlicer refs:
  - none
- Verification:
  - All 5 new unit tests green
- Exit condition: `cargo test -p arachne-perimeters --tests` shows all 7+ unit tests (Steps 5+6+7) green; `cargo test -p slicer-runtime --test arachne_parity` shows 10 passed (3 packet-1 + 7 packet-148) once the test rewrite in Step 8 is in.

### Step 8: Test rewrite — `arachne_parity.rs` rewritten to drive `run_perimeters` natively

- Task IDs:
  - none
- Objective: REWRITE `crates/slicer-runtime/tests/arachne_parity.rs` so the 7 arachne-path red tests drive `ArachnePerimeters::run_perimeters` natively (delegating to the new `arachne-perimeters/tests/*_tdd.rs` files, OR running the same harness inline). The 3 packet-1 stale-doc tests, 4 packet-149 pipeline-config tests, and 1 D-104f test are preserved with their existing predicates (they are manifest-presence / wiring-presence concerns and are correct as-is). The 7 arachne-path substring-matching tests are DELETED (the `arachne-parimeters/tests/*_tdd.rs` files are the canonical coverage). This step is sub-agent-dispatched (see Expected sub-agent dispatches) — the test rewrite is a substantial chunk of work and benefits from isolation.
- Precondition: Steps 1-7 complete (the guest module is in; the 7+ new unit tests are green; the harness pattern is established).
- Postcondition: `cargo test -p slicer-runtime --test arachne_parity` shows 10 passed (3 packet-1 stale-doc + 7 packet-148 arachne-path rewritten) and 5 red (4 packet-149 + 1 D-104f deferred). The substring-matching tests are gone.
- Files allowed to read:
  - `crates/slicer-runtime/tests/arachne_parity.rs` (the entire file, ≤ 607 lines)
  - `modules/core-modules/arachne-perimeters/tests/arachne_parity_*_tdd.rs` (the new unit tests, the canonical coverage)
  - `modules/core-modules/classic-perimeters/tests/classic_perimeters_tdd.rs` (the harness pattern)
- Files allowed to edit (≤ 1):
  - `crates/slicer-runtime/tests/arachne_parity.rs` (REWRITE; the 7 arachne-path tests are replaced with thin delegators that simply `include!` the new `arachne-perimeters/tests/*_tdd.rs` bodies, OR the arachne-path tests are deleted and the packet-1/packet-149/D-104f tests remain)
- Files explicitly out-of-bounds for this step:
  - the guest module source, manifests, Cargo.toml (read-only; no further code changes)
  - `modules/core-modules/arachne-perimeters/tests/*_tdd.rs` (read-only; the canonical coverage is in place)
- Expected sub-agent dispatches:
  - "Read `crates/slicer-runtime/tests/arachne_parity.rs` (≤ 607 lines); list the 7 arachne-path test names and the 3 stale-doc + 4 pipeline-config + 1 D-104f test names. Return SUMMARY (≤ 200 words)." — purpose: confirm the rewrite scope.
  - "Rewrite the 7 arachne-path tests in `arachne_parity.rs` to delegate to the new `arachne-perimeters/tests/arachne_parity_*_tdd.rs` files (or run the same harness inline). Verify `cargo test -p slicer-runtime --test arachne_parity` shows 10 passed, 5 red after the rewrite. Return FACT (pass count = 10, red count = 5) or SNIPPETS (first 20 lines of failure)." — purpose: full-file green count.
- Context cost: M (large mechanical rewrite; the existing substring tests are deleted or replaced with thin delegators)
- Authoritative docs:
  - none (the rewrite is mechanical: replace substring assertions with `output.wall_loops()` / `output.seam_candidates()` assertions)
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo test -p slicer-runtime --test arachne_parity 2>&1 | tee target/test-output.log` — 10 passed, 5 red
  - `cargo clippy -p slicer-runtime --test arachne_parity -- -D warnings 2>&1 | tee target/clippy-output.log` — clean
- Exit condition: AC-9 satisfied (full arachne_parity count is 10 passed, 5 red).

### Step 9: Deviation log + audit history + config-keys reference updates

- Task IDs:
  - none
- Objective: refine `D-104-OVERHANG-QUARTILE-NONE` in `docs/DEVIATION_LOG.md` to "arachne-path-only per-vertex overhang/flag/seam/boundary parity" (Status flips to `Closed — 2026-07-09: packet 148 closes the arachne-path half`); append a one-line record to `docs/14_deviation_audit_history.md`; append the two new config keys to `docs/15_config_keys_reference.md` §Walls.
- Precondition: Steps 5-8 complete (the code changes that close D-104's arachne-path half are in, and the test rewrite is complete).
- Postcondition: `rg -q 'arachne-path-only per-vertex overhang' docs/DEVIATION_LOG.md` is a hit; `rg -q 'seam_candidate_angle_threshold_deg' docs/15_config_keys_reference.md` is a hit; `rg -q 'Packet 148' docs/14_deviation_audit_history.md` is a hit.
- Files allowed to read (with line-range hints when > 300 lines):
  - `docs/DEVIATION_LOG.md` lines 22-50 (the D-104 entry to refine)
  - `docs/14_deviation_audit_history.md` (whole file, ≤ 100 lines)
  - `docs/15_config_keys_reference.md` §Walls (the section to append to)
- Files allowed to edit (≤ 3):
  - `docs/DEVIATION_LOG.md`
  - `docs/14_deviation_audit_history.md`
  - `docs/15_config_keys_reference.md`
- Files explicitly out-of-bounds for this step:
  - `docs/07_implementation_status.md` (NOT edited by this step; the packet's task IDs are not on the backlog yet)
- Expected sub-agent dispatches:
  - "Run `rg -q 'arachne-path-only per-vertex overhang' docs/DEVIATION_LOG.md; echo $?`; return FACT (exit 0 = pass)." — purpose: Doc Impact grep 1.
  - "Run `rg -q 'seam_candidate_angle_threshold_deg' docs/15_config_keys_reference.md; echo $?`; return FACT (exit 0 = pass)." — purpose: Doc Impact grep 2.
  - "Run `rg -q 'Packet 148' docs/14_deviation_audit_history.md; echo $?`; return FACT (exit 0 = pass)." — purpose: Doc Impact grep 3.
- Context cost: S
- Authoritative docs:
  - `docs/14_deviation_audit_history.md` — read for the existing row format
- OrcaSlicer refs:
  - none
- Verification:
  - All three Doc Impact greps pass
- Exit condition: D-104 row reflects the new scope; the two config keys are documented; the audit history has a one-line record.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Cargo.toml (`slicer-core` with `default-features=false`) + manifest edits, no source-code changes |
| Step 2 | S | Single-line conditional in the construction loop |
| Step 3 | S | Two-line `classify_line` extension + per-vertex flag in the construction loop |
| Step 4 | M | Per-vertex is_bridge + overhang-band lookup; largest single edit |
| Step 5 | M | New helper import + post-loop seam emission (input polygon contour, not `wall.path`) + 2 new test files (AC-6, AC-7) |
| Step 6 | M | AC-8 beading-stack offset gating in `arachne_params_from_config` + 1 new test file (AC-8 + AC-N2) |
| Step 7 | M | 5 new unit-test files driving `run_perimeters` natively (AC-1 through AC-5) |
| Step 8 | M | Test rewrite — `arachne_parity.rs` rewritten to drive `run_perimeters` natively (or delegate to the new test files); sub-agent-dispatched |
| Step 9 | S | Deviation log + audit history + config keys reference |

Aggregate: M (sum of S+S+S+M+M+M+M+M+S = M-equivalent). No step is L. The packet does not need to split before activation.

## Packet Completion Gate

- All 9 steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (each verification command dispatched and returned PASS).
- The arachne guest is rebuilt: `cargo xtask build-guests --check` is Fresh.
- The arachne guest builds to wasm: `cargo build -p arachne-perimeters --target wasm32-unknown-unknown` exits 0.
- `docs/DEVIATION_LOG.md` §D-104 row reflects the arachne-path-only scope.
- `docs/14_deviation_audit_history.md` has a one-line record of the scope refinement.
- `docs/15_config_keys_reference.md` §Walls has both new keys.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (AC-1 through AC-9, AC-N1, AC-N2).
- Confirm packet-level verification commands are green: `cargo test -p slicer-runtime --test arachne_parity` shows 10 passed, 5 red (4 packet-149 + 1 D-104f deferred).
- Confirm `cargo test -p arachne-perimeters --tests` is clean (all 7+ new unit tests green).
- Confirm `cargo build -p arachne-perimeters --target wasm32-unknown-unknown` exits 0 (the `slicer-core` dep with `default-features=false` compiles to wasm).
- Confirm `cargo clippy -p slicer-runtime --test arachne_parity -- -D warnings` is clean.
- Confirm `cargo xtask build-guests --check` is Fresh.
- Record any remaining packet-local risk explicitly before moving to `status: implemented` (the largest residual risk is the overhang-band lookup's per-vertex cost; flagged in `design.md` §Risks).
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a packet-authoring lesson for future spec-packet-generator runs.
