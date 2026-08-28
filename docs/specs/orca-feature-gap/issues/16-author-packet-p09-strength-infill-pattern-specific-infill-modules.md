# 16 — Author packet P09 — Strength / Infill pattern-specific — infill modules

Type: task
Status: open
Assignee: —
Blocked by: 06, 105, 107
Map: ../map.md

## Question

Author the spec packet for **P09 — Strength / Infill pattern-specific — infill modules** — 10 keys, Tier A plumbing, owner infill modules. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P09 — Strength / Infill pattern-specific — infill modules):

`infill_lock_depth`, `infill_overhang_angle`, `lateral_lattice_angle_1`, `lateral_lattice_angle_2`, `skeleton_infill_density`, `skeleton_infill_line_width`, `skin_infill_density`, `skin_infill_depth`, `skin_infill_line_width`, `symmetric_infill_y_axis`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
