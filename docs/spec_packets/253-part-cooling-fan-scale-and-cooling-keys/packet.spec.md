---
status: draft
packet: 253-part-cooling-fan-scale-and-cooling-keys
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/08-author-packet-p01-cooling-notes-part-cooling.md (wayfinder map: Close the OrcaSlicer FFF feature gap — packet P01)
context_cost_estimate: M
---

# Packet Contract: 253-part-cooling-fan-scale-and-cooling-keys

This packet was authored per the useful grounding in `requirements.md` §Verified Grounding; all symbol paths below were verified against the current tree at authoring time.

## Goal

Implement OrcaSlicer's 19-key cooling-config surface in the `part-cooling` module: percent-normalize `fan_max_speed`/`fan_min_speed` to Orca's 0–100 scale and port the canonical fan-decision chain (layer-time interpolation, full-fan ramp, stop/start suppression, role-based fan speeds, overhang threshold enum, time-domain fan speedup/kickstart), and join the air-filtration / chamber-temperature / exhaust-fan keys to the host's emission surface (config block + custom-G-code placeholders).

## Scope Boundaries

All 19 P01 keys are declared in `modules/core-modules/part-cooling/part-cooling.toml` and consumed per the decision-point matrix in `requirements.md`; the four header/footer keys are co-declared (existing across-module pattern) into `machine-gcode-emit` so custom G-code templates can substitute them. No host-crate changes, no WIT/IR shape change, no schema bump, no new module. The layer-slowdown stage does **not** exist in this tree, so `dont_slow_down_outer_wall` lands on the emission surface with its decision-point gap recorded — building that stage is future work, not this packet. Full lists in `requirements.md`.

## Prerequisites and Blockers

