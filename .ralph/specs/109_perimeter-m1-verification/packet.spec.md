---
status: draft
packet: 109_perimeter-m1-verification
task_ids:
  - T-100
  - T-101
  - T-102
  - T-103
  - T-104
  - T-105
  - T-P96-A
  - T-P96-C3
  - T-P96-D
  - T-P96-F
backlog_source: docs/specs/perimeter-modules-orca-parity-roadmap.md
context_cost_estimate: M
---

# Packet Contract: 109_perimeter-m1-verification

## Goal

Close M1 of the perimeter parity roadmap: build the reference-fixture parity harness, record 6 OrcaSlicer reference outputs (solid square, holed square, multi-tool triangle, overhang ramp, bridge fixture, spiral-vase cone), reshape and re-baseline the P96 4-color cube AC-22b test to assert per-color fragmentation, delete the now-unused `external_contour` IR field, close every M1 deviation registered since P102, run the full `cargo test --workspace` ceremony, and update `docs/07_implementation_status.md` to mark Classic parity complete.

## Scope Boundaries

Touches `crates/slicer-runtime/tests/integration/perimeter_parity.rs` (new harness) + fixtures directory, `crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs` (rename + reshape per ADR-0013), `crates/slicer-ir/src/slice_ir.rs` + WIT + host populator (delete `external_contour` field per T-P96-D — schema 4.3.0 → 4.4.0 additive removal with serde compat), `docs/DEVIATION_LOG.md` (close M1 entries + add D-PARITY-RESHAPE), `docs/07_implementation_status.md` (Classic parity complete), and 7 new edge-case TDD files under per-module test directories. No perimeter-module `lib.rs` changes in this packet — this is verification, baselining, and cleanup.

## Prerequisites and Blockers

- Depends on:
  - **P102, P103, P104, P105, P106, P107, P108** — all M1 implementation packets (foundations + polygon ops + propagation + spacing/fill/MMU + overhang prepass + overhang consumers + special modes/seam) must be `status: implemented` before this packet's parity harness can record meaningful baselines.
- Unblocks:
  - **M2 (real Arachne)** — the parity harness becomes the regression bed for the M2 BeadingStrategy work.
- Activation blockers: none for packet structure. Practical block: the harness (T-100) cannot record reference outputs (T-101) until the M1 implementation packets have shipped — the recorded outputs ARE the post-implementation state.

## Acceptance Criteria

- **AC-1. Given** the new parity harness at `crates/slicer-runtime/tests/integration/perimeter_parity.rs`, **when** a test loads a `(mesh_path, config_path, expected_output_path)` triple, **then** the harness slices the mesh with the given config, compares the resulting `PerimeterIR` against the recorded reference via per-field tolerance (wall count exact; per-vertex XYZ within 0.005 mm; per-vertex width within 0.01 mm; loop_type/role exact), and reports per-fixture pass/fail with the failing field named. | `cargo test -p slicer-runtime --test integration perimeter_parity_harness_self_test -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-2. Given** the six recorded reference outputs under `crates/slicer-runtime/tests/fixtures/perimeter_parity/{solid_square,holed_square,multi_tool_triangle,overhang_ramp,bridge,spiral_vase_cone}/`, **when** the parity harness runs against each, **then** every fixture passes within its calibrated tolerances. | `cargo test -p slicer-runtime --test integration perimeter_parity -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-3. Given** the 7 edge-case TDD fixtures called out in the audit (3-tool polygon, inner-wall material boundary, 0/2-vertex polygon, hole-with-thin-wall, gap-fill-in-overhang, top-flagged region, first-layer override), **when** each runs, **then** the perimeter modules' output matches the asserted shape — no panics, no silent data loss, all flags propagated. | `cargo test -p slicer-runtime --test integration perimeter_edge_cases -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-4. Given** the reshaped `cube_4color_per_layer_outer_walls_fragment_by_color_with_tool_changes` test (renamed from `cube_4color_per_layer_outer_wall_count_matches_unpainted_baseline_within_one` per ADR-0013 / T-P96-A), **when** the test runs, **then** per painted layer: (a) count of distinct outer-wall extrusion sequences ≈ N distinct colors present; (b) union of all outer-wall extrusions covers the layer's external perimeter exactly (no gap, no double-trace within ε); (c) each fragment is preceded by a `T<N>` matching its `ToolIndex`; (d) color transitions occur at cell-boundary corners within geometric tolerance. | `cargo test -p slicer-runtime --test executor cube_4color_per_layer_outer_walls_fragment_by_color_with_tool_changes -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-5. Given** the `external_contour` field deletion (T-P96-D), **when** `crates/slicer-ir/src/slice_ir.rs` is inspected, **then** `SlicedRegion` no longer carries `external_contour`, WIT does not declare the `external-contour` accessor, the host populator no longer fills it, and `CURRENT_SLICE_IR_SCHEMA_VERSION` bumps to `4.4.0` (additive removal — `#[serde(default)]` on now-absent fields still parses old fixtures). | `! rg -q 'external_contour' crates/slicer-ir/src/slice_ir.rs && ! rg -q 'external-contour' crates/slicer-schema/wit/deps/ir-types.wit && rg -q 'pub const CURRENT_SLICE_IR_SCHEMA_VERSION: SemVer = SemVer \{ major: 4, minor: 4, patch: 0' crates/slicer-ir/src/slice_ir.rs`
- **AC-6. Given** the M1 deviation closure pass (T-103) and parity re-baseline (T-P96-F), **when** `docs/DEVIATION_LOG.md` is inspected, **then** every M1 deviation entry registered since P102 (`D-OVERHANG-QUARTILE-NONE` from P104, `D-10` + `D-12` + `D-OVERHANG-QUARTILE-NONE` closed by P107, plus any others from P105/P108) carries a closure note linking to the implementing packet OR a justified residual deviation; `D-96-AC22-EXTERNAL-CONTOUR` is marked superseded; `D-<packet>-AC22-PARITY-RESHAPE` is registered with the recorded cube_4color SHA as `P<packet>_CUBE_4COLOR_PARITY_SHA`. | `rg -q 'D-96-AC22-EXTERNAL-CONTOUR.*superseded' docs/DEVIATION_LOG.md && rg -q 'D-.*-AC22-PARITY-RESHAPE' docs/DEVIATION_LOG.md && rg -q 'P<packet>_CUBE_4COLOR_PARITY_SHA\|CUBE_4COLOR_PARITY_SHA' docs/DEVIATION_LOG.md`
- **AC-7. Given** the workspace test ceremony (T-105), **when** `cargo test --workspace` runs to completion at packet close, **then** every test passes (full suite is the M1 closure gate per `CLAUDE.md` exception). | `cargo test --workspace 2>&1 | tee target/test-output.log | tail -5`

