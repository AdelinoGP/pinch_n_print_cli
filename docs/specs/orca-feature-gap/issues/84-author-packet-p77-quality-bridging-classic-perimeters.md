# 84 — Author packet P77 — Quality / Bridging — classic-perimeters

Type: task
Status: open
Assignee: —
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P77 — Quality / Bridging — classic-perimeters** — 2 keys, Tier B new logic, owner classic-perimeters. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P77 — Quality / Bridging — classic-perimeters):

`bridge_angle`, `counterbore_hole_bridging`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.
- Owner spans two sibling modules (top-surface-ironing + support-surface-ironing / classic-perimeters + arachne-perimeters) — the packet touches both manifests.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
