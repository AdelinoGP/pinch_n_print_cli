# Implementation Plan: 105_classic-spacing-fill-mmu

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first (write the failing test before the production change), then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: T-P96-A0 — OrcaSlicer-source investigation one-pager

- Task IDs:
  - `T-P96-A0` — Produce `docs/specs/orca-mmu-perimeter-investigation.md`
- Objective: dispatch the OrcaSlicer SUMMARY for the MMU per-color outer-wall fragmentation + bisector tie-break rule; author a one-pager that cites file:line references and states the tie-break rule used by Step 3.
- Precondition: workspace builds clean.
- Postcondition: T-P96-A0 deliverable grep passes; one-pager committed.
- Files allowed to read (with line-range hints when > 300 lines):
  - `docs/adr/0013-mmu-per-color-outer-wall-fragmentation.md` — read full.
  - `docs/specs/perimeter-modules-orca-parity-roadmap.md` — range-read "Inherited from P96" section.
- Files allowed to edit (≤ 3):
  - `docs/specs/orca-mmu-perimeter-investigation.md` (NEW)
- Files explicitly out-of-bounds for this step:
  - All source files.
- Expected sub-agent dispatches:
  - "Summarize OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp + PerimeterGenerator.cpp per-color branches for the MMU outer-wall fragmentation + bisector tie-break rule; return SUMMARY ≤ 200 words. No code. Specifically: which side owns the shared bisector edge when two adjacent cells of different colors share it? If the rule is deterministic, name it (e.g. lower color-ID, paint-order ID, polygon-index ordering). If non-deterministic or opaque, say so."
- Context cost: `S` (one new doc; SUMMARY-only dispatch)
- Authoritative docs: see Files allowed to read.
- OrcaSlicer refs: `MultiMaterialSegmentation.cpp`, `PerimeterGenerator.cpp` per-color branches (delegate SUMMARY).
- Verification:
  - `rg -q 'tie-break.*lower color-ID\|tie-break.*matching OrcaSlicer' docs/specs/orca-mmu-perimeter-investigation.md` — exit 0.
- Exit condition: one-pager exists with file:line citations + stated tie-break rule.

### Step 2: T-062b — IR enum additions + `bisector_edge_skip_mask` field

- Task IDs:
  - `T-062b` — Add `LoopType::GapFill` + `ExtrusionRole::GapFill` variants
  - `T-P96-C0` — Resurrect `SlicedRegion.bisector_edge_skip_mask` (IR field only — host populator in Step 3)
