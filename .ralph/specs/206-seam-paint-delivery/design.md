# Design: 206-seam-paint-delivery

## Controlling Code Paths

- Primary code path: `execute_paint_segmentation` (`crates/slicer-core/src/algos/paint_segmentation/mod.rs`) → new `seam_annotations::stamp_seam_paint_annotations` → `SlicedRegion.segment_annotations` → `project_seam_planning_view` (`crates/slicer-wasm-host/src/marshal/in_.rs`) → `paint_annotation_type` / `candidate_paint_classification` (`modules/core-modules/seam-planner-default/src/visibility.rs`); and, in parallel, `SliceRegionView::segment_annotations` → `seam_paint_boxes` → `apply_seam_paint_bias` (`crates/slicer-core/src/perimeter_utils.rs`) in both perimeter generators.
- Neighboring tests/fixtures: `crates/slicer-runtime/tests/executor/paint_channel_consumer_paths_tdd.rs` (owns the seam fixture `resources/cube_cilindrical_modifier.3mf` and the `paint_channel_seam_strokes_do_not_partition_regions` guard); `crates/slicer-runtime/tests/integration/painted_seam_enforcer_blocker_tdd.rs` (pins `apply_seam_paint_bias`'s score direction); `modules/core-modules/seam-planner-default/tests/seam_canonical_visibility_tdd.rs`; `modules/core-modules/arachne-perimeters/tests/arachne_parity_seam_candidate_tdd.rs`; `modules/core-modules/classic-perimeters/tests/classic_perimeters_tdd.rs`.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

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

## Code Change Surface

- **Selected approach.** Four coordinated changes:

  1. **Writer (`crates/slicer-core/src/algos/paint_segmentation/seam_annotations.rs`, new).** Two functions:
     - `pub(crate) fn mesh_has_seam_paint(mesh: &slicer_ir::MeshIR) -> bool` — true iff any object's `paint_data.layers` (or any layer's `strokes`) carries a semantic for which `super::is_seam_paint_semantic` holds **and** has at least one `Some` facet value or a non-empty stroke list. Cheap; runs before any allocation.
     - `pub(crate) fn stamp_seam_paint_annotations(mesh: &slicer_ir::MeshIR, layers: &mut [slicer_ir::SliceIR])` — builds, per seam semantic name (`"seam_enforcer"`, `"seam_blocker"`), one `slicer_ir::IndexedTriangleSet` from the painted facets and stroke triangles, exactly mirroring the `push_tri` accumulation in `execute_paint_segmentation`'s facet/stroke arms; slices each subset with `crate::slice_mesh_ex(&subset, &layer_zs)` (the same primitive `slice_modifier_volumes` uses) to get per-layer `Vec<ExPolygon>`; then for every layer, every region, and every region polygon, emits one `Option<PaintValue>` per contour point using the same edge-midpoint containment test as `build_modifier_segment_annotations` (`modifier_volumes::any_expolygon_contains_point` on the integer midpoint of `points[k]`/`points[(k+1) % n]`) → `Some(PaintValue::Flag(true))` inside, `None` outside. Merges into (never replaces) the region's existing `segment_annotations` map, so modifier-volume annotations survive.
  2. **Call-site rewiring (`execute_paint_segmentation`).** The head's three guards become: `mesh.objects.is_empty()` returns `slice_ir.clone()` unchanged; the `!mesh_has_any_paint(&mesh)` and `region_map.entries.is_empty()` guards route through a single `seam_only_passthrough` helper closure that returns `slice_ir.clone()` when `!mesh_has_seam_paint(&mesh)` (no clone cost on unpainted slices — AC-N3 of the pre-existing behaviour) and otherwise clones the layers, calls `stamp_seam_paint_annotations`, and returns the new `Arc`. At the end of the function, immediately before the existing `Ok(Arc::new(working))`, `stamp_seam_paint_annotations(&mesh, &mut working)` runs once when `mesh_has_seam_paint` holds. The three return paths are disjoint, so no region is stamped twice.
  3. **Helper promotion.** `seam_paint_boxes` and `seam_paint_box` move verbatim from `modules/core-modules/classic-perimeters/src/lib.rs` into `crates/slicer-core/src/perimeter_utils.rs`; `seam_paint_boxes` becomes `pub fn`, `seam_paint_box` stays private. Classic's definitions are deleted and its `use slicer_core::perimeter_utils::{…}` list gains `seam_paint_boxes`. Arachne's import list at the top of `modules/core-modules/arachne-perimeters/src/lib.rs` gains `apply_seam_paint_bias, seam_paint_boxes`; its seam-candidate loop becomes `for (poly_idx, polygon) in polygons.iter().enumerate()`, `let mut candidates`, and gains the two `seam_paint_boxes` calls plus `apply_seam_paint_bias(&mut candidates, &enforcer_polys, &blocker_polys)` before the push loop. Arachne's `polygons` are the region's *input* polygons, so `poly_idx` aligns with the annotation index contract more directly than classic's offset `outer_polys` do.
  4. **Exact-semantic classification.** In `modules/core-modules/seam-planner-default/src/visibility.rs`, `paint_marker` is deleted; `paint_annotation_type` becomes: `PaintSemantic::Custom(name)` where `name == "seam_blocker"` → `Some(Blocked)`; `name == "seam_enforcer"` → `Some(Enforced)`; every other semantic (including `SupportEnforcer`, `SupportBlocker`, `Material`, `FuzzySkin`) → `None`. The value side no longer participates in classification, but the annotation must still be *present* — `annotation_at` already yields only `Some(_)` slots — and a `PaintValue::Flag(false)` slot must classify as `None` so an explicitly-cleared vertex is not read as intent. Blocked-wins precedence in `candidate_paint_classification` is unchanged.

