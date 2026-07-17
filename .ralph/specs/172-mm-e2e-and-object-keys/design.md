# Design: 172-mm-e2e-and-object-keys

## Controlling Code Paths

- Primary code path (TASK-212): `crates/slicer-model-io/src/loader.rs::object_metadata_to_config_data` (lines 814-856; applied per build item at line 2005); part-level parallel at lines 665-699 (`fuzzy_skin`/`extruder`/`matrix` on `ModifierVolume`); sidecar capture in `crates/slicer-model-io/src/sidecar.rs` (object-scoped metadata at lines 149-166 — captures all keys verbatim, filtering is loader-side).
- Primary code path (TASK-210): `crates/slicer-runtime/src/layer_executor.rs::assemble_ordered_entities` (signature lines 1365-1372; support emission lines 1642-1665, all four path groups hardcoded to tool `0`; call sites at lines 463 and 573, unit-test call at 2300); config read pattern at `crates/slicer-runtime/src/run.rs:619-622` (`use_relative_e_distances`); `PipelineConfig` construction at `run.rs:624-643`.
- Per-object config flow (context for TASK-212): `run.rs:345-354` seeds `object_config:<id>:<key>`; `crates/slicer-scheduler/src/config_resolution.rs::resolve_per_object_configs` (lines 403-431) and `apply_overlay` (lines 520-550) route recognized keys into `ResolvedConfig` fields and unknown keys into `ResolvedConfig.extensions` (`crates/slicer-ir/src/resolved_config.rs:442+`). No second allowlist exists downstream.
- Neighboring tests/fixtures: `crates/slicer-model-io/tests/threemf_sidecar_classification_tdd.rs` (existing allowlist assertions at lines 236-279); `crates/slicer-runtime/tests/unit/tool_ordering_tdd.rs`; `crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs`; fixtures `resources/bridge_support_enforcers.3mf`, `crates/slicer-runtime/tests/fixtures/perimeter_parity/multi_tool_triangle/multi_tool_triangle.3mf`, `resources/cube_4color.3mf`.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- Config key strings are snake_case throughout (`support_filament`, `support_interface_filament`, all allowlist keys).
- MM model is filament-index-based: one nozzle, N filaments; wipe-tower logic keys off `ToolChange.to_tool`. `SupportToolSelection` values are filament indices, never extruder/nozzle IDs — do not introduce multi-extruder assumptions.
- All change surfaces are host-side (loader, runtime, scheduler-consumed config): no WIT edit, no guest source edit, no guest WASM rebuild is triggered by this packet.
- Filament-index rebase convention is locked to the existing `extruder` handling (`loader.rs:818-833`): Orca authors 1-indexed; runtime is 0-indexed; raw `0` stays `Int(0)`.

## Code Change Surface

- Selected approach:
  - **TASK-212**: extend the existing hand-written `if let Some(s) = metadata.get(...)` chain in `object_metadata_to_config_data` with the 18 keys listed in `requirements.md` §In Scope (typed Int / Int-rebased / Float / String groups), then a final pass over `metadata` keys not in the recognized set (recognized = 3 existing + 18 new + `name` + `matrix`) emitting one `log::debug!` per dropped key. Hand-written match is a user decision — no data-driven table.
  - **TASK-210**: new `pub(crate) struct SupportToolSelection { pub support_tool: u32, pub interface_tool: u32 }` (Copy, Default = {0,0}) in `layer_executor.rs`; new parameter on `assemble_ordered_entities`; assignment: `support_paths`+`raft_paths` → `support_tool`, `interface_paths`+`ironing_paths` → `interface_tool`. Threading: parse the two keys from `config_source` in `run.rs` (same pattern as `use_relative_e_distances`), add a `support_tools: SupportToolSelection` field to `PipelineConfig`, and plumb it through the per-layer execution entry points (`execute_single_layer_inner` and `prestage_layer_collection_if_path_optimization`, both in `layer_executor.rs`) to the two production call sites. `ConfigValue::Int(n)` here is already 0-rebased when it arrived via the object-metadata path; when supplied directly in raw config (CLI/fork global config) it is 1-indexed Orca convention — parse at `run.rs` with the same `v>=1 → v-1` rebase, `0`/absent → 0.
  - **TASK-211**: new `crates/slicer-runtime/tests/e2e/mm_real_fixture_gcode_tdd.rs` with `mm_painted_fixture_t0_t1` and `mm_support_filament_real_fixture`, registered in the e2e bucket harness, following the existing full-slice API usage in `crates/slicer-runtime/tests/e2e/run_slice_api_tdd.rs`.
