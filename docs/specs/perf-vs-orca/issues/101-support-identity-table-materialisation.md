# Support-attribution side table materialisation

Type: task
Label: wayfinder:task
Status: resolved (2026-10-31; Phase-1 screen — **both exact families are below the
0.80 % floor**: combined allocation+free ceiling **821.6 ms uncontended / 926.8 ms
most generous = 0.50–0.56 % of the cell = 0.62–0.70x the 1,318 ms floor**, and
t100's 3.468 s ceiling was a system-heap artifact that t102's mimalloc KEEP cut
**7.1–8.2x**. No candidate, no campaign, no production edit; recommended family 3
(keep). See ## Answer)
Assignee: 2026-10-31 OpenCode wayfinder session — the human said "continue the
perf vs orca map" (no ticket named), so the frontier's only open ticket was taken
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Allocation volume of the whole-print layer
collection](100-allocation-volume-layer-collection.md) ("What the screen leaves")

## Question

> **Premise superseded (2026-10-31):** the 3.53 s below was the **system heap**.
> [ticket 102](102-global-allocator-substitution.md) shipped mimalloc between this
> ticket being written and being taken, and the same free now measures
> **432.4–494.1 ms** (t101's own arms). Read the premise as history; the ## Answer
> re-prices everything on the shipped allocator.

The collection's whole-print free is **34,796,481 blocks / 1.358 GB / 3.53 s**
on ordinary base classic supports-on, and **99.6 %** of those blocks are
8–15-byte strings sampled as `__skirt_…` support family/body ids and `demand-N`
ids — the `support_entity_identities` side table that
`assemble_ordered_entities_with_support_identities`
(`crates/slicer-runtime/src/layer_executor.rs`) clones **per support path**.
Geometry is **0.094 %** of the blocks and **7.8 %** of the bytes
([ticket 100](100-allocation-volume-layer-collection.md)).

Which family removes that cost without losing the attribution contract, and what
does each cost?

1. **Share** one identity per body (rows reference a per-body record instead of
   cloning the demand list per path) — content-preserving by construction.
2. **Prune or gate** the table (the t20 `consume-only-when-read` pattern): it has
   no production reader today, so it need not be materialised at all unless a
   consumer asks. This changes what exists, so it needs the human's ruling and a
   decision on the two integration tests that pin it.
3. **Keep** it and accept the cost.

## Measured premise

From [ticket 100](../evidence/t100-allocation-arena/FINDINGS.md), ordinary base
classic supports-on (cell **164.803 s**, floor **1,318 ms**):

| fact | value |
| --- | ---: |
| blocks freed / bytes freed | **34,796,481** / **1,358,397,544** |
| drop, sampling-free | **3,526,544 µs** (101.3 ns per block) |
| 8–15-byte blocks | **34,662,668 = 99.6 %**, sampled `__skirt_…` / `demand-N` |
| `support_entity_identities` rows | **23,751** |
| demand-id strings (derived) | **34,615,166** ≈ 1,457 per row |
| free-side ceiling of removing the table | **3.468 s = 2.10 % = 2.63x the floor** |
| allocation-side cost of building it | **unmeasured** (t100 armed only the drop) |
| allocator swap (mimalloc) price | 3.527 → 1.302 s measured; a packager decision, not this ticket |

Consumers, re-derived at t100: written by
`assemble_ordered_entities_with_support_identities`, read only by
`crates/slicer-runtime/tests/integration/structured_support_identity.rs` and
`raft_geometry.rs`; set empty across the module marshal
(`crates/slicer-macros/src/lib.rs`); absent from `crates/slicer-schema/wit/`.

## Phase 1 — screening (mandatory)

1. **Allocation-side census**: arm the t100 counting allocator around the
   assembly (and the aggregation that fills `demand_ids`) so the 34.66M string
   *allocations* are priced, not just their free.
2. **Table shape**: Σ `demand_ids`, distinct demand-id strings, per-row length
   distribution, and the sharing factor (how many rows carry the same body's
   list) — family 1's whole value depends on it.
3. **Consumer audit**: every read of `support_entity_identities` in production
   and tests, the attribution contract each pins, and whether any view or
   dispatch path can see the table.
4. **Price each family** against the floor and state its exactness: family 1
   must yield the same rows and ids; family 2 must record what stops existing
   and which tests retire or move.
5. **Stop rule**: stop at Phase 1 if the combined (alloc + free) ceiling of the
   cheapest exact family is below the floor.

## Pre-declared floor

**0.80 % of the cell wall.** Ordinary base classic supports-on is **164.803 s**
(t91's kept ordinary arm) → **1,318 ms**. The free side alone is **2.63x** it;
the allocation side is unmeasured and additive.

## Proof gate

- Phase 1 probe reverted; both guest freshness gates exit 0.
- Phase 2 only: 16-cell output identity against
  `evidence/t91-exact-z-memo/references.json`, the standing four-cell paired
  gate (ordinary/accelerated x classic/arachne, base supports-on), and an
  equivalence test that the shared/pruned table yields the same rows and ids the
  per-path clone yields — or, for family 2, the recorded contract decision.

## Deliverables

1. The allocation-side census, the table-shape numbers and the consumer audit.
2. A priced recommendation per family, with the contract consequence of each.
3. The candidate plus its tests only for the family the human rules in. No
   auto-commit.

## Answer

**Resolved 2026-10-31 as a Phase-1 screen: both exact families are below the
pre-declared floor, so the ticket's own stop rule fires. No candidate, no
campaign, no production edit. Recommended family: 3 (keep).**

The headline is a **ledger correction to [ticket 100](100-allocation-volume-layer-collection.md)**,
not a new lever. t100 priced the table's free side at **3.468 s (2.63x the
floor)** on the **system heap**. [ticket 102](102-global-allocator-substitution.md)
then shipped mimalloc, and the same free now costs **432–494 ms — 7.1–8.2x
less**. The cause-level route this ticket was graduated to chase had already
been banked by the allocator swap.

### 1. The shape — one body's list cloned once per path

One deterministic census (identical on all three table arms) on ordinary base
classic supports-on:

| quantity | value |
| --- | ---: |
| layers / entities / `support_entity_identities` rows | 495 / 30,076 / **23,751** |
| Σ `demand_ids` across rows | **34,662,046** |
| demand-id payload bytes | 415,444,046 |
| per-row list length (min / p50 / p90 / max) | 6 / **1,322** / 2,821 / 2,847 |
| distinct `(family_id, body_id)` pairs | **431** |
| rows per body (p50 / max) | 50 / 129 → **55.1 mean** |
| bodies whose rows carry differing lists | **0** |
| distinct demand-list *contents* | **418** |
| ids retained if one list is shared per body | 428,726 = **1.24 %** |
| blocks present / retained | 34,733,299 / 429,980 = **1.24 %** |
| heap present / retained | 1,248,605,209 B / 15,471,852 B = **1.24 %** |

431 bodies, 55 paths each, every row carrying the body's whole ~1,459-id list.
`mixed_bodies = 0` proves every row of a body carries a byte-identical list, so
sharing one identity per body is **content-exact by construction** and would
keep 1.24 % of the blocks. Two facts to carry forward: 431 bodies carry only 418
distinct contents (the list is closer to a per-family set than a per-path
attribution), and the table is **1.2486 GB of live heap** — 92 % of the
collection's bytes and ~24 % of the process's ~5.3 GB peak working set (t79).
Memory is out of scope for this map (DEV-026), so it is recorded, not priced.

### 2. The two sides and the ceiling

The **allocation side is measured inside the product**, not derived: the probe
wraps `assemble_ordered_entities_with_support_identities`
(`crates/slicer-runtime/src/layer_executor.rs`) in a per-layer `Instant` window,
and the skip arm runs the same function and the same geometry work.

| arm | Σ assembly over 495 layers | whole-print drop | blocks freed |
| --- | ---: | ---: | ---: |
| table | 413.3 / 433.8 / 452.7 ms | 432.4 / 494.1 / 434.4 ms | 34,796,481 |
| skip | 21.1 / 17.3 ms | 2.614 / 2.890 ms | 62,751 |

Combined ceiling **821.6 ms uncontended / 926.8 ms most generous** =
**0.50–0.56 % of the cell = 0.62–0.70x the 1,318 ms floor**, charging 100 % of
the parallel assembly time to the critical path. Both readings are below the
floor.

The **wall is not a claim**: five runs on one binary spread **150.195–154.793 s
(3.0 %)**, 5.0x the whole instrumented ceiling, so the wall cannot resolve a
0.8 s effect on this machine. The census counters are free at this scale — the
censused drop (434.4 ms) sits between the two sampling-free drops — the opposite
of t100's system-heap measurement, where sampling cost a measured 506 ms.

### 3. Family verdicts

| family | exact? | ceiling (derived from the measured total) | contract consequence |
| --- | --- | ---: | --- |
| 1. share one identity per body | yes (`mixed_bodies = 0`) | **~812–916 ms** (98.76 %) | changes `SupportEntityIdentity`'s public shape, re-points two integration tests |
| 2. prune / gate the table | no | **822–927 ms** (100 %) | deletes or gates a `pub` IR field with no production reader; retires or moves `structured_support_identity` and `raft_geometry`; needs a human ruling |
| 3. keep | — | 0 | none |

Family 1 is the cheapest exact family and lands at **0.62x** the floor. Family
2's extra ~82 ms does not change the verdict, and its only other benefit is heap
this map rules out of scope.

### 4. Consumer audit, re-derived

Written only by `assemble_ordered_entities_with_support_identities`
(`crates/slicer-runtime/src/layer_executor.rs`), one row per support path.
**Read by no production path**: the only reads in the tree are
`crates/slicer-runtime/tests/integration/structured_support_identity.rs`
(assembly retention plus `apply_entity_order_proposal` preserving the
`entity_id` link) and `crates/slicer-runtime/tests/integration/raft_geometry.rs`
(which stores the rows in a fixture and never asserts on them). Every other
producer sets it empty (`crates/slicer-macros/src/lib.rs`,
`crates/slicer-runtime/src/anchored_rows.rs`), and it is absent from
`crates/slicer-schema/wit/`, so no wire contract depends on it. The empirical
form of that claim: **five runs with the table absent, all byte-identical to the
post-t81 reference** (`22200671167a97a7…` / 42,507,912 bytes),
`degraded=false` in every run.

### 5. Disclosures

- **The machine was not quiet.** The owner's browser used 15–56 % of 12 logical
  cores across the session and the wall drifted upward (151.3 s → 154.6 s on the
  same binary). Hence the instrumented ceiling and the absent wall claim.
- The table arm's drops still spread 432–494 ms (14 %); the ceiling uses the
  `min`/`max` envelope, over-charging the lever rather than the reverse.
- The allocation side is **summed across layers**, not critical-path; layers run
  in parallel, so the true wall contribution is smaller.
- Only the ordinary classic supports-on cell was screened; arachne-on shares the
  shape (t94) but is not re-measured.
- The 1.2486 GB heap figure is derived from the shape and agrees with the
  census's freed bytes minus the skip arm's (1,358,397,544 − 107,020,175 =
  1,251,377,369) to 0.2 %.
- Probe reverted; both guest freshness gates exit 0 at close. `verify.py
  --self-test` exits 0 checkout-only: **12/12 claims, 15/15 negative controls
  rejected**.

Evidence: [`evidence/t101-side-table/`](../evidence/t101-side-table/FINDINGS.md)
(`probe.patch`, `summary.json`, `extract.py`, `verify.py`, `REPRODUCE.md`).
