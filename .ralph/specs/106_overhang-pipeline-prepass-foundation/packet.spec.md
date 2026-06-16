---
status: draft
packet: 106_overhang-pipeline-prepass-foundation
task_ids:
  - O-T001
  - O-T002
  - O-T010
  - O-T011
  - O-T012
  - O-T020
  - O-T021
  - O-T022
  - O-T023
backlog_source: docs/specs/overhang-pipeline-restructuring.md
context_cost_estimate: M
---

# Packet Contract: 106_overhang-pipeline-prepass-foundation

## Goal

Establish the PrePass-side foundation of the overhang pipeline restructuring: author ADR-0012, add `OverhangRegion.xy_footprint` + a per-layer overhang quartile polygons extension on `SurfaceClassificationIR`, promote mesh cross-section helpers to a shared `slicer-core` module, declare and implement a new `PrePass::OverhangAnnotation` host stage that populates the new IR fields via mesh cross-sections at each layer Z, and wire the stage into the runtime's prepass schedule — so that all Tier 2 consumers (perimeter modules, future fuzzy-skin overhang variants, infill modules) can read pre-classified overhang data without the cross-layer access Tier 2 forbids.

## Scope Boundaries

Touches `docs/adr/0012-overhang-classification-at-prepass.md` (new ADR), `crates/slicer-ir/src/slice_ir.rs` (`OverhangRegion.xy_footprint` field + per-layer quartile polygons extension on `SurfaceClassificationIR`), `crates/slicer-schema/wit/deps/ir-types.wit` (mirrors), `crates/slicer-core/src/algos/mesh_analysis.rs` (populate `xy_footprint` for the existing `OverhangRegion` construction site), `crates/slicer-core/src/algos/mesh_cross_section.rs` (NEW — promoted from `support_geometry.rs`), `crates/slicer-core/src/algos/overhang_annotation.rs` (NEW — the PrePass classifier), and the runtime's prepass scheduler. View accessors, perimeter consumption, and `overhang-classifier-default` refactor are out of scope for this packet — those land in P107.

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

