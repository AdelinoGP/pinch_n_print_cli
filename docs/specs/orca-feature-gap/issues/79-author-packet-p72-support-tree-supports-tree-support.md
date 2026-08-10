# 79 — Author packet P72 — Support / Tree supports — tree-support

Type: task
Status: open
Assignee: —
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P72 — Support / Tree supports — tree-support** — 8 keys, Tier B new logic, owner tree-support. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P72 — Support / Tree supports — tree-support):

`tree_support_angle_slow`, `tree_support_auto_brim`, `tree_support_branch_angle_organic`, `tree_support_branch_diameter_organic`, `tree_support_branch_distance_organic`, `tree_support_brim_width`, `tree_support_tip_diameter`, `tree_support_top_rate`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
