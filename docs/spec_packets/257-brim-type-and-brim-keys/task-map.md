# Task Map: brim-type-and-brim-keys

Use this crosswalk when a packet spans more than one task ID, reopens prior work, or supersedes an earlier packet. **This packet emits the template's own skip clause:** it is a single-coherent-slice packet with `task_ids: []` (queue precedent — packets 234a, 253, 254, 255, 256), so the `docs/07` crosswalk is N-A. Implementation is recorded against wayfinder ticket 12 (`docs/specs/orca-feature-gap/issues/12-author-packet-p05-others-brim-skirt-brim.md`).

## Crosswalk

| Packet slice | `docs/07` task IDs | Wayfinder ticket | Notes |
| --- | --- | --- | --- |
| Whole packet (Steps 1–5) | — | 12 — Author packet P05 — Others / Brim — skirt-brim | Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; no TASK rows; re-derive the crosswalk question at completion time per the ledger-fact rule |