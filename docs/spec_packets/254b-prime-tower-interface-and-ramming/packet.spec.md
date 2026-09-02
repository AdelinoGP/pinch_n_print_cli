---
status: draft
packet: prime-tower-interface-and-ramming
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/09-author-packet-p02-multimaterial-prime-tower-wipe-tower.md (wayfinder map: Close the OrcaSlicer FFF feature gap — packet P02, interface half)
context_cost_estimate: L
tier: C
---

# Packet Contract: prime-tower-interface-and-ramming

Split half B of former packet 254. The geometry keys are in `254a-prime-tower-geometry-keys`, which this packet builds on.

## Goal

Build OrcaSlicer's prime-tower **interface-feature** and **ramming** behaviours so that all nine remaining P02 keys drive real decisions. Seven land inside the existing `wipe-tower` module at `PostPass::LayerFinalization`: `enable_tower_interface_features` gates an interface purge block whose depth comes from `filament_tower_interface_purge_volume`, preceded by a `filament_tower_interface_pre_extrusion_dist` travel and a `filament_tower_interface_pre_extrusion_length` lead-in extrusion, and followed — when `prime_tower_flat_ironing` is on — by an `ExtrusionRole::Ironing` pass covering `filament_tower_ironing_area` mm²; `enable_filament_ramming` prepends a ramming zigzag to each purge block. The remaining two land in a **new** `prime-tower-interface` module at `PostPass::GCodePostProcess`, which is the only stage with a command channel: it scans the emitted stream for `ExtrusionRole::WipeTower` runs and `GCodeCommand::ToolChange` boundaries and pushes `GCodeCommand::Temperature` for `filament_tower_interface_print_temp`, with `enable_tower_interface_cooldown_during_tower` choosing whether that command lands at the start of the tower run or at the preceding tool change.

## Scope Boundaries

In scope: `modules/core-modules/wipe-tower/` (manifest, `src/lib.rs`, tests), a new `modules/core-modules/prime-tower-interface/` core module with its guest crate, that module's registration surface (`crates/slicer-integrated-modules/`, `crates/slicer-runtime/`, `crates/pnp-cli/Cargo.toml`, the manifest-ingestion core-module count), one scheduler bounds arm, and the generated `docs/15_config_keys_reference.md`.

Out of scope: `prime_tower_infill_gap`, `prime_tower_brim_width`, `prime_tower_enable_framework` and the per-layer depth model (`254a` owns them); `prime_tower_skip_points` (returned to the queue by `254a`); cone / rib / fillet wall shapes (packet 255); canonical's per-filament (`coFloats` / `coInts`) parameter arrays — every `filament_tower_*` key is declared scalar-global here per ticket 04's ruling; canonical's MMU-gated multi-stage unload/load state machine; and any edit to `ORCA_CONFIG_PADDING` or a CONFIG_BLOCK padding twin.

## Prerequisites and Blockers

- **Depends on `254a-prime-tower-geometry-keys` (`status: draft`).** This is a **FORWARD-DEP, not a satisfied dependency**: `254a` builds `plan_layer_depths` and the `depth_offset` / `block_depth` parameters on `generate_purge_paths`, which every interface AC here composes with. `254a` must land first. Names and shapes are reconciled between the two packets' `design.md` files.
- Depends on: wayfinder tickets 06 (numbering), 05 (P02 key membership), 04 (the scalar-global ruling for canonical `coFloats` / `coInts` keys).
- Ordering, not gating: packet `255-wipe-tower-geometry-keys` shares the `wipe-tower` manifest.
- Unblocks: wayfinder ticket 09's resolution (jointly with `254a`).
- Activation blockers: `254a` not yet implemented.

## Acceptance Criteria

