---
status: implemented
packet: 132_modifier-region-split
task_ids: [TASK-257]
---

# 132_modifier-region-split

## Goal

Make modifier volumes geometrically real: slice each modifier mesh per layer, intersect with the owning region's partitioned fill polygons, and split them into wall-less sub-regions that carry their own region identity + config binding (`ModifierScope` beyond `AllFeatures`) while sharing the base region's walls (`wall_source_region_id = base`) — per ADR-0030.

## Problem Statement

Modifier volumes were ingested but never geometrically applied: config was stamped globally per object (only `ModifierScope::AllFeatures` in use), modifier meshes were never sliced/intersected, and per-region config could not reach a module (the first-match ConfigView, retired by packet 131). OrcaSlicer reference behavior: one wall set, fill partitioned at the modifier boundary, each sub-area at its own density/pattern, no walls at the modifier boundary. ADR-0030 is the governing decision.

## Architecture Constraints

- Host-only geometry + config binding: modifier-mesh slicing, fill-polygon splitting at region partition, `ModifierScope` extension, `wall-source-region-id` population. NO WIT change (contract fields from packet 130; config accessor from packet 131 — FORWARD-DEPs).
- No perimeter generation at modifier boundaries — walls stay merged on the base; no wall loops keyed to the sub-region (AC-3).
- No e2e 3MF fixture (that is M3, packet 136); tests construct objects + modifier volumes programmatically.
- Precedence composition: the split applies to the already-partitioned polygons, so `bridge > bottom > top > sparse` precedence is unchanged — pinned by test.

## Data and Contract Notes

- Sub-region identity (delivered): `modifier_sub_region_id` carries a dedicated high-bit namespace marker over the payload `base_region_id * MODIFIER_VARIANT_REGION_ID_STRIDE + hash`, with STRIDE `1_000_003` (next prime above paint's `1_000_000`). The stable hash is derived from the object ID and the sliced modifier footprint geometry, never from HashMap iteration. The raw `u64::MAX` footprint marker is reserved and excluded from the minted namespace.
- An unpainted parent produces a child with an empty `variant_chain`; a painted parent produces a child retaining the parent's chain and using the painted parent ID in the payload. `wall_source_region_id` masks the namespace bit before recovering the parent ID. This preserves the full `RegionKey` identity while keeping modifier deltas spatial: each parent remains pure outside its own child. This is the ticket 19 extension of packet 132's original empty-chain rule.
- Modifier-mesh slicing site: `split_modifier_sub_regions_for_prepass`; the same shared splitter is reused by the Tier-2 footprint fallback. Empty cross-section ⇒ no split on that layer (Z-interval scoping falls out).
- Overlapping non-support modifiers: priority first, document order breaks ties — the first winning modifier owns the footprint; later modifiers intersect only the remaining base area (recorded as the packet's chosen semantics, not a deviation).
- Invariant: partition conservation (base + sub-region sparse areas equal pre-split within 1%); no-modifier objects byte-identical (AC-N1, wedge SHA); degenerate modifier slice ⇒ no split, no panic (AC-N2).

## Locked Assumptions and Invariants

- Sub-regions carry their own resolved config via `stamp_modifier_sub_region_configs` keyed by region_id (overlay of modifier deltas onto base `ResolvedConfig`, skipping support_enforcer/blocker subtypes).
- The infill linker (packet 133) reads only `wall_source_region_id` + `tool_index` + the four fill polygons — it does not read `variant_chain`.
- Per-region config delivery (packet 131) is the companion: AC-4 proves 0.40 inside the sub-region vs 0.15 on the base.

## Risks and Tradeoffs

- Region counts grow on modifier-bearing layers; downstream per-region loops pay the iteration cost (bounded by modifier count).
- Golden carve churn from the ConfigView fix — packet 136's restore scope.

## Implementation Deviations (recorded at close)

None beyond the recorded identity/ownership resolutions above (design.md FWD-RESOLVED 2-4). Doc Impact: `docs/02_ir_schemas.md` §region partition / SlicedRegion modifier sub-region semantics (updated in-packet; identity-hash wording corrected by the 2026-08-05 doc audit).
