# t107 — `PrePass::SupportGeometry`: the producer is 9 ms, the planner is the 91.9 s

Route: [ticket 107](../../../issues/107-support-geometry-producer-decomposition.md).
Screen at HEAD `9a1921ac`, `target/release/pnp_cli.exe` = the ticket-103 binary
`4cb96942…` (no Rust change since t103; the tree carries docs and the rig's
`tree_strong` pin only).

## Headline

**`PrePass::SupportGeometry` is two stages sharing one stage id.** Re-pairing
ticket 104's raw instrumented streams gives, on base classic-on:

| bracket | ms | share of the instrumented cell | modules |
| --- | ---: | ---: | --- |
| host producer (`host:support_geometry`) | **9** | **0.0039 %** | `host:support_geometry` 9 |
| planner module stage | **91,893** | **40.13 %** | `com.core.tree-support-planner` 87,824 · `com.core.traditional-support-planner` 300 |
| t104's committed sum | 91,902 | 40.13 % | both |

Benchy classic-on is the same shape: producer **0 ms**, planner **3,260 ms**
(**26.17 %** of the cell; tree 2,943 · traditional 30). The producer is
**0.0068x the 0.80 % floor** on base-on and below it everywhere else — the
entire stage's producer side is **two orders of magnitude below the floor**, so
no shape on it can be a route. The stop rule fires: **no candidate, no
campaign, no production edit.**

**Ledger correction.** Ticket 104's `PrePass::SupportGeometry` column
(91,902 / 3,260 ms) is essentially the **planner module stage**, which ticket 88
had already attributed (55.5 s planner, "support geometry, not a host batch
shape") and the map carries as fog. The host producer it also contains is
9 ms / 0 ms.

## Table 1 — the two brackets, all four classic cells

Brackets are instrumented attribution only; cell fractions divide by the same
run's `slice_complete` wall. Floor = 0.80 % of the cell (base classic-on uses
ticket 102's committed 1.318424 s; the others 0.80 % of their ticket-103 PnP
wall).

| cell | producer ms | producer share | producer ÷ floor | planner ms | planner share | planner ÷ floor | tree / traditional |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| benchy classic-off | 0 | 0.00 % | 0.00x | 43 | 0.53 % | 0.70x | 22 / 20 |
| benchy classic-on | 0 | 0.00 % | 0.00x | 3,260 | 26.17 % | 32.85x | 2,943 / 30 |
| base classic-off | 4 | 0.0049 % | 0.0066x | 227 | 0.28 % | 0.38x | 115 / 112 |
| base classic-on | 9 | 0.0039 % | 0.0068x | 91,893 | 40.13 % | 69.70x | 87,824 / 300 |

The supports-off cells show the split from the other side: there the bracket is
almost all **planner declining** (227 / 43 ms) with the producer at 4 / 0 ms, and
the whole thing is below the floor — while on the supports-on cells the planner
owns 99.96 % of it and does real work.

## Table 2 — why the producer is cheap: the IR is small

`pnp_cli support-preview` on benchy classic-on (clean binary, ~12 s) renders the
non-sentinel `SupportGeometryIR.entries`:

| quantity | value |
| --- | --- |
| layers | 240 |
| geometry polygons | **577** |
| geometry points | **444,259** |
| geometry heap (points × 16 B + polygons × 24 B) | **~7.1 MB** |
| sentinel (intermediate-layer) entries | **1** |
| support-plan body polygons / points (rendered beside it) | 798 / 354,208 |

A deep copy of ~7 MB is sub-millisecond, which is why the producer's 0 ms
benchy bracket is real work and not a skipped stage: the IR is a **coarse
outline subset** at support layers, not a copy of the slice geometry. Base's
9 ms scales the same way.

## Findings

1. **The shared stage id is the whole story.** `stage_start`/`stage_complete`
   are keyed on `(stage, layer)`; the host builtin and the planner module stage
   both carry `PrePass::SupportGeometry`, so t104's reducer summed them. The
   reducer here pairs each completion with its own start and reads the nested
   `module_complete` rows, and it reconciles exactly with t104's committed
   91,902 / 3,260 ms — the arithmetic is consistent; the *attribution* was not.
   The scope of the correction is one stage: `PrePass::SupportGeometry` is the
   **only duplicated prepass stage id** in all four streams (`summary.json`
   records the scan); the repeated `Layer::*` ids are the 240/495 per-layer
   instances and remain informational thread-sums, not double brackets.
2. **The producer cannot be a route at any shape.** Its bracket is 0–9 ms; the
   floor is 1.318 s (base) / 0.099 s (benchy). Even deleting the entire producer
   changes no cell by a measurable amount. There is nothing to split, cache, or
   batch.
3. **The supports-on lever is the planner, unchanged.** Tree planner
   **87,824 ms = 95.6 %** of the base-on planner bracket and **2,943 ms =
   90.3 %** of benchy-on; the traditional planner is 300 ms / 30 ms (it runs and
   declines). Implied uninstrumented wall: **64.5 s** of the 160.857 s base-on
   cell and **3.25 s** of the 12.403 s benchy-on cell. That is the same term
   ticket 88 measured at 55.5 s on 2026-10-25; the difference is vintage and
   instrumented-run inflation, and neither is a new candidate.
4. **The sentinel intermediate-layer work is not a lever either.** Exactly one
   sentinel entry exists on benchy-on (one per object), and the preview shows
   its consumer (the preview counts it as skipped) and the planner discards it
   (`layer_idx >= layer_count`). Its whole cost sits inside a 0 ms bracket.
5. **The support engine's size is real, its location was mislabelled.** With
   `SupportAnalysis` (22,254 ms = 9.7 % of base-on) the support work is still
   ~50 % of the cell, as ticket 104 said — but the planner, not the geometry
   producer, carries 40.1 points of it.

## Disclosures

- **No new slice pass, no wall claim, no probe build.** The decisive
  measurement is t104's own committed raw stream, re-paired. A probe patch
  inside `execute_support_geometry` was authored against the ticket's
  pre-declared method and **reverted unbuilt** once the bracket split proved the
  producer question was already answered; it is not preserved because it was
  never executed and no probe measurement is claimed. The working tree is clean
  and `target/release/pnp_cli.exe` remains the recorded `4cb96942…`.
- The streams were captured on 2026-11-02 under disclosed external load (t104);
  the producer/planner split is a *bracket* property, so load inflates both
  sides' absolute ms but not the split. Shares are converted with ticket 103's
  quiet PnP walls, never subtracted from instrumented brackets.
- The preview run is a single ~12 s process at ~5 % load; it is used for
  polygon/point counts only, not timing. Its 77,252,577-byte JSON is
  regenerated per `REPRODUCE.md` and never committed.
- Arachne cells were not measured. The producer's cost is support-layer
  geometry; the Arachne generator does not change the prepass outline set, but
  no claim is made about the Arachne shares.

## Reproduction

[`REPRODUCE.md`](REPRODUCE.md) — `reduce.py` re-derives `summary.json` from
t104's raw streams, `count-preview.py` re-checks the IR counts, and `verify.py`
exits 0 checkout-only (**71/71 claims**, **9/9 negative controls detected**).