- **AC-1. Given** the `wipe-tower` manifest after this packet, **when** its `[config.schema]` is parsed, **then** it declares `254a`'s 12 keys **plus exactly seven new tables** with canonical defaults and bounds: `enable_tower_interface_features` (`bool`, `false`), `enable_filament_ramming` (`bool`, `true`), `prime_tower_flat_ironing` (`bool`, `false`), `filament_tower_interface_pre_extrusion_dist` (`float`, `10.0`, `min = 0.0`), `filament_tower_interface_pre_extrusion_length` (`float`, `0.0`, `min = 0.0`), `filament_tower_interface_purge_volume` (`float`, `20.0`, `min = 0.0`), `filament_tower_ironing_area` (`float`, `4.0`, `min = 0.0`) — 19 keys total. | `cargo test -p wipe-tower --test wipe_tower_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-2. Given** the new `prime-tower-interface` manifest, **when** it is parsed, **then** `[stage] id = "PostPass::GCodePostProcess"`, `[ir-access] reads = ["GCodeIR"]`, and `[config.schema]` declares exactly two keys: `filament_tower_interface_print_temp` (`type = "int"`, `default = -1`, `min = -1`) and `enable_tower_interface_cooldown_during_tower` (`type = "bool"`, `default = false`); and the scheduler's manifest ingestion reports **24** core modules, one more than today's 23. | `cargo test -p slicer-scheduler --test integration manifest_ingestion_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3. Given** `enable_tower_interface_features = true` and `filament_tower_interface_purge_volume = 40.0` with `prime_volume = 20.0`, **when** `run_finalization` emits a purge block, **then** that block's depth is computed from `40.0` (twice the depth the same fixture produces with the interface gate off), and with `enable_tower_interface_features = false` (default) the depth still comes from `prime_volume`. | `cargo test -p wipe-tower --test wipe_tower_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4. Given** `enable_tower_interface_features = true` and `filament_tower_interface_pre_extrusion_dist = 25.0`, **when** a purge block is emitted, **then** the block's leading travel entity (`flow_factor = 0.0`) spans exactly `25.0` mm rather than today's degenerate zero-length two-point travel; with the interface gate off the travel entity is unchanged from `254a`'s output. | `cargo test -p wipe-tower --test wipe_tower_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-5. Given** `enable_tower_interface_features = true` and `filament_tower_interface_pre_extrusion_length = 5.0`, **when** a purge block is emitted, **then** exactly one extruding lead-in entity of path length `5.0` mm (`flow_factor = 1.0`, `ExtrusionRole::WipeTower`) precedes the first scan line; at the default `0.0` no lead-in entity is emitted at all. | `cargo test -p wipe-tower --test wipe_tower_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-6. Given** `enable_tower_interface_features = true`, `prime_tower_flat_ironing = true` and `filament_tower_ironing_area = 9.0`, **when** a purge block is emitted, **then** an `ExtrusionRole::Ironing` pass follows the block's scan lines covering `9.0` mm² of the block's footprint (`ironing_span = 9.0 / tower_width` mm of depth, clamped to the block depth) as boustrophedon lines at the block's pitch; with `prime_tower_flat_ironing = false` (default) **or** with `enable_tower_interface_features = false` no `Ironing` entity is emitted — reproducing canonical's `m_flat_ironing = m_flat_ironing && m_use_gap_wall` conjunction shape. | `cargo test -p wipe-tower --test wipe_tower_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-7. Given** `enable_filament_ramming = false` (non-default; canonical's default is `true`), **when** a purge block is emitted, **then** no ramming entity precedes the block; and with `enable_filament_ramming = true` exactly one ramming entity precedes each block's scan lines — a boustrophedon zigzag over the block's leading `y_step = (prime_tower_infill_gap / 100) × line_width` band at `flow_factor = 1.0`, mirroring canonical `WipeTower::toolchange_Unload`'s use of `m_extra_spacing` as its `y_step`. | `cargo test -p wipe-tower --test wipe_tower_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-8. Given** `filament_tower_interface_print_temp = 250` and a command stream containing a `GCodeCommand::ToolChange` followed by a run of `GCodeCommand::Move { role: ExtrusionRole::WipeTower, .. }`, **when** `prime-tower-interface`'s `run_gcode_postprocess` runs with `enable_tower_interface_cooldown_during_tower = true`, **then** a `GCodeCommand::Temperature { tool, celsius: 250.0, wait: false }` is pushed immediately **before the first `WipeTower` move of the run**; with `enable_tower_interface_cooldown_during_tower = false` (default) the same command is pushed immediately **before the `ToolChange`**; and in both cases exactly one `Temperature` command is pushed per tower run. | `cargo test -p prime-tower-interface --test interface_temp_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-9. Given** `filament_tower_interface_print_temp = -1` (the default sentinel), **when** `run_gcode_postprocess` runs over the same stream, **then** **no** `Temperature` command is pushed and the command stream is returned unchanged — the port has no nozzle-temperature model to take canonical's "max nozzle temp" over, recorded as divergence D-254b-2. | `cargo test -p prime-tower-interface --test interface_temp_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-10. Given** the scheduler's bounds index built from both real manifests, **when** config resolution runs, **then** `filament_tower_interface_purge_volume = -1.0`, `filament_tower_ironing_area = -1.0`, `filament_tower_interface_pre_extrusion_dist = -1.0` and `filament_tower_interface_print_temp = -2` are each rejected by `ConfigBoundsIndex::check` with an out-of-range error naming the key, while `filament_tower_interface_print_temp = -1` (the Auto sentinel, at the declared `min`) is accepted. | `cargo test -p slicer-scheduler --test integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-11. Given** `cargo xtask gen-config-docs` has run, **when** `docs/15_config_keys_reference.md`'s generated tables are checked, **then** all nine keys appear — seven under owner `wipe-tower`, two under owner `prime-tower-interface` — and `--check` exits 0. | `cargo xtask gen-config-docs --check && rg -q 'filament_tower_interface_purge_volume' docs/15_config_keys_reference.md && rg -q 'enable_tower_interface_cooldown_during_tower' docs/15_config_keys_reference.md && rg -q 'enable_filament_ramming' docs/15_config_keys_reference.md; echo "exit=$?"`

## Negative Test Cases

- **AC-N1. Given** all nine keys left absent from the config, **when** `run_finalization` and `run_gcode_postprocess` run over a multi-toolchange fixture, **then** the emitted entities and commands are identical to `254a`'s post-packet output — no interface block, no lead-in, no ironing pass, no `Temperature` command — **except** that `enable_filament_ramming`'s canonical default is `true`, so the ramming entity **is** present by default and AC-7's `false` case is the identity-to-`254a` case. This asymmetry is deliberate and is the reason AC-7 asserts the `false` direction. | `cargo test -p wipe-tower --test wipe_tower_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N2. Given** a command stream with **no** `ExtrusionRole::WipeTower` move at all (a single-tool print), **when** `prime-tower-interface`'s `run_gcode_postprocess` runs with `filament_tower_interface_print_temp = 250`, **then** no `Temperature` command is pushed and the stream is unchanged — the module is inert when there is no tower. | `cargo test -p prime-tower-interface --test interface_temp_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N3. Given** the manifest schema guards, **when** any of the seven `wipe-tower` keys or two `prime-tower-interface` keys is removed or its `type`/`default`/`min`/`max` drifts from AC-1's / AC-2's exact tables, **then** the guard fails naming the offending key. | `cargo test -p wipe-tower --test wipe_tower_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p wipe-tower --test wipe_tower_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` and `cargo test -p prime-tower-interface --test interface_temp_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` (primary contracts), then `cargo xtask build-guests --check; echo "exit=$?"` — this packet adds a **new guest**, so `discover_guests` (`xtask/src/build_guests.rs`) must pick it up and the check must return exit 0 before closure. Exit `3` is `wasm-tools` missing, an infrastructure error, not clean.

## Authoritative Docs

- `docs/03_wit_and_manifest.md` §Host-Boundary Access Enforcement (Normative) and the stage-declaration sections — govern the new module's manifest and its `PostPass::GCodePostProcess` stage id.
- `docs/01_system_architecture.md` §Claim System — consulted to confirm the new module is stage-scheduled, not claim-held (`design.md` §Claims).
- `docs/04_host_scheduler.md` §Claim Resolution — same question from the scheduler side.
- `docs/02_ir_schemas.md` §CONFIG_BLOCK viewer-key contract — forbids padding edits.
- `docs/08_coordinate_system.md` — the modules work in plain mm floats; canonical's `scale_()` must not be ported.
- `docs/15_config_keys_reference.md` — generated by `cargo xtask gen-config-docs`, never hand-edited.

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` — its "Module-owned config keys (generated)" table gains rows for the seven `wipe-tower` keys and the two `prime-tower-interface` keys. Verification is key-presence (the doc has no per-module subheadings), embodied as AC-11. The edit lands through `cargo xtask gen-config-docs`, never by hand.
- `docs/07_implementation_status.md` — its generated Open Deviation Map is refreshed by `cargo xtask check-deviations` when the packet's `DEV-###` rows land (Step 8). Generated, never hand-edited.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the nine declarations (coTypes, defaults, bounds). Captured in `requirements.md` §Per-Key Canonical Evidence.
- `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower.cpp` — the `WipeTower::WipeTower` constructor (`m_use_gap_wall` cluster, `m_flat_ironing = m_flat_ironing && m_use_gap_wall`), `WipeTower::set_extruder` (per-filament `m_filpar`: pre-extrusion dist/length, print temp, `flat_iron_area`), `WipeTower::toolchange_wipe_new`, `WipeTower::finish_block_solid`, `WipeTower::get_next_pos`, `WipeTower::toolchange_Unload` (the ramming `y_step` from `m_extra_spacing`).
- `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower2.cpp` — `WipeTower2::WipeTower2` ctor (`m_enable_filament_ramming`, the cooldown flag), `WipeTower2::toolchange_Unload` / `toolchange_Load` (the staged ramming unload/load), `WipeTower2::set_extruder` (interface purge volume, print temp, `tower_ironing_area`), `WipeTower2::tool_change` (the cooldown branch), `WipeTower2::finish_layer`.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `WipeTowerIntegration::append_tcr` / `append_tcr2` and `GCode::set_extruder` (where the interface print temp and pre-extrusion length reach the emitted G-code — the analogue of this packet's `PostPass::GCodePostProcess` module).

Note: in this clone the checkout is the sibling `..\pinch_n_print_cli\OrcaSlicerDocumented` (pinned by wayfinder ticket 08's ledger note) — workers must resolve `OrcaSlicerDocumented/` against that absolute sibling path.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
