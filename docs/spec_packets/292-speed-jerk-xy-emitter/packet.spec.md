---
status: draft
packet: 292-speed-jerk-xy-emitter
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/66-author-packet-p59-speed-jerk-xy-emitter.md (wayfinder map: Close the OrcaSlicer FFF feature gap)
context_cost_estimate: M
copy_note: Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; authored under ticket 66 (P59).
---

# Packet Contract: 292-speed-jerk-xy-emitter

## Goal

Make the P59 jerk family drive host-side emission at parity with canonical `GCode::_extrude` role-gated jerk plus `GCode::travel_to` travel jerk and the first-layer junction-deviation arm — ported as a per-entity jerk-selection stage inside `crates/slicer-gcode` reusing the existing `GcodeFlavor::set_jerk_xy` / `set_junction_deviation` helpers, with no new module, IR field, or WIT change.

## Scope Boundaries

P59 is eight Tier B keys owned by the host emitter (`crates/slicer-gcode`). Claim-time grounding (ticket 66) holds all eight in: every key is live in canonical's slicing pipeline (no dead key, no alias) and zero-occurrence as behaviour in this tree — `GcodeFlavor::set_jerk_xy` / `set_junction_deviation` exist as unwired builders (`crates/slicer-gcode/src/flavor.rs`) with no call site feeding them a config value, and `DefaultGCodeEmitter::emit_gcode` emits no jerk command today. The packet declares all eight scalar-global (DEV-184(b): canonical declares all eight as per-nozzle nullable vectors `ConfigOptionFloatsNullable` read via `NOZZLE_CONFIG`; the vector model stays with ticket 125) and builds the selection stage. Defaults are identity — canonical's `default_jerk` is `0`, so the master gate leaves the default stream byte-identical; that is pinned by AC-2 (the inverse of packet 289's emitting default). Keys stay host-only omitted from the CONFIG_BLOCK (ticket-42 precedent; zero padding twins exist — verified at authoring — table untouched). First-layer junction deviation is the one deliberate port extension point: canonical's arm reads a Marlin-only `default_junction_deviation > 0` off the same first-layer header, rendered here through the existing Marlin2-only `set_junction_deviation` (DEV-184(c)).

## Prerequisites and Blockers

- Depends on: draft packet 281 (`docs/spec_packets/281-machine-motion-limits-emitter/`, `status: draft`) for stream position only — explicit FORWARD-DEP, not a satisfied dependency. 281's envelope opens the stream (its `M205` machine-limit line ahead of the start block); this packet's per-path `M205` lines follow it. Shared code is reuse, not a dep: the existing `GcodeFlavor::set_jerk_xy` / `set_junction_deviation` arms are called, never forked. Activation sequences after 281 lands.
- Related work, not a blocker: draft packet 289 (`docs/spec_packets/289-speed-acceleration-emitter/`, `status: draft`) built the adjacent per-entity `M204` accel stage with per-stream change-dedup; this packet borrows the dedup shape for jerk but shares no helper and no ordering edge — no FORWARD-DEP on 289.
- Unblocks: wayfinder ticket 66 (P59 closes with all eight in). No edge to any other draft packet.
- Activation blockers: none. All symbols below are live on HEAD (verified at authoring); DEV-184 is the next collision-free ID (LOG max DEV-171; drafts claim DEV-172–DEV-183 — re-derive `max(DEV-*)` over `docs/DEVIATION_LOG.md` + `docs/spec_packets/*/` before writing the row).

## Acceptance Criteria

- **AC-1. Given** a default config, **when** the emitter schema is inspected, **then** all eight keys are declared as scalar-global with canonical defaults — `default_jerk` float `0.0` (`min 0`), `default_junction_deviation` float `0.0` (`min 0`, `max 0.3`), `infill_jerk` float `9.0` (`min 0`), `initial_layer_jerk` float `9.0` (`min 0`), `inner_wall_jerk` float `9.0` (`min 0`), `outer_wall_jerk` float `9.0` (`min 0`), `top_surface_jerk` float `9.0` (`min 0`), `travel_jerk` float `12.0` (`min 0`) — first-wins scalar ingest of Orca vector spellings (DEV-184(b), ticket-140 precedent) with matching `docs/config/host-keys.toml` `[resolved_config]` rows. | `cargo test -p slicer-gcode --test speed_p59_jerk_emission_tdd schema_declares_all_eight_keys 2>&1 | tee target/test-output.log | tail -5`
- **AC-2. Given** default config under flavor `marlin2`, **when** a mixed-role fixture (outer/inner walls, sparse, top-solid, one travel) is emitted, **then** the stream is byte-identical to the pre-packet stream — no `M205` / `M207` / `M566` / `SET_VELOCITY_LIMIT` line appears (canonical master gate `default_jerk > 0` is closed at `0`; the role jerks never fire on their own), and the CONFIG_BLOCK keeps its line count with zero value changes (host-only omitted, no padding twin shadowed). | `cargo test -p slicer-gcode --test speed_p59_jerk_emission_tdd default_stream_is_byte_identical 2>&1 | tee target/test-output.log | tail -5`
- **AC-3. Given** `default_jerk = 8` with each role jerk set to a distinct non-default value, **when** the mixed-role fixture is emitted, **then** selection follows the canonical precedence — layer-0 print paths use `initial_layer_jerk` when `> 0` (`global_layer_index == 0`); else `OuterWall`/`ThinWall` use outer, `InnerWall` uses inner, `TopSolidInfill` uses top-surface, `SparseInfill` uses infill; `BottomSolidInfill`, `GapFill`, `BridgeInfill`/`InternalBridgeInfill`, support roles, `Skirt`/`Brim` fall through to `default_jerk` (no canonical arm — recorded, not wired); any arm at `0` falls through to default. | `cargo test -p slicer-gcode --test speed_p59_jerk_emission_tdd role_precedence_and_fallthrough 2>&1 | tee target/test-output.log | tail -5`
- **AC-4. Given** `default_jerk = 8` and `travel_jerk = 12`, **when** travel fixtures are emitted, **then** non-first-layer travels carry `travel_jerk` (flavor `marlin2` renders `M205 X12 Y12` — reuse `GcodeFlavor::set_jerk_xy`, do not fork the form), first-layer travels use the configured `travel_jerk` unchanged (no `initial_layer_travel_jerk` key exists in scope — canonical's percent-over-base travel override is a named non-borrow, DEV-184(d)), and short perimeter-adjacent travels keep the `travel_jerk` value (canonical's `outer_wall_jerk` short-travel override is a named non-borrow, DEV-184(d)). | `cargo test -p slicer-gcode --test speed_p59_jerk_emission_tdd travel_jerk_selection 2>&1 | tee target/test-output.log | tail -5`
- **AC-5. Given** `default_jerk = 8` with distinct role jerks, **when** the fixture is emitted under each flavor, **then** print jerk uses `GcodeFlavor::set_jerk_xy`'s per-flavor form (`marlin`/`marlin2`/`reprapfirmware` → `M205 X… Y…`, `repetier` → `M207 X…`, `klipper` → `SET_VELOCITY_LIMIT SQUARE_CORNER_VELOCITY=…`), identical values are change-deduped per stream (no repeat line when the selected jerk equals the last-emitted print jerk; travels dedup on their own key), and no `M566` line is emitted on any flavor (no `M566` form exists in `GcodeFlavor` — machine-max jerks are 281's envelope, not this packet). | `cargo test -p slicer-gcode --test speed_p59_jerk_emission_tdd flavor_forms_and_dedup 2>&1 | tee target/test-output.log | tail -5`
- **AC-6. Given** `default_jerk = 8` with `default_junction_deviation = 0.05` under flavor `marlin2`, **when** a first-layer print path is emitted, **then** the stream carries both the selected jerk line and `M205 J0.05` (reuse `GcodeFlavor::set_junction_deviation` — Marlin2-only, returns `None` elsewhere so other flavors emit jerk only); at `default_junction_deviation = 0` no `J` line appears on any layer; on later layers no `J` line appears at any value. | `cargo test -p slicer-gcode --test speed_p59_jerk_emission_tdd first_layer_junction_deviation 2>&1 | tee target/test-output.log | tail -5`