- Exact functions/types: `object_metadata_to_config_data` (extended), `SupportToolSelection` (new), `assemble_ordered_entities` (signature +1 param), `execute_single_layer_inner` / `prestage_layer_collection_if_path_optimization` (threading), `PipelineConfig` (+1 field), `run.rs` config parse (~10 lines), `pipeline.rs` default construction site if `PipelineConfig` is built elsewhere with struct literal syntax.
- Rejected alternatives:
  - Data-driven key table in the loader — explicitly rejected by user decision (hand-written match).
  - Reading support tools from `Blackboard` — Blackboard carries IR artifacts, not config; a typed parameter keeps determinism auditable and avoids a stringly side channel.
  - Resolving support tools per-object from `resolve_per_object_configs` — impossible today: `SupportIR` is flat with no per-object identity (comment at `layer_executor.rs:1643-1645`); recorded as an accepted deviation below.
  - Emit-side remapping in `slicer-gcode/src/emit.rs` — emit only reads `PrintEntity.tool_index` (`emit.rs:354`, `990-993`); assignment belongs where entities are born, keeping emit flavor/tool-agnostic.

## Files in Scope (read + edit)

- `crates/slicer-model-io/src/loader.rs` - role: allowlist extension; expected change: ~80 lines inside `object_metadata_to_config_data` + unknown-key logging.
- `crates/slicer-runtime/src/layer_executor.rs` - role: `SupportToolSelection` + assignment + threading; expected change: struct, +1 param on three functions, four `push(...)` tool arguments.
- `crates/slicer-runtime/src/run.rs` - role: config parse + `PipelineConfig` field; expected change: ~12 lines.
- Justified extras: `crates/slicer-runtime/src/pipeline.rs` (`PipelineConfig` struct definition + any literal construction), new test files `crates/slicer-model-io/tests/threemf_sidecar_classification_tdd.rs` (extend existing file), the in-file `#[cfg(test)] mod tests` of `layer_executor.rs` (line 2195; the `pub(crate)` symbols are unreachable from the external unit bucket, so the two new tool-selection tests run via `cargo test -p slicer-runtime --lib`), `crates/slicer-runtime/tests/e2e/mm_real_fixture_gcode_tdd.rs` (new) + e2e harness `mod` line.

## Read-Only Context

- `crates/slicer-model-io/src/loader.rs` - lines 655-700 and 805-870 only - purpose: existing conversion discipline and rebase convention.
- `crates/slicer-model-io/src/sidecar.rs` - lines 33-41, 112-166 - purpose: what metadata reaches the loader.
- `crates/slicer-runtime/src/layer_executor.rs` - lines 261-300, 430-480, 555-590, 1365-1400, 1600-1670, 2290-2320 only - purpose: threading path and support emission.
- `crates/slicer-runtime/src/run.rs` - lines 340-360 and 600-660 only - purpose: object-config seeding and `PipelineConfig` construction.
- `crates/slicer-scheduler/src/config_resolution.rs` - lines 403-431 and 520-550 only - purpose: confirm no second allowlist (verified this session).
- `crates/slicer-runtime/tests/e2e/run_slice_api_tdd.rs` - purpose: full-slice test API pattern.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies, `*.3mf` binaries - never load
- `crates/slicer-gcode/src/serialize.rs` and `emit.rs` - packets 167/171 territory / unchanged consumer; delegate symbol lookups
- `.ralph/specs/167-*`, `.ralph/specs/171-*`, `.ralph/specs/124_*` - other packets' directories; SUMMARY dispatch only
- `crates/slicer-ir/src/resolved_config.rs` - no edit needed (extensions bucket already absorbs unknown keys); bounded lookup only

## Expected Sub-Agent Dispatches

