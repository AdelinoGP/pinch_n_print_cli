---
status: draft
packet: 175-m73-progress
task_ids:
  - TASK-279
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
depends_on:
  - .ralph/specs/169-time-estimator-slice-stats (wave-1, packet status `draft` — HARD prerequisite; this packet consumes its `estimate_print` / `PrintEstimate` / `EstimatorLimits` exports, which already exist in the working tree at `crates/slicer-gcode/src/estimator.rs` (:168 / :91 / :24) with matching shapes, and its emit-site wiring at `crates/slicer-gcode/src/emit.rs:757-758`. This packet MUST NOT be activated before 169's packet is marked `implemented`.)
plan_source: docs/specs/fork-gaps-wave2-plan.md (Packet 175 — fork handoff item 15)
---

# Packet Contract: 175-m73-progress

## Goal

Emit `M73 P<pct> R<remaining_min>` plus stealth `M73 Q<pct> S<remaining_min>` progress lines (same estimate for both masks) at stream start, every layer boundary, and stream end, and append the `; filament used [mm]/[cm3]/[g]` + `; estimated printing time` comment block — all driven by packet 169's trapezoidal estimator, gated by a new `disable_m73` config key (bool, default false).

## Scope Boundaries

All new logic lives in `crates/slicer-gcode` (a per-command elapsed-time extension of 169's estimator plus a new `m73` injection module) and is wired at the existing estimator call site inside `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs:757-758`, which fills `metadata.estimated_print_time_s`; `crates/slicer-runtime/src/postpass.rs:49-51` only stashes the already-filled IR and is NOT edited); `crates/slicer-ir/src/resolved_config.rs` gains the `disable_m73` key. No WIT changes, no new WASM module, no change to the estimator's physics, no G-code flavor logic (packet 171's concern), and no per-G1-line M73 (layer-boundary + first/last granularity only — a documented deviation from Orca's per-move emission).

## Prerequisites and Blockers

- Depends on: packet `169-time-estimator-slice-stats` (TASK-275, packet status **draft**). Its estimator symbols ALREADY EXIST in the working tree with the shapes this packet consumes: `pub fn estimate_print(gcode_ir: &GCodeIR, limits: &EstimatorLimits, tool_diameters: &BTreeMap<u32, f32>) -> PrintEstimate` (`crates/slicer-gcode/src/estimator.rs:168`), `pub struct PrintEstimate` (:91), `EstimatorLimits` (:24), and the emit-site fill of `metadata.estimated_print_time_s` (`crates/slicer-gcode/src/emit.rs:757-758`). The dependency is on 169 *closing* (its tests/ceremony), not on symbol creation.
- Unblocks: fork print-host progress display (handoff item 15); firmware remaining-time display.
- Activation blockers: packet 169 not yet `implemented` — do not activate this packet before 169's acceptance ceremony completes.

## Acceptance Criteria

- **AC-1. Given** a synthetic `GCodeIR` whose commands contain three `GCodeCommand::Raw { text: ";LAYER_CHANGE" }` markers separated by timed `Move` commands, **when** `inject_m73(&mut gcode_ir, &elapsed_s)` runs (elapsed from `estimate_print_with_elapsed`), **then** the serialized output's first `M73` line is `M73 P0 R<total_min>` (where `total_min = round(total_time_s/60)`), its last `M73` line is `M73 P100 R0`, one `M73 P.. R..` pair appears at each layer boundary whose (pct, min) differs from the previous emission, and `P` values are monotonically non-decreasing. | `mkdir -p target && cargo test -p slicer-gcode --test m73 -- layer_boundary_p_r_monotonic_first_last 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"`
- **AC-2. Given** the same injected stream, **when** the output is scanned, **then** every `M73 P<p> R<r>` line is immediately followed by a stealth `M73 Q<p> S<r>` line carrying the identical `<p>` and `<r>` values (same estimate for both masks; Orca reference masks `"M73 P%s R%s\n"` / `"M73 Q%s S%s\n"`). | `mkdir -p target && cargo test -p slicer-gcode --test m73 -- stealth_q_s_mirrors_p_r 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"`
- **AC-3. Given** a `PrintEstimate` with `filament_length_mm = {0: 1000.0, 1: 500.0}`, `extruded_volume_mm3 = {0: 2405.28, 1: 1202.64}`, `total_time_s = 3725.0`, and `filament_density = Some(1.24)`, **when** `filament_stats_comment_block` renders, **then** the block contains exactly the lines `; filament used [mm] = 1000.00, 500.00`, `; filament used [cm3] = 2.41, 1.20`, `; filament used [g] = 2.98, 1.49` (values = volume_cm3 × density, 2-decimal), and `; estimated printing time (normal mode) = 1h 2m 5s` (`get_time_dhms`-style, zero-leading units omitted). | `mkdir -p target && cargo test -p slicer-gcode --test m73 -- filament_stats_block_two_tools_with_density 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"`
- **AC-4. Given** a full slice through `run_slice` on a small fixture with default config, **when** the G-code file is written, **then** it contains at least one `M73 P0 R` line, a final-region `M73 P100 R0` line, an adjacent `M73 Q`/`S` stealth pair for each, and the `; filament used [mm]` and `; estimated printing time (normal mode)` comment lines. | `mkdir -p target && cargo test -p pnp-cli --test m73_progress_tdd -- slice_emits_m73_and_filament_comments 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"`
- **AC-5. Given** the docs are amended, **when** grepping, **then** `docs/15_config_keys_reference.md` documents `disable_m73` and the `disable_m73` *definition row* (the coBool row at line 863; the two category-listing rows at lines 1063/1750 carry no marker and are untouched) in `docs/ORCA_CONFIG_REFERENCE.md` no longer carries the ❌ marker. | `rg -q 'disable_m73' docs/15_config_keys_reference.md && ! rg -q '"disable_m73".*coBool.*❌' docs/ORCA_CONFIG_REFERENCE.md && echo PASS`

## Negative Test Cases

- **AC-N1. Given** config sets `disable_m73 = true`, **when** the same fixture slices, **then** the output contains zero lines starting with `M73` (neither P/R nor Q/S), while the `; filament used [mm]` and `; estimated printing time` comment lines are still present. | `mkdir -p target && cargo test -p pnp-cli --test m73_progress_tdd -- disable_m73_suppresses_m73_keeps_comments 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"`
- **AC-N2. Given** `filament_density` is absent from config, **when** the comment block renders, **then** the `; filament used [g]` line is omitted entirely (never `0.00`), while `; filament used [mm]`, `; filament used [cm3]`, and `; estimated printing time` lines remain present. | `mkdir -p target && cargo test -p slicer-gcode --test m73 -- filament_g_line_omitted_without_density 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `mkdir -p target && cargo test -p slicer-gcode --test m73 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"`

## Authoritative Docs

- `docs/15_config_keys_reference.md` — delegated grep only; gains the `disable_m73` row.
- `docs/ORCA_CONFIG_REFERENCE.md` — delegated grep for the `disable_m73` row (coBool, default 0, comAdvanced); flip its implemented marker.
- `.ralph/specs/169-time-estimator-slice-stats/design.md` — delegated SUMMARY of the estimator export names/shapes (already reconciled above); never re-derive.

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` — add the `disable_m73` key row (bool, default false, suppresses M73 emission only, comments unaffected) - `rg -q 'disable_m73' docs/15_config_keys_reference.md`
- `docs/ORCA_CONFIG_REFERENCE.md` — flip the `disable_m73` definition row's (coBool row, line 863) implemented marker from ❌ to ✅; the category-listing rows (1063/1750) carry no marker and stay untouched - `! rg -q '"disable_m73".*coBool.*❌' docs/ORCA_CONFIG_REFERENCE.md`
- `docs/07_implementation_status.md` — add the TASK-279 row at closure (owned by `task-map.md`) - `rg -q 'TASK-279' docs/07_implementation_status.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeProcessor.cpp` — M73 masks in the `GCodeProcessor` constructor (`"M73 P%s R%s\n"` normal, `"M73 Q%s S%s\n"` stealth), `run_post_process`'s `format_line_M73_main` disable gate, first/last-line placeholder behavior (`M73 P0 R<total>` / `M73 P100 R0`) in `process_placeholders`, dedup-on-changed-value in `process_line_move`, and `get_time_dhms`-style time formatting for `; estimated printing time (normal mode) = ...`.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `update_print_stats_and_format_filament_stats` for the exact `; filament used [mm]/[cm3]/[g]` block shape (the `; filament cost` line is deliberately NOT borrowed — the fork excludes cost).
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `disable_m73` definition (coBool, default false, comAdvanced).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
