# Requirements: 106_overhang-pipeline-prepass-foundation

## Packet Metadata

- Grouped task IDs:
  - `O-T001` — Author `docs/adr/0031-overhang-classification-at-prepass.md` superseding ADR-0008's "unnecessary scope" clause
  - `O-T002` — Resolve O-1 through O-8 decisions inline in the overhang roadmap with documented defaults
  - `O-T010` — Add `xy_footprint: Vec<ExPolygon>` to `OverhangRegion`; populate at `MeshAnalysis`
  - `O-T011` — Add per-layer overhang quartile polygons to `SurfaceClassificationIR` via `overhang_quartile_polygons: HashMap<u32, Vec<QuartileBand>>`
  - `O-T012` — Promote mesh cross-section helpers from `PrePass::SupportGeometry` to shared `slicer-core/src/algos/mesh_cross_section.rs`
  - `O-T020` — Declare `PrePass::OverhangAnnotation` stage in the stage order (after MeshAnalysis + LayerPlanning); host scheduling
  - `O-T021` — Implement classifier algorithm in `slicer-core/src/algos/overhang_annotation.rs`: per consecutive layer pair, compute cross-sections, derive distance field, partition into 4 quartile bands
  - `O-T031` — Wire quartile thresholds to config (`line_width` derivation: `line_width × {0.5, 1.0, 1.5, 2.0}`)
  - `O-T023` — Host stage runner: invoke `overhang_annotation` after MeshAnalysis + LayerPlanning commit; write to Blackboard `SurfaceClassificationIR` extension field
- Backlog source: `docs/specs/overhang-pipeline-restructuring.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The current `overhang-classifier-default` at `PostPass::LayerFinalization` (per ADR-0008) classifies per-entity worst-case quartile via prior-layer **wall** geometry. The placement is correct for the wall-geometry-based algorithm because Tier 2 parallel-per-layer execution prevents cross-layer access. But the algorithm produces per-entity granularity (loses precision vs OrcaSlicer's per-segment classification), couples classification to speed-factor application (no downstream consumer can read the quartile), and leaves `Point3WithWidth.overhang_quartile` as dead IR. The perimeter parity roadmap's P104 ships `Point3WithWidth.overhang_quartile = None` indefinitely, and P108 cannot wire `extra_perimeters_on_overhangs` (T-077) because the data flow that would feed it doesn't exist.

The architecturally correct version uses **mesh cross-sections** — a 2D slice of the mesh at each layer Z plane, derived purely from `MeshIR` + `LayerPlanIR`. This runs at PrePass time with full mesh access and no cross-layer constraint. The classifier produces per-layer quartile polygon partitions that downstream Tier 2 modules consume by point-in-polygon. ADR-0008's finalization-tier reasoning remains valid for **speed-factor application** (the action `overhang-classifier-default` takes is still a finalization mutation); only the classification step moves.

This packet lands the PrePass foundation: ADR-0031 (supersedes ADR-0008's "unnecessary scope" caveat — not the whole ADR), the IR additions (xy_footprint + quartile polygons), the new `mesh_cross_section.rs` wrapper around `triangle_mesh_slicer::slice_mesh_ex` (used by `OverhangAnnotation`; `support_geometry.rs` is unchanged), the classifier algorithm itself, and the stage wiring. View accessors and the `overhang-classifier-default` refactor are deferred to P107 so each packet stays a coherent vertical slice.

## In Scope

- New ADR `docs/adr/0031-overhang-classification-at-prepass.md`. Supersedes ADR-0008's "unnecessary scope" caveat specifically; preserves ADR-0008's "speed-factor application is a finalization concern" decision.
- Closure of overhang roadmap open decisions O-1 through O-8 inline in `docs/specs/overhang-pipeline-restructuring.md` (per the roadmap's default-if-unanswered column; O-1 resolves to ADR-0031; investigation findings recorded if implementer escalates).
- IR additions in `crates/slicer-ir/src/slice_ir.rs`:
  - `OverhangRegion.xy_footprint: Vec<ExPolygon>` field (mirrors `BridgeRegion.xy_footprint`).
  - New type `QuartileBand { quartile: u8, polygons: Vec<ExPolygon> }`.
  - Extension on `SurfaceClassificationIR`: `pub overhang_quartile_polygons: HashMap<u32, Vec<QuartileBand>>` (key = layer index).
- Additive schema-version bump of `CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION` (currently `1.1.0` → next minor) with `#[serde(default)]` migration adapter. `CURRENT_SLICE_IR_SCHEMA_VERSION` (currently `4.3.0`) is NOT bumped — it governs `SliceIR`, not `SurfaceClassificationIR`.
- WIT mirrors in `crates/slicer-schema/wit/deps/ir-types.wit` for the new IR additions.
- `crates/slicer-core/src/algos/mesh_cross_section.rs` (NEW) exporting `pub fn cross_section_at_z(mesh: &MeshIR, z: f32) -> Vec<ExPolygon>` (wraps `slice_mesh_ex` from `crates/slicer-core/src/triangle_mesh_slicer.rs`; `support_geometry.rs` contains no plane-triangle intersection to promote — it operates on SliceIR/LayerPlanIR polygon data).
- `crates/slicer-core/src/algos/support_geometry.rs` is NOT changed by this packet (it has no plane-triangle intersection to remove).
- `crates/slicer-core/src/algos/mesh_analysis.rs` populates `xy_footprint` for the existing `OverhangRegion` construction site (line ~206 per packet metadata).
- `crates/slicer-core/src/algos/overhang_annotation.rs` (NEW) — the classifier producing `overhang_quartile_polygons` for the Blackboard.
- Stage declaration + scheduling: `PrePass::OverhangAnnotation` added to the `STAGE_ORDER` array in `crates/slicer-scheduler/src/execution_plan.rs` (the canonical stage order location; `stage_order.rs` only imports from `execution_plan.rs`) strictly after `MeshAnalysis` + `LayerPlanning`; host stage runner in `crates/slicer-runtime/src/prepass.rs` (or analogous) invokes the classifier and writes to Blackboard.
- 4 new TDD files covering AC-3 through AC-6 + AC-N1 + AC-N2. `mesh_analysis_overhang_xy_footprint_tdd.rs` goes alongside `crates/slicer-runtime/tests/unit/mesh_analysis_tdd.rs` and MUST be registered with a `mod mesh_analysis_overhang_xy_footprint_tdd;` line in `crates/slicer-runtime/tests/unit/main.rs` (the unit binary aggregator).
- Doc updates per the Doc Impact Statement:
  - `docs/04_host_scheduler.md` (EDIT — register `PrePass::OverhangAnnotation` in STAGE_ORDER, add stage description paragraph and Stage Prerequisites table entry)
  - `docs/01_system_architecture.md` (EDIT — register `PrePass::OverhangAnnotation` in PrePass Stage Order list, prose block, and Stage I/O Contract table)

