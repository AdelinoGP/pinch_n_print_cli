# Windowed clip area in internal-bridge qualification

Type: task
Label: wayfinder:task
Status: resolved (2026-10-30; measured **REJECTED on exactness** — the windowed
form clears the floor at **1.57–1.60 %** of the cell but deterministically changes
the internal-bridge **direction** on the screening cell; no candidate, no
campaign, probe reverted. See ## Answer)
Assignee: 2026-10-30 OpenCode wayfinder session — the human said "continue the
perf vs orca map"; the frontier re-derivation at HEAD `0b25a8b9` found **95/95
children resolved**, the session offered six measured fog routes, and the human
selected **windowed clip-area dilation**
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Whole-layer deep-infill offsets](93-whole-layer-deep-infill-offsets.md)

## Question

Can the whole-layer `+1.5 × spacing` clip-area dilation in
`compute_internal_bridge_layer`
(`crates/slicer-runtime/src/slice_postprocess_prepass.rs`) be confined to a
window around the candidates that consume it — without changing a single
qualification verdict, a single internal-bridge angle, or a single emitted
byte — and does the resulting wall win clear the map's **0.80 %** floor in
**both** modes?

**Screen first, and screen the wall, not only the accumulated time.** Ticket 93
built the sibling windowed-erosion candidate, cleared the floor at **1.38–1.43 %**
of the ordinary cell and won both ordinary cells, but the two accelerated cells
were wall-neutral (`+0.13 %` / `+0.62 %`), so the strict four-cell clause failed
and the human dropped it. The map's fog records the lesson: a host-side work
removal can be wall-neutral in accelerated cells, and the gate must be priced
**before** the campaign, not after it. Phase 1 therefore measures per-mode wall
on the screening cells and stops if the accelerated arm does not move.

## Measured premise

