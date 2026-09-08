---
status: draft
packet: 289-speed-acceleration-emitter
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/63-author-packet-p56-speed-acceleration-emitter.md (wayfinder map: Close the OrcaSlicer FFF feature gap)
context_cost_estimate: M
copy_note: Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; authored under ticket 63 (P56).
---

# Packet Contract: 289-speed-acceleration-emitter

## Goal

Make the P56 acceleration family drive host-side emission at parity with canonical `GCode::_extrude` role-gated acceleration plus `GCode::travel_to` travel acceleration — ported as a per-entity acceleration-selection stage inside `crates/slicer-gcode` reusing the existing `GcodeFlavor` M204/SET_VELOCITY_LIMIT helpers, with no new module, IR field, or WIT change.

## Scope Boundaries

P56 is eleven Tier B keys owned by the host emitter (`crates/slicer-gcode`). Claim-time grounding (ticket 63) holds all eleven in: every key is live in canonical's slicing pipeline (no dead key, no alias) and zero-occurrence as behaviour in this tree — `GcodeFlavor::set_acceleration` / `set_travel_acceleration` exist but no call site feeds them a config value, and `emit_gcode` emits no accel command today. The packet declares all eleven scalar-global (DEV-181(b): canonical declares the nine accel keys per-nozzle nullable vectors and the three infill/bridge keys as percents over a base; the vector model stays with ticket 125) and builds the selection stage plus the Klipper `ACCEL_TO_DECEL` suffix. Defaults are NOT identity — canonical's `default_acceleration` is `500 > 0`, so the default path newly emits M204 lines; that is the intended default output change, pinned by AC-2. Keys stay host-only omitted from the CONFIG_BLOCK (ticket-42 precedent; zero padding twins exist — verified at authoring — table untouched).

## Prerequisites and Blockers

- Depends on: draft packet 281 (`docs/spec_packets/281-machine-motion-limits-emitter/`, `status: draft`) for stream position only — explicit FORWARD-DEP, not a satisfied dependency. 281's envelope opens the stream (`M201 → M203 → M204 P/R/T → M205` ahead of the start block); this packet's per-path M204 lines follow it. Shared code is reuse, not a dep: the existing `GcodeFlavor::set_acceleration` / `set_travel_acceleration` arms are called, never forked. Activation sequences after 281 lands.
- Unblocks: wayfinder ticket 63 (P56 closes with all eleven in). No edge to any other draft packet: P59 (jerk, ticket 66, still open) reuses the same flavor helpers later under canonical names kept stable here.
- Activation blockers: none. All symbols below are live on HEAD (verified at authoring); DEV-181 is the next collision-free ID (LOG max DEV-171; drafts claim DEV-172–DEV-180 — re-derive `max(DEV-*)` over `docs/DEVIATION_LOG.md` + `docs/spec_packets/*/` before writing the row).

## Acceptance Criteria

- **AC-1. Given** a default config, **when** the emitter schema is inspected, **then** all eleven keys are declared as scalar-global with canonical defaults — `accel_to_decel_enable` bool `true`, `accel_to_decel_factor` float `50.0` (`min 1`, `max 100`), `default_acceleration` float `500.0` (`min 0`), `initial_layer_acceleration` float `300.0` (`min 0`), `inner_wall_acceleration` float `10000.0` (`min 0`), `outer_wall_acceleration` float `500.0` (`min 0`), `top_surface_acceleration` float `500.0` (`min 0`), `travel_acceleration` float `10000.0` (`min 0`), `bridge_acceleration` float-or-percent `50%` over `outer_wall_acceleration` (`min 0`), `sparse_infill_acceleration` float-or-percent `100%` over `default_acceleration` (`min 0`), `internal_solid_infill_acceleration` float-or-percent `100%` over `default_acceleration` (`min 0`) — with matching `docs/config/host-keys.toml` `[resolved_config]` rows. | `cargo test -p slicer-gcode --test speed_p56_accel_emission_tdd schema_declares_all_eleven_keys 2>&1 | tee target/test-output.log | tail -5`
- **AC-2. Given** default config under flavor `marlin2`, **when** a mixed-role fixture (outer/inner walls, sparse, top-solid, one travel) is emitted, **then** print paths carry `M204 P500` and travels carry `M204 T10000` (the intended default output change — the pre-packet stream has no M204 at all), and the CONFIG_BLOCK keeps its line count with zero value changes (host-only omitted, no padding twin shadowed). | `cargo test -p slicer-gcode --test speed_p56_accel_emission_tdd default_path_emits_canonical_accel 2>&1 | tee target/test-output.log | tail -5`
- **AC-3. Given** each role accel set to a distinct non-default value with `default_acceleration = 500`, **when** the mixed-role fixture is emitted, **then** selection follows the canonical precedence — layer-0 print paths use `initial_layer_acceleration` when `> 0` (`global_layer_index == 0`); else `BridgeInfill`/`InternalBridgeInfill` use bridge, `SparseInfill` uses sparse, `InternalSolidInfill` uses internal-solid, `OuterWall`/`ThinWall` use outer, `InnerWall` uses inner, `TopSolidInfill` uses top-surface; `BottomSolidInfill`, `GapFill`, support roles, `Skirt`/`Brim` fall through to `default_acceleration` (no canonical arm — recorded, not wired); any arm at `0` falls through to default. | `cargo test -p slicer-gcode --test speed_p56_accel_emission_tdd role_precedence_and_fallthrough 2>&1 | tee target/test-output.log | tail -5`
- **AC-4. Given** `bridge_acceleration = "50%"` with `outer_wall_acceleration = 800`, `sparse_infill_acceleration = "100%"` with `default_acceleration = 500`, and absolute spellings `bridge_acceleration = 250`, **when** bridge/sparse paths are emitted, **then** the percent forms resolve against their canonical bases (400 / 500) and the absolute forms emit as-is. | `cargo test -p slicer-gcode --test speed_p56_accel_emission_tdd percent_keys_resolve_against_canonical_base 2>&1 | tee target/test-output.log | tail -5`
- **AC-5. Given** default accels, **when** the fixture is emitted under each flavor, **then** travel accel uses the separate-travel arm exactly where `GcodeFlavor::supports_separate_travel_acceleration` is true (`marlin2`/`reprapfirmware` → `M204 T…`, `repetier` → `M202 …`, `marlin`/`klipper` → no separate travel command); print accel uses `set_acceleration`'s per-flavor form (`M204 S…` / `M204 P…` / `SET_VELOCITY_LIMIT ACCEL=…` / `M201 …`). | `cargo test -p slicer-gcode --test speed_p56_accel_emission_tdd travel_accel_follows_flavor_gate 2>&1 | tee target/test-output.log | tail -5`
- **AC-6. Given** flavor `klipper` with `accel_to_decel_enable = true` and `accel_to_decel_factor = 50`, **when** a print path at accel 500 is emitted, **then** the line reads `SET_VELOCITY_LIMIT ACCEL=500 ACCEL_TO_DECEL=250`; with the bool `false` the suffix is absent at any factor. | `cargo test -p slicer-gcode --test speed_p56_accel_emission_tdd klipper_accel_to_decel_suffix 2>&1 | tee target/test-output.log | tail -5`

