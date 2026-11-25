# Planner clip-batch fan-out: why is the clip core at 4.81x and what scheduling change reaches 12x?

Type: task / Phase-1 attribution
Label: wayfinder:task
Status: resolved (2026-11-12; Phase-1 attribution — **the carve batches are
  already at 11.56x; the union phase owns 64.2 % of the clip-core wall at 3.66x
  because it issues 4.4 requests per call and every call's wall is its own
  longest request.** Chunk size is the lever — merge-4 floor 3,465 ms against a
  10,176 ms union wall, a 6.7 s ceiling = 7.3 % of base-on's required cut — but
  through the phase's request count, not the saturated 256 cap. Guest assembly
  3.1 %, dispatch excess 0.9 %, load imbalance refuted. GO (narrowed) for a
  union-phase grouping candidate, graduated as
  [t143](143-union-phase-request-grouping.md). No production edit; see
  ## Answer)
Assignee: 2026-11-12 OpenCode wayfinder session — the human said "Continue the
  perf vs orca map"; the AFK frontier held this ticket (unblocked by t137's
  2026-11-12 ruling) beside the human's HITL brim ruling
  [140](140-brim-contour-input.md), so it was claimed.
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none ([137](137-planner-scheduling-gate.md) resolved 2026-11-12 —
  the standing four-cell gate form is confirmed)
Graduated from: [Planner effective parallelism](134-planner-effective-parallelism.md)'s
  2026-11-09 ruling: the clip core is **85,641 ms thread / 17,811 ms wall = 4.81x** on
  base classic-on, against the 12-thread ideal. Raising it to 12x is a derived **10.7 s**
  wall ceiling, independent of the occupancy tranche.

## Question

The planner's emit-pass clip core (`build_roles_batched` via `run_clip_batches`) fans
out **70,210 clip calls / 270.6 M input points** over **559 parallel batch calls**, yet
delivers only **4.81x** on base classic-on (**4.52x** on benchy classic-on), while the
shell stage reaches **9.26x** and overhang **11.72x** on the same host and run. Which
part of the 4.81x is batch-load imbalance, which is the per-batch serial slice, and
which is the guest-side assembly — and what dependency-preserving scheduling change
recovers the difference without changing the op set or operand size?

Candidate mechanisms to price, in order:

1. **Chunk size.** `EMIT_BATCH_LAYERS = 4` and `CLIP_BATCH_MAX_REQUESTS = 256` bound
   each batch. Measure the per-batch sweep distribution and the tail request share;
   larger or work-balanced chunks may raise the fan-out.
2. **Tail imbalance.** The probe's largest single planner clip sweep is **655 ms**
   and the largest single operand **169,716 points**, against a 3,854-point mean
   on the emit core. Measure how much of each batch's wall is its largest request;
   if the tail dominates, a representation-preserving request partition may help,
   but that is a work-set change and needs its own ruling.
3. **Guest assembly.** `build_roles_batched` does the role union, per-region carve,
   set-wide difference and territory intersect as separate `run_clip_batches` rounds.
   Measure the serial guest time between rounds; if it dominates, the round count is
   the lever, not the fan-out.

## Pre-declared gate

No production edit in this take. Any candidate it graduates must be byte-identical in
all eight cells against the active post-[t139](139-layer-plane-parity.md) reference set
(a scheduling change is output-neutral; if it is not, it is not this shape) and must
clear the standing [t137](137-planner-scheduling-gate.md) four-cell form in the four
supports-on cells with no median wall/CPU regression in the four supports-off cells. The
literal all-8-cell 0.80 % gate is structurally unreachable for a planner-only change.

## Acceptance

- A measured decomposition of the 4.81x into chunk size, tail share and guest assembly
  on base classic-on and benchy classic-on.
- A ranked list of dependency-preserving scheduling variants, each with its measured or
  derived ceiling and floor multiple; no production edit.
- A recorded go/no-go on a candidate build.

## Answer

**Resolved 2026-11-12: NO-GO on the ticket's three named mechanisms; GO
(narrowed) for one scheduling candidate. The clip core is not uniformly slow —
the carve batches are already at 11.56x, and the union phase is one request
deep.**

The clip core is **15,841.8 ms** on base classic-on (p2 15,938.2 / p3 15,370.0),
of which host clip calls are **13,956.4 ms**, marshal+decode **1,393.7 ms** and
pure guest CPU **491.7 ms**. The host wall decomposes additively into a
**6,587.6 ms** work floor (`thread/12`), **7,224.0 ms** of tail-times-chunking
and **144.8 ms** of dispatch excess; end-to-end fan-out is **4.99x**, which is
[t134](134-planner-effective-parallelism.md)'s 4.81x seen from a faster run.

