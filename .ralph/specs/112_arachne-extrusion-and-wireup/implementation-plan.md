# Implementation Plan: 112_arachne-extrusion-and-wireup

## Step Order Rationale

Centrality (Step 1) → bead_count (Step 2) → propagation (Step 3) → generate_toolpaths (Step 4) → stitch (Step 5) → simplify (Step 6) → remove_small (Step 7) → IR types + schema bump (Step 8) → real `arachne-perimeters` wire-up (Step 9) → 4 parity fixtures + cube_4color Arachne (Step 10) → deviation walk + docs (Step 11) → workspace ceremony (Step 12).

Steps 1–7 build the extrusion-generation pipeline in data-dependency order: centrality marks edges; bead_count uses those marks; propagation transforms bead-count assignments; generate_toolpaths emits lines from the propagated graph; stitch/simplify/remove_small post-process those lines. Step 8 (IR types) comes AFTER the helpers because the IR shape is informed by `generate_toolpaths`'s output. Step 9 (wire-up) consumes Steps 1–8 end-to-end; replacing the placeholder is one atomic edit. Steps 10–11 verify and document. Step 12 is the final gate.

The packet is the heaviest M2 packet (12 steps, ~13 tasks). The implementer MUST respect the 60% context cap; Step 9 is the most likely overflow point.

## Step 1 — Centrality Filtering (T-220)

- **Tasks:** T-220.
- **Objective:** Implement `filter_central(graph: &mut SkeletalTrapezoidationGraph, params: &CentralityParams)` in `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs`. Record 3 reference fixtures (square, wedge, multi-feature) + write AC-1 test.
- **Precondition:** P110 closed (`SkeletalTrapezoidationGraph` exists).
- **Postcondition:** `cargo test -p slicer-core centrality_three_fixtures` green; AC-1 falsifiable.
- **Files allowed to read:** `crates/slicer-core/src/skeletal_trapezoidation/{mod.rs, graph.rs}` (P110 outputs).
- **Files allowed to edit:** `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs` (NEW), `crates/slicer-core/src/skeletal_trapezoidation/mod.rs` (add `pub mod centrality;`), `crates/slicer-core/tests/centrality.rs` (NEW), `crates/slicer-core/tests/fixtures/arachne/centrality_{square,wedge,multi_feature}.json` (NEW).
- **Expected sub-agent dispatches:** ONE OrcaSlicer SUMMARY for `filterCentral / filterNoncentralRegions` (≤ 200 words). ONE `cargo test -p slicer-core centrality 2>&1 | tee target/test-output.log`.
- **Context cost:** M.
- **Authoritative docs:** OrcaSlicer SkeletalTrapezoidation.cpp (via SUMMARY).
- **Narrow verification:** `cargo test -p slicer-core centrality_three_fixtures 2>&1 | tee target/test-output.log`.
- **Cheapest falsifying check:** square fixture — every edge marked `central: true` (square has no boundary-distance variation).

## Step 2 — Bead-Count Assignment (T-221)

- **Tasks:** T-221 + AC-N1.
- **Objective:** Implement `assign_bead_counts` in `bead_count.rs`. Compute `r_avg = (r_min + r_max) / 2.0` per central edge; call `strategy.optimal_bead_count(2.0 * r_avg)`; store result as `bead_count` field on the edge. Write AC-2 test + AC-N1 negative test (CentralityNotRun error).
- **Precondition:** Step 1 done.
- **Postcondition:** `cargo test -p slicer-core bead_count_tapered_wedge` + `cargo test -p slicer-core bead_count_requires_centrality` green.
- **Files allowed to read:** `crates/slicer-core/src/beading/mod.rs` (P111 trait), `crates/slicer-core/src/beading/distributed.rs` (P111 base strategy), `crates/slicer-core/src/skeletal_trapezoidation/graph.rs`.
- **Files allowed to edit:** `crates/slicer-core/src/skeletal_trapezoidation/bead_count.rs` (NEW), `crates/slicer-core/src/skeletal_trapezoidation/mod.rs` (add `pub mod bead_count;`), `crates/slicer-core/tests/bead_count.rs` (NEW), `crates/slicer-core/tests/fixtures/arachne/bead_count_tapered_wedge.json` (NEW).
- **Expected sub-agent dispatches:** ONE OrcaSlicer SUMMARY for `optimal_bead_count` call site + R-derivation (≤ 100 words). ONE `cargo test`.
- **Context cost:** S.
- **Authoritative docs:** OrcaSlicer SkeletalTrapezoidation.cpp (via SUMMARY).
- **Narrow verification:** `cargo test -p slicer-core bead_count_ 2>&1 | tee target/test-output.log`.
- **Cheapest falsifying check:** AC-N1 first — calling `assign_bead_counts` on a graph with no centrality marks must return `Err(BeadCountError::CentralityNotRun)`, not panic.

