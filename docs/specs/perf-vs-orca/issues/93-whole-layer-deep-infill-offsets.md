# Whole-layer deep-infill offsets in internal-bridge qualification

Type: task
Label: wayfinder:task
Status: open (graduated 2026-10-28 from the t92 Phase-1 screen; unclaimed)
Assignee: none
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Per-surface bridge boolean locality](92-per-surface-bridge-boolean-locality.md)

## Question

Can the two whole-layer offsets on the unioned deep sparse-infill area — the
`+1.5 × spacing` clip area and the `−4.5 × spacing` erosion filter — be made
cheaper or skipped without changing the internal-bridge qualification verdict,
and does the resulting wall win clear the map's **0.80 %** floor on the four
base classic cells?

**Screen first.** The total bracket is measured, but which fraction is
removable is not. Phase 1 sizes the removable part; Phase 2 builds the
candidate only if it clears the floor.

## Measured premise

The [t92 Phase-1 screen](../evidence/t92-per-surface-bridge/FINDINGS.md)
instrumented ordinary base classic supports-off and found that
`compute_internal_bridge_layer`
(`crates/slicer-runtime/src/slice_postprocess_prepass.rs`) spends almost
nothing on its per-surface shapes and almost everything on two whole-layer
offsets of `deep_infill_area` (the unioned sparse infill in the bridge-depth
window, from `gather_areas_w_depth`):

| term | run 1 | run 2 | calls | run 2 wall-eq | run 2 share |
|---|---:|---:|---:|---:|---:|
| `offset(deep_infill_area, +1.5 × spacing)` (clip area) | 12,462.9 ms | 11,375.2 ms | 130 | 1,233.2 ms | **1.69 %** |
| `offset(deep_infill_area, −4.5 × spacing)` (erosion filter) | 10,042.1 ms | 9,276.2 ms | 14 | 1,005.6 ms | **1.37 %** |
| **both** | **22,505.1 ms** | **20,651.4 ms** | — | **2,238.8 ms** | **3.06 %** |

Accumulated over the layer axis; wall-equivalent = accumulated ÷ the run's own
cpu/wall (t92's established conversion). The current base classic supports-off
cell wall is **73.17 s** ([t87 scoreboard](../evidence/scoreboard-refresh-20261025/SCOREBOARD.md)),
so the floor is **0.585 s**. The pair is **3.8–4.3x the floor** and **~9x t92's
own (measured-dead) ceiling**.

Operand facts measured by t92:

- `deep_infill_clip_area` is only **1–7 polygons** per call (mean 1.75) but the
  Miter offset costs a median **81.6 ms** (max 222.4 ms) — complex ring
  geometry, not polygon count.
- The erosion runs on the **14 of 130** calls whose qualified set is non-empty
  (18 qualified polygons total) and still costs **44 %** of the pair, up to
  **428.3 ms** on one layer.
- `region.internal_solid_fill` is tiny (196 surfaces over 130 calls, max 7), so
  the per-surface consumer set (`offset(surface, spacing)` + intersections) is
  **0.25 s accumulated** — the offsets, not their consumers, are the cost.

Both offsets are **whole-layer operands**: `deep_infill_clip_area` is the
per-surface clip operand *and* the `internal_bridge_angles` operand
(`expansion_area` intersection, per-candidate `area` filter), and the erosion
is the `retain` predicate's operand. Restricting the *consumer* to a bbox (t92's
Phase 2 step 1) cannot remove the offset build — which is why this is a
separate ticket, not a reopened t92.

Known shapes to price, in order of expected value:

1. **Lazy dilation.** Build `offset(deep_infill_area, 1.5 × spacing)` only when
   a conservative bbox precheck says some surface can reach the deep area
   (surface bbox grown by `spacing + 1.5 × spacing` plus a Miter-extension
   margin, vs the deep-area bbox). Exact by construction; the question is how
   many of the 130 calls skip it. t92 saw 130 calls and 14 qualified sets, but
   did **not** count how many calls had any surface passing the clip test.
2. **Windowed erosion.** Only the 14 qualified layers pay the erosion, and the
   qualified set is 18 polygons. Clip the deep-area operand to the qualified
   set's bbox grown by `4.5 × spacing` plus a Miter safety margin, erode, then
   clip the result back to the window. Miter offsets are local, so the result
   inside the window is unchanged; the win depends on how much of the layer the
   window covers. t84's overhang window (retains 97.84 % of points → 0.19 %
   ceiling, rejected) is the precedent to beat: here the window is driven by a
   tiny qualified set, not by a band layer.
3. **Cheaper erosion predicate.** `polygon ∩ erode(deep, 4.5 × spacing) ≠ ∅`
   is not provably equal to `dilate(polygon, 4.5 × spacing) ∩ deep ≠ ∅` under
   Miter joins. Price it only if it can be stated as a representation-equivalent
   test with a proof or an exhaustive fixture check; otherwise reject it in
   Phase 1 rather than building it.
4. **Duplicate recompute.** The phase-B path recomputes a layer's whole
   `compute_internal_bridge_layer` when lower candidates exist. Count how often
   the same `(layer, deep_infill_area)` is offset twice; if it is common, the
   recompute path may reuse the first pass's offset results (exact: the deep
   area does not depend on bridge commits).

## Phase 1 — screening (mandatory)

1. Instrument an ordinary base classic supports-off slice with a
   reverted-before-close probe that records, per call: the deep-area polygon,
   hole and ring-vertex counts, both offset spans, whether any surface passed
   the clip test, the conservative bbox-precheck verdict for shapes 1, 2 and 4,
   and the number of duplicate recomputes.
2. Convert each shape's removable accumulated time to wall with the run's
   measured cpu/wall ratio and compare the **sum** against **0.585 s**.
3. If the sum's conservative ceiling is below the floor, close as
   **measured-dead** with the probe reverted; no candidate.
4. If above, proceed to Phase 2 with only the shapes that cleared the floor
   individually or in a stated combination.

## Phase 2 — candidate (conditional)

1. Implement the measured shapes; no geometry function changes.
2. Emitted G-code must stay byte-identical to the post-t81 reference set in all
   sixteen cells, both modes.
3. An equivalence test on a fixture with a non-empty qualified set where at
   least one surface is disjoint from the deep area, and one where the
   qualified set is far from most of the deep area (the windowing case), each
   comparing the gated/lazy path against the current per-layer reference.

## Pre-declared floor

**0.80 % of the cell wall** (0.585 s on the current base classic supports-off
cell). Phase 1 measures the removable ceiling; Phase 2's paired campaign
measures the actual win.

## Proof gate

- Phase 1 probe reverted; both guest freshness gates exit 0.
- Phase 2 only: 16-cell output-identity matrix against the post-t81 reference
  set, both modes; the equivalence tests above.
- Phase 2 only: standing paired ordinary + accelerated gate on the four base
  classic cells, 6 alternating pairs per cell, 12 threads, zero starvation
  exclusions, every pair byte-identical, process CPU ≤ baseline + 3.0 %.
- Both guest freshness gates exit 0 (host-only candidate).

## Deliverables

1. The Phase 1 screening table with per-shape removable ceilings against the
   floor.
2. If above the floor, the candidate plus its equivalence tests.
3. The paired campaign results and a measured recommendation to the human. No
   auto-commit.
