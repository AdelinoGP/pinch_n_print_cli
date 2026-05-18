---
status: implemented
packet: 35a_resolved-config-propagation
task_ids:
  - TASK-166
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 35a_resolved-config-propagation

## Goal

Resolve user-supplied CLI config (`--config <json>`) into a per-object `slicer_ir::ResolvedConfig` and stamp it into every `RegionPlan` during the host built-in `PrePass::RegionMapping`, so `top_shell_layers`, `bottom_shell_layers`, and every other declared `ResolvedConfig` field reach their consumers (e.g. `crates/slicer-host/src/layer_slice.rs::execute_layer_slice`) end-to-end through the `slicer-host` binary instead of being silently overwritten by `ResolvedConfig::default()`.

## Scope Boundaries

- In scope:
  - New host-side resolver module `crates/slicer-host/src/config_resolution.rs` that turns the CLI `HashMap<ConfigKey, ConfigValue>` into a base `ResolvedConfig` plus per-object overlays.
  - Strict-on-typed-fields, permissive-on-unknown-keys resolution: known declared fields enforce their `ConfigValue` variant; unrecognized keys land in `ResolvedConfig.extensions`.
  - New flat CLI key convention `object_config:<object_id>:<config_key>` for per-object overrides (mirrors the existing `object_height:<id>` host-seeding pattern).
  - Extend `region_mapping::commit_region_mapping_builtin` to accept the resolved per-object map and stamp every `RegionPlan.config` with the resolved value for that entry's `object_id`.
  - Plumb the resolved-configs map from `slicer-host`'s `main.rs` through `pipeline::PipelineConfig` and the prepass-execution path so the built-in receives it on the live binary path.
  - Tests covering: resolver unit behavior (defaults, typed-field happy path, unknown-key passthrough, type-mismatch rejection), per-object overlay precedence, RegionMapping stamping, CLI E2E propagation through the binary, and a CLI startup rejection on a malformed declared field.
- Out of scope:
  - Per-region (paint-keyed) overlays — requires WIT additions on `region-id`-scoped sources; future packet.
  - Print profile inheritance, vendor profile loading, or any non-flat JSON config shape.
  - Module-internal `ConfigView` filtering — already correct via `bind_module_config_view`; this packet does not change per-module declared-read enforcement.
  - Changes to `harvest_layer_plan_ir` — `ActiveRegion.resolved_config` is left as a module-emitted advisory; the host stamps the authoritative value in `RegionPlan.config` at RegionMapping time.

## Prerequisites and Blockers

- Depends on: nothing in flight; the surfaces this packet touches all exist (parsed CLI source, RegionMapping built-in, RegionPlan struct, ResolvedConfig fields).
- Unblocks: packet `36_bridge-detector-orca-parity` (needs user-tunable bridge-detector config), packet `37_fill-role-claims` (needs per-claim config selection), and any future packet that exposes user-tunable fields on `RegionPlan.config`.
- Activation blockers: none after scope is agreed; per-object overlay key convention is locked at `object_config:<object_id>:<config_key>`.

## Acceptance Criteria

