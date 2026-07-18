---
status: implemented
packet: 106_overhang-pipeline-prepass-foundation
task_ids:
  - O-T001
  - O-T002
  - O-T010
  - O-T011
  - O-T012
  - O-T020
  - O-T021
  - O-T031
  - O-T023
backlog_source: docs/specs/overhang-pipeline-restructuring.md
context_cost_estimate: M
---

# Packet Contract: 106_overhang-pipeline-prepass-foundation

## Goal

Establish the PrePass-side foundation of the overhang pipeline restructuring: author ADR-0031, add `OverhangRegion.xy_footprint` + a per-layer overhang quartile polygons extension on `SurfaceClassificationIR`, extract mesh cross-section helpers to a shared `slicer-core` module (net-new; sourced from `crates/slicer-core/src/triangle_mesh_slicer.rs`), declare and implement a new `PrePass::OverhangAnnotation` host stage that populates the new IR fields via mesh cross-sections at each layer Z, and wire the stage into the runtime's prepass schedule — so that all Tier 2 consumers (perimeter modules, future fuzzy-skin overhang variants, infill modules) can read pre-classified overhang data without the cross-layer access Tier 2 forbids.

## Scope Boundaries

Touches `docs/adr/0031-overhang-classification-at-prepass.md` (new ADR), `crates/slicer-ir/src/slice_ir.rs` (`OverhangRegion.xy_footprint` field (net-new) + `QuartileBand` type (net-new) + per-layer quartile polygons extension on `SurfaceClassificationIR` (net-new)), `crates/slicer-schema/wit/deps/ir-types.wit` (mirrors), `crates/slicer-core/src/algos/mesh_analysis.rs` (populate `xy_footprint` for the existing `OverhangRegion` construction site), `crates/slicer-core/src/algos/mesh_cross_section.rs` (NEW — wraps `slice_mesh_ex` from `crates/slicer-core/src/triangle_mesh_slicer.rs`; does NOT promote from `support_geometry.rs` which contains no plane-triangle intersection code), `crates/slicer-core/src/algos/overhang_annotation.rs` (NEW — the PrePass classifier), and the runtime's prepass scheduler (`crates/slicer-scheduler/src/execution_plan.rs` STAGE_ORDER array). View accessors, perimeter consumption, and `overhang-classifier-default` refactor are out of scope for this packet — those land in P107.

## Prerequisites and Blockers

- Depends on:
  - **P102 (perimeter foundations)** — schema-version conventions, `slicer-ir` baseline.
  - The infill-fill-partition Phase 2.0 (already landed, verified at packet 102 generation time).
- Unblocks:
  - **P107 (overhang consumers + refactor)** — depends on the IR additions + populated data this packet lands.
  - **P108 (perimeter special modes + seam)** — T-077 (`extra_perimeters_on_overhangs`) becomes a real consumer once this packet + P107 ship.
  - Future overhang-aware modules (fuzzy-skin variants, overhang-specialised infill patterns).
- Activation blockers: none — overhang-pipeline-restructuring roadmap's open decisions O-1 through O-8 are resolved at Step 1 with documented defaults (per the roadmap's existing default-if-unanswered column).

## Acceptance Criteria

