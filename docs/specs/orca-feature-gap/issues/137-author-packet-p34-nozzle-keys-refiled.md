# 137 — Author packet P34 (re-filed) — Extruder / Nozzle / Nozzle keys

Type: task
Status: open
Assignee: —
Blocked by: 125
Map: ../map.md

## Question

Close **P34 — Extruder / Nozzle / Nozzle** — 4 keys, Tier B — once
[ticket 125](125-rule-per-tool-config-model.md) (itself gated on
[ticket 126](126-overlay-resolved-field-narrowing.md)) has ruled on the
per-tool config model: `nozzle_hrc`, `nozzle_type`, `nozzle_volume`,
`required_nozzle_HRC`.

Re-filed by [ticket 41](41-author-packet-p34-extruder-nozzle-nozzle-emitter.md):
a packet today would be 100% declaration-only (Authoring rule 1), so no packet
number was taken and no key was declared. Key membership is unchanged from
[05-asset-packet-list.md](./05-asset-packet-list.md) P34; the authoring ticket
for these keys is this one (137), not 41 — the 28→119 / 39→136 pattern.

### Canonical grounding (ticket 41's measurement, oracle `D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented` — re-derive the path at point of use)

All four keys are **live in canonical** (rule 3 keeps them in scope), but every
read site is in `GCodeProcessor`, not in `Print`/`GCode` slicing geometry:

- `nozzle_hrc` (`coInt` scalar, default `0`): `GCodeProcessor::apply_config`
  (both overloads) copies the scalar to every extruder;
  `Print::get_hrc_by_nozzle_type` supplies the fallback hardness when
  non-positive; `GCodeProcessor::update_slice_warnings` compares each used
  filament's requirement against its mapped extruder and appends
  `NOZZLE_HRC_CHECKER` to `GCodeProcessorResult::warnings`. Non-fatal, aborts
  nothing, emits no G-code; surfaced by `Plater::get_slice_warning_string`
  (GUI / post-export warning list).
- `nozzle_type` (`coEnums` per-extruder vector, nullable, default
  `{ntUndefine}`; registered values `undefine` / `hardened_steel` /
  `stainless_steel` / `tungsten_carbide` / `brass`): same warning path, as the
  fallback-HRC source.
- `required_nozzle_HRC` (`coInts` per-filament vector, default `{0}`): same
  warning path, as the per-filament requirement side of the comparison.
- `nozzle_volume` (`coFloats` per-extruder vector, nullable, default `{0.0}`):
  `GCodeProcessor::process_filaments` resets the active extruder's remaining
  volume on toolchange; `GCodeProcessor::process_elegoo_M6211` (via
  `process_M6211`, only when the printer model starts with "elegoo") uses it to
  apportion flushed material in filament statistics / load timing. Ignored on
  non-Elegoo printers: **no emitted-G-code effect outside Elegoo**.

### Tree state (ticket 41, re-derive at claim time)

- Zero occurrences of all four keys, `NOZZLE_HRC`, `M6211`, and `elegoo`
  (case-insensitive) under `crates/` / `modules/` / `xtask/`.
- The tier table's `crates/slicer-gcode` owner is **wrong for this tree**:
  the emitter branches on tool index / feedrate / extrusion / retraction /
  toolchanges and has no nozzle-identity decision point; `nozzle_diameter`
  itself travels via module `extensions`/raw config, not a `ResolvedConfig`
  field.
- The port has **no GCodeProcessor equivalent**: no post-export warning list
  (`SliceEventCollector` has degraded counters/flag only — the ticket-102 fog
  item), no Elegoo dialect seam, no remaining-volume tracking.
- Three keys are per-extruder/per-filament vectors (`nozzle_type`,
  `nozzle_volume`, `required_nozzle_HRC`) and the fourth (`nozzle_hrc`) is
  compared per-extruder against per-filament requirements: full parity needs
  the per-tool axis (today `extract_float_or_first` keeps element 0 and drops
  the rest — ticket 118's finding), i.e. ticket 125's ruling.

### Authoring obligations (same as any queue ticket)

- Use `/spec-packet-generator`; the gate is `/spec-review <packet> --preflight`.
- Apply 02's parity-evidence standard; re-derive packet number from disk per
  ticket 06's rule; re-derive the owner seam per the ticket-27 lesson.
- Re-size under the "Packets are for complex implementation only" rule: if the
  warning-list seam exists by then and the work is declare-and-wire, implement
  directly in the session instead of authoring.

### Open design questions (for claim time, not now)

- **Warning vs fatal.** Canonical is explicitly a non-fatal post-export
  warning. If the port maps it onto a fatal validator (ticket-26/113
  precedent), that is a deliberate divergence needing human sign-off per
  ticket 02 — surface it first, never file the deviation silently.
- **Vendor scope of `nozzle_volume`.** Its only behavioural effect is
  Elegoo-M6211 attribution. Whether that is 03-class out-of-scope vendor work
  or a statistics feature worth building is a scoping ruling for this ticket,
  not pre-decided here.
- **Whether `GCodeProcessor` warnings satisfy Authoring rule 3** is recorded
  as settled-yes for these keys (libslic3r read sites, none in the excluded
  set); do not re-litigate without new evidence.

Resolved when all four keys are either live behind behaviour-changing decision
points with tests asserting the change at non-default values, or consciously
ruled out of scope per rule 3 with the queue count shrunk accordingly.

## Answer
