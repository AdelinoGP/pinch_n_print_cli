# 16 — Author packet P09 — Strength / Infill pattern-specific — infill modules

Type: task
Status: resolved
Assignee: wayfinder session (ses_fa4c1f06bffel3YpS0Y76chRFh) — claimed 2026-09-01, resolved 2026-09-01
Blocked by: 06, 105, 107
Map: ../map.md

## Question

Author the spec packet for **P09 — Strength / Infill pattern-specific — infill modules** — 10 keys, Tier A plumbing, owner infill modules. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P09 — Strength / Infill pattern-specific — infill modules):

`infill_lock_depth`, `infill_overhang_angle`, `lateral_lattice_angle_1`, `lateral_lattice_angle_2`, `skeleton_infill_density`, `skeleton_infill_line_width`, `skin_infill_density`, `skin_infill_depth`, `skin_infill_line_width`, `symmetric_infill_y_axis`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

Packet `docs/spec_packets/263-infill-pattern-specific-keys/` authored (`draft`), preflight
**PASS** (report in the packet dir). **All 10 keys re-adjudicated declared-with-gap — a
pure-declaration packet: zero module-source reads, zero behavior change at any value.**
Canonical grounding (delegated reads) proved every decision point lives in an unshipped
pattern class or behind pattern gating the port's patterns never activate: six keys
(`infill_lock_depth`, `skin_infill_density`, `skin_infill_depth`, `skin_infill_line_width`,
`skeleton_infill_density`, `skeleton_infill_line_width`) are consumed only by
`FillLockedZag::fill_surface_locked_zag`; `lateral_lattice_angle_1`/`2` only by
`FillLateralLattice::fill_surface`; `infill_overhang_angle` only by
`FillLateralHoneycomb::fill_surface`; and **`symmetric_infill_y_axis` — the one key with a
live in-port decision point (the rectilinear scan-line generator) — is canonical-activated
only when the sparse pattern is zigzag/crosszag/lockedzag** (`Fill.cpp` `Layer::make_fills`
gate, verified verbatim; never for `ipRectilinear`), so wiring it would implement behavior
canonical never activates for this port's patterns; the zigzag-family re-open condition is
recorded in the key's disposition. The 10 tables land in `rectilinear-infill.toml` with
canonical type/default/bounds (percent forms per ticket 107, width forms per the in-tree
convention, bool for `symmetric_infill_y_axis`); guard is the net-new
`infill_pattern_specific_config_schema_tdd.rs` (distinct binary from 262's guard — no file
collision; shared-manifest append churn with packet 262 recorded as queue-order merge
churn, `toml = "0.8"` dev-dep add-if-absent). Zero deviation rows (5 parseable float
defaults match; `25%`/`100%` never enter the numeric comparison map; bool matches under the
ticket-100 comparison — block stays at 26, re-measured 2026-09-01); zero CONFIG_BLOCK
padding twins (honest absence pinned by AC-4). No user rulings required. Ledger facts
re-derived at authoring: next packet number 263 (disk-derived), deviation rows 26
(measured 2026-09-01). Unblocks nothing downstream; P10 (ticket 17) unaffected.
