# 58 — Author packet P51 — Quality / Precision — emitter

Type: task
Status: open
Assignee: —
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P51 — Quality / Precision — emitter** — 2 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P51 — Quality / Precision — emitter):

`enable_arc_fitting`, `resolution` (the latter re-adjudicated from the rename pool in ticket 105: canonical `resolution` is a generation-time global simplify — `PerimeterGenerator.cpp` `ex.simplify_p`, `Brim.cpp`, `Fill.cpp`, `Layer.cpp`, `PrintObjectSlice.cpp`, `Print.cpp`, `TreeSupport` — plus emit-side arc density in `GCodeWriter.cpp`; the host's emit-time per-role `gcode_resolution` is a different decision point that stays. Packet grounding decides where the generation-time decision lands; check the `ORCA_CONFIG_PADDING` `("resolution", "0.012")` entry in `crates/slicer-gcode/src/serialize.rs` against canonical default 0.01 while in there)

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer
