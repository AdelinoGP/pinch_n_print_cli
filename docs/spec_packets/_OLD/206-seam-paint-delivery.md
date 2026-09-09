---
status: implemented
packet: 206-seam-paint-delivery
task_ids:
  - TASK-322
---

# 206-seam-paint-delivery

## Goal

Deliver seam paint to the seam placer end-to-end: add a seam-annotation writer module (`crates/slicer-core/src/algos/paint_segmentation/seam_annotations.rs`) wired into `execute_paint_segmentation` so `SlicedRegion.segment_annotations` carries `PaintSemantic::Custom("seam_enforcer")` / `Custom("seam_blocker")` on every return path including the seam-only short-circuit (DEV-123 open half), replace `paint_marker`'s substring test in `paint_annotation_type` (`modules/core-modules/seam-planner-default/src/visibility.rs`) with exact seam-semantic matching so support paint stops reading as seam intent (DEV-133), and promote `seam_paint_boxes` into `slicer_core::perimeter_utils` so `modules/core-modules/arachne-perimeters/src/lib.rs` gains the `apply_seam_paint_bias` call it lacks (DEV-134).

## Problem Statement

Seam paint has a fully-ported consumer chain and no producer. `crates/slicer-model-io/src/loader.rs` decodes 3MF `paint_seam` data into `PaintSemantic::Custom("seam_enforcer")` / `Custom("seam_blocker")` `PaintLayer`s; packet 108 built the reader (`seam_paint_boxes` → `slicer_core::perimeter_utils::apply_seam_paint_bias`) in `modules/core-modules/classic-perimeters/src/lib.rs`; `paint_annotation_type` (`modules/core-modules/seam-planner-default/src/visibility.rs`) classifies annotations for the canonical `EnforcedBlockedSeamPoint` comparator. Nothing writes those semantics into `SlicedRegion.segment_annotations`. The only production writer into that map, `build_modifier_segment_annotations` (`crates/slicer-core/src/algos/paint_segmentation/mod.rs`), keys strictly on `ModifierVolumeLayer.semantic`, i.e. `SupportEnforcer` / `SupportBlocker`.

Three defects compose into one slice:

- **DEV-123 (open half).** No writer, so the whole chain is starved. The region-split half is already closed on the working tree by `is_seam_paint_semantic`, which now filters seam semantics out of `mesh_has_any_paint`, out of the `let dominant_semantic = { … }` binding block inside `execute_paint_segmentation`'s per-object scan (a `let` block, not a function — locate it by the `let dominant_semantic` line and read its enclosing braces), and out of both `painted_subsets` accumulations. That fix has a consequence the plan did not anticipate and this packet must handle: on a **seam-only** mesh `mesh_has_any_paint` returns `false`, so `execute_paint_segmentation` short-circuits at its second guard and never reaches any writer at all.
- **DEV-133.** `paint_marker` substring-matches `"enforcer"` / `"blocker"` (and `"enforced"` / `"blocked"`), so the strings `SupportEnforcer` and `SupportBlocker` classify as seam intent. Because `build_modifier_segment_annotations` is today the only writer, support paint is currently the seam planner's *only* live paint input — the exact inversion of canonical, where `gather_enforcers_blockers` (`SeamPlacer.cpp`) reads `mv->seam_facets` exclusively.
- **DEV-134.** `modules/core-modules/arachne-perimeters/src/lib.rs` calls `generate_sharp_corner_seam_candidates` but never `apply_seam_paint_bias`, so `wall_generator = arachne` would drop the bias entirely. Canonical has no such split: seam paint is applied in `SeamPlacer.cpp` on finished loops, generator-independently.

They must land together. The writer alone makes DEV-133's leak user-visible (support paint would compete with real seam paint); the writer without DEV-134 makes seam bias silently classic-only.

No prior packet is reopened or superseded. Packet 108 (T-P98-SEAM) built the reader this packet finally feeds; its contract is preserved unchanged.

## Architecture Constraints

- **The `mesh_has_any_paint` short-circuit is the load-bearing constraint, and the plan did not account for it.** `is_seam_paint_semantic` (already landed for DEV-123) filters seam semantics out of `mesh_has_any_paint`, so on a seam-only mesh `execute_paint_segmentation` returns `slice_ir.clone()` at its second guard and never reaches any writer. The seam writer therefore cannot be a sibling *call* of `build_modifier_segment_annotations` inside the per-layer loop — that loop is unreachable for the most important case. It must be an independent pass invoked on every non-trivial return path.
- The seam writer must NOT re-admit seam semantics to `mesh_has_any_paint`, to the `let dominant_semantic = { … }` binding block inside `execute_paint_segmentation` (a `let` block, not a function — locate the `let dominant_semantic` line and read its enclosing braces), or to either `painted_subsets` accumulation. Re-admitting them would revive precisely the defect DEV-123's closed half fixed: on `resources/cube_cilindrical_modifier.3mf` the whole MMU cell decomposition was labelled `seam_enforcer` and every layer below the lowest painted facet was destroyed.
- Index contract: `segment_annotations[semantic][poly_idx][vertex_idx]` is indexed against the owning `SlicedRegion.polygons[poly_idx].contour.points`. Both consumers depend on this — `seam_paint_boxes` indexes `poly.contour.points` by position, and `annotation_at` (`modules/core-modules/seam-planner-default/src/visibility.rs`) indexes `contours[contour_idx][vertex_idx]`. The writer must emit one inner `Vec` per region polygon and one slot per contour point of that polygon, even where the slot is `None`.
- `build_modifier_segment_annotations` carries a D14 invariant that its output is routed to the BASE chain only. The seam writer has **no such restriction and must not inherit one**: canonical applies seam enforcers/blockers on finished loops regardless of region provenance, so every emitted region — BASE and painted variant chains alike — is stamped against its own polygons.
- `execute_paint_segmentation` uses raw (untransformed) object-mesh vertices in both the facet and stroke accumulation arms, and `slice_modifier_volumes` slices `mv.mesh` raw. The seam writer must mirror this and apply no world transform, or its polygons will not register against the layer geometry.
- `slice_modifier_volumes` and its `crate::slice_mesh_ex` call are **not** `#[cfg(feature = "host-algos")]`-gated, while the `painted_subsets` / `propagate_top_bottom` block is. The seam writer must sit on the ungated side so it works in a bare `-p slicer-core` build; conversely any `slicer-core` test that exercises it alongside gated code must still be run with `--features host-algos` (see `CLAUDE.md` §Test Discipline).
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Data and Contract Notes