- Objective: extend `LoopType` and `ExtrusionRole` with `GapFill` arm, mark both `#[non_exhaustive]`, add `pub bisector_edge_skip_mask: Vec<Vec<bool>>` on `SlicedRegion`, bump schema 4.2.0 → 4.3.0; mirror in WIT + host populator + view accessor. Update every exhaustive match site in the workspace to add the new arm.
- Precondition: Step 1 exit condition met; `cargo check --workspace --all-targets` clean.
- Postcondition: AC-5 IR-field grep passes; `cargo xtask build-guests --check` no STALE; all exhaustive matches compile.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-ir/src/slice_ir.rs` — range-read by `rg -n 'LoopType|ExtrusionRole|SlicedRegion|CURRENT_SLICE_IR_SCHEMA_VERSION'`.
  - `crates/slicer-schema/wit/deps/ir-types.wit` — full file.
  - `crates/slicer-wasm-host/src/host.rs` — range-read by `rg -n 'SliceRegionData|sliced_region_to_data'`.
  - `crates/slicer-sdk/src/views.rs` — range-read by `rg -n 'fn bridge_areas\|fn nonplanar_surface'`.
- Files allowed to edit (≤ 3 per sub-step):
  - 2a (IR + WIT): `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-schema/wit/deps/ir-types.wit`, `crates/slicer-core/src/algos/prepass_slice.rs` (one-line `Vec::new()` initializer in struct-literal).
  - 2b (host + view): `crates/slicer-wasm-host/src/host.rs`, `crates/slicer-sdk/src/views.rs`.
  - 2c (downstream match arms): the LOCATIONS dispatch reports specific files; expect `modules/core-modules/part-cooling/src/lib.rs`, GCodeEmit role priority table, possibly `path-optimization-default`. Each consumer gets a 1-3 line arm addition.
- Files explicitly out-of-bounds for this step:
  - Any perimeter module `lib.rs` (Step 4+ work).
  - `slicer-helpers` (Step 3+ work).
  - `paint_segmentation/` (Step 3 work).
- Expected sub-agent dispatches:
  - "Find all exhaustive `match` blocks on `LoopType` across the workspace; return LOCATIONS ≤ 20 entries."
  - "Find all exhaustive `match` blocks on `ExtrusionRole` across the workspace; return LOCATIONS ≤ 20 entries."
  - "Run `cargo build --tests --workspace`; return FACT (pass/fail) — catches WIT type identity break + missing match arms."
  - "Run `cargo xtask build-guests --check`; return FACT (clean / STALE list)."
- Context cost: `M` (three crates + downstream match arms; two LOCATIONS dispatches)
- Authoritative docs:
  - `docs/02_ir_schemas.md` — delegate SUMMARY for `LoopType`, `ExtrusionRole`, `SlicedRegion`.
  - `docs/03_wit_and_manifest.md` — read §"WIT/Type Changes Checklist".
  - `CLAUDE.md` — §"WIT/Type Changes Checklist" + §"Guest WASM Staleness".
- OrcaSlicer refs: none.
- Verification:
  - `rg -q 'pub bisector_edge_skip_mask: Vec<Vec<bool>>' crates/slicer-ir/src/slice_ir.rs` — exit 0.
  - `rg -q 'pub const CURRENT_SLICE_IR_SCHEMA_VERSION: SemVer = SemVer \{ major: 4, minor: 3, patch: 0' crates/slicer-ir/src/slice_ir.rs` — exit 0.
  - `cargo build --tests --workspace 2>&1 | tee target/test-output.log` — FACT.
  - `cargo xtask build-guests --check` — no STALE.
- Exit condition: IR additions present, workspace compiles end-to-end, no STALE guests.

### Step 3: T-P96-C0 host populator — `compute_bisector_edge_skip_mask`

- Task IDs:
  - `T-P96-C0` (host populator half; IR half landed in Step 2)
- Objective: implement `compute_bisector_edge_skip_mask` in `crates/slicer-core/src/algos/paint_segmentation/` and call it at paint-segmentation commit; mask uses tie-break rule from Step 1 one-pager (default "lower color-ID owns" if Step 1 didn't surface a more specific rule).
- Precondition: Step 2 exit condition met.
- Postcondition: AC-5 host-populator test passes; AC-N3 (single-color all-false) passes.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/algos/paint_segmentation/` — `wc -l` each file; range-read by `rg -n 'voronoi|cell_neighbor|bisector' crates/slicer-core/src/algos/paint_segmentation/`.
  - `docs/specs/orca-mmu-perimeter-investigation.md` (just authored in Step 1).
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/algos/paint_segmentation/<chosen>.rs` (the file where cell construction happens; likely `voronoi_graph.rs` or `mod.rs`).
  - `crates/slicer-core/tests/paint_segmentation_bisector_mask_tdd.rs` (NEW).
  - Possibly one other file in the same directory for the call site if separation is needed.
- Files explicitly out-of-bounds for this step:
  - Perimeter modules.
  - `slicer-helpers`.
  - Other `slicer-core` algos.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-core --test paint_segmentation_bisector_mask_tdd`; return FACT pass/fail + assertion text on fail."
  - "Find the call site in paint_segmentation that constructs the final per-cell SlicedRegion polygons; return LOCATIONS ≤ 5 entries."
- Context cost: `M` (algorithm port; new test)
- Authoritative docs:
  - `docs/specs/orca-mmu-perimeter-investigation.md` (Step 1 output).
  - `docs/adr/0013-mmu-per-color-outer-wall-fragmentation.md`.
- OrcaSlicer refs: cited in Step 1; no new dispatch.
- Verification:
  - `cargo test -p slicer-core --test paint_segmentation_bisector_mask_tdd 2>&1 | tee target/test-output.log` — FACT.
- Exit condition: AC-5 + AC-N3 host-populator portion green; mask outer Vec aligns with `polygons` Vec; inner Vec[j] aligns with `points[j]..points[(j+1)%len]` edge.

### Step 4: T-050/T-051/T-052/T-053 — Spacing model + outer/inner widths

- Task IDs:
  - `T-050` — Flow math in `slicer-helpers::flow`
  - `T-051` — outer/inner widths replacing single line_width
  - `T-052` — `ext_perimeter_spacing2` + `perimeter_spacing` arithmetic
  - `T-053` — `precise_outer_wall` mode (gated)
- Objective: add `slicer_helpers::flow` module; register the four config keys; rewrite the wall-inset computation in both perimeter modules to use distinct outer/inner widths and the canonical spacing formula.
- Precondition: Step 3 exit condition met; `cargo check --workspace --all-targets` clean.
- Postcondition: AC-1 verification command passes.
- Files allowed to read (with line-range hints when > 300 lines):
  - Both perimeter modules' `lib.rs` — range-read the `run_perimeters` body and the wall-inset loop.
  - `docs/13_slicer_helpers_crate.md` — full.
