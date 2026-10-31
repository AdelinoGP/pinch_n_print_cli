# Support-attribution side table materialisation

Type: task
Label: wayfinder:task
Status: open
Assignee: none
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Allocation volume of the whole-print layer
collection](100-allocation-volume-layer-collection.md) ("What the screen leaves")

## Question

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
