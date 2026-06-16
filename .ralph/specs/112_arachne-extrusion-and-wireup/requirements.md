# Requirements: 112_arachne-extrusion-and-wireup

## Packet Metadata

- Grouped task IDs:
  - `T-220` — Port centrality filtering (`filterCentral`, `filterNoncentralRegions`).
  - `T-221` — Bead-count assignment on central edges (`optimal_bead_count(R)` per edge).
  - `T-222` — Port bead-count upward + downward propagation (`propagateBeadingsUpward`, `propagateBeadingsDownward`) — marks `TransitionMiddle` / `TransitionEnd`.
  - `T-223` — Port `generateToolpaths()` — emits `Vec<VariableWidthLines>` (sorted by `inset_idx`).
  - `T-224` — Define `ExtrusionLine` + `ExtrusionJunction` IR types; `Point3WithWidth` round-trips via converter.
  - `T-225` — Port `stitch_extrusions` (join open polylines within `bead_width_x - 1nm`).
  - `T-226` — Port `simplifyToolPaths` (DP simplification per `ExtrusionLine`).
  - `T-227` — Port `removeSmallLines` (drop odd, non-closed lines shorter than `min_length_factor * min_width`).
  - `T-230` — Wire `slicer-core::{arachne, beading, skeletal_trapezoidation}` into `arachne-perimeters::run_perimeters`. Module produces WallLoops with per-junction width; pre-processing + SKT + beading + extrusion-gen runs end-to-end on golden fixture.
  - `T-231` — Extend parity harness (P109 / T-100) with 4 Arachne fixtures: tapered wedge, narrow strip with widening, max-bead-count cap, complex multi-feature polygon. Plus cube_4color Arachne extension via T-P96-E preprocessing.
  - `T-232` — Walk every M2 deviation entry from T-003 update; close or justify.
  - `T-233` — Update `docs/01_system_architecture.md` Tier-2 box to reflect real Arachne availability; remove "iterative-inset approximation" caveat.
  - `T-234` — Final `cargo test --workspace` (closure-ceremony, CLAUDE.md workspace-test exception).
- Backlog source: `docs/specs/perimeter-modules-orca-parity-roadmap.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

P110 shipped the foundations (Voronoi wrapper, SkeletalTrapezoidationGraph, parabolic discretization, 9-stage preprocess, per-color MMU dedup, arachne-perimeters module skeleton with placeholder `run_perimeters`). P111 shipped the BeadingStrategy stack (trait, 5 strategies, factory, 11 config keys, D-9 strip-pass). Neither produces walls — the placeholder `run_perimeters` from P110 still returns `Ok(())` and traces a `warn!`. P112 closes the loop: extrusion generation reads the SKT graph's centrality marks + per-edge bead counts + propagated transitions and emits `Vec<VariableWidthLines>`; stitch + simplify + removeSmall clean the output; `arachne-perimeters::run_perimeters` calls the whole pipeline end-to-end (preprocess → voronoi → SKT graph → centrality → bead counts → propagation → generate_toolpaths → stitch → simplify → removeSmall → emit WallLoops with variable widths).

T-224 adds `ExtrusionLine` + `ExtrusionJunction` IR types. These are NEW additions (additive schema change); the bump is minor-version (e.g., 4.4.0 → 4.5.0 depending on M1 close state). Both types use `#[serde(default)]` on any new optional fields for round-trip safety with pre-bump fixtures.

T-231 extends P109's parity harness with 4 Arachne-specific fixtures (tapered wedge tests variable widths; narrow strip with widening tests the Widening strategy; max-bead-count cap tests the Limited strategy; complex multi-feature polygon tests the whole SKT graph end-to-end). It also extends the cube_4color test from P109 to assert Arachne produces per-color fragmented walls — this is the M2 half of T-P96-E (M1 half landed in P105 via Classic; the per-color preprocessing from P110 + this packet's wire-up makes Arachne ship the same parity behavior).

T-232 (deviation walk) closes D-7 (boostvoronoi selection — via ADR-0010 in P110), D-9 (sentinel strip — via T-215b in P111), and D-15 (Arachne MMU path — via investigation in P102). Any new deviations registered during M2 work get closure entries or justified-residual status.

T-233 (architecture doc) flips the Tier-2 box: the old "iterative-inset width approximation" caveat documented the variable-width-perimeters placeholder; with real Arachne shipping, the caveat is replaced by "real Arachne (Voronoi + SkeletalTrapezoidation + BeadingStrategy stack)" citing P112.

