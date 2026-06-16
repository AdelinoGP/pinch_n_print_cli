# Implementation Plan: 101_slicer-helpers-polygon-ops

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
- Objective: extend `crates/slicer-helpers/src/polygon_ops.rs` with the three new `pub fn`s; write `offset2_ex_tdd`, `offset2_ex_collapse_tdd`, `keep_largest_contour_only_tdd`.
- Precondition: workspace builds clean before any edit.
- Postcondition: AC-1, AC-N2, AC-5 verification commands pass.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-helpers/src/polygon_ops.rs` — full file (≤ 300 lines expected; `wc -l` first; range-read if larger).
  - `docs/13_slicer_helpers_crate.md` — full file (≤ 250 lines).
- Files allowed to edit (≤ 3):
  - `crates/slicer-helpers/src/polygon_ops.rs`
  - `crates/slicer-helpers/tests/offset2_ex_tdd.rs` (NEW; also contains `offset2_ex_collapse_tdd` as the negative case OR new file `offset2_ex_collapse_tdd.rs`)
  - `crates/slicer-helpers/tests/keep_largest_contour_only_tdd.rs` (NEW)
- Files explicitly out-of-bounds for this step:
  - Any other `slicer-helpers/src/*.rs` file — handled in later steps.
  - Any `slicer-ir` or WIT file.
- Expected sub-agent dispatches:
  - "Summarize OrcaSlicerDocumented/src/libslic3r/ClipperUtils.cpp for `offset2_ex` parameter order and `ClipperSafetyOffset`; return SUMMARY ≤ 100 words."
  - "Run `cargo test -p slicer-helpers --test offset2_ex_tdd --test offset2_ex_collapse_tdd --test keep_largest_contour_only_tdd`; return FACT pass/fail with assertion text on fail."
- Context cost: `M` (one source file + three new tests; SUMMARY dispatch)
- Authoritative docs:
  - `docs/13_slicer_helpers_crate.md` — read full.
  - `docs/08_coordinate_system.md` — read full (the offset/area calculations cross the mm↔unit boundary).
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/ClipperUtils.cpp` — delegate SUMMARY only.
- Verification:
  - `cargo test -p slicer-helpers --test offset2_ex_tdd 2>&1 | tee target/test-output.log` — FACT.
  - `cargo test -p slicer-helpers --test offset2_ex_collapse_tdd 2>&1 | tee target/test-output.log` — FACT.
  - `cargo test -p slicer-helpers --test keep_largest_contour_only_tdd 2>&1 | tee target/test-output.log` — FACT.
- Exit condition: AC-1, AC-N2, AC-5 green; `polygon_ops.rs` exports the three new `pub fn`s.

### Step 2: Add `medial_axis` + `ThickPolyline` / `Point2WithWidth` / `variable_width`

- Task IDs:
  - `T-041` — Port `medial_axis(min, max, &out)`
  - `T-042` — Add `ThickPolyline`, `Point2WithWidth`, `variable_width` converter
- Objective: introduce the IR types in `slicer-ir`, port `medial_axis` to a new file in `slicer-helpers`, update WIT; write `medial_axis_rectangle_tdd`, `medial_axis_degenerate_input_tdd`, `thick_polyline_variable_width_tdd`.
- Precondition: Step 1 exit condition met; `cargo check --workspace --all-targets` clean.
- Postcondition: AC-2, AC-N1, AC-3 verification commands pass; `cargo xtask build-guests --check` reports no STALE.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-ir/src/slice_ir.rs` — range-read by `rg -n 'ExtrusionPath3D|Point3WithWidth|ExtrusionRole|CURRENT_SLICE_IR_SCHEMA_VERSION'` then ±40 lines.
  - `crates/slicer-schema/wit/deps/ir-types.wit` — full (≤ 200 lines expected).
  - `docs/08_coordinate_system.md` — full.
- Files allowed to edit (≤ 3):
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-helpers/src/medial_axis.rs` (NEW)
  - `crates/slicer-schema/wit/deps/ir-types.wit`
- Files explicitly out-of-bounds for this step:
  - Other `slicer-helpers` source files (`polygon_ops`, `polygon_tree`, `geometry`).
  - `slicer-helpers/src/lib.rs` — module declaration added in Step 3 (right before introducing polygon_tree + geometry).
  - Wait — `medial_axis` module declaration is needed for Step 2's test to even compile. Add the single `pub mod medial_axis;` line in `slicer-helpers/src/lib.rs` here. Treat it as a 4th file allowed for this step due to compile dependency.
- Expected sub-agent dispatches:
  - "Summarize OrcaSlicerDocumented/src/libslic3r/Geometry/MedialAxis.cpp (or Polygon.cpp if MedialAxis lives there) for the `medial_axis(min, max, &out)` parameter contract and degenerate-input handling; return SUMMARY ≤ 150 words, no code."
  - "Run `cargo test -p slicer-ir --test thick_polyline_variable_width_tdd`; return FACT pass/fail."
  - "Run `cargo test -p slicer-helpers --test medial_axis_rectangle_tdd --test medial_axis_degenerate_input_tdd`; return FACT pass/fail with assertion text on fail."
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
  - `cargo test -p slicer-helpers --test medial_axis_rectangle_tdd 2>&1 | tee target/test-output.log` — FACT.
  - `cargo test -p slicer-helpers --test medial_axis_degenerate_input_tdd 2>&1 | tee target/test-output.log` — FACT.
  - `cargo build --tests --workspace 2>&1 | tee target/test-output.log` — FACT (catches WIT type identity break).
  - `cargo xtask build-guests --check` — must report no STALE entries (rebuild if needed).
- Exit condition: AC-2, AC-N1, AC-3 green; `CURRENT_SLICE_IR_SCHEMA_VERSION` bumped additively; no STALE guests.

### Step 3: Add `polygon_tree` hole/contour containment + tree builder

- Task IDs:
  - `T-043` — Port hole/contour containment + tree-builder
- Objective: implement `PolygonTreeNode` + `build_polygon_tree` in a new file; write `polygon_tree_tdd`.
- Precondition: Step 2 exit condition met; `cargo check --workspace --all-targets` clean.
- Postcondition: AC-4 verification command passes.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-helpers/src/lib.rs` — current module declarations.
  - `docs/specs/perimeter-modules-orca-parity-roadmap.md` — read T-043 row only.
- Files allowed to edit (≤ 3):
  - `crates/slicer-helpers/src/polygon_tree.rs` (NEW)
  - `crates/slicer-helpers/src/lib.rs` (add `pub mod polygon_tree;`)
  - `crates/slicer-helpers/tests/polygon_tree_tdd.rs` (NEW)
- Files explicitly out-of-bounds for this step:
  - Any other source file.
- Expected sub-agent dispatches:
  - "Summarize OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1727-1779 for the hole/contour containment + child-ordering algorithm; return SUMMARY ≤ 150 words, no code."
  - "Run `cargo test -p slicer-helpers --test polygon_tree_tdd`; return FACT pass/fail."
- Context cost: `S` (one new source file, one new test, one mod declaration; SUMMARY dispatch)
- Authoritative docs:
  - `docs/specs/perimeter-modules-orca-parity-roadmap.md` — T-043 row only.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1727-1779` — delegate SUMMARY.
- Verification:
  - `cargo test -p slicer-helpers --test polygon_tree_tdd 2>&1 | tee target/test-output.log` — FACT.
- Exit condition: AC-4 green; `build_polygon_tree` returns deterministically-ordered children (ascending source index per parent).

### Step 4: Promote ray ops from `arachne-perimeters` to `slicer-helpers::geometry`

- Task IDs:
  - `T-045` — Promote `ray_to_polygons`, `nearest_point_on_polygons`, `point_to_segment_nearest`
- Objective: create `crates/slicer-helpers/src/geometry.rs` with verbatim-equivalent implementations; replace the local definitions in `arachne-perimeters/src/lib.rs` with `use` imports; ensure all existing `arachne-perimeters` tests stay green.
- Precondition: Step 3 exit condition met; `cargo check --workspace --all-targets` clean.
- Postcondition: AC-6 verification command passes; `arachne-perimeters` existing tests remain green.
- Files allowed to read (with line-range hints when > 300 lines):
  - `modules/core-modules/arachne-perimeters/src/lib.rs` — range-read lines 326–466 (the existing ray ops) to confirm signature preservation.
- Files allowed to edit (≤ 3):
  - `crates/slicer-helpers/src/geometry.rs` (NEW)
  - `crates/slicer-helpers/src/lib.rs` (add `pub mod geometry;`)
  - `modules/core-modules/arachne-perimeters/src/lib.rs` (delete locals + add `use`)
- Files explicitly out-of-bounds for this step:
  - `classic-perimeters/src/lib.rs` (no consumer change there).
  - Any `slicer-ir` file (no IR change in this step).
- Expected sub-agent dispatches:
  - "Find all callers of `ray_to_polygons`, `nearest_point_on_polygons`, `point_to_segment_nearest` across the workspace; return LOCATIONS ≤ 20 entries."
  - "Run `cargo test -p arachne-perimeters --tests`; return FACT pass/fail with failing-test names if any."
  - "Run `cargo test -p slicer-helpers --tests`; return FACT pass/fail."
- Context cost: `S` (one file moved, two thin edits)
- Authoritative docs:
  - None beyond what was read in earlier steps.
- OrcaSlicer refs:
  - None.
- Verification:
  - `! rg -q '^fn (ray_to_polygons|nearest_point_on_polygons|point_to_segment_nearest)' modules/core-modules/arachne-perimeters/src/lib.rs` — exit 0 = clean.
  - `rg -q 'pub fn (ray_to_polygons|nearest_point_on_polygons|point_to_segment_nearest)' crates/slicer-helpers/src/geometry.rs` — exit 0 = exports present.
  - `cargo test -p arachne-perimeters --tests 2>&1 | tee target/test-output.log` — FACT pass.
- Exit condition: AC-6 green; all `arachne-perimeters` tests pass; only one consumer (`arachne-perimeters`) of the ray ops in the workspace.

### Step 5: Doc impact landing

- Task IDs:
  - Doc impact for `T-040` through `T-045`: `docs/13_slicer_helpers_crate.md` and `docs/02_ir_schemas.md`.
- Objective: add documentation sections per the Doc Impact Statement; record schema-bump rationale.
- Precondition: Step 4 exit condition met.
- Postcondition: all three Doc Impact Statement greps return hits.
- Files allowed to read (with line-range hints when > 300 lines):
  - `docs/13_slicer_helpers_crate.md` — full.
  - `docs/02_ir_schemas.md` — range-read by `rg -n 'Schema Versioning|Variable-width'` then ±30 lines.
- Files allowed to edit (≤ 3):
  - `docs/13_slicer_helpers_crate.md`
  - `docs/02_ir_schemas.md`
- Files explicitly out-of-bounds for this step:
  - Any source file (no further code edits).
- Expected sub-agent dispatches:
  - "For each grep in the Doc Impact Statement, run `rg -q` on the listed path; return FACT pass/fail per grep."
- Context cost: `S` (two doc edits)
- Authoritative docs:
  - The two files being edited.
- OrcaSlicer refs:
  - None.
- Verification:
  - `rg -q 'medial_axis.*ExPolygon.*ThickPolyline' docs/13_slicer_helpers_crate.md` — exit 0.
  - `rg -q 'ThickPolyline.*Point2WithWidth' docs/02_ir_schemas.md` — exit 0.
  - `rg -q 'ThickPolyline.*additive' docs/02_ir_schemas.md` — exit 0.
- Exit condition: all Doc Impact greps pass.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | One source + three new tests; one SUMMARY dispatch. |
| Step 2 | M | Three crates touched; longest LOC delta; two SUMMARY dispatches; guest-WASM rebuild gate. |
| Step 3 | S | One new source + one new test + one mod declaration. |
| Step 4 | S | One file moved verbatim; two thin edits. |
| Step 5 | S | Two doc edits. |

Aggregate context cost: `M`. No single step is `L`. Per-step file edit count never exceeds 3 (Step 2's "4th file" is the `pub mod` declaration, which is a single-line addition justified by compile dependency on the same step's source file).

## Packet Completion Gate

- All five steps complete; each step's exit condition met.
- AC-1, AC-2, AC-3, AC-4, AC-5, AC-6, AC-N1, AC-N2 verification commands all return PASS via worker dispatch.
- `cargo check --workspace --all-targets` clean.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo xtask build-guests --check` reports no STALE guests.
- `docs/07_implementation_status.md` updated for each T-040..T-045 entry — via worker dispatch.
- `packet.spec.md` ready to move from `status: draft` → `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` and confirm each returns PASS.
- Confirm the three gate commands in `packet.spec.md` §Verification are green.
- Record schema-bump direction (4.1→4.2 or 4.2→4.3 depending on packet 100 sequencing) in the closure log.
- Record any remaining packet-local risk in `.ralph/specs/101_slicer-helpers-polygon-ops/closure-log.md` (likely candidates: medial_axis tolerance scaling rule for non-1mm fixtures; clarifying whether AC-1's join tolerance is calibrated tight enough for Phase 5/6 thin-wall usage).
- Confirm the implementer's peak context usage stayed under 70%. If Step 2 pushed it higher, log it as evidence for splitting `medial_axis` into its own packet in similar future work.
