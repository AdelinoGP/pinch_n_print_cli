# Split and union the whole-layer clip-area dilation

Type: task
Label: wayfinder:task
Status: resolved (2026-10-30; measured **REJECTED on price** — the most
favourable split is **0.41–0.43 %** of the cell, about half the 0.80 % floor, and
not representation-safe on top (byte-identical 1/130, set-equal 2/130); no
candidate, no campaign, probe reverted. See ## Answer)
Assignee: 2026-10-30 OpenCode wayfinder session — the human said "continue the
perf vs orca map"; the frontier re-derivation at HEAD `6c1a2334` found **98/98
children resolved** (t98's resolution was still uncommitted and was committed
first as `1240999c`), the session offered six measured fog routes, and the human
selected **split-and-union the clip dilation**
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Windowed clip area in internal-bridge
qualification](96-windowed-clip-area-dilation.md) ("what the screen leaves open")

## Question

Can the whole-layer `offset(deep_infill_area, 1.5 × spacing)` call in
`compute_internal_bridge_layer`
(`crates/slicer-runtime/src/slice_postprocess_prepass.rs`) be made cheaper by
offsetting the **disjoint deep-infill polygons separately and unioning the
pieces** — dilation distributes over a union — and is the resulting form
representation-safe, and above the map's **0.80 %** floor?

The windowed operand is dead ([ticket 96](96-windowed-clip-area-dilation.md)):
its consumer `determine_bridging_angle` scans anchors **globally**, so no finite
candidate-driven window is exact. This ticket keeps the **whole** operand and
attacks only the offset's own cost, which is the last representation-safe shape
the screen left open.

## Measured premise

From [ticket 96](96-windowed-clip-area-dilation.md) and its predecessors
(ordinary base classic supports-off, the map's screening cell, 73.17 s):

| fact | value |
| --- | --- |
| production dilation, 130 calls | **11,672.7–11,813.6 ms** accumulated |
| share of the cell | **1.57–1.60 %** |
| floor | **0.80 %** = 585.4 ms |
| `deep_infill_area` shape | **50 polygons / 7,084 ring vertices** median; 318 / 13,388 max |
| median per-call cost | **91.3 ms** (p90 174.9 ms, max 264.8 ms) |
| consumers | qualification, per-candidate `area`, and the **global** anchor scan |

The union cost of the split shape was **not** measured by any prior take; the
ticket exists to price it.

## Phase 1 — screening (mandatory)

1. Env-gated probe on ordinary base classic supports-off: per call, time the
   production dilation and the counterfactual — **offset every operand polygon
   on its own, then union the pieces** — and record the operands' shape, the
   offsets-only span (the union-free **lower bound**), the explicit-union span,
   and a byte / set equality check of the split result against production.
2. Two runs with **opposite pricing orders** (production first vs split first),
   so a price that exists only in one order is read as an artifact. Both arms
   must emit the reference `292df501…` / 21,023,739 bytes.
3. Convert the measured saving to wall with the run's **own** cpu/wall ratio and
   compare against the floor. Report the union-free bound too: if even that is
   below the floor, no split shape in the family can clear it.
4. **Stop rule:** stop at Phase 1 if the union-free bound is below the floor, or
   if the split result is not byte-identical. A shape that is both cheaper and
   exact is the only Phase-2 candidate.

## Pre-declared floor

**0.80 % of the cell wall.** Ordinary base classic supports-off is **73.17 s**
(t87), so the floor is **585.4 ms**. Using t87's cell wall is the generous
reading: t95's keep has since lowered PnP's own cell, which only *shrinks* the
floor.

## Proof gate

- Phase 1 probe reverted; both guest freshness gates exit 0.
- Phase 2 only: 16-cell output-identity matrix against
  `t91-exact-z-memo/references.json`, plus the standing four-cell paired gate.
- Phase 2 only: unit tests for a fixture with two disjoint deep-infill clusters,
  comparing `qualified` and `angles` between the split and whole paths.

## Deliverables

1. The Phase 1 table: production span, offsets-only span, union span, split
   total, order spread, wall-equivalent share, floor verdict, exactness counts.
2. If (and only if) the shape is above the floor and byte-identical, the
   candidate plus its equivalence tests.
3. A measured recommendation to the human. No auto-commit.

## Answer

**Measured REJECTED on price — no candidate, no campaign, probe reverted
(2026-10-30, HEAD `1240999c`).** The split shape is genuinely cheaper per call
(**0.8230x / 0.8252x**, order-independent within **0.27 %**) but the call is too
small a share of the cell for a 17.5 % cut to clear the floor.

Phase 1, ordinary base classic supports-off, one host binary with env-gated arms,
130 calls per run:

| term | prod-first | split-first |
| --- | ---: | ---: |
| production dilation | 13,752.5 ms | 14,099.3 ms |
| per-polygon offsets only (union-free bound) | 11,078.4 ms | 11,376.7 ms |
| explicit union of the pieces | 239.8 ms | 257.8 ms |
| split arm total | 11,318.3 ms | 11,634.5 ms |
| **relative cut** | **0.8230x** | **0.8252x** |
| saving, wall-equivalent (run's own cpu/wall) | 314.3 ms | 298.1 ms |
| **share of 73.17 s** | **0.43 %** | **0.41 %** |

**The floor is 585.4 ms; the measured saving is 0.51–0.54x of it.** Even with the
explicit union free, the share is **0.47 %**, still below — so no split shape in
this family can clear the floor, because the per-polygon offsets are 80.6 % of
the production call. The probe's own production spans are inflated by the
counterfactual contending in the same 12-thread parallel phase (t96 read
11.67–11.81 s), so only the **within-run ratio** is a claim.

**Why the shape is cheaper, which is the reusable part.** Offsets-only is
**80.6 %** of production, so the batched `ClipperOffset` spends **~19 %** in its
internal union/cleanup, while an explicit union of the already-rounded pieces
costs only **~2 %**. Splitting does not skip work; it **partitions the cleanup**.
That is a real engine fact and it is why the shape looked promising — it is just
too small in absolute terms on this call.

**Exactness — independently fatal, and secondary.** The split arm is
byte-identical on **1 / 130** calls and set-equal on **2 / 130**, with **5,329**
differing polygons, identically in both orders. Dilation distributes over a
disjoint union in continuous geometry; Clipper2's integer Miter offset plus a
second union does not reproduce the single-pass result. The clip area feeds a
**global** nearest-anchor scan (t96), so any representation change is fatal even
where the set is unchanged. The differing *area* was not quantified — the price
verdict is independent of it.

**What this leaves for the map.** The split family is closed at a measured
ceiling of **0.41–0.43 %**. The only shape t96 named that is still standing is to
change the angle consumer (`determine_bridging_angle`) so the clip area need not
be whole — OrcaSlicer geometry-parity work, which the map holds **out of scope**
absent a human ruling. The cost decomposition (80 % per-path offset / 19 %
batched union-cleanup / 2 % explicit union) is recorded in the map's fog so a
future take does not re-derive it.

Disclosures: both runs are instrumented and their walls are attribution only; the
two probe walls are 27.0 / 17.9 s above the 73.17 s baseline and differ from each
other by 9.1 s for the same work; the tree carries no production change; both
guest freshness gates exit 0 before and after. Evidence:
[`evidence/t99-clip-split-union/`](../evidence/t99-clip-split-union/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 6/6 negative controls rejected).
