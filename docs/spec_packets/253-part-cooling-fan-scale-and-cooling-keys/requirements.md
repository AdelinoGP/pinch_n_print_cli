# Requirements: 253-part-cooling-fan-scale-and-cooling-keys

## Packet Metadata

- Grouped task IDs: none — the feature-gap queue's established pattern is `task_ids: []` (packet 234a precedent); `docs/07_implementation_status.md` holds no TASK row for this queue.
- Backlog source: `docs/specs/orca-feature-gap/issues/08-author-packet-p01-cooling-notes-part-cooling.md` (wayfinder map "Close the OrcaSlicer FFF feature gap", packet P01).
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

OrcaSlicer's cooling configuration is 19 % of every FFF feature gap packet queue's smallest owner-scoped slice (packet P01), and today Pinch 'n Print implements almost none of it faithfully:

- The `part-cooling` module declares 8 keys and reads 4; `fan_min_speed` is declared but **never read** (04-asset note), so `reduce_fan_stop_start_freq`'s never-off idle behavior has no input to idle at.
- `fan_max_speed`/`fan_min_speed` are raw S-values (0–255, defaults 255/51) while Orca declares percents (0–100, defaults 100/20) — the ticket-99 finding. Any preset or 3MF carrying Orca's percent values is misinterpreted as a raw S-value off by a factor of 2.55.
- The module's fan decision is `layer_index < close_fan ? 0 : fan_max_speed` plus a bridge-role bump — none of Orca's layer-time interpolation, full-fan ramp, stop/start suppression, role-fan speeds, overhang threshold, or fan speedup/kickstart exist.
- The air-filtration / chamber-temperature / exhaust-fan keys (`activate_air_filtration`, `activate_chamber_temp_control`, `during_print_exhaust_fan_speed`, `complete_print_exhaust_fan_speed`) do not exist anywhere in code. Canonical `GCode::_do_export` **emits** them directly through `GCodeWriter::set_exhaust_fan` (`M106 P3 S…`) and `GCodeWriter::set_chamber_temperature` (`M191`/`M141`) at the header and footer — they are not custom-template placeholders, which is what an earlier draft of this packet assumed. This port's `machine-gcode-emit` already owns those two insertion points, so the missing thing is the emission, not a placeholder.
- `dont_slow_down_outer_wall` canonically gates `CoolingBuffer`'s slowdown of external perimeters, and this tree has no layer-slowdown decision point at all: `slow_down_for_layer_cooling`, `slow_down_min_speed`, and `slow_down_layer_time` are declared in the `part-cooling` manifest with **zero read sites**. Declaring a fourth inert key beside them is exactly the disposition the map now prohibits, so this packet builds the stage.

The slice is coherent because the 19 keys share one decision stream — the per-layer cooling response — split across the only two owners that can act on it: `part-cooling` at `PostPass::LayerFinalization` decides fan speed and layer slowdown from the same layer-time estimate, and `machine-gcode-emit` at `PostPass::GCodePostProcess` owns the header/footer sites where the remaining four keys are emitted.

## In Scope

All 19 P01 keys (membership from `05-asset-packet-list.md` as amended by ticket 99):

