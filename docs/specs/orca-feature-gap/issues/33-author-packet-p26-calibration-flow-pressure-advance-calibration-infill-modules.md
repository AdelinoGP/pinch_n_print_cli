# 33 — Author packet P26 — Calibration / Flow / Pressure advance calibration — infill modules

Type: task
Status: open
Assignee: —
Blocked by: 06, 105, 107
Map: ../map.md

## Question

Author the spec packet for **P26 — Calibration / Flow / Pressure advance calibration — infill modules** — 1 keys, Tier B new logic, owner infill modules. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P26 — Calibration / Flow / Pressure advance calibration — infill modules):

`calib_flowrate_topinfill_special_order`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