- **AC-1. Given** the new ADR file at `docs/adr/0031-overhang-classification-at-prepass.md`, **when** it is inspected, **then** it documents (a) the changed algorithm — mesh cross-sections vs walls — (b) the changed use case — multiple Tier 2 consumers — (c) what remains valid from ADR-0008 — speed-factor application stays at finalization — and (d) explicitly supersedes ADR-0008's "unnecessary scope" caveat. | `rg -q 'changed algorithm.*mesh cross-section' docs/adr/0031-overhang-classification-at-prepass.md && rg -q 'supersedes.*ADR-0008.*unnecessary scope' docs/adr/0031-overhang-classification-at-prepass.md`
- **AC-2. Given** the resolved roadmap decisions, **when** `docs/specs/overhang-pipeline-restructuring.md` is inspected post-packet, **then** every open decision O-1 through O-8 is marked CLOSED with the resolution captured inline (defaults: new ADR-0031 per O-1; extend SurfaceClassificationIR per O-2; extract mesh cross-section wrapper from `triangle_mesh_slicer.rs` per O-3; line_width-derived thresholds per O-4; after MeshAnalysis+LayerPlanning per O-5; keep overhang-classifier-default per O-6; polygon partition per O-7; fold D-12 per O-8). | `! rg -q '^\| O-[1-8] \| [^~]' docs/specs/overhang-pipeline-restructuring.md`
- **AC-3. Given** the IR additions, **when** `crates/slicer-ir/src/slice_ir.rs` is inspected, **then** `OverhangRegion` carries `pub xy_footprint: Vec<ExPolygon>` (net-new, mirroring `BridgeRegion.xy_footprint` pattern at line ~581), and `SurfaceClassificationIR` carries `pub overhang_quartile_polygons: HashMap<u32, Vec<QuartileBand>>` where `QuartileBand { pub quartile: u8, pub polygons: Vec<ExPolygon> }` (net-new; P107 consumer will use the same `QuartileBand` name), and `CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION` bumps additively from `1.1.0` (additive, `#[serde(default)]` migration adapter preserves old fixtures). `CURRENT_SLICE_IR_SCHEMA_VERSION` is NOT bumped by this packet — that constant governs `SliceIR`, not `SurfaceClassificationIR`. | `rg -q 'pub xy_footprint: Vec<ExPolygon>' crates/slicer-ir/src/slice_ir.rs && rg -q 'pub overhang_quartile_polygons: HashMap<u32, Vec<QuartileBand>>' crates/slicer-ir/src/slice_ir.rs && rg -q 'pub struct QuartileBand' crates/slicer-ir/src/slice_ir.rs && rg -q 'CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION' crates/slicer-ir/src/slice_ir.rs`
- **AC-4. Given** the new `mesh_cross_section.rs` module, **when** it is inspected, **then** it exposes `pub fn cross_section_at_z(mesh: &MeshIR, z: f32) -> Vec<ExPolygon>` (or equivalent signature). NOTE: this module wraps `slice_mesh_ex` from `crates/slicer-core/src/triangle_mesh_slicer.rs` — it does NOT "promote from support_geometry.rs" (that file contains no plane-triangle intersection code; it works from SliceIR/LayerPlanIR polygons). `support_geometry.rs` is unchanged by this step. | `rg -q 'pub fn cross_section_at_z' crates/slicer-core/src/algos/mesh_cross_section.rs`
- **AC-5. Given** the new classifier algorithm at `crates/slicer-core/src/algos/overhang_annotation.rs`, **when** an overhang-ramp reference fixture is sliced (a 10 mm × 10 mm × 10 mm cube with one face at 45° overhang), **then** the per-layer quartile polygons produce the expected band partition: each layer Z gets a 4-band partition of its 2D footprint with band 1 (closest to support) furthest from the previous-layer cross-section and band 4 (most overhanging) closest to the overhang edge; tolerances calibrated for `line_width = 0.4 mm` (band thresholds `line_width × {0.5, 1.0, 1.5, 2.0}`). | `cargo test -p slicer-core --test overhang_annotation_ramp_tdd --features host-algos -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-6. Given** the stage wiring, **when** the runtime executes prepass for the overhang-ramp fixture, **then** `PrePass::OverhangAnnotation` runs strictly after `PrePass::MeshAnalysis` and `PrePass::LayerPlanning` (verifiable by stage-order trace), and the Blackboard's `SurfaceClassificationIR` carries non-empty `overhang_quartile_polygons` for at least one layer index post-PrePass. | `cargo test -p slicer-runtime --test executor prepass_overhang_annotation_stage_order_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-7. Given** the canonical scheduler STAGE_ORDER, **when** `docs/04_host_scheduler.md` is inspected, **then** `PrePass::OverhangAnnotation` appears in the STAGE_ORDER list AFTER `PrePass::LayerPlanning`, AND `docs/01_system_architecture.md` PrePass Stage Order also lists it. | `rg -q 'PrePass::OverhangAnnotation' docs/04_host_scheduler.md && rg -q 'OverhangAnnotation' docs/01_system_architecture.md`

## Negative Test Cases