## Negative Test Cases

- **AC-N1. Given** out-of-range values (any jerk `-1`, `default_junction_deviation = 0.31` or a non-numeric spelling), **when** the slice is validated, **then** each is rejected with a stable bounds error naming the key (ticket-113 reject-the-slice rule; canonical mins/maxes are GUI hints — DEV-184(a)). | `cargo test -p slicer-gcode --test speed_p59_jerk_emission_tdd out_of_range_values_reject 2>&1 | tee target/test-output.log | tail -5`
- **AC-N2. Given** all seven role/travel jerks at non-default values with `default_jerk = 0` (and `default_junction_deviation = 0`), **when** the fixture is emitted, **then** no `M205` / `M207` / `SET_VELOCITY_LIMIT` line appears anywhere in the output (canonical master gate — the whole stage is inert). | `cargo test -p slicer-gcode --test speed_p59_jerk_emission_tdd zero_default_disables_all_jerk_emit 2>&1 | tee target/test-output.log | tail -5`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-gcode --test speed_p59_jerk_emission_tdd 2>&1 | tee target/test-output.log | tail -5`

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (modular pipeline + community extensibility constraint on the emitter-stage shape)
- `docs/01_system_architecture.md` - delegated SUMMARY (claim system section only — jerk scales no E and selects no algorithm; rule 4 trigger test does not fire: in-module emission parameter, not cross-module selection)

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` section "Host keys" - `rg -q 'travel_jerk' docs/15_config_keys_reference.md` (regenerated by `cargo xtask gen-config-docs` in Step 1b; freshness pinned by `cargo xtask gen-config-docs --check` in Step 4)
- `docs/config/host-keys.toml` section "[resolved_config]" - `rg -q 'travel_jerk' docs/config/host-keys.toml`
- `docs/DEVIATION_LOG.md` section "DEV-184" - `rg -q 'DEV-184' docs/DEVIATION_LOG.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the eight keys' declared types/defaults/bounds (confirm `default_jerk` 0 / `default_junction_deviation` 0 max 0.3 / the five role jerks 9 / `travel_jerk` 12, all `coFloats` per-nozzle nullable vectors; do not re-derive the packet's table without this read)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::_extrude` jerk-selection chain (precedence order first-layer → outer → inner → top-surface → infill → default, the `> 0` fallthrough per arm, and the `default_jerk > 0` master gate) plus `GCode::process_layer` first-layer header (the `initial_layer_jerk` emit-and-restore and the Marlin-only `default_junction_deviation` arm) (borrow the order and gates; port the selection emitter-side onto `entity.path.role` + `global_layer_index`, pure per-entity — no stateful restore)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::travel_to` travel-jerk selection (`travel_jerk` default, `initial_layer_travel_jerk` percent-over-base first-layer override via `get_abs_value_at`, short-travel `outer_wall_jerk` override near external/overhang perimeters, all gated on `default_jerk > 0`) plus `GCode::_do_export` Calib-PA-line use of `outer_wall_jerk` (borrow the default only; the two overrides and the Calib arm are named non-borrows — see `requirements.md`)
- `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` — `GCodeWriter::set_jerk_xy` per-flavor forms (Marlin-family `M205 X/Y`, Repetier `M207 X`, Klipper `SET_VELOCITY_LIMIT SQUARE_CORNER_VELOCITY`) and `GCodeWriter::set_junction_deviation` (Marlin-only `M205 J` under `m_max_junction_deviation > 0`) (borrow the forms; the port's existing `GcodeFlavor` arms already match — reuse, do not fork)
- `OrcaSlicerDocumented/src/libslic3r/Print.cpp` — `Print::validate` jerk checks (the `== 1` artifact warning on default/outer/inner, the machine-max exceedance list, the Marlin+JD `ignore_jerk_validation` skip) (cite as the warn-only evidence for DEV-184(a); borrow nothing — the port rejects per ticket 113)

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
