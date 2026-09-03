# Design: 267-printer-machine-power-recovery-emitter

## Controlling Code Paths

- Primary emitter path: `DefaultGCodeEmitter::emit_gcode` in `crates/slicer-gcode/src/emit.rs` — builds the GCodeIR command stream (opening with `ExtrusionMode`, then per-layer marker triples and moves), then runs the estimator, `crate::m73::inject_m73` (gated on `if !self.resolved_config.disable_m73`), and the filament-stats comment block. The envelope and recovery commands are prepended/pushed in this function.
- Flavor path: `run_slice` (`crates/slicer-runtime/src/run.rs`) resolves `gcode_flavor` from the config source into `GcodeFlavor` and passes it to `DefaultGCodeSerializer::with_flavor`; this packet extends the same resolution to `DefaultGCodeEmitter::with_flavor`.
- Postpass path: `MachineGcodeEmit::run_gcode_postprocess` in `modules/core-modules/machine-gcode-emit/src/lib.rs` — Step 4 currently prepends the resolved `machine_start_gcode` template ahead of every input command; this packet changes the insertion point to after the leading run of non-M73 Raw commands so the host's envelope precedes the start template.
- Dialect helpers: `GcodeFlavor` in `crates/slicer-gcode/src/flavor.rs` already owns the ported `GCodeWriter.cpp` command forms (`set_acceleration`, `set_travel_acceleration`, `set_jerk_xy`, `set_junction_deviation`), pinned by `crates/slicer-gcode/tests/gcode_flavor_dialect_tdd.rs`; the envelope's M204/M205 forms are built in the emitter from the same canonical shapes (the existing helpers are per-move forms, not the envelope's M204 P/R/T line, so the envelope formats its own lines).
- Config paths: the three keys are host-consumed (`ResolvedConfig` fields) and declared in the `machine-gcode-emit` manifest for contract completeness; host-side bounds/enum enforcement is `ConfigBoundsIndex::from_modules` / `check` in `crates/slicer-scheduler/src/config_resolution.rs`.
- OrcaSlicer comparison: see `requirements.md` section `OrcaSlicer Reference Obligations`; do not repeat delegation rules here.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- Feedrate units: the envelope values are mm/s config values emitted verbatim, except RRF, where canonical multiplies the M203 and M566 values by 60 (mm/min). No coordinate-system scaling applies — these are not polygon coordinates, so the 1 unit = 100 nm hazard does not touch this packet.
- All runtime config key strings remain snake_case. The three keys are host-consumed and must be declared in the owner manifest before the config reference and bounds layer see them.
- No WIT, IR, or public schema-version change is needed. `GCodeCommand::Raw` (`crates/slicer-ir/src/slice_ir.rs`) already carries verbatim setup commands, and the serializer's Raw arm writes the text plus its own newline — so envelope/recovery Raw text must NOT carry a trailing newline (the flavor helpers' newline-terminated strings are for direct stream writes, not for Raw commands).
- `ORCA_CONFIG_PADDING` (`crates/slicer-gcode/src/serialize.rs`) and every CONFIG_BLOCK twin are **out of bounds**: not read, not edited, not asserted. Map Authoring rule 2 — the padding table is not parity evidence and is never a deliverable. AC-N3 asserts the file is untouched.
- The stream-ordering contract pinned by `machine_start_gcode_precedes_m73_and_extrusion_mode` (`modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs`) must survive: the start template stays ahead of the M73 pair and `ExtrusionMode`. The PrintStart change only adds the envelope run ahead of the start template.

## Tier Derivation

Ticket 04's rubric: **Tier A** is plumbing into a decision point that already exists; **Tier B** is new logic in an existing owner; **Tier C** is a new module at a new seam.