1. **Percent normalization (Tier B):** `fan_max_speed`, `fan_min_speed` — re-declare percent 0–100 (defaults 100/20 = Orca byte-identical), migrate every read site, convert percent→S-value at emission via canonical `GCodeWriter::set_fan`'s `floor(255.5 × p / 100)`.
2. **Fan curve port (Tier B + Tier A):** port `CoolingBuffer.cpp::apply_layer_cooldown`'s `change_extruder_set_fan` decision into `PartCooling` — `fan_cooling_layer_time`, `full_fan_speed_layer`, `reduce_fan_stop_start_freq` (now actually wired to `fan_min_speed`), first-layers gate; the already-declared `slow_down_layer_time`/`slow_down_min_speed` become read as interpolation inputs.
3. **Role-fan keys (Tier A):** `internal_bridge_fan_speed`, `ironing_fan_speed`, `support_material_interface_fan_speed` — declared, read, applied with `-1`-disabled fallback semantics per canonical precedence.
4. **Overhang threshold (Tier A):** `overhang_fan_threshold` — enum `0%|10%|25%|50%|75%|95%` (canonical default `95%`), classifying entities by `overhang_quartile` bands and bridge roles.
5. **Time-domain fan control (Tier A):** `fan_kickstart`, `fan_speedup_time`, `fan_speedup_overhangs` — re-time rising fan commands per canonical `FanMover` semantics using a per-entity time model derived from IR geometry.
6. **Auxiliary channel (Tier A):** `auxiliary_fan`, `additional_cooling_fan_speed` — `M106 P2 S{n}` per-layer raw annotation, percent-converted, gated on `auxiliary_fan`.
7. **Header/footer G-code synthesis (Tier B):** `activate_air_filtration`, `activate_chamber_temp_control`, `during_print_exhaust_fan_speed`, `complete_print_exhaust_fan_speed` — declared in **`machine-gcode-emit`** (the module that owns the `PrintStart`/`PrintEnd` injection sites) and made to *emit* the canonical lines there: `M106 P3 S{n}` after the start block and after the end block (canonical `GCodeWriter::set_exhaust_fan`), `M191 S{t}` before the start block and `M141 S0` after the end block (canonical `GCodeWriter::set_chamber_temperature`). The supporting key `chamber_temperature` is declared alongside them because the chamber emission is `activate_chamber_temp_control && chamber_temperature > 0` — it is an input this behaviour needs, not a 20th counted key.
8. **Layer-time slowdown stage (Tier B):** port canonical `CoolingBuffer`'s layer-slowdown (`CoolingBuffer::calculate_layer_slowdown`, fed by `CoolingBuffer::parse_layer_gcode`'s adjustability classification) into `PartCooling` — when a layer's estimated print time falls below `slow_down_layer_time`, the adjustable entities' speeds are scaled down toward the `slow_down_min_speed` floor, and `dont_slow_down_outer_wall` removes `OuterWall` entities from the adjustable set. The stage writes speeds through the existing `FinalizationOutputBuilder::modify_entity` / `EntityMutation::SetSpeedFactor` surface (no WIT, IR-schema, or host change). This also makes the three `slow_down_*` keys already in the manifest genuinely read for the first time.
9. **Declarations + docs:** every key in this packet carries Orca-parity defaults, bounds, enums, display/group metadata; `docs/15_config_keys_reference.md` regenerated; guest WASM rebuilt.

## Out of Scope

- Canonical's **per-filament config vectors**. Orca reads `activate_air_filtration` / `activate_chamber_temp_control` / the two exhaust speeds per filament via `get_at(extruder.id())` and reduces them with `max`/OR across active extruders, behind an outer printer-level `support_air_filtration` bool and alongside `activate_air_filtration_during_print` / `activate_air_filtration_on_completion`. This port has no per-filament profile model (the map's Tier-D fog), so those four extra canonical keys are **not** ported and the gating collapses to the two bools this packet declares — a recorded divergence in `design.md`, not a gap.
- Porting canonical `FanMover` as a textual post-filter over serialized G-code — this packet re-times at the annotation level instead (see `design.md` §Rejected alternatives).
- Rewriting travel-move feedrates in the slowdown stage. Canonical's `CoolingBuffer` adjusts extrusion moves only; travel is untouched here too.
- `object_config` / per-object overrides of the new keys (host plumbing exists for typed fields; extensions-bucket keys ride global config only).
- Renaming or removing any of the three `slow_down_*` keys already in the `part-cooling` manifest. They are not among the 19 P01 keys, but this packet's slowdown stage (In Scope item 8) makes all three **read** for the first time — `slow_down_for_layer_cooling` as the stage's enable, `slow_down_layer_time` as the target, `slow_down_min_speed` as the floor. Their names and defaults stay exactly as declared.
- Any other feature-gap packet (P02+ in `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md`).
- `OrcaSlicerDocumented/` code changes — read-only checkout, never modified.

## Authoritative Docs

