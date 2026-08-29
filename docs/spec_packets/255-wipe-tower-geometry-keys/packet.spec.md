---
status: draft
packet: 255-wipe-tower-geometry-keys
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/10-author-packet-p03-multimaterial-prime-tower-wipe-tower.md (wayfinder map: Close the OrcaSlicer FFF feature gap — packet P03)
context_cost_estimate: M
---

# Packet Contract: 255-wipe-tower-geometry-keys

This packet was authored per the useful grounding in `requirements.md` §Verified Grounding; all symbol paths below were verified against the current tree (and the canonical checkout, via delegation) at authoring time.

## Goal

Declare OrcaSlicer's P03 wipe-tower config keys in the `wipe-tower` module manifest with Orca-parity defaults and bounds, and wire the one key whose decision point exists in this tree — `wipe_tower_extra_flow`, multiplying the purge scan-line paths' `flow_factor` — while recording honest decision-point gaps for the other declared keys, exactly as packets 253/254 did. One key (`wipe_tower_max_purge_speed`) is **excluded as an alias finding**: the host feedrate key `wipe_tower_speed` (`FeedrateConfig`) already implements it (defaults both 90), and a module-side re-declaration would create the duplicate-spelling class that wayfinder ticket 107 collapses.

## Scope Boundaries

12 of the 13 P03 keys are declared in `modules/core-modules/wipe-tower/wipe-tower.toml`; `wipe_tower_extra_flow` (percent, default 100%, bounds [100, 300]) is additionally read by the module and becomes the scan-line purge paths' `flow_factor` (`(value/100)`, replacing the hardcoded `1.0` in `generate_purge_paths` — the entity set consumed by both the legacy `process()` path and the live `run_finalization()` path; the prime line and travel entity keep their flow). No output change at defaults (factor 1.0 is identity); the two percent-typed defaults add exactly 2 CONFIG_BLOCK lines via the packet-185 percent transport. The other 10 declared keys gate canonical tower geometry this port's rectangular scan-line tower does not have (cone/rib/fillet walls, bridging pass, rotation, wall types, flush routing, ramming spacing, sparse layers) and are declared + emitted-surface-only with recorded gaps. `wipe_tower_max_purge_speed` is not declared (alias finding; disposition in `requirements.md` §Per-key parity evidence). No host-crate logic changes, no WIT/IR shape change, no schema bump, no new module.

## Prerequisites and Blockers

- Depends on: wayfinder tickets 06 + 100 (both resolved — packet-number rule; wipe-tower rename workstream whose `printable_area` adaptation this manifest carries), and ticket 05 for key membership.
- Queue order: packet 254 (P02, same owner) precedes this packet; its 13 declarations land first. This packet's schema test asserts the union accordingly (precondition in Step 2).
- Unblocks: the next packet in `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` order.
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** the `wipe-tower` manifest after this packet, **when** its `[config.schema]` is parsed, **then** it declares the 8 pre-existing keys (`enable_prime_tower`, `wipe_tower_x`, `wipe_tower_y`, `prime_tower_width`, `prime_volume`, `line_width`, `printable_area`, `retract_length`) **plus** the 12 P03 keys with Orca defaults/bounds — `purge_in_prime_tower` bool true, `single_extruder_multi_material` bool true, `wipe_tower_bridging` float 10.0 unbounded, `wipe_tower_cone_angle` float 30.0 bounded [0, 90], `wipe_tower_extra_flow` percent `"100%"` bounded [100, 300], `wipe_tower_extra_rib_length` float 0.0 max 300 (no min), `wipe_tower_extra_spacing` percent `"100%"` bounded [100, 300], `wipe_tower_fillet_wall` bool true, `wipe_tower_no_sparse_layers` bool false, `wipe_tower_rib_width` float 8.0 bounded [0, 300], `wipe_tower_rotation_angle` float 0.0 unbounded, `wipe_tower_wall_type` enum `["rectangle", "cone", "rib"]` default `"rib"` — and does **not** declare `wipe_tower_max_purge_speed`. | `cargo test -p wipe-tower --test wipe_tower_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-2. Given** the wired `wipe_tower_extra_flow`, **when** `generate_purge_paths` builds purge entities, **then** every scan-line path's `flow_factor` equals the configured factor (`"150%"` → 1.5, `"200%"` → 2.0 on both scan-line points), with **no** config entry the factor is exactly 1.0 (the manifest default identity — byte-identical geometry at defaults), the travel entity's `flow_factor` stays 0.0 and the prime entity's stays 1.0 in all cases, and values outside [100, 300] cannot reach the module (bounds-rejected upstream). | `cargo test -p wipe-tower 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-3. Given** the scheduler's bounds index built from the loaded `wipe-tower` manifest, **when** config resolution runs, **then** both percent-typed schema defaults thread into `ResolvedConfig.extensions` as `ConfigValue::Percent` (`wipe_tower_extra_flow` → `Percent(100.0)`, `wipe_tower_extra_spacing` → `Percent(100.0)`) under an empty source while the non-percent defaults (e.g. `wipe_tower_cone_angle`, `wipe_tower_wall_type`) stay manifest-side and absent from `extensions`; `wipe_tower_extra_flow = 99%` and `301%` are rejected with out-of-range errors naming the key while `100%` and `300%` are accepted; and a `wipe_tower_wall_type` value outside its enum domain (e.g. `"hexagon"`) is rejected by enum-membership check while `"rib"` is accepted. | `cargo test -p slicer-scheduler --test wipe_tower_p03_config_bounds_tdd 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-4 (docs).** `docs/15_config_keys_reference.md` regenerated tables list the 12 declared keys under owner `wipe-tower` (module-config-keys table), and its Orca-deviations table gains no new row for them (all defaults match Orca; the sparse-layer and max-purge-speed notes in `requirements.md` are behavioral/alias dispositions, not default deviations). | `cargo xtask gen-config-docs --check 2>&1 | tail -3 && rg -q 'wipe_tower_wall_type' docs/15_config_keys_reference.md && rg -q 'wipe_tower_extra_flow' docs/15_config_keys_reference.md && echo AC4-PASS`

