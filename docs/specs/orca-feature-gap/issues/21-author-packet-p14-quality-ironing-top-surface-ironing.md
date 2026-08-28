# 21 — Author packet P14 — Quality / Ironing — top-surface-ironing

Type: task
Status: open
Assignee: —
Blocked by: 06, 106
Map: ../map.md

## Question

Author the spec packet for **P14 — Quality / Ironing — top-surface-ironing** — 4 keys (3 Tier A plumbing + 1 Tier B logic), owner top-surface-ironing. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P14 — Quality / Ironing — top-surface-ironing), amended by ticket 07:

`ironing_angle`, `ironing_angle_fixed`, `ironing_inset`, `ironing_type`

The `ironing_type` key is the 07 reclassification: it widens the shared
`ironing_enabled` bool (declared identically by both top-surface-ironing and
support-surface-ironing) to Orca's 4-mode enum (no ironing/top/topmost/solid)
— enum modes are unexpressible today. Mode-dependent layer selection is new
logic (Tier B); parity evidence per 02 (canonical: `Fill.cpp::Layer::make_ironing`).

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.
- Owner spans two sibling modules (top-surface-ironing + support-surface-ironing / classic-perimeters + arachne-perimeters) — the packet touches both manifests.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