- `docs/specs/orca-feature-gap/issues/08-author-packet-p01-cooling-notes-part-cooling.md` — the ticket; direct read (30 lines).
- `docs/specs/orca-feature-gap/issues/02-parity-evidence-standard.md` — the evidence standard binding every key below; direct read (~80 lines).
- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` — the 19 keyed rows defining tier/owner framing (ranged read of the cooling section only).
- `docs/15_config_keys_reference.md` — regenerated artifact; never hand-edited (grep-verified post-regen).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet (the checkout is the **sibling** path `F:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented` — not `./OrcaSlicerDocumented`):

- `src/libslic3r/GCode/CoolingBuffer.cpp` — `apply_layer_cooldown` (lambda `change_extruder_set_fan`): the full fan-speed decision — `reduce_fan_stop_start_freq` base, `slow_down_layer_time` / `fan_cooling_layer_time` interpolation, `full_fan_speed_layer` ramp, role-fan marker precedence (`OVERHANG > INTERNAL_BRIDGE > SUPP_INTERFACE > IRONING`), and `-1` fallbacks; `parse_layer_gcode` / `process_layer`: `dont_slow_down_outer_wall` external-perimeter gating (decision point absent from this port).
- `src/libslic3r/GCode.cpp` — `check_overhang_fan`: `overhang_fan_threshold` overlap comparisons; `process_layers`: `FanMover` construction from `fan_speedup_time` / `fan_speedup_overhangs` / `fan_kickstart`; `_do_export`: air-filtration / chamber-temp / exhaust-fan emission structure (never ported — header/footer semantics belong to custom G-code templates here).
- `src/libslic3r/GCode/FanMover.cpp` — `_process_gcode_line`: re-timing of rising fan commands and the kickstart burst.
- `src/libslic3r/PrintConfig.cpp` — `PrintConfigDef`: declarations, types, defaults (incl. `overhang_fan_threshold` defaulting to `Overhang_threshold_bridge` = `"95%"`) and `s_keys_map_OverhangFanThreshold`.
- `src/libslic3r/GCodeWriter.cpp` — `set_fan`: the `255.5 × speed / 100` percent→S conversion (also `set_additional_fan` for the `P2` channel).

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

Every key this packet implements carries evidence per the map's ticket 02 standard:

- **Canonical read + described behaviour.** For each key, cite the canonical consumer (file + function, never line numbers) and describe its behaviour in `requirements.md`. Reads of `OrcaSlicerDocumented/` are delegated per the orca-delegation snippet.
- **Invariants, not goldens.** Behaviour is pinned with invariant/property tests (counts preserved, mappings hold, emitted values equal expected). Golden G-code comparison is not part of the standard — the checkout is not built and cannot be run.
- **Ported Orca tests are acceptable evidence.** When `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its assertions into PnP's suite with the standard porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
- **Plumbing keys** (a threshold feeding an existing decision point): the default resolves to the canonical value AND a test proves the value reaches the consumer. No behavioural test required.
- **Unverifiable behaviour:** surface the key and the reason to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row (single source of truth, CI-checked by `cargo xtask check-deviations`) and proceed with documented scope. Never defer the key or block the packet on unverifiability alone, and never file a row without the human having been asked.

## Per-Key Canonical Evidence (verified this session at authoring time)