- **AC-1. Given** the new ADR file at `docs/adr/0012-overhang-classification-at-prepass.md`, **when** it is inspected, **then** it documents (a) the changed algorithm — mesh cross-sections vs walls — (b) the changed use case — multiple Tier 2 consumers — (c) what remains valid from ADR-0008 — speed-factor application stays at finalization — and (d) explicitly supersedes ADR-0008's "unnecessary scope" caveat. | `rg -q 'changed algorithm.*mesh cross-section' docs/adr/0012-overhang-classification-at-prepass.md && rg -q 'supersedes.*ADR-0008.*unnecessary scope' docs/adr/0012-overhang-classification-at-prepass.md`
- **AC-2. Given** the resolved roadmap decisions, **when** `docs/specs/overhang-pipeline-restructuring.md` is inspected post-packet, **then** every open decision O-1 through O-8 is marked CLOSED with the resolution captured inline (defaults: new ADR-0012 per O-1; extend SurfaceClassificationIR per O-2; promote mesh cross-section per O-3; line_width-derived thresholds per O-4; after MeshAnalysis+LayerPlanning per O-5; keep overhang-classifier-default per O-6; polygon partition per O-7; fold D-12 per O-8). | `! rg -q '^\| O-[1-8] \| [^~]' docs/specs/overhang-pipeline-restructuring.md`
- **AC-3. Given** the IR additions, **when** `crates/slicer-ir/src/slice_ir.rs` is inspected, **then** `OverhangRegion` carries `pub xy_footprint: Vec<ExPolygon>` (mirroring `BridgeRegion.xy_footprint` pattern), and `SurfaceClassificationIR` carries `pub overhang_quartile_polygons: HashMap<u32, Vec<QuartileBand>>` where `QuartileBand { pub quartile: u8, pub polygons: Vec<ExPolygon> }`, and `CURRENT_SLICE_IR_SCHEMA_VERSION` bumps additively (4.3.0 → 4.4.0 or current+1 — additive, `#[serde(default)]` migration adapter preserves old fixtures). | `rg -q 'pub xy_footprint: Vec<ExPolygon>' crates/slicer-ir/src/slice_ir.rs && rg -q 'pub overhang_quartile_polygons: HashMap<u32, Vec<QuartileBand>>' crates/slicer-ir/src/slice_ir.rs && rg -q 'pub struct QuartileBand' crates/slicer-ir/src/slice_ir.rs`
- **AC-4. Given** the promoted helper, **when** `crates/slicer-core/src/algos/mesh_cross_section.rs` is inspected, **then** it exposes `pub fn cross_section_at_z(mesh: &MeshIR, z: f32) -> Vec<ExPolygon>` (or equivalent signature), and `crates/slicer-core/src/algos/support_geometry.rs` consumes the promoted helper (no longer carries the inline plane-triangle intersection); existing `support_geometry` tests stay green. | `rg -q 'pub fn cross_section_at_z' crates/slicer-core/src/algos/mesh_cross_section.rs && cargo test -p slicer-core --test prepass_support_geometry_tdd 2>&1 | tee target/test-output.log`
- **AC-5. Given** the new classifier algorithm at `crates/slicer-core/src/algos/overhang_annotation.rs`, **when** an overhang-ramp reference fixture is sliced (a 10 mm × 10 mm × 10 mm cube with one face at 45° overhang), **then** the per-layer quartile polygons produce the expected band partition: each layer Z gets a 4-band partition of its 2D footprint with band 1 (closest to support) furthest from the previous-layer cross-section and band 4 (most overhanging) closest to the overhang edge; tolerances calibrated for `line_width = 0.4 mm` (band thresholds `line_width × {0.5, 1.0, 1.5, 2.0}`). | `cargo test -p slicer-core --test overhang_annotation_ramp_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-6. Given** the stage wiring, **when** the runtime executes prepass for the overhang-ramp fixture, **then** `PrePass::OverhangAnnotation` runs strictly after `PrePass::MeshAnalysis` and `PrePass::LayerPlanning` (verifiable by stage-order trace), and the Blackboard's `SurfaceClassificationIR` carries non-empty `overhang_quartile_polygons` for at least one layer index post-PrePass. | `cargo test -p slicer-runtime --test executor prepass_overhang_annotation_stage_order_tdd -- --nocapture 2>&1 | tee target/test-output.log`

## Negative Test Cases

- **AC-N1. Given** a flat-top cube mesh (no overhang facets), **when** `OverhangRegion` construction runs at `MeshAnalysis`, **then** `xy_footprint` is `Vec::new()` (empty Vec; not panicking on empty facet set; not `None`); and `PrePass::OverhangAnnotation` produces an empty `overhang_quartile_polygons` HashMap. | `cargo test -p slicer-core --test overhang_annotation_no_overhang_case -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. Given** the new stage being scheduled BEFORE `PrePass::LayerPlanning` (a contract violation — incorrect stage order), **when** the runtime initialises, **then** scheduling validation rejects the configuration with a deterministic error naming the violating dependency. | `cargo test -p slicer-runtime --test executor prepass_overhang_annotation_stage_order_tdd violation_case -- --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-core --test overhang_annotation_ramp_tdd overhang_annotation_no_overhang_case && cargo test -p slicer-runtime --test executor prepass_overhang_annotation_stage_order_tdd`

## Authoritative Docs

- `docs/specs/overhang-pipeline-restructuring.md` — Phase 0/1/2 task rows + open decisions O-1..O-8. Range-read the relevant phases.
- `docs/adr/0008-overhang-as-finalization-module.md` — the predecessor ADR being partially superseded (read full; ~30 lines).
- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — D-10 + D-12 closure entries (referenced by ADR-0012). Range-read those rows.
- `docs/01_system_architecture.md` — Tier 1 prepass block (range-read §"Tier 1 — PrePass"; ~30 lines).
- `docs/02_ir_schemas.md` — `OverhangRegion`, `BridgeRegion`, `SurfaceClassificationIR`, schema-versioning contract. Delegate SUMMARY.
- `CLAUDE.md` — §"Guest WASM Staleness" + §"WIT/Type Changes Checklist".

## Doc Impact Statement (Required)

- `docs/adr/0012-overhang-classification-at-prepass.md` (NEW) — authored by O-T001 — `rg -q 'changed algorithm.*mesh cross-section' docs/adr/0012-overhang-classification-at-prepass.md`
- `docs/specs/overhang-pipeline-restructuring.md` — close O-1..O-8 entries inline — `! rg -q '^\| O-[1-8] \| [^~]' docs/specs/overhang-pipeline-restructuring.md`
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
