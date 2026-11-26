# 48 — Author packet P41 — Multimaterial / Multimaterial advanced — emitter

Type: task
Status: resolved
Assignee: wayfinder session (ses_f87dc4c86ffeu5OPQmBe4z8dHq) — claimed 2026-09-06, resolved 2026-09-06
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P41 — Multimaterial / Multimaterial advanced — emitter** — 1 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P41 — Multimaterial / Multimaterial advanced — emitter):

`support_object_skip_flush`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Re-sized at claim time: not authorable now, returned to the queue as
unimplemented — no packet, no code change** (the ticket-28/39/41/45 shape).

Claim-time grounding from the tree, not the tier table (map Notes,
"Packets are for complex implementation only"):

- **Zero occurrences in the tree.** `support_object_skip_flush` appears
nowhere under `crates/` / `modules/` / `xtask/` outside docs and the
queue assets — no `ResolvedConfig` field, no manifest row, no read site.
- **Canonical passes Authoring rule 3 (live, stays in scope).** Two read
sites, both in the slicing pipeline (`GCode.cpp`): the sequential-print
toolchange path (sets `m_filament_instances_code` from the incoming
object instance's labeled id before `set_extruder`) and the by-layer
extrusion loop (sets it from `filament_to_print_instances[extruder_id]`
before the toolchange). The stored code is later emitted as `M624 <code>`
ahead of `filament_end_gcode` / the toolchange. Declaration is `coBool`
default `false` (`PrintConfig.cpp`).
- **Both sites are gated on machinery this port does not have.**
Each read sits inside `if (... && m_enable_exclude_object && ...)`.
`m_enable_exclude_object` is set only for BBL printers in non-calibration
mode, and the effect (`M624` label codes, `m_label_objects_ids`,
`EXCLUDE_OBJECT` / `M486` markers) is the exclude-object feature —
which is P44's scope (`exclude_object`, `gcode_label_objects`,
ticket 51, still open). This tree has no `exclude_object` /
`gcode_label_objects` / `M624` / label-id concept at all (grep over
`crates/` + `modules/` returns nothing).
- **Wiring the key alone would be declaration-only** (Authoring rule 1):
with no exclude-object seam, there is no decision point for the bool to
drive and no `M624` emission to gate. A packet today would either declare
one bool (prohibited) or swallow P44's whole exclude-object feature to
earn it — the wrong packet boundary (the ticket-40 `start_end_points`
lesson: do not wire a rider before its carrier lands).

Owner note (ticket-27 hazard, recorded not re-derived): the tier table's
`crates/slicer-gcode (exclude-object emission)` owner is where the carrier
will land, but no seam exists to verify against yet — P44's authoring
re-derives it.

### Records

- `04-asset-tier-assignment.md`: `support_object_skip_flush` row annotated
returned-to-queue, sequences after P44 (ticket 51).
- `05-asset-packet-list.md`: P41 annotated returned-to-queue; key sequences
after (or folds into) P44 when ticket 51 lands — P44's 6→7 keys stay under
the B ceiling 12, so the fold is cheap at that ticket's claim time. No new
ticket filed (the `start_end_points` shape, not the 136/137/138 re-file
shape: the blocker is one packet, not the general per-tool model).
- No packet number taken, no key declared, no code change, no deviation
row (canonical default `false` recorded here for P44's authoring, not as
a divergence).

### Re-entry condition

When ticket 51 (P44 — Others / G-code output — emitter) implements the
exclude-object seam (`M624` / label ids / object markers), P41's key folds
into it or sequences immediately after it. P44's claim-time sizing owns
that call.
