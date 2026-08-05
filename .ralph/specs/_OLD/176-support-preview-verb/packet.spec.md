---
status: implemented
packet: 176-support-preview-verb
task_ids:
  - TASK-291
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
plan_source: docs/specs/fork-gaps-wave2-plan.md (Packet 176 — fork handoff item 13)
---

# Packet Contract: 176-support-preview-verb

## Goal

Add a `pnp_cli support-preview --input <3mf> --output <path>` verb that runs only the prepass pipeline prefix via the existing `prepare_prepass_context`, reads the committed `SupportGeometryIR` off the blackboard, and writes per-layer support polygons (contour + holes, in mm) plus layer z as a versioned, fork-facing JSON contract — no per-layer module execution, no G-code.

## Scope Boundaries

One new subcommand variant in `crates/pnp-cli/src/main.rs`, one new handler module `crates/pnp-cli/src/support_preview.rs`, and one new fork-facing contract doc `docs/20_support_preview.md`. The verb reuses `slicer_model_io::load_model`, `slicer_runtime::parse_cli_config_source`, and `slicer_runtime::prepare_prepass_context` exactly as `visual_debug.rs` does — no new runtime entry point, no changes to `run.rs`/`prepass.rs`/`layer_executor.rs`, no WIT or module changes, and no rendering (the fork draws the overlay itself).

## Prerequisites and Blockers

- Depends on: nothing — `prepare_prepass_context` (run.rs:744), `Blackboard::support_geometry()` (blackboard.rs:271), and `SupportGeometryIR` (slice_ir.rs:1175) all ship today.
- Unblocks: fork paint-time support overlay (handoff item 13).
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** `resources/bridge_support_enforcers.3mf` with config enabling supports (`enable_support = true`), **when** `pnp_cli support-preview --input ... --output preview.json` runs, **then** the file parses as JSON with top-level fields `schema_version: "1.0.0"`, `units: "mm"`, `layer_count` (u32 > 0), and a `layers` array where each record has `layer_index` (u32), `z_mm` (finite f64 > 0, strictly increasing across records), and `support` — an array of expolygons each shaped `{ "contour": [[x,y],...], "holes": [[[x,y],...],...] }` with every coordinate a finite f64 in mm — and at least one record has a non-empty `support` array. | `mkdir -p target && cargo test -p pnp-cli --all-targets --test support_preview_tdd -- preview_json_schema_and_nonempty_support 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"`
- **AC-2. Given** the same run, **when** coordinates are compared against the committed IR, **then** each emitted `[x, y]` equals the corresponding `Point2` internal value × 1e-4 (1 unit = 100 nm ⇒ mm = units × 10⁻⁴) within 1e-6, and `z_mm` for `layer_index = i` equals `plan.global_layers[i].z` — proving no double or missed unit conversion. | `mkdir -p target && cargo test -p pnp-cli --all-targets --test support_preview_tdd -- coordinates_are_mm_not_internal_units 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"`
- **AC-3. Given** any successful run, **when** the process exits, **then** exit code is 0, the output path contains only the JSON document, and no `.gcode` file was produced anywhere under the output directory (the verb never reaches per-layer or postpass tiers). | `mkdir -p target && cargo test -p pnp-cli --all-targets --test support_preview_tdd -- no_gcode_side_effects_exit_zero 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"`
- **AC-4. Given** entries whose `SupportGeometryKey.global_support_layer_index` is the `u32::MAX` intermediate-layer sentinel, **when** the JSON is written, **then** those entries are excluded from `layers` and their count is reported in a top-level `skipped_intermediate_entries` (u32, present even when 0). | `mkdir -p target && cargo test -p pnp-cli --all-targets --test support_preview_tdd -- intermediate_sentinel_entries_skipped_and_counted 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"`
- **AC-5. Given** the contract doc, **when** grepping, **then** `docs/20_support_preview.md` exists, documents `schema_version`, the mm units rule, the per-layer record shape, and states that support/interface role split is NOT available at this stage (single `support` role), and `.claude/doc-index.md` lists the new doc. | `rg -q 'schema_version' docs/20_support_preview.md && rg -q 'interface' docs/20_support_preview.md && rg -q '20_support_preview' .claude/doc-index.md && echo PASS`

## Negative Test Cases

- **AC-N1. Given** the same model with `enable_support = false` (or no committed `SupportGeometryIR` at all), **when** the verb runs, **then** it exits 0 and writes valid JSON with `layers: []` and `layer_count` still reflecting the plan's layer count — never an error, never a missing file. | `mkdir -p target && cargo test -p pnp-cli --all-targets --test support_preview_tdd -- support_disabled_yields_empty_layers_exit_zero 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"`
- **AC-N2. Given** a nonexistent `--input` path, **when** the verb runs, **then** it exits nonzero with an error message naming the path on stderr and writes no output file. | `mkdir -p target && cargo test -p pnp-cli --all-targets --test support_preview_tdd -- missing_input_errors_without_output 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
  - `mkdir -p target && cargo test -p pnp-cli --all-targets --test support_preview_tdd 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"`

## Authoritative Docs

- `docs/19_visual_debug.md` — delegated SUMMARY of how the visual-debug verb wires `prepare_prepass_context` + blackboard reads (precedent this verb mirrors); over 300 lines, never read fully.
- `docs/08_coordinate_system.md` — direct ranged read of the units table only (1 unit = 100 nm; `units_to_mm`).
- `docs/20_support_preview.md` — authored by this packet (fork-facing contract).

## Doc Impact Statement (Required)

- `docs/20_support_preview.md` — new fork-facing contract doc: schema_version 1.0.0, mm units, per-layer record shape, sentinel-skip rule, no-interface-split statement, latency contract (prepass-only) - `rg -q 'schema_version' docs/20_support_preview.md && rg -q '1 unit = 100 nm|mm' docs/20_support_preview.md`
- `.claude/doc-index.md` — add the one-line index row for `docs/20_support_preview.md` - `rg -q '20_support_preview' .claude/doc-index.md`
- `docs/07_implementation_status.md` — canonical TASK-291 support-preview row - `rg -q 'TASK-291.*support-preview' docs/07_implementation_status.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