| Key | Canonical declaration | Canonical consumer (file + function) | Ported behaviour |
| --- | --- | --- | --- |
| `fan_max_speed` | `PrintConfig.cpp` `PrintConfigDef` (coFloats, default 100, [%]) | `CoolingBuffer.cpp::apply_layer_cooldown` lambda `change_extruder_set_fan`; `GCodeWriter.cpp::set_fan` for %→S | curve maximum; `255.5 × p / 100` floor at emission |
| `fan_min_speed` | same (coFloats, default 20) | same lambda; also the `reduce_fan_stop_start_freq` idle base | curve interpolation floor; idle base when suppressing stop/start |
| `fan_cooling_layer_time` | same (coFloats, default 60, [0, 1000]) | same lambda — top of interpolation window | interpolation upper bound |
| `fan_kickstart` | same (coFloat, default 0, min 0) | `GCode.cpp::process_layers` → `FanMover` (`GCode/FanMover.cpp::_process_gcode_line`) | max-speed burst N seconds before demand |
| `fan_speedup_time` | same (coFloat, default 0) | same `FanMover` construction + re-timing | rising fan emitted N seconds early |
| `fan_speedup_overhangs` | same (coBool, default true) | same gate `(!only_overhangs \|\| role == erOverhangPerimeter)` | restrict early emission to overhang layers |
| `full_fan_speed_layer` | same (coInts, default 0) | same lambda's ramp factor | linear ramp from `close_fan` to `full_fan_speed_layer` |
| `reduce_fan_stop_start_freq` | same (coBools, default false) | same lambda's base `fan_speed_new = R ? fan_min_speed : 0` | fan idles at min instead of off |
| `dont_slow_down_outer_wall` | same (coBools, default false) | `CoolingBuffer.cpp::parse_layer_gcode` — read into `PerExtruderAdjustments::dont_slow_down_outer_wall`, then clears `adjust_external` in the same function's line-classification loop so external-perimeter moves are never marked adjustable | removes `OuterWall` entities from the slowdown stage's adjustable set (the stage is built by this packet, item 8) |
| `auxiliary_fan` | same (coBool machine-level, default false) | `GCode.cpp::_do_export` (P2 switch) | P2-channel enable + placeholder |
| `additional_cooling_fan_speed` | same (coInts, default 0, [0,100]) | same lambda's `change_extruder_set_fan` P2 line | per-layer P2 speed |
| `activate_air_filtration` | same (coBools, default false) | `GCode.cpp::_do_export` — per-filament `get_at`, under the printer-level `support_air_filtration` bool, gating both exhaust-fan emissions | single global bool gating both `M106 P3` emissions (per-filament collapse = recorded divergence) |
| `activate_chamber_temp_control` | same (coBools, default false) | `GCode.cpp::_do_export` — OR-reduced over extruders; with `chamber_temperature > 0` and a negative `custom_gcode_sets_temperature` check, calls `GCodeWriter::set_chamber_temperature(t, true)` before the start template and `(0, false)` after `postamble()` | emits `M191 S{t}` before the `PrintStart` block and `M141 S0` after the `PrintEnd` block |
| `during_print_exhaust_fan_speed` | same (coInts, default 60, [0,100]) | `GCode.cpp::_do_export` — max across enabled filaments, then `GCodeWriter::set_exhaust_fan`, which writes `M106 P3 S<(int)(speed / 100.0 * 255)>` | emits `M106 P3 S{n}` last in the `PrintStart` block |
| `complete_print_exhaust_fan_speed` | same (coInts, default 80, [0,100]) | `GCode.cpp::_do_export` end block, after `postamble()` and after the chamber-off line; same `GCodeWriter::set_exhaust_fan` writer | emits `M106 P3 S{n}` last in the `PrintEnd` block |
| `internal_bridge_fan_speed` | same (coInts, default −1, [%]) | same lambda — `_INTERNAL_BRIDGE_FAN_START` marker path; −1 falls back to `overhang_fan_speed` | role fan with fallback |
| `ironing_fan_speed` | same (coInts, default −1) | same lambda — `erIroning` marker path, −1 = disabled | role fan, −1 = disabled |
| `support_material_interface_fan_speed` | same (coInts, default −1) | same lambda — `_SUPP_INTERFACE` marker path | role fan, −1 = disabled |
| `overhang_fan_threshold` | same (coEnums, default `Overhang_threshold_bridge` = `"95%"`; map `0%|10%|25%|50%|75%|95%`) | `GCode.cpp` lambda `check_overhang_fan` (overlap comparisons) | quartile-band classifier (mapping table in design) |

Note: the repo's snapshot reference `docs/ORCA_CONFIG_REFERENCE.md` records the `overhang_fan_threshold` default as 50%; the fresh canonical read this session (`PrintConfig.cpp` `PrintConfigDef`, `set_default_value(new ConfigOptionEnumsGeneric{ (int)Overhang_threshold_bridge })`) says the default is `95%`. This packet follows the fresh read and records the discrepancy here — do **not** re-size off the snapshot column (map Notes rule).

### Supporting key declared by this packet (not one of the 19)

