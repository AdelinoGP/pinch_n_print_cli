---
status: draft
packet: 254-prime-tower-keys-wipe-tower
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/09-author-packet-p02-multimaterial-prime-tower-wipe-tower.md (wayfinder map: Close the OrcaSlicer FFF feature gap — packet P02)
context_cost_estimate: M
---

# Packet Contract: 254-prime-tower-keys-wipe-tower

This packet was authored per the useful grounding in `requirements.md` §Verified Grounding; all symbol paths below were verified against the current tree at authoring time.

## Goal

Declare OrcaSlicer's 13 P02 prime-tower config keys in the `wipe-tower` module manifest with Orca-parity defaults and bounds, and wire the one key whose decision point exists in this tree — `prime_tower_infill_gap`, driving the tower's scan-line pitch — while recording the honest decision-point gaps for the other 12 (interface cluster, ramming, framework, brim, travel-avoid), exactly as packet 253 did for `dont_slow_down_outer_wall`.

## Scope Boundaries

All 13 P02 keys are declared in `modules/core-modules/wipe-tower/wipe-tower.toml`; `prime_tower_infill_gap` (percent) is additionally read by the module and becomes the scan-line pitch factor `(value/100) × line_width`, replacing the hardcoded `y += line_width` advance — a behavior change at defaults that this packet owns with updated invariant tests. The 12 keys whose canonical consumers (interface-feature cluster, ramming sequence, framework walls, first-layer brim, travel-avoid skip points) have no analogue in this port's rectangular scan-line tower are declared + emitted-surface-only (user-supplied values reach the G-code CONFIG_BLOCK via the existing extensions bucket), with each absent decision point recorded — building the interface/ramming tower is future work, not this packet. No host-crate logic changes, no WIT/IR shape change, no schema bump, no new module. Full lists in `requirements.md`.

## Prerequisites and Blockers

