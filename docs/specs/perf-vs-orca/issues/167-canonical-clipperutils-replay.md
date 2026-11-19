# Canonical-kernel replay: how fast are Orca's own ClipperUtils ops on PnP's prepass operands?

Type: task / measurement (canonical side, no PnP change)
Label: wayfinder:task
Status: open
Assignee: unclaimed
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: the [t166 probe](../evidence/t166-shell-opening-parity/FINDINGS.md)
§4, which challenges the premise of the operand-volume program (t129 → t131).

## Question

**If PnP's captured prepass operands run through canonical's *actual*
implementation (`ClipperUtils.cpp` on ClipperLib/Clipper1, with
`offset_expolygon_inner`'s per-contour offsetting and `ShortestEdgeLength`), how
does the per-op time compare with PnP's Rust Clipper2 on the same operands with
the same parameters?**

The map has read "the gap is operand volume" since t131. That reading rests on
t70's claim that the kernel is at parity. But t69/t70's twin was **C++ Clipper2**
(`driver.cpp` includes `clipper2/clipper.h`), and canonical's hot path is not
Clipper2: `ClipperUtils.cpp` uses `ClipperLib::Clipper` / `ClipperLib::ClipperOffset`,
and only a few sites call `Clipper2Utils`. Canonical's operand sizes were also
inferred, never measured. The t166 probe measured 10–11 µs per input point on
the shell opening, and the cost there comes from operand shape (inverted
sliver erosion), not volume. If canonical's kernel and wrapper are several times
faster on identical operands, the route is a kernel/wrapper port, not a hunt for
smaller operands.

## Scope

1. **Build a canonical driver** against the local OrcaSlicer checkout's
   `ClipperUtils.cpp` + its ClipperLib (the t69 harness pattern: one process per
   case, warmup + 5 timed repeats, and parse/flatten outside the timer). Call
   `offset_ex`, `offset2_ex`/`opening_ex`, `diff_ex`, `intersection_ex` and
   `union_ex` exactly as canonical does. Scale coordinates ×100 to Orca's 1 nm
   units.
2. **Corpus**: t69's existing `t68sites` + `t55` cases, plus a new capture of the
   ShellClassification Pass 1 openings (`apply_opening` operands, both PnP's and
   canonical's parameters) and the `gate_internal_bridge_sites` offsets on base
   classic-off.
3. **Three columns per case**: PnP Rust Clipper2 (current wrapper), C++ Clipper2
   (t69's twin, for continuity) and canonical ClipperUtils. Report the medians
   per op class and per site, with output area/vertex deltas so that a fast but
   wrong kernel is visible.
4. **Single-thread and 12-thread loaded**: the stages run at 8–12x parallelism,
   so also run the corpus under a 12-way concurrent load to expose
   memory-bandwidth effects.
5. **No PnP change.** This verifies the premise; adoption is a separate ruling.

## Decision this unlocks

- Canonical is **≥ 3x faster** per op on the hot sites: graduate a
  kernel/wrapper port (per-contour offsetting, edge decimation, or a Clipper1
  port) as the prepass route, and re-price t129's "operand volume" shapes.
- Parity (within 1.5x): t131's reading stands. The remaining lever is work and
  operand shape, and parameter-parity audits (like t166) are the productive
  search.

## Acceptance

- `evidence/t167-canonical-clipperutils-replay/` with the driver source,
  provenance (Orca checkout commit, compiler, flags), the per-case results and a
  `verify.py`.
- A map destination-status block stating which branch fired.

## Answer

_(pending)_
