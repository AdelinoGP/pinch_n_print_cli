# 97 — Author packet P90 — Multimaterial / Multimaterial advanced (2/2) — new: interlocking

Type: task
Status: closed (dissolved into ticket 96)
Assignee: —
Blocked by: 06, 96
Map: ../map.md

## Question

Author the spec packet for **P90 — Multimaterial / Multimaterial advanced (2/2) — new: interlocking** — 3 keys, Tier C new module, owner new module interlocking. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P90 — Multimaterial / Multimaterial advanced (2/2) — new: interlocking):

`interlocking_boundary_avoidance`, `interlocking_depth`, `interlocking_orientation`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Scaffold the new module via `pnp_cli module new`; new surface gated per repo rules.
- **Conforms to the interlocking ADR + module scaffold authored by ticket 96** — do not re-decide the seam or the ADR.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Dissolved into ticket 96's packet — not authored separately.**

All three of this ticket's keys are carried by
[`docs/spec_packets/306-interlocking-beams-slice-prepass/`](../../../spec_packets/306-interlocking-beams-slice-prepass/),
authored under ticket 96, `status: draft`, preflight PASS.

The 3+3 split cannot be implemented as two packets. Canonical's P89 enabling gate reads
**this ticket's** `interlocking_depth` in the same condition as the P89 trio
(`!interlocking_beam || interlocking_beam_layer_count < 1 || interlocking_depth < 1 ||
interlocking_beam_width < EPSILON`, `InterlockingGenerator::generate_interlocking_structure`),
so a P89-only packet cannot open its own gate without hardcoding a P90 key.
`interlocking_orientation` is applied before the voxel walk and unapplied on every output,
threading through every function such a packet would author; `interlocking_boundary_avoidance`
selects the whole `air_filtering` branch, including `handleThinAreas` and
`growBorderAreasPerpendicular`. Splitting buys nothing, because the work is the voxel
generator and the generator needs all six keys.

Precedent: ticket 31 (P24 dissolved into the prime-tower body). Packet 306's `task-map.md`
records the absorption under §Absorbed queue entry, mapping each of these three keys to the
packet step that implements it. Queue count unchanged: the keys are implemented, not
descoped.