The clip area is the **largest single term** in the function, measured twice on
ordinary base classic supports-off (the ticket's screening cell):

| take | run 1 | run 2 | calls | wall-eq | share of 73.17 s |
|---|---:|---:|---:|---:|---:|
| t92 (2026-10-27) | 12,462.9 ms | 11,375.2 ms | 130 | 1,233.2 ms | **1.69 %** |
| t93 (2026-10-28) | 12,770.0 ms | 12,434.6 ms | 130 | 1,354.4 ms | **1.85 %** |

Accumulated over the layer axis; wall-equivalent = accumulated ÷ the run's own
cpu/wall ratio (the map's established conversion, t89 → t91 → t92 → t93).
Floor **0.585 s**. The dilation is **2.1–2.3x the floor** before any windowing
loss.

Operand facts, measured:

- `deep_infill_area` is the unioned sparse infill over the bridge-depth window:
  **50 polygons / 7,084 ring vertices** median, 318 / 13,388 max.
- The offset costs a median **91.3 ms** per call (p90 174.9 ms, max 264.8 ms) —
  ring geometry, not polygon count.
- `region.internal_solid_fill` is tiny: **196 surfaces over 130 calls**, max 7.
  Every one of the 130 calls has at least one such surface (t93: `bbox_skip=1`
  on 0 of 130), so an emptiness skip is worth **zero**.
- A bbox window around the *qualified* set grown by `14 × spacing` retained
  **12.9 % of the operand's ring vertices** (median, 42 calls) and the windowed
  erosion cost **9.5 %** of the production erosion's time — so a window that
  covers the candidates, not the qualified set, is the only open question.

## Consumers of the clip area (source-verified 2026-10-30)

`deep_infill_clip_area` has exactly three consumers, all inside
`compute_internal_bridge_layer`; the value is never returned and never stored:

1. the per-surface qualification operand:
   `intersection(offset(surface, spacing), clip_area).is_empty()`;
2. `internal_bridge_angles`' `expansion_area` term:
   `intersection(closing_ex(expansion_area, ε), clip_area)`, which enters
   `limiting_area = area ∪ …` and then `boundary_anchor_polylines`;
3. `internal_bridge_angles`' per-candidate `area`:
   `intersection(offset(candidate, spacing), clip_area)`.

The function returns only `qualified` (a subset of `internal_solid_fill`) and
`angles`; both are compared by the emitted G-code.

## Exactness argument (to be verified empirically, not assumed)

Let `A` be `deep_infill_area`, `s` the qualify spacing, `r = 1.5 s`, and
`C = offset(A, r)` the production clip area. Clipper2's Miter limit 2.0 bounds
every offset vertex within `2r` of `A`, and membership in `C` is exactly
`dist(p, A) ≤ r`.

- **Qualification and `area`.** `offset(surface, s) ⊆ surface ⊕ 2s`, so the
  consumer reads only `S ⊕ 2s`. Every point `p` there has its nearest `A` point
  within `r` of `p`, hence inside `S ⊕ (2s + r)`. Clipping the operand to
  `S ⊕ (2s + r)` and the result back to `S ⊕ 2s` leaves the consumer's view
  unchanged.
- **Anchors.** `total_fill_area ⊇ S`, so `offset(total_fill_area, 1.3 s)` is
  always within `3.3 s` of any sample on a candidate edge. A removed
  `expansion_area ∩ C` part outside a window `W = S ⊕ 8s` is farther than
  `8s` from every surface, so its `0.3 s` anchor ring is farther than `5.7 s`
  from any sample and can never win `nearest_anchor_angle`.
- **Shape used.** `W` = union over surfaces of `box(S) ⊕ 8s`; operand =
  `intersection(A, W ⊕ 3s)`; result = `intersection(offset(operand, r), W)`.
  Both intersections are ordinary `intersection` calls; no geometry function
  changes.

The argument is a *plan* for the proof, not the proof: Phase 1 also checks the
production and windowed clip areas for set equality inside `W` on every call,
and Phase 2's gate is byte-identical G-code.

## Phase 1 — screening (mandatory)

1. Env-gated probe (`PNP_T96_PROBE=1`) on ordinary **and** accelerated base
   classic supports-off: per call, the production dilation span, the windowed
   operand + clip + offset spans, the operand and window shape counts, and a
   `difference`-both-ways set check of `C_prod ∩ W` against `C_win` inside `W`.
2. A windowed arm (`PNP_T96_WINDOW=1`) run **uninstrumented** on both modes,
   paired against the baseline arm in the same session, with outputs
   byte-identical to `292df501…` (21,023,739 bytes).
3. Convert the removable accumulated time to wall (run's own cpu/wall) and
   compare against the floor; **and** report the paired uninstrumented wall
   delta per mode.
4. **Stop rule:** if the accelerated arm's paired wall delta is zero or
   positive (no win), close as **measured-dead** with the probe reverted and no
   candidate — the t93 failure mode is not re-run through a campaign.
5. If above floor in both modes, proceed to Phase 2.

## Phase 2 — candidate (conditional)

1. Replace the whole-layer dilation with the measured windowed form; no
   geometry function changes, no IR, no guest change.
2. Emitted G-code byte-identical to the post-t81 reference set in all sixteen
   cells, both modes.
3. Unit tests: (a) a fixture whose candidates are far from most of the deep
   area, comparing `compute_internal_bridge_layer`'s `qualified` and `angles`
   against the whole-layer path; (b) a fixture with two disjoint candidate
   clusters; (c) an empty-candidate fixture.
4. Standing paired ordinary + accelerated gate on the four base classic cells,
   6 alternating pairs per cell, 12 threads, zero starvation exclusions, every
   pair byte-identical, process CPU ≤ baseline + 3.0 %.

## Pre-declared floor

**0.80 % of the cell wall.** Ordinary base classic supports-off is **73.17 s**
(t87), so the floor is **0.585 s**; accelerated is **72.92 s**, floor **0.583 s**.
Phase 1 measures the removable ceiling and the per-mode wall; Phase 2's campaign
measures the actual win.

## Proof gate

- Phase 1 probe reverted; both guest freshness gates exit 0.
- Phase 2 only: 16-cell output-identity matrix, both modes, against
  `t91-exact-z-memo/references.json`; the unit tests above.
- Phase 2 only: the standing four-cell paired gate.
- Both guest freshness gates exit 0 (host-only candidate).

## Deliverables

1. The Phase 1 table: per-mode dilation accumulation, windowed counterfactual,
   set-equality verdict, and paired uninstrumented wall delta.
2. If above the floor in both modes, the candidate plus its equivalence tests.
3. The paired campaign results and a measured recommendation to the human. No
   auto-commit.

## Answer

**Measured REJECTED on exactness — no candidate, no campaign, probe reverted
(2026-10-30, HEAD `0b25a8b9`).** The windowed form is **2.0x the floor** on price
and **not representation-safe**.

Phase 1, ordinary base classic supports-off, one host binary with env-gated arms,
130 calls per run:

| term | run 1 | run 2 | wall-eq (run 2) | share of 73.17 s |
|---|---:|---:|---:|---:|
| production whole-layer dilation | 11,813.6 ms | 11,672.7 ms | — | — |
| windowed operand + offset + result clip | 1,033.1 ms | 1,019.3 ms | — | — |
| **saving** | **10,780.5 ms** | **10,653.5 ms** | **1,171 ms** | **1.60 %** |

The two runs agree within **1.2 %** (window priced cold-after-full vs
full-priced-cold-after-window), and the windowed operand keeps **12.2 %** of the
deep-area ring vertices (median) — the same retention t93 measured for its
erosion window (12.9 %).

**The rejection.** The windowed arm emits `5fc8fa72…` / 21,023,889 bytes against
the reference `292df501…` / 21,023,739, identically in two independent runs
(`PNP_T96_WINDOW` alone and with the probe). Five G-code blocks differ; the first
is an `Internal Bridge` at `;Z:21.4` with **8 → 10** moves, i.e. a changed bridge
**direction**. At that layer the production and windowed clip areas are
**set-equal inside the window** (`mismatch_polys = 0`), so the qualification
verdict and per-candidate `area` are unchanged. The failure is therefore the third
consumer: `internal_bridge_angles`' `expansion_area ∩ clip_area` term, which
feeds `boundary_anchor_polylines` and `determine_bridging_angle`.

The argument the ticket planned — `total_fill_area ⊇ surface` puts a
`1.3 × spacing` anchor within `3.3 × spacing` — is **false on this job**. With
qualification and `area` unchanged, the only input left is the anchor set, so the
nearest anchor lay **beyond the 8 × spacing window** (or the cut introduced a
closer ring): the anchor reach is not bounded by the planned `3.3 × spacing`.
When a candidate sits in the interior of the fill/expansion area, neither
`offset(total_fill_area, 1.3 × spacing)` nor `offset(limiting_area, 0.3 × spacing)`
has a nearby ring, so `nearest_anchor_angle` reads the expansion-area boundary
at whatever distance it happens to be; removing or adding it at the window edge
changes the modal direction. No finite candidate-driven window is safe for this
consumer; this is the mirror of t93, where the same operand's **erosion** was
safe because its consumers (`retain`, per-candidate `offset(candidate, spacing)`)
are bounded by `2 × spacing`.

Two secondary facts, recorded: **116 of 130** calls report a non-zero
inside-the-window set difference (**1,071** sliver polygons total), so the cut
edge perturbs the offset at the boundary even before the anchor argument; and the
single uninstrumented window-arm wall (**76.557 s**, cpu/wall 9.26) is unpaired and
is **not** a wall claim — the wall screen was never reached.

**What survives for the map.** The target is right: the whole-layer dilation is
**11.7–11.8 s accumulated**, and a 12 %-vertex window removes 91 % of it. The
remaining safe shapes keep the **whole** operand and make the offset itself
cheaper — e.g. offsetting the disjoint deep polygons separately and unioning
(dilation distributes over a disjoint union), if measured below the whole-layer
call — or change the angle consumer, which is parity work
(`determine_bridging_angle`). Both are fog, not candidates.

Disclosures: instrumented walls are attribution only; the `8 × spacing` /
`11 × spacing` margins are the ticket's plan, and a margin sweep was not run
because the anchor reach is not bounded by the consumer's reach. Both guest
freshness gates exit
0 before and after; the tree carries no production change. Evidence:
[`evidence/t96-windowed-clip-area/`](../evidence/t96-windowed-clip-area/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 6/6 negative controls rejected).
