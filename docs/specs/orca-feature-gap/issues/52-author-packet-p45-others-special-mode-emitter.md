# 52 — Author packet P45 — Others / Special mode — emitter

Type: task
Status: open
Assignee: —
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P45 — Others / Special mode — emitter** — 5 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P45 — Others / Special mode — emitter):

`spiral_finishing_flow_ratio`, `spiral_mode`, `spiral_mode_max_xy_smoothing`, `spiral_mode_smooth`, `spiral_starting_flow_ratio`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.
- `spiral_mode` is cross-cutting (print/orchestration + emitter) — note the slicing-side aspect in the packet's design.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
