---
status: implemented
packet: 35a_resolved-config-propagation
task_ids:
  - TASK-166
---

# 35a_resolved-config-propagation

## Goal

Resolve user-supplied CLI config (`--config <json>`) into a per-object `slicer_ir::ResolvedConfig` and stamp it into every `RegionPlan` during the host built-in `PrePass::RegionMapping`, so `top_shell_layers`, `bottom_shell_layers`, and every other declared `ResolvedConfig` field reach their consumers (e.g. `crates/slicer-host/src/layer_slice.rs::execute_layer_slice`) end-to-end through the `slicer-host` binary instead of being silently overwritten by `ResolvedConfig::default()`.

## Problem Statement

`crates/slicer-host/src/dispatch.rs::harvest_layer_plan_ir` (line ~1651) hardcodes `resolved_config: ResolvedConfig::default()` on every `ActiveRegion` it builds from a `LayerProposal`. The host built-in `PrePass::RegionMapping` (`region_mapping.rs:190`) then clones `region.resolved_config` into `RegionPlan.config`, so every consumer of `RegionMapIR.entries[*].config` sees codebase defaults regardless of what the user supplied via `--config`.

The CLI parses `--config` correctly into a `HashMap<ConfigKey, ConfigValue>` at `main.rs:135-150` and feeds it to `bind_module_config_view` for per-module `ConfigView` filtering — that works. The missing surface is the host-side resolver that converts the same raw `HashMap` into `slicer_ir::ResolvedConfig` and stamps it on the per-region IR that downstream stages read. Without that, `top_shell_layers`, `bottom_shell_layers`, and every other declared `ResolvedConfig` field are unreachable from user input.

This was first surfaced by packet `35_multi-layer-top-bottom-thickness`, whose Part 2 binary E2E test (`benchy_multi_layer_top_bottom_evidence`) had to be strengthened to bypass the binary path precisely because config propagation is broken end-to-end (DEV-040). Packets `36_bridge-detector-orca-parity` and `37_fill-role-claims` are blocked on this fix because their behavior is config-tunable.

This packet does NOT modify packet 35 or its files; packet 35 closed correctly within its CONSUMER-side scope and remains `implemented`. This packet adds the missing PRODUCER-side plumbing.

## Architecture Constraints

- The `ResolvedConfig` shape is fixed by `crates/slicer-ir/src/slice_ir.rs:575-660`. No schema bump (no field additions/removals).
- `harvest_layer_plan_ir` (`dispatch.rs:1612-1690`) is OUT OF SCOPE. `ActiveRegion.resolved_config` remains `ResolvedConfig::default()` from the module path; the host stamps the authoritative value at RegionMapping time. This narrows authority: modules cannot tamper with config that downstream stages will read.
- `bind_module_config_view` (`execution_plan.rs:63-91`) is UNCHANGED. Per-module `ConfigView` filtering by declared schema continues to work exactly as today.
- `parse_cli_config_source` (`execution_plan.rs:193-211`) is UNCHANGED. JSON shape limits (no nested objects, no `null`) persist.
- Per-object overlay key convention is `object_config:<object_id>:<config_key>` — flat, colon-delimited, mirrors the existing `object_height:<object_id>` host-injected pattern. `<object_id>` MUST match `ObjectMesh.id` (UUID-ish strings already produced by `model_loader`).
- Determinism: `BTreeMap<String, ResolvedConfig>` (not `HashMap`) is required for the per-object resolved map so iteration order is stable across runs (matches the `RegionMapIR` deterministic-iteration contract in `docs/04`).

## Data and Contract Notes

- IR or manifest contracts touched:
  - `RegionPlan.config: ResolvedConfig` — same shape; this packet changes how the value is sourced, not the shape.
  - `ResolvedConfig.extensions: HashMap<String, ConfigValue>` — used by the permissive fallback for unknown keys (already documented as the overflow bucket in `docs/02 §304-344`).
  - No `SemVer` bumps anywhere.
- WIT boundary considerations: none. WIT files unchanged; per-module `ConfigView` filtering unchanged.
- Determinism or scheduler constraints:
  - `BTreeMap<String, ResolvedConfig>` (sorted by `object_id`) keeps RegionMapping iteration deterministic.
  - The default-fallback `ResolvedConfig` is computed once at startup; `Arc`-shared across rayon workers; immutable.

## Locked Assumptions and Invariants

- `ObjectMesh.id` is the canonical `<object_id>` string used in all overlay keys; it is already populated by `model_loader::load_model` and never mutated.
- `parse_cli_config_source` continues to flatten any input that is not a top-level JSON object into `ConfigSourceParseError::NotAnObject`. The resolver receives only `HashMap<String, ConfigValue>` after that gate.
- `ResolvedConfig::default()` is the correct fallback when no user input is supplied (matches existing behavior).
- `RegionMapIR.entries` is keyed by `(global_layer_index, object_id, region_id)`; the stamp uses `object_id` from the key, which is guaranteed non-empty by `harvest_layer_plan_ir`'s canonical-id parsing (`dispatch.rs:1598-1610`).
- A user passing `top_shell_layers = 0` is honored (not coerced) — this is the documented "disable" path in `layer_slice.rs:303,319` and `packet 35` AC.

## Risks and Tradeoffs

- **`ActiveRegion.resolved_config` becomes effectively dead** for the live path. Documented as advisory in the IR doc comment so future contributors don't add producers expecting it to be authoritative. No removal in this packet — keeping the field avoids a schema bump and existing test fixtures that build `ActiveRegion` continue to compile.
- **Module-emitted config from a future custom layer planner is silently overwritten**. Acceptable because the host built-in is defined as the single authoritative resolver of per-region config (`docs/04 §RegionMapIR Compilation`). Documented in the new module's rustdoc.
- **CLI shape grows new key family `object_config:<id>:<key>`** without docs/03 schema declaration. Mitigated by the existing `object_height:*` host-injected precedent; the resolver is the only consumer, so module manifests don't need to declare these keys.
- **Strict typing on declared fields can break a user passing a quoted integer** (`"top_shell_layers": "4"`). Acceptable — caught by the new negative-case AC; CLI exits with a clear error.
- **Test surface area grows by 3 files**, but this packet adds ≤ 80 LOC of test per file and no test reads the workspace beyond the new resolver and RegionMapping path.