## Negative Test Cases

- **AC-N1. Given** out-of-range values (any accel `-1`, `accel_to_decel_factor = 0` or `101`, bool spelled `"yes"`), **when** the slice is validated, **then** each is rejected with a stable bounds error naming the key (ticket-113 reject-the-slice rule; canonical mins/maxes are GUI hints — DEV-181(a)). | `cargo test -p slicer-gcode --test speed_p56_accel_emission_tdd out_of_range_values_reject 2>&1 | tee target/test-output.log | tail -5`
- **AC-N2. Given** `default_acceleration = 0` with all other accels at non-default values, **when** the fixture is emitted, **then** no `M204` / `M201` / `SET_VELOCITY_LIMIT` line appears anywhere in the output (canonical master gate — the whole stage is inert). | `cargo test -p slicer-gcode --test speed_p56_accel_emission_tdd zero_default_disables_all_accel_emit 2>&1 | tee target/test-output.log | tail -5`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-gcode --test speed_p56_accel_emission_tdd 2>&1 | tee target/test-output.log | tail -5`

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (modular pipeline + community extensibility constraint on the emitter-stage shape)
- `docs/01_system_architecture.md` - delegated SUMMARY (claim system section only — accelerations scale no E and select no algorithm; rule 4 trigger test does not fire: in-module emission parameter, not cross-module selection)

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` section "Host keys" - `rg -q 'travel_acceleration' docs/15_config_keys_reference.md` (regenerated by `cargo xtask gen-config-docs` in Step 1b; freshness pinned by `cargo xtask gen-config-docs --check` in Step 4)
- `docs/config/host-keys.toml` section "[resolved_config]" - `rg -q 'travel_acceleration' docs/config/host-keys.toml`
- `docs/DEVIATION_LOG.md` section "DEV-181" - `rg -q 'DEV-181' docs/DEVIATION_LOG.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the eleven keys' declared types/defaults/bounds (confirm `default_acceleration` 500 / `travel_acceleration` 10000 / `inner_wall_acceleration` 10000 / `initial_layer_acceleration` 300 / `outer_wall_acceleration` 500 / `top_surface_acceleration` 500, the three percent defaults with their `ratio_over` bases, and `accel_to_decel_factor` coPercent 50 min 1 max 100; do not re-derive the packet's table without this read)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::_extrude` acceleration-selection chain (precedence order first-layer → bridge → sparse → internal-solid → outer → inner → top-surface → default, the `> 0` fallthrough per arm, and the `default_acceleration > 0` master gate) (borrow the order and gates; port the selection emitter-side onto `entity.path.role` + `global_layer_index`, pure per-entity — no stateful restore)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::travel_to` travel-acceleration selection (`travel_acceleration` default, first-layer and short-travel overhang/external-perimeter overrides, wipe-tower accel) plus `GCode::_do_export` Calib-PA-line use of `outer_wall_acceleration` (borrow the default + flavor split; short-travel overrides and the Calib arm are `[FWD]` — see `design.md`)
- `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` — `GCodeWriter::set_acceleration_internal` per-flavor forms (legacy-Marlin `M204 S`, Marlin2/RRF `M204 P`, Klipper `SET_VELOCITY_LIMIT ACCEL` + `ACCEL_TO_DECEL` under `accel_to_decel_enable`, Repetier `M201`, Bambu none) and `GCodeWriter::supports_separate_travel_acceleration` (which flavors take a separate travel command) (borrow the forms; the port's existing `GcodeFlavor` arms already match — reuse, do not fork)
- `OrcaSlicerDocumented/src/libslic3r/Print.cpp` — `Print::validate` acceleration checks (machine-max caps/warnings per role, `accel_to_decel_factor` range) (cite as the warn-and-cap-only evidence for DEV-181(a); borrow nothing — the port rejects per ticket 113)

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