## Step 3 — Bead-Count Propagation (T-222)

- **Tasks:** T-222.
- **Objective:** Implement `propagate_beadings_upward(graph)` and `propagate_beadings_downward(graph)` in `propagation.rs`. Mark `TransitionMiddle` / `TransitionEnd` per OrcaSlicer. Record 3 reference fixtures + write AC-3 test.
- **Precondition:** Step 2 done.
- **Postcondition:** `cargo test -p slicer-core propagation_three_fixtures` green.
- **Files allowed to read:** Steps 1–2 outputs; `crates/slicer-core/src/skeletal_trapezoidation/graph.rs`.
- **Files allowed to edit:** `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` (NEW), `crates/slicer-core/src/skeletal_trapezoidation/mod.rs` (add `pub mod propagation;`), `crates/slicer-core/tests/propagation.rs` (NEW), `crates/slicer-core/tests/fixtures/arachne/propagation_*.json` (NEW).
- **Expected sub-agent dispatches:** ONE OrcaSlicer SUMMARY for `propagateBeadingsUpward / Downward` (≤ 200 words). ONE `cargo test`.
- **Context cost:** M.
- **Authoritative docs:** OrcaSlicer SkeletalTrapezoidation.cpp (via SUMMARY).
- **Narrow verification:** `cargo test -p slicer-core propagation_three_fixtures 2>&1 | tee target/test-output.log`.
- **Cheapest falsifying check:** A single-bead-count-uniform fixture — no edges should be marked as transitions.

## Step 4 — Toolpath Generation (T-223)

- **Tasks:** T-223.
- **Objective:** Implement `generate_toolpaths(graph) -> Vec<VariableWidthLines>` in `arachne/generate_toolpaths.rs`. Output sorted by `inset_idx` ascending; per-junction widths match OrcaSlicer. Write AC-4 test against tapered-wedge fixture.
- **Precondition:** Step 3 done.
- **Postcondition:** `cargo test -p slicer-core generate_toolpaths_tapered_wedge` green.
- **Files allowed to read:** Steps 1–3 outputs; `crates/slicer-core/src/arachne/mod.rs` (P110); `crates/slicer-core/src/skeletal_trapezoidation/graph.rs`.
- **Files allowed to edit:** `crates/slicer-core/src/arachne/generate_toolpaths.rs` (NEW), `crates/slicer-core/src/arachne/mod.rs` (add `pub mod generate_toolpaths;`), `crates/slicer-core/tests/generate_toolpaths.rs` (NEW), `crates/slicer-core/tests/fixtures/arachne/toolpaths_tapered_wedge.json` (NEW).
- **Expected sub-agent dispatches:** ONE OrcaSlicer SUMMARY for `generateToolpaths` (≤ 200 words). ONE `cargo test`.
- **Context cost:** M.
- **Authoritative docs:** OrcaSlicer SkeletalTrapezoidation.cpp (via SUMMARY).
- **Narrow verification:** `cargo test -p slicer-core generate_toolpaths_tapered_wedge 2>&1 | tee target/test-output.log`.
- **Cheapest falsifying check:** Output's `inset_idx` MUST be monotone ascending — assert in test.