- Files allowed to edit (≤ 3 per sub-step):
  - 4a (helpers): `crates/slicer-helpers/src/flow.rs` (NEW), `crates/slicer-helpers/src/lib.rs` (mod declaration), `crates/slicer-helpers/tests/flow_tdd.rs` (NEW; spacing-formula unit test).
  - 4b (manifests): `modules/core-modules/classic-perimeters/classic-perimeters.toml`, `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`, `docs/15_config_keys_reference.md`.
  - 4c (consumers): `modules/core-modules/classic-perimeters/src/lib.rs`, `modules/core-modules/arachne-perimeters/src/lib.rs`, `crates/slicer-runtime/tests/integration/outer_inner_width_and_spacing_tdd.rs` (NEW).
- Files explicitly out-of-bounds for this step:
  - `slicer-ir` (Step 2 closed the IR work).
  - Thin-wall / gap-fill code paths (Step 6).
  - Wall-sequence code paths (Step 5).
- Expected sub-agent dispatches:
  - "Summarize OrcaSlicerDocumented/src/libslic3r/Flow.cpp for `Flow::new_from_width_height` math; return SUMMARY ≤ 100 words."
  - "Summarize OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1501-1506,1644 for ext_perimeter_spacing2 + precise_outer_wall gating; return SUMMARY ≤ 150 words."
  - "Run `cargo test -p slicer-helpers --test flow_tdd`; return FACT pass/fail."
  - "Run `cargo test -p slicer-runtime --test integration outer_inner_width_and_spacing_tdd`; return FACT pass/fail."
- Context cost: `M` (largest step — three sub-steps + two OrcaSlicer SUMMARYs + new tests)
- Authoritative docs:
  - `docs/specs/perimeter-modules-orca-parity-roadmap.md` — T-050/T-051/T-052/T-053 rows.
  - `docs/13_slicer_helpers_crate.md`.
- OrcaSlicer refs: `Flow.cpp`, `PerimeterGenerator.cpp:1501-1506,1644` (delegate SUMMARY).
- Verification:
  - `cargo test -p slicer-helpers --test flow_tdd 2>&1 | tee target/test-output.log` — FACT.
  - `cargo test -p slicer-runtime --test integration outer_inner_width_and_spacing_tdd 2>&1 | tee target/test-output.log` — FACT.
- Exit condition: AC-1 green; spacing measured between walls matches `ext_perimeter_spacing2 = (outer + inner) / 2` and `perimeter_spacing = inner`.

### Step 5: T-054/T-054b/T-054c — wall_sequence migration + modes

- Task IDs:
  - `T-054` — Register `wall_sequence` in perimeter manifests; deregister from `path-optimization-default`
  - `T-054b` — Implement `OuterInner` + `InnerOuter` modes in `wall_sequence_reorder`
  - `T-054c` — Implement `InnerOuterInner` sandwich mode
