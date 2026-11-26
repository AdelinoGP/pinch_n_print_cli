---
status: draft
packet: 286-seam-slope-wipe-emitter
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/60-author-packet-p53-quality-seam-emitter.md (wayfinder map: Close the OrcaSlicer FFF feature gap)
context_cost_estimate: M
copy_note: Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; authored under ticket 60 (P53).
---

# Packet Contract: 286-seam-slope-wipe-emitter

## Goal

Make the eight P53 seam-slope/loop-wipe keys drive host-side emission at parity with canonical `GCode::extrude_loop` sloped-seam and loop-wipe behaviour — ported as an additive generalisation of packet 285's scarf stage plus an independent loop-end wipe site inside `crates/slicer-gcode`, with no new module, IR field, or WIT change.

## Scope Boundaries

P53 is eight Tier B keys owned by the host emitter (`crates/slicer-gcode`). All eight are zero-occurrence as behaviour on HEAD: six have no occurrence anywhere under `crates/`/`modules/`/`xtask/`; `seam_slope_type` and `wipe_on_loops` occur only as `ORCA_CONFIG_PADDING` twins (rule 2: not evidence). The packet builds the missing decision points where the port already emits — the per-entity loop in `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`) — as scalar-global `ResolvedConfig` fields at canonical defaults. Canonical declares all eight scalar (verified against the map oracle at authoring), so no per-tool vector model rides ticket 125. One intended default CONFIG_BLOCK value change: the live `wipe_on_loops = false` shadows the stale `("wipe_on_loops", "1")` padding twin (count unchanged).

## Prerequisites and Blockers

- Depends on: none for authoring. Implementation sequences after draft packet `285-seam-scarf-joint-emitter` lands (this packet generalises its scarf stage additively; names reconciled in `design.md`) — activation-blocked until 285 lands.
- Unblocks: wayfinder ticket 60 (P53 closes alone). No dependency edge to draft packet `277-retraction-wipe-travel-firmware-emitter`: the loop-wipe site here fires at loop end, 277's wipe `Move` fires at retracts; a loop end coinciding with a retract emits at most one wipe (precedence pinned by AC-6).
- Activation blockers: packet 285 landed. All symbols below are live on HEAD (verified 2026-09-07); DEV-178 is the next collision-free ID (LOG max DEV-171; drafts propose DEV-172–DEV-177 — re-derive `max(DEV-*)` over `docs/DEVIATION_LOG.md` + `docs/spec_packets/*/` before writing the row).

## Acceptance Criteria

