# 27 — Close P20 — Printer / Machine / Printer identity — emitter

Type: task
Status: resolved
Assignee: wayfinder session (2026-09-03)
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P20 — Printer / Machine / Printer identity — emitter** — 2 keys, Tier A plumbing, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P20 — Printer / Machine / Printer identity — emitter):

`printer_model`, `printer_structure`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Closed by direct implementation — no packet**, under the map's "Packets are for
complex implementation only" rule. Sized from the tree at claim time: one key was
already live, and the other needed one enum declaration plus one gate.

### `printer_structure` — implemented directly

**The tier table's owner was wrong.** `04-asset-tier-assignment.md` assigned both
keys to `crates/slicer-gcode` ("printer_technology in serialize.rs"); the key has
no emitter behaviour at all. Its canonical footprint is entirely time-lapse
machinery in `GCode::process_layer` (`GCode.cpp`):

- `m_timelapse_warning_code` bits — psI3 + spiral vase, psI3 + by-object print
  sequence. GUI diagnostics; nothing to port.
- the `m_farthest_point_timelapse.enabled` fold — gated on
  `farthest_point_timelapse`, a Bambu feature (ticket 03's out-of-scope class).
- **`need_insert_timelapse_gcode_for_traditional`** —
  `(is_i3_printer && !m_spiral_vase) || is_multi_extruder`, where `is_i3_printer`
  is `printer_structure == PrinterStructure::psI3`. This decides whether
  `time_lapse_gcode` is injected at each layer change, and it is the one real,
  non-GUI, non-vendor behaviour the key carries.

This port's `time_lapse_gcode` injection site belongs to **`machine-gcode-emit`**,
not the emitter, so that is where the key is declared — an `enum` over
`undefine`/`corexy`/`i3`/`hbot`/`delta` defaulting to `undefine`
(`modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`), matching
canonical's `PrintConfigDef::init_fff_params` value list. The gate lives in
`run_gcode_postprocess` (`modules/core-modules/machine-gcode-emit/src/lib.rs`) and
suppresses the **TimeLapse template itself** rather than one emission site, so
every site observes it.

The key has no `ResolvedConfig` field, so it rides `ResolvedConfig::extensions`
through `to_config_map()` to the guest's `ConfigView` — verified end-to-end, not
inferred (see Verification).

**Three recorded divergences, filed as `DEV-168`:**

1. **`undefine` (the default) does not suppress.** Canonical injects nothing on an
   undefined single-extruder machine. This tree had no structure gate at all and
   setting `time_lapse_gcode` has always been sufficient to get injection;
   suppressing at the default would silently drop output for every existing user.
   Same shape as ticket 26's `printable_height` default ruling.
2. **The `!m_spiral_vase` clause is unwired.** This port has no spiral mode
   (`spiral_mode` is an unimplemented queue key; the gap is pinned by
   `crates/slicer-runtime/tests/arachne_parity_gaps.rs`). When spiral lands it
   must extend this gate.
3. **`is_multi_extruder` is approximated.** Canonical reads
   `nozzle_diameter.size() > 1` — a printer property. No extruder-count key
   reaches a `PostPass` module here, so the stand-in is "this print performs a
   toolchange" (`GCodeCommand::ToolChange` present in the command stream). The two
   differ on a multi-extruder printer running a single-tool print.

### `printer_model` — already covered, no code change

The key is already user-settable and already drives a live decision point:
`serialize_config_block` (`crates/slicer-gcode/src/serialize.rs`) synthesizes
`Generic PNP Printer` **only** when `raw_config` lacks the key, so a user- or
fork-supplied value wins through the `emit_config_kv` dedup path. User config keys
reach `raw_config` verbatim (`config_source` → `run_pipeline_with_raw_config`), so
no plumbing is missing. Both arms are already pinned by
`config_block_synthesizes_non_bbl_printer_model` and
`config_block_fork_keys_never_shadowed`
(`crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`),
and the contract is documented in `docs/02_ir_schemas.md`. This is **not**
`ORCA_CONFIG_PADDING` evidence (Authoring rule 2): the key has no padding twin —
it is an active synthesis branch with a real fallback.

Canonical's remaining pipeline reads are all vendor-proprietary:
`is_bambu_x2d_printer` (`GCode.cpp`), `GCodeProcessor::s_IsBBLPrinter`, and the
Elegoo M6211 time estimate (`ElegooGCodeProcessorHelper.cpp`). Those fall in
ticket 03's Bambu-proprietary / vendor-hardware out-of-scope class; the rest are
`Preset.cpp` (preset management, 03's class) and `Format/SL1.cpp` (SLA). Nothing
in-scope is left unimplemented, so the key is **not** returned to the queue.

### Harness defect fixed in passing

`machine_start_end_gcode_emission_tdd.rs`'s schema sweep routed every non-numeric
manifest key through a `_ =>` arm that seeded `pipeline_source` with an
**empty-string sentinel** (a guard against multiline G-code templates leaking into
the CONFIG_BLOCK). An enum default is neither multiline nor unsafe, and the empty
string is not a legal enum value, so the sweep made `resolve_global_config` fail
with `TypeMismatch { key: "printer_structure", actual: "unsupported enum value
''" }` before the module ever ran. Added an explicit `"enum"` arm that routes the
real default into both sources, like int/float/bool. Any future enum key on this
module would have hit the same wall.

### Verification

- `cargo test -p machine-gcode-emit` — 33 passed, 0 failed, including the four new
  gate tests: default/`undefine` and `i3` inject; `corexy`/`hbot`/`delta` suppress
  the time-lapse site **only** (the neighbouring `layer_change_gcode` still fires);
  a toolchange in the stream restores injection under `corexy`.
- `cargo test -p slicer-runtime --test integration machine_start_end_gcode_emission`
  — 17 passed, 0 failed, including
  `printer_structure_gates_time_lapse_injection_end_to_end`, which slices the real
  fixture through the real `machine-gcode-emit` guest and asserts 0 time-lapse
  lines under `printer_structure = corexy` against a non-zero count at the default.
  That is the rule-6(b) obligation — a behaviour change asserted at a non-default
  value — and it is also what proves the key crosses the live transport.
- `cargo test -p slicer-runtime --test integration gcode_header_thumbnail` — 23
  passed, 0 failed (the `printer_model` evidence).
- `cargo test -p slicer-gcode` — all binaries green; CONFIG_BLOCK unchanged.
- `cargo xtask build-guests --check` reported stale before the run; guests were
  rebuilt (`built 46 guest(s)`, exit 0) and the integration test only went green
  after that rebuild — the first run's failure was the stale guest, not the gate.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
  `cargo xtask check-literals` — 0 violations.
  `cargo xtask gen-config-docs` — regenerated (one new module-key row).

Not run: `cargo test --workspace`. Per the repo's Test Discipline that is a
packet-closure ceremony command, and this ticket closed without a packet.

