# 14 — Author packet P07 — Others / Fuzzy Skin — fuzzy-skin

Type: task
Status: open
Assignee: —
Blocked by: 06, 103
Map: ../map.md

## Question

Author the spec packet for **P07 — Others / Fuzzy Skin — fuzzy-skin** — 7 keys, Tier A plumbing, owner fuzzy-skin. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P07 — Others / Fuzzy Skin — fuzzy-skin):

`fuzzy_skin`, `fuzzy_skin_first_layer`, `fuzzy_skin_mode`, `fuzzy_skin_noise_type`, `fuzzy_skin_octaves`, `fuzzy_skin_persistence`, `fuzzy_skin_scale`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
