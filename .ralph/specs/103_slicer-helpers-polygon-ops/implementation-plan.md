# Implementation Plan: 103_slicer-helpers-polygon-ops

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first (write the failing test before the production change), then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: Add `offset2_ex` / `opening_ex` / `keep_largest_contour_only` to `polygon_ops`

- Task IDs:
  - `T-040` — Port `offset2_ex(polys, -d, +d)` and `opening_ex(polys, d)`
  - `T-044` — Port `keep_largest_contour_only`
- Objective: extend `crates/slicer-core/src/polygon_ops.rs` (EXISTING file — do NOT create new) with the three new `pub fn`s; write `offset2_ex_tdd`, `offset2_ex_collapse_tdd`, `keep_largest_contour_only_tdd`.
- Precondition: workspace builds clean before any edit.
- Postcondition: AC-1, AC-N2, AC-5 verification commands pass.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/polygon_ops.rs` — full file (`wc -l` first; range-read if larger than 300 lines; already exists).
  - `docs/13_slicer_helpers_crate.md` — §Out of Scope only (≈ 20 lines) — confirms per-layer geometry ops belong in `slicer-core`.
  - `docs/01_system_architecture.md` — §"Crate Responsibilities" only.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/polygon_ops.rs` (EXTEND existing)
  - `crates/slicer-core/tests/offset2_ex_tdd.rs` (NEW; also contains `offset2_ex_collapse_tdd` as the negative case OR separate `offset2_ex_collapse_tdd.rs`)
  - `crates/slicer-core/tests/keep_largest_contour_only_tdd.rs` (NEW)
- Files explicitly out-of-bounds for this step:
  - Any `slicer-helpers/src/*.rs` file — wrong crate; see Locked Assumptions.
  - Any other `slicer-core/src/*.rs` file — handled in later steps.
  - Any `slicer-ir` or WIT file.
- Expected sub-agent dispatches:
  - "Summarize OrcaSlicerDocumented/src/libslic3r/ClipperUtils.cpp for `offset2_ex` parameter order and `ClipperSafetyOffset`; return SUMMARY ≤ 100 words."
  - "Run `cargo test -p slicer-core --test offset2_ex_tdd --test offset2_ex_collapse_tdd --test keep_largest_contour_only_tdd`; return FACT pass/fail with assertion text on fail."
- Context cost: `M` (one source file + three new tests; SUMMARY dispatch)
- Authoritative docs:
  - `docs/01_system_architecture.md` — §"Crate Responsibilities".
  - `docs/08_coordinate_system.md` — read full (the offset/area calculations cross the mm↔unit boundary).
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/ClipperUtils.cpp` — delegate SUMMARY only.
- Verification:
  - `cargo test -p slicer-core --test offset2_ex_tdd 2>&1 | tee target/test-output.log` — FACT.
  - `cargo test -p slicer-core --test offset2_ex_collapse_tdd 2>&1 | tee target/test-output.log` — FACT.
  - `cargo test -p slicer-core --test keep_largest_contour_only_tdd 2>&1 | tee target/test-output.log` — FACT.
- Exit condition: AC-1, AC-N2, AC-5 green; `crates/slicer-core/src/polygon_ops.rs` exports the three new `pub fn`s.

### Step 2: Add `medial_axis` + `ThickPolyline` / `Point2WithWidth` / `variable_width`

- Task IDs:
  - `T-041` — Port `medial_axis(min, max, &out)`
  - `T-042` — Add `ThickPolyline`, `Point2WithWidth`, `variable_width` converter
- Objective: introduce the IR types in `slicer-ir`, port `medial_axis` to a new file in `slicer-core`, update WIT; write `medial_axis_rectangle_tdd`, `medial_axis_degenerate_input_tdd`, `thick_polyline_variable_width_tdd`.
- Precondition: Step 1 exit condition met; `cargo check --workspace --all-targets` clean.
- Postcondition: AC-2, AC-N1, AC-3 verification commands pass; `cargo xtask build-guests --check` reports no STALE.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-ir/src/slice_ir.rs` — range-read by `rg -n 'ExtrusionPath3D|Point3WithWidth|ExtrusionRole|CURRENT_SLICE_IR_SCHEMA_VERSION'` then ±40 lines.
  - `crates/slicer-schema/wit/deps/ir-types.wit` — full (≤ 200 lines expected).
  - `docs/08_coordinate_system.md` — full.