- Objective: migrate `wall_sequence` config registration per ADR-0011; implement all three modes in `slicer-helpers::perimeter_utils::wall_sequence_reorder`; call from both perimeter modules; in-module wall tree built during generation and discarded after reorder.
- Precondition: Step 4 exit condition met.
- Postcondition: AC-2 verification command passes for all three modes.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-helpers/src/perimeter_utils.rs` — full (post-P102).
  - `modules/core-modules/path-optimization-default/path-optimization-default.toml`.
  - Both perimeter modules' `lib.rs` (`run_perimeters` body).
- Files allowed to edit (≤ 3 per sub-step):
  - 5a (helper): `crates/slicer-helpers/src/perimeter_utils.rs`, `crates/slicer-helpers/tests/wall_sequence_reorder_tdd.rs` (NEW).
  - 5b (config migration): `modules/core-modules/classic-perimeters/classic-perimeters.toml`, `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`, `modules/core-modules/path-optimization-default/path-optimization-default.toml`.
  - 5c (consumers): `modules/core-modules/classic-perimeters/src/lib.rs`, `modules/core-modules/arachne-perimeters/src/lib.rs`.
- Files explicitly out-of-bounds for this step:
  - `slicer-ir` (no IR change in this step).
  - Thin-wall / gap-fill / MMU code paths.
- Expected sub-agent dispatches:
  - "Summarize OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1801-1913 for wall_sequence reorder including InnerOuterInner sandwich; return SUMMARY ≤ 200 words, no code."
  - "Run `cargo test -p slicer-helpers --test wall_sequence_reorder_tdd`; return FACT pass/fail per mode."
- Context cost: `M` (helper + manifests + two-module consumer)
- Authoritative docs:
  - `docs/adr/0011-perimeter-module-owns-wall-sequencing.md` — read full.
  - `docs/specs/perimeter-modules-orca-parity-roadmap.md` — T-054/b/c rows.
- OrcaSlicer refs: `PerimeterGenerator.cpp:1801-1913` (delegate SUMMARY).
- Verification:
  - `cargo test -p slicer-helpers --test wall_sequence_reorder_tdd 2>&1 | tee target/test-output.log` — FACT.
  - `rg -q 'wall_sequence' modules/core-modules/path-optimization-default/path-optimization-default.toml` — exit 1 (key deregistered).
- Exit condition: AC-2 green; `wall_sequence` registered only in perimeter manifests.

### Step 6: T-060/T-061/T-062/T-063/T-064/T-065 — Thin-walls + gap-fill emission

- Task IDs:
  - `T-060` — Register `detect_thin_wall`
  - `T-061` — Thin-wall detection cascade
  - `T-062` — ThinWall emission as WallLoop
  - `T-063` — Gap collection per-inset
  - `T-064` — Gap-fill emission as WallLoop{GapFill}
  - `T-065` — Register `gap_infill_speed` + `filter_out_gap_fill`
- Objective: register the three config keys; implement the thin-wall + gap-fill code paths in both perimeter modules using `slicer-helpers::medial_axis` + `offset2_ex` + `opening_ex` from P103.
- Precondition: Step 5 exit condition met.
- Postcondition: AC-3, AC-N1, AC-4, AC-N2 verification commands pass.
- Files allowed to read (with line-range hints when > 300 lines):
  - Both perimeter modules' `lib.rs`.
  - `crates/slicer-helpers/src/medial_axis.rs` (from P103) — confirm signature.
- Files allowed to edit (≤ 3 per sub-step):
  - 6a (manifests): both perimeter `.toml`, `docs/15_config_keys_reference.md`.
  - 6b (thin-wall consumer): `modules/core-modules/classic-perimeters/src/lib.rs`, `modules/core-modules/arachne-perimeters/src/lib.rs`, `crates/slicer-runtime/tests/integration/thin_wall_emission_tdd.rs` (NEW).
  - 6c (gap-fill consumer): same two `lib.rs` (re-edit), `crates/slicer-runtime/tests/integration/gap_fill_emission_tdd.rs` (NEW).
- Files explicitly out-of-bounds for this step:
  - `slicer-helpers` — medial_axis / offset2_ex already exist from P103.
  - MMU code paths (Step 7).
- Expected sub-agent dispatches:
  - "Summarize OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1596-1609 + 1665-1670,1930-1958 for thin-wall + gap-fill cascades; return SUMMARY ≤ 200 words."
  - "Run `cargo test -p slicer-runtime --test integration thin_wall_emission_tdd gap_fill_emission_tdd`; return FACT pass/fail per case."
- Context cost: `M` (two-module rewrites + two new integration tests)
- Authoritative docs:
  - `docs/specs/perimeter-modules-orca-parity-roadmap.md` — Phase 6 rows.
- OrcaSlicer refs: `PerimeterGenerator.cpp:1596-1609,1665-1670,1930-1958` (delegate SUMMARY).
- Verification:
  - `cargo test -p slicer-runtime --test integration thin_wall_emission_tdd 2>&1 | tee target/test-output.log` — FACT.
  - `cargo test -p slicer-runtime --test integration gap_fill_emission_tdd 2>&1 | tee target/test-output.log` — FACT.
- Exit condition: AC-3 + AC-N1 + AC-4 + AC-N2 green.

### Step 7: T-P96-B/C1/C2 — Revert external_contour + consume bisector mask

- Task IDs:
  - `T-P96-B` — Revert `external_contour` consumption in both perimeter modules
  - `T-P96-C1` — Classic consumes mask per-cell
  - `T-P96-C2` — Variable-width consumes mask per-cell
- Objective: remove the `external_contour` call sites; implement per-cell outer-wall trace that skips edges where `bisector_edge_skip_mask[i][j] == true`; mask consumption layer goes outermost (after wall_sequence reorder); single-color baseline unchanged.
- Precondition: Step 6 exit condition met; AC-5 host populator passes (Step 3); IR field exists (Step 2).
- Postcondition: AC-6 + AC-N3 verification commands pass.
- Files allowed to read (with line-range hints when > 300 lines):
  - Both perimeter modules' `lib.rs` — range-read the per-cell trace loop.
- Files allowed to edit (≤ 3):
  - `modules/core-modules/classic-perimeters/src/lib.rs`
  - `modules/core-modules/arachne-perimeters/src/lib.rs`
  - `crates/slicer-runtime/tests/integration/mmu_bisector_dedup_tdd.rs` (NEW)
- Files explicitly out-of-bounds for this step:
  - `slicer-ir` (field present from Step 2; not edited).
  - `slicer-core/paint_segmentation/` (populator present from Step 3; not edited).
  - `slicer-helpers` (no helper change in this step).
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test integration mmu_bisector_dedup_tdd`; return FACT pass/fail per case (4-color cube test + single-color baseline test)."
  - "Find call sites of `region.external_contour()` in the perimeter modules; return LOCATIONS ≤ 5 entries (expected zero after revert)."
