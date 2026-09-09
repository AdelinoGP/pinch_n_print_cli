# 81 — Author packet P74 — Strength / Top/bottom shells — object-level planning

Type: task
Status: resolved
Assignee: wayfinder session (ses_20260909_P74) — claimed 2026-09-09, resolved 2026-09-09
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P74 — Strength / Top/bottom shells — object-level planning** — 2 keys, Tier B new logic, owner object-level planning. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P74 — Strength / Top/bottom shells — object-level planning):

`bottom_shell_thickness`, `top_shell_thickness`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Tier B held, owner confirmed but narrowed to the prepass seam, packet authored, no re-file.** Both keys live in canonical's slicing pipeline (`PrintObject.cpp::discover_horizontal_shells` top/bottom projection loops — the count-floor `i < itop` / `i > ibottom` arms plus the `||` thickness arms against `print_z` / `bottom_z` with the `EPSILON` margin, verified against the oracle at claim time; `PrintConfig.cpp` coFloat top `0.6` / bottom `0.0` min 0) and are zero-occurrence as behaviour in this tree (verified by tree-wide `rg` over `crates/` + `modules/` + `xtask/` — no reads, no `ORCA_CONFIG_PADDING` rows, no manifest declarations, no `ResolvedConfig` fields; no draft packet owns the decision — the `shell_thickness` substring hits in 299 are the P73 `ensure_vertical_shell_thickness` mode key, a different decision). Ticket 04's "object-level planning" owner is accurate but is a seam, not a module: the decision points land in the host prepass `compute_region_updates` Pass-2 shadow walks fed by `resolve_shell_counts`' `region_map.config_for` reads (the ticket-36 precedent), never module-side.

Packet `docs/spec_packets/300-top-bottom-shell-thickness/` authored (`draft`), **preflight PASS** (S0–S8, all tree-verified: 8/8 pre-existing symbols resolved with file:line evidence; DEV-191 verified next-free — LOG max DEV-171, drafts claim DEV-172–DEV-190; no schema bump — no IR/WIT change; ADR-0062/0063 conformance, not amendment; `--lib` test homes need no aggregator registration; one self-correction — AC-3's default-identity rationale tightened to the exact 3-layers-×-0.2-mm = 0.6-mm `EPSILON` argument, and one S5-hygiene fix dropping the unbumped `CURRENT_SLICE_IR_SCHEMA_VERSION` from the Depends line). Membership 2/2, none shed, none returned: the thickness pair extends the count pair's projection walks with the canonical `||` arm (`0` = disabled = pre-packet shape); defaults ARE identity (AC-3 pins the flat-geometry fixture byte-identical — inverse of 299's emitting default). Canonical scalarity IS held (both scalar — no ticket-125 vector arm); no numeric range rejection (ticket-113 rule; negatives saturate to disabled, and the packet carries no rejection criterion — AC-N1 is a lock-bypass invariant, not a validation gate); locked-path conformance pinned (AC-N1); CONFIG_BLOCK honest absence (rides 132; no padding edits, rule 2); rule 4 does not fire (in-walk arms, `seam_position` precedent). Three divergences in new **DEV-191**: the `PrintObject::infill` scatter is not borrowed as a second site (prepass projection already carries the thickness downstream), the spiral-mode bottom-layer gate is not borrowed (`spiral_mode` unimplemented), and `Print.cpp` reslice-invalidation rides ticket 124. No code change; no queue-count change; no new fog, nothing out of scope.
