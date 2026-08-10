# 21 — Author packet P14 — Quality / Ironing — top-surface-ironing

Type: task
Status: open
Assignee: —
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P14 — Quality / Ironing — top-surface-ironing** — 3 keys, Tier A plumbing, owner top-surface-ironing. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P14 — Quality / Ironing — top-surface-ironing):

`ironing_angle`, `ironing_angle_fixed`, `ironing_inset`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.
- Owner spans two sibling modules (top-surface-ironing + support-surface-ironing / classic-perimeters + arachne-perimeters) — the packet touches both manifests.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