## Step 5 — Stitch Extrusions (T-225)

- **Tasks:** T-225.
- **Objective:** Implement `stitch_extrusions(lines, max_gap)` in `arachne/stitch.rs`. Join open polylines within `max_gap`; primary perimeters (closed, `inset_idx == 0`) NEVER split or merged across distinct loops. Write AC-6 test.
- **Precondition:** Step 4 done.
- **Postcondition:** `cargo test -p slicer-core stitch_extrusions_preserves_primary` green.
- **Files allowed to read:** Step 4 output; `crates/slicer-core/src/arachne/mod.rs`.
- **Files allowed to edit:** `crates/slicer-core/src/arachne/stitch.rs` (NEW), `crates/slicer-core/src/arachne/mod.rs` (add `pub mod stitch;`), `crates/slicer-core/tests/stitch.rs` (NEW), `crates/slicer-core/tests/fixtures/arachne/stitch_input_*.json` (NEW).
- **Expected sub-agent dispatches:** ONE OrcaSlicer SUMMARY for `stitch_extrusions` (≤ 150 words). ONE `cargo test`.
- **Context cost:** S.
- **Authoritative docs:** OrcaSlicer WallToolPaths.cpp (via SUMMARY).
- **Narrow verification:** `cargo test -p slicer-core stitch_extrusions_preserves_primary 2>&1 | tee target/test-output.log`.
- **Cheapest falsifying check:** Primary preservation invariant — any line where `is_closed == true && inset_idx == 0` must be byte-identical pre- and post-stitch.

## Step 6 — Simplify Toolpaths (T-226)

- **Tasks:** T-226.
- **Objective:** Implement `simplify_toolpaths(lines, dp_epsilon)` in `arachne/simplify.rs`. Douglas-Peucker simplification per `ExtrusionLine`; junction widths preserved. Write AC-7 test.
- **Precondition:** Step 5 done.
- **Postcondition:** `cargo test -p slicer-core simplify_toolpaths_vertex_count` green.
- **Files allowed to read:** Step 5 output.
- **Files allowed to edit:** `crates/slicer-core/src/arachne/simplify.rs` (NEW), `crates/slicer-core/src/arachne/mod.rs` (add `pub mod simplify;`), `crates/slicer-core/tests/simplify.rs` (NEW).
- **Expected sub-agent dispatches:** ONE OrcaSlicer SUMMARY for `simplifyToolPaths` (≤ 100 words). ONE `cargo test`.
- **Context cost:** S.
- **Authoritative docs:** OrcaSlicer WallToolPaths.cpp (via SUMMARY).
- **Narrow verification:** `cargo test -p slicer-core simplify_toolpaths_vertex_count 2>&1 | tee target/test-output.log`.
- **Cheapest falsifying check:** Per-junction widths preserved across simplification (vertex count drops; widths stay).

## Step 7 — Remove Small Lines (T-227)

- **Tasks:** T-227 + AC-N3.
- **Objective:** Implement `remove_small_lines(lines, min_length_factor, min_width)` in `arachne/remove_small.rs`. Primary preservation: closed `inset_idx == 0` lines NEVER removed. Transition lines shorter than `min_length_factor * min_width` are removed. Write AC-8 + AC-N3 tests.
- **Precondition:** Step 6 done.
- **Postcondition:** `cargo test -p slicer-core remove_small_lines_preserves_primary` + `cargo test -p slicer-core remove_small_lines_all_primary_invariant` green.
- **Files allowed to read:** Step 6 output.
- **Files allowed to edit:** `crates/slicer-core/src/arachne/remove_small.rs` (NEW), `crates/slicer-core/src/arachne/mod.rs` (add `pub mod remove_small;`), `crates/slicer-core/tests/remove_small.rs` (NEW).
- **Expected sub-agent dispatches:** ONE OrcaSlicer SUMMARY for `removeSmallLines` (≤ 100 words). ONE `cargo test`.
- **Context cost:** S.
- **Authoritative docs:** OrcaSlicer WallToolPaths.cpp (via SUMMARY).
- **Narrow verification:** `cargo test -p slicer-core remove_small_lines_ 2>&1 | tee target/test-output.log`.
- **Cheapest falsifying check:** AC-N3 — all-primary input → zero lines removed regardless of length.

