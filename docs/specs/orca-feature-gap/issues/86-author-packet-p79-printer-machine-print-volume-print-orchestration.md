# 86 — Author packet P79 — Printer / Machine / Print volume — print-orchestration

Type: task
Status: open
Assignee: —
Blocked by: 06
Map: ../map.md

## Question

> **Ticket 32 note (2026-09-03):** all three keys are read together by canonical's
> `Print::sequential_print_clearance_valid`, which guards `print_sequence ==
> PrintSequence::ByObject` — a print mode this port does not have. They cannot
> close as print-orchestration config plumbing; the validator that reads them is
> owned by
> [124 — Author packet — sequential printing (print-by-object) and toolhead clearance validation](./124-author-packet-sequential-printing-and-toolhead-clearance.md),
> along with `nozzle_height` (folded in from the dissolved P25). **Fold this
> packet's keys into 124 when this ticket is claimed**, or say why not. Note
> `extruder_clearance_height_to_rod` has a second, independent read in
> `TimelapsePosPicker` — check whether that one is separable before folding.

Author the spec packet for **P79 — Printer / Machine / Print volume — print-orchestration** — 3 keys, Tier B new logic, owner print-orchestration. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P79 — Printer / Machine / Print volume — print-orchestration):

`extruder_clearance_height_to_lid`, `extruder_clearance_height_to_rod`, `extruder_clearance_radius`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
