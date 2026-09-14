---
status: draft
packet: config-scope-resolution_10_remaining-automatic-values
task_ids:
  - TASK-571
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: remaining-automatic-values

## Goal

Resolve a zero-valued configured extrusion speed at G-code emission from the active tool's maximum volumetric speed and the emitted move's width, layer height, and flow factor, while proving that no uncovered geometry-dependent `-1` config sentinel exists in the assembled registry.

## Scope Boundaries

This packet owns only Phase C, where move geometry and the active tool are available: `filament_max_volumetric_speed`, the emitter's zero-speed fallback, its invalid-geometry failure path, and a registry-derived negative-sentinel census. Packet 04 exclusively owns percent-authored `overhang_1_4_speed` through `overhang_4_4_speed` over `outer_wall_speed` and the two config-only negative mirror rules; this packet must not reinterpret or re-expand them.

## Prerequisites and Blockers

- Depends on: **FORWARD-DEP** packet 04, `config-scope-resolution_04_automatic-value-expansion` (`status: draft`), for `ExpansionContext { nozzle_diameter_mm, tool_bases }`, `expand_automatic_values`, `ExpansionError::{UnknownBaseKey, MissingAutoBase, NonPositiveBase}`, and its net-new expected test target `crates/slicer-config/tests/automatic_value_expansion_tdd.rs`; Step 1 name-reconciles that path after packet 04 lands.
- Depends on: **FORWARD-DEP** packet 05, `config-scope-resolution_05_scope-resolution-module` (`status: draft`), for the `slicer_config::resolution` module and its normative scope precedence.
- Unblocks: no queued packet identified by the approved plan.
- Activation blockers: packets 04 and 05 must land, their exports must be reconciled against code, and Step 1's derived census must confirm there is no Phase-C negative sentinel or name the newly discovered in-scope sentinel before implementation proceeds.

## Acceptance Criteria

