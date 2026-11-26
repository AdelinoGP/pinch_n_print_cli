---
status: draft
packet: 283-printer-timing-emitter
task_ids: []
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
copy_note: Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; authored under ticket 56 (P49).
---

# Packet Contract: 283-printer-timing-emitter

## Goal

Make `machine_load_filament_time`, `machine_unload_filament_time`, `machine_tool_change_time`, and `time_cost` drive host-side print-time estimation and footer statistics at parity with canonical `GCodeProcessor::process_filament_change` and `GCode::update_print_estimated_stats`.

## Scope Boundaries

P49 is four Tier B keys owned by the host emitter (`crates/slicer-gcode`). All four are zero-occurrence in `crates/`, `modules/`, and `xtask/`; the packet builds the missing decision points inside the print-time estimator (`estimate_command_deltas`'s `ToolChange` arm, today count-only) and the footer stats block (`filament_stats_comment_block`) as scalar-global `ResolvedConfig` fields in the adjacent machine-key shape (`cli_opt @printer`, `Option<f32> = None`). No new module, no IR/WIT change, no per-tool vector model (canonical declares all four scalar `coFloat`; the ticket-125 ruling is not engaged).

## Prerequisites and Blockers

- Depends on: none. No adjacent draft packet shares a code surface (`filament_cost` is Tier D deferred — this packet builds printer-cost only, not the canonical filament-cost total).
- Unblocks: nothing downstream. P49 closes alone.
- Activation blockers: none. All symbols below are live on HEAD (verified 2026-09-07); DEV-175 is the next collision-free ID (LOG + packets max DEV-174; drafts 276/277/281 propose DEV-171/172/173 and 282 proposes DEV-174).

## Acceptance Criteria

- **AC-1. Given** a default config, **when** the emitter schema is inspected, **then** `machine_load_filament_time`, `machine_unload_filament_time`, `machine_tool_change_time` (seconds) and `time_cost` (money/h) are declared as scalar-global `ResolvedConfig` fields defaulting to effective `0.0`, with matching `docs/config/host-keys.toml` `[resolved_config]` rows. | `cargo test -p slicer-gcode --test printer_timing_stats_tdd schema_declares_four_keys 2>&1 | tail -5`
- **AC-2. Given** default config (all four unset), **when** a multi-tool `GCodeIR` stream is estimated, **then** `total_time_s`, the per-command elapsed timeline, the footer comment block, and the CONFIG_BLOCK are byte-identical to HEAD (unset `None` is omitted from `to_config_map`, so no padding-twin shadowing is needed and none is added). | `cargo test -p slicer-gcode --test printer_timing_stats_tdd default_path_is_identity 2>&1 | tail -5`
- **AC-3. Given** `machine_load_filament_time = 2.0`, `machine_unload_filament_time = 3.0`, `machine_tool_change_time = 5.0`, **when** a stream with exactly one `ToolChange` is estimated, **then** `total_time_s` exceeds the unset-config total by exactly `10.0` seconds (plain-sum PnP simplification of the canonical conditional table — DEV-175(a)). | `cargo test -p slicer-gcode --test printer_timing_stats_tdd toolchange_adds_configured_sum 2>&1 | tail -5`
- **AC-4. Given** the AC-3 config, **when** the elapsed timeline is built, **then** every per-command elapsed value at and after the `ToolChange` index is shifted by exactly `10.0` seconds relative to the unset run (M73 `R`/`S` remaining minutes move with the estimate). | `cargo test -p slicer-gcode --test printer_timing_stats_tdd elapsed_shifts_after_toolchange 2>&1 | tail -5`
- **AC-5. Given** `time_cost = 36.0` and a print whose estimated total is `T` seconds, **when** the footer stats block is built, **then** it contains a printer-cost line whose value equals `36.0 * T / 3600.0` formatted to two decimals (canonical `update_print_estimated_stats` formula verbatim). | `cargo test -p slicer-gcode --test printer_timing_stats_tdd printer_cost_line_value 2>&1 | tail -5`
- **AC-6. Given** `time_cost` unset (default), **when** the footer stats block is built, **then** it contains no cost line (canonical emits no printer-cost footer label — the label is PnP-invented, DEV-175(b)). | `cargo test -p slicer-gcode --test printer_timing_stats_tdd no_cost_line_at_default 2>&1 | tail -5`
- **AC-7. Given** the generated config reference, **when** inspected after `cargo xtask gen-config-docs`, **then** the four keys appear with effective default `0.0` and no new default-deviation row (DEV-175 is a behaviour record, not a default mismatch). | `cargo test -p slicer-runtime --test unit host_keys_doc_lock 2>&1 | tail -3`

## Negative Test Cases

- **AC-N1. Given** any of the three timing keys negative (e.g. `machine_tool_change_time = -1.0`), **when** the emitter resolves, **then** the slice is rejected with a stable validation error (min-0 enforcement; canonical never enforces — DEV-175(c)). | `cargo test -p slicer-gcode --test printer_timing_stats_tdd negative_time_rejected 2>&1 | tail -5`
- **AC-N2. Given** `time_cost = -5.0`, **when** the emitter resolves, **then** the slice is rejected with a stable validation error. | `cargo test -p slicer-gcode --test printer_timing_stats_tdd negative_time_cost_rejected 2>&1 | tail -5`
- **AC-N3. Given** the packet lands, **when** `ORCA_CONFIG_PADDING` in `crates/slicer-gcode/src/serialize.rs` is inspected, **then** no timing/cost row was added (rule 2: padding is never evidence; unset keys are omitted from the resolved config, set keys thread via `to_config_map`). | `rg -q 'machine_load_filament_time|machine_tool_change_time|machine_unload_filament_time|time_cost' crates/slicer-gcode/src/serialize.rs && exit 1 || exit 0`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-gcode --test printer_timing_stats_tdd 2>&1 | tail -5`

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (modular pipeline + community extensibility constraints on emitter changes)
- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - direct range read of P49 rows only (tier B, owner `crates/slicer-gcode (estimator.rs)`)
- `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` - direct range read of P49 section only (4-key membership)

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` section "Host keys" - `rg -q 'machine_tool_change_time' docs/15_config_keys_reference.md`
- `docs/config/host-keys.toml` section "[resolved_config]" - `rg -q 'machine_tool_change_time' docs/config/host-keys.toml`
- `docs/DEVIATION_LOG.md` section "DEV-175" - `rg -q 'DEV-175' docs/DEVIATION_LOG.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::init_fff_params`: the four declarations (all `coFloat`, default `0.0`; already grounded at authoring)
- `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeProcessor.cpp` — `GCodeProcessor::process_filament_change` (both overloads): the conditional per-change time composition quoted in `design.md`
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `GCode::update_print_estimated_stats`: the `time_cost * normal_print_time / 3600.0` printer-cost addition (already grounded verbatim at authoring)

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
