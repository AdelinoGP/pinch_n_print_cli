---
status: draft
packet: 285-seam-scarf-joint-emitter
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/59-author-packet-p52-quality-seam-emitter.md (wayfinder map: Close the OrcaSlicer FFF feature gap)
context_cost_estimate: M
copy_note: Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; authored under ticket 59 (P52).
---

# Packet Contract: 285-seam-scarf-joint-emitter

## Goal

Make the eight P52 seam/scarf keys drive host-side emission at parity with canonical `GCode::extrude_loop` / `GCode::_extrude` scarf-joint, seam-gap, and wipe-speed behaviour — ported as a PnP scarf/slope emission stage inside `crates/slicer-gcode`, with no new module, IR field, or WIT change.

## Scope Boundaries

P52 is eight Tier B keys owned by the host emitter (`crates/slicer-gcode`). All eight are zero-occurrence as behaviour on HEAD: seven have no occurrence anywhere under `crates/`/`modules/`/`xtask/`; `seam_gap` occurs only as the `("seam_gap", "10%")` padding twin (rule 2: not evidence) and an unrelated wave-overhangs test fn name. The packet builds the missing decision points where the port already emits — the per-entity loop in `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`) and the `resolve_feedrate` role-speed seam — as scalar-global `ResolvedConfig` fields at canonical defaults. Canonical declares all eight scalar (verified against the map oracle at authoring), so no per-tool vector model rides ticket 125. One intended default output change: `seam_gap` clipping goes live at its canonical 10% default (loops shorten; count unchanged). Everything else is byte-identical at defaults.

## Prerequisites and Blockers

- Depends on: none for authoring. Implementation of the `role_based_wipe_speed` arm carries a FORWARD-DEP on draft packet `277-retraction-wipe-travel-firmware-emitter` (its wipe `Move` is the only wipe subject; names reconciled in `design.md`).
- Unblocks: wayfinder ticket 59 (P52 closes alone). P53's slope-variant/wipe keys (queued under ticket 60, not yet authored) extend this packet's slope stage when authored; no dependency edge is added — that relationship is a sequencing note, not a block.
- Activation blockers: none. All symbols below are live on HEAD (verified 2026-09-07); DEV-177 is the next collision-free ID (LOG max DEV-171; drafts propose DEV-172–DEV-176 — re-derive `max(DEV-*)` over `docs/DEVIATION_LOG.md` + `docs/spec_packets/*/` before writing the row).

## Acceptance Criteria

- **AC-1. Given** a default config, **when** the emitter schema is inspected, **then** all eight keys are declared as scalar-global with canonical defaults — `has_scarf_joint_seam` bool `false`, `seam_slope_conditional` bool `false`, `role_based_wipe_speed` bool `true`, `scarf_angle_threshold` int `155` (`min 0`, `max 180`), `scarf_joint_flow_ratio` float `1.0` (`min 0`, `max 2`), `scarf_joint_speed` float-or-percent `100%` (`min 1`), `scarf_overhang_threshold` percent `40%` (`min 0`), `seam_gap` float-or-percent `10%` (`min 0`) — with matching `docs/config/host-keys.toml` `[resolved_config]` rows and `machine-gcode-emit.toml` `[config.schema.*]` tables. | `cargo test -p slicer-gcode --test seam_scarf_joint_emission_tdd schema_declares_all_eight_keys 2>&1 | tee target/test-output.log | tail -5`
- **AC-2. Given** default config (`has_scarf_joint_seam = false`, `seam_gap = 10%`), **when** a closed-loop fixture is emitted, **then** zero scarf paths emit, and every loop's start/end is clipped by exactly the resolved gap (10% of nozzle diameter) versus the pre-packet path — same move count, shortened terminal segments — while the CONFIG_BLOCK gains no lines (the live key shadows the untouched `("seam_gap", "10%")` padding twin via `emit_config_kv` dedup). | `cargo test -p slicer-gcode --test seam_scarf_joint_emission_tdd default_path_clips_gap_no_scarf 2>&1 | tee target/test-output.log | tail -5`
- **AC-3. Given** `has_scarf_joint_seam = true` with all other defaults, **when** the same closed-loop fixture is emitted, **then** scarf paths emit (overlapped sloped seam segments replacing the sharp start/end), scarf segments carry flow factor exactly `1.0` and capped speed exactly the role speed (100% default), and the move count is strictly greater than the AC-2 count. | `cargo test -p slicer-gcode --test seam_scarf_joint_emission_tdd scarf_enable_emits_sloped_paths 2>&1 | tee target/test-output.log | tail -5`
- **AC-4. Given** `has_scarf_joint_seam = true` + `seam_slope_conditional = true`, **when** a smooth loop and a sharp-corner loop are emitted, **then** the smooth loop (all corner angles above `scarf_angle_threshold = 155°`) scars and the sharp loop does not; with `seam_slope_conditional = false` both scar. | `cargo test -p slicer-gcode --test seam_scarf_joint_emission_tdd conditional_gates_on_smoothness 2>&1 | tee target/test-output.log | tail -5`
- **AC-5. Given** `has_scarf_joint_seam = true` + `scarf_joint_flow_ratio = 0.5` + `scarf_joint_speed = 50%`, **when** the fixture is emitted, **then** every scarf segment carries flow factor exactly `0.5` and capped speed exactly half the role speed; with `scarf_joint_speed = 30` (absolute mm/s) the cap is exactly `30` mm/s (`1800` mm/min feedrate). | `cargo test -p slicer-gcode --test seam_scarf_joint_emission_tdd scarf_flow_and_speed_modulate 2>&1 | tee target/test-output.log | tail -5`
- **AC-6. Given** draft packet 277's wipe `Move` landed, **when** a wipe emits with `role_based_wipe_speed = true` (default), **then** the wipe `Move` feedrate equals the emitting role's speed; with `role_based_wipe_speed = false` it equals `FeedrateConfig::wipe_speed`. (FORWARD-DEP on draft `277-retraction-wipe-travel-firmware-emitter`; activation-blocked until 277 lands.) | `cargo test -p slicer-gcode --test seam_scarf_joint_emission_tdd wipe_speed_selects_role_or_configured 2>&1 | tee target/test-output.log | tail -5`

