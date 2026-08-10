# 96 — Author packet P89 — Multimaterial / Multimaterial advanced (1/2) — new: interlocking

Type: task
Status: open
Assignee: —
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P89 — Multimaterial / Multimaterial advanced (1/2) — new: interlocking** — 3 keys, Tier C new module, owner new module interlocking. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P89 — Multimaterial / Multimaterial advanced (1/2) — new: interlocking):

`interlocking_beam`, `interlocking_beam_layer_count`, `interlocking_beam_width`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Scaffold the new module via `pnp_cli module new`; new surface gated per repo rules.
- **Authors the interlocking module's ADR** (algorithm port: port-strategy + seam + data-flow decisions; number re-derived from disk at authoring time).

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