- IR/manifest contracts: none changed. `SlicedRegion.segment_annotations` is `HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>>` and gains two new possible keys, both `PaintSemantic::Custom`. No schema version constant is bumped; no manifest key is added or removed, so `crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs` is unaffected.
- WIT boundary: unchanged. `segment-annotations-entry` / `segment-annotations-polygon` (`crates/slicer-schema/wit/deps/ir-types.wit`) already carry `paint-semantic`'s `custom(string)` case, and `project_seam_planning_view` forwards every key and slot 1:1 under a semantic-name sort. Two drop conditions there bound observability: a region with empty `polygons` is skipped, and a region whose exact `RegionKey` (including `variant_chain`) is absent from the `RegionMapIR` is skipped. Neither is changed by this packet, but AC-1 asserts on `execute_paint_segmentation`'s output directly rather than on the marshaled view, so neither can mask a writer defect.
- Determinism/scheduler constraints: `stamp_seam_paint_annotations` must iterate seam semantics in a fixed order (sorted semantic name) and must not depend on `HashMap` iteration order, so repeated slices of the same model produce byte-identical IR. It runs inside the existing paint-segmentation prepass and adds no scheduler edge.

## Locked Assumptions and Invariants

- **Encoding lock.** Seam intent is expressed as `PaintSemantic::Custom("seam_enforcer")` / `Custom("seam_blocker")` with value `PaintValue::Flag(true)`. Both consumers key on this one encoding; `PaintValue::Custom("enforced"/"blocked")` is no longer a recognized encoding anywhere.
- **Exact-match lock.** `paint_annotation_type` classifies on an exact semantic-name match only. Adding a third seam semantic requires editing that match arm — deliberately, so no future `Custom` name can leak in by resemblance.
- **Index-alignment lock.** `segment_annotations[sem][p]` has exactly `region.polygons[p].contour.points.len()` slots. `seam_paint_boxes` and `annotation_at` both index positionally and would silently mis-attribute otherwise.
- **Single-stamp lock.** The three `execute_paint_segmentation` return paths that invoke the writer are mutually exclusive; `stamp_seam_paint_annotations` is not idempotent-by-design (it merges), so double invocation must remain impossible.
- Reversibility: no config default and no schema version changes, so the behaviour change is not gated behind a flag — a model with no seam paint is bit-identical before and after (AC-N2, AC-N3).

## Risks and Tradeoffs

- **Seam bias becomes live for the first time.** Vertex-aligned seam paint can now move or exclude a seam in both the planning prepass (AC-10) and the perimeter generators (AC-6), so any baseline captured from a seam-painted fixture is invalidated. AC-N3 guards `resources/cube_cilindrical_modifier.3mf`'s region structure, and the arachne/classic agreement in AC-6 guards generator behavior.
- **Vertex-only sampling, on both consumers.** Seam paint is delivered at contour *vertices* only — the planner emits one candidate per contour vertex, and `seam_paint_boxes` builds one box per painted vertex. A stroke falling strictly between two vertices affects neither consumer. This is a resolution limit of the delivery channel, not a defect in it, and it applies symmetrically: no consumer is given mid-edge resolution the other lacks, so the two cannot disagree about where paint applies. Raising it means adding edge sampling to *both* the planner's candidate generation and the perimeter boxes together, and is out of scope here.
- **Deleting the `SupportEnforcer`/`SupportBlocker` arms is a behaviour removal.** Any model relying on support paint to move a seam loses that (accidental) effect. That is the point of DEV-133, and canonical agrees, but it is a user-visible regression from the user's perspective and belongs in the deviation-row closure note.
- **The `Flag(false)` case.** `build_modifier_segment_annotations` emits only `Some(Flag(true))` / `None`, and the writer follows suit, so `Flag(false)` should never appear. The classifier nonetheless treats it as `None` defensively; if a future writer emits it, the two consumers must stay consistent (`seam_paint_boxes` already requires `Flag(true)`).
- **Guest staleness.** This packet edits `crates/slicer-core/**` and three `modules/core-modules/*/src/**` trees. A `slicer-core` edit is the *silent* staleness mode (old geometry code runs without a loud instantiation failure), so `cargo xtask build-guests --check` is not optional here.
- **Cost.** `stamp_seam_paint_annotations` slices up to two extra meshes per slice and does an O(points) point-in-polygon test per region. Impact unmeasured; it is guarded by `mesh_has_seam_paint`, so unpainted and MMU-only slices pay only that boolean scan.
