# 05 — Decide packet granularity and grouping

Type: grilling
Status: resolved
Assignee: wayfinder session (ses_01687ac58ffe3kSB9jRYEh0v4Q)
Blocked by: 03, 04
Map: ../map.md

## Question

How are in-scope keys grouped into spec packets, and how big is one packet?

A packet must be executable by `/swarm` in a bounded run and must pass
`/spec-review --preflight`. Grouping axis is genuinely undecided:

- By **owning module** (all missing `seam-placer` keys in one packet) — keeps the
  diff local and the guest rebuild to one module, but mixes cost tiers.
- By **Orca UI section** (all of "Precision", all of "Bridging") — matches how a
  user perceives the feature, but can straddle modules.
- By **cost tier** (a "Tier A batch" packet of 30 plumbing keys) — maximises the
  cheapest-first ordering, but produces packets with no coherent theme, which
  makes acceptance criteria hard to write.

Also settle:

- Ceiling on keys per packet, and whether that ceiling differs by tier.
- Whether a Tier C feature is always exactly one packet.
- Whether any packet needs an ADR authored alongside it, and if so whether the
  ADR is a separate ticket on this map or part of the packet-authoring ticket.

Output: the grouping rule plus the resulting packet list — titles and key
membership only, not contents. That list is what the map's authoring tickets
will be cut from.

## Answer

Asset: [`05-asset-packet-list.md`](./05-asset-packet-list.md) — the grouping
rule, split points, and the full 91-packet list (titles + key membership,
ordered tier-major / owner / section).

### The rule (all rulings confirmed with the human)

- **Grouping axis: owning module, then Orca UI section.** The owner split
  already separates tiers almost everywhere (Prime tower's 26 A keys are all
  wipe-tower, its 2 B keys are emitter; Seam's 16 B are emitter, its 1 A is
  seam-placer) — so the tier axis is nearly redundant with the owner axis and
  serves only as a purity check plus the queue-order key. Packet order:
  tier-major (A, B, C — the destination's cheapest-first), then owning module
  (04's tie-breaker), then section.
- **Ceilings differ by tier, grounded in step size:** A ≤ 25 keys (declare +
  wire, S-steps), B ≤ 12 keys (new logic + tests, M-steps), C ≤ 4 keys (new
  module, ADR + guest rebuild). Oversize groups split by sub-theme: Prime
  tower 13+13, Retraction 10+10, Seam 8+8, Walls and surfaces 9+9, interlocking
  3+3. (Precedent check: existing packets are 4–5 steps / 85–215 lines per
  file; packet 212 covers 2 keys — small packets are normal.)
- **Tier C:** one feature = one module = one packet, split above 4 keys by
  feature cluster — only interlocking (6 keys) splits, 3+3. The first
  interlocking packet authors the module scaffold + ADR; the second conforms.
  (User ruling: "Split large modules".)
- **ADR:** only interlocking and mmu-segmented-region packets author ADRs,
  inside the packet-authoring ticket, number re-derived from disk at authoring
  time. Domain-modeling ruling, confirmed by the human: routine config-driven
  modules have no ADR in this repo (skirt-brim, part-cooling, top-surface-
  ironing, support-surface-ironing, wipe-tower, tree-support — zero ADR refs);
  algorithm ports and undecided seams do (infill-linker ADR-0026, lightning
  ADR-0029, support-planner ADR-0048; ADR-0033's host-bridge warning).
  elefant-foot, polyhole, contour-compensation are routine `pnp_cli module new`
  scaffolds with parity-dictated behavior — no ADR.
- **No merging of small groups** — the queue is 91 packets, of which 36 are
  ≤2-key groups. Exploration showed merging breaks the B ceiling immediately
  (an emitter misc would be 15 B keys) and costs theme; 2-key packets are
  normal precedent. (User ruling after exploration.)
- **Shared-owner keys** assigned by decision point: `printable_height` family →
  emitter, `bed_exclude_area` → wipe-tower, `timelapse_type` → wipe-tower
  (primary), `spiral_mode` → emitter (cross-cutting noted), ironing keys →
  top-surface-ironing.

### Counts (verified against 04's official tiers)

91 packets = 20 A (116 keys) + 65 B (223 keys) + 6 C (15 keys) = **354 keys**.
Not packetized: 47 Tier D keys (per-filament fog) and 2 fog-blocked Tier A
keys (`filament_density`, `filament_diameter`). 116 + 2 + 223 + 15 + 47 = 403 ✓.
`hole_to_polyhole_max_edges` remains missing from the inventory (04's flag).

### Fog graduated by this ticket

- **"The bulk authoring batches"** — graduated: 91 authoring tickets (one per
  packet), cut from the asset, all blocked on 06 (queue numbering decides
  whether authoring can proceed while `200-205` are in flight).
- Tier D filament-model fog unchanged (see map "Not yet specified").
