# Verify canonical's prepass cost: is the 8/8 projection inside t110's tap accuracy?

Type: task / measurement (canonical side)
Label: wayfinder:task
Status: resolved
Resolved: 2026-11-19 — **input verified.** Canonical prepass on 2.5.1 (`5013b1bd…`) is 104.7 ms (bound) / 569.7 / 2,172.4 / 10,368.1 ms (points) for benchy-off / benchy-on / base-off / base-on. That is −0.7…+3.6 % of t110 on the three points; benchy supports-on is 1.036x t110, inside the 1.53x asked about. t164's 8/8 at canonical prepass stands, binding cell `benchy arachne-on` 0.890x → 0.898x, slack 1.53x → 1.47x.
Assignee: 2026-11-18 Claude Code wayfinder session — the human said "pick up the first ticket"; 165 is first on the frontier.
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Phase re-attribution at HEAD](164-phase-re-attribution.md) — at
canonical prepass cost all eight cells now win (**0.396x–0.890x**), but the whole
result rests on one fixed input, t110's canonical prepass wall, and the two
binding cells have only **1.53x / 1.59x** slack on it.

## Question

**On the installed OrcaSlicer 2.5.1, at the matched job, what is the prepass
wall on each of the four fixture/supports classes — and are the two benchy
supports-on values within 1.53x of t110's 549.8 ms?**

t164's arithmetic replaces PnP's prepass with t110's canonical prepass wall and
holds every other phase at this take's measured share. The binding cells are:

| cell | prepass budget | t110 canonical | slack |
| --- | ---: | ---: | ---: |
| benchy arachne-on | 0.840 s | 0.550 s | **1.53x** |
| benchy classic-on | 0.874 s | 0.550 s | **1.59x** |
| base classic-on | 15.690 s | 10.445 s | 1.50x |
| base arachne-on | 18.228 s | 10.445 s | 1.75x |

`benchy classic-off`'s 50.5 ms is already flagged in t110 as a **bound, not a
point** (below 3x the tap's 138.9 ms worst poll gap); the benchy supports-on
values are points. If any supports-on class measures materially above its
budget, t164's 8/8 stops holding and the destination needs per-layer work again.

## Scope

1. **Re-run t110's tap on 2.5.1.** The `plate_1.gcode.tmp` → `plate_1.gcode`
   rename plus per-growth CPU sampling; `--info` as the startup+import control;
   medians of 3 per class. Stamp the executable hash (t161's boundary).
2. **Four classes, not eight cells**: benchy-off, benchy-on, base-off, base-on.
   The prepass is a function of fixture and supports (t110 measured identical
   values across classic/arachne), so this is 4 × (1 warmup + 3 measured) tap
   runs plus the info controls.
3. **Resolution check first, not last.** Report the tap's poll gap and state
   which values are points and which are bounds, exactly as t110 did. A value
   below 3x the gap is a bound.
4. **Recompute t164's slack table** with the measured values and re-state the
   per-cell projection and the binding cell. If the projection flips, say so.
5. **No PnP change, no candidate, no adoption.** This verifies an input; it does
   not touch the engine.

## Pre-declared gate

**No performance gate.** Correctness bar:

- the tap's wall agrees with the rig's own `GetProcessTimes` wall within 1 % on
  every measured run (t110's accuracy control);
- every run is a matched-job run (matched keys asserted from the resolved G-code
  appendix, as the scoreboard rig does);
- medians of three per class, per-run values disclosed;
- the poll gap is stated and each value labelled point or bound;
- no number is compared against a PnP wall except through t164's own arithmetic,
  recomputed here.

## Acceptance

- A reduced artifact under `evidence/t165-canonical-prepass-cost/` with the four
  class walls, the per-run rows, the tap gap, the point/bound labels, and a
  `verify.py` re-deriving every number and exiting 0 on a plain checkout.
- t164's slack table and binding-cell statement re-derived with the new input.
- The map's destination-status block updated with the verified (or refuted)
  input.

## Answer

**Verified: the input holds and the projection does not flip.** One window,
OrcaSlicer 2.5.1 `5013b1bd…` stamped on all 24 rows, 4 classes × (1 warmup + 3
measured) plus `--info` controls in the same rounds. Every slice run asserts the
current matched job from its resolved appendix, and the tap wall agrees with
`GetProcessTimes` within 0.861 % on every run.

| class | t165 (2.5.1) | t110 (2.5.0) | label |
| --- | ---: | ---: | --- |
| benchy-off | 104.7 ms | 50.5 ms | bound (< 3 × 56.8 ms gap) |
| benchy-on | 569.7 ms | 549.8 ms | point |
| base-off | 2,172.4 ms | 2,103.0 ms | point |
| base-on | 10,368.1 ms | 10,444.8 ms | point |

Re-run through t164's own arithmetic, with only this input changed: **8/8 still
win** (0.400x–0.898x). The binding cell is `benchy arachne-on` at 0.898x, slack
1.47x; next are `benchy classic-on` 0.882x (1.53x) and `base classic-on` 0.783x
(1.51x). A window-scaled sensitivity check (worst 0.906x) flips nothing.
Disclosed: `timeBeginPeriod(1)` did not improve the poll gap (16.6–56.8 ms). The
base prepass carries the 416 ms `--info` spread. The arachne cells reuse the
classic tap, as t164 did. Evidence:
[`evidence/t165-canonical-prepass-cost/`](../evidence/t165-canonical-prepass-cost/FINDINGS.md)
(`verify.py` PASS, `--self-test` 11/11).