## Negative Test Cases

- **AC-N1. Given** the bounds index, **when** a config source sets `wipe-tower`'s `wipe_tower_extra_flow` to `99%` (below its `[100, 300]` bound) or to `301%` (above it), **then** resolution rejects the value with the existing out-of-range error naming the key — the percent declarations are bounds-enforced, not inert. | `cargo test -p slicer-scheduler --test wipe_tower_p03_config_bounds_tdd -- rejects_out_of_range_extra_flow 2>&1 | tee target/test-output.log | grep -E "^test result"`
- **AC-N2. Given** a core module other than `wipe-tower` (e.g. `part-cooling`, whose manifest declares none of the 12 keys), **when** it receives resolved config containing `wipe_tower_extra_flow`, **then** `ConfigView::from_declared` still hides the key from it — the declarations leak no wipe-tower config into modules that did not opt in. | `cargo test -p slicer-runtime --test integration -- undeclared_p03_wipe_tower_keys_stay_hidden_from_other_modules 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Verification

Gate commands (the authoritative full matrix lives in `requirements.md` §Verification Commands):

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p wipe-tower 2>&1 | tee target/test-output.log | grep -E "^test result"`

## Authoritative Docs

- `docs/specs/orca-feature-gap/issues/10-author-packet-p03-multimaterial-prime-tower-wipe-tower.md` — the wayfinder ticket defining this packet's scope (direct read).
- `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` — P03 row: 13 keys, Tier A (ranged read, ~10 lines around the P03 heading).
- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` — the 13 Tier A rows (ranged read ~15 lines; over 300 lines total: delegate beyond these rows).
- `docs/specs/orca-feature-gap/issues/02-parity-evidence-standard.md` — evidence standard (direct read).
- `docs/15_config_keys_reference.md` — regeneration target; never hand-edited (delegated regen + grep verification only).

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` module-config-keys table — regenerated by `cargo xtask gen-config-docs`; verification grep: `rg -q 'wipe_tower_wall_type' docs/15_config_keys_reference.md && rg -q 'wipe_tower_extra_flow' docs/15_config_keys_reference.md`
- No prose doc claims the port's purge-path flow multiplier is a fixed 1.0 today; if implementation finds one (grep `flow_factor` under `docs/`), it names the packet in its amendment.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet (the checkout is the **sibling** path `F:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented` — not `./OrcaSlicerDocumented`):

- `src/libslic3r/PrintConfig.cpp` — `PrintConfigDef`: the 13 declaration facts (types, defaults, bounds, enum options) quoted in `requirements.md` §Per-key parity evidence.
- `src/libslic3r/GCode/WipeTower2.cpp` — `WipeTower2` constructor (member reads: `m_bridging`, `m_wipe_tower_cone_angle`, `m_extra_flow`, `m_extra_spacing_wipe`/`m_extra_spacing_ramming`, `m_wall_type`/`use_gap_wall`, `m_max_speed`, `m_no_sparse_layers`, `m_rib_length`, rotation) and `toolchange_Wipe` (the `m_extra_flow` flow multiplier — the behavior this packet's one wiring mirrors), `finish_layer` (bridging spacing + wall-type selection + max-speed feedrate capping), `generate` (`m_extra_rib_length`), `extract_wipe_volumes` (`purge_in_prime_tower` / `single_extruder_multi_material` flush zeroing).
- `src/libslic3r/GCode/WipeTower.cpp` — legacy `WipeTower` constructor (same member cluster; confirms the keys are not Type2-only).
- `src/libslic3r/GCode.cpp` — `WipeTowerIntegration::tool_change` (`wipe_tower_no_sparse_layers` consumption outside the tower class).
<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).