- Depends on: wayfinder tickets 06 + 99 (both resolved — packet-number rule and the P01 amendment that added `fan_max_speed`/`fan_min_speed` as Tier B keys).
- Unblocks: the next packet in the feature-gap queue (per `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` order).
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** the `part-cooling` manifest after this packet, **when** its `[config.schema]` is parsed, **then** it declares exactly 25 keys: the 19 P01 keys below **plus** the 6 pre-existing keys it already declares (`close_fan_the_first_x_layers`, `enable_overhang_bridge_fan`, `overhang_fan_speed`, `slow_down_for_layer_cooling`, `slow_down_min_speed`, `slow_down_layer_time`). The 19 carry Orca-parity defaults — `fan_min_speed` 20, `fan_max_speed` 100, `fan_cooling_layer_time` 60.0, `fan_kickstart` 0.0, `fan_speedup_time` 0.0, `fan_speedup_overhangs` true, `full_fan_speed_layer` 0, `reduce_fan_stop_start_freq` false, `dont_slow_down_outer_wall` false, `auxiliary_fan` false, `additional_cooling_fan_speed` 0, `activate_air_filtration` false, `activate_chamber_temp_control` false, `during_print_exhaust_fan_speed` 60, `complete_print_exhaust_fan_speed` 80, `internal_bridge_fan_speed` -1, `ironing_fan_speed` -1, `support_material_interface_fan_speed` -1, `overhang_fan_threshold` enum `"95%"` — with `fan_max_speed`/`fan_min_speed` now bounded [0, 100] percent, the seven role-fan/percentage keys bounded `[-1, 100]` (the exhaust pair [0, 100]), and the threshold enum limited to `0%, 10%, 25%, 50%, 75%, 95%`. | `cargo test -p part-cooling --test cooling_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-2. Given** the percent normalization, **when** a percent fan value is converted to an S-value, **then** every conversion site uses `floor(255.5 × percent / 100)` (canonical `GCodeWriter::set_fan`) — so the defaults 20/100 yield exactly the S-values today's raw defaults produce (S51/S255), and every other percent loses at most the canonical single-half truncation: the default-path output is byte-identical to the pre-packet module on the no-overhang, always-max path it previously emitted. | `cargo test -p part-cooling --test part_cooling_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3. Given** the ported fan curve (canonical `CoolingBuffer.cpp::apply_layer_cooldown`, lambda `change_extruder_set_fan`), **when** a layer's print time `T` is evaluated against `close_fan_the_first_x_layers` (`C`), `slow_down_layer_time` (`S`), `fan_cooling_layer_time` (`F`), `full_fan_speed_layer` (`L`), and `reduce_fan_stop_start_freq` (`R`), **then**: layers below `C` emit fan 0; `R` makes the off-branch fan equal `fan_min_speed` instead of 0; `T < S` forces `fan_max_speed`; `S ≤ T < F` interpolates linearly from max (at `S`) to min (at `F`); `T ≥ F` yields the `R`-dependent base; and the index ramp `factor = (layer_index + 1 − C) / (L − C)` scales the result only when `layer_index + 1 < L` (ramp inert when `L ≤ C`). Each branch is pinned by its own named invariant test. | `cargo test -p part-cooling --test cooling_curve_parity_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4. Given** a finalization layer containing role-fan regions, **when** the fan decision runs, **then** speed selection follows canonical precedence overhang > internal-bridge > support-interface > ironing: `BridgeInfill` (or points whose `overhang_quartile` meets `overhang_fan_threshold`) selects `overhang_fan_speed`; `InternalBridgeInfill` selects `internal_bridge_fan_speed` with the `-1` fallback to `overhang_fan_speed`; `SupportInterface` applies `support_material_interface_fan_speed` only when `≥ 0`; `Ironing` applies `ironing_fan_speed` only when `≥ 0`; and with default config (all `-1` except overhang) a bridge-over-support-interface layer emits the `overhang_fan_speed` value — never an unsigned wrap of `-1`. | `cargo test -p part-cooling --test cooling_curve_parity_tdd -- role_fan_precedence_and_minus_one_fallbacks 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-5. Given** `overhang_fan_threshold` set to each of its six enum values, **when** entities with per-point `overhang_quartile` bands 1–4 and `BridgeInfill`/`InternalBridgeInfill` roles are classified, **then** the thresholds map as `10%` → any quartile, `25%` → quartile ≥ 2, `50%` → quartile ≥ 3, `75%` → quartile 4, `95%` → bridge roles only, `0%` → external-perimeter roles (`OuterWall`) qualify per canonical `check_overhang_fan`'s `Overhang_threshold_none → is_external_perimeter(role)` semantics, with `BridgeInfill`/`InternalBridgeInfill` roles qualifying under every threshold value. | `cargo test -p part-cooling --test cooling_curve_parity_tdd -- overhang_threshold_maps_quartile_bands 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-6. Given** `fan_kickstart > 0` or `fan_speedup_time > 0`, **when** a rising fan command is emitted, **then** the module re-times it to the entity boundary at least `fan_speedup_time` seconds (or the kickstart duration at `fan_max_speed`) earlier than the demanding entity, gated to overhang-containing layers only when `fan_speedup_overhangs` is true, using the same per-entity time model as AC-3; both flags at their defaults (0) leave emission timing unchanged. | `cargo test -p part-cooling --test cooling_curve_parity_tdd -- fan_kickstart_and_speedup_retime_rising_fan 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-7. Given** `auxiliary_fan` true with `additional_cooling_fan_speed` set, **when** layers are finalized, **then** each layer after `close_fan_the_first_x_layers` carries an `M106 P2 S{n}` raw annotation (percent-converted), a `P2`-zeroing fan-off accompanies the final layer, and no `P2` line exists at all when `auxiliary_fan` is false. | `cargo test -p part-cooling --test cooling_curve_parity_tdd -- auxiliary_fan_emits_p2_channel 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-8. Given** the four header/footer keys co-declared into `machine-gcode-emit`, **when** a custom machine start/end G-code template references `[during_print_exhaust_fan_speed]`, `[complete_print_exhaust_fan_speed]`, `[activate_air_filtration]`, or `[activate_chamber_temp_control]`, **then** `machine-gcode-emit`'s placeholder substitution resolves every reference from the module's ConfigView (the emission-surface wiring), with unset keys resolving to their schema defaults; values stay raw percents in templates (no ×2.55 conversion). | `cargo test -p machine-gcode-emit --test cooling_placeholder_reachability_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-9 (docs).** `docs/15_config_keys_reference.md` regenerated tables list all 19 keys under owner `part-cooling` (module-config-keys table) and its Orca-deviations table gains no new row for them (all defaults match Orca); the raw-0–255 fan scale is purged from the docs tree. | `cargo xtask gen-config-docs --check 2>&1 | tail -3 && (rg -n '0–255|0-255' docs/15_config_keys_reference.md docs/01_project_overview.md | grep -i 'fan' && exit 1 || echo AC9-PASS)`

## Negative Test Cases