**The phase contrast is the answer.** `carve_run` issues **65,233 requests over
312 calls** (p50 **256** per call — `CLIP_BATCH_MAX_REQUESTS` is saturated) and
fans out at **11.56x** (40,977.6 ms thread in 3,544.7 ms). `union_run` issues
**482 requests over 109 calls** (p50 **4.4** per call) and fans out at
**3.66x** (37,237.5 ms in 10,176.2 ms) while owning **64.2 %** of the clip-core
wall. Its summed per-call `max(thread/12, longest request)` is **10,168.3 ms**
against a **10,176.2 ms** wall: **8 ms** of the phase is anything but each
call's longest request. At 4.4 requests per call, 7.6 of 12 threads are idle by
construction — this is a **parallelism-width** deficit, not load imbalance.

Against the ticket's candidates, in order:

1. **Chunk size — right, for the wrong reason.** The 256-request cap is not
   binding where the work is (carve already runs at 256 requests/call and
   11.96x); the binding chunk is the *phase's* request count, which is 4.4
   because `union_run` is called once per `build_roles_batched` and four
   buffered layers hold only ~4.4 unions. Merging across chunk-batches takes
   the union floor to **5,480.2 ms** at 8 layers, **3,465.0 ms** at 16 layers
   and **3,103.1 ms** at a whole-phase batch. The 16-layer form recovers
   **6,711.2 ms = 94.9 %** of the 7,073 ms tail gap; LPT's 4/3 bound puts the
   achievable saving at **5.6–6.7 s**.
2. **Tail imbalance — true but not rebalanceable.** The tail *is* the wall, but
   with 4.4 independent requests there is nothing to rebalance; only more
   requests per call or shorter requests change it.
3. **Guest assembly — refuted.** 491.7 ms (3.1 %) of the clip core, of which
   `carve_build`'s request construction is 305.5 ms. Marshal+decode is the
   larger non-host term at 1,393.7 ms (8.8 %) and is an operand-locality issue,
   not a scheduling one.

**Ranked dependency-preserving variants (base classic-on; floor 1,010 ms;
required cut 92.5 s):**

| rank | variant | ceiling | floor multiple | share of cut | verdict |
| --- | --- | ---: | ---: | ---: | --- |
| 1 | Union-phase request grouping — `EMIT_BATCH_LAYERS` 4 → 16, or a union-only accumulator across four chunk-batches | **5.6–6.7 s** (floor 3,465 ms, LPT ≤ 4,620 ms) | **6.6x** | **7.3 %** | **GO (narrowed)**, exact by construction; [t143](143-union-phase-request-grouping.md) |
| 2 | Split the heavy union requests (longest 401.0 ms) | 7.1 s, the same tail | 7.0x | 7.6 % | work-set change; needs its own ruling |
| 3 | Carve chunking / 256 cap | ≤ 0.13 s (carve is 11.56x against 12.0x) | 0.13x | 0.14 % | no headroom |
| 4 | Dispatch / load balance | 0.14 s measured excess | 0.14x | 0.16 % | refuted |
| 5 | Guest assembly | 0.49 s | 0.49x | 0.53 % | refuted |
| 6 | Marshal / operand locality | 1.4 s | 1.4x | 1.5 % | below the tranche bar |

Benchy classic-on reproduces the shape at 17x smaller scale: clip core 910.2 ms,
carve 11.18x, union 3.71x with a 442.8 ms wall against a 148.8 ms merge-4 floor
(**294.0 ms** saved, 2.9 % of the 10.01 s cell).

**Gate.** The candidate is planner-only, so the supports-off cells are
structurally inert and the literal all-8-cell form stays unreachable; it is
judged under the standing [t137](137-planner-scheduling-gate.md) four-cell form
(positive 0.80 % paired wall/CPU in the four supports-on cells, byte-identical
output in all eight against the post-[t139](139-layer-plane-parity.md)
reference set, no supports-off regression). **Memory is the first check**:
`EMIT_BATCH_LAYERS` is a documented memory bound, and 4 → 16 quadruples the
buffered collision sets; the union-only accumulator is the lower-risk form.

**Route arithmetic:** 6.71 s is **7.3 %** of base classic-on's 92.5 s required
prepass cut, **5.3 %** of its 126.27 s cell and **17 %** of t134's 39.8 s
planner stage — a tranche of t135 Stage 2's order, not a destination step. The
route remains operand volume / geometry.

Evidence: [`evidence/t136-clip-fanout/`](../evidence/t136-clip-fanout/FINDINGS.md)
(155 checks; `--self-test` rejects 27/27 negative controls; all six probe
G-code hashes equal the post-t139 reference set). No production edit; probe reverted.