- **Given** a `HashMap<ConfigKey, ConfigValue>` containing `top_shell_layers = Int(4)` and no other keys, **when** the new `resolve_global_config` helper runs, **then** the returned `ResolvedConfig.top_shell_layers == 4`, `bottom_shell_layers == 3` (default), and `extensions.is_empty() == true`. | `cargo test -p slicer-host --test config_resolution_tdd resolver_maps_top_shell_layers -- --exact --nocapture`
- **Given** a config source containing one declared key `top_shell_layers = Int(2)` and one unknown key `experimental_xyz = String("on")`, **when** `resolve_global_config` runs, **then** `ResolvedConfig.top_shell_layers == 2` and `ResolvedConfig.extensions.get("experimental_xyz") == Some(&ConfigValue::String("on".into()))`. | `cargo test -p slicer-host --test config_resolution_tdd resolver_unknown_key_routes_to_extensions -- --exact --nocapture`
- **Given** a config source containing `top_shell_layers = Int(3)` and `object_config:obj-A:top_shell_layers = Int(5)`, **when** `resolve_per_object_configs` runs against an object set `["obj-A", "obj-B"]`, **then** the returned `BTreeMap<String, ResolvedConfig>` maps `"obj-A".top_shell_layers == 5` and `"obj-B".top_shell_layers == 3`. | `cargo test -p slicer-host --test config_resolution_tdd resolver_per_object_overrides_global -- --exact --nocapture`
- **Given** a `LayerPlanIR` with two active regions on objects `obj-A` and `obj-B`, an `ExecutionPlan`, and a per-object resolved-config map where `obj-A.top_shell_layers == 5` and `obj-B.top_shell_layers == 3`, **when** `commit_region_mapping_builtin` runs, **then** the committed `RegionMapIR.entries` contains exactly two entries; the entry whose `object_id == "obj-A"` has `config.top_shell_layers == 5`, the entry whose `object_id == "obj-B"` has `config.top_shell_layers == 3`. | `cargo test -p slicer-host --test region_mapping_resolved_config_tdd commit_stamps_per_object_resolved_config -- --exact --nocapture`
- **Given** the Benchy STL fixture and the live core-modules directory, **when** `slicer-host run` is invoked once with `--config '{"top_shell_layers": 1, "bottom_shell_layers": 1}'` and once with `--config '{"top_shell_layers": 4, "bottom_shell_layers": 4}'`, **then** the second run's `;TYPE:Top surface` block count is strictly greater than the first run's, AND the second run's `;TYPE:Bottom surface` block count is strictly greater than the first run's. | `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_user_top_shell_layers_propagates_through_binary -- --exact --nocapture`

## Negative Test Cases

- **Given** a config source containing `top_shell_layers = String("four")`, **when** `resolve_global_config` runs, **then** it returns `Err(ConfigResolutionError::TypeMismatch { key, expected, actual })` where `key == "top_shell_layers"` and `expected == "Int"`. | `cargo test -p slicer-host --test config_resolution_tdd resolver_rejects_string_for_top_shell_layers -- --exact --nocapture`
- **Given** the Benchy STL fixture and a JSON config file containing `{"top_shell_layers": "four"}`, **when** `slicer-host run --config <path>` is invoked, **then** the process exits with a non-zero status, no `--output` file is written, and stderr contains both the literal `top_shell_layers` and the literal `expected Int` (substring match). | `cargo test -p slicer-host --test benchy_end_to_end_tdd cli_rejects_top_shell_layers_string -- --exact --nocapture`

## Verification

- `cargo build --workspace` — packet must build cleanly (delegate; FACT pass/fail).
- `cargo clippy --workspace -- -D warnings` — packet must pass clippy gate (delegate; FACT pass/fail).
- `cargo test -p slicer-host --tests` — full slicer-host test suite must remain green (delegate; FACT pass/fail with failing-test list on red).

## Authoritative Docs

- `docs/02_ir_schemas.md` — `ResolvedConfig` field list (lines `~575-660` per packet 35 design); `RegionMapIR` and `RegionPlan` shape. Read directly (≤ 200 lines of relevant range).
- `docs/03_wit_and_manifest.md` — `[config.schema]` declaration rules (host-boundary enforcement), confirms `ResolvedConfig` field names are the canonical key surface. Delegate a SUMMARY for the relevant section only.
- `docs/04_host_scheduler.md` — `PrePass::RegionMapping` (built-in) section and the "RegionMapIR Compilation" subsection. Delegate a SUMMARY (file is large).
- `docs/DEVIATION_LOG.md` — DEV-040 entry; this packet's close-out is conditional on flipping DEV-040 to `Closed`. Delegate a single FACT read of the row.

## OrcaSlicer Reference Obligations

- None. The codebase already deliberately deviates on default values for `top_shell_layers` (`3` here vs Orca's `4`); this packet is plumbing-only and does not change defaults. Implementer MUST NOT load anything under `OrcaSlicerDocumented/` for this packet.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md` — included because this packet covers exactly one task ID but explicitly closes a deviation (`DEV-040`) surfaced by an earlier packet (35), and traceability matters.

## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list;
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly;
- delegate every `cargo` run and authoritative-doc fact-check;
- stop reading at 60% context and hand off at 85%.

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
