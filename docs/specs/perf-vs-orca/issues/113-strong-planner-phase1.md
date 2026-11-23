# Strong support planner Phase 1: sub-term decomposition and a representation-safe shape

Type: task
Label: wayfinder:task
Status: resolved (2026-11-04; Phase-1 screen — **stop under the all-8-cell
contract**, no candidate: the occupancy-offset batch is representation-safe and
clears the floor on all four supports-on cells, but the supports-off
whole-stage ceiling is 0.22–0.51 % of the cell, below 0.80 %. See ## Answer)
Assignee: 2026-11-03 OpenCode wayfinder session (ticket picked as first frontier
child at HEAD `19a5edfe`; the human said "continue the perf vs orca map")
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 112
Graduated from: [ticket 111](111-prepass-rewrite-scope-ruling.md)'s ruling
(Q3, 2026-11-03: the Strong planner is the second campaign, after
`PrePass::Slice`).

## Question

Where does the **87,824 ms** `com.core.tree-support-planner` module stage go on
base classic-on (**2,943 ms** on benchy classic-on), and is any of it a
representation-safe shape that clears the **0.80 %** floor on all 8 cells?

This is the second campaign under ticket 111's scoped parity exception. The
planner may still turn out representation-safe, in which case it needs no
exception; if it needs canonical parity, the exception already covers it.

## Context

- [Ticket 107](107-support-geometry-producer-decomposition.md): the
  `PrePass::SupportGeometry` stage id is shared by a **9 ms** host producer and
  the planner module. The planner is **95.6 %** of the **91,893 ms** stage and
  **38.4 %** of the base classic-on cell; the producer is priced dead at
  **0.0068x** the floor.
- [Ticket 88](88-supports-on-increment-attribution.md): the supports-on increment
  is **~69 % support geometry / ~31 % host**; the planner is **52 %** of the
  classic increment, and its **55.5 s** attribution refreshes to **64.5 s**
  implied under t104's share arithmetic.
- [Ticket 78](78-tree-support-planner-emit-batching.md) took the batched clip core
  from **79.7 s** to **~30.7 s** (2.6x effective). The residual named by t88 is
  the **serial avoidance ladder, offsets, MST and node moves** — support geometry,
  not a host batch shape. No representation-safe shape was found in t107's take;
  re-enter only with a measured sub-term and a representation argument.
- [Ticket 111](111-prepass-rewrite-scope-ruling.md): candidate acceptance is the
  fresh **16-cell** reference set, arms byte-identical within themselves, the
  candidate-vs-old-reference delta audited before timing, and the standing paired
  wall/CPU gate at the 0.80 % floor in **all 8 cells**.

## Scope

Phase-1 decomposition and shape search only: no production edit, no candidate, no
timing campaign, no reference-set generation.

1. Re-derive the planner's current sub-term split at HEAD from the existing
   profile and evidence: the clip core, the avoidance ladder, offsets, MST, node
   moves, marshalling, and any remaining host work. Separate module time from
   host time.
2. Identify the largest sub-terms and test each against a representation-safety
   argument: does the shape change support geometry, or only how the same geometry
   is computed? A parity shape is allowed under t111, but it must still be named
   and its output delta bounded.
3. For each viable shape, pre-declare the expected wall ceiling on the gated
   cells and the output delta; recommend **build** or **stop**.

## Deliverable

A resolution with the fresh sub-term split, the candidate shapes (representation
argument, ceiling, cells moved, risk), and a recommendation. If stop, the ceiling
against the 0.80 % floor on all 8 cells and which cells fail.

## Stop rule

If no shape clears **0.80 %** on **all 8 cells**, or the output delta cannot be
bounded, record the ceiling and stop: no candidate ticket, no production edit.
The planner's share of the supports-on increment is large enough that a
below-floor ceiling is a real result, not a failure to try.

## Acceptance

The analysis is a resolution with the evidence attached; no production edit and no
timing campaign. If a shape is recommended, the next ticket is a candidate
implementation under ticket 111's acceptance contract.

## Evidence starts

- [`evidence/t107-support-geometry-producer/`](../evidence/t107-support-geometry-producer/FINDINGS.md)
  (stage/producer split and the producer's ceiling).
- [`evidence/t88-supports-increment/`](../evidence/t88-supports-increment/FINDINGS.md)
  (the 55.5 s planner attribution and the sub-term shape).
- [`evidence/t78-planner-emit-batching/`](../evidence/t78-planner-emit-batching/FINDINGS.md)
  (the clip-core batching and its measured limit).
- [`evidence/t104-phase-split/phase-split.json`](../evidence/t104-phase-split/phase-split.json)
  (the `PrePass::SupportGeometry` stage wall and share).
- `modules/core-modules/tree-support-planner/src/lib.rs`
  (`com.core.tree-support-planner`).

## Answer

**Verdict: stop under ticket 111's all-8-cell contract. No candidate ticket and
no production edit. The planner's largest term is `inflate_model_occupancy`, a
representation-safe host offset batch shape that clears the 0.80 % floor on all
four supports-on cells; but the whole support-geometry stage on the
supports-off cells is below the floor even if deleted, so no planner-only (or
support-stage) shape can satisfy the all-8 clause. The carry decision is
graduated as [ticket 117](117-strong-planner-carry-ruling.md).**

Evidence:
[`evidence/t113-strong-planner-phase1/`](../evidence/t113-strong-planner-phase1/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 6/6 negative controls rejected).

### 1. What the 87,824 ms is made of

The 87,824 ms figure is t104's **instrumented** tree-planner bracket at
2026-11-01, not a wall. The fresh HEAD `19a5edfe` probe measures the module at
**58,498 ms** on a **200,099 ms** cell (29.23 %; the instrumented cell is
+24.4 % over t103's 160.857 s quiet median). Applying the HEAD composition to
the t104 bracket gives ≈**49.3 s** occupancy inflation, ≈**27.2 s** clip core,
≈**5.5 s** drop loop, ≈**2.1 s** contacts, ≈**1.8 s** node roles, ≈**0.6 s**
node ellipses, ≈**0.3 s** volumes, ≈**0.2 s** smoothing, ≈**0.03 s** stamp,
≈**0.8 s** residual. The composition, not the absolute, is the transferable
result.

| sub-term | base classic-on | benchy classic-on | base arachne-on | benchy arachne-on |
| --- | ---: | ---: | ---: | ---: |
| `inflate_model_occupancy` | **56.2 %** | **46.0 %** | **51.4 %** | **41.4 %** |
| clip core (`build_roles_batched`) | 31.0 % | 28.6 % | 35.9 % | 32.4 % |
| drop loop (ladders + MST + moves) | 6.2 % | 10.8 % | 6.0 % | 11.1 % |
| contacts | 2.4 % | 6.0 % | 2.2 % | 5.4 % |
| node roles + ellipses | 2.7 % | 5.0 % | 2.9 % | 5.9 % |
| volumes + smoothing + stamp | 0.6 % | 1.3 % | 0.6 % | 1.3 % |

Shares are of the planner's own probe total; the residual (0.9–2.5 %) is emit
scaffolding and the drain. The clip core is already batched: role union is its
largest phase (64.5 % of the core on base), the carve second, and the other
differences are noise.

### 2. The shape: batch the occupancy offsets

`inflate_model_occupancy` (`modules/core-modules/tree-support-planner/src/lib.rs`)
is a pure `host::offset_polygons` at `support_object_xy_distance`, Miter join,
zero arc tolerance. It is called once per emitted layer and once per emitted
origin; the two halves are almost exactly equal (**16.40 s + 16.37 s** on base
classic-on; **0.751 s + 0.743 s** on benchy classic-on). Buffering the raw
polygon inputs and issuing `offset_polygons_batch` per chunk runs the **same**
`slicer_core::polygon_ops::offset` per request, input-order results (t65/t78
precedent). It changes how the same geometry is computed, not the geometry:
**representation-safe, no canonical-parity exception, zero expected output
delta**. The probe was byte-identical to the uninstrumented HEAD baseline on
base classic-on (`4b6d2843…`), benchy classic-on (`3b8fa86a…`) and benchy
arachne-on (`d4644c14…`).

Ceiling on the four supports-on cells (same-run floor; "2.6x" is t78's measured
effective batch clamp, "12x" the ideal fan-out):

| cell | `p6d` ÷ floor | 2.6x ceiling ÷ floor | 12x ceiling ÷ floor |
| --- | ---: | ---: | ---: |
| benchy classic-on | 12.09x | **7.44x** | 11.08x |
| benchy arachne-on | 7.74x | **4.76x** | 7.09x |
| base classic-on | 20.43x | **12.58x** | 18.73x |
| base arachne-on | 12.95x | **7.97x** | 11.87x |

The per-origin half alone sits inside the existing `EMIT_BATCH_LAYERS = 4`
buffer and would still leave **≈2.4x–6.3x** the floor at the 2.6x clamp, so the
shape can be staged.

### 3. Why the all-8 gate is structural

The planner is a supports-only stage. On the supports-off cells the whole
`PrePass::SupportGeometry` stage is the ceiling, not the planner:

| cell | whole stage | stage share | stage ÷ floor | planner module | planner ÷ floor |
| --- | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 46 ms | 0.51 % | 0.63x | 23 ms | 0.32x |
| base classic-off | 300 ms | 0.22 % | 0.28x | 141 ms | 0.13x |

Even deleting the entire stage cannot reach 0.80 % on those cells. The all-8
clause in [ticket 111](111-prepass-rewrite-scope-ruling.md) is therefore
incompatible with any planner-scoped or support-stage candidate; it can only be
satisfied if another leg (ticket 116's `PrePass::Slice` P1, present in all eight
cells) carries the supports-off cells.

### 4. Ledger corrections

- **t88/t107's "no host batch shape in the non-clip residual" is falsified.**
  The avoidance ladder, offsets, MST and node moves together are **6.2 % /
  10.8 %** of the planner; the largest residual is the **host offset batch** at
  **56.2 % / 46.0 %**. The earlier takes measured the clip core and the ladders,
  not the occupancy inflation.
- **The supports-off planner is below the floor.** t107's **227 / 43 ms** bracket
  is refreshed at HEAD as **141 / 23 ms** for the tree planner; the whole stage
  is **300 / 46 ms**. No support-stage shape can clear 0.80 % there.
- The `base arachne-on` probe wall (416.4 s) and the `benchy arachne-on` wall
  (27.1 s) are ≈2.1–2.2x their older quiet cells; their shares and floor
  multiples are usable, their absolute walls are not wall claims.

### What this does not authorize

- No occupancy-batch implementation, no `offset_polygons_batch` edit, no
  candidate ticket. The carry route is [ticket 117](117-strong-planner-carry-ruling.md)'s
  decision.
- No claim that the planner got faster at HEAD: the t104 bracket and the HEAD
  probe carry different cell inflation and the merge moved the surrounding
  pipeline; only the composition is asserted.
- No Arachne-off capture and no wall claim from any instrumented pass.
- No change to the standing paired acceptance or KEEP/DROP rules.
