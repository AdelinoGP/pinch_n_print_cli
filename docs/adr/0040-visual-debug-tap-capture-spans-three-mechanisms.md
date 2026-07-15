# ADR-0040 — Visual-Debug Tap Capture Spans Three Mechanisms

## Status

Accepted.

## Context

The visual-pipeline-debug spec (`docs/specs/visual-pipeline-debug.md`) promises
post-stage visual taps for every scheduler stage, and ADR-0037 requires taps to
read committed IR at the executor boundary without adding any module, WIT, or
Blackboard API. The initial capture implementation (packet 158,
`SUPPORTED_TAP_STAGE_IDS` in `crates/slicer-runtime/src/layer_executor.rs`)
covered only the seven per-layer `Layer::*` taps whose source is an `apply`
commit into a `LayerArena` slot, and `execute_captured_stages` was built around
that one boundary: a per-layer closure truncated at the furthest requested tap.

Grounding the remaining inventory against the tree shows the other taps do not
share that boundary. Their source IR lives in three structurally different
places, so a single capture mechanism cannot serve them all.

## Decision

Recognize three tap classes, each with its own capture mechanism and dependency
closure. A visual-debug request's furthest selected tap determines which
mechanism runs.

- **Blackboard-read taps.** MeshAnalysis, SeamPlanning, SupportGeometry,
  PaintSegmentation, RegionMapping, OverhangAnnotation, `Layer::Slice`, and
  `Layer::PaintRegionAnnotation`/`SlicePostProcess`. Their source is a
  whole-print, write-once Blackboard slot (`SurfaceClassificationIR`,
  `SeamPlanIR`, `SupportGeometryIR`/`SupportPlanIR`, `SliceIR`, `RegionMapIR`),
  committed during prepass and immutable during Tier 2. Capture is a read of the
  already-committed slot after `prepare_prepass_context` returns — an owned clone
  of the `Arc<…>` payload, no `LayerArena`, no per-layer dispatch. Closure = the
  prepass built-ins/modules through the stage that commits the requested slot.
  This is the cheapest class and needs a capture entry point separate from
  `execute_captured_stages`.
- **Arena taps.** The seven `Layer::*` taps already shipped. Source is an
  `apply` commit into a per-layer `LayerArena` slot; the per-layer closure
  truncates at the furthest requested tap and executes only selected layers
  (Tier 2 per-layer work is cross-layer-independent).
- **PostPass whole-print taps.** LayerFinalization and GCodeEmit. Their source
  (`Vec<LayerCollectionIR>` after finalization; `GCodeIR` after emit) requires
  the full pipeline prefix: prepass → all per-layer stages over all layers →
  layer finalization → `execute_postpass`. These cannot use the truncation
  trick; reaching them slices the whole model. They execute whole-print but
  render only the request's selected layers, and the bundle manifest records the
  whole-print closure. Selecting GCodeEmit legitimately triggers G-code emission,
  which the spec's dependency-closure criterion permits when a final G-code view
  is requested.

All three read committed IR only; none adds a module, WIT, or Blackboard API
(ADR-0037 preserved).

## Consequences

- The "execute only the minimal dependency closure" property holds strictly for
  Blackboard-read and arena taps. For PostPass taps it is explicitly relaxed to
  "the whole-print prefix," documented per-tap and recorded in the manifest —
  the deviation is bounded to the two PostPass taps, not the general contract.
- The capture layer gains a Blackboard-read entry point and a PostPass capture
  path alongside `execute_captured_stages`; the manifest closure fields
  (`executed_stage_ids`, `executed_layer_indices`) must represent whole-print
  execution for a PostPass tap, not only the per-layer truncation model.
- Cost is predictable from the furthest tap's class: prepass-only, per-layer, or
  whole-print. A request mixing classes runs at the furthest class's cost.

## Alternatives Considered

- **Force every tap through `execute_captured_stages`.** Rejected: prepass slots
  are committed before per-layer work and PostPass sources after it, so the
  per-layer truncation boundary cannot reach either without misrepresenting what
  executed.
- **Drop PostPass IR taps and rely only on the `final_gcode` text renderer.**
  Rejected: the structured `GCodeIR` and post-finalization `LayerCollectionIR`
  views carry role/travel/synthetic-layer structure the serialized text render
  flattens; both taps earn their place despite the whole-print cost.
- **Add a Blackboard capture hook inside each prepass built-in.** Rejected as
  unnecessary plumbing: the post-`prepare_prepass_context` slot accessors already
  are the committed-read boundary.
