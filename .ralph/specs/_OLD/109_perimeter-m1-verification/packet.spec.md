---
status: implemented
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

Touches `crates/slicer-runtime/tests/integration/perimeter_parity.rs` (new harness) + fixtures directory, `crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs` (rename + reshape per ADR-0013), `crates/slicer-ir/src/slice_ir.rs` + WIT + host populator (delete `external_contour` field per T-P96-D — schema version: next-minor bump computed at activation from the live constant, NOT hardcoded; P105 also bumps `SliceIR` and may land first, so the target is 4.4.0 or 4.5.0 depending on ordering — additive removal with serde compat), `docs/DEVIATION_LOG.md` (close M1 entries + add D-PARITY-RESHAPE), `docs/07_implementation_status.md` (Classic parity complete), and 7 new edge-case TDD files under per-module test directories. No perimeter-module `lib.rs` changes in this packet — this is verification, baselining, and cleanup. **AMENDED IN-REVIEW (scope-boundary deviation recorded):** M1 verification surfaced defects that could not honestly be deferred, so this boundary was broken under registered deviations — `modules/core-modules/classic-perimeters/src/lib.rs` (degenerate-contour guard, D-109-M1-VERIFICATION-FIXES(b)), `modules/core-modules/seam-placer/src/lib.rs` (fatal→graceful correction, D-109-SEAM-FATAL-CORRECTED), and `crates/slicer-core/src/algos/{mesh_analysis,prepass_slice}.rs` (flat-bridge detection, D-109-M1-VERIFICATION-FIXES(a)). These are behaviour changes, not verification; each is user-authorized and logged in `docs/DEVIATION_LOG.md`. Future readers must not treat "verification only" as literally true for P109.

## Prerequisites and Blockers

- **FORWARD-DEP BLOCKERS (all status: draft as of 2026-06-19):**
  - **P104** (`status: draft`) — perimeter propagation + surface rules; must be `implemented` before the parity harness can record meaningful baselines.
  - **P105** (`status: draft`) — classic spacing + fill + MMU; supplies `bisector_edge_skip_mask: Vec<bool>` (flat per-edge per ADR-0013) and `edge_offset_for_polygon(region, poly_idx) -> usize` in `perimeter_utils`; LOCKED shape — this packet consumes both. Must be `implemented` before Step 5 (`external_contour` deletion) can proceed, and before any `bisector_edge_skip_mask` consumption is verifiable.
  - **P106** (`status: draft`) — overhang prepass foundation; must be `implemented` before overhang-fixture baselines (T-101 overhang_ramp fixture) are meaningful.
  - **P107** (`status: draft`) — overhang consumers + refactor; closes D-107-series deviations referenced in AC-6.
  - **P108** (`status: draft`) — special modes + seam; must be `implemented` before spiral-vase-cone and bridge fixtures produce parity-correct output.
  - **P102, P103** — assumed `implemented` (foundations + polygon ops); verify before activation.
- This M1-verification packet legitimately depends on the whole P102–P108 chain. The harness (T-100) structure is net-new work in this packet; recorded baselines (T-101) and the workspace ceremony (T-105) are gated on the chain completing.
- Unblocks:
  - **M2 (real Arachne)** — the parity harness becomes the regression bed for the M2 BeadingStrategy work.
- Activation blockers: P104–P108 must reach `status: implemented` before this packet activates into implementation. The packet files themselves can be drafted and reviewed independently.

## Acceptance Criteria