## Step 8 — IR Types + Schema Bump (T-224)

- **Tasks:** T-224 + AC-5 + AC-N2.
- **Objective:** Add `ExtrusionLine` + `ExtrusionJunction` to `crates/slicer-ir/src/slice_ir.rs` with `#[serde(default)]` on new optional fields. Bump `CURRENT_SLICE_IR_SCHEMA_VERSION` minor version. Declare WIT records in `crates/slicer-schema/wit/deps/ir-types.wit`. Update host populator if needed. Write AC-5 round-trip + AC-N2 legacy deserialization tests.
- **Precondition:** Steps 1–7 done. MUST read current `CURRENT_SLICE_IR_SCHEMA_VERSION` via `rg -n 'pub const CURRENT_SLICE_IR_SCHEMA_VERSION' crates/slicer-ir/src/slice_ir.rs` before editing. Live value at refinement = `4.6.0` (P105/P106/P109 shipped; P105 carried it to 4.4.0 for `GapFill`), so target = `4.7.0`. Target = activation-time value with minor+1. Do NOT hardcode `4.7.0` if a parallel branch bumps first.
- **Postcondition:** `cargo test -p slicer-ir extrusion_line_roundtrip` + `cargo test -p slicer-ir extrusion_line_legacy_deserialization` green. `cargo xtask build-guests --check` CLEAN after rebuild.
- **Files allowed to read:** `crates/slicer-ir/src/slice_ir.rs` (range-read by `rg -n 'ExtrusionLine\|ExtrusionJunction\|Point3WithWidth\|CURRENT_SLICE_IR_SCHEMA_VERSION'`); `crates/slicer-schema/wit/deps/ir-types.wit` (full ≤ 300 LOC); `docs/02_ir_schemas.md` (schema-versioning section).
- **Files allowed to edit:** `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-schema/wit/deps/ir-types.wit`, `crates/slicer-wasm-host/src/host.rs` (if host populator needs the new types), `crates/slicer-sdk/src/views.rs` (only if a consumer requires the accessor — see Open Questions), `crates/slicer-ir/tests/extrusion_line_roundtrip.rs` (NEW).
- **Expected sub-agent dispatches:** ONE OrcaSlicer LOCATIONS for `ExtrusionLine` + `ExtrusionJunction` in `ExtrusionEntity.h` (≤ 10 entries). ONE `cargo xtask build-guests` (rebuild ALL guests after schema change). ONE `cargo xtask build-guests --check` to confirm CLEAN.
- **Context cost:** M.
- **Authoritative docs:** OrcaSlicer ExtrusionEntity.h (via LOCATIONS); `docs/02_ir_schemas.md` schema-versioning section.
- **Narrow verification:** `cargo test -p slicer-ir extrusion_line_ 2>&1 | tee target/test-output.log && cargo xtask build-guests --check 2>&1 | tee target/test-output.log`.
- **Cheapest falsifying check:** AC-N2 first — pre-bump JSON (no `is_odd`/`is_closed` fields) must round-trip via `serde(default)`.

## Step 9 — Real `arachne-perimeters::run_perimeters` Wire-Up (T-230)