| Key | Canonical declaration | Canonical consumer (file + function) | Ported behaviour |
| --- | --- | --- | --- |
| `chamber_temperature` | `PrintConfig.cpp` `PrintConfigDef::init_fff_params` (coInts, default 0, min 0) | `GCode.cpp::_do_export` — max-reduced across extruders into `max_chamber_temp`; the chamber emission requires `max_chamber_temp > 0` | the `S` value of the `M191` line; default 0 keeps the emission inert |

`chamber_temperature` is declared because `activate_chamber_temp_control` cannot drive a behaviour without it — it is an input to a decision point this packet builds, not a key counted toward P01's coverage. The map/ticket update is listed in the packet's session report.

## Returned to Queue — unimplemented

**None.** Every one of the 19 P01 keys drives a decision point that exists in this tree after this packet: 15 in `part-cooling` (fan curve, role fans, threshold, re-timing, P2 channel, slowdown stage) and 4 in `machine-gcode-emit` (header/footer synthesis). `dont_slow_down_outer_wall` was the one key the pre-rules draft of this packet declared without a consumer; this revision builds the layer-time slowdown stage it gates (In Scope item 8) rather than shedding it.

## Ruled Dead-in-Canonical

**None.** All 19 keys were re-verified this session against the sibling checkout as having read sites inside `src/libslic3r/` in the slicing pipeline, excluding `ConfigManipulation.cpp`, GUI, tooltips, and preset/invalidation plumbing. The five keys that looked most likely to be dead were checked individually:

- `activate_air_filtration`, `activate_chamber_temp_control`, `during_print_exhaust_fan_speed`, `complete_print_exhaust_fan_speed` — all four are read in `GCode.cpp::_do_export` and emitted through `GCodeWriter::set_exhaust_fan` / `GCodeWriter::set_chamber_temperature`. They are **not** placeholder-only; the earlier draft of this packet claimed placeholder reachability was their whole story, and that was wrong. (`during_print_exhaust_fan_speed` is *additionally* published to the placeholder parser as `during_print_exhaust_fan_speed_num`, scaled `item / 100.0 * 255`; the other three are not.)
- `dont_slow_down_outer_wall` — read in `CoolingBuffer.cpp::parse_layer_gcode`, clearing `adjust_external`.