T-234 (closure ceremony) runs the full `cargo test --workspace`. This is the workspace-test exception per CLAUDE.md — every prior verification in P112 was narrow (per-crate or per-test); the closure ceremony is the gate that catches cross-cutting regressions in M1 modules that M2 wire-up might have introduced.

## In Scope

- `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs` (NEW) — T-220.
- `crates/slicer-core/src/skeletal_trapezoidation/bead_count.rs` (NEW) — T-221.
- `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` (NEW) — T-222.
- `crates/slicer-core/src/arachne/generate_toolpaths.rs` (NEW) — T-223.
- `crates/slicer-core/src/arachne/stitch.rs` (NEW) — T-225.
- `crates/slicer-core/src/arachne/simplify.rs` (NEW) — T-226.
- `crates/slicer-core/src/arachne/remove_small.rs` (NEW) — T-227.
- `crates/slicer-core/tests/{centrality.rs, bead_count.rs, propagation.rs, generate_toolpaths.rs, stitch.rs, simplify.rs, remove_small.rs}` + fixtures (NEW).
- `crates/slicer-ir/src/slice_ir.rs` (EDIT) — add `ExtrusionLine` + `ExtrusionJunction` types; bump `CURRENT_SLICE_IR_SCHEMA_VERSION` minor version.
- `crates/slicer-schema/wit/deps/ir-types.wit` (EDIT) — declare `extrusion-line` and `extrusion-junction` records.
- `crates/slicer-wasm-host/src/host.rs` (EDIT) — populate new fields for guest reads if needed.
- `crates/slicer-sdk/src/views.rs` (EDIT) — expose `extrusion_line` accessor if used by other modules (likely deferred to a follow-on; this packet's `arachne-perimeters` builds the lines internally and converts to `Point3WithWidth` for the existing WallLoop surface).
- `modules/core-modules/arachne-perimeters/src/lib.rs` (EDIT) — replace placeholder `run_perimeters` with real wire-up.
- `crates/slicer-runtime/tests/fixtures/perimeter_parity/{tapered_wedge,narrow_strip_widening,max_bead_count_cap,complex_multi_feature,cube_4color_arachne}/` (NEW) — 4 Arachne fixtures + cube_4color Arachne reference.
- `crates/slicer-runtime/tests/integration/perimeter_parity.rs` (EDIT) — extend harness if comparators need Arachne-specific tolerances; add Arachne suite entry.
- `crates/slicer-runtime/tests/executor/arachne_perimeters_simple_square.rs` (NEW) — AC-9 standalone test.
- `docs/DEVIATION_LOG.md` (EDIT) — close D-7/D-9/D-15 + any new M2 entries.
- `docs/01_system_architecture.md` (EDIT) — Tier-2 caveat removal + real-Arachne line.
- `docs/02_ir_schemas.md` (EDIT) — `ExtrusionLine`/`ExtrusionJunction` entries + version bump rationale.
- `docs/07_implementation_status.md` (EDIT) — M2 complete entry.
- `docs/specs/perimeter-modules-orca-parity-roadmap.md` (EDIT) — flip T-220..T-234 + M2 marker to DONE.

## Out of Scope

- Arachne-specific config keys beyond those registered in P111 (`m_params.*`) — any newly discovered config keys ship in a follow-on packet.
- Performance optimization of the Arachne pipeline — wall-clock measurement isn't gated.
- Multi-region Arachne edge cases beyond the 4 fixtures + cube_4color — additional fixtures ship in audit follow-ons.
- Spiral-vase + non-planar — orthogonal sibling roadmaps (per D-3, D-11).
- Overhang pipeline restructuring — closed by P106/P107 in M1.
- Classic-perimeters edits — M1 frozen.

## Authoritative Docs

| Doc | Size | Read strategy |
| --- | --- | --- |
| `docs/specs/perimeter-modules-orca-parity-roadmap.md` | ~400 lines | Range-read Phases 12 + 13 rows. |
| `docs/02_ir_schemas.md` | ~900 lines | Range-read schema-versioning section + existing `Point3WithWidth` for T-224's converter. |
| `docs/03_wit_and_manifest.md` | ~600 lines | Range-read WIT type declaration syntax. |
| `docs/05_module_sdk.md` | ~700 lines | Range-read `PerimeterOutputBuilder` API surface. |
| `docs/01_system_architecture.md` | varies | Range-read Tier-2 section. |
| `docs/07_implementation_status.md` | varies | Range-read current M2 status section. |
| `docs/DEVIATION_LOG.md` | varies | Range-read D-7/D-9/D-15 entries. |
| `docs/specs/orca-mmu-perimeter-investigation.md` | ≤ 200 lines | Read full (small) — guides cube_4color Arachne fixture for T-231. |
| `CLAUDE.md` | ~600 lines | Read §"Test Discipline" — confirms workspace-test ceremony exception for T-234. |

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked).

