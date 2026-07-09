# Implementation Plan: 148-arachne-per-vertex-parity

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: Add `slicer-core` dependency (with `default-features=false`), the 3 new config keys, and the test-fixture setter

- Task IDs:
  - none
- Objective: make the helpers from `slicer_core::perimeter_utils` reachable from the arachne module; register the three new config keys in the arachne manifest (`precise_outer_wall`, `seam_candidate_angle_threshold_deg`, AND `wall_sequence` — the AC-8 gate reads `wall_sequence`, which classic registers at `classic-perimeters.toml:81` but arachne does not; without it the gate cannot be evaluated); add an `overhang_quartile_polygons(...)` setter to `SliceRegionViewBuilder` in `crates/slicer-sdk/src/test_support/fixtures.rs` (the builder has `bridge_areas(...)` at fixtures.rs:355 and `overhang_areas(...)` at :387 but NO quartile-band setter today — AC-5's unit-test fixture cannot be built without it).
- Precondition: `parity/arachne` is checked out at `182892ad`; cargo build is green; the 3 packet-1 red tests are red (3 passed, 12 red in `arachne_parity`).
- Postcondition: `cargo check -p arachne-perimeters` is green; `cargo build -p arachne-perimeters --target wasm32-unknown-unknown` is green (the `slicer-core` dep with `default-features=false` compiles to wasm); the arachne manifest TOML has three new sections; `SliceRegionViewBuilder::overhang_quartile_polygons(...)` compiles (`cargo check -p slicer-sdk --all-targets --features test` — the bare `--all-targets` form fails on a clean baseline because `test_support` is feature-gated); `cargo xtask build-guests --check` reports Fresh.
- Files allowed to read (with line-range hints when > 300 lines):
  - `modules/core-modules/arachne-perimeters/Cargo.toml` (26 lines, full)
  - `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` (204 lines, full)
  - `modules/core-modules/classic-perimeters/Cargo.toml` (≤ 30 lines, full — the precedent for the slicer-core dep)
  - `modules/core-modules/classic-perimeters/classic-perimeters.toml` (197 lines, lines 75-99 for precise_outer_wall + seam_candidate_angle_threshold_deg, lines 81-86 for wall_sequence — confirm byte-for-byte)
  - `crates/slicer-sdk/src/test_support/fixtures.rs` lines 340-400 (the `bridge_areas`/`overhang_areas` setters — the pattern for the new quartile-band setter)
- Files allowed to edit (≤ 3):
  - `modules/core-modules/arachne-perimeters/Cargo.toml`
  - `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`
  - `crates/slicer-sdk/src/test_support/fixtures.rs`
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
- Objective: TDD. FIRST write `modules/core-modules/arachne-perimeters/tests/arachne_parity_outer_wall_boundary_type_tdd.rs` (drives `run_perimeters` natively per the harness at `classic_perimeters_tdd.rs:1-70`; asserts `output.wall_loops()` where `perimeter_index == 0` has `boundary_type == WallBoundaryType::ExteriorSurface`) and confirm it is RED. THEN replace the hardcoded `WallBoundaryType::Interior` at `arachne-perimeters/src/lib.rs:302` with a conditional returning `ExteriorSurface` for `line.inset_idx == 0` and `Interior` otherwise; confirm the new test is GREEN. **Do NOT verify via the old substring test `arachne_parity_arachne_path_outer_wall_boundary_type_hardcoded_interior` — it hardcodes its own failure (`arachne_parity.rs:444-446` constructs a local `Interior` and asserts it is not `Interior`) and can NEVER pass regardless of module changes; it stays red until Step 8 deletes it.**
- Precondition: Step 1 complete; the arachne module compiles; the fixtures setter from Step 1 exists.
- Postcondition: `cargo test -p arachne-perimeters --test arachne_parity_outer_wall_boundary_type_tdd` is green.
- Files allowed to read (with line-range hints when > 300 lines):
  - `modules/core-modules/arachne-perimeters/src/lib.rs` lines 280-310 (the construction loop)
  - `crates/slicer-ir/src/slice_ir.rs` lines 1418-1428 (WallBoundaryType enum) — `rg 'pub enum WallBoundaryType' crates/slicer-ir/src/slice_ir.rs` is enough
  - `modules/core-modules/classic-perimeters/tests/classic_perimeters_tdd.rs` lines 1-70 (the native harness pattern)
- Files allowed to edit (≤ 3):
  - `modules/core-modules/arachne-perimeters/src/lib.rs`
  - `modules/core-modules/arachne-perimeters/tests/arachne_parity_outer_wall_boundary_type_tdd.rs` (new)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/**`, `crates/slicer-ir/src/**`, `crates/slicer-sdk/src/**` (read-only)
  - `crates/slicer-runtime/tests/arachne_parity.rs` (untouched until Step 8)
- Expected sub-agent dispatches:
  - "Run `cargo test -p arachne-perimeters --test arachne_parity_outer_wall_boundary_type_tdd 2>&1 | tee target/test-output.log`; return FACT (pass) or SNIPPETS (first 20 lines of failure)." — purpose: AC-1 green (run once before the code change expecting RED, once after expecting GREEN).
- Context cost: S
- Authoritative docs:
  - `docs/02_ir_schemas.md` §1418-1428 — delegate SUMMARY if implementer needs the enum variants
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:383` — `inset_idx` assignment
- Verification:
  - `cargo test -p arachne-perimeters --test arachne_parity_outer_wall_boundary_type_tdd` — pass
- Exit condition: the new unit test was RED before the lib.rs edit and is GREEN after it. (The full `arachne_parity` pass count does NOT move this step — the old substring tests are structurally un-passable and are removed in Step 8.)

### Step 3: AC-2 + AC-3 — `LoopType::ThinWall` + `is_thin_wall` flag

- Task IDs:
  - none
- Objective: TDD. FIRST write `arachne-perimeters/tests/arachne_parity_thin_wall_loop_type_tdd.rs` (AC-2) and `arachne_parity_is_thin_wall_flag_tdd.rs` (AC-3) driving `run_perimeters` natively with the 0.25 mm × 5 mm thin-strip fixture and `detect_thin_wall=true`; confirm both RED. THEN extend `classify_line` (lib.rs:206-214) to return `LoopType::ThinWall` and wire `feature_flags[i].is_thin_wall = true` in the construction loop for those walls; confirm both GREEN. **ThinWall predicate:** `classify_line` today maps ALL `is_odd` lines to `GapFill` (lib.rs:207-208). The candidate predicate is `line.is_odd && line.inset_idx == 0 && params.print_thin_walls` → ThinWall (the widened center-line bead of a region thinner than one bead), with deeper odd lines staying GapFill — CONFIRM against the delegated `WallToolPaths.cpp` summary before implementing (see design.md §Open Questions). Note `classify_line` currently takes only `&ExtrusionLine`; threading `print_thin_walls` into it (extra parameter) is part of this step. The classic gate (`medial_axis_enabled`) does NOT apply here — medial-axis is the classic thin-wall mechanism; arachne thin walls come from the WideningBeadingStrategy.
- Precondition: Step 2 complete.
- Postcondition: both new unit tests are green.
- Files allowed to read (with line-range hints when > 300 lines):
  - `modules/core-modules/arachne-perimeters/src/lib.rs` lines 200-220 (classify_line) and 280-310 (construction loop)
  - `modules/core-modules/classic-perimeters/src/lib.rs` lines 760-800 (the ThinWall emission path; classic precedent for the flag shape, NOT for the gate)
- Files allowed to edit (≤ 3):
  - `modules/core-modules/arachne-perimeters/src/lib.rs`
  - `modules/core-modules/arachne-perimeters/tests/arachne_parity_thin_wall_loop_type_tdd.rs` (new)
  - `modules/core-modules/arachne-perimeters/tests/arachne_parity_is_thin_wall_flag_tdd.rs` (new)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/beading/**` (the beading-strategy stack is correct; do not touch)
  - `crates/slicer-runtime/tests/arachne_parity.rs` (untouched until Step 8)
- Expected sub-agent dispatches:
  - "Summarize the ThinWall/odd-line classification in `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp` (the 783-790 region and wherever odd toolpaths are classified); return SUMMARY (≤ 200 words) answering: is a thin-wall bead distinguished by is_odd + inset 0, or another signal?" — purpose: pin the ThinWall predicate before implementing.
  - "Run `cargo test -p arachne-perimeters --test arachne_parity_thin_wall_loop_type_tdd 2>&1 | tee target/test-output.log`; return FACT (pass) or SNIPPETS (first 20 lines of failure)." — purpose: AC-2 green.
  - "Run `cargo test -p arachne-perimeters --test arachne_parity_is_thin_wall_flag_tdd 2>&1 | tee target/test-output.log`; return FACT (pass) or SNIPPETS (first 20 lines of failure)." — purpose: AC-3 green.
- Context cost: S
- Authoritative docs:
  - `docs/02_ir_schemas.md` §1505-1516 — LoopType::ThinWall (delegate)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/WideningBeadingStrategy.cpp:27-77` — thin-wall widening
- Verification:
  - Both new AC-2 and AC-3 unit tests green
- Exit condition: both unit tests were RED before the lib.rs edit and are GREEN after; the ThinWall predicate used matches the delegated OrcaSlicer summary (or the divergence is recorded in design.md §Open Questions resolution).

### Step 4: AC-4 + AC-5 — per-vertex `is_bridge` and `overhang_quartile` from region data

- Task IDs:
  - none
- Objective: TDD. FIRST write `arachne-perimeters/tests/arachne_parity_is_bridge_flag_tdd.rs` (AC-4; fixture uses `SliceRegionViewBuilder::bridge_areas(...)` at fixtures.rs:355) and `arachne_parity_overhang_quartile_tdd.rs` (AC-5; fixture uses the `overhang_quartile_polygons(...)` setter added in Step 1); confirm both RED. THEN, in the construction loop, populate `feature_flags[i].is_bridge` per-vertex via `point_in_any_polygon(pt, region.bridge_areas())`, and populate `path.points[i].overhang_quartile` per-vertex via the band lookup against `region.overhang_quartile_polygons()`; confirm both GREEN. **Do NOT verify via the old substring tests — `arachne_parity_arachne_path_is_bridge_flag_never_set` greps for a source line containing both `is_bridge` and `true` (arachne_parity.rs:477-479), which the real fix (`is_bridge: point_in_any_polygon(...)`) does not produce; it stays red until Step 8 deletes it.**
- Precondition: Step 3 complete; `slicer-core` is a reachable dep and the fixtures setter exists (Step 1).
- Postcondition: both new unit tests are green.
- Files allowed to read (with line-range hints when > 300 lines):
  - `modules/core-modules/arachne-perimeters/src/lib.rs` lines 280-310
  - `crates/slicer-core/src/perimeter_utils.rs:608` (`point_in_any_polygon` signature) and `:316-331` (overhang-band lookup pattern in `expolygon_to_path3d`)
  - `crates/slicer-sdk/src/views.rs:388, 468` (accessor signatures)
  - `crates/slicer-ir/src/slice_ir.rs:1542-1558` (`Point3WithWidth.overhang_quartile` field)
- Files allowed to edit (≤ 3):
  - `modules/core-modules/arachne-perimeters/src/lib.rs`
  - `modules/core-modules/arachne-perimeters/tests/arachne_parity_is_bridge_flag_tdd.rs` (new)
  - `modules/core-modules/arachne-perimeters/tests/arachne_parity_overhang_quartile_tdd.rs` (new)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-core/src/perimeter_utils.rs` (the helper is correct; do not touch)
  - `crates/slicer-sdk/src/views.rs` (the accessor is correct; do not touch)
  - `crates/slicer-runtime/tests/arachne_parity.rs` (untouched until Step 8)
- Expected sub-agent dispatches:
  - "Summarize `crates/slicer-core/src/perimeter_utils.rs:316-331` (the overhang-band lookup pattern in `expolygon_to_path3d`); return SUMMARY ≤ 200 words." — purpose: confirm the canonical lookup shape.
  - "Run `cargo test -p arachne-perimeters --test arachne_parity_is_bridge_flag_tdd 2>&1 | tee target/test-output.log`; return FACT (pass) or SNIPPETS (first 20 lines of failure)." — purpose: AC-4 green.
  - "Run `cargo test -p arachne-perimeters --test arachne_parity_overhang_quartile_tdd 2>&1 | tee target/test-output.log`; return FACT (pass) or SNIPPETS (first 20 lines of failure)." — purpose: AC-5 green.
- Context cost: M (the band lookup is the largest single edit; the bridge lookup is trivial)
- Authoritative docs:
  - `docs/02_ir_schemas.md` §1520-1533 (WallFeatureFlags), §1542-1558 (Point3WithWidth) — delegate SUMMARY
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:675-678` — per-vertex is_bridge
  - `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:2113-2119` — overhang quartile population
- Verification:
  - AC-4 + AC-5 unit tests green
- Exit condition: both unit tests were RED before the lib.rs edit and are GREEN after.

### Step 5: AC-6 + AC-7 — seam-candidate emission + `precise_outer_wall` registration

- Task IDs:
  - none
- Objective: in `run_perimeters`, after the `for wall in walls { output.push_wall_loop(wall)?; }` loop, iterate the outer region and call `generate_sharp_corner_seam_candidates(&region.polygons()[0].contour, region.z(), seam_candidate_angle_threshold_deg)`, then `output.push_seam_candidate(c.position, c.score)?` for each. The seam helper takes a `&slicer_ir::Polygon` (units-space input contour) — NOT `&wall.path` (mm-space `ExtrusionPath3D`); the call shape matches classic's `lib.rs:889-900` (`generate_sharp_corner_seam_candidates(&poly.contour, z, threshold_deg)`). Then verify the test for AC-7 (manifest presence of `precise_outer_wall` and `seam_candidate_angle_threshold_deg` keys) — Step 1 already added the manifest entries, so AC-7 should now be green on the test run. AC-6 needs the new code path; AC-7 just needs Step 1's manifest change to be present.
- Precondition: Step 4 complete; `slicer-core` and the two new config keys are reachable.
- Postcondition: the new unit tests `arachne_parity_seam_candidate_tdd` (AC-6) and `arachne_parity_precise_outer_wall_manifest_tdd` (AC-7) are green. (The old substring tests `arachne_parity_arachne_path_seam_candidate_producer_missing` and `..._precise_outer_wall_not_registered` stay red until Step 8 deletes them.) Write both test files FIRST (RED), then implement (GREEN).
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
  - AC-6 + AC-7 unit tests green
- Exit condition: both unit tests were RED before the lib.rs edit and are GREEN after. (The 10-passed / 5-red full-file count is Step 8's exit condition, not this step's.)

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

### Step 7: Aggregate unit-suite verification — `arachne-perimeters/tests/` all green

- Task IDs:
  - none
- Objective: consolidation gate. The 8 unit-test files were created inside Steps 2-6 (TDD-first per step); this step runs the whole module test suite once and confirms no cross-test interference (shared fixtures, config bleed) before the Step 8 rewrite of `arachne_parity.rs`.
- Precondition: Steps 1-6 complete (all per-step unit tests individually green).
- Postcondition: `cargo test -p arachne-perimeters --tests` shows all 8 test files green in one run.
- Files allowed to read:
  - `target/test-output.log` (via Grep, after the dispatched run)
- Files allowed to edit (≤ 3):
  - none (any failure here is triaged back to the owning step; do not patch tests in this step)
- Files explicitly out-of-bounds for this step:
  - `modules/core-modules/arachne-perimeters/src/lib.rs` (read-only; the implementation is already in from Steps 1-6)
  - the manifests (read-only; Step 1 already set them)
- Expected sub-agent dispatches:
  - "Run `cargo test -p arachne-perimeters --tests 2>&1 | tee target/test-output.log`; return FACT (all test binaries green, count) or SNIPPETS (failing-test detail blocks, ≤ 20 lines each)." — purpose: whole-suite verification.
- Context cost: S (verification only)
- Authoritative docs:
  - none
- OrcaSlicer refs:
  - none
- Verification:
  - All 8 unit-test files green in one `--tests` run
- Exit condition: `cargo test -p arachne-perimeters --tests` exits 0.

### Step 8: Test rewrite — `arachne_parity.rs` rewritten to drive `run_perimeters` natively

- Task IDs:
  - none
- Objective: REWRITE `crates/slicer-runtime/tests/arachne_parity.rs` so the 7 arachne-path red tests drive `ArachnePerimeters::run_perimeters` natively (delegating to the new `arachne-perimeters/tests/*_tdd.rs` files, OR running the same harness inline). The 3 packet-1 stale-doc tests, 4 packet-149 pipeline-config tests, and 1 D-104f test are preserved with their existing predicates (they are manifest-presence / wiring-presence concerns and are correct as-is). The 7 arachne-path substring-matching tests are DELETED (the `arachne-perimeters/tests/*_tdd.rs` files are the canonical coverage; note two of the old tests are structurally un-passable — the boundary-type test hardcodes its own failure at arachne_parity.rs:444-446 and the is_bridge test greps for `is_bridge`+`true` on one source line — so deletion, not repair, is the correct move). This step is sub-agent-dispatched (see Expected sub-agent dispatches) — the test rewrite is a substantial chunk of work and benefits from isolation.
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
- Objective: refine the RATIONALE of `D-104-OVERHANG-QUARTILE-NONE` in `docs/DEVIATION_LOG.md` to "arachne-path-only per-vertex overhang/flag/seam/boundary parity" — the row is ALREADY `Closed 2026-07-03` (DEVIATION_LOG.md:22); do NOT change its Status, only sharpen the rationale text and note packet 148 closed the arachne-path residue; append a one-line record to `docs/14_deviation_audit_history.md`; append the three new config keys (`precise_outer_wall`, `seam_candidate_angle_threshold_deg`, `wall_sequence`) to `docs/15_config_keys_reference.md` §Walls.
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
| Step 1 | S | Cargo.toml (`slicer-core` with `default-features=false`) + 3 manifest keys (incl. `wall_sequence`) + fixtures.rs quartile-band setter |
| Step 2 | S | AC-1 unit test (TDD-first) + single-line conditional in the construction loop |
| Step 3 | M | AC-2/AC-3 unit tests (TDD-first) + `classify_line` ThinWall arm (predicate confirmed via OrcaSlicer dispatch) + per-vertex flag |
| Step 4 | M | AC-4/AC-5 unit tests (TDD-first) + per-vertex is_bridge + overhang-band lookup; largest single edit |
| Step 5 | M | New helper import + post-loop seam emission (input polygon contour, not `wall.path`) + 2 new test files (AC-6, AC-7), TDD-first |
| Step 6 | M | AC-8 beading-stack offset gating in `arachne_params_from_config` + 1 new test file (AC-8 + AC-N2) |
| Step 7 | S | Aggregate `cargo test -p arachne-perimeters --tests` verification only (test files created in Steps 2-6) |
| Step 8 | M | Test rewrite — old substring tests deleted from `arachne_parity.rs` (two are structurally un-passable); sub-agent-dispatched |
| Step 9 | S | Deviation log rationale (row already Closed) + audit history + config keys reference |

Aggregate: M (sum of S+S+M+M+M+M+S+M+S = M-equivalent). No step is L. The packet does not need to split before activation.

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
