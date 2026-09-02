# Design: prime-tower-interface-and-ramming

## Tier Re-derivation

**Tier C.** Map Authoring rule 1 requires a packet that builds a decision point to be re-tiered B or C. This is **C, not B**, for one reason: it introduces a **new core module** (`prime-tower-interface`) that participates in a stage the existing `wipe-tower` module cannot reach, with a cross-crate registration surface (`slicer-integrated-modules`, `slicer-runtime`, `pnp-cli`, the scheduler's core-module count) and a new guest artifact. The seven `wipe-tower`-side keys on their own would be Tier B.

## Claims

Neither module holds a claim, and neither should.

- `wipe-tower` declares `holds = []` / `requires = []` and is stage-scheduled at `PostPass::LayerFinalization`.
- `prime-tower-interface` is authored the same way: `holds = []`, `requires = []`, `[stage] id = "PostPass::GCodePostProcess"`, `[ir-access] reads = ["GCodeIR"]`.

Map Authoring rule 4's claim-holder trigger test asks whether the config selects between *separate implementations that must live in separate modules and be resolved through the claim seam*. None of the nine keys does: seven are scalars and booleans parameterising one purge-block generator the module implements itself, and two are scalars parameterising one temperature emitter. A `claim:prime-tower-interface` seam would have exactly one possible holder.

**The new module is nevertheless a rule-4 win, for the other reason the rule gives.** Canonical emits the interface temperature from inside the same G-code writer that builds the tower (`GCode::set_extruder`, `WipeTowerIntegration::append_tcr`). This port's stages separate those concerns, and rather than widening `FinalizationOutputBuilder` with a command channel to reproduce canonical's coupling, the packet puts the emitter where the architecture already puts command emission. That is "new decision points go where the architecture puts them", recorded as divergence D-254b-5.

## Which existing mechanism carries the new data

| New data | Carrier | Status |
| --- | --- | --- |
| the seven `wipe-tower` keys | module manifest `[config.schema]` → `bind_module_config_view` (`crates/slicer-scheduler/src/execution_plan.rs`) → `ConfigView` | existing |
| the two `prime-tower-interface` keys | the new module's own manifest, same path | existing mechanism, new manifest |
| interface block depth, lead-in, ironing span | computed inside `generate_purge_paths` from `254a`'s `block_depth` / `depth_offset` parameters | **produced by `254a`** (FORWARD-DEP; names reconciled between the two packets) |
| tower location in the emitted stream | `GCodeCommand::Move { role: ExtrusionRole }` — `role` is already on the variant, and `ExtrusionRole::WipeTower` already exists (`crates/slicer-ir/src/slice_ir.rs`) | existing; **no new IR field** |
| tool-change boundaries in the stream | `GCodeCommand::ToolChange { after_entity_index, from, to }` | existing |
| the temperature command itself | `GCodeCommand::Temperature { tool, celsius, wait }`, pushed via `GcodeOutputBuilder::push_temperature(tool, celsius, wait)` (`crates/slicer-sdk/src/postpass_builders.rs`), serialized by `GcodeFlavor::set_temperature` (`crates/slicer-gcode/src/flavor.rs`) into `M104`/`M109` (`G10 P`/`M116` on RepRapFirmware) | existing end-to-end and already marshalled; **no production module has ever constructed one**, which is the gap this packet closes |

No WIT type, IR schema bump, `ResolvedConfig` field, SDK trait method, or builder method is added. `run_gcode_postprocess` is an existing default-implemented trait method (`crates/slicer-sdk/src/traits.rs`) the new module overrides.

## Selected Approach

### Part 1 — interface block and ramming, inside `wipe-tower`

`generate_purge_paths` becomes a small pipeline over the block's Y band (`[depth_offset, depth_offset + block_depth)`, both supplied by `254a`'s plan pass):