- **Tasks:** T-230 + AC-9.
- **Objective:** IMPLEMENT `run_perimeters` in the P110-created empty skeleton (`arachne-perimeters/src/lib.rs` at P112 activation contains only `Ok(())` + `warn!` — the old 512-line iterative-inset fake was DELETED by P108/T-090). Pipeline: preprocess → voronoi → SKT → centrality → bead_count → propagation → generate_toolpaths → stitch → simplify → remove_small → emit WallLoops with variable widths. Handle MMU via T-P96-E per-color iteration. Write AC-9 test (simple square produces walls). IMPORTANT: `WallLoop.path` is `ExtrusionPath3D`, NOT `Vec<Point3WithWidth>` — use `extrusion_line_to_extrusion_path3d()` converter. `PerimeterRegion` has NO `wall_count` field — count is `walls.len()`. `LoopType::GapFill` IS available (P105/T-062b added it at 4.4.0) — map inset_idx 0→`Outer`, ≥1→`Inner`, and use `GapFill` for odd/gap-fill lines where appropriate.
- **Precondition:** Steps 1–8 done. `cargo xtask build-guests --check` CLEAN.
- **Postcondition:** `cargo test -p slicer-runtime --test executor arachne_perimeters_simple_square_produces_walls` green. The skeleton's `warn!`-only path is replaced with the real SKT pipeline. `cargo xtask build-guests --check` CLEAN after rebuild.
- **Files allowed to read:** Steps 1–8 outputs (all `slicer-core/src/{arachne,skeletal_trapezoidation,beading}/*` + `slicer-core/src/voronoi.rs`); `modules/core-modules/arachne-perimeters/src/lib.rs` (the P110 skeleton — empty `LayerModule` + `warn!`); `modules/core-modules/classic-perimeters/src/lib.rs` (range — emission pattern only, ≤ 100 lines); `docs/05_module_sdk.md` (`PerimeterOutputBuilder` API).
- **Files allowed to edit:** `modules/core-modules/arachne-perimeters/src/lib.rs` (REPLACE placeholder body), `crates/slicer-runtime/tests/executor/arachne_perimeters_simple_square.rs` (NEW), `crates/slicer-runtime/tests/executor/main.rs` (EDIT — add `mod arachne_perimeters_simple_square;` — S7 REQUIRED: the executor binary is aggregated; without this registration `cargo test --test executor <name>` silently runs 0 tests).
- **Expected sub-agent dispatches:** ONE `cargo xtask build-guests` (rebuild after wire-up). ONE `cargo xtask build-guests --check`. ONE `cargo test -p slicer-runtime --test executor arachne_perimeters_simple_square_produces_walls` — return FACT pass/fail.
- **Context cost:** M (most likely overflow point — pipeline call chain is dense).
- **Authoritative docs:** `docs/05_module_sdk.md` (`PerimeterOutputBuilder`); `docs/03_wit_and_manifest.md` (guest WASM patterns).
- **Narrow verification:** `cargo xtask build-guests --check 2>&1 | tee target/test-output.log && cargo test -p slicer-runtime --test executor arachne_perimeters_simple_square_produces_walls 2>&1 | tee target/test-output.log`.
- **Cheapest falsifying check:** `rg -q 'voronoi_from_segments\|filter_central\|assign_bead_counts' modules/core-modules/arachne-perimeters/src/lib.rs` — the real SKT pipeline calls must be present. The skeleton's `warn!` must be absent from the final version (`! rg -q 'no walls produced' modules/core-modules/arachne-perimeters/src/lib.rs`).

## Step 10 — Parity Fixtures (T-231)