- Question: confirm `support_filament`/`support_interface_filament` semantics (1-indexed, 0 = no dedicated filament) in `PrintConfig.cpp` option definitions; scope: `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp`; return: `FACT`; purpose: Step 2 rebase rule.
- Question: confirm the 18 extended-allowlist key spellings against Orca's per-object settable set; scope: `OrcaSlicerDocumented/src/slic3r/GUI/GUI_Factories.cpp`; return: `LOCATIONS` (≤20); purpose: Step 1 before pinning tests.
- Question: which harness files declare `mod` entries for the `unit` and `e2e` test buckets; scope: `crates/slicer-runtime/tests/`; return: `FACT`; purpose: Steps 2-3 registration.
- Question: does `multi_tool_triangle.3mf` slice to ≥2 tools under the default module set, and what config does its existing parity test pass; scope: `crates/slicer-runtime/tests/` (grep `multi_tool_triangle`); return: `FACT` + ≤10 lines; purpose: Step 3 fixture config.
- Question: does `cargo check --workspace --all-targets` pass; scope: workspace; return: `FACT` + ≤20 error lines; purpose: every step's gate.

## Data and Contract Notes

- IR/manifest contracts: `PrintEntity.tool_index` stays a pure selector (packet-125 invariant: `region_key.region_id` is identity, never tool); `SupportIR` schema unchanged; `LayerCollectionIR` unchanged.
- WIT boundary: none crossed.
- Determinism/scheduler constraints: `SupportToolSelection` is derived once from `config_source` before layer execution — a pure function of config, identical across layers and runs; no scheduler-visible change.
- Per-object flow contract: loader-admitted keys become `object_config:<id>:<key>` entries; `apply_cli_key` patches declared `ResolvedConfig` fields, everything else lands in `ResolvedConfig.extensions` after `bounds.check`. The loader allowlist is the ONLY gate.
- Accepted deviation: docs/07 TASK-210 wording implies per-object support filament; flat `SupportIR` (no object identity) makes the selection global in this packet. Record in the closure notes on the docs/07 row; a future SupportIR-identity packet lifts it.

## Locked Assumptions and Invariants

- Default `SupportToolSelection {0, 0}` reproduces today's byte-identical output — no key, no behavior change (AC-N1 falsifies).
- 1-indexed→0-indexed rebase for all filament-selector keys, `0` meaning "no dedicated filament" → tool 0 (locked to the existing `extruder` convention).
- The hand-written-match shape of `object_metadata_to_config_data` is a user decision; implementers must not refactor it into a table even if shorter.
- Unknown object-metadata keys are logged, never silently dropped (AC-N2 falsifies) and never inserted untyped into the config map.

## Risks and Tradeoffs

- `multi_tool_triangle.3mf` may need explicit filament/config keys to produce two tools in the e2e harness; the Step 3 FACT dispatch on its existing parity test resolves this before test authoring. Fallback fixture: `resources/cube_4color.3mf` (4 painted colors, guaranteed multi-tool per `cube_4color_gcode_output_tdd.rs`).
- Threading a new parameter through `execute_single_layer_inner` touches a hot, heavily-tested path; the change is signature-only plus a passthrough, and `tool_ordering`/`cube_4color` suites gate regressions.
- Interface-ironing → interface tool is an interpretation (Orca irons interfaces with the interface filament); flagged [FWD] below.
- Newly admitted keys reaching `extensions` may surface as new config-hash inputs (`resolved_config.rs` hashes extensions) — determinism is preserved (same input → same hash) but per-object config identity may split where it previously merged; covered by the executor regression suite.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2 threading)
- Highest-risk dispatch and required return format: `multi_tool_triangle` fixture-config FACT — must return the exact config the existing parity test uses in ≤10 lines; reject a full test-file dump.

## Open Questions

- [FWD] Should `ironing_paths` in `SupportIR` follow the interface tool or the support tool? Packet locks interface tool (support ironing targets interface tops); implementer may flip only with a delegated Orca `SupportMaterial.cpp`/`GCode.cpp` FACT showing otherwise, updating AC-3's test in the same change.
- [FWD] `mm_support_filament_real_fixture` may show T1 only on layers that actually contain support; the AC requires "at least one `T1` line", which holds regardless of layer distribution — implementer should assert presence, not position.
- [FWD] If the `run.rs` global read and an object-level `support_filament` disagree, the global wins in this packet (flat SupportIR); log a debug note when an object-level `support_filament` key is present so the limitation is observable.