- Files allowed to edit (≤ 3, plus 1 single-line compile-dependency edit):
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-core/src/medial_axis.rs` (NEW)
  - `crates/slicer-schema/wit/deps/ir-types.wit`
  - `crates/slicer-core/src/lib.rs` — exempted single-line addition: `pub mod medial_axis;` is required for this step's test to compile against the new module. No other edits to `lib.rs` are permitted in this step.
- Files explicitly out-of-bounds for this step:
  - `slicer-helpers/src/**` — wrong crate.
  - Other `slicer-core` source files (`polygon_tree`, `geometry`).
- Expected sub-agent dispatches:
  - "Summarize OrcaSlicerDocumented/src/libslic3r/Geometry/MedialAxis.cpp (or Polygon.cpp if MedialAxis lives there) for the `medial_axis(min, max, &out)` parameter contract and degenerate-input handling; return SUMMARY ≤ 150 words, no code."
  - "Run `cargo test -p slicer-ir --test thick_polyline_variable_width_tdd`; return FACT pass/fail."
  - "Run `cargo test -p slicer-core --test medial_axis_rectangle_tdd --test medial_axis_degenerate_input_tdd`; return FACT pass/fail with assertion text on fail."
  - "Run `cargo xtask build-guests --check`; return FACT clean / STALE list."
- Context cost: `M` (largest step — three crates touched, longest LOC delta, two SUMMARY dispatches)
- Authoritative docs:
  - `docs/02_ir_schemas.md` — delegate SUMMARY for §"Variable-width geometry".
  - `docs/03_wit_and_manifest.md` — read §"WIT/Type Changes Checklist".
  - `CLAUDE.md` — §"WIT/Type Changes Checklist" and §"Guest WASM Staleness".
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Geometry/MedialAxis.cpp` or `Polygon.cpp` — delegate SUMMARY.
- Verification:
  - `cargo test -p slicer-ir --test thick_polyline_variable_width_tdd 2>&1 | tee target/test-output.log` — FACT.
  - `cargo test -p slicer-core --test medial_axis_rectangle_tdd 2>&1 | tee target/test-output.log` — FACT.
  - `cargo test -p slicer-core --test medial_axis_degenerate_input_tdd 2>&1 | tee target/test-output.log` — FACT.
  - `cargo build --tests --workspace 2>&1 | tee target/test-output.log` — FACT (catches WIT type identity break).
  - `cargo xtask build-guests --check` — must report no STALE entries (rebuild if needed).
- Exit condition: AC-2, AC-N1, AC-3 green; `CURRENT_SLICE_IR_SCHEMA_VERSION` bumped additively; no STALE guests.

### Step 3: Add `polygon_tree` hole/contour containment + tree builder

- Task IDs:
  - `T-043` — Port hole/contour containment + tree-builder
- Objective: implement `PolygonTreeNode` + `build_polygon_tree` in a new file under `slicer-core`; write `polygon_tree_tdd`.
- Precondition: Step 2 exit condition met; `cargo check --workspace --all-targets` clean.
- Postcondition: AC-4 verification command passes.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/lib.rs` — current module declarations.
  - `docs/specs/perimeter-modules-orca-parity-roadmap.md` — read T-043 row only.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/polygon_tree.rs` (NEW)
  - `crates/slicer-core/src/lib.rs` (add `pub mod polygon_tree;`)
  - `crates/slicer-core/tests/polygon_tree_tdd.rs` (NEW)
- Files explicitly out-of-bounds for this step:
  - `slicer-helpers/src/**` — wrong crate.
  - Any other source file.
- Expected sub-agent dispatches:
  - "Summarize OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1727-1779 for the hole/contour containment + child-ordering algorithm; return SUMMARY ≤ 150 words, no code."
  - "Run `cargo test -p slicer-core --test polygon_tree_tdd`; return FACT pass/fail."
- Context cost: `S` (one new source file, one new test, one mod declaration; SUMMARY dispatch)
- Authoritative docs:
  - `docs/specs/perimeter-modules-orca-parity-roadmap.md` — T-043 row only.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1727-1779` — delegate SUMMARY.
- Verification:
  - `cargo test -p slicer-core --test polygon_tree_tdd 2>&1 | tee target/test-output.log` — FACT.
- Exit condition: AC-4 green; `build_polygon_tree` returns deterministically-ordered children (ascending source index per parent).

### Step 4: Promote ray ops from `arachne-perimeters` to `slicer-core::geometry` with OrcaSlicer-faithful API

- Task IDs:
  - `T-045` — Promote `ray_to_polygons`, `closest_point_on_segment`, `closest_point_on_polygons` with redesigned API
- Objective: create `crates/slicer-core/src/geometry.rs` with OrcaSlicer-faithful typed API (see Locked Assumptions in design.md); replace the local definitions in `arachne-perimeters/src/lib.rs` with `use slicer_core::geometry::*`; migrate the `width_at_point` call site at `~line 435`; ensure all existing `arachne-perimeters` tests stay green (AC-7).
- Precondition: Step 3 exit condition met; `cargo check --workspace --all-targets` clean; `rg -n 'pub struct Vec2' crates/slicer-ir/src/` returns empty.
- Postcondition: AC-6 and AC-7 verification commands pass; `arachne-perimeters` existing tests remain green.
- Files allowed to read (with line-range hints when > 300 lines):
  - `modules/core-modules/arachne-perimeters/src/lib.rs` — range-read lines 420–560 (the `width_at_point` call at `~435`, the existing ray ops: `nearest_point_on_polygons` @ ~443, `ray_to_polygons` @ ~468, the private `Ray` struct + `ray_segment_intersect` @ ~500–530, `point_to_segment_nearest` @ ~535) — purpose: understand current signatures and call site to migrate.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/geometry.rs` (NEW) — implement `Vec2`, `Ray`, `ClosestPoint`, `RayHit`, `point_to_segment_distance_squared`, `closest_point_on_segment`, `closest_point_on_polygons`, `ray_to_polygons`.
  - `crates/slicer-core/src/lib.rs` (add `pub mod geometry;`)
  - `modules/core-modules/arachne-perimeters/src/lib.rs` — delete local ray op definitions; add `use slicer_core::geometry::*`; migrate `width_at_point` call site to use `Ray { origin: *point, direction: Vec2 { x: dir_x, y: dir_y } }` and `.map(|hit| hit.distance).unwrap_or(0.0)` with a comment: "legacy: when far boundary not found, width is just near_dist — documented intent preserved during promotion."
- Files explicitly out-of-bounds for this step:
  - `slicer-helpers/src/**` — wrong crate.
  - `classic-perimeters/src/lib.rs` (no consumer change there).
  - Any `slicer-ir` file (no IR change in this step).
- Expected sub-agent dispatches:
  - "Find all callers of `ray_to_polygons`, `nearest_point_on_polygons`, `point_to_segment_nearest` across the workspace; return LOCATIONS ≤ 20 entries." — confirm only one consumer before deleting local definitions.
  - "Run `cargo test -p arachne-perimeters --tests`; return FACT pass/fail with failing-test names if any." — AC-7 caller-migration regression.
  - "Run `cargo test -p slicer-core --tests`; return FACT pass/fail." — AC-6 geometry exports.
- Context cost: `S` (one new file, two thin edits; no OrcaSlicer dispatch needed)
- Authoritative docs:
  - design.md §Locked Assumptions — API pattern contract.
- OrcaSlicer refs:
  - None for this step (API shape established by Locked Assumptions, not OrcaSlicer source).
- Verification:
  - `! rg -q '^fn (ray_to_polygons|nearest_point_on_polygons|point_to_segment_nearest)' modules/core-modules/arachne-perimeters/src/lib.rs` — exit 0 = local definitions removed.
  - `rg -q 'pub fn ray_to_polygons\(ray: &Ray.*Option<RayHit>' crates/slicer-core/src/geometry.rs && rg -q 'pub struct Vec2' crates/slicer-core/src/geometry.rs && rg -q 'pub fn closest_point_on_segment' crates/slicer-core/src/geometry.rs` — AC-6 signature check.
  - `cargo test -p arachne-perimeters 2>&1 | tee target/test-output.log` — FACT pass (AC-7).
- Exit condition: AC-6 and AC-7 green; all `arachne-perimeters` tests pass; only one consumer (`arachne-perimeters`) of the ray ops in the workspace; `width_at_point` call site uses `unwrap_or(0.0)` with documented comment.

### Step 5: Doc impact landing

- Task IDs:
  - Doc impact for `T-040` through `T-045`: `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`, `docs/DEVIATION_LOG.md`.
- Objective: add documentation sections per the Doc Impact Statement; record schema-bump rationale; add `D-103-API-PARITY-UPGRADE` entry to deviation log.
- Precondition: Step 4 exit condition met.
- Postcondition: all four Doc Impact Statement greps return hits.
- Files allowed to read (with line-range hints when > 300 lines):
  - `docs/01_system_architecture.md` — range-read §"Crate Responsibilities" by `rg -n 'slicer-core|crate.*responsib' docs/01_system_architecture.md` then ±20 lines.
  - `docs/02_ir_schemas.md` — range-read by `rg -n 'Schema Versioning|Variable-width'` then ±30 lines.
  - `docs/DEVIATION_LOG.md` — read the last 30 lines to align entry format.
- Files allowed to edit (≤ 3):
  - `docs/01_system_architecture.md`
  - `docs/02_ir_schemas.md`
  - `docs/DEVIATION_LOG.md` (add `D-103-API-PARITY-UPGRADE` entry)
- Files explicitly out-of-bounds for this step:
  - Any source file (no further code edits).
  - `docs/13_slicer_helpers_crate.md` — this doc already carves per-layer geometry out of `slicer-helpers`; no update needed for P103.
- Expected sub-agent dispatches:
  - "For each grep in the Doc Impact Statement, run `rg -q` on the listed path; return FACT pass/fail per grep."
- Context cost: `S` (three doc edits)
- Authoritative docs:
  - The three files being edited.
- OrcaSlicer refs:
  - None.
- Verification:
  - `rg -q 'offset2_ex|medial_axis|polygon_tree|keep_largest_contour_only' docs/01_system_architecture.md` — exit 0. *(Corrected 2026-07-02: the original command escaped the pipes as `\|`, which rg treats as a literal `|` character, so it could never match.)*
  - `rg -q 'ThickPolyline.*Point2WithWidth' docs/02_ir_schemas.md` — exit 0.
  - `rg -q 'ThickPolyline.*additive' docs/02_ir_schemas.md` — exit 0.
  - `rg -q 'D-103-API-PARITY-UPGRADE' docs/DEVIATION_LOG.md` — exit 0.
- Exit condition: all four Doc Impact greps pass.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | One source + three new tests; one SUMMARY dispatch. |
| Step 2 | M | Three crates touched; longest LOC delta; two SUMMARY dispatches; guest-WASM rebuild gate. |
| Step 3 | S | One new source + one new test + one mod declaration. |
| Step 4 | S | One new file (`geometry.rs` with OrcaSlicer-faithful API), two thin edits (lib.rs mod declaration, call-site migration in `arachne-perimeters`). |
| Step 5 | S | Three doc edits (`docs/01_system_architecture.md`, `docs/02_ir_schemas.md`, `docs/DEVIATION_LOG.md`). |

Aggregate context cost: `M`. No single step is `L`. Per-step file edit count never exceeds 3 (Step 2's "4th file" is the `pub mod` declaration, which is a single-line addition justified by compile dependency on the same step's source file).

## Packet Completion Gate

- All five steps complete; each step's exit condition met.
- AC-1, AC-2, AC-3, AC-4, AC-5, AC-6, AC-7, AC-N1, AC-N2 verification commands all return PASS via worker dispatch.
- `cargo check --workspace --all-targets` clean.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo xtask build-guests --check` reports no STALE guests.
- `docs/07_implementation_status.md` updated for each T-040..T-045 entry — via worker dispatch.
- `packet.spec.md` ready to move from `status: draft` → `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` and confirm each returns PASS (now includes AC-7).
- Confirm the three gate commands in `packet.spec.md` §Verification are green.
- Record schema-bump direction (4.1→4.2 or 4.2→4.3 depending on packet 100 sequencing) in the closure log.
- Confirm `D-103-API-PARITY-UPGRADE` entry is present in `docs/DEVIATION_LOG.md`.
- Record any remaining packet-local risk in `.ralph/specs/103_slicer-helpers-polygon-ops/closure-log.md` (likely candidates: medial_axis tolerance scaling rule for non-1mm fixtures; clarifying whether AC-1's join tolerance is calibrated tight enough for Phase 5/6 thin-wall usage; confirming `unwrap_or(0.0)` at `arachne-perimeters:~435` is semantically correct for all caller states).
- Confirm the implementer's peak context usage stayed under 70%. If Step 2 pushed it higher, log it as evidence for splitting `medial_axis` into its own packet in similar future work.