Hits in `Print.cpp` invalidation lists, `Preset.cpp`, and `PrintConfig.cpp` definitions are preset/invalidation plumbing and were excluded per the map's rule 3.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1`/`AC-1b` (declarations + bounds in each owner), `AC-2` (three distinct canonical converters) with `AC-2b` (default-path identity, supplementary only), `AC-3` (fan-curve branch matrix), `AC-4` (role-fan precedence and -1 fallbacks), `AC-5` (threshold mapping), `AC-6` (kickstart/speedup re-timing), `AC-7` (auxiliary P2 channel), `AC-8`/`AC-8b`/`AC-8c` (exhaust-fan and chamber-temperature emission, and the user-template suppression), `AC-9` (docs regeneration), `AC-10` (layer-time slowdown stage), `AC-11` (`dont_slow_down_outer_wall` exclusion).
- Negative: `AC-N1` (bounds enforcement of new keys), `AC-N2` (no key leakage into non-declaring modules), `AC-N3` (exhaustive conversion-formula pin).
- **Map gate (b) coverage.** Every one of the 19 keys has at least one AC asserting a behaviour change at a non-default value: `fan_max_speed`/`fan_min_speed`/`fan_cooling_layer_time`/`full_fan_speed_layer`/`reduce_fan_stop_start_freq` -> AC-3; `internal_bridge_fan_speed`/`ironing_fan_speed`/`support_material_interface_fan_speed` -> AC-4; `overhang_fan_threshold` -> AC-5; `fan_kickstart`/`fan_speedup_time`/`fan_speedup_overhangs` -> AC-6; `auxiliary_fan`/`additional_cooling_fan_speed` -> AC-7; `activate_air_filtration`/`during_print_exhaust_fan_speed`/`complete_print_exhaust_fan_speed` -> AC-8; `activate_chamber_temp_control` -> AC-8b; `dont_slow_down_outer_wall` -> AC-11. AC-2b and the negative halves of AC-8/AC-8b assert default-path identity and are supplementary in every case, never a key's only evidence.
- Cross-packet impact: `machine-gcode-emit` is touched here; a packet that also edits it must sequence after this one. `overhang-classifier-default` is read but not edited — it writes speed mutations at the same stage, and Step 7 verifies the composition. Packet P15 (`support_ironing`) may later consume `ironing_fan_speed`'s decision point but owns no change here.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p part-cooling 2>&1 | tee target/test-output.log | grep -E "^test result"` | all module behaviour + schema tests | FACT pass/fail + counts |
| `cargo test -p part-cooling --test cooling_curve_parity_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` | the new canonical-curve invariant suite | FACT pass/fail |
| `cargo test -p machine-gcode-emit 2>&1 | tee target/test-output.log | grep -E "^test result"` | header/footer emission + the default-stream regression pin | FACT pass/fail |
| `cargo test -p machine-gcode-emit --test exhaust_and_chamber_emission_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` | AC-1b, AC-8, AC-8b, AC-8c | FACT pass/fail |
| `cargo test -p part-cooling --test layer_slowdown_parity_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` | AC-10, AC-11 — the layer-time slowdown stage | FACT pass/fail |
| `cargo test -p slicer-scheduler --test config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` | bounds/enum enforcement of the 19 keys | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration -- gcode_part_cooling_emission_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"` | emission-surface regression pin (module inside the aggregated `integration` binary, registered at `tests/integration/main.rs`) | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract -- integrated_parity_part_cooling 2>&1 | tee target/test-output.log | grep -E "^test result"` | module→host parity contract stays green (module registered in the aggregated `contract` binary) | FACT pass/fail |
| `cargo xtask gen-config-docs --check 2>&1 | tail -5` | docs/15 regenerated tables match manifests | FACT (tail lines) |
| `cargo xtask build-guests --check; echo "exit=$?"` | guest freshness after manifest/src edits | exit code |
| `cargo check --workspace --all-targets` | compile gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask check-literals 2>&1 | tail -3` | struct-literal churn gate (new test fixtures) | FACT (tail lines) |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

- Steps build in order: percent normalization precedes the curve port (curve tests assert percent-domain expectations); the curve port precedes the slowdown stage, which **reuses** the curve's layer-time estimator rather than writing a second one; the manifests precede every test that reads them; docs regen is last. Step 8 (header/footer emission) depends only on Step 2 and may run in parallel with Steps 3-7 — it touches a different crate.
- The existing tests `first_layers_disabled_then_fan_on`, `overhang_region_bumps_fan`, and slicer-runtime's `gcode_part_cooling_emission_tdd` fixtures assert raw-scale bytes (`S255`, `S100`) — under percent normalization the *defaults* still produce the exact module-emitted bytes (100% → S255 via `floor(255.5)`; the overhang bump at `overhang_fan_speed=100` × `fan_max_speed=100` percent → 100% → S255). Any fixture whose input data was raw-typed must restate its intent in percent and **re-derive** expected bytes through `floor(255.5 × p / 100)` (e.g. a 50% expectation is `floor(127.75)` = **S127**, not S128) — never silently edit a byte to pass.
- `OrcaSlicerDocumented/` lives at the sibling path (see orca-delegation in `packet.spec.md`): all canonical reads delegated.

## Context Discipline Notes

- The fan-curve port (Step 4) is the packet's deepest step: delegate canonical re-reads (`CoolingBuffer.cpp` lambda quotes) rather than re-deriving from memory; the load-bearing interpolation formula is already quoted in this file's per-key table's source (ticket grounding above) but must be re-verified against the sibling checkout at implementation time.
- `crates/slicer-ir/src/slice_ir.rs` is long: ranged reads only, located by symbol (`ExtrusionRole`, `ExtrusionPath3D`, `Point3WithWidth`, `PrintEntity`, `LayerAnnotation`, `EntitySpeedProfile`, `LayerCollectionIR`). Do not trust a line pin written here — grep the symbol.
- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` is long: read the cooling section rows only.
- Never load `target/`, `OrcaSlicerDocumented/` directly, or generated lockfiles.