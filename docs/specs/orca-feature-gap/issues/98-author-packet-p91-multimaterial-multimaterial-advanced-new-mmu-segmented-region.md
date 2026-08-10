# 98 — Author packet P91 — Multimaterial / Multimaterial advanced — new: mmu-segmented-region

Type: task
Status: open
Assignee: —
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P91 — Multimaterial / Multimaterial advanced — new: mmu-segmented-region** — 2 keys, Tier C new module, owner new module mmu-segmented-region. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P91 — Multimaterial / Multimaterial advanced — new: mmu-segmented-region):

`mmu_segmented_region_interlocking_depth`, `mmu_segmented_region_max_width`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Scaffold the new module via `pnp_cli module new`; new surface gated per repo rules.
- **Authors an ADR** for the seam decision: guest module vs host-side wiring into the existing paint_segmentation pipeline (ADR-0033 warns undocumented host-bridge instances repeat).

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
