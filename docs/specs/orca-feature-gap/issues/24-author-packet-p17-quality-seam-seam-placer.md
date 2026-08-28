# 24 — Author packet P17 — Quality / Seam — seam-placer

Type: task
Status: open
Assignee: —
Blocked by: 06, 102
Map: ../map.md

## Question

Author the spec packet for **P17 — Quality / Seam — seam-placer** — 1 keys, Tier A plumbing, owner seam-placer. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P17 — Quality / Seam — seam-placer):

`staggered_inner_seams`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
