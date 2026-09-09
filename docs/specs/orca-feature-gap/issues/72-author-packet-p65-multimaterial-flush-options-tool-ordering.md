# 72 — Author packet P65 — Multimaterial / Flush options — tool-ordering

Type: task
Status: resolved
Assignee: wayfinder session (ses_20260908_P65) — claimed 2026-09-08, resolved 2026-09-08
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P65 — Multimaterial / Flush options — tool-ordering** — 3 keys, Tier B new logic, owner tool-ordering. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P65 — Multimaterial / Flush options — tool-ordering):

`flush_into_infill`, `flush_into_objects`, `flush_into_support`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Authored as packet 294** (`docs/spec_packets/294-flush-into-purge-reuse-wipe-tower/`, `draft`), preflight **PASS** (S0–S8 clean; S2's one `DEV-186` hit re-verified as the packet's own files — absent from the LOG, first collision-free with LOG max DEV-171 and drafts through DEV-185; S5/S6/S8 verified against the tree with shape notes). Tier B held, membership held 3-in with **no code change**: all three live in canonical and zero-occurrence here (no `crates/`/`modules/`/`xtask/` read, no `ORCA_CONFIG_PADDING` twin, no prior packet) — so the packet declares all three scalar-global bools on `wipe-tower.toml` (canonical defaults `false`/`false`/`true`) and builds the per-toolchange wiping-volume subtraction in the depth path (`purge_volume_for` → subtract → `purge_depth_for`, bed-bounds follows via the same helper). Owner corrected `tool-ordering` → `wipe-tower` (ordering ignores config and owns sequence only; the purge decision point is wipe-tower's — ticket-27/39/40 precedent). Canonical per-object shape NOT held (DEV-186(a) scalar-global; object-config axis is the named future, explicitly not ticket 125); filament-assignment vetoes named non-borrow (DEV-186(b)); bridge roles never count (DEV-186(c)); order untouched (DEV-186(d)). No `ResolvedConfig`/host-keys/CONFIG_BLOCK change (honest absence, AC-N1). Packet number `293` → `294` derived from disk; DEV-186 first collision-free. 04 tier rows corrected (owner + packet pointer) + 05 P65 annotated (3-in at 294); no queue-count change; no new fog, nothing out of scope.