`disable_m73` alone is Tier A — the decision point exists (`DefaultGCodeEmitter::emit_gcode`'s M73 gate) and only the manifest declaration is missing. The other two build decision points that do not exist, both inside the existing owner (`crates/slicer-gcode`): the envelope emission and the recovery emission. No module is created and no seam is added, so Tier C does not apply. The packet is **mixed A/B** (was all-A in the tier table): `disable_m73` A, `emit_machine_limits_to_gcode` B, `enable_power_loss_recovery` B. The map's tier table needs the correction; it is listed in § Map and Ticket Updates Required and is not applied from here.

## Claims and Carriers (map rules 1 and 4)

- **No new claim is introduced.** The three keys are scalar toggles and a mode string — no algorithm-selecting enum, so rule 4's holder-only shape does not fire (the Q8 trigger test: in-module mode branching, not cross-module algorithm selection).
- **Which existing mechanism carries the new data:**
  - The three keys ride `ResolvedConfig` typed fields into the emitter (`DefaultGCodeEmitter::resolved_config`), the same channel `disable_m73` already uses. They are additionally declared in the `machine-gcode-emit` manifest so the config reference, the bounds layer, and the module's `ConfigView` see them (ticket-04's ResolvedConfig-only contract ruling).
  - The flavor rides a new `DefaultGCodeEmitter::flavor` field fed by `run_slice`'s existing `gcode_flavor` resolution — the same value the serializer already receives.
  - The envelope and recovery commands ride `GCodeCommand::Raw` in the GCodeIR command stream, so postpass modules and the serializer see them like any other setup command.
  - **No WIT change, no IR schema bump, no new module, no new claim.**

## Recorded Divergences

`DIV-267-1` through `DIV-267-7` are design-local labels for this packet only. They are **not** `docs/DEVIATION_LOG.md` IDs and must not be greped for there; per ticket 02 a log row is filed only after the human has been asked and signed off. (Note: ticket 22's answer already uses `DIV-267-A`/`DIV-267-B` as design-local labels for the support-ironing subject divergence; this packet's numbered labels are distinct and do not collide.)

- **DIV-267-1 — M201 is not emitted.** Canonical's M201 line carries per-axis `machine_max_acceleration_x/y/z/e`; PnP has none of those fields (P47 keys). `machine_max_acceleration_extruding` is canonical's M204 P source, not an M201 E substitute, so no M201 line is emitted at all rather than a fabricated partial.
- **DIV-267-2 — M204 R is omitted.** Canonical's M204 line includes R = `machine_max_acceleration_retracting` (Marlin legacy/Marlin2); PnP lacks that field (P47 key). The PnP M204 line carries P and T only.
- **DIV-267-3 — M205 J is not emitted.** Canonical emits `M205 J` from `machine_max_junction_deviation` for Marlin2; PnP lacks the field (P47 key).
- **DIV-267-4 — input shaping is not emitted.** Canonical's optional M593 block is gated on `input_shaping_emit` and the input-shaping keys; PnP has none of them.
- **DIV-267-5 — the Bambu M1003 recovery form is not emitted.** Canonical emits `M1003 S1/S0` for Bambu printers; PnP's `GcodeFlavor` has no Bambu variant, so the recovery command is Marlin2-only (M413). Adding a Bambu flavor is out of scope.
- **DIV-267-6 — `silent_mode` is returned to the queue.** Canonical reads every `machine_max_*` key as a stride-2 normal/stealth pair and `silent_mode` selects the variant; PnP's scalar `Option<f32>` fields have no variant dimension. The key is returned, not declared (see `requirements.md` § Returned to Queue).
- **DIV-267-7 — scalar values, not per-extruder maxima.** Canonical takes the maximum over the used extruders of each value; PnP's fields are scalar globals and the envelope uses them directly. The estimator already reads the same scalars, so the envelope and the estimate agree.

## Code Change Surface

