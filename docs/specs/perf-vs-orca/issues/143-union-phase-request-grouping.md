# Union-phase request grouping: batch the planner's union requests across four chunk-batches

Type: task / measured candidate
Label: wayfinder:task
Status: resolved (2026-11-12; measured **KEEP** — buffering depth 12 collapses
  the union tail from 10.09 s to 4.85 s on base classic-on, the clip core from
  14.16 s to 8.93 s, and the paired wall by **9.56 s / 12.80 %** (16.00x the
  0.80 % floor, 10.3 % of the cell's 92.5 s cut). The pre-declared 16-layer form
  **fails its own merge-4 bracket** (5.79 s against a 4.62 s LPT bound) because
  the same 482 requests cost 24 % more thread-time once 16 layers are buffered;
  8 lands on the merge-2 floor and 12 is the measured optimum. All eight cells
  byte-identical; supports-off structurally inert and inside the null band. The
  strict CPU-lower clause passes only 1/4 and is disclosed. See ## Answer.)
Blocked by: none ([137](137-planner-scheduling-gate.md) resolved 2026-11-12 —
  the standing four-cell gate form is confirmed; [136](136-planner-clip-batch-fanout.md)
  resolved 2026-11-12 with the measured ceiling)
Assignee: 2026-11-12 OpenCode wayfinder session — the human said "Continue the
  perf vs orca map" (no ticket named); 143 is the first unblocked, unclaimed
  AFK frontier ticket in order ([144](144-brim-anchor-reconstruction.md) is the
  other). Resolved as a measured KEEP.
Parent: [Slice performance vs OrcaSlicer](../map.md)
Graduated from: [Planner clip-batch fan-out](136-planner-clip-batch-fanout.md)'s
  2026-11-12 Phase-1: the planner's clip core is **4.99x** end-to-end, and the
  deficit is entirely `union_run` — **10,176.2 ms** of the 13,956.4 ms host clip
  wall on base classic-on at **3.66x**, with **4.4 requests per call** and a
  wall equal to the sum of each call's longest request (10,168.3 of
  10,176.2 ms). `carve_run`, the phase the batch API was built for, is at
  **11.56x** with the 256-request cap saturated. Merging the union phase across
  four chunk-batches takes its measured floor from 10,176.2 ms to
  **3,465.0 ms**; LPT's 4/3 bound puts the achievable saving at **5.6–6.7 s**
  (**7.3 %** of base classic-on's 92.5 s required prepass cut, **6.6x** the
  1,010 ms floor).

## Question

Group the planner's union requests across four `EMIT_BATCH_LAYERS` chunks (16
layers) instead of issuing them one chunk at a time, and measure whether it
lands inside the t136 bracket.

The requests are already independent and already carry the same arguments; only
the call grouping changes, so exactness is by construction. Two forms, in
order:

1. **Raise `EMIT_BATCH_LAYERS` 4 → 16.** One-line shape, but it also quadruples
   the buffered collision sets, and the constant is documented as a memory
   bound. Measure peak working set at 4 / 8 / 16 layers before claiming
   anything.
2. **Union-only accumulator.** Defer just the union requests across four chunk
   batches, holding carved geometry rather than collision sets. More code, no
   memory cliff; price it first if form 1 moves peak RSS materially.

Do not widen scope: `set_run` and `body_cut` carry the same width deficit at
1.5 % of the clip core and are fixed by the same change, but the carve phase is
already at 11.56x and must not be touched.

## Pre-declared gate

The candidate is planner-only (`PrePass::SupportGeometry` /
`com.core.tree-support-planner`) and the supports-off cells return before
`plan_for_object`, so the literal all-8-cell 0.80 % gate is structurally
unreachable. It is judged under the standing
[t137](137-planner-scheduling-gate.md) four-cell form:

- positive 0.80 % paired wall/CPU improvement in the four supports-on cells;
- byte-identical output in all eight against the active post-[t139](139-layer-plane-parity.md)
  reference set;
- no median wall/CPU regression in the four supports-off cells.

The measured ceiling is **6,711 ms** (merge-4 floor, union wall 10,176.2 →
3,465.0 ms) and LPT's 4/3 bound puts the achievable saving at
**5,556–6,711 ms** (union wall 3,465–4,620 ms). A candidate that does not
**save at least 3,500 ms** on base classic-on has not reached the priced band
and is a DROP on price. Disclose peak working set and time-to-first-layer at
every buffering depth tried.

## Acceptance

- A measured candidate with the 8-cell output check and the four-cell paired
  gate, on base classic-on and benchy classic-on at minimum.
- The buffering-depth table (4 / 8 / 16 layers): union wall, clip-core wall,
  peak working set.
- A recorded KEEP / DROP with the achieved saving as a share of base
  classic-on's 92.5 s required cut.
- Probe reverted; production edit only if KEEP.

## Answer

