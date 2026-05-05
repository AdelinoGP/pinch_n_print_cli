# Requirements: 35a_resolved-config-propagation

## Packet Metadata

- Grouped task IDs:
  - `TASK-166`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`crates/slicer-host/src/dispatch.rs::harvest_layer_plan_ir` (line ~1651) hardcodes `resolved_config: ResolvedConfig::default()` on every `ActiveRegion` it builds from a `LayerProposal`. The host built-in `PrePass::RegionMapping` (`region_mapping.rs:190`) then clones `region.resolved_config` into `RegionPlan.config`, so every consumer of `RegionMapIR.entries[*].config` sees codebase defaults regardless of what the user supplied via `--config`.

The CLI parses `--config` correctly into a `HashMap<ConfigKey, ConfigValue>` at `main.rs:135-150` and feeds it to `bind_module_config_view` for per-module `ConfigView` filtering — that works. The missing surface is the host-side resolver that converts the same raw `HashMap` into `slicer_ir::ResolvedConfig` and stamps it on the per-region IR that downstream stages read. Without that, `top_shell_layers`, `bottom_shell_layers`, and every other declared `ResolvedConfig` field are unreachable from user input.

This was first surfaced by packet `35_multi-layer-top-bottom-thickness`, whose Part 2 binary E2E test (`benchy_multi_layer_top_bottom_evidence`) had to be strengthened to bypass the binary path precisely because config propagation is broken end-to-end (DEV-040). Packets `36_bridge-detector-orca-parity` and `37_fill-role-claims` are blocked on this fix because their behavior is config-tunable.

This packet does NOT modify packet 35 or its files; packet 35 closed correctly within its CONSUMER-side scope and remains `implemented`. This packet adds the missing PRODUCER-side plumbing.

## In Scope

- New module `crates/slicer-host/src/config_resolution.rs` with:
  - `pub fn resolve_global_config(source: &HashMap<ConfigKey, ConfigValue>) -> Result<ResolvedConfig, ConfigResolutionError>`
  - `pub fn resolve_per_object_configs(source: &HashMap<ConfigKey, ConfigValue>, object_ids: &[String]) -> Result<BTreeMap<String, ResolvedConfig>, ConfigResolutionError>`
  - `pub enum ConfigResolutionError { TypeMismatch { key, expected, actual } }` with `Display`/`Error`.
