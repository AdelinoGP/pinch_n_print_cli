# Requirements: automatic-value-expansion

## Packet Metadata

- Grouped task IDs: `TASK-565`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`
- Approved-plan queue row: 4

## Problem Statement

Automatic config values currently escape resolution in incompatible forms. `resolve_role_width` implements the width precedence and nozzle fallback, `resolve_support_line_width_mm` separately implements support width, percent-aware module readers choose bases at consumption, and `feedrate.rs::read_speed` drops percent-authored overhang speeds because it has no expansion base. The two config-only negative sentinels still mirror their top-side settings inside support guests. This duplicates policy and allows raw placeholders whose bases are fully config-known to reach a `ConfigView` even though all required Phase-B inputs are already known after scope merge. Canonical declares the overhang-speed percent family over `outer_wall_speed`; only numeric zero retains the existing role-context no-override behavior.

## In Scope

- Add `slicer_config::ExpansionContext`, carrying a positive global `nozzle_diameter_mm` and deterministic per-tool absolute base maps keyed by tool index and snake_case config key.
- Add `slicer_config::expand_automatic_values`, an atomic (no partial commit on error) expansion operation over one already-merged `ResolvedConfig`, parameterized by `ConfigSchemaRegistry`, `ExpansionContext`, and optional tool index.
- Resolve `ConfigValue::Percent` and percent-marked `ConfigValue::FloatOrPercent` only when the corresponding `RegistryEntry.base_key` names the absolute base. The selected tool's matching absolute base wins; otherwise the merged config's base value is used; `nozzle_diameter` may come from the explicit context. Successful output is an absolute numeric value, never another percent placeholder. This includes all four `overhang_1_4_speed` through `overhang_4_4_speed` percent forms over the scope-resolved `outer_wall_speed`; a plain numeric `0.0` is preserved as the existing no-override value.
- Add the missing `support_line_width -> nozzle_diameter` typed base to its existing declaration in `modules/core-modules/tree-support-planner/tree-support-planner.toml`. Retype the four declarations in `modules/core-modules/overhang-classifier-default/overhang-classifier-default.toml` to `float_or_percent` with `base_key = "outer_wall_speed"`, and give their live host declarations matching `wire_type` through the four `SPEED_META` entries in `crates/slicer-ir/src/feedrate.rs`, so registry reconciliation succeeds. Preserve packet 1's `bridge_line_width` and `initial_layer_line_width -> nozzle_diameter` metadata unchanged.
- Expand the two width-zero rules centrally: `line_width = 0` becomes `1.125 * nozzle_diameter`; `support_line_width = 0` becomes `nozzle_diameter`. Explicit positive millimetre values pass through. Zero/non-finite required nozzle bases reject rather than producing zero, NaN, or Infinity.
- Expand exactly two config-only negative sentinel rules: `support_interface_bottom_layers = -1` copies `support_interface_top_layers`, and `support_bottom_interface_spacing = -1` copies `support_interface_spacing`. Explicit zero values remain zero. A missing matching top key rejects rather than inventing a default.
- Invoke expansion in both `run_slice_with_collector` and `prepare_prepass_context` after global/object resolution and before `build_live_execution_plan`/`bind_module_config_view` or `FeedrateConfig::from_raw_config`. Carry the same registry/context authority through `crates/slicer-runtime/src/pipeline.rs` on the full-slice path and directly from `prepare_prepass_context` on its direct configured-prepass path. The configured prepass family in `crates/slicer-runtime/src/prepass.rs` currently accepts raw/default/object inputs and internally rebuilds per-tool and per-paint-semantic maps, so each rebuilt map must be expanded immediately after its scope resolver returns and before `commit_region_mapping_builtin` can reach any `RegionMapIR::intern_config`. The separately resolved run-level tool map used by the emitter must also be expanded before use. Build the registry from packet 1's `ModuleDeclaration`, `HostChannels`, `assemble_registry`, and `AssemblyOutcome` surfaces; do not create a second schema census or registry representation.
- Reduce `resolve_role_width` to bridge override, first-layer override, role dispatch, then the already-expanded `line_width`; remove its `1.125 * nozzle_diameter` fallback. Preserve its existing precedence order.
- Remove `resolve_support_line_width_mm` and use the centrally expanded `support_line_width` value at its host consumers. Remove the four support guest mirror implementations so production behavior has one sentinel owner; update direct guest fixtures to supply expanded values.
- Add independently authored expected-value tests for exact width and four-member overhang-speed percentages, zero and `-1` cases, tool-base selection, error paths, runtime ordering, registry type/base reconciliation, and the role-width fallback reduction. Runtime coverage must author covered placeholders at global, object, tool, and paint-semantic scope, exercise a material-painted tool region plus a non-support modifier child so all three current `execute_region_mapping_inner` interning branches are represented, inspect both bound module config and every emitted `RegionMapIR.configs` value, and fail if the `prepass.rs`-local tool/paint expansion is removed. The speed fixture uses `outer_wall_speed = 60.0` and literal expectations `25% -> 15.0`, `50% -> 30.0`, `75% -> 45.0`, and `100% -> 60.0`; expected values must never come from a second invocation of the resolver under test.
- Commit a deterministic visual-debug request/config fixture using `resources/regression_wedge.stl`, `Layer::Perimeters`, layer 0, and `filament_lines`, then check the manifest fields required by `docs/19_visual_debug.md`.
- Extend `xtask/src/gen_config_docs.rs::KeyRow`, module-row parsing, and `render_table` so generated tables carry a `Base key` column (`—` when absent), add the inline `render_table_preserves_base_key` regression, update `docs/02_ir_schemas.md`, and regenerate `docs/15_config_keys_reference.md` for the new resolution placement and all support/overhang typed bases.
- Compatibility checklist: IR schema versions touched — none; WIT package bumps — none; CLI output wire versions — none; manifest schema vocabulary/version — unchanged (existing `type`/`base_key` declaration values change); public wire/IR structs — unchanged. No `CURRENT_*_SCHEMA_VERSION` or WIT package declaration is bumped, and the live `CONFIG_SCHEMA_WIRE_VERSION` remains exactly `1.3.0`. The current canonical WIT inventory is the exact set asserted by AC-7, including `slicer:layer-slice-postprocess@1.0.0`; this packet neither predicts nor reserves a future package version.

## Out of Scope

- Queue row 5's replacement of the five scope resolvers, scope-delta precedence implementation, Z-grid query, scope-stack API, and layer-planning WIT record.
- Queue row 6's registry-driven complete `to_config_map`, universal default delivery, and removal of all guest `.unwrap_or(literal)` fallbacks.
- Queue row 10's emitter-owned volumetric speed `0 = filament_max_volumetric_speed / mm3_per_mm` and every geometry-, layer-, flow-, or move-dependent `-1 = auto` rule. It does not own the overhang-speed percent family, which the approved plan assigns to this packet's Phase B.
- Changing bridge/first-layer/role precedence, inventing a flow model, or changing support-width's accepted DEV-154 representation.
- Public IR fields, WIT records/functions/packages, CLI JSON shapes, manifest field names, schema/version constants, or mm↔internal-unit conversion behavior.
- General cleanup of unrelated percent-aware call sites whose registry entry has no typed `base_key`.

## Authoritative Docs

- `docs/specs/config-scope-resolution-plan.md` — direct full read; approved Phase-B speed-percent ownership in row 4 and row 10's narrower volumetric/context-dependent remainder.
- `docs/02_ir_schemas.md` — ranged reads around `ResolvedConfig`, `RegionMapIR::intern_config`, and `IR Versioning Contract`.
- `docs/11_operational_governance_and_acceptance_gate.md` — ranged read of compatibility dimensions and wire policy.
- `docs/19_visual_debug.md` — ranged reads around request shape, manifest reading, and execution closure.
- `docs/adr/0067-unified-config-schema-registry.md` — accepted single-registry and typed-base decision.
- `docs/adr/0068-config-scope-is-a-wire-encoding.md` — accepted amendment placing config-only expansion in Phase B.
- `docs/22_test_quality.md` — delegated/ranged check of independent oracle and false-green rules before test authoring.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Flow.cpp` — verify `Flow::new_from_config_width` / `Flow::auto_extrusion_width` zero and role behavior; cite by function, never source line.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — verify `PrintConfigDef::init_fff_params` declares all four overhang-speed keys as float-or-percent with `ratio_over = "outer_wall_speed"`.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — verify `GCode::_extrude` distinguishes numeric-zero role fallback from the emitter volumetric auto rule left to packet 10.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportParameters.hpp` — verify `number_of_support_interface_bottom_layers` mirrors the top count for a negative bottom value.

## Acceptance Summary

- Positive: `AC-1` through `AC-7` in `packet.spec.md` cover hand-computed Phase-B width/speed values, tool bases, real registry metadata, role dispatch, runtime placement, visual evidence, generated docs, and compatibility.
- Negative: `AC-N1` through `AC-N3` cover unavailable typed bases without partial mutation, missing matching auto bases, and zero/non-finite nozzle safety.
- Cross-packet impact: packet 5 consumes the expansion API before its new resolver interns configs; packet 10 consumes the same `ExpansionContext` but owns only emitter volumetric/context-dependent Phase C, never the four overhang percentages. Packet 1 remains the sole producer of the registry types listed as FORWARD-DEPs.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only closure gates.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test --all-targets -p slicer-config --test automatic_value_expansion_tdd -- --nocapture 2>&1 \| tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` | All independent expansion success/error cases, including atomic failure behavior. | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test --all-targets -p slicer-config --test registry_assembly_tdd phase_b_speed_percent_family_declares_typed_outer_wall_base -- --nocapture 2>&1 \| tee target/test-output.log >/dev/null; rg -q "test phase_b_speed_percent_family_declares_typed_outer_wall_base \.\.\. ok" target/test-output.log'` | All four overhang speed declarations reconcile as `float_or_percent` with registry `base_key = "outer_wall_speed"` and default zero. | FACT pass/fail |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test --all-targets -p slicer-core --test flow_tdd role_width -- --nocapture 2>&1 \| tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` | Existing precedence plus reduced role fallback. | FACT pass/fail |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test --all-targets -p slicer-runtime --test integration automatic_value_expansion_tdd::runtime_expands_global_object_tool_and_paint_before_delivery -- --nocapture 2>&1 \| tee target/test-output.log >/dev/null; rg -q "test automatic_value_expansion_tdd::runtime_expands_global_object_tool_and_paint_before_delivery \.\.\. ok" target/test-output.log'` | Runtime ordering in both production run paths, including `prepass.rs`-local tool/paint reconstruction and all resulting region-map configs. | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo xtask build-guests --check 2>&1 \| tee target/test-output.log >/dev/null'` | Guest artifacts are fresh after core/IR/support guest edits; verdict is the command exit code. | FACT exit 0/nonzero |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test --all-targets -p xtask render_table_preserves_base_key 2>&1 \| tee target/test-output.log >/dev/null; rg -q "render_table_preserves_base_key .* ok" target/test-output.log; cargo xtask gen-config-docs --check 2>&1 \| tee target/test-output.log >/dev/null; python3 -c "from pathlib import Path; s=Path(\"docs/15_config_keys_reference.md\").read_text(encoding=\"utf-8\"); assert \"| Base key |\" in s; assert \"support_line_width\" in s and \"nozzle_diameter\" in s; assert all(k in s and any(k in line and \"outer_wall_speed\" in line for line in s.splitlines()) for k in (\"overhang_1_4_speed\",\"overhang_2_4_speed\",\"overhang_3_4_speed\",\"overhang_4_4_speed\"))"'` | `KeyRow`/`module_rows`/`render_table` preserve manifest `base_key`, and generated config rows expose the support and four overhang bases. | FACT pass/fail |
| `cargo check --workspace --all-targets` | All target kinds compile. | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Required lint gate. | FACT pass/fail |
| `cargo xtask check-literals` | Required watched-struct literal gate. | FACT pass/fail |
| `cargo xtask check-test-quality --report` | Required touched-test quality report; fix or justify findings on touched tests. | FACT pass/fail plus touched-file findings only |
| `bash -lc 'set -euo pipefail; mkdir -p target; python3 -c "import shutil; shutil.rmtree(\"target/visual-debug/config-scope-resolution-04\", ignore_errors=True)"; cargo run --bin pnp_cli -- visual-debug --request crates/pnp-cli/tests/fixtures/automatic_value_expansion/visual-debug.json --output target/visual-debug/config-scope-resolution-04 2>&1 \| tee target/test-output.log >/dev/null; python3 -c "import json,pathlib; p=pathlib.Path(\"target/visual-debug/config-scope-resolution-04\"); m=json.loads((p/\"manifest.json\").read_text(encoding=\"utf-8\")); assert m[\"images\"] and m[\"executed_stage_ids\"]"'` | Required geometry-affecting visual-debug gate with explicit UTF-8 manifest decoding on Windows. | FACT pass/fail; manifest fields on failure |

## Step Completion Expectations

- Expansion must stage all replacements and commit them only after every dependency and sentinel rule succeeds; error tests must prove the original config remains bit-equal.
- Registry declaration metadata—including all four overhang `float_or_percent`/`outer_wall_speed` entries—lands before runtime wiring, and runtime wiring lands before duplicate consumer fallbacks are removed.
- Both production entry points remain behaviorally aligned: no `run_slice_with_collector`-only or `prepare_prepass_context`-only expansion, and no tool/paint placeholder may be reintroduced by configured prepass reconstruction.
- Every guest-source edit is followed by guest rebuild if stale and an exit-0 freshness check before interpreting module-dispatch results.

## Context Discipline Notes

- `crates/slicer-runtime/src/run.rs`, `crates/slicer-ir/src/resolved_config.rs`, the support guest sources, `docs/02_ir_schemas.md`, `docs/19_visual_debug.md`, and `docs/22_test_quality.md` are long; use named-symbol/ranged reads only.
- Delegate the percent/base-key manifest census and every OrcaSlicer check; do not browse all manifests or upstream source in the implementer's context.
- Never load `target/test-output.log` whole; return the failing test name, assertion, and at most 20 surrounding lines.