- **AC-N1. Given** a flat-top cube mesh (no overhang facets), **when** `OverhangRegion` construction runs at `MeshAnalysis`, **then** `xy_footprint` is `Vec::new()` (empty Vec; not panicking on empty facet set; not `None`); and `PrePass::OverhangAnnotation` produces an empty `overhang_quartile_polygons` HashMap. | `cargo test -p slicer-core --test overhang_annotation_no_overhang_case --features host-algos -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. Given** the new stage being scheduled BEFORE `PrePass::LayerPlanning` (a contract violation — incorrect stage order), **when** the runtime initialises, **then** scheduling validation rejects the configuration with a deterministic error naming the violating dependency. | `cargo test -p slicer-runtime --test executor prepass_overhang_annotation_stage_order_tdd violation_case -- --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-core --test overhang_annotation_ramp_tdd --features host-algos 2>&1 | tee target/test-output.log`
- `cargo test -p slicer-core --test overhang_annotation_no_overhang_case --features host-algos 2>&1 | tee target/test-output.log`
- `cargo test -p slicer-runtime --test executor prepass_overhang_annotation_stage_order_tdd 2>&1 | tee target/test-output.log`
- `cargo xtask build-guests --check`

## Authoritative Docs

- `docs/specs/overhang-pipeline-restructuring.md` — Phase 0/1/2 task rows + open decisions O-1..O-8. Range-read the relevant phases.
- `docs/adr/0008-overhang-as-finalization-module.md` — the predecessor ADR being partially superseded (read full; ~30 lines).
- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — D-10 + D-12 closure entries (referenced by ADR-0031). Range-read those rows.
- `docs/01_system_architecture.md` — Tier 1 prepass block (range-read §"Tier 1 — PrePass"; ~30 lines).
- `docs/02_ir_schemas.md` — `OverhangRegion`, `BridgeRegion`, `SurfaceClassificationIR`, schema-versioning contract. Delegate SUMMARY.
- `CLAUDE.md` — §"Guest WASM Staleness" + §"WIT/Type Changes Checklist".

## Doc Impact Statement (Required)

- `docs/adr/0031-overhang-classification-at-prepass.md` (NEW) — authored by O-T001 — `rg -q 'changed algorithm.*mesh cross-section' docs/adr/0031-overhang-classification-at-prepass.md`
- `docs/specs/overhang-pipeline-restructuring.md` — close O-1..O-8 entries inline — `! rg -q '^\| O-[1-8] \| [^~]' docs/specs/overhang-pipeline-restructuring.md`
- `docs/04_host_scheduler.md` — STAGE_ORDER updated to include `PrePass::OverhangAnnotation` after `PrePass::LayerPlanning`; stage description paragraph + Stage Prerequisites table entry added — `rg -q 'PrePass::OverhangAnnotation' docs/04_host_scheduler.md`
- `docs/01_system_architecture.md` §"Tier 1 — PrePass" — add `PrePass::OverhangAnnotation` to the stage block (after MeshAnalysis + LayerPlanning) — `rg -q 'PrePass::OverhangAnnotation' docs/01_system_architecture.md`
- `docs/02_ir_schemas.md` §"SurfaceClassificationIR" — document `OverhangRegion.xy_footprint` + `overhang_quartile_polygons` field — `rg -q 'OverhangRegion.*xy_footprint' docs/02_ir_schemas.md && rg -q 'overhang_quartile_polygons' docs/02_ir_schemas.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:159-199` — `detect_steep_overhang` reference algorithm. Delegate a SUMMARY ≤ 150 words confirming the algorithm uses **slice polygons** (not wall geometry) and the threshold formula uses `extrusion_width` × multiplier per quartile band. The Rust port uses **mesh cross-sections** (a strict superset of slice polygons) — this dispatch validates that the migration is faithful in spirit.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

## Deviations

- **2026-07-01** — WIT mirrors skipped: packet scope (line 26) listed `ir-types.wit` mirrors, but neither `OverhangRegion` nor `SurfaceClassificationIR` is mirrored anywhere in `crates/slicer-schema/wit/` — the IR is host-only per `design.md`; guest projection lands in P107 view accessors. No WIT edit was made.
- **2026-07-01** — `cross_section_at_z` signature: takes `&IndexedTriangleSet` (mm-space) instead of `&MeshIR` — `MeshIR` is a multi-object container; object/transform resolution is the caller's job. Allowed by AC-4's "or equivalent signature" clause.
- **2026-07-01** — [design.md §Data and Contract Notes] — Specified: line width from `outer_wall_line_width` with `line_width` fallback | Implemented: additional terminal fallback to hardcoded 0.4 mm when neither key is present or the value is non-Float (`overhang_annotation_producer.rs::resolve_line_width_mm`) | Reason: prepass cannot fail on absent width config; 0.4 mm matches the guest-side default used by classic/arachne perimeter modules.
- **2026-07-01** — OrcaSlicer threshold formula: Orca's actual banded overhang classification (`ExtrusionProcessor.hpp` / `GCode.cpp`) uses 6 bands at `extrusion_width × {0.1, 0.25, 0.5, 0.75, 0.87, 1.0}` applied at gcode time to wall extrusions; this packet uses the roadmap O-4 default of 4 bands at `line_width × {0.5, 1.0, 1.5, 2.0}` over mesh cross-sections (band 4 = open-ended remainder, so the 2.0 multiplier is a band label, not a boundary). Recorded per the packet's acceptance-ceremony instruction.
- **2026-07-02 (recorded retroactively, packets-102–109 review)** — ADR slot change: the overhang-classification-at-prepass ADR this packet authored was planned (and cited by P107 and both sibling roadmaps) for slot 0022, but slot 0022 had been taken by `0022-explicit-per-region-origin-for-perimeter-output-builders.md` (packet 127) before this packet landed, so the ADR shipped as `docs/adr/0031-overhang-classification-at-prepass.md`. The slot change was not recorded here at close, leaving live docs (`perimeter-modules-orca-parity-roadmap.md`, `overhang-pipeline-restructuring.md`, `docs/01`, `docs/07`, module doc comments) pointing at the wrong ADR file for a week; all repointed to 0031 in the review pass.
