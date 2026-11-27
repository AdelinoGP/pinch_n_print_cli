# 47 — Author packet P40 — Multimaterial / Flush options — emitter

Type: task
Status: resolved
Assignee: wayfinder session (ses_f8b0a4a63ffeJCTh2GTtNdfmbN) — claimed 2026-09-06, resolved 2026-09-06
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P40 — Multimaterial / Flush options — emitter** — 2 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P40 — Multimaterial / Flush options — emitter):

`filament_flush_temp`, `filament_flush_volumetric_speed`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Re-sized at claim time: closed by direct implementation, no packet** (the
ticket-36/37/40/42/46 shape, under the map's "Packets are for complex
implementation only" rule — the title's "Author packet" is a rotted ledger
fact, not an instruction). Both keys are placeholder-only in canonical with
zero tree occurrences, and the decision point they drive already exists, so a
spec packet would be dead-weight ceremony on two declare-and-wire keys.

Canonical grounding (oracle `D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented`,
file + function only): `filament_flush_temp` (`PrintConfig.cpp` defaults:
`coInts` nullable, default 0, min 0, max `max_temp` = 1500 — "0 indicates the
upper bound of the recommended nozzle temperature range") and
`filament_flush_volumetric_speed` (`coFloats` nullable, default 0, min 0, max
200 — "0 indicates the max volumetric speed") are read per filament at every
`GCode.cpp` toolchange/placeholder site
(`update_placeholder_parser_with_variant_params`, `set_extruder`, and the
toolchange placeholder builder) via `get_at`, with `0` falling back to
`nozzle_temperature_range_high` / `filament_max_volumetric_speed` (both Tier D
deferred), and published as the derived `flush_temperatures` /
`flush_volumetric_speeds` placeholders for custom G-code. The fast-purge
branch (`filament_flush_temp_fast`, `flush_multiplier_fast`,
`prime_volume_mode`) is explicitly out of the queue (05's P23 note). Both keys
live — in scope.

Tree state at claim: zero occurrences in `crates/` / `modules/` / `xtask/`.
The tier table's `crates/slicer-gcode` owner is wrong (ticket-27 hazard):
canonical's reads are all placeholder publication, and this tree's
substitution seam is the generic sweep in `run_gcode_postprocess`
(`modules/core-modules/machine-gcode-emit/src/lib.rs`), which publishes every
declared int/float key — no key-specific arm needed (int/float render
canonically via `format_placeholder_value`, unlike 276's word-form-bool
guarantee). Owner re-derived to `machine-gcode-emit`.

Change (6 files, no packet number taken):
- `crates/slicer-ir/src/resolved_config.rs` — scalar-global `cli` fields
  `filament_flush_temp: u32 = 0` / `filament_flush_volumetric_speed: f32 =
  0.0` (canonical defaults; per-tool via the `tool_config:<idx>:` axis,
  vector ingest rides 125) + `to_config_map` inserts (required — unlike the
  host-only P35 pair, the module placeholder path reads the map) + `PartialEq`
  / `Hash` arms.
- `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` —
  `[config.schema.filament_flush_temp]` (int, 0, 0–1500) +
  `[config.schema.filament_flush_volumetric_speed]` (float, 0.0, 0.0–200.0);
  bounds mirror canonical (port enforces where canonical hints — ticket 113
  rule). No `lib.rs` change: the sweep substitutes both keys into every
  template (`change_filament_gcode`, machine start/end) once declared.
- Tests (6): `flush_keys_config_schema_tdd.rs` manifest guard (type/default/
  bounds per key); 2 placeholder-render cases in `machine_gcode_emit_tdd`
  (non-default substitution at a `ToolChange` site, default 0/0.0 inert
  identity); 2 `ResolvedConfig` default + round-trip cases in
  `resolved_config_defaults_tdd`.
- `DEV-171` (`docs/DEVIATION_LOG.md`): scalar-not-vector, 0-with-no-fallback,
  raw-names-not-plural-derived-names. `docs/15_config_keys_reference.md`
  regenerated (2 key rows; deviation count 26).
- 04 tier rows → live with the re-derived owner; 05 P40 → closed-direct.

Gates: `machine-gcode-emit` full suite green (2 schema + 35 harness incl. 2
new + binding), `slicer-ir` full green (incl. 2 new), `slicer-gcode` 17
binaries green (no CONFIG_BLOCK fallout — defaults inert, default templates
empty), ticket-46's 6 runtime selection tests green after a 2-line
`..Default::default()` cleanup of its new test literals (clippy
`needless_update` — that dirty work is ticket 46's, the fix is noted here so
its commit session doesn't rediscover it); `cargo check --workspace
--all-targets` clean; `cargo clippy --workspace --all-targets -- -D warnings`
clean; `cargo xtask check-literals` clean; `gen-config-docs --check` clean;
all 46 guests rebuilt (`build-guests --check` exit 0). Pre-existing reds
noted, untouched: `check-deviations --check` fails on the clean tree too
(doc-07 map out of sync — ticket-42 precedent).
