# 80 — Author packet P73 — Strength / Advanced (Strength) — object-level planning

Type: task
Status: open
Assignee: —
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P73 — Strength / Advanced (Strength) — object-level planning** — 4 keys, Tier B new logic, owner object-level planning. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P73 — Strength / Advanced (Strength) — object-level planning):

`ensure_vertical_shell_thickness`, `extra_solid_infills`, `infill_combination`, `infill_combination_max_layer_height`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
