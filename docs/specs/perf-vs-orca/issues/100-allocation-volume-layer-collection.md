# Allocation volume of the whole-print layer collection

Type: task
Label: wayfinder:task
Status: resolved (2026-10-30; Phase-1 screen — the arena / bulk-free family
**as scoped is measured-dead**: construction is only reachable at creation, and
at creation it would address **0.094 %** of the blocks, because the storm is the
per-path support-attribution side table, not geometry. The allocator arm clears
the floor but is a packager decision. No candidate, no campaign, probe reverted;
the census graduates [ticket 101](101-support-identity-table-materialisation.md).
See ## Answer)
Assignee: 2026-10-30 OpenCode wayfinder session — the human said "continue the
perf vs orca map" (no ticket named); the frontier re-derivation found **99/99
children resolved**, the session offered six measured fog routes, and the human
selected **allocation-volume arena / bulk-free**
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Remaining whole-print collection drop inside G-code
emission](94-remaining-whole-print-drop.md) ("What this leaves")

## Question

Can the **3.24–3.56 s** serial whole-print free inside `host:gcode_emit` — the
drop of the emitted `Vec<LayerCollectionIR>` — be cut below the map's **0.80 %**
floor by changing the collection's **allocation shape** (a bulk-free arena:
one allocation per print, or one per layer), rather than its **drop strategy**?

