# 86 — Author packet P79 — Printer / Machine / Print volume — print-orchestration

Type: task
Status: resolved
Assignee: wayfinder session (ses_20260910_P79) — claimed 2026-09-10, resolved 2026-09-10
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

**P79 dissolved; all three keys folded into ticket 124** (the ticket-32 note's instruction, confirmed — no "why not").

Claim-time grounding, verified against the oracle (`D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented`) and re-derived from this tree:

- **All three keys pass rule 3** (live in canonical's slicing pipeline) **and are zero-occurrence as behaviour here.** Tree-wide search over `crates/` + `modules/` + `xtask/` + `resources/` finds only the snapshot's `docs/ORCA_CONFIG_REFERENCE.md` rows plus one `ORCA_CONFIG_PADDING` twin (`"print_sequence"`, `crates/slicer-gcode/src/serialize.rs` — rule 2: not evidence). Canonical spelling, defaults, and readers, all fresh-read:
  - `extruder_clearance_height_to_rod` — `PrintConfig.cpp` `coFloat`, default **40**, min 0.
  - `extruder_clearance_height_to_lid` — `PrintConfig.cpp` `coFloat`, default **120**, min 0.
  - `extruder_clearance_radius` — `PrintConfig.cpp` `coFloat`, default **40**, min 0.
  - Main reader: `Print::sequential_print_clearance_valid` (`Print.cpp`) — horizontal hull-inflation test (`0.5 × radius + skirt offset − 0.1`), vertical lid/rod test against instance bounding boxes in arrange order, plus the `exclude_polys` single-object arm. Declared as a `Print::validate` invalidation dependency (`Print.cpp`, alongside `nozzle_height`).
  - `nozzle_height` (already ticket 124's via ticket 32) is the fourth reader-side input: `is_all_objects_are_short` (`Print.hpp` — every object shorter than `nozzle_height`) selects the short-object horizontal arm, and the `object_skirt_offset` computation (`Print.cpp`) reads it for the tall-object skirt branch. Canonical default **2.5**, min 0.
- **A standalone P79 packet cannot close** (rule 1: declaration-only is prohibited). The keys' only slicing meaning is the validator this port lacks — no validator, no per-instance convex hulls, no arrange order, no sequential mode. Ticket 124 already carries the mode (`print_sequence`), the fourth key (`nozzle_height`), and the validator feature, and its Question already lists these three P79 keys as carried content. This ticket adds nothing 124 does not already own.
- **The TimelapsePosPicker second read is separable and NOT folded.** `extruder_clearance_height_to_rod` + `extruder_clearance_radius` are read in the picker's constructor (`TimelapsePosPicker.cpp`, rod-limit height + clearance radius for traditional-timelapse park positioning), alongside `extruder_printable_height` (per-extruder vectors) and `print_sequence`/`timelapse_type`. Those reads belong to the timelapse park-position feature — which has no ticket, no owner, and no tree seam (this port injects `time_lapse_gcode` from `machine-gcode-emit` with no position picker at all) — not to the sequential validator. Folding it into 124 would smuggle an unrelated feature into that ticket's scope ruling. It is recorded here as a known non-borrow, not as queue work.
- **One adjacent finding for ticket 124's authoring, not this ticket's scope:** `extruder_clearance_max_radius` is a legacy alias (`PrintConfig.cpp` `handle_legacy`: `extruder_clearance_max_radius` → `extruder_clearance_radius`), and `PrintConfig.cpp`'s min-object-distance helper already couples `print_sequence == ByObject` with the radius. Both ride 124's grounding, not a separate ticket.
- Owner note (ticket-27 hazard): the tier table's `print/orchestration` owner is reviewed against canonical, not against this tree — this tree has no `Print::validate` analog. Where the validator lands (scheduler validation vs `run_slice`) is ticket 124's authoring decision, which is exactly why the keys follow the feature ticket rather than authoring standalone.

No packet number taken, no deviation rows, no `ORCA_CONFIG_PADDING` edit (the `print_sequence` twin stays untouched). No code change; no queue-count change. 04/05 rows annotate the fold.