## Out of Scope

- `SliceRegionView::overhang_areas()` accessor — already added as a stub in P104 (T-023). After this packet ships the data, the accessor returns non-empty naturally; no further accessor work in this packet.
- `SliceRegionView::overhang_quartile_polygons()` accessor — P107 (O-T031). The IR field exists post-this-packet; P107 adds the view accessor on top.
- `overhang-classifier-default` refactor (consume from IR instead of compute) — P107 (O-T040..O-T042).
- Verification harness (overhang ramp end-to-end gcode parity) — P107 (O-T050..O-T052).
- Deviation closure for D-10 / D-12 / D-OVERHANG-QUARTILE-NONE — P107 (O-T053).
- Any perimeter module change — P108 (T-077 reshape, post-rename of current P106).

## Authoritative Docs

| Doc | Size | Read strategy |
| --- | --- | --- |
| `docs/specs/overhang-pipeline-restructuring.md` | ~150 lines | Read full. |
| `docs/adr/0008-overhang-as-finalization-module.md` | ~30 lines | Read full. |
| `docs/adr/0031-overhang-classification-at-prepass.md` | NEW | Author in Step 1. ADR slot 0031 is the next free slot (0021 is highest existing). |
| `docs/specs/perimeter-modules-orca-parity-roadmap.md` | ~700 lines | Range-read D-10 + D-12 entries only. |
| `docs/01_system_architecture.md` | varies | Range-read §"Tier 1 — PrePass" (~30 lines). |
| `docs/02_ir_schemas.md` | ~900 lines | Delegate SUMMARY for `OverhangRegion`, `BridgeRegion`, `SurfaceClassificationIR`. |
| `docs/03_wit_and_manifest.md` | ~400 lines | Range-read §"WIT/Type Changes Checklist" (~30 lines). |
| `CLAUDE.md` | ~600 lines | Range-read §"Guest WASM Staleness" + §"WIT/Type Changes Checklist". |

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:159-199` — `detect_steep_overhang` reference algorithm. SUMMARY ≤ 150 words. Confirms the algorithm uses slice polygons + `extrusion_width × N` band thresholds. Our Rust port uses mesh cross-sections (functionally equivalent at a single Z; better at sub-layer resolution).

## Acceptance Summary

- Positive cases: `AC-1` (ADR-0031 authored + supersedes correctly), `AC-2` (open decisions closed), `AC-3` (IR field present + schema bump on CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION), `AC-4` (mesh_cross_section.rs created wrapping slice_mesh_ex), `AC-5` (overhang ramp classifier produces expected band partition), `AC-6` (stage runs after MeshAnalysis+LayerPlanning; Blackboard carries non-empty data).
- Negative cases: `AC-N1` (no overhang → empty Vec, no panic), `AC-N2` (stage scheduled before LayerPlanning → validation rejects).
- Refinements not captured in Given/When/Then:
  - Mesh cross-section signature: the implementer picks the exact signature based on the existing `slice_mesh_ex` slicer in `crates/slicer-core/src/triangle_mesh_slicer.rs` (NOT `support_geometry.rs`, which has no plane-triangle intersection code). AC-4's grep matches `pub fn cross_section_at_z` — if a different name (`cross_section`, `slice_at_z`) emerges from refactoring, the AC grep is updated.
  - Quartile threshold formula: `line_width × {0.5, 1.0, 1.5, 2.0}` is the documented default per O-4. If T-P96-A0 (in P105) or this packet's OrcaSlicer SUMMARY surfaces a different formula, the implementer records the deviation in the closure log.
- Cross-packet impact: depends on P102. Unblocks P107, P108, and any future overhang-aware module.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Compile after IR + WIT additions | FACT pass/fail; SNIPPETS ≤ 20 lines on fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clippy gate | FACT pass/fail |
| `cargo test -p slicer-core --test overhang_annotation_ramp_tdd` | AC-5 | FACT pass/fail |
| `cargo test -p slicer-core --test overhang_annotation_no_overhang_case` | AC-N1 | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor prepass_support_geometry` | AC-4 regression (existing test stays green after helper promotion) | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor prepass_overhang_annotation_stage_order_tdd` | AC-6 + AC-N2 | FACT pass/fail per case |
| `cargo xtask build-guests --check` | Guest WASM coherence after WIT change | FACT clean / STALE list |
| `rg -q 'PrePass::OverhangAnnotation' docs/01_system_architecture.md` | AC doc grep | FACT pass/fail |

## Step Completion Expectations

- Cross-step invariant: existing `prepass_support_geometry_tdd` and `mesh_analysis_tdd` regression tests MUST stay green after every step. The helper promotion (Step 3) and `xy_footprint` populate-site addition (Step 2) are the most likely regression vectors.
- Step ordering rationale: ADR + decisions first (Step 1) because they record the invariants Steps 2-5 rely on. IR additions (Step 2) before helper promotion (Step 3) because the IR types are referenced by the helper's tests. Classifier algorithm (Step 4) needs the helper + IR types. Stage wiring (Step 5) needs the classifier. Docs (Step 6) record what shipped.
- Shared scratch state: none.

## Context Discipline Notes

- `crates/slicer-core/src/algos/mesh_analysis.rs` is ~700 lines per packet metadata — range-read by `rg -n 'OverhangRegion|overhang_facets'` first, then ±40 lines.
- `crates/slicer-core/src/algos/support_geometry.rs` — read only the plane-triangle intersection function being promoted; do NOT load full.
- Likely temptation read: existing `overhang-classifier-default/src/classify.rs` to understand the current algorithm. **Skip** — that's the algorithm being deprecated by this restructuring; reading it risks copying the per-entity-worst-case approach instead of the new per-XY-distance approach.
- Sub-agent return-format for the heaviest dispatch: OrcaSlicer `detect_steep_overhang` SUMMARY (≤ 150 words). Must describe the band-threshold formula and the slice-polygon input. Re-dispatch if pseudocode appears.