The drop strategy is closed. [Ticket 94](94-remaining-whole-print-drop.md)
measured all three of its shapes at or below the floor: layer-by-layer
consumption is a structural zero (the walk is serial, 176–253 ms, and nothing
else runs), a parallel free is a wash (classic −0.321 s, arachne +0.200 s on
Windows' process heap), and deferring the free into the post-emission tail has
an absolute ceiling of that tail (**1.37–1.60 s**, 1.02x–1.22x the floor) before
any overlap loss. What t94 left is the diagnosis: the free is
**allocation-volume** bound (~5.3 GB peak working set, t79), so the lever is
**fewer/larger allocations**.

The fog entry that graduated this ticket set two entry conditions, and both are
this ticket's Phase 1:

1. **A measured allocation count and bytes per layer.** Nothing in the map has
   censused the collection; the split says 93–94 % of the bracket is the free,
   but not whether that cost is **per block** or **per byte**.
2. **A representation-safe arena design, priced against its own construction
   cost.** A per-print arena built by copying the collection would still free
   every original block (the same storm) on top of the copy, so only
   arena-at-creation avoids it. The screen must price both sides before any
   candidate is proposed.

## Measured premise

Ordinary base supports-on, the map's screening cells for the postpass free
([t94](../evidence/t94-remaining-drop/FINDINGS.md), [t79](../evidence/t79-emit-batch-sweep/FINDINGS.md),
[t91](../evidence/t91-exact-z-memo/FINDINGS.md)):

| fact | classic-on | arachne-on |
| --- | ---: | ---: |
| `host:gcode_emit` bracket | 3,811 ms | 3,438 ms |
| of which the collection drop | **3,558 ms** | **3,242 ms** |
| emission walk | 252.8 ms | 195.3 ms |
| cell wall (t91 kept ordinary arm) | **164.803 s** | **166.931 s** |
| **0.80 % floor** | **1,318 ms** | **1,335 ms** |
| drop as a share of the cell | 2.16 % | 1.94 % |

Peak working set on this cell class is **~5.3 GB** (t79). The drop is **above
the floor** — 2.7x / 2.4x it — so the only open question is whether a
bulk-free shape actually recovers it.

The collection's own allocation census and the free cost of a bulk free at the
same byte volume are **unmeasured**; this ticket exists to take them.

## Phase 1 — screening (mandatory)

1. Env-gated **allocation census** on ordinary base classic supports-on: a
   counting global allocator (relaxed atomics only, no locks, no map — the
   `--report` accounting allocator's per-block mutex path would dominate the
   measurement) armed **only** around the collection's drop in
   `DefaultGCodeEmitter::emit_gcode_owned` (`crates/slicer-gcode/src/emit.rs`).
   Record blocks freed, bytes freed, a log2 size histogram, blocks and bytes per
   layer, the drop's microseconds, and the implied **ns per block** and
   **ns per byte**.
2. **Bulk-free counterfactual at the same byte volume**, in both pricing orders
   across two runs (bulk-first, bulk-after), so an ordering artifact cannot pass
   as a price: (a) **one block** of the whole collection's bytes — the per-print
   arena free; (b) **495 blocks** of one layer's bytes each — the per-layer
   arena free. Both arms touch the pages (a real arena holds live data) and the
   touch time is reported separately from the free.
3. **Copy-into-arena construction price**: the measured free of the originals
   plus a memcpy of the collection's bytes, so the "flatten a live collection
   into an arena" variant is priced with the storm it inherits. State explicitly
   that only arena-at-creation avoids that term, and what that would require
   (the marshal/assembly boundary, not the emitter).
4. **Allocator arm (measurement, not candidate)**: the same drop and census with
   a different global allocator (`mimalloc`, present in the local registry cache)
   in a probe build, to separate "the block count is the cost" from "the system
   heap is the cost". Shipping a different allocator is a **packager** decision
   under the in-repo contract (`docs/16_slicer_report.md` §"Global allocator
   contract"), so this arm prices the family and never proposes the change.
5. Both probe arms must emit the post-t81 ordinary references byte for byte:
   classic-on `22200671167a97a7efb1b912e9e1661a317b5e81dab169ec154ab5ad4b9f028d`
   (42,507,912 bytes), arachne-on
   `0b29e1bf928f1c0a160ebdedf08245edfd1d0433b3807ad1118b4578a378981c`
   (42,330,265 bytes).
6. **Stop rule:** stop at Phase 1 if the bulk-free ceiling (the measured drop
   minus the measured bulk free) is below the floor in the screening cell, or if
   the construction cost of the cheapest reachable arena shape is at or above
   that saving. A shape that is both cheaper and exact is the only Phase-2
   candidate.

## Pre-declared floor

**0.80 % of the cell wall**, the map's standing candidate floor. Screening cell
is ordinary base classic supports-on at **164.803 s** (t91's kept ordinary arm;
the cell is re-derived, not inherited — the archived 73.17 s screening cell is
supports-**off** and carries no postpass drop of this size) → **1,318 ms**. The
arachne-on cell's floor is **1,335 ms**.

## Proof gate

- Phase 1 probe reverted; both guest freshness gates exit 0
  (`cargo xtask build-guests --check`, and `--accelerated --check` if the
  accelerated tree is touched).
- Phase 2 only: the 16-cell output-identity matrix against
  `evidence/t91-exact-z-memo/references.json`, plus the standing four-cell paired
  gate (ordinary/accelerated x classic/arachne, base supports-on), with the
  per-pair byte-identity and cpu/wall disclosures the map requires.

## Deliverables

1. The Phase 1 tables: census (blocks, bytes, per layer, size histogram, ns per
   block vs ns per byte), the bulk-free counterfactuals in both orders, the
   copy-into-arena construction price, the allocator arm, the wall-equivalent
   share, and the floor verdict for each family.
2. If (and only if) a family is above the floor and exact, the candidate plus its
   equivalence tests.
3. A measured recommendation to the human — which family, at what price, with the
   scope question (IR arena vs packager allocator) stated. No auto-commit.

## Answer

**Measured Phase-1 screen — the arena / bulk-free family as scoped is
REJECTED: mis-aimed and construction-unreachable. No candidate, no campaign, no
production edit, probe reverted (2026-10-30, HEAD `cc2c54e6`).**

Four probe runs on ordinary base classic supports-on, all clean and all
byte-identical to the post-t81 reference
`22200671167a97a7efb1b912e9e1661a317b5e81dab169ec154ab5ad4b9f028d`
(42,507,912 bytes): a sampling-free census, a both-orders bulk run, a
shape/sample/copy run, and a mimalloc allocator arm.

### 1. The census — the free is per block, and the blocks are not geometry

| quantity | value |
| --- | ---: |
| layers / entities / points | 495 / 30,076 / 2,782,098 |
| blocks freed | **34,796,481** |
| bytes freed | **1,358,397,544** (1.358 GB) |
| drop, sampling-free | **3,526,544 µs** (t94 measured 3,558 ms on this cell) |
| ns per block | **101.3** |
| blocks in the 8–15-byte class | **34,662,668 = 99.6 %** |
| that class, sampled content | `__skirt_…` (support family/body ids), `demand-N` |
| `support_entity_identities` rows | 23,751 |
| demand-id strings (derived: bucket 4 − 2 per row) | 34,615,166 ≈ **1,457 per row** |
| geometry blocks (entities + layers + annotations + retracts) | 32,673 = **0.094 %** |
| geometry bytes (points + entities + layers) | 105.8 MB = **7.8 %** |

`assemble_ordered_entities_with_support_identities`
(`crates/slicer-runtime/src/layer_executor.rs`) clones `family_id`, `body_id`
and the body's whole `demand_ids: Vec<String>` **for every emitted support
path**, so a body whose demand list holds ~1,457 ids re-allocates those strings
once per path. That table is the dominant non-geometry term: geometry is 7.8 %
of the bytes, so the rows' `Vec<String>` buffers and their payloads are ~92 %
of the collection — 99.6 % of its free blocks.

### 2. Family verdicts

| family | verdict | measured |
| --- | --- | --- |
| **bulk-free arena, per print** (1 block) | ceiling above the floor, **construction unreachable** | freeing the same 1.358 GB as one block costs **44.5–63.8 ms** after the drop (189.7 ms before it, collection live). Ceiling = 3.527 s − 0.058 s = **3.468 s = 2.10 %** of the 164.803 s cell = **2.63x** the 1,318 ms floor. But flattening a live collection costs a **73.3–115.1 ms** copy **plus the same 3.527 s storm** (3.60 s total, **+73 ms worse**). Only arena-at-creation avoids that, and an at-creation arena aimed at the geometry removes **0.094 %** of the blocks. |
| **bulk-free arena, per layer** (495 blocks) | same | 58.3 ms after / 120.5 ms before — block granularity is not what makes the storm. |
| **allocator substitution** (mimalloc) | above the floor, but a **packaging** decision | drop **3.527 s → 1.302 s measured** (saving **2.225 s = 1.35 %** of the cell = **1.69x** floor); corrected for the probe's own 0.506 s sampling overhead, **0.796 s → 2.731 s = 1.66 % = 2.07x** floor. `docs/16_slicer_report.md` §"Global allocator contract" leaves the shipped allocator to packagers, so this prices the family and proposes no change; it also removes no work — 34.8M blocks are still freed. |

So the ticket's question is answered **no**: a bulk-free layer collection does
not buy the drop, because the collection is not a geometry arena — it is an
attribution table.

### 3. What the screen leaves

The reachable shape is the table itself ([ticket
101](101-support-identity-table-materialisation.md)): it has **no production
reader** (written at assembly, read only by
`structured_support_identity.rs` and `raft_geometry.rs` under
`crates/slicer-runtime/tests/integration/`, empty across the module marshal in
`crates/slicer-macros/src/lib.rs`, absent from `crates/slicer-schema/wit/`), it
is cloned per path, and its **free-side ceiling alone is 3.468 s (2.63x the
floor)** plus an unmeasured allocation-side cost. Families to rule between:
share one identity per body, prune/gate the table (the t20 pattern), or keep it.

### 4. Disclosures

- Instrumented walls (200.7–270.7 s) are not wall claims: `--profile
  --profile-verbose` and the probe's `eprintln` inflate them; the 164.803 s cell
  is t91's kept ordinary arm.
- The sampling instrumentation costs **0.506 s** on the same drop (34.66M extra
  atomic increments plus an 8-byte copy each): 3.527 s sampling-free vs 4.033 s
  sampled. The mimalloc arm was measured **with** sampling, so **1.302 s is an
  upper bound** and **0.796 s is derived**, labelled as derived everywhere.
- The `before` bulk arms ran with the collection still live (189.7 / 120.5 ms vs
  44.5–63.8 / 58.3 ms after); the verdict uses the conservative (max) arm.
- The copy arm was initially **elided by the optimiser** (dead stores into freed
  memory): the first run's `copy_us = 0` is discarded and the fixed arm
  (`black_box`-ed checksum read back) supplies 73.3–115.1 ms.
- Only classic-on was censused; arachne-on's drop was 3,242 ms (t94) with the
  same side table, not re-measured here.
- Both guest freshness gates exit 0 at close; the probe is host-only and
  reverted (no guest rebuilt, no guest artifact touched).

Evidence: [`evidence/t100-allocation-arena/`](../evidence/t100-allocation-arena/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 12/12 claims, 13/13 negative
controls rejected).