Files to inspect for this packet — ONE dispatch per function/file:

| File / Function | Dispatch | Return ≤ |
| --- | --- | --- |
| `Arachne/SkeletalTrapezoidation.cpp::filterCentral` + `filterNoncentralRegions` | SUMMARY | 200 words — centrality predicate + filter loop body |
| `Arachne/SkeletalTrapezoidation.cpp::optimal_bead_count` call site + R-derivation | SUMMARY | 100 words — how `r_min`/`r_max`/`r_avg` map to `optimal_bead_count` input |
| `Arachne/SkeletalTrapezoidation.cpp::propagateBeadingsUpward / Downward` | SUMMARY | 200 words — propagation pass body + TransitionMiddle/End marker rule |
| `Arachne/SkeletalTrapezoidation.cpp::generateToolpaths` | SUMMARY | 200 words — `Vec<VariableWidthLines>` emission + inset_idx sort |
| `Arachne/WallToolPaths.cpp::stitch_extrusions` | SUMMARY | 150 words — gap-join rule + primary preservation |
| `Arachne/WallToolPaths.cpp::simplifyToolPaths` | SUMMARY | 100 words — DP epsilon |
| `Arachne/WallToolPaths.cpp::removeSmallLines` | SUMMARY | 100 words — removal rule + primary invariant |
| `libslic3r/ExtrusionEntity.h` (`ExtrusionLine`, `ExtrusionJunction`) | LOCATIONS | 10 entries — struct fields + invariants |

For T-231's 4 Arachne fixtures: ONE SUMMARY per fixture (≤ 100 words each) — describe expected `PerimeterIR` shape (wall count, role distribution, per-junction width). 4 dispatches total. The recorded fixtures are JSON files derived from these expectations.

For T-231's cube_4color Arachne extension: NO direct OrcaSlicer read needed. Use `docs/specs/orca-mmu-perimeter-investigation.md` (P102/T-P96-A0 one-pager) as the authoritative source.

## Acceptance Summary

- Positive cases: `AC-1` (centrality 3 fixtures), `AC-2` (bead_count tapered_wedge), `AC-3` (propagation 3 fixtures), `AC-4` (generateToolpaths tapered_wedge), `AC-5` (ExtrusionLine round-trip + schema bump), `AC-6` (stitch primary preservation), `AC-7` (simplify vertex count), `AC-8` (removeSmall primary preservation), `AC-9` (arachne-perimeters real wire-up simple-square), `AC-10` (4 Arachne parity fixtures green), `AC-11` (M2 deviations closed), `AC-12` (architecture doc updated), `AC-13` (workspace test ceremony green).
- Negative cases: `AC-N1` (bead_count requires centrality), `AC-N2` (ExtrusionLine pre-bump JSON deserializes), `AC-N3` (removeSmall all-primary invariant).
- Refinements not captured in Given/When/Then:
  - The schema-version bump in AC-5 is additive (`#[serde(default)]` on new optional fields). Likely path: 4.4.0 → 4.5.0 if P109 closed at 4.4.0; verify the post-P109 value at packet activation.
  - The cube_4color Arachne extension fixture under T-231 reuses `crates/slicer-runtime/tests/fixtures/perimeter_parity/cube_4color_orca.gcode` (recorded by P109 / T-P96-C3) — Arachne wired against this fixture MUST produce the same parity result, validating the per-color preprocessing chain from P110 + this packet's wire-up.
  - T-234 (workspace ceremony) is dispatched to a sub-agent per CLAUDE.md (`FACT pass/fail` return). The implementer does NOT absorb the full output.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Cross-crate compile after IR + arachne-perimeters edits | FACT pass/fail; SNIPPETS ≤ 20 lines on fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clippy gate | FACT pass/fail |
