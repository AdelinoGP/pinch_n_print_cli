---
status: draft
packet: 112_arachne-extrusion-and-wireup
task_ids:
  - T-220
  - T-221
  - T-222
  - T-223
  - T-224
  - T-225
  - T-226
  - T-227
  - T-230
  - T-231
  - T-232
  - T-233
  - T-234
backlog_source: docs/specs/perimeter-modules-orca-parity-roadmap.md
context_cost_estimate: M
---

# Packet Contract: 112_arachne-extrusion-and-wireup

## Goal

Close M2 of the perimeter parity roadmap: port the Arachne extrusion-generation pipeline (centrality filtering T-220, per-edge bead-count assignment T-221, bead-count upward+downward propagation T-222, `generateToolpaths` T-223), add the `ExtrusionLine` + `ExtrusionJunction` IR types T-224 with serde compat (schema bump), port `stitch_extrusions` (T-225) + `simplifyToolPaths` (T-226) + `removeSmallLines` (T-227), wire the whole pipeline into `arachne-perimeters::run_perimeters` (T-230 — replaces the P110 placeholder), extend the M1 parity harness with 4 Arachne fixtures (T-231: tapered-wedge, narrow-strip-with-widening, max-bead-count-cap, complex-multi-feature-polygon), walk every M2 deviation entry and close or justify (T-232), update `docs/01_system_architecture.md` Tier-2 to drop the "iterative-inset approximation" caveat (T-233), and run the M2 closure-ceremony `cargo test --workspace` (T-234, CLAUDE.md §"Test Discipline" workspace-test exception).

## Scope Boundaries

Touches `crates/slicer-core/src/skeletal_trapezoidation/` (extend with `centrality.rs`, `bead_count.rs`, `propagation.rs`), `crates/slicer-core/src/arachne/` (extend with `generate_toolpaths.rs`, `stitch.rs`, `simplify.rs`, `remove_small.rs`), `crates/slicer-ir/src/slice_ir.rs` (add `ExtrusionLine` + `ExtrusionJunction` with serde compat + version bump), `modules/core-modules/arachne-perimeters/src/lib.rs` (replace placeholder with real `run_perimeters`), `crates/slicer-runtime/tests/fixtures/perimeter_parity/` (add 4 Arachne fixtures), `crates/slicer-runtime/tests/integration/perimeter_parity.rs` (extend harness if needed for Arachne-specific comparators), `docs/DEVIATION_LOG.md` (M2 deviation walk), `docs/01_system_architecture.md` (Tier-2 caveat removal), `docs/07_implementation_status.md` (M2 complete entry), `docs/02_ir_schemas.md` (`ExtrusionLine`/`ExtrusionJunction` schema entry), and WIT (the new IR types).

## Prerequisites and Blockers

- Depends on:
  - **P110** — Voronoi wrapper, SkeletalTrapezoidationGraph, parabolic discretization, 9-stage preprocess + T-P96-E per-color MMU, `arachne-perimeters` skeleton.
  - **P111** — BeadingStrategy trait + 5 strategies + factory + 11 config keys.
  - **P109** — M1 parity harness exists and is green (Arachne fixtures extend this harness).
- Unblocks:
  - **Perimeter parity at OrcaSlicer M2 level** — this packet is the last M2 implementation packet. M2-DONE flips after T-234 green.
- Activation blockers: D-15 closed by `docs/specs/orca-mmu-perimeter-investigation.md` (T-P96-A0 in P102) — Arachne MMU path is documented; T-231's cube_4color Arachne fixture relies on T-P96-E preprocessing from P110 to be in place AND on this packet's T-230 wire-up to produce per-color fragmented walls.

## Acceptance Criteria

