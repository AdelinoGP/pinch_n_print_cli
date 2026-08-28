# 22 — Author packet P15 — Support / Support ironing — support-surface-ironing

Type: task
Status: open
Assignee: —
Blocked by: 06, 106
Map: ../map.md

## Question

Author the spec packet for **P15 — Support / Support ironing — support-surface-ironing** — 2 keys, Tier A plumbing, owner support-surface-ironing. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P15 — Support / Support ironing — support-surface-ironing), amended by ticket 07:

`support_ironing_pattern`, `support_ironing`

The `support_ironing` key is the 07 reclassification: an independent bool so
support-interface ironing no longer rides the shared `ironing_enabled` bool
(declared identically by both support-surface-ironing and
top-surface-ironing — the two Orca features cannot be toggled independently
today).

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
