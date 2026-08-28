# 15 — Author packet P08 — Strength / Infill — infill modules

Type: task
Status: open
Assignee: —
Blocked by: 06, 105, 107
Map: ../map.md

## Question

Author the spec packet for **P08 — Strength / Infill — infill modules** — 7 keys, Tier A plumbing, owner infill modules. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P08 — Strength / Infill — infill modules):

`fill_multiline`, `gap_fill_target`, `internal_solid_infill_pattern`, `solid_infill_direction`, `solid_infill_rotate_template`, `sparse_infill_pattern`, `sparse_infill_rotate_template`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