- **AC-1. Given** an `OuterWall` extrusion with configured `outer_wall_speed = 0`, tool 0 `filament_max_volumetric_speed = 8.0` mm³/s, two points 10 mm apart with width `0.4` mm, layer height `0.2` mm, `flow_factor = 1.0`, and speed factor `1.0`, **when** `DefaultGCodeEmitter::emit_gcode` emits the second point, **then** its move feedrate is exactly `F6000` because `mm3_per_mm = 0.4 × 0.2 × 1.0 = 0.08` mm³/mm and the Phase-C automatic speed is `8.0 / 0.08 = 100` mm/s; the expected value is a literal and is not computed by the production resolver. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-gcode --all-targets --test volumetric_auto_speed_tdd zero_role_speed_uses_move_geometry_and_volumetric_limit -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test zero_role_speed_uses_move_geometry_and_volumetric_limit .* ok" target/test-output.log'`
- **AC-2. Given** two otherwise identical zero-speed paths assigned to tools 0 and 1 with resolved `filament_max_volumetric_speed` values `8.0` and `12.0` mm³/s, **when** the emitter processes both paths at width `0.4` mm, height `0.2` mm, and `flow_factor = 1.0`, **then** the emitted feedrates are the literal per-tool values `F6000` and `F9000`, while a separate `outer_wall_speed = 30.0` row remains exactly `F1800` and does not enter the auto branch. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-gcode --all-targets --test volumetric_auto_speed_tdd per_tool_volumetric_auto_and_explicit_speed_are_distinct -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test per_tool_volumetric_auto_and_explicit_speed_are_distinct .* ok" target/test-output.log'`
- **AC-3. Given** the assembled live registry after packets 04 and 05, **when** `registry_negative_sentinel_census_has_no_unowned_phase_c_candidate` derives every host/module declaration whose numeric default or lower bound is negative, **then** every derived key is classified by production metadata or an explicit tested rule, `support_interface_bottom_layers` and `support_bottom_interface_spacing` remain packet-04-owned, and the test fails loudly if any newly declared negative-capable key lacks an owner classification; no hand-maintained complete registry roster is used. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test automatic_value_expansion_tdd registry_negative_sentinel_census_has_no_unowned_phase_c_candidate -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test registry_negative_sentinel_census_has_no_unowned_phase_c_candidate .* ok" target/test-output.log'`
- **AC-4. Given** a committed model-mode request at `crates/pnp-cli/tests/fixtures/config_scope_resolution_10/visual-debug.json` whose `source.config` references `crates/pnp-cli/tests/fixtures/config_scope_resolution_10/visual-debug-config.json`, and that companion JSON config parsed by `parse_cli_config_source` contains exactly `outer_wall_speed = 0` and `filament_max_volumetric_speed = 8.0`, **when** `pnp_cli visual-debug` renders `PostPass::GCodeEmit` filament lines, **then** `manifest.json` reports `schema_version = "1.0.0"`, `source.kind = "model"`, a non-empty `images` entry with `tap = "PostPass::GCodeEmit"`, `visualization = "filament_lines"`, an existing non-empty `png_path`, array-valued warnings, and `PostPass::GCodeEmit` in `executed_stage_ids`. | `bash -lc 'set -euo pipefail; mkdir -p target; python3 -c "import shutil; shutil.rmtree(\"target/visual-debug/config-scope-resolution-10\", ignore_errors=True)"; cargo run --bin pnp_cli -- visual-debug --request crates/pnp-cli/tests/fixtures/config_scope_resolution_10/visual-debug.json --output target/visual-debug/config-scope-resolution-10 2>&1 | tee target/test-output.log >/dev/null; python3 -c "import json,pathlib; cfg=pathlib.Path(\"crates/pnp-cli/tests/fixtures/config_scope_resolution_10/visual-debug-config.json\"); assert json.loads(cfg.read_text(encoding=\"utf-8\"))=={\"outer_wall_speed\":0,\"filament_max_volumetric_speed\":8.0}; root=pathlib.Path(\"target/visual-debug/config-scope-resolution-10\"); m=json.loads((root/\"manifest.json\").read_text(encoding=\"utf-8\")); assert m[\"schema_version\"]==\"1.0.0\"; assert m[\"source\"][\"kind\"]==\"model\"; assert isinstance(m[\"warnings\"],list); assert \"PostPass::GCodeEmit\" in m[\"executed_stage_ids\"]; xs=[x for x in m[\"images\"] if x[\"tap\"]==\"PostPass::GCodeEmit\" and x[\"visualization\"]==\"filament_lines\"]; assert xs and all(isinstance(x[\"warnings\"],list) and x[\"png_path\"] and (root/x[\"png_path\"]).is_file() for x in xs)"'`
- **AC-5. Given** the new host config declaration and Phase-C placement, **when** docs and generated config references are checked, **then** `docs/02_ir_schemas.md` names `filament_max_volumetric_speed` and states that its zero-speed fallback is deferred until per-move width/flow and the layer's `height_delta` are available in `DefaultGCodeEmitter::emit_gcode`, and `docs/15_config_keys_reference.md` contains the generated `filament_max_volumetric_speed` key row. No IR schema version, WIT package, CLI JSON wire version, or manifest-schema vocabulary changes. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo xtask gen-config-docs --check 2>&1 | tee target/test-output.log >/dev/null; rg -q "filament_max_volumetric_speed" docs/02_ir_schemas.md; rg -q "DefaultGCodeEmitter::emit_gcode" docs/02_ir_schemas.md; rg -q "filament_max_volumetric_speed" docs/15_config_keys_reference.md'`

## Negative Test Cases

- **AC-N1. Given** configured role speed `0` with zero, negative, NaN, or infinite `filament_max_volumetric_speed`, width, layer height, or `flow_factor`, **when** the emitter attempts an extrusion move, **then** it returns `GCodeEmitError::Emit` naming the invalid key or geometric input and emits no zero, NaN, or infinite feedrate. | `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-gcode --all-targets --test volumetric_auto_speed_tdd invalid_volumetric_auto_inputs_fail_closed -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test invalid_volumetric_auto_inputs_fail_closed .* ok" target/test-output.log'`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-gcode --all-targets --test volumetric_auto_speed_tdd`

## Authoritative Docs

- `docs/specs/config-scope-resolution-plan.md` — direct ranged reads of RC-8, the three expansion phases, queue row 10, and cross-cutting visual/freshness gates.
- `docs/02_ir_schemas.md` — ranged reads of `ResolvedConfig`, float handling, and its equality/hash contract.
- `docs/19_visual_debug.md` — ranged reads of request, manifest, and `PostPass::GCodeEmit` rendering contracts.
- `docs/22_test_quality.md` — ranged read of oracle independence, roster, vacuity, and negative-control requirements.

## Doc Impact Statement (Required)

- `docs/02_ir_schemas.md` section `ResolvedConfig` — document the new filament maximum and Phase-C emitter placement. Verification: `rg -q 'filament_max_volumetric_speed' docs/02_ir_schemas.md && rg -q 'DefaultGCodeEmitter::emit_gcode' docs/02_ir_schemas.md`.
- `docs/config/host-keys.toml` `[resolved_config]` — add the machine-readable `filament_max_volumetric_speed` entry mirroring the Step 2 `ResolvedConfig` default so `cargo xtask gen-config-docs` can emit its row; `host_keys_doc_lock_tdd` locks the value to the live default. Verification: `rg -q 'filament_max_volumetric_speed' docs/config/host-keys.toml`.
- `docs/15_config_keys_reference.md` generated config rows — regenerate from the host declaration and `docs/config/host-keys.toml` entry. Verification: `cargo xtask gen-config-docs --check && rg -q 'filament_max_volumetric_speed' docs/15_config_keys_reference.md`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — verify `GCode::_extrude`'s `filament_max_volumetric_speed / mm3_per_mm` zero-speed fallback, tool selection, and invalid-flow guards by function name, never line number.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
