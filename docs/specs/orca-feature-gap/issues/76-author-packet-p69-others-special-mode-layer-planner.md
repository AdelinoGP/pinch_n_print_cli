# 76 — Author packet P69 — Others / Special mode — layer-planner

Type: task
Status: open
Assignee: —
Blocked by: 06, 104
Map: ../map.md

## Question

> **Ticket 32 note (2026-09-03):** `print_sequence` is the mode gate for
> sequential printing, whose clearance validation is owned by
> [124 — Author packet — sequential printing (print-by-object) and toolhead clearance validation](./124-author-packet-sequential-printing-and-toolhead-clearance.md).
> It cannot close here: `print_sequence == PrintSequence::ByObject` selects a
> print mode this port does not have, and the keys that make it meaningful
> (`nozzle_height`, `extruder_clearance_*`) live in ticket 124. **Fold
> `print_sequence` into 124 when this ticket is claimed.** `slicing_mode` is a
> separate question and may well stay with P69 — this session decides that, not
> ticket 124.

Author the spec packet for **P69 — Others / Special mode — layer-planner** — 2 keys, Tier B new logic, owner layer-planner. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P69 — Others / Special mode — layer-planner):

`print_sequence`, `slicing_mode`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