- Depends on: wayfinder tickets 06 + 100 (both resolved — the packet-number rule and the wipe-tower rename workstream whose `bed_shape` → `printable_area` value-format adaptation this manifest already carries), and ticket 05 for key membership.
- Unblocks: the next packet in the feature-gap queue (per `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` order) and P03 (ticket 10), which shares the wipe-tower owner.
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** the `wipe-tower` manifest after this packet, **when** its `[config.schema]` is parsed, **then** it declares exactly 21 keys: the 8 pre-existing ones (`enable_prime_tower`, `wipe_tower_x`, `wipe_tower_y`, `prime_tower_width`, `prime_volume`, `line_width`, `printable_area`, `retract_length`) **plus** the 13 P02 keys carrying Orca defaults — `enable_filament_ramming` true, `enable_tower_interface_cooldown_during_tower` false, `enable_tower_interface_features` false, `prime_tower_enable_framework` false, `prime_tower_flat_ironing` false, `prime_tower_skip_points` true (all bool), `prime_tower_brim_width` float 3.0 bounded `[-1, …]` (−1 = canonical "Auto" sentinel), `prime_tower_infill_gap` percent `"150%"` bounded `[100, …]`, `filament_tower_interface_pre_extrusion_dist` float 10.0 bounded `[0, …]`, `filament_tower_interface_pre_extrusion_length` float 0.0 bounded `[0, …]`, `filament_tower_interface_print_temp` int −1 bounded `[-1, …]`, `filament_tower_interface_purge_volume` float 20.0 bounded `[0, …]`, `filament_tower_ironing_area` float 4.0 bounded `[0, …]`. | `cargo test -p wipe-tower --test wipe_tower_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-2. Given** the wired `prime_tower_infill_gap`, **when** the module generates tower purge paths, **then** the scan-line advance is `(percent/100) × line_width` (read as `ConfigValue::Percent`, falling back to the 150×0.4 default), so a config of `"200%"` doubles the pitch and one of `"110%"` yields `1.1 × line_width`; the pitch is never below `line_width` for any accepted value; and with no config entry the module uses the schema default (pitch `0.6` mm at `line_width = 0.4`), replacing today's hardcoded `line_width` advance. Pitch-dependent baseline assertions in `modules/core-modules/wipe-tower/tests/` are updated to the canonical-formula values. | `cargo test -p wipe-tower 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3. Given** the scheduler's bounds index built from the loaded `wipe-tower` manifest, **when** config resolution runs, **then** `prime_tower_infill_gap = 99%` and `prime_tower_brim_width = -2.0` are rejected with out-of-range errors naming the key, while `prime_tower_infill_gap = 110%` and the `prime_tower_brim_width` Auto sentinel `-1.0` are accepted; and the percent-typed schema default `150%` is threaded into `ResolvedConfig.extensions` as `ConfigValue::Percent(150.0)` when the profile supplies no value (the packet-185 transport path), whereas the bool/float-typed defaults (e.g. `prime_tower_brim_width = 3.0`) do **not** enter `extensions` — they stay manifest-side and are applied by the module's read fallback. | `cargo test -p slicer-scheduler --test wipe_tower_config_bounds_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4 (docs).** `docs/15_config_keys_reference.md` regenerated tables list all 13 keys under owner `wipe-tower` (module-config-keys table), and its Orca-deviations table gains no new row for them (all defaults match Orca; the scan-line-basis divergence recorded in `design.md` is behavioral, not a default deviation). | `cargo xtask gen-config-docs --check 2>&1 | tail -3 && rg -q 'prime_tower_infill_gap' docs/15_config_keys_reference.md && rg -q 'enable_filament_ramming' docs/15_config_keys_reference.md && echo AC4-PASS`

## Negative Test Cases

- **AC-N1. Given** the bounds index, **when** a config source sets `wipe-tower`'s `prime_tower_infill_gap` to `99` (below its `[100, …]` bound) or `prime_tower_brim_width` to `-2.0` (below `[-1, …]`), **then** resolution rejects the value with the existing `ConfigBoundsIndex::check` out-of-range error naming the key — the new declarations are bounds-enforced, not inert. | `cargo test -p slicer-scheduler --test wipe_tower_config_bounds_tdd -- rejects_out_of_range_prime_tower_values 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N2. Given** a core module other than `wipe-tower` (e.g. `part-cooling`, whose manifest declares none of the 13 keys), **when** it receives resolved config containing `prime_tower_infill_gap`, **then** `ConfigView::from_declared` still hides the key from it — the declarations leak no wipe-tower config into modules that did not opt in. | `cargo test -p slicer-runtime --test integration -- undeclared_prime_tower_keys_stay_hidden_from_other_modules 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Verification

Gate commands (the authoritative full matrix lives in `requirements.md` §Verification Commands):

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-scheduler --test wipe_tower_config_bounds_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Authoritative Docs

- `docs/specs/orca-feature-gap/issues/09-author-packet-p02-multimaterial-prime-tower-wipe-tower.md` — the wayfinder ticket defining this packet's scope (direct read, 23 lines).
- `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` — P02 row: 13 keys, Tier A (ranged read, ~10 lines around the P02 heading).
- `docs/specs/orca-feature-gap/issues/02-parity-evidence-standard.md` — evidence standard (direct read, ~80 lines).
- `docs/15_config_keys_reference.md` — regeneration target; never hand-edited (delegated regen + grep verification only).

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` module-config-keys table - regenerated by `cargo xtask gen-config-docs`; verification grep: `rg -q 'prime_tower_infill_gap' docs/15_config_keys_reference.md && rg -q 'filament_tower_ironing_area' docs/15_config_keys_reference.md`
- `docs/01_project_overview.md` - amend any prose describing the wipe tower's scan-line advance as equal to `line_width`; verification grep: `rg -n 'wipe.tower' docs/01_project_overview.md | grep -i 'scan.line\|advance' || echo AC4-docs-PASS`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet (the checkout is the **sibling** path `F:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented` — not `./OrcaSlicerDocumented`):

- `src/libslic3r/PrintConfig.cpp` — `PrintConfigDef`: the 13 declarations, types, defaults, and bounds (including `prime_tower_brim_width`'s `min = -1` with the f_enum_open "Auto" sentinel, and `prime_tower_infill_gap`'s `min = 100`).
- `src/libslic3r/GCode/WipeTower.cpp` — the constructor around `m_perimeter_width`/`m_extra_spacing` initialization: `m_extra_spacing = config.prime_tower_infill_gap.value/100` and `m_perimeter_width = nozzle_diameter × Width_To_Nozzle_Ratio`; `align_perimeter` and the wipe-path `dy` sites: the scan-line spacing formula `m_extra_spacing × m_perimeter_width`.
- `src/libslic3r/GCode/WipeTower2.cpp` — `toolchange_ChangeExtruder`: the interface-feature cluster (`enable_tower_interface_features`, temp/purge/pre-extrusion/ironing-area per-filament parameters) whose decision points are absent here; `toolchange_Unload`: the ramming sequence gated by `enable_filament_ramming`; `compute_wall_skip_points`: the internal skip-points vector (config key is a bool, not a point list).
- `src/libslic3r/GCode.cpp` — `_do_export`: `prime_tower_skip_points` gating travel-avoid-perimeter emission.
- `src/libslic3r/Print.cpp` — `validate`: the 13 keys as re-slice-invalidating config (evidence they are live, not dead-in-canonical).
<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).