| `cargo test -p slicer-core centrality 2>&1 \| tee target/test-output.log` | AC-1 | FACT pass/fail per fixture |
| `cargo test -p slicer-core bead_count 2>&1 \| tee target/test-output.log` | AC-2 + AC-N1 | FACT pass/fail |
| `cargo test -p slicer-core propagation 2>&1 \| tee target/test-output.log` | AC-3 | FACT pass/fail per fixture |
| `cargo test -p slicer-core generate_toolpaths 2>&1 \| tee target/test-output.log` | AC-4 | FACT pass/fail |
| `cargo test -p slicer-ir extrusion_line 2>&1 \| tee target/test-output.log` | AC-5 + AC-N2 | FACT pass/fail |
| `cargo test -p slicer-core stitch 2>&1 \| tee target/test-output.log` | AC-6 | FACT pass/fail |
| `cargo test -p slicer-core simplify 2>&1 \| tee target/test-output.log` | AC-7 | FACT pass/fail |
| `cargo test -p slicer-core remove_small 2>&1 \| tee target/test-output.log` | AC-8 + AC-N3 | FACT pass/fail |
| `cargo xtask build-guests --check` | Guest WASM coherence after IR + arachne-perimeters edits | FACT clean / STALE list |
| `cargo test -p slicer-runtime --test executor arachne_perimeters_simple_square_produces_walls 2>&1 \| tee target/test-output.log` | AC-9 | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration arachne_perimeter_parity 2>&1 \| tee target/test-output.log` | AC-10 (4 fixtures + cube_4color Arachne) | FACT pass/fail per fixture |
| `for d in D-7 D-9 D-15; do rg -q "$d.*CLOSED\|$d.*closed" docs/DEVIATION_LOG.md \|\| { echo "MISSING $d"; exit 1; }; done` | AC-11 | FACT pass per deviation |
| `! rg -q 'iterative-inset width approximation' docs/01_system_architecture.md && rg -q 'Voronoi.*SkeletalTrapezoidation.*BeadingStrategy' docs/01_system_architecture.md` | AC-12 | FACT pass/fail |
| `cargo test --workspace 2>&1 \| tee target/test-output.log \| tail -5` | T-234 / AC-13 closure ceremony | FACT (summary line + count) |

## Step Completion Expectations

- Cross-step invariant: every prior M1 + M2 packet's regression tests must stay green throughout. If a prior test fails after the `arachne-perimeters::run_perimeters` real wire-up, it's a signal that the new module's claims (perimeter-generator) collide with another module's claims; the DAG validation from P110's AC-N2 should catch this — investigate before patching.
- Step ordering rationale: extrusion-generation primitives (Steps 1–7) → IR types (Step 8 — additive change, no break) → real wire-up (Step 9 — replaces placeholder) → parity fixtures (Step 10 — depends on wire-up to slice meaningful output) → deviation walk + docs (Step 11) → workspace ceremony (Step 12 — final gate).
- Shared scratch state: the 4 Arachne parity JSON fixtures + cube_4color Arachne reference are written once in Step 10. Subsequent steps must not edit them. If Step 11 or Step 12 reveals a regression that would make a fixture stale, the implementer halts and traces the regression (do NOT just re-record).
- T-234 (workspace ceremony) MUST be the last step. If it fails, the closure log records the failure mode and the packet stays in-progress; the implementer does NOT flip status to `implemented` until the suite is green.

## Context Discipline Notes

- This packet has 12 steps — the heaviest M2 packet. The largest is Step 9 (real wire-up + the arachne_perimeters_simple_square test).
- `crates/slicer-ir/src/slice_ir.rs` is ~1700 LOC — range-read by `rg -n 'ExtrusionLine\|ExtrusionJunction\|Point3WithWidth\|CURRENT_SLICE_IR_SCHEMA_VERSION'`.
- `crates/slicer-runtime/tests/integration/perimeter_parity.rs` (from P109) — read full at Step 10 to extend; the file is small (≤ 200 LOC at P109 close).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp` (~3000 LOC) — multiple SUMMARY dispatches across Steps 1–4. Each capped at 200 words.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp` (~2500 LOC) — SUMMARY dispatches at Steps 5–7. Each capped at 150 words.
- Likely temptation: re-read OrcaSlicer source to disambiguate generateToolpaths edge cases. **Use the SUMMARY dispatch + the recorded golden fixtures** — the goldens are the source of truth for parity. If a function can't make a golden green after 2 attempts, re-dispatch a tighter SUMMARY for that specific edge case.
- Sub-agent return-format for the heaviest dispatch: `generateToolpaths` SUMMARY MUST be ≤ 200 words. If it returns > 250, re-dispatch tighter focused on the inset-emission loop body.
- T-234 (workspace ceremony) MUST be dispatched. The implementer does NOT absorb >200 lines of cargo output — sub-agent returns FACT pass/fail + summary line + count.