- **Exact functions, traits, manifests, tests, and fixtures.**
  - New: `seam_annotations::mesh_has_seam_paint`, `seam_annotations::stamp_seam_paint_annotations`, `slicer_core::perimeter_utils::seam_paint_boxes` (pub), `modules/core-modules/seam-planner-default/tests/seam_paint_semantic_exactness_tdd.rs`, `crates/slicer-runtime/tests/integration/arachne_seam_paint_bias_tdd.rs` (+ its `mod` line in `crates/slicer-runtime/tests/integration/main.rs`), four `#[test]` fns in `crates/slicer-runtime/tests/executor/paint_channel_consumer_paths_tdd.rs`.
  - Modified: `execute_paint_segmentation`, `mod.rs`'s `mod` declaration list, `paint_annotation_type`, arachne's seam-candidate loop and import list, classic's import list, `paint_annotations_set_point_type`.
  - Deleted: `paint_marker`; classic's private `seam_paint_boxes` / `seam_paint_box`.
  - No manifest (`*.toml`) change: neither generator gains a config key, and `seam-planner-default`'s declared keys are untouched.

- **Rejected alternatives and reasons.**
  - *Re-admit seam semantics to `mesh_has_any_paint` and write inside the per-layer loop.* Rejected: this is the exact code path that killed every layer below z≈0.5 on `resources/cube_cilindrical_modifier.3mf`, and it would re-couple seam paint to the cell decomposition that DEV-123 just decoupled.
  - *Write only into the BASE chain, mirroring `build_modifier_segment_annotations`' D14 rule.* Rejected: painted variant chains own their own walls, so seam candidates generated on them would receive no bias — a silent half-fix on any mesh that is both MMU- and seam-painted.
  - *Duplicate `seam_paint_boxes` into arachne.* Rejected: DEV-127 in the same remediation queue exists because exactly this kind of copy drifted. One shared `pub fn` in `slicer-core`, which both modules already depend on for `apply_seam_paint_bias` and `build_wall_flags`.
  - *Keep `paint_marker` but anchor it (`starts_with("seam_")`).* Rejected: `PaintSemantic::Custom` is an open string space; an exact two-name match is the only rule that cannot leak, and it is what DEV-133 specifies.
  - *Emit `PaintValue::Custom("enforced")` to preserve the current test encoding.* Rejected: `seam_paint_boxes` filters on `Some(PaintValue::Flag(true))`, so a `Custom` value would feed the seam planner while starving the perimeter generators. One encoding, `Flag(true)`, for both consumers.

## Files in Scope (read + edit)