## Negative Test Cases

- **AC-N1. Given** a deliberately-broken fixture (e.g., expected output edited to mismatch by 0.1 mm on one vertex), **when** the parity harness runs, **then** it reports the specific fixture name + the field that differs + the actual vs expected values (does NOT silently pass). | `cargo test -p slicer-runtime --test integration perimeter_parity_harness_self_test deliberate_mismatch_detection -- --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace 2>&1 | tee target/test-output.log` (T-105 / closure ceremony)

## Authoritative Docs

- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — Phase 9 (T-100..T-105), Inherited from P96 (T-P96-A, T-P96-C3, T-P96-D, T-P96-F). Range-read those rows.
- `docs/adr/0013-mmu-per-color-outer-wall-fragmentation.md` — guides AC-22b reshape language.
- `docs/02_ir_schemas.md` — schema-version contract for additive-removal bump.
- `docs/07_implementation_status.md` — M1 status entry format.
- `docs/DEVIATION_LOG.md` — M1 entries to close + format reference.
- `CLAUDE.md` — §"Test Discipline" / workspace-test exception for closure ceremony.

## Doc Impact Statement (Required)

- `docs/07_implementation_status.md` — mark Classic parity (M1) complete with packet IDs P102..P109 listed — `rg -q 'M1.*Classic parity.*complete\|Classic parity complete.*P102.*P103.*P104.*P105.*P106.*P107.*P108.*P109' docs/07_implementation_status.md`
- `docs/DEVIATION_LOG.md` — close M1 deviations + add D-PARITY-RESHAPE + supersede D-96-AC22-EXTERNAL-CONTOUR — `rg -q 'D-96-AC22-EXTERNAL-CONTOUR.*superseded' docs/DEVIATION_LOG.md && rg -q 'D-.*-AC22-PARITY-RESHAPE' docs/DEVIATION_LOG.md`
- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — mark Phases 1–9 as DONE in the milestone summary; flip M1 marker — `rg -q 'M1.*\bDONE\b\|M1.*shipped\|M1.*complete' docs/specs/perimeter-modules-orca-parity-roadmap.md`
- `docs/02_ir_schemas.md` §"Schema Versioning" — record the 4.3.0 → 4.4.0 bump as additive removal of `external_contour` — `rg -q '4\.4\.0.*external_contour\|external_contour.*removed.*4\.4\.0' docs/02_ir_schemas.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked).

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` (general parity behavior) — only used to record reference outputs for T-101's six fixtures. The implementer dispatches one SUMMARY per fixture covering the expected `PerimeterIR` shape (wall count, role distribution, loop_type distribution). No code reads. The recorded reference fixtures are JSON files, not OrcaSlicer code.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
