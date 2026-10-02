# Requirements: scope-resolution-module

## Packet Metadata

- Grouped task IDs: `TASK-566`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The current host resolves global, object, paint-semantic, and tool values through `resolve_global_config`, `resolve_per_object_configs`, `resolve_per_paint_semantic_configs`, `resolve_per_tool_configs`, and private `apply_overlay` in `crates/slicer-scheduler/src/config_resolution.rs`, then applies modifiers/paint/tool through `overlay_resolved` in `crates/slicer-core/src/algos/region_mapping.rs`. The former reparses wire prefixes and the latter infers presence by comparing full records with defaults, so explicit default-valued overrides and fields omitted from the hand-written merge are lost. `run_slice_with_collector` and `prepare_prepass_context` also independently format host-only object namespaces, and the layer-planner guest repeats that protocol across WIT.

## In Scope

- Add `slicer_config::resolution` as the sole scope-delta-to-`ResolvedConfig` authority.
- Export `resolve_scope_stack` for RegionMapping and `query_z_grid` for LayerPlanning.
- Preserve deterministic low-to-high order `global < object < layer range < modifier < paint semantic < tool`; row 5 implements current global/object/modifier/paint/tool inputs and reserves layer range between object and modifier.
- Preserve modifier priority-ascending/last-writer-wins, paint-semantic lexical order, and tool-last behavior.
- Apply packet 04's `expand_automatic_values` only after applicable deltas merge and before interning or WIT delivery.
- Add `ResolvedObjectLayerConfig { object_id: String, object_height: f64, layer_height: f64, first_layer_height: f64, support_raft_layers: u32 }` as the host-side Z-grid result.
- Reject absent, non-finite, or non-positive object height as `ResolutionError::InvalidObjectHeight` without partial output.
- Migrate both production entry points and RegionMapping away from prefix reconstruction and `overlay_resolved`.
- Delete the five old scheduler resolver/overlay functions and `overlay_resolved` after their callers/tests use the unified API.
- Major-bump `slicer:prepass-layer-planning` exactly once to `2.0.0`; add WIT `object-layer-config` with the five fields and add `object-configs: list<object-layer-config>` to `run`.
- Thread generated WIT records through schema stage metadata, host dispatch, macro glue, SDK trait adaptation, the default guest, and the in-tree test guest.
- Delete guest functions `object_layer_height` and `object_height` and the corresponding `format!` host-prefix reads.
- Add table-driven expectations written as literal results, not computed by another resolver path.
- Update the three governing architecture/contract docs.

## Out of Scope

- Reading `Metadata/layer_config_ranges.xml` or adding `ConfigScope::LayerRange`.
- Implementing world-Z half-open interval matching, catch-up inheritance, layer-range overlap application, or layer-range selector-denial checks; queue row 9 owns them.
- Changing the settled overlap rules: later-starting range wins for `layer_height`; conflicting values for the same non-`layer_height` key are a load error when row 9 wires ranges.
- Scope eligibility declaration/enforcement (row 7), typed modifier kind (row 8), resolved `ConfigView` cleanup/drop-unknown (row 6), or remaining Phase-C expansion (row 10).
- Aliases, config key renames, IR schema changes, manifest vocabulary changes, or broad guest fallback cleanup.
- Any implementation of fictional pre-existing Rust symbols named `layer_height_profile_from_ranges` or `LayerRanges::assign`; neither exists in the Rust tree.

## Authoritative Docs

- `docs/specs/config-scope-resolution-plan.md` — direct ranged reads of Resolution, Layer range geometry semantics, Guests and delivery, Testing, and queue row 5.
- `docs/adr/0068-config-scope-is-a-wire-encoding.md` — typed delta and resolution-phase expansion decision.
- `docs/adr/0069-scope-eligibility-is-a-per-key-deny-list.md` — settled loud-rejection policy, not implemented here.
- `docs/02_ir_schemas.md` — ranged reads of Config Key Namespaces and modifier resolution.
- `docs/03_wit_and_manifest.md` — ranged reads of per-stage packages and prepass interfaces.
- `docs/04_host_scheduler.md` — ranged reads of LayerPlanning and RegionMapping.
- `docs/11_operational_governance_and_acceptance_gate.md` — ranged read of WIT major-version policy.
- `docs/22_test_quality.md` — ranged reads of self-referential oracles and independently derivable inputs.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Slicing.cpp` — `layer_height_profile_from_ranges` and related `layer_height_profile` behavior; confirm only the later-starting-wins Z-grid rule reserved for queue row 9, citing functions rather than line numbers.

## Acceptance Summary

- Positive: `AC-1` through `AC-6` in `packet.spec.md` cover precedence, explicit-default presence, Z-grid records, production-path convergence, WIT v2 delivery, and docs.
- Negative: `AC-N1` and `AC-N2` cover invalid object geometry and the no-fiction/no-premature-layer-range boundary.
- Cross-packet impact: packet 06 consumes always-resolved outputs; packet 07 adds eligibility; packet 08 supplies typed modifier deltas; packet 09 inserts and verifies layer ranges; packet 10 consumes resolved Phase-C bases.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test scope_resolution_tdd 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` | Unified resolver and Z-grid contract | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-scheduler --all-targets --test scheduler_integration config_resolution -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` | Migrated scheduler behavior | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --all-targets --test integration scope_resolution_module_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` | Both production entry points | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-wasm-host --all-targets --test contract prepass_layer_planning_v2_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` | Typed WIT v2 host/guest delivery | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo xtask build-guests --check` | Artifact-verified WIT/guest freshness | FACT exit code and stale guest names only |
| `cargo check --workspace --all-targets` | All target/type surfaces | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate | FACT pass/fail |
| `cargo xtask check-literals` | Struct-literal churn gate | FACT pass/fail |
| `cargo xtask check-test-quality --report` | Touched-test quality report | FACT findings in touched files only |

## Step Completion Expectations

- Do not delete an old resolver until all production and test callers compile against `slicer_config::resolution`.
- Land the WIT declaration, package metadata, generated adapters, dispatch call, both guests, and package-identity assertions as one coordinated compatibility change.
- Run guest freshness only after every WIT-dependent edit is present; rebuild stale guests before attributing a component failure.
- Preserve the exact overlap policy in docs while leaving its Rust implementation to row 9.

## Context Discipline Notes

The WIT bump crosses several generated-binding consumers; use symbol-location dispatches and bounded edits rather than reading whole macro/dispatch files. Never inspect generated bindings or guest artifacts directly.
