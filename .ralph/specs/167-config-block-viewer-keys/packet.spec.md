---
status: draft
packet: 167-config-block-viewer-keys
task_ids:
  - TASK-273
backlog_source: docs/07_implementation_status.md
context_cost_estimate: S
---

# Packet Contract: 167-config-block-viewer-keys

## Goal

Purge every speed/acceleration/jerk-valued key from `ORCA_CONFIG_PADDING` in `crates/slicer-gcode/src/serialize.rs` (replacing them with neutral cosmetic keys so OrcaSlicer's ~80-key CONFIG_BLOCK minimum gate still passes), synthesize a safe non-Bambu `printer_model` when the fork's raw_config omits it, and document the fork-facing required-key contract in `docs/02_ir_schemas.md`.

## Scope Boundaries

This packet touches only the CONFIG_BLOCK serialization in `crates/slicer-gcode/src/serialize.rs`, its integration test coverage in `crates/slicer-runtime/tests/integration/gcode_header_thumbnail_config_blocks_tdd.rs`, one new normative doc subsection in `docs/02_ir_schemas.md`, and the `TASK-273` crosswalk in `docs/07_implementation_status.md`. The verbatim raw_config passthrough (`serialize_config_block` real-key dump) and the `emit_config_kv` dedup contract are unchanged. Full scope lists live in `requirements.md`.

## Prerequisites and Blockers

- Depends on: none.
- Unblocks: packet 169-time-estimator-slice-stats (real machine limits in test fixtures per the wave-1 plan queue).
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** the post-change `ORCA_CONFIG_PADDING` table, **when** grepping its entries in `crates/slicer-gcode/src/serialize.rs`, **then** no padding key ends in `_speed`, contains `speed_`, contains `acceleration`, contains `jerk`, or starts with `machine_max_` (the exact removal list is in `design.md`; `slow_down_min_speed`, `travel_speed`, `default_acceleration`, `travel_jerk` are all gone). | `awk '/^const ORCA_CONFIG_PADDING/,/^\];/' crates/slicer-gcode/src/serialize.rs | grep -E '"(machine_max_[a-z_]*|[a-z_]*speed[a-z_]*|[a-z_]*acceleration[a-z_]*|[a-z_]*jerk[a-z_]*)"' ; test $? -eq 1 && echo PASS || echo FAIL`

- **AC-2. Given** a slice with a minimal raw_config (no fork-supplied keys beyond defaults), **when** the G-code is emitted, **then** the region between `; CONFIG_BLOCK_START` and `; CONFIG_BLOCK_END` contains at least 80 `; key = value` lines, keeping OrcaSlicer's minimum-keys gate satisfied. | `mkdir -p target && cargo test -p slicer-runtime --test integration -- config_block_meets_orca_minimum_key_gate 2>&1 | tee target/test-output.log | grep "^test result"`

- **AC-3. Given** a raw_config that does not contain `printer_model`, **when** the CONFIG_BLOCK is emitted, **then** it contains exactly one `; printer_model = ` line whose value is `Generic PNP Printer` (a value containing no `Bambu` substring, so OrcaSlicer's `s_IsBBLPrinter` drag-in default is not triggered). | `mkdir -p target && cargo test -p slicer-runtime --test integration -- config_block_synthesizes_non_bbl_printer_model 2>&1 | tee target/test-output.log | grep "^test result"`

- **AC-4. Given** the fork-facing contract documentation, **when** grepping `docs/02_ir_schemas.md`, **then** a "CONFIG_BLOCK viewer-key contract" subsection exists naming `printer_model`, `filament_density`, `filament_cost`, `printable_area`, `nozzle_diameter`, and the `machine_max_*` family as fork-supplied required keys. | `grep -q "CONFIG_BLOCK viewer-key contract" docs/02_ir_schemas.md && grep -q "machine_max_" docs/02_ir_schemas.md && echo PASS || echo FAIL`

## Negative Test Cases

- **AC-N1. Given** a raw_config that supplies `machine_max_acceleration_extruding = 20000` and `printer_model = MyFork Printer`, **when** the CONFIG_BLOCK is emitted, **then** each of those keys appears exactly once with the fork-supplied value — never shadowed, duplicated, or overridden by padding or by the `printer_model` synthesis. | `mkdir -p target && cargo test -p slicer-runtime --test integration -- config_block_fork_keys_never_shadowed 2>&1 | tee target/test-output.log | grep "^test result"`

- **AC-N2. Given** the existing CONFIG_BLOCK duplicate-key invariant, **when** the pre-existing block tests run, **then** the no-duplicate-keys and block-ordering assertions still pass with the reworked padding table. | `mkdir -p target && cargo test -p slicer-runtime --test integration -- gcode_header 2>&1 | tee target/test-output.log | grep "^test result"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `mkdir -p target && cargo test -p slicer-runtime --test integration -- config_block 2>&1 | tee target/test-output.log | grep "^test result"`

## Authoritative Docs

- `docs/02_ir_schemas.md` — read only the "G-code envelope blocks (Normative — packet 55)" section (around lines 1660-1720); this packet appends the new contract subsection there. The file is 1811 lines — never load in full.
- `docs/ORCA_CONFIG_REFERENCE.md` — 2404 lines; delegate a LOCATIONS lookup only if neutral replacement key candidates need upstream-default confirmation.

## Doc Impact Statement (Required)

- `docs/02_ir_schemas.md` section "CONFIG_BLOCK viewer-key contract" (new subsection under "G-code envelope blocks") — `rg -q 'CONFIG_BLOCK viewer-key contract' docs/02_ir_schemas.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