- Selected approach: declare the three keys in the owner manifest; add the two new `ResolvedConfig` fields; wire the flavor into the emitter; prepend the envelope and push the recovery commands in `emit_gcode`; change the postpass PrintStart insertion point to after the leading non-M73 Raw run; pin everything with invariant tests.
- Envelope construction (in the emitter): when `emit_machine_limits_to_gcode` is set and the flavor is Marlin/Marlin2/RepRapFirmware, build Raw commands in canonical order — M203 from `machine_max_speed_x/y/z/e` (RRF × 60), M204 from `machine_max_acceleration_extruding` (P) and `machine_max_acceleration_travel` (T; Marlin legacy substitutes extruding for T), M205 from `machine_max_jerk_x/y/z/e` (RRF emits M566 × 60) — each group emitted when at least one contributing field is `Some`, with only the configured axes present, in canonical axis order. The envelope is prepended to the command stream before `inject_m73` runs, so the stream order is [envelope, M73 P0/Q0, ExtrusionMode, ...].
- Recovery construction (in the emitter): at the start of the second emitted layer (the `emitted_layer_count == 1` layer, after its marker triple), push `M413 S1` when the mode is `"enable"` and `M413 S0` when `"disable"`, Marlin2 only; after the last layer's commands, push `M413 S0` when the mode is `"enable"`, Marlin2 only. `"printer_configuration"` and all other flavors emit nothing.
- PrintStart rule (in the module): scan the input stream from index 0; skip the leading run of `GCodeCommand::Raw` commands whose text does not start with `M73`; emit the resolved start template at the first non-skippable command. With no envelope the leading run is empty and the template still opens the stream (current behaviour, pinned by the existing test); with the envelope the template lands after it.
- Exact functions, traits, manifests, tests, and fixtures:
  - `DefaultGCodeEmitter` struct, `new`, `new_with_config`, a new `with_flavor` builder, and `emit_gcode` in `crates/slicer-gcode/src/emit.rs`.
  - `[config.schema]` in `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`.
  - `MachineGcodeEmit::run_gcode_postprocess` Step 4 in `modules/core-modules/machine-gcode-emit/src/lib.rs`.
  - The `cli` macro invocation, `to_config_map`, `impl PartialEq`, and `impl Hash` in `crates/slicer-ir/src/resolved_config.rs`.
  - `run_slice`'s emitter construction in `crates/slicer-runtime/src/run.rs`.
  - `docs/config/host-keys.toml` `[resolved_config]` and `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs` (`resolved_bool` / `resolved_str` arms).
  - `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_config_schema_tdd.rs` (net-new, auto-discovered) and `machine_gcode_emit_tdd.rs` (envelope ordering case).
  - `crates/slicer-gcode/tests/gcode_emit_tdd.rs` (envelope and recovery invariants).
  - `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` (bounds arm).
  - Generated `docs/15_config_keys_reference.md` via `cargo xtask gen-config-docs`.
- Rejected alternatives and reasons:
  - Emit the envelope from the serializer (`ThumbnailAwareSerializer` string post-processing): rejected — the envelope is part of the command stream in canonical (`GCode::_do_export`), and keeping it in the GCodeIR makes it visible to postpass modules and testable as commands rather than as output-text surgery.
  - Keep the PrintStart rule as "prepend ahead of every command" and accept the envelope after the start template: rejected — canonical emits the envelope before the start gcode; the module change is small and pinned by tests.
  - Emit M201 E from `machine_max_acceleration_extruding`: rejected — that field is canonical's M204 P source, not the M201 E source; conflating two canonical keys would be fabrication (DIV-267-1).
  - Add a Bambu flavor or input-shaping keys to reach full canonical parity: rejected — both are out of this packet's scope and are recorded as divergences with their owning work named.
  - Declare `silent_mode` with a no-op read: rejected by Authoring rule 1 — a declaration-only key is prohibited; the key is returned to the queue (DIV-267-6).

## Files in Scope (read + edit)

- `crates/slicer-gcode/src/emit.rs` - role: host emitter; expected change: `flavor` field + `with_flavor`, envelope prepend, recovery pushes, and the `emit_gcode` ordering.
- `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` - role: owner manifest; expected change: three new `[config.schema]` tables.
- `modules/core-modules/machine-gcode-emit/src/lib.rs` - role: postpass PrintStart insertion; expected change: insert after the leading non-M73 Raw run.
- `crates/slicer-ir/src/resolved_config.rs` - role: typed config; expected change: two new `cli` fields plus `to_config_map` / `PartialEq` / `Hash` entries.
- `crates/slicer-runtime/src/run.rs` - role: flavor wiring; expected change: `.with_flavor(flavor)` on the emitter construction.
- `docs/config/host-keys.toml` and `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs` - role: host-key documentation/lock; expected change: two rows + two lock arms.
- `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_config_schema_tdd.rs` (net-new), `machine_gcode_emit_tdd.rs`, `crates/slicer-gcode/tests/gcode_emit_tdd.rs`, `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` - role: tests; expected change: schema guard, ordering case, envelope/recovery invariants, bounds arm.
- `docs/15_config_keys_reference.md` - generated output only; changed through `cargo xtask gen-config-docs`.