1. **Depth source.** `effective_volume = if interface_features { interface_purge_volume } else { purge_volume }`; `254a`'s `block_depth` formula takes `effective_volume` as its numerator. This is the only change to `254a`'s depth computation, and it is a pure substitution.
2. **Lead-in travel.** The existing leading travel entity (today a degenerate two-point zero-length move at `(tower_x, tower_y)`) becomes a real `pre_extrusion_dist`-long travel into the block start when the interface gate is on; otherwise it is emitted exactly as `254a` emits it.
3. **Lead-in extrusion.** When `pre_extrusion_length > 0.0` and the gate is on, one extruding entity of that path length precedes the scan lines, clamped to `tower_width` so it stays inside the footprint (the same clamp the existing prime entity uses).
4. **Ramming.** When `enable_filament_ramming` (default `true`), a zigzag entity covers the block's leading `y_step` band, `y_step = (infill_gap_percent / 100) × line_width` — canonical `WipeTower::toolchange_Unload` uses `m_extra_spacing` as exactly this. It precedes the scan lines and follows the lead-in.
5. **Scan lines.** Unchanged from `254a` apart from starting after whatever the steps above consumed of the band.
6. **Ironing.** When `interface_features && flat_ironing`, a trailing `ExtrusionRole::Ironing` boustrophedon pass covers `ironing_span = (ironing_area / tower_width).min(block_depth)` of depth at the block's pitch.

Order within a block is fixed and asserted: travel → lead-in → ramming → scan lines → ironing → prime. `254a`'s reverse-order insertion loop and its ascending depth rank (INV-6 there) are untouched.

### Part 2 — the `prime-tower-interface` module

A new core module, scaffolded from `machine-gcode-emit` (the existing `PostPass::GCodePostProcess` module) as the structural template. Its `run_gcode_postprocess(commands, output, config)`:

1. Returns immediately when `filament_tower_interface_print_temp < 0` (the default `-1`), pushing nothing — D-254b-2, asserted by AC-9.
2. Otherwise walks `commands` identifying **tower runs**: maximal contiguous spans of `GCodeCommand::Move { role: ExtrusionRole::WipeTower, .. }`, together with the nearest preceding `GCodeCommand::ToolChange`.
3. For each tower run, pushes exactly one `Temperature { tool, celsius: print_temp as f32, wait: false }` — positioned **before the first `WipeTower` move** when `enable_tower_interface_cooldown_during_tower` is `true`, and **before the preceding `ToolChange`** when it is `false` (the default). `tool` is the `ToolChange`'s `to` field, falling back to `0` when no preceding tool change exists.
4. Emits nothing when the stream contains no `WipeTower` move at all — AC-N2.

The two positions are canonical's own distinction: `WipeTower2::tool_change`'s cooldown branch chooses between boosting temperature *during tower printing* and boosting it *at the toolchange*.

### Rejected alternatives

- **Widening `FinalizationOutputBuilder` with a command channel** so `wipe-tower` could push the temperature itself. Rejected: it is a WIT/SDK boundary change (an explicit `[BLOCK]` trigger for this session) made solely to reproduce canonical's coupling, when the port already has a stage whose job is exactly this.
- **Adding the temperature logic to `machine-gcode-emit`.** Rejected: that module's contract is the eleven registered injection points and `[key]` substitution. Prime-tower temperature is unrelated behaviour and would make a focused module a grab bag.
- **Declaring a `nozzle_temperature` key so `-1` could resolve to a max.** Rejected: the key would be driven by nothing else in the tree, which is precisely the declaration-only shape rule 1 prohibits. D-254b-2 records the honest alternative.

## Code Change Surface (authoritative files-in-scope)

**Existing module:**
- `modules/core-modules/wipe-tower/wipe-tower.toml` — seven new `[config.schema.*]` tables.
- `modules/core-modules/wipe-tower/src/lib.rs` — seven new `WipeTower` fields + `from_config` arms; `generate_purge_paths` gains the lead-in / ramming / ironing stages and the `effective_volume` substitution.
- `modules/core-modules/wipe-tower/tests/wipe_tower_config_schema_tdd.rs` — extended (created by `254a`).
- `modules/core-modules/wipe-tower/tests/wipe_tower_tdd.rs` — AC-3 … AC-7, AC-N1.

