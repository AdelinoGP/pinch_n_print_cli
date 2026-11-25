# Tree-support render: fold the per-layer `Layer::Support` render into the prepass?

Type: grilling
Label: wayfinder:grilling
Status: resolved (2026-11-17; human ruling — **EXPERIMENT, not adoption**.
  The fold is pursued as a measurement of whether the planner/renderer
  structure costs wall, via an env-gated host probe; graduated
  [160](160-tree-support-prepass-batch-probe.md). Adoption, contract change and
  any Support family glossary/ADR change are reserved for a separate ruling.)
Assignee: 2026-11-17 Claude Code wayfinder session — the human said "Continue
  the perf vs orca map"; the frontier was empty (1–158 resolved) and the human
  picked fog item (a) of "The benchy per-layer program is margin, not route"
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: map fog, "The benchy per-layer program is margin, not route
  (t154)", item (a) — "whether the per-layer `tree-support` render can be
  folded into the prepass, which is a phase move and architecture work, not a
  per-op shape — no ticket, and it must not inherit a price from the bracket".

## Question

Should the map pursue moving `com.core.tree-support`'s per-layer render
(`TreeSupport::run_support`, `modules/core-modules/tree-support/src/lib.rs`,
dispatched in `Layer::Support`) into a whole-object prepass pass, as canonical
OrcaSlicer generates tree-support toolpaths inside
`PrintObject::_generate_support_material` → `TreeSupport::generate`
(`TreeSupport.cpp`) — and if so, under what framing (phase move, work-set
change, or a per-op render-cost ticket instead)?

## Facts going in (code facts verified 2026-11-17; numbers cited from their tickets)

- **Canonical has the same two halves PnP has.** `TreeSupport::generate`
  (`TreeSupport.cpp`, pin `v2.5.0` = `40ecc66f78`) runs
  `generate_contact_points` → `drop_nodes` → `smooth_nodes` → `draw_circles`
  → `generate_toolpaths`. `generate_toolpaths` is a per-layer `tbb::parallel_for`
  that turns the drawn areas into wall + fill extrusions. PnP's
  `tree-support-planner` (prepass) ports the first four; `tree-support`
  (`Layer::Support`) is the `generate_toolpaths` counterpart (`render_polygon`:
  `wall_count` insets plus a scan fill). So t154 §5's "supports-on cells pay
  support work twice" is a **phase-placement** difference, not duplicated work:
  canonical's toolpath step is also layer-parallel, just scheduled inside the
  support step rather than inside a per-layer pipeline.
- **Price on record (t154, ceiling not price):** `tree-support` is the top
  per-layer carrier on three supports-on cells — **47.8 %** (benchy classic-on),
  **44.0 %** (benchy arachne-on), **36.7 %** (base classic-on) of per-layer
  thread; wall-equivalent ceilings **9.8 % / 8.4 % / 6.0 %** of cell wall. It is
  **96.6–96.8 % in-module self fuel** (`clip_polygons` scope 2.9–3.2 %): kernel
  and algorithm work, not plumbing.
- **The per-layer phase is already parallel:** t154 effective parallelism
  **9.65x / 9.77x / 11.18x / 11.30x** on the four supports-on cells.
- **The destination term is the prepass, not the per-layer phase.** After t158,
  the cross-window composition puts all eight cells below 1.0x **at canonical
  prepass cost** (base classic-on **0.8343x**); the open route is the 64.3–84.5 %
  prepass cut (t154). A phase move *into* the prepass adds to the term that must
  shrink.

## Answer

**Ruling (human, 2026-11-17): pursue the fold as an experiment** — "to verify
if the structure I planned (the current one) is hurting". The recommended
decline-and-reframe (a per-op ticket only) was not taken; the per-op comparison
is folded into the experiment as a control instead.

Decisions, in grilling order:

1. **Q1 — framing:** experiment, not adoption. The question the experiment
   answers is *does scheduling the Support layer renderer per layer, inside the
   per-layer pipeline, cost wall against a whole-object pass?* — the
   phase-placement question, with the work held constant.
2. **Q2 — shape: host batch plus a null-renderer ceiling.** An env-gated host
   probe dispatches the **unchanged** `tree-support` guest for every layer in
   one parallel whole-object pass after `PrePass::SupportGeometry`, caches the
   outputs, and `Layer::Support` replays them. A second arm skips the render
   entirely (null renderer) to bound what placement could cost at most. No WIT
   change. Risk carried: the per-layer `SliceRegionView` may carry
   post-perimeter syncs; batch-arm byte-identity is the check.
3. **Q3 — verdict rule: per cell.** On the four supports-on cells, the
   structure **hurts** in a cell where the batch arm is faster by ≥ the 0.80 %
   floor *and* outside the measured null band; otherwise it is **exonerated**
   in that cell. Supports-off cells are checked for inertness only.
4. **Q4 — per-op control: yes, if the rig captures it.** Canonical's
   `TreeSupport::generate` profiler reports `STAGE_GENERATE_TOOLPATHS` at
   `BOOST_LOG` info; record it beside PnP's render thread/wall time, or disclose
   the gap. This separates "placement hurts" from "the render is per-op slower".
5. **Q5 — disposition: probe only.** Reverted, `probe.patch` preserved (t155 /
   t157 precedent). Adoption — a production fold, a contract change, or a
   change to the Support family / Support layer renderer glossary terms in
   `CONTEXT.md` — needs a separate ruling on the numbers.

**Correction recorded:** t154 §5's "the supports-on cells pay support work
twice" is wrong as worded. Canonical `generate_toolpaths` is the counterpart of
PnP's renderer and is itself a per-layer `tbb::parallel_for`; the difference is
where the toolpath step is scheduled (inside the support step vs inside the
per-layer pipeline), not duplicated work.

**Domain model:** unchanged. `CONTEXT.md`'s Support family = Support strategy
planner (cross-layer) + Support layer renderer (layer-independent) stands; the
experiment keeps both halves and moves only *when* the renderer is dispatched.
No ADR (nothing hard to reverse is decided).