## Read-Only Context

- `crates/slicer-gcode/src/emit.rs` - `DefaultGCodeEmitter` struct and builders, `emit_gcode` head (ExtrusionMode open), the layer loop (marker triple, `emitted_layer_count`), and the estimator/M73 tail only.
- `crates/slicer-gcode/src/flavor.rs` - the `GcodeFlavor` enum and its dialect helpers, for the envelope's flavor gate and command shapes.
- `crates/slicer-gcode/src/m73.rs` - `inject_m73`'s head-pair insertion, to confirm the envelope lands ahead of the M73 pair.
- `modules/core-modules/machine-gcode-emit/src/lib.rs` - `run_gcode_postprocess` Step 4 and the injection registry only.
- `crates/slicer-ir/src/resolved_config.rs` - the `cli` macro invocation, `to_config_map`, `PartialEq`, and `Hash` regions only.
- `crates/slicer-scheduler/src/config_resolution.rs` - `ConfigBoundsIndex::from_modules` / `check` machinery only.
- `docs/03_wit_and_manifest.md` and `docs/15_config_keys_reference.md` - targeted ranges or delegated summaries only.
- `OrcaSlicerDocumented/...` - delegated canonical inspection only.

## Out-of-Bounds Files

- `crates/slicer-gcode/src/serialize.rs` - fully out of bounds (`ORCA_CONFIG_PADDING` and CONFIG_BLOCK twins; map rule 2; AC-N3).
- `crates/slicer-ir/src/feedrate.rs`, `crates/slicer-gcode/src/estimator.rs` - the estimator's machine-limit reads are unchanged.
- `docs/ORCA_CONFIG_REFERENCE.md` - hand-maintained source snapshot, untouched.
- `docs/specs/orca-feature-gap/map.md` and `docs/specs/orca-feature-gap/issues/**` - the map/ticket updates are listed in § Map and Ticket Updates Required and applied by the authoring session, never from inside the packet.
- `target/`, `Cargo.lock`, generated code, vendored dependencies, and unrelated crates - never load directly.

## Expected Sub-Agent Dispatches

- Question: confirm the exact `emit_gcode` insertion points — where the command list is complete before `inject_m73`, and where the second-emitted-layer marker triple is pushed; scope: `crates/slicer-gcode/src/emit.rs`; return: `LOCATIONS`; purpose: Step 3.
- Question: confirm the `cli` macro's field forms and the exact `to_config_map` / `PartialEq` / `Hash` regions to extend; scope: `crates/slicer-ir/src/resolved_config.rs`; return: `LOCATIONS`; purpose: Step 2.
- Question: confirm the scheduler bounds binary loads the real machine-gcode-emit manifest and quote the existing enum/bool rejection assertion shape; scope: `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` and `crates/slicer-scheduler/src/config_resolution.rs`; return: `FACT`; purpose: Step 5.
- Question: confirm the canonical M204 gating and the exact M203/M205 RRF factor application; scope: delegated `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` `print_machine_envelope`; return: `SUMMARY` (≤ 200 words); purpose: parity evidence, only if a worker disputes `requirements.md`'s table.
- Question: run each verification command; scope: as listed in `requirements.md` § Verification Commands; return: `FACT` pass/fail; purpose: per-step exits.

## Data and Contract Notes

- IR/manifest contracts: the three keys are host-consumed `ResolvedConfig` fields; the manifest declarations make them visible to the module's `ConfigView`, the config reference, and the bounds layer. The module itself does not read them.
- WIT boundary: none. The envelope and recovery commands are `GCodeCommand::Raw` in the existing GCodeIR.
- Determinism: the envelope and recovery commands are pure functions of the resolved config and the flavor; no iteration order or timing input. The recovery fires on the second *emitted* layer (the `emitted_layer_count == 1` layer), so empty layers do not shift it.
- Raw text: the serializer's Raw arm writes the text verbatim plus its own newline; envelope/recovery Raw text must not carry a trailing newline.

## Locked Assumptions and Invariants