## Negative Test Cases

- **AC-N1. Given** out-of-range values (`scarf_angle_threshold = 200`, `scarf_joint_flow_ratio = 3.0`, `scarf_joint_speed = 0`, `scarf_overhang_threshold = -1`, `seam_gap = -1`), **when** the slice is validated, **then** each is rejected with a stable bounds error naming the key (ticket-113 reject-the-slice rule; the three bools carry no range — canonical bounds are GUI hints — DEV-177(c)). | `cargo test -p slicer-gcode --test seam_scarf_joint_emission_tdd out_of_range_values_reject 2>&1 | tee target/test-output.log | tail -5`
- **AC-N2. Given** a path carrying `order_lock` (ADR-0062/0063), **when** emission runs with scarf enabled and `seam_gap = 10%`, **then** the locked path is emitted unclipped and unscarved (producer-guaranteed footprint; the emitter neither clips nor reshapes it), while neighbouring unlocked loops clip and scar normally. | `cargo test -p slicer-gcode --test seam_scarf_joint_emission_tdd locked_paths_bypass_gap_and_scarf 2>&1 | tee target/test-output.log | tail -5`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-gcode --test seam_scarf_joint_emission_tdd 2>&1 | tee target/test-output.log | tail -5`

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (modular pipeline + community extensibility constraint on the scarf stage shape)
- `docs/08_coordinate_system.md` - direct range read (mm↔unit boundary for the gap/length math)

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` section "Host keys" - `rg -q 'has_scarf_joint_seam' docs/15_config_keys_reference.md` (regenerated by `cargo xtask gen-config-docs` in Step 1b; freshness pinned by `cargo xtask gen-config-docs --check` in Step 6)
- `docs/config/host-keys.toml` section "[resolved_config]" - `rg -q 'has_scarf_joint_seam' docs/config/host-keys.toml`
- `docs/DEVIATION_LOG.md` section "DEV-177" - `rg -q 'DEV-177' docs/DEVIATION_LOG.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::extrude_loop` scarf/slope gating, smoothness test, overhang test, and seam-gap clipping shape (borrow the gate order; port it emitter-side)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::_extrude` scarf flow-ratio multiplication and scarf-speed capping shape (borrow the factor/cap application point)
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `Wipe::wipe` + `Wipe::calculateWipeRetractionLengths` role-speed-vs-configured selection (reconcile with draft 277's wipe `Move` before wiring AC-6)
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the eight keys' declared types/defaults/bounds (confirm, do not re-derive the packet's table without this read)

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
