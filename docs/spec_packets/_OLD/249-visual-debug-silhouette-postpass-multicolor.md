---
status: implemented
packet: 249-visual-debug-silhouette-postpass-multicolor
task_ids:
  - TASK-449
  - TASK-450
  - TASK-451
---

# 249-visual-debug-silhouette-postpass-multicolor

## Goal

Extend the `silhouette` visualization kind (packet 247, schema 1.2.0) to `PostPass::LayerFinalization` via the plan's D10 single whole-print capture shape (one `StageCapture` carrying the finalized `Vec<LayerCollectionIR>` once — never one whole-print clone per layer), rendering typed `Point3WithWidth` segment projections inflated by `width / 2` on schedule-z-diff slabs, and bring `color_by: "tool"` to silhouettes (D17): per-(layer, tool) interval classes on tool-carrying captures, ascending-tool paint order, `tool_palette` manifest emission, and the per-capture `RenderError::ToolColorUnavailable` fail-closed contract replacing packet 247's blanket validation rejection.

## Problem Statement

Tree-support and finalization defects (entity merge order, travel-reconciled output, per-tool structure) are only visible after `PostPass::LayerFinalization`, but packet 247 rejects postpass silhouettes for a structural reason it hands forward: `run_postpass_taps` (`crates/pnp-cli/src/visual_debug.rs`) builds **one `StageCapture` per (tap, applicable layer), each cloning the entire whole-print IR** (`capture.finalized_layers.clone()` per row — plan fact 8, verified). An all-layers silhouette through that shape clones the whole print once per layer — an OOM-shaped cost on exactly the large models a side view serves. The plan's D10 mandates a single whole-print capture for silhouette consumption with the per-layer rows byte-untouched for top-down consumers. Multicolor (D17) lands here too: silhouettes gain `color_by: "tool"` on tool-carrying captures with the per-capture `ToolColorUnavailable` contract replacing 247's blanket validation rejection — 247's `[FWD to packet 249]` names both obligations.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- This path never converts units: `Point3WithWidth.x/y/z/width` and `LayerCollectionIR.z` are millimeter `f32`s end-to-end (`docs/08`); no `Point2`/`mm_to_units` appears in the new code.
- Projector single-owner rule (archived spec, binding): rectangle corners go through `Projector::project(x_or_y_mm, z_mm)`; no new transform.
- D10 shape lock: the whole-print capture is a **shape of `StageCapture` rows**, not a change to `CapturedIr` (the plan rejects `Arc`-ing the payload — it would change the serialized `typed_capture` shape and every match arm). `StageCapture.layer_index`/`layer_z` on a whole-print row are `0`/`0.0` and documented unread; no consumer may branch on them.
- The existing per-layer postpass rows and every top-down render path are byte-frozen (AC-10); `shapes_for_styled` is not touched — silhouette tool coloring lives entirely in the composite path.
- Struct-literal churn gate (`docs/21_data_defaults_and_fixtures.md`): fixtures of `LayerCollectionIR` use `..Default::default()`; `PrintEntity` has no `Default` by design — use the existing suites' `// exhaustive:` waiver; `PostPassCapture` implements `Default` (its `::default()` is the existing `run_postpass_taps` construction pattern).

## Data and Contract Notes

- IR/manifest contracts: no IR/WIT/schema change anywhere — `PostPassCapture`, `StageCapture`, `CapturedIr`, and every manifest field shape are consumed as-is; the only manifest deltas are 247's existing fields on new entries (`view`, `layers_rendered`, `color_by`, `tool_color_source`, `tool_palette`). 1.0/1.1 bundles byte-frozen (unchanged code paths; AC-10 regression pins the postpass one).
- WIT boundary: none.
- Determinism/scheduler constraints: whole-print extraction iterates `layers` sorted by `global_layer_index` (sort defensively — `Vec` order is producer order); rectangle emission ascending layer → class order (role ranks or tool index) → interval start; group order per 247 (STAGE_ORDER position, then tap, then role-before-tool within a (tap, view)); schedule built from sorted finalized indices; all sources are `Vec`s — no `HashMap` iteration.
- Viewport note: `compute_silhouette_viewport_bounds` consumes `geometry_points_mm`, whose `CapturedIr::LayerFinalization` arm already includes every layer's entity points and travel destinations (verified) — so a whole-print capture makes framing selection-independent by construction; travel destinations may widen the horizontal frame slightly beyond extrusion, matching the top-down viewport's existing behavior. Width inflation (≤ half a line width) may exceed the geometry-point union but sits far inside the 2 mm margin — accepted, noted for reviewers.

## Locked Assumptions and Invariants

- One whole-print `StageCapture` per postpass tap on silhouette bundles; per-layer rows byte-identical for every non-silhouette bundle (both pinned by AC-1's two arms).
- Whole-print rows carry `layer_index: 0` / `layer_z: 0.0` as documented-unread placeholders — no consumer branches on them; the schedule is the sole selection/slab authority for whole-print captures.
- Slabs for finalized layers are consecutive-z diffs from the capture's own layers, first from 0 — never the blackboard layer plan, never per-region heights (which the IR does not carry).
- Tool coloring is legal only where the capture carries tool assignment; everywhere else fails closed with the existing `ToolColorUnavailable` — silhouette gets no looser rule than the top-down renderer's pinned contract (plan D17/R6).
- `render_silhouette_composite` ≡ `render_silhouette_composite_styled` with `RenderStyle::default()`, byte-for-byte (AC-8).
- Grouping key is (tap, view, color mode); filenames `{sanitized_tap}_silhouette_{view}[_tool].png`; one silhouette plane per bundle (247 invariant, inherited).

## Risks and Tradeoffs

- `run_postpass_taps` still executes the whole print (tiers 2–4) for any postpass tap — D10 removes the clone multiplication, not the execution cost; that cost is inherent to postpass taps (plan facts 8/15) and documented.
- A silhouette request mixing a blackboard tap and `PostPass::LayerFinalization` under `color_by: "tool"` fails at render time (per-capture contract), not validation — a user discovers the incompatibility later than a validation error would tell them. Deliberate: this is the pinned top-down contract 247's `[FWD]` mandates; the error names the offending tap.
- The (tap, view, color mode) grouping key change touches 247's assembly code — the role-only path must produce byte-identical bundles before/after (AC-10 plus 247's own suite gate this).
- If packet 248 lands first, its model-source-only tool rejection is removed by this packet's validator edit; if 249 lands first, 248's narrowing becomes a no-op edit on an already-lifted check. Both orders compose because the two packets touch disjoint validator clauses; the swarm executes queue order regardless.
