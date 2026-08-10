# 94 — Author packet P87 — Quality / Precision — new: polyhole

Type: task
Status: open
Assignee: —
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P87 — Quality / Precision — new: polyhole** — 3 keys, Tier C new module, owner new module polyhole. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P87 — Quality / Precision — new: polyhole):

`hole_to_polyhole`, `hole_to_polyhole_threshold`, `hole_to_polyhole_twisted`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Scaffold the new module via `pnp_cli module new`; new surface gated per repo rules.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