- `crates/slicer-core/src/algos/paint_segmentation/seam_annotations.rs` — role: the writer; expected change: new file, plus one `mod seam_annotations;` line in `crates/slicer-core/src/algos/paint_segmentation/mod.rs`.
- `crates/slicer-core/src/algos/paint_segmentation/mod.rs` — role: guard-block and tail rewiring in `execute_paint_segmentation`; expected change: three return paths routed through the seam pass; no other logic touched.
- `crates/slicer-core/src/perimeter_utils.rs` — role: home of the promoted shared helper; expected change: `pub fn seam_paint_boxes` + private `seam_paint_box` added next to `apply_seam_paint_bias`.
- `modules/core-modules/classic-perimeters/src/lib.rs` — role: donor of the promoted helper; expected change: two private fns deleted, import list extended. (Fourth+ file: justified because the promotion is only complete when the donor stops defining a duplicate; AC-7 fails otherwise.)
- `modules/core-modules/arachne-perimeters/src/lib.rs` — role: DEV-134 call site; expected change: seam-candidate loop gains `enumerate`, `mut`, two `seam_paint_boxes` calls and one `apply_seam_paint_bias` call.
- `modules/core-modules/seam-planner-default/src/visibility.rs` — role: DEV-133 discriminator; expected change: `paint_marker` deleted, `paint_annotation_type` rewritten to exact match.

The six-file surface exceeds the three-file target because the packet is a producer/consumer pair that cannot be split without shipping a half-wired channel (see `requirements.md` §Problem Statement). `implementation-plan.md` keeps every individual step within the ≤3-edit cap.

## Read-Only Context

- `crates/slicer-core/src/algos/paint_segmentation/modifier_volumes.rs` — whole file is short; purpose: `slice_modifier_volumes`' use of `crate::slice_mesh_ex` and `any_expolygon_contains_point`, which the writer mirrors.
- `crates/slicer-core/src/algos/paint_segmentation/mod.rs` — `build_modifier_segment_annotations` window only - purpose: the exact edge-midpoint containment shape the writer copies.
- `crates/slicer-wasm-host/src/marshal/in_.rs` — `project_seam_planning_view` window only - purpose: confirm the marshal is content-preserving and note its two drop conditions (empty `polygons`; `RegionKey` absent from `region_map`), which bound what AC-1 can observe end-to-end.
- `crates/slicer-runtime/tests/executor/paint_channel_consumer_paths_tdd.rs` — the fixture-helper block and `paint_channel_seam_strokes_do_not_partition_regions` only - purpose: reuse helpers; do not re-derive.
- `modules/core-modules/seam-planner-default/src/visibility.rs` — `annotation_at`, `has_enforced_annotation`, `is_central_enforcer_vertex`, `candidate_paint_classification` - purpose: confirm Blocked-wins precedence and the central-enforcer run heuristic survive the discriminator change.
- `crates/slicer-runtime/tests/integration/painted_seam_enforcer_blocker_tdd.rs` — purpose: `apply_seam_paint_bias`'s pinned score direction; the new AC-6 test must not contradict it.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate; never load.
- `target/`, `Cargo.lock`, `modules/core-modules/*/wit-guest/target/**`, generated code, vendored dependencies — never load.
- `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-schema/wit/**` — no IR or WIT change is made; delegate any symbol lookup.
- `crates/slicer-core/src/algos/paint_segmentation/{top_bottom.rs, voronoi_graph.rs, colorize.rs, extract_segments.rs, painted_line*.rs, compose_variants.rs}` — the cell-decomposition machinery is untouched.
- `modules/core-modules/support-planner/**`, `modules/core-modules/seam-placer/**` — neither is edited.
- `crates/slicer-core/src/algos/paint_segmentation/mod.rs`'s `#[cfg(test)] mod tests` — the in-module shell-index tests belong to packet 128/207 territory.

## Expected Sub-Agent Dispatches