**Resolved 2026-11-12; measured KEEP.** The buffering depth is the union phase's
width lever. Depth **12** merges the phase from 109 calls of 4.4 requests into
**37 calls of ~13**, takes the probe's union wall **10,093.1 → 4,848.7 ms**
(**−52.0 %**) and the clip core **14,158.5 → 8,934.5 ms** on base classic-on,
and saves a paired **9.56 s / 12.80 %** of the whole-slice wall
(**65.13 s** from **74.70 s**, 3/3 pairs, **16.00x** the 0.80 % floor).

**The pre-declared 16-layer form fails its own bracket.** t136's merge-4
projection assumed the same per-request cost; at sixteen layers the same 482
requests cost **46.92 s** of thread-time against **37.68 s** at depth 4
(**+24 %**), because sixteen layers of buffered collision sets and the
sixteen-way occupancy payload compete for cache and bandwidth. Its union wall
is **5,793.2 ms** against the **4,620 ms** LPT bound, worse than depth 12's
4,848.7 ms at the same inflated thread-time. Depth 8 lands on t136's merge-2
floor (**5,459.7 ms** vs 5,480.2 ms predicted). **12 was measured after the
table** and is the depth the production constant carries; that selection is
disclosed rather than presented as pre-declared.

**Measured buffering-depth table** (probe build, `RAYON_NUM_THREADS=12`):

| depth | base union | base clip core | base peak RSS | benchy union | benchy clip core |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 4 | 10,093.1 ms | 14,158.5 ms | 2.356 GiB | 447.9 ms | 698.9 ms |
| 8 | 5,459.7 ms | 9,602.9 ms | 2.275 GiB | 283.9 ms | 607.5 ms |
| **12** | **4,848.7 ms** | **8,934.5 ms** | **2.276 GiB** | **200.3 ms** | **425.7 ms** |
| 16 | 5,793.2 ms | 9,751.3 ms | 2.429 GiB | 223.8 ms | 435.3 ms |

Peak working set is **lower** at 8/12 than at 4 on base (−85.8 MiB) and rises
only at 16 (+78.6 MiB); the effective bound is cache/bandwidth, not RSS. The
first per-layer stage event improves 69.7 → 56.4 s on base (−19.0 %).

**Output: byte-identical in all eight cells**, both arms one run each, byte
counts equal, exit 0. Base cells reproduce t139's set exactly
(`ca14d055…` / `585c6a8e…` / `d8fdc1d9…` / `c9fee59a…`); benchy cells match the
take's baseline on the **replacement** benchy export (see the fixture
disclosure below). The arms share a byte-identical host binary (`b238d640…`);
only the planner guest differs (`68b6f811…` → `f6632de0…`).

**Paired gate (3 interleaved pairs, 48 runs):** supports-on wall
**+6.72 / +7.36 / +12.80 / +13.41 %** (benchy classic-on / arachne-on, base
classic-on / arachne-on), **8.40x–16.76x** the floor, 11 of 12 pairs positive.
Supports-off: +2.64 / −0.18 / −1.92 / −3.31 % wall, all inside the same-arm
null band (2.10–4.40 % wall, 0.37–4.24 % CPU) and structurally inert
(`enable_support: false` returns before `plan_for_object`); the candidate was
also faster on both benchy supports-off cells in the single output runs.
**The strict CPU-lower clause passes 1/4** (+0.63 % benchy classic-on; −1.26…
−1.83 % on the rest, inside the ±1.88–1.91 % null CPU band): a wider fan-out
burns more CPU to finish sooner, and the CPU record is disclosed as such rather
than read as cheaper work.

**Route:** 9.56 s is **10.3 %** of base classic-on's 92.5 s required prepass
cut; the ticket's 3,500 ms DROP bar is cleared by **2.7x**. Indicative
composition only: t142's composed 3.695x × (1 − 0.1280) ≈ **3.22x**. A tranche
of t135 Stage 2's order, not a destination step; the operand-volume route
(t129/t131) still owns the rest.

**Fixture disclosure.** The user-supplied `tmp/3dbenchy.stl` was lost and
replaced on 2026-11-12 with a different export of the same model (240
layers / 48.0 mm, ~225.7k triangles). t139's four benchy hashes are retired;
this take records the replacement-fixture baseline
(`a89e449b…` / `dceb7a48…` / `3286449f…` / `be786718…`) and checks
candidate == baseline on it. `tmp/base.stl` is byte-identical to the original,
so every base cell still reproduces t139 exactly. The re-baseline is graduated
as [Benchy fixture replacement](145-benchy-fixture-rebaseline.md).

Evidence: [`evidence/t143-union-grouping/`](../evidence/t143-union-grouping/FINDINGS.md)
(`verify.py` 200 checks, `--self-test` 24/24 negative controls rejected;
`run_depth.py`, `reduce_t143.py`, `run_t143.ps1`, `reference.json`). Probe
reverted; production carries only `EMIT_BATCH_LAYERS = 12` and its measured
rationale.

**Frontier now:** [144](144-brim-anchor-reconstruction.md) (AFK, unblocked) and
[t145](145-benchy-fixture-rebaseline.md) (AFK, unblocked, new).