- `emit_machine_limits_to_gcode` defaults to `true` (canonical) and `enable_power_loss_recovery` defaults to `"printer_configuration"` (canonical); with all machine-limit fields `None`, the default path emits no envelope and no recovery line — byte-identical output.
- The envelope precedes `machine_start_gcode`, the M73 pair, and `ExtrusionMode`; the start template still precedes the M73 pair and `ExtrusionMode` when no envelope is present.
- The recovery enable fires exactly once (second emitted layer) and the recovery disable fires exactly once (end of stream, `enable` mode only).
- No P47 machine-limit field is added by this packet; AC-1 asserts the manifest does not declare them.
- No `silent_mode` occurrence is added anywhere in `crates/` or `modules/`; AC-N2 asserts it.
- No WIT/IR/schema-version change occurs, so there is no struct-literal blast radius beyond the two new `ResolvedConfig` fields (the `cli` macro generates the struct and `Default`; the tree's `ResolvedConfig { .. }` literals use functional update, verified at authoring time — re-derive the count at point of use).

## Risks and Tradeoffs

- The envelope is a partial subset of canonical's (DIV-267-1..4): a user who sets only the P47 fields will see no M201/M205 J/M593 lines. Bounded, named, and tested for what it does claim; the packet does not claim byte parity with canonical's full envelope.
- The PrintStart rule ("after the leading non-M73 Raw run") encodes knowledge of the host's envelope shape in the module. It is a small, documented rule pinned by tests; the alternative (serializer string surgery) was rejected as worse.
- The recovery's "second emitted layer" differs from canonical's "second layer" only when layer 1 produces no output; the divergence is bounded and noted in the design.
- Manifest and module source edits feed guest WASM artifacts; stale artifacts can make otherwise correct module tests fail until the required guest check/rebuild runs.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 3, the emitter behaviour plus invariant tests)
- Highest-risk dispatch and required return format: the `emit_gcode` insertion-point confirmation, `LOCATIONS`; the canonical M204 gating dispute check, `SUMMARY`.

## Open Questions

- `[FWD]` The exact canonical gating of the M204 group (whether it fires on `machine_max_acceleration_travel` being configured for all flavors, with Marlin legacy substituting extruding for T) is captured in `requirements.md`'s evidence table from the authoring-time delegated read. If a worker disputes it, re-dispatch the canonical read before finalizing the M204 gate; the ACs pin the observable PnP forms either way.
- `[FWD]` P47 (ticket 54) owns the missing machine-limit fields; when it lands per-axis acceleration, retracting acceleration, and junction deviation, this packet's envelope should grow the M201, M204 R, and M205 J groups. No activation blocker — the change is additive in the same builder.
- `[FWD]` The `silent_mode` follow-up (ticket 117) needs a per-variant machine-limit model; when P47 widens the fields, the variant selection should be revisited.

**No `[BLOCK]`.** The packet needs no new WIT interface, no IR schema bump, and no new module: the envelope and recovery ride `GCodeCommand::Raw` in the existing GCodeIR, the flavor rides an existing resolution, and the three keys ride `ResolvedConfig` plus the owner manifest.

## Map and Ticket Updates Required

Listed only; **not applied by this packet** (the map and tickets are out of bounds for the packet; the authoring session applies them).

1. **Tier correction.** The map's P18 entry and ticket 04's tier table carry P18 as all-Tier-A. It is **mixed A/B**: `disable_m73` A, `emit_machine_limits_to_gcode` B, `enable_power_loss_recovery` B.
2. **Coverage-count correction.** P18 covers **3** keys, not 4. `silent_mode` is returned to the queue as unimplemented, needs a per-variant machine-limit model; the tier row and the packet-list entry are updated and the follow-up is filed as ticket 117.
3. **Stale P14/P15 map wording.** The map's ticket-21 entry and the fog entry still say the P14 packet uses a "deterministic layer-index fallback" / "zero-degree base plus a layer-index turn"; packet 266's final design reads `infill_direction` (DIV-266-A withdrew the layer-index turn). The fog entry's "Packet 267 records this (DIV-267-A/DIV-267-B)" for the support-ironing subject divergence is stale — packet 267 is P18, and that divergence is recorded in ticket 22's answer. Both are corrected in the map.
4. **Ticket 25 closure.** The authoring ticket is marked resolved with the packet directory linked.
