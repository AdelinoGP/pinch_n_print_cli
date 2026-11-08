# Prepass rewrite: scope, output contract and acceptance

Type: grilling
Label: wayfinder:grilling
Status: resolved (2026-11-03; **authorized** — a scoped canonical-parity
exception for the prepass, Slice-first, with a fresh 16-cell reference set, an
all-8-cell paired gate, and a Phase-1 parity analysis before any production
edit; see ## Answer)
Assignee: 2026-11-03 OpenCode wayfinder session — the human said "continue the
  perf vs orca map" (no ticket named), the frontier re-derivation at HEAD
  `f4a73892` found **110/110 children resolved — no open child, no claim**, and
  the human selected **rule on the prepass rewrite** from the fog.
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: the fog entry **"The destination is a prepass-rewrite decision,
  and its two carrying shapes are human rulings first (t104, 2026-11-02)"**,
  reframed by [ticket 110](110-orca-side-attribution.md) (2026-11-03).

## Question

The destination is **8 of 8 cells won** on matched settings, and the park still
needs **57–82 %** of each cell's wall removed (t103/t106). [ticket
104](104-prepass-vs-per-layer-attribution.md) measured that the prepass is
**68.5–89.5 %** of every classic cell, and named the two carrying shapes:

- **`PrePass::Slice`** — **41,472 ms (51.2 %)** of base classic-off and
  **44,558 ms (19.5 %)** of base classic-on. [ticket
  59](59-prep-slice-flat-bridge-closing-reduction.md) closed the
  representation-safe family: the closing result is consumed only as the
  per-component **≥ 10 %** overlap gate inside `assemble_flat_bridge_areas`
  (`crates/slicer-core/src/algos/prepass_slice.rs`), and any change to the input
  path list, call sequence or offset configuration moved the artifact field and
  flipped gate verdicts on **92/239** benchy and **320/469** base layers.
- **The Strong support planner** — **87,824 ms (38.4 %)** of base classic-on and
  **2,943 ms (23.6 %)** of benchy classic-on, inside the
  `PrePass::SupportGeometry` stage that shares its id; the host producer is
  **9 ms / 0 ms** (t107). [ticket 88](88-supports-on-increment-attribution.md)
  measured the supports-on increment as **~69 % support geometry / ~31 % host**,
  with the planner at **52 %** of the increment; [ticket
  107](107-support-geometry-producer-decomposition.md) found no
  representation-safe producer shape.

The ruling has to settle, in order:

1. **Output contract** — must a prepass rewrite preserve PnP's current
   representation byte-for-byte (the t59 dead end), or may it change output under
   a canonical-parity exception, with a fresh reference set and the map's
   disclosure rules?
2. **Scope** — which shape(s) the exception covers: `PrePass::Slice`'s closing
   consumer, the Strong planner (matched-engine work after the t105 pin), both,
   or neither.
3. **Acceptance** — the gate an output-changing candidate faces, given that the
   standing paired proof requires byte-identical arms.
4. **Staging** — whether the rewrite is stage 1 of a larger program. Even
   deleting **both** largest stages leaves, inside t104's instrumented
   arithmetic against Orca's quiet walls (an upper bound on the improvement),
   **39,452 ms / 13,814.5 ms = 2.86x** on base classic-off and **96,615 ms /
   24,140.3 ms = 4.00x** on base classic-on — so a prepass-only rewrite cannot
   reach the destination on its own.

## Facts the ruling must weigh

- **The map's out-of-scope boundary.** OrcaSlicer geometry-parity work is out of
  scope; the only recorded exception **before this ticket** is [ticket
  81](81-resolution-slicing-time-contour-simplification.md)'s `resolution` port,
  and "the next such question goes back to the human". This ticket *is* that
  question for the prepass.
- **The engine gap is larger than the ratio of record.** [ticket
  110](110-orca-side-attribution.md) measured Orca's whole slicing at **0.64 /
  1.28 / 3.68 / 14.01 s** against PnP's instrumented **7.37 / 11.78 / 80.22 /
  228.32 s** (**11.6x / 9.2x / 21.8x / 16.3x**), with the prepass carrying both
  engines. The base-on row is an **instrumented** PnP span: t104's own
  disclosure says that pass ran **+43.6 %** over its t103 median and instrumented
  walls are never wall claims, while the uninstrumented paired record is
  **160.857 s** (t106) -> **6.02x**, not 9.49x. Treat t110's base-on multiples as
  upper bounds until a ledger correction is taken; the qualitative conclusion
  (the engine gap exceeds the wall ratio) survives.
- **The feature stack is nearly free on Orca.** A slice with 1 wall loop, 1
  shell, 0 % infill and no support still costs **13,639.6 ms** against the full
  base classic-off cell's **13,814.5 ms** (t110), so Orca's advantage is not
  "it does less prepass work".
- **The destination is not reachable by deleting the per-layer phase either**
  (t104: **3.02x / 4.51x / 4.66x / 7.60x** remain), and the per-layer + emit
  phases are **4.4x–7.3x** behind on Orca's side (t110, again an instrumented
  PnP side).
- **A rewrite that changes output invalidates the standing paired proof.** The
  map's KEEP/DROP rules require both arms byte-identical and the output equal to
  the post-t81 reference set; a parity-changing candidate needs a new reference
  set and a disclosed job comparison instead.

