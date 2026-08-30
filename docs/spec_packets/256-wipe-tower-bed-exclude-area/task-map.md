# Task Map: 256-wipe-tower-bed-exclude-area

Use this crosswalk when a packet spans more than one task ID, reopens prior work, or supersedes an earlier packet. **This packet emits the template's own skip clause:** it is a single-coherent-slice packet with `task_ids: []` (queue precedent — packets 234a, 253, 254, 255), so the `docs/07` crosswalk is N-A. Implementation is recorded against wayfinder ticket 11 (`docs/specs/orca-feature-gap/issues/11-author-packet-p04-printer-machine-print-volume-wipe-tower.md`).

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| N-A | Steps 1–5 | `docs/specs/orca-feature-gap/issues/11-…` | `modules/core-modules/wipe-tower/**`, scheduler tests | `PrintConfig.cpp`, `Print.cpp` | `M` | no TASK row exists for the feature-gap queue (packet 234a precedent; re-derive at completion time — this paragraph is a ledger statement frozen at authoring time) |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.