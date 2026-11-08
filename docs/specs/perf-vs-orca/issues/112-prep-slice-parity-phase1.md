# `PrePass::Slice` parity Phase 1: canonical closing vs the artefact-bound gate

Type: task
Label: wayfinder:task
Status: resolved (2026-11-03; **build P1** recommended — canonical flat-bridge
classification replaces the enclosure gate; ceiling clears the 0.80 % floor in
all 8 cells by 19.1x–62.5x with a bounded, measured delta; P2 deferred, P3
rejected. See ## Answer)
Assignee: 2026-11-03 OpenCode wayfinder session (ticket picked as first frontier
child at HEAD `2dade816`; the human said "continue the perf vs orca map")
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [ticket 111](111-prepass-rewrite-scope-ruling.md)'s ruling
(Q3/Q6, 2026-11-03: scoped parity exception, `PrePass::Slice` first, Phase-1
parity analysis before any production edit).

## Question

What does canonical OrcaSlicer's closing pipeline do differently from PnP's
flat-bridge closing, and which of those differences can be expressed as a parity
shape over the per-component **>= 10 %** overlap gate? For each viable shape,
what is its pre-declared wall ceiling and output delta on the eight map cells?

## Context

- [Ticket 59](59-prep-slice-flat-bridge-closing-reduction.md) closed the
  representation-safe family. The closing result is consumed only as the
  per-component **>= 10 %** overlap gate inside
  `assemble_flat_bridge_areas`
  (`crates/slicer-core/src/algos/prepass_slice.rs`); the reported geometry is the
  untouched component. A local-closing candidate flipped gate verdicts on
  **92/239** benchy layers and **320/469** base layers; the far-rejection skip
  family had **0 hits**; the local shape was only **~12 %** cheaper in
  accumulated thread-time.
- [Ticket 104](104-prepass-vs-per-layer-attribution.md) measures
  `PrePass::Slice` at **41,472 ms (51.2 %)** of base classic-off and **44,558 ms
  (19.5 %)** of base classic-on.
- [Ticket 111](111-prepass-rewrite-scope-ruling.md) authorizes a **scoped
  canonical-parity exception** for the prepass and requires this analysis before
  any production edit. The acceptance contract for a later candidate is: fresh
  **16-cell** reference set, arms byte-identical within themselves, baseline
  reproduces the post-t81 set, the candidate-vs-old-reference G-code delta is
  audited before timing, and the standing paired wall/CPU gate at the
  **0.80 %** floor passes in **all 8 cells**.
- Canonical citation rule: cite OrcaSlicer by file + function name, **never by
  line number**. If a local checkout is unavailable, state the evidence gap and
  do not infer canonical behavior from memory.

## Scope

Source-level parity and consumer analysis only: no production edit, no candidate,
no timing campaign, no reference-set generation.

1. Re-derive the consumer/reach map of the closing result at the current HEAD.
   The definitions live in `crates/slicer-core/src/algos/prepass_slice.rs`
   (`assemble_flat_bridge_areas`, `gate_bridge_areas_by_unsupported_span`,
   `update_external_bridge_orientation`) and are called from
   `crates/slicer-runtime/src/slice_postprocess_prepass.rs`; also follow
   `sync_perimeter_infill_areas_into_slice`
   (`crates/slicer-runtime/src/region_partition.rs`), the guest consumers named
   in t59's answer, and the visual-debug SliceIR capture.
2. Pin the canonical difference: which OrcaSlicer functions and which
   configuration (`ORCA_MORPH_MITER_LIMIT`, `morph_passes`,
   `slice_closing_radius`) produce the artifact field PnP's gate sees, and what a
   parity shape would have to reproduce or replace.
3. Design **1–3** parity shapes. For each: the canonical behaviour it tracks, the
   output delta it can cause, the wall ceiling (measured, or bounded from the
   existing profile), the cells it moves, and the risk.
4. Recommend **build** (one shape) or **stop**.

## Deliverable

A resolution with:

- the consumer/reach map at HEAD;
- the canonical-vs-PnP difference, both sides named by file + function;
- the candidate shapes with pre-declared ceilings and delta bounds;
- a recommendation, and if stop, the ceiling against the 0.80 % floor on all 8
  cells and which cells fail.

## Stop rule

If no shape's ceiling clears **0.80 %** on **all 8 cells**, or the output delta
cannot be bounded, record the ceiling and stop: no candidate ticket, no
production edit. A shape that only clears the floor on base classic-off is not
enough for the all-8-cell gate.

## Acceptance

The analysis is a resolution with the evidence attached; no production edit and no
timing campaign. If a shape is recommended, the next ticket is a candidate
implementation under ticket 111's acceptance contract.

## Evidence starts

- [`evidence/t59-flat-bridge-closing/FINDINGS.md`](../evidence/t59-flat-bridge-closing/FINDINGS.md)
  and its `reduced/` set.
- [`evidence/t27-serial-floor/PREMISE-AUDIT.md`](../evidence/t27-serial-floor/PREMISE-AUDIT.md)
  and `FLAT-BRIDGE-EXPERIMENT.md`.
- [`evidence/t104-phase-split/phase-split.json`](../evidence/t104-phase-split/phase-split.json)
  (stage walls and shares).
- `crates/slicer-core/src/algos/prepass_slice.rs`,
  `crates/slicer-core/src/polygon_ops.rs` (`closing_ex`),
  `crates/slicer-runtime/src/slice_postprocess_prepass.rs`,
  `crates/slicer-runtime/src/region_partition.rs`.
- Canonical OrcaSlicer source, if a local checkout is available; otherwise record
  the evidence gap.

## Answer

**Verdict: build P1 — replace the enclosure gate with canonical OrcaSlicer's
flat-bridge classification. The ceiling clears the 0.80 % floor in all eight
cells by 19.1x–62.5x, and the output delta is bounded and measured on both heavy
fixtures. P2 is deferred (its delta cannot be bounded without canonical's wave
expansion); P3 is rejected on correctness. No production edit was made.**

Evidence: [`evidence/t112-prep-slice-parity/`](../evidence/t112-prep-slice-parity/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 9/9 negative controls rejected).

### 1. Canonical has no enclosure closing

`PrintObject::detect_surfaces_type` (`PrintObject.cpp`) computes the bottom
surface as `opening_ex(diff_ex(layer_slice_surfaces,
safety_offset(lower_layer_slices), ApplySafetyOffset::Yes),
external_perimeter_flow.scaled_width() / 10)` and tags it `stBottomBridge` unless
`bottom_is_fully_supported` (which needs `support_top_z_distance == 0`; the
default is **0.2 mm**, so **every** unsupported bottom is a bridge in all eight
matched cells). No enclosure test, no 12 mm radius, no per-component fraction
gate.

The only closing in canonical's bridge path is `merge_bridges`
(`LayerRegion.cpp`): `closing_ex(bridge_polygons ∪ wave_expanded_anchors,
0.55 × 0.65 × 1.05 × solid_infill_spacing)` — ≈ **0.15–0.17 mm** — to fill
unassigned holes after shell regularisation. Different operand, **~70–80×
smaller radius**, different consumer.

PnP's `closing_ex(support = slices − lower, 12 mm)` + `≥ 10 % refill` gate
(`assemble_flat_bridge_areas`, `crates/slicer-core/src/algos/prepass_slice.rs`) is
a PnP-local discriminator with **no canonical counterpart and no `DEV` row**.
Therefore no shape can preserve the gate's exact verdicts *and* claim parity; the
parity question is to replace the gate.

### 2. Consumer map at HEAD (`2dade816`)

t59's map holds. Additions since then: `arachne-perimeters` and `infill-linker`
read `SliceRegionView::bridge_areas()`, `rectilinear-infill` splits external vs
internal bridges, packet 234a's `gate_internal_bridge_sites`
(`crates/slicer-runtime/src/slice_postprocess_prepass.rs`) uses
`region.bridge_areas` as its `boundary_plines` operand, and
`paint_segmentation::split_regions` intersects and re-saves the field across
modifier splits. The direct consumer of the closing result is still the single
boolean gate; every other consumer takes the emitted polygons.

### 3. Shapes

- **P1 (recommended)** — in `assemble_flat_bridge_areas`, delete the closing and
  the fraction gate; emit `opening_ex(flat_bridge, 0.1 × external_perimeter_width,
  Miter, 3.0)` and keep every component. Add canonical's `ApplySafetyOffset::Yes`
  (inflate the lower set by `SCALED_EPSILON` = 1e-4 mm = 1 PnP unit before the
  diff) and resolve the width as `resolve_contact_params` does
  (`outer_wall_line_width`, else `line_width`). `flat_bridge_closing_join` becomes
  an unused key.
- **P2 (deferred)** — canonical classification over the whole unsupported set,
  retiring the facet-derived `assemble_bridge_areas`. Its **ceiling is at least
  P1's** (it also removes the facet offsets and the bottom-footprint work), but
  canonical follows it with wave expansion into shell/sparse/top zones
  (`expand_expolygons` → `propagate_waves_ex`) and trims those zones; PnP offsets
  by a fixed 1.0 mm `expansion_margin_mm` and trims nothing. The G-code delta is
  unbounded until that port exists, so the stop rule fires on the **delta**
  clause. Next prepass take, not this campaign.
- **P3 (rejected)** — keep the gate and swap in `merge_bridges`' radius. Not the
  canonical operand or consumer; collapses the gate to micro-gaps while t59
  measured verdicts on knife-edges (min distance to the threshold 4e-5 / 2.4e-4).

### 4. Ceilings (all eight cells)

Fresh `--profile` on both fixtures at HEAD: the closing is **86.8 %** (benchy) /
**70.8 %** (base) of `PrePass::Slice`'s accumulated thread-time. Ceiling =
`min(closing thread-time, stage wall) − measured canonical opening`.

| cells | net ceiling | % of cell | × the 0.80 % floor |
| --- | ---: | ---: | ---: |
| benchy (4 cells) | **1,893 ms** | 15.3–24.7 % | **19.1x–30.8x** |
| base (4 cells) | **37,682 ms** | 23.4–50.0 % | **29.2x–62.5x** |

The opening itself costs 89.9 ms / 246.7 ms, i.e. **146x / 853x** cheaper than
the closing it replaces. Arachne cells reuse the classic measurement on a
source-level argument: `PrePass::Slice` precedes shell/perimeter generation,
`wall_generator` is never read there, and the stage's only config inputs are
`slice_closing_radius` and `flat_bridge_closing_join`.

### 5. Delta (bounded, measured)

Committed flat-family output changes on **182 / 239** benchy layers and
**437 / 469** base layers. Net area **+8.86 mm² (+2.0 %)** / **+115.90 mm²
(+35.9 %)**: the gate rejected **68.4 / 187.2 mm²** of free-edge and cantilever
area that canonical calls bridges, while the 0.04 mm opening removed
**59.58 / 71.27 mm² (11.8 % / 14.0 %)** of thin features (component population
3,268 → 315 and 11,693 → 2,454). A bonus: layer 0 pays a 12 mm closing whose
result the next stage clears — **595.8 ms** on benchy, 1.1 ms on base; P1 removes
it.

### 6. What the candidate owns

Under [ticket 111](111-prepass-rewrite-scope-ruling.md)'s contract: fresh
16-cell reference set; baseline reproduces the post-t81 set; G-code delta audited
before timing and confined to the flat-preference area; paired wall/CPU gate at
0.80 % in **all 8** cells. The shape is graduated as
[ticket 116](116-prep-slice-canonical-bridge-classification.md). Tests to move:
the enclosure unit tests in `crates/slicer-core/tests/algo_prepass_slice_tdd.rs`,
`bridge_false_site_gating_tdd.rs`, `bridge_detector_tdd.rs`, and the reference
G-code set.

### What this does not authorize

- No P2, no `assemble_bridge_areas` retirement, no wave-expansion port.
- No radius swap on the existing gate (P3).
- No production edit in this ticket; the probe is in `probe.patch` and was
  reverted.
- Ratios of record still predate the `a5e9a403` merge;
  [ticket 115](115-post-merge-reconciliation.md) owns re-baselining them.