- Context cost: `M` (two-module rewrite + new integration test with 4-color fixture)
- Authoritative docs:
  - `docs/adr/0013-mmu-per-color-outer-wall-fragmentation.md`.
- OrcaSlicer refs: cited in Step 1's one-pager.
- Verification:
  - `cargo test -p slicer-runtime --test integration mmu_bisector_dedup_tdd 2>&1 | tee target/test-output.log` — FACT.
  - `! rg -q '\.external_contour\(\)' modules/core-modules/classic-perimeters/src/lib.rs modules/core-modules/arachne-perimeters/src/lib.rs` — exit 0 (revert complete).
- Exit condition: AC-6 + AC-N3 green; no external_contour calls remaining in module code.

### Step 8: Doc impact landing

- Task IDs:
  - Doc impact for the whole packet (covers T-062b, T-P96-C0, T-050..T-065, T-054*).
- Objective: land doc impact statement edits.
- Precondition: Step 7 exit condition met.
- Postcondition: all five Doc Impact Statement greps return hits.
- Files allowed to read (with line-range hints when > 300 lines):
  - `docs/02_ir_schemas.md` — range-read sections being edited.
  - `docs/13_slicer_helpers_crate.md` — full.
  - `docs/15_config_keys_reference.md` — range-read.
- Files allowed to edit (≤ 3):
  - `docs/02_ir_schemas.md`, `docs/13_slicer_helpers_crate.md`, `docs/15_config_keys_reference.md`.
- Files explicitly out-of-bounds for this step:
  - All source files.
- Expected sub-agent dispatches:
  - "For each grep in the Doc Impact Statement, run `rg -q` on the listed path; return FACT pass/fail per grep."
- Context cost: `S` (three doc edits)
- Authoritative docs: the three edited files.
- OrcaSlicer refs: none.
- Verification:
  - All five Doc Impact Statement greps return hits.
- Exit condition: Doc Impact Statement fully landed.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | One new doc; SUMMARY dispatch. |
| Step 2 | M | Three crates; LOCATIONS for downstream arms; guest-WASM gate. |
| Step 3 | M | Algorithm port + new TDD; one LOCATIONS dispatch. |
| Step 4 | M | Three sub-steps; two OrcaSlicer SUMMARYs; two new tests. |
| Step 5 | M | Helper + manifest migration + two-module consumer; one SUMMARY. |
| Step 6 | M | Two-module rewrites + two new integration tests; one SUMMARY. |
| Step 7 | M | Two-module rewrite + 4-color fixture integration test. |
| Step 8 | S | Three doc edits. |

Aggregate context cost: `M` (risk-flagged — 19 tasks; implementer should consider re-spawning a fresh agent after Step 4 if context exceeds 65%). No single step is `L`. Per-step file edit count never exceeds 3 (sub-step structure preserved throughout).

## Packet Completion Gate

- All eight steps complete; each step's exit condition met.
- AC-1 through AC-6 + AC-N1/N2/N3 all return PASS via worker dispatch.
- `cargo check --workspace --all-targets` clean.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo xtask build-guests --check` reports no STALE guests.
- `docs/07_implementation_status.md` updated for each T-050..T-P96-C2 entry — via worker dispatch.
- `packet.spec.md` ready to move from `status: draft` → `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` and confirm each returns PASS.
- Confirm the three gate commands in `packet.spec.md` §Verification are green.
- Record the actual schema-bump version chosen (4.3.0 vs 4.2.0 depending on P104 sequencing) in the closure log.
- Record any T-P96-A0 investigation findings that deviated from the "lower color-ID owns" default in the closure log.
- Note in the closure log that `external_contour` IR field remains in `SlicedRegion` until P107 T-P96-D — this is by design per ADR-0013.
- Confirm the implementer's peak context usage stayed under 70%. If exceeded, log it as a packet-authoring lesson for future spec-packet-generator runs (likely indicates Step 4 needs further subdivision in similar future packets).
