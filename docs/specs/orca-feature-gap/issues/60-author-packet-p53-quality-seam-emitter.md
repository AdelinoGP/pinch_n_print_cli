# 60 — Author packet P53 — Quality / Seam (2/2) — emitter

Type: task
Status: open
Assignee: —
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P53 — Quality / Seam (2/2) — emitter** — 8 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P53 — Quality / Seam (2/2) — emitter):

`seam_slope_entire_loop`, `seam_slope_inner_walls`, `seam_slope_min_length`, `seam_slope_start_height`, `seam_slope_steps`, `seam_slope_type`, `wipe_before_external_loop`, `wipe_on_loops`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