**New module** (scaffolded from `modules/core-modules/machine-gcode-emit/`):
- `modules/core-modules/prime-tower-interface/prime-tower-interface.toml` — manifest: `[stage] id = "PostPass::GCodePostProcess"`, `[ir-access] reads = ["GCodeIR"]`, `[claims] holds = [] / requires = []`, two `[config.schema.*]` tables.
- `modules/core-modules/prime-tower-interface/Cargo.toml`, `src/lib.rs`.
- `modules/core-modules/prime-tower-interface/wit-guest/Cargo.toml`, `wit-guest/src/lib.rs`.
- `modules/core-modules/prime-tower-interface/tests/interface_temp_tdd.rs` — AC-8, AC-9, AC-N2 (standalone binary; a fresh `tests/` dir has no aggregator).

**Registration surface** (derived by following how `skirt-brim` is registered):
- `crates/slicer-integrated-modules/Cargo.toml` and `crates/slicer-integrated-modules/src/lib.rs`.
- `crates/slicer-runtime/Cargo.toml` and `crates/slicer-runtime/src/lib.rs`.
- `crates/pnp-cli/Cargo.toml`.
- `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs` — the core-module count assertion, currently `23` with a dated comment; becomes `24`.

**Host-side tests:**
- `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs` — AC-10 arm (existing, already registered).
- `docs/15_config_keys_reference.md` — regenerated only.

## Read-only context (allowed reads, no edits)

- `modules/core-modules/machine-gcode-emit/` — the `PostPass::GCodePostProcess` structural template (manifest shape, `run_gcode_postprocess` override, guest crate layout).
- `crates/slicer-sdk/src/traits.rs` — `run_gcode_postprocess`'s signature. Located window only; over the ceiling.
- `crates/slicer-sdk/src/postpass_builders.rs` — `GcodeOutputBuilder::push_temperature`. Located window only.
- `crates/slicer-ir/src/slice_ir.rs` — the `GCodeCommand` enum and `ExtrusionRole`. Located windows only; far over the ceiling.
- `modules/core-modules/skirt-brim/` — the reference for what "registering a core module" touches.

## Out of bounds (must not be loaded or edited)

- `crates/slicer-schema/wit/` — **no WIT change is required and none is permitted.** If an implementer concludes one is needed, that is a `[BLOCK]`: stop and report rather than editing.
- `crates/slicer-gcode/src/serialize.rs`'s `ORCA_CONFIG_PADDING` and every padding twin (map Authoring rule 2).
- `crates/slicer-ir/src/resolved_config.rs` — no host config change.
- `docs/spec_packets/254a-prime-tower-geometry-keys/` and `docs/spec_packets/255-wipe-tower-geometry-keys/` — sibling packets; never edited from here.
- Every other core module except the two named above and the two read-only templates.

## Expected Dispatches

| Question | Scope | Return format |
| --- | --- | --- |
| Confirm `WipeTower2::tool_change`'s cooldown branch — does the flag choose *during tower* vs *at toolchange*? (only if disputed) | `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower2.cpp` | SUMMARY ≤ 200 words |
| Confirm the ramming `y_step` derives from `m_extra_spacing` (only if disputed) | `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower.cpp` `toolchange_Unload` | SUMMARY ≤ 200 words |
| Confirm `m_flat_ironing = m_flat_ironing && m_use_gap_wall` (only if disputed) | `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower.cpp` ctor | SUMMARY ≤ 200 words |
| Enumerate every site that must change to register a new core module | `crates/slicer-integrated-modules/`, `crates/slicer-runtime/`, `crates/pnp-cli/Cargo.toml` — grep for `skirt-brim` / `skirt_brim` | LOCATIONS ≤ 20 |
| Every `cargo test` / `check` / `clippy` / `xtask` run | — | FACT pass/fail (+ ≤ 20 lines on failure) |

## Architecture Constraints