- **AC-N1. Given** a config TOML setting `part-cooling`'s `overhang_fan_threshold` to `"30%"` (not in the enum) or `internal_bridge_fan_speed` to `101` (above its `[−1, 100]` bound), **when** resolution runs, **then** the host rejects the value with the existing `ConfigBoundsIndex::check` out-of-range error naming the key — the new declarations are bounds-enforced, not inert. | `cargo test -p slicer-scheduler --test config_bounds_enforcement_tdd -- new_cooling_keys_bounds_enforced 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N2. Given** a core module other than `part-cooling`/`machine-gcode-emit` (e.g. `fuzzy-skin`, whose manifest declares none of the 19 keys), **when** it receives resolved config, **then** `ConfigView::from_declared` still hides all 19 keys from it — the declarations leak no cooling config into modules that did not opt in. | `cargo test -p slicer-runtime --test integration -- undeclared_keys_stay_hidden_from_other_modules 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N3. Given** the percent normalization, **when** any conversion site is fed the extremes 0 and 100, **then** it yields S0 and S255 respectively and never an intermediate rounding drift from the shared conversion — caught by an exhaustive test over all 101 percent values asserting `floor(255.5 × p / 100)` for every one. | `cargo test -p part-cooling --test cooling_curve_parity_tdd -- percent_to_s_conversion_matches_canonical_formula_exhaustively 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p part-cooling 2>&1 | tee target/test-output.log | grep -E "^test result"`
- `cargo test -p machine-gcode-emit 2>&1 | tee target/test-output.log | grep -E "^test result"`
- `cargo xtask gen-config-docs --check 2>&1 | tail -5`
- `cargo xtask build-guests --check; echo "exit=$?"`
- `cargo xtask check-literals 2>&1 | tail -3`

## Authoritative Docs

- `docs/specs/orca-feature-gap/issues/08-author-packet-p01-cooling-notes-part-cooling.md` — the wayfinder ticket defining this packet's scope (direct read, 30 lines).
- `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` — P01 row: 19 keys, mixed 17-Tier-A + 2-Tier-B (ranged read, ~15 lines around the P01 heading).
- `docs/specs/orca-feature-gap/issues/02-parity-evidence-standard.md` — evidence standard (direct read, ~80 lines).
- `docs/15_config_keys_reference.md` — regeneration target; never hand-edited (delegated regen + grep verification only).

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` module-config-keys table + Orca-deviations table - regenerated by `cargo xtask gen-config-docs`; verification grep: `rg -q 'overhang_fan_threshold' docs/15_config_keys_reference.md && rg -q 'support_material_interface_fan_speed' docs/15_config_keys_reference.md`
- `docs/01_project_overview.md` - amend any prose describing part-cooling fan keys on the raw 0–255 scale; verification grep: `rg -n '0–255|0-255' docs/01_project_overview.md | grep -i 'fan' || echo PASS`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet (the checkout is the **sibling** path `F:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented` — not `./OrcaSlicerDocumented`):

- `src/libslic3r/GCode/CoolingBuffer.cpp` — `apply_layer_cooldown` (lambda `change_extruder_set_fan`): the full fan-speed decision — `reduce_fan_stop_start_freq` base, `slow_down_layer_time` / `fan_cooling_layer_time` interpolation, `full_fan_speed_layer` ramp, role-fan marker precedence (`OVERHANG > INTERNAL_BRIDGE > SUPP_INTERFACE > IRONING`), and `-1` fallbacks; `parse_layer_gcode` / `process_layer`: `dont_slow_down_outer_wall` external-perimeter gating (decision point absent from this port).
- `src/libslic3r/GCode.cpp` — `check_overhang_fan`: `overhang_fan_threshold` overlap comparisons; `process_layers`: `FanMover` construction from `fan_speedup_time` / `fan_speedup_overhangs` / `fan_kickstart`; `_do_export`: air-filtration / chamber-temp / exhaust-fan emission structure (never ported — header/footer semantics belong to custom G-code templates here).
- `src/libslic3r/GCode/FanMover.cpp` — `_process_gcode_line`: re-timing of rising fan commands and the kickstart burst.
- `src/libslic3r/PrintConfig.cpp` — `PrintConfigDef`: declarations, types, defaults (incl. `overhang_fan_threshold` defaulting to `Overhang_threshold_bridge` = `"95%"`) and `s_keys_map_OverhangFanThreshold`.
- `src/libslic3r/GCodeWriter.cpp` — `set_fan`: the `255.5 × speed / 100` percent→S conversion (also `set_additional_fan` for the `P2` channel).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).