- **Tasks:** T-231 + AC-10.
- **Objective:** Record 4 Arachne-specific parity fixtures (tapered wedge, narrow strip with widening, max-bead-count cap, complex multi-feature polygon) + cube_4color Arachne reference. Extend `crates/slicer-runtime/tests/integration/perimeter_parity.rs` with the Arachne suite entry. Write AC-10 test.
- **Precondition:** Step 9 done. P109's `perimeter_parity.rs` exists.
- **Postcondition:** `cargo test -p slicer-runtime --test integration arachne_perimeter_parity` green; all 4 + 1 fixtures pass within calibrated tolerances.
- **Files allowed to read:** `crates/slicer-runtime/tests/integration/perimeter_parity.rs` (full, ≤ 200 LOC from P109); `docs/specs/orca-mmu-perimeter-investigation.md` (full, for cube_4color Arachne); P109's `cube_4color_orca.gcode` reference path.
- **Files allowed to edit:** `crates/slicer-runtime/tests/fixtures/perimeter_parity/{tapered_wedge,narrow_strip_widening,max_bead_count_cap,complex_multi_feature,cube_4color_arachne}/` (NEW directories with `mesh.stl`/`config.toml`/`expected_perimeter_ir.json`), `crates/slicer-runtime/tests/integration/perimeter_parity.rs` (EDIT — add Arachne suite — P109 (`implemented`) already ships this file; it is PRESENT), `crates/slicer-runtime/tests/integration/main.rs` (EDIT — add `mod perimeter_parity;` — S7 REQUIRED: the integration binary is aggregated; P109 owns this registration but if P112 extends the file, it must ensure the mod declaration is present in main.rs).
- **Expected sub-agent dispatches:** FOUR OrcaSlicer SUMMARYs (≤ 100 words each, one per fixture) describing expected `PerimeterIR` shape. ONE `cargo test`.
- **Context cost:** M.
- **Authoritative docs:** `docs/specs/orca-mmu-perimeter-investigation.md` (for cube_4color Arachne); P109 parity harness.
- **Narrow verification:** `cargo test -p slicer-runtime --test integration arachne_perimeter_parity 2>&1 | tee target/test-output.log`.
- **Cheapest falsifying check:** tapered_wedge fixture — variable widths observable in output `WallLoop.path` (not all identical).

## Step 11 — Deviation Walk + Docs (T-232, T-233)

- **Tasks:** T-232 + T-233 + Doc Impact Statement.
- **Objective:** Walk every M2 deviation entry (D-7, D-9, D-15, plus any new) and close or justify with target follow-on packet IDs. **IMPORTANT:** D-7/D-9/D-15 live in `docs/specs/perimeter-modules-orca-parity-roadmap.md` (NOT `docs/DEVIATION_LOG.md`). The AC-11 verification grep targets the roadmap, not the deviation log. Any new deviations created during M2 work that go into `docs/DEVIATION_LOG.md` MUST use the `D-112-<SLUG>` format per the log's live convention. Update `docs/01_system_architecture.md` Tier-2 `Layer::Perimeters` to name the real Arachne pipeline (Voronoi + SkeletalTrapezoidation + BeadingStrategy) citing P112 — there is no "iterative-inset" caveat to drop (P108 already cleaned it). Update `docs/02_ir_schemas.md` with `ExtrusionLine`/`ExtrusionJunction` entries + schema-bump rationale. Update `docs/07_implementation_status.md` M2 complete entry. Flip Phase 12 + 13 + M2 rows in roadmap.
- **Precondition:** Step 10 done. All ACs except AC-11/AC-12/AC-13 already green.
- **Postcondition:** AC-11 + AC-12 green; Doc Impact Statement assertions all green.
- **Files allowed to read:** `docs/specs/perimeter-modules-orca-parity-roadmap.md` (D-7/D-9/D-15 closure entries live here, Phases 12 + 13 + M2 marker), `docs/DEVIATION_LOG.md` (for format reference + any new M2 deviations), `docs/01_system_architecture.md` (Tier-2 section), `docs/02_ir_schemas.md` (schema-versioning + existing-types format), `docs/07_implementation_status.md` (M2 status section).
- **Files allowed to edit:** `docs/DEVIATION_LOG.md`, `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`, `docs/07_implementation_status.md`, `docs/specs/perimeter-modules-orca-parity-roadmap.md`.
- **Expected sub-agent dispatches:** None (docs work).
- **Context cost:** S.
- **Authoritative docs:** the five doc files themselves.
- **Narrow verification:** AC-11's shell loop (targeting `docs/specs/perimeter-modules-orca-parity-roadmap.md` for D-7/D-9/D-15) + AC-12's grep — both deterministic.
- **Cheapest falsifying check:** `rg -q 'Voronoi' docs/01_system_architecture.md && rg -q 'SkeletalTrapezoidation' docs/01_system_architecture.md && rg -q 'BeadingStrategy' docs/01_system_architecture.md` — if the real-Arachne naming is absent, T-233 didn't land.