<!-- snippet: coord-system -->
**Coordinate system:** 1 unit = 100 nm (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Any constant ported from OrcaSlicer must be divided by 100. Use `Point2::from_mm(x, y)` / `mm_to_units()` for conversions. Full porting checklist in `docs/08_coordinate_system.md`.

Applies to Part 1 only: the lead-in distance, the ramming `y_step` and the ironing span are all mm, matching `WipeTower`'s existing plain-mm-float geometry (`tower_x`, `line_width`, `Point3WithWidth.x/y`). Never port `scale_`/`unscale` or a scaled literal. `filament_tower_ironing_area` is an **area** in mm², converted to a depth by dividing by `tower_width` — the one place a unit changes, and the conversion is explicit.

<!-- snippet: wasm-staleness -->
**Guest WASM staleness:** both modules' `.toml` and `src/lib.rs` are guest-fingerprint inputs — `guest_input_paths` (`xtask/src/build_guests.rs`) covers the guest `Cargo.toml`, every file under the guest `src/`, and for `GuestTree::Core` the parent module's `src/`, its `Cargo.toml`, and every depth-1 `*.toml` under the module dir. This packet also adds a **new** guest, which `discover_guests` must pick up. `cargo xtask build-guests --check` must return exit `0` (`EXIT_FRESH`) before any host-integration or dispatch test result is attributed to this packet. Exit `1` = rebuild and re-run. **Exit `3` = `wasm-tools` missing — an infrastructure error, not clean; it prints no `STALE:` line, so never decide freshness by grepping for `STALE:`.**

**Config key naming:** snake_case in both manifests and in every `config.get(...)` call (CLAUDE.md).

**Blast radius — new core module.** Adding a module is not a leaf change. The step that creates it owns, in the same commit: the crate + guest crate, the manifest, the workspace member entry, the dependency edges in `crates/slicer-integrated-modules/Cargo.toml`, `crates/slicer-runtime/Cargo.toml` and `crates/pnp-cli/Cargo.toml`, the registry entries in `crates/slicer-integrated-modules/src/lib.rs` and `crates/slicer-runtime/src/lib.rs`, and the **hard-asserted count** in `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs`, which today asserts exactly `23` core modules with a dated comment and a failure message that names the number. That count is test-assertion fallout of exactly the kind the blast-radius rule requires be pre-baked into the step, not discovered by a follow-up `cargo check`.

**Blast radius — `WipeTower` struct fields.** Seven new fields; `WipeTower` is constructed through `WipeTower::from_config` in the tests, so no `WipeTower { .. }` literal should need a `..` rest — confirm with `cargo xtask check-literals` rather than assuming. Any new test fixture for a watched type carries a `..` rest or an `// exhaustive: <reason>` waiver (`docs/21_data_defaults_and_fixtures.md`).

**Blast radius — default-path change from `enable_filament_ramming`.** Its canonical default is `true`, so **the default path changes**: every purge block gains a ramming entity. Entity-count assertions in `modules/core-modules/wipe-tower/tests/` and in `src/lib.rs`'s `#[cfg(test)]` module, plus `crates/slicer-runtime/tests/contract/integrated_parity_wipe_tower_tdd.rs` and `crates/slicer-runtime/tests/executor/finalization_live_tdd.rs` (both set `prime_volume`), are fallout the owning step updates **to the new expected count**, never by loosening the assertion. This is the one key in the packet whose default is not the identity value, and AC-N1 says so explicitly.

**Test-binary suitability:** AC-3 … AC-7 assert entity geometry from `run_finalization` and are homed in `wipe_tower_tdd.rs`, which drives the module directly. AC-8 / AC-9 / AC-N2 assert command-stream output from `run_gcode_postprocess` and are homed in the new module's own standalone `interface_temp_tdd.rs`, which can construct a `&[GcodeCommand]` fixture and a `GcodeOutputBuilder` without any end-to-end driver. AC-2 and AC-10 are host-side and are homed in already-registered aggregator files.

## Invariants

- **INV-1 (block order).** Within a purge block the entity order is exactly travel → lead-in → ramming → scan lines → ironing → prime, with absent stages simply omitted. Pinned across AC-4 … AC-7.
- **INV-2 (band containment).** Every entity a block emits — including the ironing pass — stays inside that block's Y band `[depth_offset, depth_offset + block_depth)` and inside `[tower_x, tower_x + tower_width]` in X. This preserves `254a`'s INV-1 (block disjointness) unchanged.
- **INV-3 (gate conjunction).** No ironing entity is emitted unless `enable_tower_interface_features && prime_tower_flat_ironing`. Pinned by AC-6's two negative directions.
- **INV-4 (one temperature per tower run).** `prime-tower-interface` pushes exactly one `Temperature` command per maximal `WipeTower`-role run, never zero and never two. Pinned by AC-8.
- **INV-5 (sentinel inertness).** At `filament_tower_interface_print_temp = -1` the command stream is returned byte-identical. Pinned by AC-9.
- **INV-6 (tool identity).** Purge and interface entities keep `254a`'s `tool_index = tc.to_tool`; `region_id` stays a pure identity and is never read as the tool (D-125-TOOL-IDENTITY-SPLIT). The `Temperature` command's `tool` is the `ToolChange`'s `to` field, which is the same quantity.
- **INV-7 (padding untouched).** `ORCA_CONFIG_PADDING` gains no entries and loses none.

## Risks

- **R-1 — `254a` not landed.** Every interface AC composes with `254a`'s `block_depth` / `depth_offset` parameters. Starting early forks `generate_purge_paths` twice. Mitigation: the FORWARD-DEP is stated in `packet.spec.md` §Prerequisites and is an activation blocker.
- **R-2 — ramming default flips the default path.** The one non-identity default in the packet. Mitigation: AC-N1 states the asymmetry explicitly and AC-7 asserts the `false` direction, so a reviewer cannot mistake the changed baselines for a regression.
- **R-3 — new-module registration half-done.** A discovered-but-unregistered module fails the integrated-edition build; a registered-but-undiscovered one fails `build-guests --check`. Mitigation: one commit, enumerated in the blast-radius note, with the `23 → 24` count pre-baked into the step.
- **R-4 — tower-run detection over-splitting.** If the emitted stream interleaves non-`WipeTower` moves inside a tower block (e.g. a travel move with a different role), the maximal-run scan would see several runs and push several `Temperature` commands. Mitigation: INV-4 and AC-8's "exactly one per run"; the implementer treats a non-extruding move between two `WipeTower` moves as part of the run, and the fixture in `interface_temp_tdd.rs` includes that interleaving case.
- **R-5 — guest staleness masking, with a new guest.** Mitigation: the exit-0 gate after every step, and the explicit "exit 3 is not clean" note.
- **R-6 — sibling manifest collision.** `254a` and packet 255 touch the same `wipe-tower` manifest and `from_config`. Mitigation: land `254a` first; all edits here are additive tables and additive arms.

## Open Questions

- **[FWD] Per-filament parameter model.** Five keys here are canonical `coFloats`/`coInts` declared scalar-global (D-254b-1). A per-filament model would make them exact and would also unblock `254a`'s D-254a-3 sibling question and packet 258's `filament_diameter` array. Forwarded to the Tier-D work the map already parks; it gates nothing here.
- **[FWD] Nozzle-temperature model.** D-254b-2's `-1` sentinel would resolve canonically if the tree had one. Forwarded to whichever packet introduces per-tool temperature configuration; the sentinel's current meaning ("no override") is asserted, not assumed.
- **[FWD] Staged unload/load.** D-254b-4 leaves canonical's multi-stage retraction to the existing retract machinery. A future ramming packet should reconcile the two rather than layer on top.

**No [BLOCK].** Every type, builder method, enum variant and trait method this packet uses was verified present in the tree at authoring: `GCodeCommand::{Move { role }, ToolChange, Temperature}`, `ExtrusionRole::{WipeTower, Ironing}`, `GcodeOutputBuilder::push_temperature`, `run_gcode_postprocess`, `GcodeFlavor::set_temperature`. No new WIT interface, no IR schema bump, no new `ResolvedConfig` field.

## Context Cost

Aggregate: **L**, driven by the new-module scaffold and its registration surface rather than by any single hard problem. Per-step costs are in `implementation-plan.md`; **no single step is rated L.** The new-module work is split into two M steps — scaffold + discovery + core-module count (Step 5), then the integrated-registry wiring (Step 6) — which is the finest split that keeps the tree compiling and the scheduler's manifest-ingestion count true at every commit boundary.
