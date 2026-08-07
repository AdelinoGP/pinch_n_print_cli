# 05 — Decide packet granularity and grouping

Type: grilling
Status: open
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
