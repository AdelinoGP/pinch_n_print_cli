---
status: implemented
packet: 172-mm-e2e-and-object-keys
task_ids:
  - TASK-210
  - TASK-211
  - TASK-212
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 172-mm-e2e-and-object-keys

## Goal

Close the open multi-material backlog slice TASK-210/211/212 (plus fork handoff item 9): route `support_filament`/`support_interface_filament` into the support tool assignment in `assemble_ordered_entities`, codify the user-verified painted-3MF behavior as a real-fixture T0/T1 G-code E2E, and extend the hand-written object-metadata allowlist match in `object_metadata_to_config_data` with the OrcaSlicer per-object keys the fork writes.

## Scope Boundaries

This packet touches the loader allowlist in `crates/slicer-model-io/src/loader.rs`, the support-tool assignment in `crates/slicer-runtime/src/layer_executor.rs` plus the config threading that feeds it, and new E2E/unit tests over existing in-repo fixtures. The MM model stays filament-index-based (wipe-tower keys off `ToolChange.to_tool`); no multi-extruder machine model is introduced, no new fixtures are authored, and no WIT/guest contracts change. Full lists live in `requirements.md`.

## Prerequisites and Blockers

- Depends on: none (no overlap with wave-1 packets 167/169: this packet does not touch `serialize.rs`, `ORCA_CONFIG_PADDING`, or the estimator).
- Unblocks: fork handoff items 4 and 9; future per-object support tuning.
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** object-level sidecar metadata containing `wall_loops=3`, `top_shell_layers=4`, `bottom_shell_layers=3`, `raft_layers=2`, `support_interface_top_layers=2`, `support_interface_bottom_layers=2`, `layer_height=0.28`, `brim_width=5.0`, `support_threshold_angle=40`, `support_top_z_distance=0.2`, `seam_position=rear`, `sparse_infill_density=20%`, `sparse_infill_pattern=gyroid`, `brim_type=outer_only`, `fuzzy_skin=external`, `support_base_pattern=rectilinear`, **when** `object_metadata_to_config_data` converts it, **then** the returned map contains all 16 keys with types Int(3), Int(4), Int(3), Int(2), Int(2), Int(2), Float(0.28), Float(5.0), Float(40.0), Float(0.2), String("rear"), String("20%"), String("gyroid"), String("outer_only"), String("external"), String("rectilinear") respectively. The strengthened fixture loads an in-memory 3MF and checks every rendered entry; percentage density remains a String.
  | `mkdir -p target && cargo test -p slicer-model-io --all-targets --test threemf_sidecar_classification_tdd -- extended_object_allowlist_types 2>&1 | tee target/test-output.log | grep "^test result: ok"`

- **AC-2. Given** object-level metadata `support_filament=2` and `support_interface_filament=3` (OrcaSlicer 1-indexed), **when** `object_metadata_to_config_data` converts it, **then** the map contains `support_filament = Int(1)` and `support_interface_filament = Int(2)` (rebased to 0-indexed, mirroring the existing `extruder` rebase in the same function). The strengthened test also asserts that raw `support_filament=0` remains `Int(0)`.
  | `mkdir -p target && cargo test -p slicer-model-io --all-targets --test threemf_sidecar_classification_tdd -- support_filament_keys_rebased 2>&1 | tee target/test-output.log | grep "^test result: ok"`