- Question: does `gather_enforcers_blockers` (`SeamPlacer.cpp`) read any volume other than `mv->seam_facets`, and where is enforcer/blocker scoring applied relative to `process_classic`/`process_arachne`?; scope: `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.{cpp,hpp}`; return: `SUMMARY` (≤200 words); purpose: Steps 5–6 parity justification.
- Question: list every struct-literal / call site of `seam_paint_boxes` and `seam_paint_box` across `crates/` and `modules/` (excluding `target/`); scope: `crates/**/*.rs`, `modules/**/*.rs`; return: `LOCATIONS` (≤20 entries); purpose: Step 4 blast radius.
- Question: does any test outside `modules/core-modules/seam-planner-default/tests/` construct `PaintSemantic::SupportEnforcer` or `SupportBlocker` annotations and assert a seam `point_type`?; scope: `crates/**/tests/**`, `modules/**/tests/**`; return: `LOCATIONS` (≤20 entries); purpose: Step 6 test-assertion fallout.
- Question: `cargo test -p slicer-runtime --test executor seam_paint_` — pass/fail plus failing assertion; scope: cargo; return: `FACT` pass/fail with ≤20 lines on failure; purpose: Steps 2–3 verification.
- Question: `cargo xtask build-guests --check` — does it report any `STALE:` line?; scope: cargo; return: `FACT` pass/fail; purpose: after Steps 2, 4, 5, 6.

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

- **Seam bias becomes live for the first time.** Every previously-unbiased seam-painted model will now move its seam. This is the intended fix, but it means any baseline captured from a seam-painted fixture is invalidated. `resources/cube_cilindrical_modifier.3mf` is the known such fixture; AC-N3 guards its region structure, and the arachne/classic agreement in AC-6 guards the direction of the change.
- **Deleting the `SupportEnforcer`/`SupportBlocker` arms is a behaviour removal.** Any model relying on support paint to move a seam loses that (accidental) effect. That is the point of DEV-133, and canonical agrees, but it is a user-visible regression from the user's perspective and belongs in the deviation-row closure note.
- **The `Flag(false)` case.** `build_modifier_segment_annotations` emits only `Some(Flag(true))` / `None`, and the writer follows suit, so `Flag(false)` should never appear. The classifier nonetheless treats it as `None` defensively; if a future writer emits it, the two consumers must stay consistent (`seam_paint_boxes` already requires `Flag(true)`).
- **Guest staleness.** This packet edits `crates/slicer-core/**` and three `modules/core-modules/*/src/**` trees. A `slicer-core` edit is the *silent* staleness mode (old geometry code runs without a loud instantiation failure), so `cargo xtask build-guests --check` is not optional here.
- **Cost.** `stamp_seam_paint_annotations` slices up to two extra meshes per slice and does an O(points) point-in-polygon test per region. Impact unmeasured; it is guarded by `mesh_has_seam_paint`, so unpainted and MMU-only slices pay only that boolean scan.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2, the writer)
- Highest-risk dispatch and required return format: the `seam_paint_boxes` / `seam_paint_box` call-site inventory before Step 4 — `LOCATIONS`, ≤20 entries. A missed site leaves a duplicate definition and fails AC-7 after the edit is already spread across two modules.

## Open Questions

- `[FWD]` Should the writer stamp regions whose `variant_chain` is non-empty, or BASE only? Design decision above is **all regions**, on the canonical argument that `SeamPlacer.cpp` is region-agnostic. The implementer must confirm this does not violate any assertion in `crates/slicer-core/src/algos/paint_segmentation/mod.rs`'s `assert_per_object_shell_index_invariant` or the D14 comment block, and must add the observation to the AC-3 test (which asserts index alignment on *every* annotated region, BASE or variant). Resolvable within the packet; not an activation blocker.
- `[FWD]` Arachne's seam-candidate loop iterates the region's *input* `polygons`, while classic iterates the *offset* `outer_polys`. Both index `segment_annotations` by the same `poly_idx`. Classic's alignment is documented as valid because outer-wall vertex ordering/count is preserved from the original contour; arachne's is valid trivially. AC-6 asserts the two agree on one fixture; if they disagree, the implementer must report the divergence rather than adjusting either index base, because a mismatched index base is a mis-attribution bug, not a tolerance question.
- `[FWD]` `resources/cube_cilindrical_modifier.3mf` carries seam paint whose lowest painted facet sits at z ∈ (0.4, 0.5]. AC-1 must assert on a layer at or above that band; asserting "some layer" is sufficient and is how the AC is worded, but the implementer should print the annotated layer indices as a diagnostic so a future onset change is visible.