## Decision tree

Settled by the grilling rounds on 2026-11-03; no branch remains open.

- Q1 output contract: **scoped canonical-parity exception** for the prepass; the
  rest of the OrcaSlicer parity program stays out of scope.
- Q2 staging: **stage 1 accepted**. The 8/8 destination stays; the per-layer/emit
  program is the next stage's fog.
- Q3 shape scope: **both** carrying shapes are covered; **`PrePass::Slice`
  first**, the Strong planner second.
- Q4 acceptance: a fresh **16-cell** reference set; candidate and baseline arms
  byte-identical within themselves; the baseline reproduces the post-t81 set; the
  candidate-vs-old-reference delta is audited before timing; the standing paired
  wall gate with CPU corroboration and the **0.80 %** floor governs.
- Q5 gate cells: **all 8 cells**, strict paired wall, CPU corroborating.
- Q6 first step: **Phase-1 parity analysis**; no production edit until it shows a
  path above the floor with a bounded output delta.

## Answer

**Verdict: authorized — a scoped canonical-parity exception for the prepass,
`PrePass::Slice` first, gated on all 8 cells with a fresh 16-cell reference set,
and no production edit until a Phase-1 parity analysis clears the 0.80 % floor
with a bounded output delta.**

### The ruling

1. **Output contract.** PnP's output may change **only where a canonical prepass
   shape requires it**. The exception is scoped to the prepass; the rest of the
   OrcaSlicer parity program stays out of scope, and the next parity question
   still goes back to the human. This is the second recorded exception, after
   [ticket 81](81-resolution-slicing-time-contour-simplification.md)'s
   `resolution` port, and it is not a standing licence.
2. **Scope and order.** Both carrying shapes are covered, but the first campaign
   is **`PrePass::Slice`** (**41,472 ms = 51.2 %** of base classic-off, **44,558 ms
   = 19.5 %** of base classic-on, present in all 8 cells). The **Strong support
   planner** (**87,824 ms = 38.4 %** of base classic-on, **2,943 ms = 23.6 %** of
   benchy classic-on) is the **second** campaign, graduated as
   [ticket 113](113-strong-planner-phase1.md) and blocked by ticket 112; it may
   still turn out representation-safe, in which case it needs no exception.
3. **Acceptance.** A fresh **16-cell reference set** (8 cells x ordinary /
   accelerated). The candidate's ordinary and accelerated arms must be
   byte-identical **within themselves**; the baseline arm must be byte-identical
   within itself and reproduce the post-t81 reference set; the
   candidate-vs-old-reference G-code delta is audited (feature / TYPE /
   support-interface counts, byte volume, and confinement to prepass-derived
   geometry) **before** timing. The standing paired wall gate with CPU
   corroboration and the map's **0.80 %** floor still governs, and the gate is
   **all 8 cells** — no four-cell subset. A candidate that passes on one cell and
   regresses an unmeasured one is not a KEEP.
4. **First step.** A **Phase-1 parity and consumer analysis**, not a candidate. It
   must pin what canonical OrcaSlicer's closing pipeline does differently from
   PnP's `assemble_flat_bridge_areas`
   (`crates/slicer-core/src/algos/prepass_slice.rs`), map the difference onto the
   per-component **>= 10 %** overlap gate and its consumers, design the parity
   shape, and pre-declare its expected wall ceiling and output delta. It stops
   with no production edit if the ceiling is below the floor on the gated cells
   or the delta cannot be bounded.
5. **Staging.** The prepass rewrite is **stage 1** of a multi-stage engine
   program; the **8/8 destination stays**. Even deleting both largest prepass
   stages leaves base classic-off at **2.86x** and base classic-on at **4.00x**
   inside t104's instrumented arithmetic against Orca's quiet walls (an upper
   bound on the improvement), so the per-layer / emit program is the **next
   stage's fog**, not part of this ruling.

### Ledger caveat carried forward

t110's base-on figures — **9.49x** end-to-end, **16.3x** slicing, **19.6x**
prepass, **13.7x** support-stage increment — divide an **instrumented** PnP span
by Orca's quiet process wall. t104's own disclosure puts that instrumented pass
at **+43.6 %** over its t103 median and says instrumented walls are never wall
claims; the uninstrumented paired record is **160.857 s** (t106) -> **6.02x**.
The ruling therefore treats those base-on multiples as **upper bounds**. The
qualitative conclusion (the engine gap exceeds the wall ratio) survives; the
correction is recorded in **Not yet specified** rather than silently folded into
this ticket.

### What this does not authorize

- No production edit before the Phase-1 analysis (ticket 112) completes.
- No parity work outside the prepass; no new reference set for unrelated jobs.
- No relaxation of the t105 style pin, the t103 matched keys, or the paired
  ordinary + accelerated protocol.
- No claim that the destination closes on the prepass rewrite alone.

### Next

The Phase-1 analysis is graduated as
[ticket 112](112-prep-slice-parity-phase1.md), open and unclaimed. The planner
campaign is graduated as [ticket 113](113-strong-planner-phase1.md), open and
blocked by 112. The t110 base-on correction stays a fog note until the human
picks it.
