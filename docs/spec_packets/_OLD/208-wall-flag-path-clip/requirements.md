# Requirements: 208-wall-flag-path-clip

## Packet Metadata

- Grouped task IDs: `TASK-324`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M` (contested — see the `[BLOCK]` in `design.md` §Open Questions)

## Problem Statement

`build_wall_flags` and its helper `nearest_original_vertex` (`crates/slicer-core/src/perimeter_utils.rs`) attribute paint to inner-wall vertices by mapping each inset-ring vertex to the nearest *original* contour vertex and inheriting that vertex's `PaintSemantic::Material` / `PaintSemantic::FuzzySkin` annotation. This is a Pinch 'n Print invention with no canonical precedent, recorded as DEV-126. Canonical never attributes paint per vertex at all: `group_region_by_fuzzify` (`Feature/FuzzySkin/FuzzySkin.cpp`) groups `LayerRegion` surfaces into `ExPolygons` per config, and both `apply_fuzzy_skin` overloads route the finished wall path through `Algorithm::split_line(path, r.expolygons, closed)` and act only on the runs the clip reports as `clipped`. Attribution accuracy in PnP is therefore bounded by vertex spacing rather than by the paint-region boundary — a vertex near a semantic boundary can inherit the wrong side.

Grounding against the tree changed the shape of the fix twice relative to the plan row:

1. **The plan's "needs Clipper-based line-split infrastructure that does not exist in-tree" is half right.** `split_line` genuinely has zero occurrences under `crates/` and `modules/`, but `clip_polylines` (`crates/slicer-core/src/polygon_ops.rs`) already performs open-path clipping against an `ExPolygon` set through `Clipper64` with `add_open_subject`. What it cannot do is what `split_line` exists to do: it returns only the inside runs, drops the outside runs, leaves output ordering unspecified, and carries no provenance back to the source vertex. Canonical recovers provenance by stashing the source index in the Clipper `Z` coordinate (`ClipperZUtils::ZPath`); the workspace binding `clipper2-rust` exposes `Point64 { x, y }` with **no** `Z` channel, so that carrier cannot be ported. The port is therefore a native segment/edge intersection walk, not a new Clipper call.
2. **PnP delivers no paint areas to any module in production.** The WIT surface for it already exists and is dead: `crates/slicer-schema/wit/deps/ir-types.wit` declares `record semantic-region { object-id, polygons: list<ex-polygon>, value: paint-value }` and `paint-region-layer-view.get-regions`, and `HostPaintRegionLayerView::get_regions` (`crates/slicer-wasm-host/src/host.rs`) reads `PaintRegionLayerData.regions_by_semantic` — which both production construction sites (`crates/slicer-wasm-host/src/host.rs` and `crates/slicer-wasm-host/src/dispatch.rs`) initialise to `HashMap::new()` and never write. The Rust-side `PaintRegionLayerView` (`crates/slicer-sdk/src/traits.rs`) has no `get_regions` at all; its doc comment records that the v1 `PaintRegionIR`/`SemanticRegion` types were deleted in packet 95 (D8) and that per-layer paint now travels on `SliceIR.regions[*].segment_annotations` (D14). So the canonical clip has no input today, and supplying one is part of this packet rather than a prerequisite it can assume.

The area data itself is not missing — it is discarded. `build_modifier_segment_annotations` (`crates/slicer-core/src/algos/paint_segmentation/mod.rs`) derives the per-vertex annotations by testing each contour-edge midpoint against `modifier_volumes::ModifierVolumeLayer::polygons` with `any_expolygon_contains_point`; and `segments_to_expolygons_by_color` in the same file already returns `BTreeMap<Option<PaintValue>, Vec<ExPolygon>>`. The per-vertex map is a lossy projection of `ExPolygon` data the host already holds.

## In Scope

- New module `crates/slicer-core/src/line_split.rs` (registered in `crates/slicer-core/src/lib.rs`) porting canonical `Algorithm::split_line`: `pub struct SplitJunction { p: Point2, clipped: bool, src_idx: i64 }` with `is_src()` / `get_src_index()`, and `pub fn split_line(path: &[Point2], clip: &[ExPolygon], closed: bool) -> Vec<SplitJunction>`. Carries the standard OrcaSlicer porting header from `docs/ORCASLICER_ATTRIBUTION.md`.
- New IR field `SlicedRegion.paint_areas: HashMap<PaintSemantic, Vec<(PaintValue, Vec<ExPolygon>)>>` (`crates/slicer-ir/src/slice_ir.rs`), `#[serde(default)]`, mirroring `segment_annotations`' placement and ordering discipline.
- New writer `build_modifier_paint_areas` in `crates/slicer-core/src/algos/paint_segmentation/mod.rs`, a sibling of `build_modifier_segment_annotations`, populating `paint_areas` from the same `ModifierVolumeLayer::polygons` intersected with the chain polygons.
- WIT: `record paint-area-entry` and `paint-areas: func() -> list<paint-area-entry>` on `slice-region-view` in `crates/slicer-schema/wit/deps/ir-types.wit`; host marshal beside `segment_annotations` in `crates/slicer-wasm-host/src/marshal/in_.rs`; SDK accessor beside `segment_annotations()`.
- Rewrite of `build_wall_flags` (`crates/slicer-core/src/perimeter_utils.rs`): the `inset_ring_points` / `original_polygons` parameter pair is replaced by `wall_path: Option<&[Point2]>` plus `paint_areas: &HashMap<PaintSemantic, Vec<(PaintValue, Vec<ExPolygon>)>>`; attribution runs through `split_line`; `nearest_original_vertex` is deleted.
- Both call sites: `modules/core-modules/classic-perimeters/src/lib.rs` (`build_ring_wall`) and `modules/core-modules/arachne-perimeters/src/lib.rs` (the `ring_pts_units` site).
- Test rewrites: `crates/slicer-core/tests/inner_wall_concave_reprojection_tdd.rs` is replaced by `crates/slicer-core/tests/inner_wall_paint_clip_tdd.rs` carrying the same notched-rectangle fixture and the same *outcome* assertions restated against paint areas. `crates/slicer-core/tests/inner_wall_material_boundary_tdd.rs` is kept and must continue to pass unmodified apart from the mechanical signature update, because every one of its cases already exercises the index-based fallback (`None, None`), not the reprojection path.
- New test files `crates/slicer-core/tests/line_split_tdd.rs` and `crates/slicer-core/tests/paint_areas_writer_tdd.rs`.
- Doc edits listed in `packet.spec.md` §Doc Impact Statement.

