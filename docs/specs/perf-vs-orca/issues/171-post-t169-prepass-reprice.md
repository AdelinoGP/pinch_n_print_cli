# Re-price the prepass work-set, the phase split and S5/S6 after t169

Type: task / attribution + re-price (instrumented, no candidate)
Label: wayfinder:task
Status: open
Assignee: unclaimed
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [t169 offset edge decimation](169-canonical-offset-edge-decimation-and-flat-opening.md)
([`evidence/t169-offset-decimation-flat-opening/FINDINGS.md`](../evidence/t169-offset-decimation-flat-opening/FINDINGS.md)),
which cut whole-cell wall by **21.91–25.08 %** on the four `base` cells and
**5.74–15.21 %** on the four benchy cells and cut `polygon_ops::offset`'s own
self wall by **41.0–55.5 %**. Every operand-volume price in the map was taken
against the pre-t169 primitive.

## Question

**After t169's decimation, what do the prepass work-set, the phase split, and
the two priced operand shapes (S5 windowed internal-bridge offsets, S6 exact
occupancy memo) cost — and does any of them still clear the 0.80 % floor on all
eight cells?**

Concretely:

1. **Phase split.** Re-derive prepass / per-layer / postpass / outside shares on
   all eight cells. t164's split (prepass **67.0–87.2 %**, per-layer 9.3–26.3 %,
   postpass 3.2–6.8 %) was measured on the pre-t169 tree, where the prepass was
   the offset-heavy phase; t169 removed 41–55 % of the offset primitive's
   self-time, so the shares have moved and every projection built on them is
   stale.
2. **Prepass stage split.** Which stages now carry the supports-off cluster and
   the supports-on cluster? t126/t164's `ShellClassification` (14.7 / 15.3 s) and
   `SupportGeometry` + `SupportAnalysis` (68.9 s of 126.3 s) figures are
   pre-decimation. In particular, is `PrePass::InternalBridges` still worth
   anything after the `Internal Bridge` section itself fell **72.0 / 73.2 %** in
   path on the two base supports-off cells?
3. **S5 and S6 re-price.** t131 priced S5 at **21.0 s thread / 2.5 s wall** on
   base classic-off and S6 at **14,227.7 ms thread / 6,504 ms wall** on base
   classic-on. S5's operand is the internal-bridge offset path, whose downstream
   section already collapsed; S6's occupancy offsets run through the same
   decimated harvest. Neither number may be quoted again until it is re-measured
   on the current tree.
4. **Per-op primitive census.** Re-run the t131-style per-op split (sweep vs
   wrapper) or its cheaper `--profile` analogue, so the next engine question is
   asked about costs that survive canonical wrapper parity rather than costs
   decimation already removed.

## What is already known

- **t169 landed** the canonical `append_offset_path` harvest
  (`0.005 * |delta|`, both passes of `offset2_ex` with pass 2 at
  `|delta2| * 0.005`) and it is the currency.
- **`polygon_ops::offset` self wall (one `--profile` pass/cell, attribution
  only):** 40.3 / 110.3 / 39.3 / 115.2 s for base classic-off / classic-on /
  arachne-off / arachne-on, against 82.1 / 247.9 / 80.5 / 245.8 s before t169.
  `offset2_ex` fell 12.6–25.1 % on the same passes.
- **t163's board** is the last full two-tool scoreboard (ordinary/accelerated):
  benchy 1.782 / 2.802 / 1.940 / 2.829, base 1.389 / 2.710 / 1.457 / 2.618. It
  is pre-t169; t169's §9 route table is an **estimate** built from it, not a
  measurement of record. A fresh two-tool board is a separate question and is
  not in this ticket's scope.
- **Guest fuel is not uniformly lower:** it falls 13.5–21.5 % on the classic
  cells and rises 1.4–3.1 % on the three Arachne cells, so a re-price must not
  assume the guest half got cheaper everywhere.

## Scope

1. One instrumented window, three passes per cell (`--instrument-stderr`, the
   t164 harness is the template), all eight cells, every pass validated
   (exit 0, `degraded: false`, G-code hash equal to
   [`reference.json`](../evidence/t169-offset-decimation-flat-opening/reference.json)).
2. A `--profile` pass per cell per arm (or a cheaper per-op census) to attribute
   the surviving `polygon_ops` scopes.
3. Re-price S5 and S6 from the measured stage and op splits, with the arithmetic
   shown, and state for each whether the 0.80 % all-8-cell floor is still
   reachable.
4. A short list of shapes that survive, each with its measured carrier share —
   or the explicit finding that no operand-volume shape survives and the route
   must be re-aimed.

## Acceptance

- `evidence/t171-post-t169-reprice/` with the instrumented rows, the re-derived
  phase and stage splits, the S5/S6 re-price arithmetic, `FINDINGS.md`, and a
  checkout-only `verify.py --self-test` that re-derives every quoted number from
  the committed reduced set.
- Map: destination-status block, Decisions-so-far line, and an updated
  **Not yet specified** (either the surviving shapes with their new prices, or
  the re-aim).
- No production edit. This ticket prices; it does not build.

## Answer

_(pending)_
