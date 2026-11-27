# 41 — Author packet P34 — Extruder / Nozzle / Nozzle — emitter

Type: task
Status: closed
Assignee: wayfinder session (ses_f8c404e02ffeA88fMulKm6Ki8P) — claimed 2026-09-05
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P34 — Extruder / Nozzle / Nozzle — emitter** — 4 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P34 — Extruder / Nozzle / Nozzle — emitter):

`nozzle_hrc`, `nozzle_type`, `nozzle_volume`, `required_nozzle_HRC`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Re-sized at claim time: not authorable now, all four keys re-filed — no
packet, no code change** (the ticket-28/39 shape, under the "Packets are for
complex implementation only" rule).

All four keys (`nozzle_hrc`, `nozzle_type`, `nozzle_volume`,
`required_nozzle_HRC`) have **zero occurrences** in `crates/` / `modules/` /
`xtask/`, and canonical grounding (oracle
`D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented`) shows every read
site is in `GCodeProcessor`, not slicing geometry: the HRC trio feeds the
non-fatal `NOZZLE_HRC_CHECKER` post-export warning
(`GCodeProcessor::apply_config` both overloads,
`GCodeProcessor::update_slice_warnings` with the
`Print::get_hrc_by_nozzle_type` fallback, surfaced by
`Plater::get_slice_warning_string`); `nozzle_volume` (`coFloats`
per-extruder) feeds only Elegoo-`M6211` flush attribution
(`GCodeProcessor::process_filaments` + `process_elegoo_M6211`, ignored on
non-Elegoo). All four are live in canonical, so none is ruled out of scope.
The tier table's `crates/slicer-gcode` owner is wrong for this tree — the
emitter has no nozzle-identity decision point, the port has no warning-list
seam (degraded counters only), no Elegoo seam, and no per-tool vector
ingestion (`extract_float_or_first` keeps element 0; three keys are
per-extruder/per-filament vectors and the fourth is compared per-extruder). A
packet today would be 100% declaration-only (rule 1), and building the
per-tool ingestion inside it would duplicate ticket 125.

**Re-filed as
[137 — Author packet P34 (re-filed)](137-author-packet-p34-nozzle-keys-refiled.md),
blocked on 125** (the per-tool config model ruling, itself gated on 126).
The warning-vs-fatal divergence question and the `nozzle_volume`
Elegoo vendor-scope ruling ride the re-file. Tier-table + packet-list P34
rows annotate the re-file; no queue-count change (nothing covered, nothing
ruled dead).