- **AC-3. Given** a `SupportToolSelection { support_tool: 1, interface_tool: 2 }` threaded into `assemble_ordered_entities` alongside a `SupportIR` with non-empty `support_paths`, `interface_paths`, `raft_paths`, and `ironing_paths`, **when** entities are assembled, **then** every entity from `support_paths` and `raft_paths` has `tool_index == 1` and every entity from `interface_paths` and `ironing_paths` has `tool_index == 2` (replacing today's hardcoded `0`). The test lives in the existing in-file `#[cfg(test)] mod tests` of `layer_executor.rs` (the symbols are `pub(crate)`; an external `--test unit` binary cannot reach them). | `mkdir -p target && cargo test -p slicer-runtime --all-targets --lib -- support_tool_selection_assigns_entities 2>&1 | tee target/test-output.log | grep "^test result: ok"`

- **AC-4. Given** a full slice of `resources/bridge_support_enforcers.3mf` with config `enable_support=true` and `support_filament=2`, **when** G-code is emitted, **then** the output contains at least one `T1` tool-change line and at least one `T0` line (support material printed on filament index 1, model on 0). | `mkdir -p target && cargo test -p slicer-runtime --all-targets --test e2e -- mm_support_filament_real_fixture 2>&1 | tee target/test-output.log | grep "^test result: ok"`

- **AC-5. Given** a full slice of the painted multi-tool fixture `crates/slicer-runtime/tests/fixtures/perimeter_parity/multi_tool_triangle/multi_tool_triangle.3mf` with its in-repo config, **when** G-code is emitted, **then** the output contains both a `T0` line and a `T1` line, and at least one tool-change transition between them (codifying the manually-verified painted-3MF → correct-color G-code behavior as a real-fixture E2E). | `mkdir -p target && cargo test -p slicer-runtime --all-targets --test e2e -- mm_painted_fixture_t0_t1 2>&1 | tee target/test-output.log | grep "^test result: ok"`

## Negative Test Cases

- **AC-N1. Given** no `support_filament`/`support_interface_filament` keys anywhere in config, **when** a supported model is sliced, **then** support entities keep `tool_index == 0` and the emitted G-code contains no support-driven tool change — all pre-existing executor and unit tests pass unchanged. Asserted by an in-file `#[cfg(test)]` test (`support_tool_selection_default_keeps_tool_zero`, `SupportToolSelection::default()` → every support/interface/raft/ironing entity has `tool_index == 0`) alongside the pre-existing external `tool_ordering` suite (see `requirements.md` matrix). | `mkdir -p target && cargo test -p slicer-runtime --all-targets --lib -- support_tool_selection_default_keeps_tool_zero 2>&1 | tee target/test-output.log | grep "^test result: ok"`

- **AC-N2. Given** object-level metadata with a non-numeric value `support_filament=abc` and an unknown key `frobnicate_mode=7`, **when** `object_metadata_to_config_data` converts it, **then** neither key appears in the returned map, a `log::warn!` names the invalid `support_filament` value, and a `log::debug!` names `frobnicate_mode` as an unrecognized dropped key (unknown keys are logged, not silently dropped). The strengthened test captures both records and asserts neither key appears in the loaded object config. | `mkdir -p target && cargo test -p slicer-model-io --all-targets --test threemf_sidecar_classification_tdd -- invalid_and_unknown_object_keys_logged 2>&1 | tee target/test-output.log | grep "^test result: ok"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `mkdir -p target && cargo test -p slicer-runtime --all-targets --test e2e -- mm_ 2>&1 | tee target/test-output.log | grep "^test result: ok"`

## Authoritative Docs

- `docs/07_implementation_status.md` - delegated; TASK-210/211/212 rows (lines 137-139) flipped at closure via `task-map.md`.
- `docs/02_ir_schemas.md` - delegated bounded lookup of `SupportIR` and `PrintEntity.tool_index` sections only.
- `docs/specs/fork-gaps-wave2-plan.md` - packet-172 section only (lines 29-33).

## Doc Impact Statement (Required)

- `docs/02_ir_schemas.md` — extend the per-object config / object-metadata subsection with the enlarged allowlist and the `support_filament`/`support_interface_filament` rebase semantics - `rg -q 'support_filament' docs/02_ir_schemas.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/slic3r/GUI/GUI_Factories.cpp` — `SettingsFactory` per-object/per-part settable option categories (source of the extended allowlist key set; confirm spellings before pinning tests).
- `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — `_BBS_3MF_Exporter::_add_model_config_file_to_archive` writes object/volume config keys unbounded (`config.keys()`), confirming the fork ships arbitrary per-object keys the loader must not silently drop.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `support_filament` / `support_interface_filament` definitions: 1-indexed filament selectors where 0 means "no dedicated filament / use current".

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
