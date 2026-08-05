---
status: implemented
packet: 172-mm-e2e-and-object-keys
task_ids:
  - TASK-210
  - TASK-211
  - TASK-212
---

# 172-mm-e2e-and-object-keys

## Goal

Close the open multi-material backlog slice TASK-210/211/212 (plus fork handoff item 9): route `support_filament`/`support_interface_filament` into the support tool assignment in `assemble_ordered_entities`, codify the user-verified painted-3MF behavior as a real-fixture T0/T1 G-code E2E, and extend the hand-written object-metadata allowlist match in `object_metadata_to_config_data` with the OrcaSlicer per-object keys the fork writes.

## Problem Statement

Three interlocking multi-material gaps (fork handoff items 4 and 9, wave-2 plan `docs/specs/fork-gaps-wave2-plan.md`):

1. **TASK-210** — support material cannot select its own filament: every support/interface/raft/ironing path is hardcoded to `tool_index = 0` in `assemble_ordered_entities` (`crates/slicer-runtime/src/layer_executor.rs:1642-1665`), while walls/infill get real per-region tool resolution. No `support_filament`/`support_interface_filament` key exists anywhere in the Rust workspace.
2. **TASK-211** — no real-fixture MM E2E: painted-3MF → correct-color G-code was verified only manually in OrcaSlicer's viewer; existing T0/T1 assertions run on synthetic IR (`gcode_toolchange_wrapping.rs`, `tool_ordering_tdd.rs`). Real fixtures exist in-repo (`multi_tool_triangle.3mf`, `bridge_support_enforcers.3mf`, `cube_4color.3mf`).
3. **TASK-212 + item 9** — the object-metadata allowlist `object_metadata_to_config_data` (`crates/slicer-model-io/src/loader.rs:814-856`) admits exactly `extruder`, `enable_support`, `support_type`; the fork writes the full Orca per-object key set untouched (Orca's `bbs_3mf.cpp::_add_model_config_file_to_archive` serializes `config.keys()` unbounded) and every other key is silently dropped. Downstream needs no second gate: admitted keys flow as `object_config:<id>:<key>` (`crates/slicer-runtime/src/run.rs:345-354`) through `resolve_per_object_configs` → `apply_overlay` (`crates/slicer-scheduler/src/config_resolution.rs:403-431, 520-550`) into `apply_cli_key` (`crates/slicer-ir/src/resolved_config.rs:495`), with unknown-to-`ResolvedConfig` keys surviving in `extensions`.

These form one coherent slice because the E2E (TASK-211) is the acceptance vehicle for the routing (TASK-210), and the allowlist (TASK-212) is what lets fork-authored per-object MM keys reach the pipeline at all.

## Architecture Constraints

- Config key strings are snake_case throughout (`support_filament`, `support_interface_filament`, all allowlist keys).
- MM model is filament-index-based: one nozzle, N filaments; wipe-tower logic keys off `ToolChange.to_tool`. `SupportToolSelection` values are filament indices, never extruder/nozzle IDs — do not introduce multi-extruder assumptions.
- All change surfaces are host-side (loader, runtime, scheduler-consumed config): no WIT edit, no guest source edit, no guest WASM rebuild is triggered by this packet.
- Filament-index rebase convention is locked to the existing `extruder` handling (`loader.rs:818-833`): Orca authors 1-indexed; runtime is 0-indexed; raw `0` stays `Int(0)`.

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

- Resolved: `crates/slicer-runtime/tests/fixtures/perimeter_parity/multi_tool_triangle/multi_tool_triangle.3mf` passes the two-tool E2E and is the fixture used by AC-5. `resources/cube_4color.3mf` remains a separate painted-region regression fixture.
- Threading a new parameter through `execute_single_layer_inner` touches a hot, heavily-tested path; the change is signature-only plus a passthrough, and `tool_ordering`/`cube_4color` suites gate regressions.
- Interface-ironing → interface tool is an interpretation (Orca irons interfaces with the interface filament); flagged [FWD] below.
- Newly admitted keys reaching `extensions` may surface as new config-hash inputs (`resolved_config.rs` hashes extensions) — determinism is preserved (same input → same hash) but per-object config identity may split where it previously merged; covered by the executor regression suite.
