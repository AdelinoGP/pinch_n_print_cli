# 32 — Author packet P25 — Extruder / Nozzle / Nozzle — skirt-brim

Type: task
Status: resolved
Assignee: wayfinder session (2026-09-03)
Blocked by: 06
Map: ../map.md

## Question

Author the spec packet for **P25 — Extruder / Nozzle / Nozzle — skirt-brim** — 1 keys, Tier B new logic, owner skirt-brim. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P25 — Extruder / Nozzle / Nozzle — skirt-brim):

`nozzle_height`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Not authored — P25 is dissolved. `nozzle_height` is not a skirt-brim key and
not config plumbing; it is one input to a feature this port does not have:
sequential printing (print-by-object) and its toolhead-clearance validation.**
Folded into
[124 — Author packet — sequential printing (print-by-object) and toolhead clearance validation](./124-author-packet-sequential-printing-and-toolhead-clearance.md).

### The owner in ticket 04 is wrong

Ticket 04's owner column says `skirt-brim`, presumably because canonical's only
non-GUI read of `nozzle_height` outside `Print::is_all_objects_are_short` sits in
a function called `Print::object_skirt_offset`. That name is misleading: the
value it returns is **never used to generate a skirt**. Its callers in
`libslic3r/` are `Print::sequential_print_clearance_valid` (twice — once to size
`obj_distance` for the convex-hull horizontal test, once to deflate each
instance's bounding box for the height test); the rest are `Arrange.cpp`,
`ArrangeJob.cpp` and `GLCanvas3D.cpp`, i.e. the GUI arranger. Neither
`Print::_make_skirt` nor `Print::_make_brim` reads it. `object_skirt_offset` is a
*clearance* computation that happens to account for per-object skirts widening an
object's footprint, not a skirt-generation one. This is the ticket-27 precedent
again: re-derive the owner from the decision point the key actually drives, not
from a neighbouring symbol's name.

### What the key actually selects in canonical

`nozzle_height` (`coFloat`, `PrintConfig.cpp`) is the height of the nozzle tip
below the toolhead's widest part — the answer to "can the gantry pass over an
already-finished object". Two reads, both in the sequential-print cluster:

1. **`Print::is_all_objects_are_short`** (`Print.hpp`) — `all_of(objects, height
   < scale_(nozzle_height))`. Consumed by `Print::sequential_print_clearance_valid`
   (short objects need only a nozzle-radius separation, not the full
   `extruder_clearance_radius`) and by `object_skirt_offset`'s first branch.
2. **`Print::object_skirt_offset`**'s second branch —
   `draft_shield == dsEnabled || skirt_height * max_layer_height > nozzle_height -
   margin_height`, i.e. "the skirt is tall enough to be an obstacle itself", which
   collapses the offset to `skirt_distance + line_width`.

Everything else is out of the slicing pipeline: `Preset.cpp` (preset plumbing),
`Print.cpp`'s step-invalidation option list, `PrintConfig.cpp`'s definition and
its `<= 0` validation, and `Arrange.cpp` / `ArrangeJob.cpp` / `Plater.cpp` (GUI).
The key passes rule 3 — it has genuine `libslic3r/` read sites — so it is **not**
out of scope.

### Why nothing lands today

This port has no sequential printing and no clearance model at all. Verified from
disk: `nozzle_height`, `extruder_clearance_radius`,
`extruder_clearance_height_to_rod`, `skirt_type` and `draft_shield` have **zero**
occurrences anywhere under `crates/`, `modules/`, `xtask/`; `print_sequence`
appears exactly once, as the hardcoded `("print_sequence", "by layer")` row of
`ORCA_CONFIG_PADDING` (`crates/slicer-gcode/src/serialize.rs`) — which rule 2
says is not evidence of anything. There is no convex-hull instance test, no
`bed_exclude_area`, and no per-object skirt grouping (ticket 13 already found
`skirt_type` declared-with-gap for exactly that reason).

So both decision points `nozzle_height` drives are inside a validator that does
not exist, guarding a print mode this port does not offer. A P25 packet today
would declare one float and wire it to nothing — 100% declaration-only,
prohibited by rule 1.

### The cluster, and why it is one feature not three tickets

`Print::sequential_print_clearance_valid` reads `nozzle_height`,
`extruder_clearance_radius`, `extruder_clearance_height_to_rod` and
`extruder_clearance_height_to_lid` together; the mode it guards is
`print_sequence == PrintSequence::ByObject`. The queue splits those across three
tickets with three different owners:

| ticket | packet | keys | ticket-04 owner |
| --- | --- | --- | --- |
| 32 (this one) | P25 | `nozzle_height` | skirt-brim |
| [76](./76-author-packet-p69-others-special-mode-layer-planner.md) | P69 | `print_sequence`, `slicing_mode` | layer-planner |
| [86](./86-author-packet-p79-printer-machine-print-volume-print-orchestration.md) | P79 | `extruder_clearance_*` (3) | print-orchestration |

None of the three can close on its own: the clearance keys have no validator
without the mode, the mode has no meaning without the clearance test, and
`nozzle_height` is an input to both. Ticket 124 owns the feature; 76 and 86 are
annotated to fold into it when claimed (their own sessions make that call —
`slicing_mode` in particular is a separate question and may well stay with P69).

### Disposition

- `nozzle_height` — **unimplemented, carried by ticket 124**. It stays in the
  queue count (in scope, not out of scope).
- P25 as a packet is **dissolved**; no packet number was taken.
- No key declared, no code change, no deviation filed — nothing exists here to
  diverge from.
