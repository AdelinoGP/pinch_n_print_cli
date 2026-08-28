# 20 — Author packet P13 — Support / Support — support-planner

Type: task
Status: open
Assignee: —
Blocked by: 06, 104
Map: ../map.md

## Question

Author the spec packet for **P13 — Support / Support — support-planner** — 12 keys, Tier A plumbing, owner support-planner. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P13 — Support / Support — support-planner):

`enforce_support_layers`, `raft_first_layer_expansion`, `support_bottom_z_distance`, `support_critical_regions_only`, `support_expansion`, `support_object_first_layer_gap`, `support_object_xy_distance`, `support_remove_small_overhang`, `support_style`, `support_threshold_angle`, `support_threshold_overlap`, `support_type`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
