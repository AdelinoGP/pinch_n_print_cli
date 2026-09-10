# 98 — Author packet P91 — Multimaterial / Multimaterial advanced — new: mmu-segmented-region

Type: task
Status: open
Assignee: —
Blocked by: 06
Map: ../map.md

## Question

> **Read before claiming (added by ticket 96, 2026-09-10).** Both of this ticket's keys
> appear to be **already live** in this tree as host-side `ResolvedConfig` fields
> (`crates/slicer-ir/src/resolved_config.rs`), driving `run_phase5_width_limit`
> (`crates/slicer-core/src/algos/paint_segmentation/width_limit.rs`) — canonical's
> `cut_segmented_layers` parity — with end-to-end coverage in
> `crates/slicer-runtime/tests/executor/cube_4color_phase5_tdd.rs`. They landed in
> `b18c00b3` (2026-06-13), before ticket 01's asset was generated, and read `live=no` there
> only because ticket 01's probe does not scrape `ResolvedConfig` `cli` declarations — see
> [149](./149-measure-resolvedconfig-blind-spot-in-gap-inventory.md). **First act on claiming
> this ticket: verify that against the tree.** If it holds, P91 is a coverage confirmation
> (plus whatever gap remains between `run_phase5_width_limit` and canonical), not a new
> module, and the queue count drops by 2. The third member of that trio, the beam bool, was
> renamed to canonical `interlocking_beam` and re-homed by packet 306 under ticket 96; this
> ticket does not own it and must not re-home Phase 5.


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
