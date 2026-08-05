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
---

# 106_overhang-pipeline-prepass-foundation

## Goal

Establish the PrePass-side foundation of the overhang pipeline restructuring: author ADR-0031, add `OverhangRegion.xy_footprint` + a per-layer overhang quartile polygons extension on `SurfaceClassificationIR`, extract mesh cross-section helpers to a shared `slicer-core` module (net-new; sourced from `crates/slicer-core/src/triangle_mesh_slicer.rs`), declare and implement a new `PrePass::OverhangAnnotation` host stage that populates the new IR fields via mesh cross-sections at each layer Z, and wire the stage into the runtime's prepass schedule — so that all Tier 2 consumers (perimeter modules, future fuzzy-skin overhang variants, infill modules) can read pre-classified overhang data without the cross-layer access Tier 2 forbids.

## Problem Statement

The current `overhang-classifier-default` at `PostPass::LayerFinalization` (per ADR-0008) classifies per-entity worst-case quartile via prior-layer **wall** geometry. The placement is correct for the wall-geometry-based algorithm because Tier 2 parallel-per-layer execution prevents cross-layer access. But the algorithm produces per-entity granularity (loses precision vs OrcaSlicer's per-segment classification), couples classification to speed-factor application (no downstream consumer can read the quartile), and leaves `Point3WithWidth.overhang_quartile` as dead IR. The perimeter parity roadmap's P104 ships `Point3WithWidth.overhang_quartile = None` indefinitely, and P108 cannot wire `extra_perimeters_on_overhangs` (T-077) because the data flow that would feed it doesn't exist.

The architecturally correct version uses **mesh cross-sections** — a 2D slice of the mesh at each layer Z plane, derived purely from `MeshIR` + `LayerPlanIR`. This runs at PrePass time with full mesh access and no cross-layer constraint. The classifier produces per-layer quartile polygon partitions that downstream Tier 2 modules consume by point-in-polygon. ADR-0008's finalization-tier reasoning remains valid for **speed-factor application** (the action `overhang-classifier-default` takes is still a finalization mutation); only the classification step moves.

This packet lands the PrePass foundation: ADR-0031 (supersedes ADR-0008's "unnecessary scope" caveat — not the whole ADR), the IR additions (xy_footprint + quartile polygons), the new `mesh_cross_section.rs` wrapper around `triangle_mesh_slicer::slice_mesh_ex` (used by `OverhangAnnotation`; `support_geometry.rs` is unchanged), the classifier algorithm itself, and the stage wiring. View accessors and the `overhang-classifier-default` refactor are deferred to P107 so each packet stays a coherent vertical slice.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- ADR-0008 invariant preserved: speed-factor application stays a `PostPass::LayerFinalization` concern. This packet does NOT touch `overhang-classifier-default` (P107 work).
- ADR-0031 invariant introduced: classification uses mesh cross-sections, not wall geometry. The classifier reads only `MeshIR` + `LayerPlanIR` — no per-layer slice data, no Tier 2 per-region data.
- Schema-version contract: bump `CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION` (currently `1.1.0`) additively (→ `1.2.0`). `CURRENT_SLICE_IR_SCHEMA_VERSION` (currently `4.3.0`) is NOT bumped — one IR, one constant. `#[serde(default)]` on the new fields preserves old fixtures.
- WIT type identity: `OverhangRegion` record gains `xy-footprint` field; `SurfaceClassificationIR` gains `overhang-quartile-polygons` accessor (host-only — no guest reads of this top-level IR; guests get the per-region projection via P107's view accessors).
- Stage ordering invariant: `PrePass::OverhangAnnotation` MUST execute after both `MeshAnalysis` and `LayerPlanning` commit. The scheduler validates this; AC-N2 covers the violation case.
- The classifier is deterministic: same `MeshIR` + same `LayerPlanIR` + same threshold config → same `overhang_quartile_polygons`.

## Data and Contract Notes

- IR or manifest contracts touched: `OverhangRegion` gains a field; `SurfaceClassificationIR` gains a field. Additive via `#[serde(default)]`. Schema version bumps minor.
- WIT boundary considerations: `OverhangRegion` is a leaf record inside `surface-classification-ir`; the `xy-footprint` field add is backward-compatible. `overhang-quartile-polygons` is a HashMap accessor — pick the WIT representation (likely `list<tuple<u32, list<quartile-band>>>` or a dedicated record type).
- Determinism or scheduler constraints: `overhang_annotation` is a pure function over `MeshIR` + `LayerPlanIR` + threshold config. Deterministic. The new stage has explicit precondition declarations on `MeshAnalysis` + `LayerPlanning`; the scheduler validates ordering.
- Quartile threshold defaults: `line_width × {0.5, 1.0, 1.5, 2.0}` per O-4 default. `line_width` comes from the `outer_wall_line_width` config (or `line_width` fallback if outer/inner widths are not registered yet — i.e., before P105 ships its width split).

## Locked Assumptions and Invariants

- `OverhangRegion.xy_footprint` (net-new field) mirrors `BridgeRegion.xy_footprint` (which exists at line ~581 in `slice_ir.rs`): per-region 2D projection of the underlying facets, populated at `MeshAnalysis`. P104/P107/P108 are downstream consumers of this field.
- `overhang_quartile_polygons` outer key is the global layer index (`u32`). Inner Vec carries one `QuartileBand` per quartile (1, 2, 3, 4); a layer with no overhang has an empty Vec at that key (or the key is absent — both semantics are valid; pick one and document).
- The classifier classifies based on distance to **previous-layer cross-section**, NOT to previous-layer walls. The threshold band formula uses `outer_wall_line_width × {0.5, 1.0, 1.5, 2.0}` (or `line_width` if width split isn't shipped yet).
- The new stage runs strictly after `MeshAnalysis` + `LayerPlanning` and strictly before any Tier 2 stage. It does not depend on `PaintSegmentation` or any other PrePass stage.
- ADR-0031 supersedes only ADR-0008's "unnecessary scope" caveat — NOT ADR-0008 in its entirety. The speed-factor-application-at-finalization decision stays.

## Risks and Tradeoffs

- Schema-bump race with P105: if P105 lands first at 4.3.0, this packet bumps to 4.4.0. If this packet lands first (less likely given dependency on P105's MMU foundation work — verify), it bumps to 4.3.0 and P105's bump becomes 4.4.0. Doc-impact greps allow either ordering; document in closure log.
- Mesh cross-section helper signature: the implementer must preserve the exact semantics of `support_geometry.rs`'s existing implementation. AC-4's `prepass_support_geometry_tdd` is the regression bed; if it fails after promotion, the implementer halts and diagnoses before continuing.
- `line_width` vs `outer_wall_line_width` for threshold formula: depends on P105 sequencing. If P105 ships first with the outer/inner width split, this packet uses `outer_wall_line_width`. Otherwise, fallback to `line_width`. Documented in `OverhangAnnotation` config-read code.
- Quartile threshold formula deviation: if OrcaSlicer SUMMARY surfaces a formula different from `× {0.5, 1.0, 1.5, 2.0}`, the implementer documents the deviation in the closure log and the threshold-defaulting code; AC-5's tolerance covers small differences.