- **AC-1. Given** centrality filtering in `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs`, **when** `filter_central(graph: &mut SkeletalTrapezoidationGraph, params: &CentralityParams)` runs against three OrcaSlicer reference fixtures (square, wedge, multi-feature), **then** the post-filter `central: bool` marker on every edge matches the recorded reference exactly (zero discrepancy per fixture). | `cargo test -p slicer-core centrality_three_fixtures -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-2. Given** per-edge bead-count assignment in `bead_count.rs`, **when** `assign_bead_counts(graph: &mut SkeletalTrapezoidationGraph, strategy: &dyn BeadingStrategy)` runs against a golden tapered-wedge fixture, **then** each central edge carries an integer `bead_count` derived from `strategy.optimal_bead_count(2.0 * r_avg)` (where `r_avg = (r_min + r_max) / 2.0`), matching OrcaSlicer's recorded per-edge counts exactly. | `cargo test -p slicer-core bead_count_tapered_wedge -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-3. Given** propagation in `propagation.rs`, **when** `propagate_beadings_upward(graph)` followed by `propagate_beadings_downward(graph)` runs against three reference fixtures, **then** edges are correctly marked as `TransitionMiddle` or `TransitionEnd`, matching OrcaSlicer's recorded edge markers within zero discrepancy. | `cargo test -p slicer-core propagation_three_fixtures -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-4. Given** `generate_toolpaths(graph) -> Vec<VariableWidthLines>` in `arachne/generate_toolpaths.rs`, **when** called against the tapered-wedge fixture, **then** the output (a) is sorted by `inset_idx` ascending (outer first, inner later), (b) per-junction width topology matches OrcaSlicer's recorded reference within 0.01 mm (100 units), (c) the number of lines per inset_idx matches OrcaSlicer's count exactly. | `cargo test -p slicer-core generate_toolpaths_tapered_wedge -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-5. Given** the new `ExtrusionLine { junctions: Vec<ExtrusionJunction>, inset_idx: u32, is_odd: bool, is_closed: bool }` and `ExtrusionJunction { p: Point3WithWidth, perimeter_index: u32 }` IR types in `crates/slicer-ir/src/slice_ir.rs`, **when** an `ExtrusionLine` round-trips through `serde_json::to_string` + `from_str`, **then** the deserialized struct equals the original; `CURRENT_SLICE_IR_SCHEMA_VERSION` bumps to `4.7.0` (additive change). | `cargo test -p slicer-ir extrusion_line_roundtrip -- --nocapture 2>&1 | tee target/test-output.log && rg -q 'pub const CURRENT_SLICE_IR_SCHEMA_VERSION: SemVer = SemVer \{ major: 4, minor: 7, patch: 0' crates/slicer-ir/src/slice_ir.rs`
- **AC-6. Given** `stitch_extrusions(lines, max_gap)` in `arachne/stitch.rs`, **when** called against a fixture where two open polylines should join within `bead_width_x - 1 nm`, **then** the output joins them into a single closed line; primary perimeters (closed, `inset_idx == 0`) are never split or merged across distinct loops. | `cargo test -p slicer-core stitch_extrusions_preserves_primary -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-7. Given** `simplify_toolpaths(lines, params)` in `arachne/simplify.rs` (Douglas-Peucker per `ExtrusionLine`), **when** called against the tapered-wedge fixture, **then** the per-line vertex count matches OrcaSlicer's simplified output within 1 vertex (DP rounding tolerance). | `cargo test -p slicer-core simplify_toolpaths_vertex_count -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-8. Given** `remove_small_lines(lines, min_length_factor, min_width)` in `arachne/remove_small.rs`, **when** called against a fixture mixing primary perimeters + short transition lines, **then** primary perimeters (closed, `inset_idx == 0`) are NEVER removed; transition lines shorter than `min_length_factor * min_width` ARE removed; closed even-`inset_idx` lines are NEVER removed regardless of length. | `cargo test -p slicer-core remove_small_lines_preserves_primary -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-9. Given** the real `arachne-perimeters::run_perimeters` (T-230 — replaces P110 placeholder), **when** called against a single-region simple-square `SlicedRegion`, **then** (a) the placeholder `warn!` from P110 is GONE, (b) the output `PerimeterRegion.walls` carries `wall_count` `WallLoop`s sorted by `inset_idx` ascending, (c) each `WallLoop.path` is a `Vec<Point3WithWidth>` derived from the `ExtrusionLine` → `Point3WithWidth` converter, (d) variable widths are observable (not all identical). | `cargo test -p slicer-runtime --test executor arachne_perimeters_simple_square_produces_walls -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-10. Given** the 4 Arachne fixtures in `crates/slicer-runtime/tests/fixtures/perimeter_parity/{tapered_wedge,narrow_strip_widening,max_bead_count_cap,complex_multi_feature}/`, **when** the parity harness (extended from P109's T-100) runs against each, **then** every fixture passes within its calibrated tolerances (per-junction width within 0.01 mm; per-vertex XYZ within 0.005 mm; inset_idx exact). | `cargo test -p slicer-runtime --test integration arachne_perimeter_parity -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-11. Given** the M2 deviation walk (T-232), **when** `docs/DEVIATION_LOG.md` is inspected, **then** every M2 deviation entry from D-7 (closed by ADR-0010 in P110), D-9 (closed by T-215b in P111), D-15 (closed by `docs/specs/orca-mmu-perimeter-investigation.md` in P102) carries a closure note pointing at the implementing packet, AND any new deviations registered during M2 work are closed or carry justified-residual status with a target follow-on packet ID. | `for d in D-7 D-9 D-15; do rg -q "$d.*CLOSED\|$d.*closed" docs/DEVIATION_LOG.md || { echo "MISSING $d"; exit 1; }; done`
- **AC-12. Given** `docs/01_system_architecture.md` (T-233), **when** the Tier-2 section is inspected, **then** the "iterative-inset width approximation" caveat is REMOVED and replaced with a "real Arachne (Voronoi + SkeletalTrapezoidation + BeadingStrategy stack)" line citing this packet's ID. | `! rg -q 'iterative-inset width approximation' docs/01_system_architecture.md && rg -q 'Voronoi.*SkeletalTrapezoidation.*BeadingStrategy' docs/01_system_architecture.md`
- **AC-13. Given** the M2 closure-ceremony test (T-234), **when** `cargo test --workspace` runs to completion at packet close, **then** every test passes (full suite is the M2 closure gate per CLAUDE.md §"Test Discipline" workspace-test exception). | `cargo test --workspace 2>&1 | tee target/test-output.log | tail -5`

## Negative Test Cases

- **AC-N1. Given** a `SkeletalTrapezoidationGraph` where centrality has NOT been run, **when** `assign_bead_counts` is called, **then** the call returns `Err(BeadCountError::CentralityNotRun)` (not a panic; not a silent garbage output). | `cargo test -p slicer-core bead_count_requires_centrality -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. Given** an `ExtrusionLine` deserialized from a schema-pre-bump JSON (no `is_odd` field), **when** the deserializer runs, **then** `serde(default)` fills `is_odd = false` (the additive-change migration path); no parse error. | `cargo test -p slicer-ir extrusion_line_legacy_deserialization -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-N3. Given** `remove_small_lines` against an all-primary input (every line `inset_idx == 0` and `is_closed == true`), **when** the function runs, **then** zero lines are removed regardless of length (primary preservation invariant). | `cargo test -p slicer-core remove_small_lines_all_primary_invariant -- --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace 2>&1 | tee target/test-output.log` (T-234 / M2 closure ceremony — final gate)

## Authoritative Docs

- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — Phases 12 (T-220..T-227) + 13 (T-230..T-234). Range-read those rows.
- `docs/02_ir_schemas.md` — schema-version contract for additive `ExtrusionLine`/`ExtrusionJunction` types.
- `docs/03_wit_and_manifest.md` — WIT type declarations for the new IR types.
- `docs/05_module_sdk.md` — `PerimeterOutputBuilder` API surface for T-230 real wire-up.
- `docs/01_system_architecture.md` — Tier-2 section for T-233 caveat removal.
- `docs/07_implementation_status.md` — M2 status entry format.
- `docs/DEVIATION_LOG.md` — M2 entries to close.
- `CLAUDE.md` — §"Test Discipline" / workspace-test exception for T-234.
- `docs/specs/orca-mmu-perimeter-investigation.md` (from P102 / T-P96-A0) — Arachne MMU path for T-231 cube_4color fixture.

## Doc Impact Statement (Required)

- `docs/07_implementation_status.md` — M2 marked complete with packet IDs P110..P112 listed — `rg -q 'M2.*complete.*P110.*P111.*P112\|M2.*P110.*P111.*P112.*complete' docs/07_implementation_status.md`
- `docs/01_system_architecture.md` — Tier-2 "iterative-inset approximation" caveat REMOVED; real-Arachne line added — `! rg -q 'iterative-inset width approximation' docs/01_system_architecture.md && rg -q 'Voronoi.*SkeletalTrapezoidation.*BeadingStrategy' docs/01_system_architecture.md`
- `docs/02_ir_schemas.md` — record schema bump rationale for `ExtrusionLine` + `ExtrusionJunction` additions — `rg -q 'ExtrusionLine\|ExtrusionJunction' docs/02_ir_schemas.md`
- `docs/DEVIATION_LOG.md` — D-7/D-9/D-15 closures recorded — verified by AC-11 shell loop.
- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — flip T-220..T-227 + T-230..T-234 rows to DONE; flip M2 milestone marker to DONE — `rg -q 'T-220.*DONE' docs/specs/perimeter-modules-orca-parity-roadmap.md && rg -q 'M2.*DONE\|M2.*shipped\|M2.*complete' docs/specs/perimeter-modules-orca-parity-roadmap.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked).

Files to inspect for this packet — ONE dispatch per file:

| File | Dispatch | Return ≤ |
| --- | --- | --- |
| `Arachne/SkeletalTrapezoidation.cpp::filterCentral / filterNoncentralRegions` | SUMMARY | 200 words — centrality predicate + filter loop |
| `Arachne/SkeletalTrapezoidation.cpp::propagateBeadingsUpward / Downward` | SUMMARY | 200 words — propagation pass + TransitionMiddle/End marking |
| `Arachne/SkeletalTrapezoidation.cpp::generateToolpaths` | SUMMARY | 200 words — `Vec<VariableWidthLines>` emission + inset_idx sort |
| `Arachne/WallToolPaths.cpp::stitch_extrusions` | SUMMARY | 150 words — gap-join rule + primary preservation |
| `Arachne/WallToolPaths.cpp::simplifyToolPaths` | SUMMARY | 100 words — DP epsilon per `ExtrusionLine` |
| `Arachne/WallToolPaths.cpp::removeSmallLines` | SUMMARY | 100 words — removal rule + primary invariant |
| `libslic3r/ExtrusionEntity.h` (`ExtrusionLine`, `ExtrusionJunction`) | LOCATIONS | 10 entries — struct fields + invariants |

For T-231's 4 Arachne parity fixtures: ONE SUMMARY per fixture (≤ 100 words each) describing the expected `PerimeterIR` shape (wall count, role distribution, per-junction width). 4 dispatches. The recorded fixtures live as committed JSON files; OrcaSlicer is NOT called at test time.

For T-231's cube_4color Arachne extension: use `docs/specs/orca-mmu-perimeter-investigation.md` (from P102 / T-P96-A0) — no direct OrcaSlicer read.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