## Step 12 — Workspace Closure Ceremony (T-234)

- **Tasks:** T-234 + AC-13.
- **Objective:** Run the full suite via the gated entry point `cargo xtask test --workspace --summary` (NOT bare `cargo test --workspace`) as the M2 closure gate per CLAUDE.md §"Test Discipline" workspace-test exception; the gate fires the guest-WASM freshness check first (this packet rebuilt the `arachne-perimeters` guest). Dispatch the run to a sub-agent that returns FACT pass/fail + summary line + count.
- **Precondition:** Steps 1–11 done. All other ACs green. `cargo xtask build-guests --check` CLEAN.
- **Postcondition:** Full workspace suite green. Closure log authored. `status: implemented` flipped.
- **Files allowed to read:** `CLAUDE.md` §"Test Discipline" (re-confirm the exception applies).
- **Files allowed to edit:** `.ralph/specs/112_arachne-extrusion-and-wireup/closure-log.md` (NEW), `.ralph/specs/112_arachne-extrusion-and-wireup/packet.spec.md` (status flip).
- **Expected sub-agent dispatches:** ONE `cargo xtask test --workspace --summary 2>&1 | tee target/test-output.log | tail -20` — return FACT pass/fail + summary line + count. The implementer does NOT absorb the full output.
- **Context cost:** S.
- **Authoritative docs:** `CLAUDE.md` §"Test Discipline".
- **Narrow verification:** `tail -5 target/test-output.log` after the dispatch.
- **Cheapest falsifying check:** the dispatch FACT — pass/fail is the gate.

## Packet Completion Gate

- All 13 ACs + 3 AC-Ns pass per their pipe-suffix commands.
- `cargo check --workspace --all-targets` green.
- `cargo clippy --workspace --all-targets -- -D warnings` green.
- `cargo xtask build-guests --check` CLEAN.
- `cargo test -p slicer-core 2>&1 | tee target/test-output.log` shows all new tests passing.
- `cargo test -p slicer-runtime --test integration arachne_perimeter_parity` green (4 fixtures + cube_4color Arachne).
- `cargo xtask test --workspace --summary 2>&1 | tee target/test-output.log` green (T-234 closure ceremony — gated entry point, sub-agent dispatched).
- Doc Impact Statement assertions verified by `rg` checks.
- Closure log authored before status flip; records: schema-version values pre/post bump; any goldens that needed re-recording with NOTE explaining why; D-7/D-9/D-15 closure rationale paragraphs; cube_4color Arachne reference decision (same-as-Classic OR Arachne-specific); any new M2 deviations registered + their target follow-on packet IDs.
- Status flipped to `implemented` in `packet.spec.md` YAML frontmatter.
- `docs/specs/perimeter-modules-orca-parity-roadmap.md` M2 marker shows DONE.

## Context Budget Cap

Aggregate cost: M. If the implementer reaches 60% context at any step, they STOP, write the partial-state to closure-log.md, and hand off — the remaining steps inherit. Step 9 (real wire-up) is the most likely overflow point because it composes 8 prior steps' outputs into a single call chain. If Step 9 overflows, the implementer SHOULD split T-230 into single-region wire-up first (handle MMU per-color in a follow-on P112a). Step 10 (parity fixtures) is the second-most-likely overflow because each fixture recording involves an OrcaSlicer SUMMARY dispatch; if Step 10 overflows, ship 2 fixtures here + 2 + cube_4color in P112b.

Step 12 (workspace ceremony) MUST be dispatched. The implementer does NOT absorb >200 lines of cargo output — the dispatch FACT is the gate result.
