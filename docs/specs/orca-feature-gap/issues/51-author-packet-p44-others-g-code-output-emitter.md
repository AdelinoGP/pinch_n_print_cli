# 51 — Author packet P44 — Others / G-code output — emitter

Type: task
Status: resolved
Assignee: wayfinder session (ses_f863e798affe301F6V4b7FDikC)
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P44 — Others / G-code output — emitter** — 6 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P44 — Others / G-code output — emitter):

`exclude_object`, `filename_format`, `gcode_comments`, `gcode_flavor`, `gcode_label_objects`, `reduce_infill_retraction`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Authored as packet 278** (`docs/spec_packets/278-gcode-output-emitter-modes/`,
`draft`), preflight **PASS** (no blockers, no high findings). Claim-time
re-derivation kept the Tier B sizing but changed the membership and two owners:
**5 keys in, `filename_format` returned to the queue, no code change.**

- Tree grounding (2026-09-07): `exclude_object`, `gcode_comments`,
  `gcode_label_objects`, `reduce_infill_retraction` are zero-occurrence as
  behaviour (the one `reduce_infill_retraction` hit is the
  `ORCA_CONFIG_PADDING` row — rule 2, not evidence); `gcode_flavor` is only
  partially wired (parsed at `crates/slicer-runtime/src/run.rs`,
  echoed into CONFIG_BLOCK at `crates/slicer-gcode/src/serialize.rs`, no
  flavor-dependent syntax decision). No M624/exclude/label seam, no comments
  gate, no filename expansion, no `needs_retraction` function exist.
- Canonical grounding (oracle `D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented`,
  sibling checkout): all six pass rule 3 and stay in scope —
  `GCode.cpp::apply_print_config` / `process_layer` (exclude_object),
  `Print.cpp::output_filename` (filename_format),
  `GCode.cpp::do_export` / `_extrude` (gcode_comments),
  `GCodeWriter.cpp::GCodeWriter::apply_print_config` +
  `GCode.cpp::process_layer` / `_print_first_layer_extruder_temperatures`
  (gcode_flavor), `GCode.cpp::process_layer` (gcode_label_objects),
  `GCode.cpp::needs_retraction` (reduce_infill_retraction).
- **`filename_format` returned to the queue as unimplemented** (owner
  correction): its canonical consumer selects an artifact filename while
  PnP's artifact write is CLI-owned (`Cmd::Slice` writes the explicit
  `--output` path or stdout) — an emitter must never choose filesystem
  paths. Missing feature named: automatic placeholder-based output naming
  with no explicit `--output`. Natural host is the open P84/P85 host-export
  scope (tickets 91/92); 04/05 annotation rides the packet's close-out step.
  No queue-count change (still in scope, different owner).
- **`reduce_infill_retraction` owner-corrected** `crates/slicer-gcode` →
  `path-optimization-default`: packet-15/TASK-120d1 established the path
  optimizer as retract-policy owner and `DefaultGCodeEmitter::emit_gcode`
  as serializer only. The packet builds the consecutive-entity travel
  classification there and makes the current internal suppression opt-in.
- **`support_object_skip_flush` sequenced, not folded** (ticket 48's
  re-entry call, owned at claim time): it needs a Bambu flavor, `M624`/`M625`
  label-code emission, and a filament-flush toolchange carrier — adding the
  bool now would be declaration-only (rule 1). Stays queued behind this
  packet's exclude-object seam; no new ticket (the `start_end_points` shape).
- P44 now covers **5 keys**; every retained key has a behaviour-changing AC
  at non-default with a narrow test command; no padding edits; canonical
  on-wire spellings specified (exact `gcode_flavor` variants, strict
  normal-path validation with a rejection AC).