- **AC-1. Given** the new parity harness at `crates/slicer-runtime/tests/integration/perimeter_parity.rs`, **when** a test loads a `(mesh_path, config_path, expected_output_path)` triple, **then** the harness slices the mesh with the given config, compares the resulting `PerimeterIR` against the recorded reference via per-field tolerance (wall count exact; per-vertex XYZ within 0.005 mm; per-vertex width within 0.01 mm; loop_type/role exact), and reports per-fixture pass/fail with the failing field named. | `cargo test -p slicer-runtime --test integration perimeter_parity_harness_self_test -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-2. Given** the six recorded reference outputs under `crates/slicer-runtime/tests/fixtures/perimeter_parity/{solid_square,holed_square,multi_tool_triangle,overhang_ramp,bridge,spiral_vase_cone}/`, **when** the parity harness runs against each, **then** every fixture passes within its calibrated tolerances. | `cargo test -p slicer-runtime --test integration perimeter_parity -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-3. Given** the 7 edge-case TDD fixtures called out in the audit (3-tool polygon, inner-wall material boundary, 0/2-vertex polygon, hole-with-thin-wall, gap-fill-in-overhang, top-flagged region, first-layer override), **when** each runs, **then** the perimeter modules' output matches the asserted shape — no panics, no silent data loss, all flags propagated. | `cargo test -p slicer-runtime --test integration perimeter_edge_cases -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-4. Given** the reshaped `cube_4color_per_layer_outer_walls_fragment_by_color_with_tool_changes` test (renamed from `cube_4color_per_layer_outer_wall_count_matches_unpainted_baseline_within_one` per ADR-0013 / T-P96-A), **when** the test runs, **then** per painted layer: (a) count of distinct outer-wall extrusion sequences ≈ N distinct colors present; (b) each per-color fragment is a closed loop, and the union of the fragments' silhouette-portions covers the layer's external silhouette with no gap beyond a color-boundary line-width and no silhouette double-trace (each silhouette point owned by exactly one fragment), while the total outer-wall length materially exceeds the bare silhouette perimeter because each per-cell loop also traces interior bisector walls (Model A — NOT a single silhouette trace); (c) each fragment is preceded by a `T<N>` matching its `ToolIndex`; (d) color transitions occur at cell-partition boundary junctions (interior cell boundaries meeting the silhouette), not only at the cube's 4 outer silhouette corners, and the per-color fragments are independent closed loops rather than one merged wall whose color flips at corners. | `cargo test -p slicer-runtime --test executor cube_4color_per_layer_outer_walls_fragment_by_color_with_tool_changes -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-5. Given** the `external_contour` field deletion (T-P96-D), **when** `crates/slicer-ir/src/slice_ir.rs` is inspected, **then** `SlicedRegion` no longer carries `external_contour`, WIT does not declare the `external-contour` accessor, the host populator (`crates/slicer-wasm-host/src/host.rs`) no longer fills it, `crates/slicer-sdk/src/views.rs` no longer exposes `external_contour()` or `set_external_contour()`, and `crates/slicer-core/src/algos/paint_segmentation/bisector_ownership.rs` no longer computes or assigns it (including removal of `populate_external_contours` and its 3 tests at lines ~178-247). `CURRENT_SLICE_IR_SCHEMA_VERSION` is bumped to `4.6.0` — a **compatible-removal minor** bump. Field removal is *major* by default per `docs/02_ir_schemas.md` §"IR Versioning Contract", but this is a documented exception because all three hold: no live consumer (superseded by ADR-0013 Model-A, consumption removed in P105), serde ignores the now-absent field (old fixtures still parse), and every module declares `max_ir_schema = 5.0.0` so a `5.0.0` host would fail the scheduler's `validate_ir_versions` gate for EVERY module. (The original "additive removal" phrasing was imprecise — corrected in-review to *compatible removal*; see the Contract note in `docs/02_ir_schemas.md`.) NOTE: `bisector_edge_skip_mask: Vec<bool>` (flat per-edge, LOCKED per ADR-0013) from P105 is the mechanism that makes `external_contour` removable — this deletion MUST be forward-dep-blocked on P105 shipping. | `! rg -q 'external_contour' crates/slicer-ir/src/slice_ir.rs && ! rg -q 'external-contour' crates/slicer-schema/wit/deps/ir-types.wit && ! rg -q 'populate_external_contours\|external_contour' crates/slicer-core/src/algos/paint_segmentation/bisector_ownership.rs && rg -qU 'CURRENT_SLICE_IR_SCHEMA_VERSION: SemVer = SemVer \{\s*major: 4,\s*minor: 6' crates/slicer-ir/src/slice_ir.rs`
- **AC-6. Given** the M1 deviation closure pass (T-103) and parity re-baseline (T-P96-F), **when** deviation files are inspected, **then**: (a) `D-104-OVERHANG-QUARTILE-NONE` — will be in `docs/DEVIATION_LOG.md` once P104 ships (per P104's Doc Impact contract); T-103 adds a closure note to that entry; **FORWARD-DEP on P104**; (b) `D-10` and `D-12` live in `docs/specs/perimeter-modules-orca-parity-roadmap.md` (confirmed absent from `docs/DEVIATION_LOG.md` by grep) — T-103 closes them in the roadmap, not in the log; (c) `D-96-AC22-EXTERNAL-CONTOUR` IS in `docs/DEVIATION_LOG.md` (line 101, confirmed by grep) — this packet supersedes it in-place (the entry already says "Closed — packet 96"; add supersession note referencing ADR-0013 + P109); (d) `D-109-AC22-PARITY-RESHAPE` is registered in `docs/DEVIATION_LOG.md`. **Corrected in-review:** the original AC required recording a byte-exact `P109_CUBE_4COLOR_PARITY_SHA`, but ADR-0013 Model-A structural assertions replaced the byte-golden because the cube_4color path has documented `boostvoronoi` medial-axis non-determinism that makes a byte-SHA flaky. The deviation entry therefore records `P109_CUBE_4COLOR_PARITY_SHA` as **INTENTIONALLY not pinned** *with that rationale* — not a recorded hash. The verification below asserts the honest "not pinned" rationale is present, so the token cannot be gamed by mere presence. | `rg -q 'D-96-AC22-EXTERNAL-CONTOUR.*ADR-0013\|P109\|superseded' docs/DEVIATION_LOG.md && rg -q 'D-109-AC22-PARITY-RESHAPE' docs/DEVIATION_LOG.md && rg -q 'P109_CUBE_4COLOR_PARITY_SHA.*INTENTIONALLY not pinned' docs/DEVIATION_LOG.md && rg -q 'D-10.*CLOSED\|CLOSED.*D-10' docs/specs/perimeter-modules-orca-parity-roadmap.md && rg -q 'D-12.*CLOSED\|CLOSED.*D-12' docs/specs/perimeter-modules-orca-parity-roadmap.md`
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

- `docs/07_implementation_status.md` — mark Classic parity (M1) complete with packet IDs P102..P109 listed — `rg -q 'M1.*Classic parity.*complete|Classic parity complete.*P102.*P103.*P104.*P105.*P106.*P107.*P108.*P109' docs/07_implementation_status.md`
- `docs/DEVIATION_LOG.md` — close M1 deviations + add D-109-AC22-PARITY-RESHAPE + supersede D-96-AC22-EXTERNAL-CONTOUR — `rg -q 'D-96-AC22-EXTERNAL-CONTOUR.*superseded' docs/DEVIATION_LOG.md && rg -q 'D-109-AC22-PARITY-RESHAPE' docs/DEVIATION_LOG.md`
- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — mark Phases 1–9 as DONE in the milestone summary; flip M1 marker — `rg -q 'M1.*\bDONE\b|M1.*shipped|M1.*complete' docs/specs/perimeter-modules-orca-parity-roadmap.md`
- `docs/02_ir_schemas.md` §"Schema Versioning" — record the schema bump as a documented **compatible-removal minor** (`4.6.0`) of `external_contour`, with the three-condition rationale (no consumer, serde-tolerant, within every module's `max_ir_schema`); field removal is *major* by default but this exception is justified in the Contract note — `rg -q 'external_contour.*removed\|removed.*external_contour' docs/02_ir_schemas.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

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