- `region_mapping::commit_region_mapping_builtin` accepts `&BTreeMap<String, ResolvedConfig>` and stamps each `RegionPlan.config` from the entry's `object_id`. Falls back to a `default` resolved config when an object has no entry.
- `pipeline::PipelineConfig` carries `Arc<BTreeMap<String, ResolvedConfig>>` (or equivalent shared reference) so `prepass::*` can hand it to the built-in.
- `slicer-host`'s `main.rs` calls `resolve_per_object_configs` once after `parse_cli_config_source` and threads the result through `PipelineConfig`.
- New flat CLI key convention `object_config:<object_id>:<config_key>` for per-object overrides (`object_id` matches the `ObjectMesh.id` already populated by `model_loader`).
- Tests:
  - `crates/slicer-host/tests/config_resolution_tdd.rs` (NEW) — resolver unit coverage including defaults, typed-field happy path, unknown-key passthrough into `extensions`, per-object overlay precedence, and `TypeMismatch` rejection.
  - `crates/slicer-host/tests/region_mapping_resolved_config_tdd.rs` (NEW) — RegionMapping stamps per-object `RegionPlan.config` from the supplied map, including the default-fallback case.
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` (EDIT — append) — two-run binary AC asserting strict inequality of `;TYPE:Top surface` / `;TYPE:Bottom surface` block counts under N=1 vs N=4, plus a CLI-rejects-bad-type negative case.

## Out of Scope

- Per-region (paint-keyed) config overlays. Requires WIT-level `region-id`-scoped config sources; tracked separately in any future packet that needs paint-keyed tunables.
- Print profile inheritance, vendor profile loading, or non-flat JSON config shapes. `parse_cli_config_source` continues to reject nested objects and `null`.
- Module-internal `ConfigView` filtering. `bind_module_config_view` already enforces the docs/03 declared-read invariant per module; this packet does not touch that boundary.
- Modifications to `harvest_layer_plan_ir`. `ActiveRegion.resolved_config` is left as a module-emitted advisory; the host stamps the authoritative value at RegionMapping time.
- Changes to `OrcaSlicerDocumented/`. None of its parity surfaces are relevant to this fix; default values are intentionally divergent from Orca and unchanged here.

## Authoritative Docs

- `docs/02_ir_schemas.md` — read directly, lines `~575-660` (`ResolvedConfig` definition with `top_shell_layers`/`bottom_shell_layers` fields and `extensions` overflow bucket) and lines `~360-410` (`RegionMapIR` / `RegionPlan` shape).
- `docs/03_wit_and_manifest.md` — delegate a SUMMARY of the `[config.schema]` declaration rules and `ConfigView` filtering invariants. Confirm that `ResolvedConfig` field names are the canonical key surface and that host-injected non-module-declared keys (e.g. `object_height:<id>`) are an existing precedent.
- `docs/04_host_scheduler.md` — delegate a SUMMARY of "PrePass lifecycle" and "RegionMapIR Compilation" sections only. File is large; never load in full.
- `docs/DEVIATION_LOG.md` — delegate a single FACT lookup of the DEV-040 row at packet completion to confirm the row can be flipped to `Closed`.

## OrcaSlicer Reference Obligations

- None. The codebase already deliberately deviates from Orca on the default values that this packet's plumbing exposes (`top_shell_layers = 3` here vs Orca's `4`). This packet does not change defaults; it only ensures user-supplied values propagate. Implementer MUST NOT load `OrcaSlicerDocumented/` for this packet.

## Acceptance Summary

- Positive cases: see `packet.spec.md` Acceptance Criteria. Each criterion ends with a pipe-suffixed `cargo test ... -- --exact --nocapture` command targeting a specific test name.
- Negative cases:
  - `resolve_global_config` returns `Err(ConfigResolutionError::TypeMismatch)` when a known field receives a wrong-typed JSON value (e.g. `top_shell_layers: "four"`).
  - The `slicer-host` binary exits non-zero with stderr mentioning the offending key and expected type when given a malformed declared field.
- Measurable outcomes:
  - `RegionMapIR.entries[*].config.top_shell_layers` equals the user-supplied (or per-object overridden) value, never the codebase default when the user supplied a value.
  - Strict inequality of `;TYPE:Top surface` and `;TYPE:Bottom surface` block counts on Benchy under `top_shell_layers = 1` vs `top_shell_layers = 4` proves the value reaches `classify_region_surfaces` end-to-end via the binary.
  - Existing `benchy_multi_layer_top_bottom_evidence` test continues to pass (the binary path it now exercises is no longer behaviorally divergent from its API-bypass Part 1).
- Cross-packet impact: closes DEV-040 in `docs/DEVIATION_LOG.md`; unblocks packets `36_bridge-detector-orca-parity` and `37_fill-role-claims`.

## Verification Commands

- `cargo test -p slicer-host --test config_resolution_tdd -- --nocapture` — delegation-friendly: returns one assertion per failing test name.
- `cargo test -p slicer-host --test region_mapping_resolved_config_tdd -- --nocapture` — same.
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_user_top_shell_layers_propagates_through_binary cli_rejects_top_shell_layers_string -- --nocapture` — runs only the two new tests touched by this packet.
- `cargo build --workspace` — workspace gate (delegate, FACT pass/fail).
- `cargo clippy --workspace -- -D warnings` — workspace gate (delegate, FACT pass/fail).

All commands above are delegation-friendly: each emits a small parseable signal (exit code + per-test status). Sub-agents must return FACT pass/fail or, on failure, SNIPPETS of the failing assertion plus ≤ 20 lines of context.

## Step Completion Expectations

For each step in `implementation-plan.md`:

- Precondition: stated per step.
- Postcondition: stated per step.
- Falsifying check: stated per step (always a `cargo test ... --exact` run dispatched to a sub-agent).
- Files allowed to read (with line-range hints when > 300 lines): listed in `design.md` §Read-Only Context and refined per step.
- Files allowed to edit (≤ 3): listed per step in `implementation-plan.md`.
- Expected sub-agent dispatches: listed per step.
- Step context cost: each step rated `S` or `M`. None rated `L`.

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated:
  - `crates/slicer-host/src/dispatch.rs` is large; only `harvest_layer_plan_ir` (line `~1612-1690`) needs ranged reading, and only to confirm the producer-side default that this packet leaves alone. Do NOT open the rest.
  - `docs/04_host_scheduler.md` — large; delegate a SUMMARY of just "PrePass lifecycle" / "RegionMapIR Compilation".
- OrcaSlicer trees the implementer must NOT load directly: anything under `OrcaSlicerDocumented/`. Not relevant to this packet.
- Likely temptation reads (skip):
  - `crates/slicer-host/src/wit_host.rs` — the WIT boundary for ConfigView is unchanged here; this packet stamps `RegionPlan.config`, not the per-module `ConfigView`. Reading `wit_host.rs` will inflate budget without informing the implementation.
  - `modules/core-modules/layer-planner-default/` — the layer planner's `LayerProposal` output is left as-is; per-region config no longer flows through the module surface.
  - Full `slicer_ir::slice_ir.rs` — only `ResolvedConfig` (lines `~575-660`) is needed; do not read other IR types.
- Sub-agent return-format hints for the heaviest dispatches:
  - "Run `cargo test -p slicer-host --test <name>`; return FACT pass/fail. On fail, return SNIPPETS of failing assertion + ≤ 20 lines."
  - "Summarize `docs/04_host_scheduler.md` §RegionMapIR Compilation; ≤ 200-word SUMMARY, no code."
