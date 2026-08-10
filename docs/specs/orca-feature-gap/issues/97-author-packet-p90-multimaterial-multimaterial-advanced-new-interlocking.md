# 97 — Author packet P90 — Multimaterial / Multimaterial advanced (2/2) — new: interlocking

Type: task
Status: open
Assignee: —
Blocked by: 06, 89
Map: ../map.md

## Question

Author the spec packet for **P90 — Multimaterial / Multimaterial advanced (2/2) — new: interlocking** — 3 keys, Tier C new module, owner new module interlocking. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P90 — Multimaterial / Multimaterial advanced (2/2) — new: interlocking):

`interlocking_boundary_avoidance`, `interlocking_depth`, `interlocking_orientation`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Scaffold the new module via `pnp_cli module new`; new surface gated per repo rules.
- **Conforms to the interlocking ADR + module scaffold authored by ticket 89** — do not re-decide the seam or the ADR.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
