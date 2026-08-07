# 02 — Set the canonical-parity evidence standard for gap packets

Type: grilling
Status: open
Blocked by: —
Map: ../map.md

## Question

What evidence must each packet in this queue carry that its implementation
matches canonical OrcaSlicer behaviour — and what is the fallback when that
evidence cannot be obtained?

`CLAUDE.md` makes canonical parity correctness "the highest priority" and
forbids weakening the canonical implementation to make tests pass. But
OrcaSlicer is **not vendored in this repo**, citations may not use line numbers,
and a queue of this size cannot afford a bespoke parity investigation per key.

Settle, with the human:

- Does a local OrcaSlicer checkout exist on this machine, and may packets assume
  the implementer has one? (If not, the standard must be checkout-free.)
- What tiers of evidence are acceptable: canonical function read + described
  behaviour, a recorded fixture, a golden G-code comparison, or documented
  behaviour only?
- For pure config-plumbing keys (a threshold that feeds an existing decision
  point), is "default matches upstream + key reaches the consumer" sufficient,
  or does every key need a behavioural test?
- When a key's canonical behaviour cannot be verified, is the correct move to
  file a `docs/DEVIATION_LOG.md` row, defer the key, or block the packet?

The answer becomes a boilerplate acceptance-criteria block reused by every
packet this map authors, so it must be concrete enough to paste.

This ticket is deliberately unblocked: it constrains packet *shape*, not packet
*contents*, so it does not need the inventory.