- **AC-1. Given** a default config, **when** the emitter schema is inspected, **then** all eight keys are declared as scalar-global with canonical defaults — `seam_slope_type` enum `none` (`none`/`external`/`all`), `seam_slope_start_height` float-or-percent `0` (`min 0`, no max), `seam_slope_entire_loop` bool `false`, `seam_slope_min_length` float `20.0` (`min 0`, no max), `seam_slope_steps` int `10` (`min 1`, no max), `seam_slope_inner_walls` bool `false`, `wipe_before_external_loop` bool `false`, `wipe_on_loops` bool `false` — with matching `docs/config/host-keys.toml` `[resolved_config]` rows and `machine-gcode-emit.toml` `[config.schema.*]` tables. | `cargo test -p slicer-gcode --test seam_slope_wipe_emission_tdd schema_declares_all_eight_keys 2>&1 | tee target/test-output.log | tail -5`
- **AC-2. Given** default config (`seam_slope_type = none`, both wipe keys `false`), **when** a closed-loop fixture is emitted, **then** zero sloped segments and zero loop-wipe moves emit — geometry byte-identical to the pre-packet path — and the CONFIG_BLOCK keeps its line count with exactly one value change (the live `wipe_on_loops` shadows the `("wipe_on_loops", "1")` padding twin via `emit_config_kv` dedup; bool spelling rides ticket 132). | `cargo test -p slicer-gcode --test seam_slope_wipe_emission_tdd default_path_no_slope_no_wipe 2>&1 | tee target/test-output.log | tail -5`
- **AC-3. Given** `seam_slope_type = external` (all else default, 285's stage landed), **when** an external loop and an inner-wall loop are emitted, **then** the external loop's seam region emits a sloped ramp and the inner loop does not; with `seam_slope_type = none` neither ramps; with `seam_slope_type = all` both ramp subject to AC-5's inner gate. | `cargo test -p slicer-gcode --test seam_slope_wipe_emission_tdd slope_type_gates_which_loops_ramp 2>&1 | tee target/test-output.log | tail -5`
- **AC-4. Given** `seam_slope_type = external`, **when** the fixture is emitted with `seam_slope_steps = 5` versus `10`, **then** the sloped region emits strictly fewer moves at 5 than at 10; with a loop shorter than `seam_slope_min_length` no ramp emits; with `seam_slope_entire_loop = true` versus `false` the emitted paths differ (whole-loop ramp versus seam-region ramp). | `cargo test -p slicer-gcode --test seam_slope_wipe_emission_tdd steps_length_and_entire_loop_modulate_ramp 2>&1 | tee target/test-output.log | tail -5`
- **AC-5. Given** `seam_slope_type = all`, **when** an inner-wall loop is emitted with `seam_slope_inner_walls = false` versus `true`, **then** the loop ramps only at `true`; with `seam_slope_start_height` non-zero versus `0` the sloped output differs at equal move count (shifted ramp start). | `cargo test -p slicer-gcode --test seam_slope_wipe_emission_tdd inner_walls_and_start_height_modulate 2>&1 | tee target/test-output.log | tail -5`
- **AC-6. Given** `wipe_on_loops = true`, **when** an external loop is emitted, **then** exactly one inward wipe move emits before leaving the loop; with `wipe_before_external_loop = true` an inward move emits before the external loop when neighbouring perimeters qualify; with both `false` none emit; with a loop end coinciding with a retract site at most one wipe emits (loop-wipe precedence over 277's retract wipe). | `cargo test -p slicer-gcode --test seam_slope_wipe_emission_tdd loop_wipes_emit_at_loop_boundaries 2>&1 | tee target/test-output.log | tail -5`

## Negative Test Cases

- **AC-N1. Given** out-of-range values (`seam_slope_steps = 0`, `seam_slope_min_length = -1`, `seam_slope_start_height = -1`, `seam_slope_type = "diagonal"`), **when** the slice is validated, **then** each is rejected with a stable bounds error naming the key (ticket-113 reject-the-slice rule; the four bools carry no range — canonical bounds are GUI hints — DEV-178(a)). | `cargo test -p slicer-gcode --test seam_slope_wipe_emission_tdd out_of_range_values_reject 2>&1 | tee target/test-output.log | tail -5`
- **AC-N2. Given** a path carrying `order_lock` (ADR-0062/0063), **when** emission runs with sloping enabled and both wipe keys `true`, **then** the locked path is emitted unramped and unwiped (producer-guaranteed footprint; the emitter neither reshapes nor adds moves to it), while neighbouring unlocked loops ramp and wipe normally. | `cargo test -p slicer-gcode --test seam_slope_wipe_emission_tdd locked_paths_bypass_slope_and_wipe 2>&1 | tee target/test-output.log | tail -5`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-gcode --test seam_slope_wipe_emission_tdd 2>&1 | tee target/test-output.log | tail -5`

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (modular pipeline + community extensibility constraint on the slope/wipe stage shape)
- `docs/08_coordinate_system.md` - direct range read (mm↔unit boundary for the min-length/start-height math)

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` section "Host keys" - `rg -q 'seam_slope_type' docs/15_config_keys_reference.md` (regenerated by `cargo xtask gen-config-docs` in Step 1b; freshness pinned by `cargo xtask gen-config-docs --check` in Step 4)
- `docs/config/host-keys.toml` section "[resolved_config]" - `rg -q 'seam_slope_type' docs/config/host-keys.toml`
- `docs/DEVIATION_LOG.md` section "DEV-178" - `rg -q 'DEV-178' docs/DEVIATION_LOG.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::extrude_loop` slope gating order (`seam_slope_type` loop match, `seam_slope_min_length` gate, `seam_slope_inner_walls` gate) and `ExtrusionLoopSloped` ramp construction from `seam_slope_steps`/`seam_slope_start_height`/`seam_slope_entire_loop` (borrow the gate order; port it emitter-side onto 285's stage)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::extrude_loop` loop-wipe emission (`wipe_before_external_loop` pre-external move, `wipe_on_loops` pre-leave move, neighbour-qualification shape) (borrow the trigger sites; the retract-wipe split stays with draft 277)
- `OrcaSlicerDocumented/src/libslic3r/Layer.cpp` — `Layer::is_perimeter_compatible` slope-compatibility grouping (decide whether the emitter needs the grouping or the loop-type gate suffices; record as divergence if dropped)
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the eight keys' declared types/defaults/bounds (confirm, do not re-derive the packet's table without this read)

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
