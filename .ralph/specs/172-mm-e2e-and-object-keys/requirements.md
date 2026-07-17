# Requirements: 172-mm-e2e-and-object-keys

## Packet Metadata

- Grouped task IDs: `TASK-210`, `TASK-211`, `TASK-212`
- Backlog source: `docs/07_implementation_status.md` (rows at lines 137-139, all open)
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Three interlocking multi-material gaps (fork handoff items 4 and 9, wave-2 plan `docs/specs/fork-gaps-wave2-plan.md`):

1. **TASK-210** — support material cannot select its own filament: every support/interface/raft/ironing path is hardcoded to `tool_index = 0` in `assemble_ordered_entities` (`crates/slicer-runtime/src/layer_executor.rs:1642-1665`), while walls/infill get real per-region tool resolution. No `support_filament`/`support_interface_filament` key exists anywhere in the Rust workspace.
2. **TASK-211** — no real-fixture MM E2E: painted-3MF → correct-color G-code was verified only manually in OrcaSlicer's viewer; existing T0/T1 assertions run on synthetic IR (`gcode_toolchange_wrapping.rs`, `tool_ordering_tdd.rs`). Real fixtures exist in-repo (`multi_tool_triangle.3mf`, `bridge_support_enforcers.3mf`, `cube_4color.3mf`).
3. **TASK-212 + item 9** — the object-metadata allowlist `object_metadata_to_config_data` (`crates/slicer-model-io/src/loader.rs:814-856`) admits exactly `extruder`, `enable_support`, `support_type`; the fork writes the full Orca per-object key set untouched (Orca's `bbs_3mf.cpp::_add_model_config_file_to_archive` serializes `config.keys()` unbounded) and every other key is silently dropped. Downstream needs no second gate: admitted keys flow as `object_config:<id>:<key>` (`crates/slicer-runtime/src/run.rs:345-354`) through `resolve_per_object_configs` → `apply_overlay` (`crates/slicer-scheduler/src/config_resolution.rs:403-431, 520-550`) into `apply_cli_key` (`crates/slicer-ir/src/resolved_config.rs:495`), with unknown-to-`ResolvedConfig` keys surviving in `extensions`.

These form one coherent slice because the E2E (TASK-211) is the acceptance vehicle for the routing (TASK-210), and the allowlist (TASK-212) is what lets fork-authored per-object MM keys reach the pipeline at all.

## In Scope

- **TASK-212**: extend the hand-written match in `loader.rs::object_metadata_to_config_data` (user decision: hand-written match, NOT a data-driven table) with exactly these keys:
  - Int: `wall_loops`, `top_shell_layers`, `bottom_shell_layers`, `raft_layers`, `support_interface_top_layers`, `support_interface_bottom_layers` (warn-and-skip on parse failure, mirroring `extruder`).
  - Int with 1-indexed→0-indexed rebase (mirroring `extruder`): `support_filament`, `support_interface_filament` (raw `0` = "no dedicated filament" stays `Int(0)`).
  - Float: `layer_height`, `brim_width`, `support_threshold_angle`, `support_top_z_distance` (warn-and-skip on parse failure).
  - String passthrough: `seam_position`, `sparse_infill_density`, `sparse_infill_pattern`, `brim_type`, `fuzzy_skin`, `support_base_pattern`.
  - Unrecognized keys: emit one log line naming each dropped key (excluding the known-benign `name` and `matrix` sidecar keys) — logged, never silently dropped.
- **TASK-210**: a `SupportToolSelection` (support tool + interface tool, 0-indexed, both defaulting to 0) parsed from `config_source` keys `support_filament`/`support_interface_filament` at the `PipelineConfig` construction site (`run.rs`, next to the `use_relative_e_distances` read at `run.rs:619-622`), threaded through the per-layer execution path into `assemble_ordered_entities` (signature at `layer_executor.rs:1365-1372`; call sites at `layer_executor.rs:463`, `:573`, test `:2300`): `support_paths` + `raft_paths` → support tool; `interface_paths` + `ironing_paths` → interface tool.
- **TASK-211**: new real-fixture E2E tests in the `e2e` bucket: `mm_painted_fixture_t0_t1` over `crates/slicer-runtime/tests/fixtures/perimeter_parity/multi_tool_triangle/multi_tool_triangle.3mf` (T0 and T1 both emitted) and `mm_support_filament_real_fixture` over `resources/bridge_support_enforcers.3mf` with `enable_support=true`, `support_filament=2` (support prints on T1).
- Unit tests: extended-allowlist typing + rebase + unknown-key logging (slicer-model-io), `SupportToolSelection` entity assignment + default-zero (in-file `#[cfg(test)]` tests in `layer_executor.rs`, run via `cargo test -p slicer-runtime --lib` — the `pub(crate)` symbols are unreachable from the external unit bucket).
- `docs/02_ir_schemas.md` per-object allowlist + rebase-semantics doc update.

## Out of Scope

- **Per-object support-filament granularity**: `SupportIR` is flat (no per-object identity — see the comment at `layer_executor.rs:1643-1645`), so support tool selection is resolved from the global/default config, not per object. Lifting `SupportIR` to per-object identity is a future packet; the docs/07 row wording "per-object support_filament" is satisfied at the granularity the IR supports today and the deviation is recorded in `design.md`.
- Wipe-tower behavior changes (MM model stays keyed off `ToolChange.to_tool`; no multi-extruder machine model, no new purge logic).
- Any `serialize.rs` / CONFIG_BLOCK / padding change (packets 167 and 171 own that file).
- New fixtures or re-exports from OrcaSlicer (only existing in-repo fixtures are used).
- Making the newly admitted per-object keys *behaviorally honored* by modules that ignore them today (e.g. per-object `layer_height` scheduling); this packet guarantees they reach the per-object `ResolvedConfig`/`extensions`, not that every module consumes them.
- Part/volume-level (`ModifierVolume`) allowlist extension beyond what exists at `loader.rs:665-699`.

## Authoritative Docs

- `docs/07_implementation_status.md` - always delegated; rows 137-139 flipped at closure.
- `docs/02_ir_schemas.md` - large; delegate bounded lookups of `SupportIR`, `PrintEntity`, and per-object config sections.
- `docs/04_host_scheduler.md` - delegate a SUMMARY of per-object config resolution if `apply_overlay` semantics need re-verification.
- `docs/specs/fork-gaps-wave2-plan.md` - packet-172 section only (lines 29-33).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/slic3r/GUI/GUI_Factories.cpp` — `SettingsFactory` per-object/per-part settable option categories (source of the extended allowlist key set; confirm spellings before pinning tests).
- `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — `_BBS_3MF_Exporter::_add_model_config_file_to_archive` writes object/volume config keys unbounded (`config.keys()`), confirming the fork ships arbitrary per-object keys the loader must not silently drop.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `support_filament` / `support_interface_filament` definitions: 1-indexed filament selectors where 0 means "no dedicated filament / use current".

## Acceptance Summary

- Positive: `AC-1` through `AC-5` in `packet.spec.md`. Refinement: AC-4/AC-5 must assert on the emitted G-code text (post-serializer), not on IR, so the whole ToolChange → `T<n>` chain is covered.
- Negative: `AC-N1`, `AC-N2`.
- Cross-packet impact: none on packets 167/171 (no `serialize.rs` edits here). A future `SupportPlanIR.raft_plan` packet (124) may re-route raft tool selection; this packet's raft→support-tool rule is the interim behavior.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `mkdir -p target && cargo test -p slicer-model-io --test threemf_sidecar_classification_tdd 2>&1 \| tee target/test-output.log \| grep "^test result"` | Extended allowlist typing, rebase, unknown-key logging, plus pre-existing sidecar assertions | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `mkdir -p target && cargo test -p slicer-runtime --lib -- support_tool_selection 2>&1 \| tee target/test-output.log \| grep "^test result"` | In-file `assemble_ordered_entities` tool assignment + default-zero tests (AC-3, AC-N1) | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test unit -- tool_ordering 2>&1 \| tee target/test-output.log \| grep "^test result"` | Pre-existing external tool-ordering suite unchanged (AC-N1 regression) | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test e2e -- mm_ 2>&1 \| tee target/test-output.log \| grep "^test result"` | Both real-fixture MM E2E tests | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test executor -- cube_4color 2>&1 \| tee target/test-output.log \| grep "^test result"` | Painted-region tool attribution unaffected by the threading change | FACT pass/fail |
| `cargo check --workspace --all-targets` | Whole-workspace type gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate | FACT pass/fail |

## Step Completion Expectations

The loader allowlist (Step 1) and the tool routing (Step 2) are independent; the E2E step (Step 3) requires Step 2 (support-filament fixture) and benefits from Step 1 (fixture metadata keys admitted). The `SupportToolSelection` struct name and field shape fixed in Step 2 is consumed verbatim by Step 3's fixtures — do not rename between steps.

## Context Discipline Notes

- `crates/slicer-runtime/src/layer_executor.rs` is >2300 lines — read only the ranges named in `design.md` (261-300, 430-480, 555-590, 1365-1400, 1600-1670, 2290-2320).
- `crates/slicer-model-io/src/loader.rs` is 3031 lines — read only 655-700 and 805-870.
- `crates/slicer-runtime/src/run.rs` — read only 340-360 and 600-660.
- Fixture 3MFs are binary — never open them; slice them via tests only.