## Out of Scope

- Fuzzy-skin *jitter* generation and the `fuzzy-skin` module — this packet delivers the flag, not the displacement.
- Seam paint delivery (packet 206) and per-region shell config (packet 207); no edit to `execute_paint_segmentation`'s seam filter `is_seam_paint_semantic` or its `region_map.configs` read.
- The arachne call site's `variant_fuzzy` gap (it passes a hard-coded `false` with an in-line comment saying D14 painted-variant fuzzy skin is not wired into arachne yet). Preserve that literal; do not opportunistically fix it here.
- Hole-ring flag handling. `classic-perimeters`' `build_ring_wall` deliberately passes the honest "no annotation" default for `is_contour == false`; the clip rewrite must preserve that behaviour verbatim and must not extend attribution to holes.
- Reviving `paint-region-layer-view.get-regions`. That dead surface is diagnosed here and recorded as a rejected alternative in `design.md`; removing or populating it is a separate packet.
- Any change to `crates/slicer-core/src/polygon_ops.rs`' existing `clip_polylines` behaviour or its `polygon_ops_tdd.rs` coverage.

## Authoritative Docs

- `docs/08_coordinate_system.md` — direct range read; the clip is integer-unit arithmetic and every mm boundary must convert.
- `docs/02_ir_schemas.md` — delegate a SUMMARY of the `SlicedRegion` section; the doc is long and only that section applies.
- `docs/DEVIATION_LOG.md` — read only the DEV-126 row; never load the file whole.
- `docs/ORCASLICER_ATTRIBUTION.md` — direct read of the "Standard Porting Header" block; `line_split.rs` is a port and must carry it.
- `CLAUDE.md` §"WIT/Type Changes Checklist", §"Guest WASM Staleness", §Test Discipline (the `host-algos` feature trap) — direct read.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Algorithm/LineSplit.hpp` and `LineSplit.cpp` — the `SplitLineJunction` struct (`p`, `clipped`, `src_idx`) and the `split_line` / `do_split_line` contract, including the `closed` flag's duplicate-first-point wrapping. Borrowed shape; the `ClipperZUtils::ZPath` src-index carrier is deliberately NOT borrowed (see `design.md`).
- `OrcaSlicerDocumented/src/libslic3r/Feature/FuzzySkin/FuzzySkin.cpp` — `apply_fuzzy_skin` (Polygon and `Arachne::ExtrusionLine` overloads), `group_region_by_fuzzify`, `should_fuzzify`. Borrowed: the region-as-`ExPolygons` grouping, the rotate-to-a-non-clipped-junction step, and the run-walking loop that brackets a clipped run with un-flagged anchor points.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-9`. Refinements not stated in their Given/When/Then text: AC-4's fixture reuses the exact coordinates from the deleted `inner_wall_concave_reprojection_tdd.rs` (notched rectangle, 300000 × 100000 units, notch at `x ∈ [100000, 200000]`, `y ∈ [50000, 100000]`; 9-vertex ring with the artefact vertex at `(250000, 90000)`), so the replacement is demonstrably not weaker than the test it retires. AC-5 is the untouched-fallback guard and must be run *before* the `build_wall_flags` rewrite as well as after.
- Negative: `AC-N1` through `AC-N3`.
- Cross-packet impact: `SlicedRegion` gains a field. Packet 206 (seam paint delivery) and packet 207 (per-region shell config) both edit `crates/slicer-core/src/algos/paint_segmentation/mod.rs`; whichever lands second rebases on the other. No signature they touch is changed here.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-core --features host-algos --test line_split_tdd 2>&1 \| tail -5` | AC-1, AC-2, AC-N2 — the `split_line` port in isolation | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo test -p slicer-core --features host-algos --test paint_areas_writer_tdd 2>&1 \| tail -5` | AC-3 — the `paint_areas` writer | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test inner_wall_paint_clip_tdd 2>&1 \| tail -5` | AC-4, AC-N1, AC-N3 — clip-based attribution | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test inner_wall_material_boundary_tdd 2>&1 \| tail -5` | AC-5 — index fallback unregressed | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract inner_wall_boundary_type 2>&1 \| tail -5` | inner walls still flow through `build_wall_flags` end to end | FACT pass/fail |
| `cargo test -p classic-perimeters --test boundary_paint_tdd 2>&1 \| tail -5` | classic call site unregressed | FACT pass/fail |
| `cargo test -p arachne-perimeters --test boundary_paint_tdd 2>&1 \| tail -5` | arachne call site + the D-154 regression guard | FACT pass/fail |
| `rg -c 'nearest_original_vertex' crates/ modules/ ; test $? -eq 1` | AC-6 — helper deleted | FACT pass/fail |
| `cargo xtask build-guests --check` | guest freshness after `slicer-core` / WIT / module edits | FACT clean/STALE |
| `cargo check --workspace --all-targets` | compile gate incl. test targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |

## Step Completion Expectations

- `crates/slicer-core` uses `default = []` with `host-algos` behind a feature. **Every** `slicer-core` test command in this packet carries `--features host-algos`; a bare `-p slicer-core` run silently compiles zero tests for feature-gated files and prints `ok` (see `CLAUDE.md` §Test Discipline).
- Steps 1–2 (`line_split`) must be green before Step 5 rewrites `build_wall_flags`; the rewrite has no fallback if the primitive is wrong.
- The `SlicedRegion` field addition (Step 3) owns its struct-literal blast radius in the same step. The struct derives `Default`, so `..Default::default()` sites are safe; explicit exhaustive literals are not.
- The WIT edit (Step 4) triggers `CLAUDE.md` §"WIT/Type Changes Checklist" in full: search every `wit_host.rs` / `dispatch.rs` / `wit_guest` for the affected type, verify type identity across the component boundary, run `cargo build --tests`, and edit only the canonical source under `crates/slicer-schema/wit/`.
- After any step that edits `crates/slicer-core/**`, `crates/slicer-schema/wit/**`, `crates/slicer-ir/**`, `crates/slicer-sdk/**`, or `modules/core-modules/*/src/**`, run `cargo xtask build-guests --check` before attributing any guest or dispatch failure to the change.

## Context Discipline Notes

- `crates/slicer-core/src/perimeter_utils.rs` and `crates/slicer-core/src/polygon_ops.rs` are both long. Read only the ranged windows named in `design.md` §Read-Only Context.
- `crates/slicer-core/src/algos/paint_segmentation/mod.rs` and `crates/slicer-wasm-host/src/host.rs` are very long and must never be read whole; use the symbol-anchored windows in `design.md`.
- `crates/slicer-core/src/algos/paint_segmentation/mod.rs`, `modules/core-modules/arachne-perimeters/src/lib.rs` and `modules/core-modules/classic-perimeters/src/lib.rs` carried uncommitted working-tree modifications when this packet was authored. Ground against the tree as it stands on disk, not against a remembered diff.
