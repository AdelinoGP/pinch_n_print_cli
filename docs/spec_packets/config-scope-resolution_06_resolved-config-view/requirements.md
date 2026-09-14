# Requirements: resolved-config-view

## Packet Metadata

- Grouped task IDs: `TASK-567`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The same `ConfigView` type currently denotes both resolved region config and raw object fallback config. Defaults therefore fail to reach guests, 89 literal fallbacks across 13 guests carry behavior, extensions can bypass the registry contract, and `ResolvedConfig::to_config_map` hand-restates emission keys. Unknown values also remain retained after warning even though the approved staged migration permits dropping them once a no-drop oracle proves declared values are not lost.

## In Scope

- Reconcile packet 05's landed `resolve_scope_stack`, `query_z_grid`, `ResolvedObjectLayerConfig`, and WIT 2.0.0 exports before edits.
- Replace raw-source `bind_module_config_view` behavior with module-filtered materialization from the effective resolved config and registry defaults.
- Preserve module declaration encapsulation while guaranteeing every declared key is present.
- Route every extension write through registry type and bounds validation before storing it in `ResolvedConfig.extensions`.
- Add optional per-key `config_block` metadata to host and module declaration channels; default it to true.
- Mark exactly `mmu_segmented_region_max_width`, `mmu_segmented_region_interlocking_depth`, and `mmu_segmented_region_interlocking_beam` false.
- Replace hand-maintained `ResolvedConfig::to_config_map`/G-code emission selection with registry-driven effective-value projection.
- Delete the 89 production config-read-chain literal fallbacks under the user-approved classification and per-guest census.
- Add an independently generated one-value-per-registry-key config and combine it with `resources/cube_4color.3mf` in a no-skip `run_slice` e2e asserting zero unrecognized warnings.
- Flip packet 03's warn-and-keep behavior to warn-and-drop only after that e2e is green.
- Update resolved-view and manifest metadata docs.

## Out of Scope

- New config aliases or key renames.
- `denied_scopes` authoring/enforcement and inert manifest-table deletion (packet 07).
- Typed modifier kind, layer-range ingestion/application, and Phase-C speed expansion (packets 08–10).
- Changing `ResolvedConfig.extensions`' storage type, interner identity, or hash semantics.
- Treating `unwrap_or_else`, sort comparators, non-config `Option` values, tests, or non-literal fallbacks as part of the 89-site cleanup.
- Any WIT or public IR schema-version bump.

## Authoritative Docs

- `docs/specs/config-scope-resolution-plan.md` — bounded sections RC-4/RC-5, Extensions, Guests and delivery, and Cross-cutting requirements.
- `docs/adr/0067-unified-config-schema-registry.md` and `docs/adr/0068-config-scope-is-a-wire-encoding.md` — registry and typed-ingestion authority.
- `docs/02_ir_schemas.md` and `docs/03_wit_and_manifest.md` — resolved config and manifest contracts.
- `docs/22_test_quality.md` — independent population derivation, no silent fixture skip, and negative controls.

## Acceptance Summary

- Positive: `AC-1` through `AC-7` in `packet.spec.md` cover resolved completeness, typed extensions, the exact fallback census, registry-driven emission, the no-drop oracle, warn-to-drop, and docs.
- Negative: `AC-N1` and `AC-N2` cover atomic bounds rejection and module encapsulation.
- Cross-packet impact: packets 08–10 may rely on guest views containing effective declared values without local literals.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-config --all-targets --test resolved_config_view_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` | Registry projection, extensions, and unknown-drop contract | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --all-targets --test contract resolved_config_view_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` | Production binding and 89-site census | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --all-targets --test e2e resolved_config_view_no_drop_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` | Real no-drop slice | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --all-targets --test integration gcode_header_thumbnail_config_blocks_tdd::config_block_is_registry_driven -- --exact --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "config_block_is_registry_driven .* ok" target/test-output.log'` | Effective CONFIG_BLOCK projection | FACT pass/fail |
| `cargo xtask build-guests --check` | Artifact-verified guest freshness | FACT exit code and stale names only |
| `cargo check --workspace --all-targets` | All-target type gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate | FACT pass/fail |
| `cargo xtask check-literals` | Struct-literal gate | FACT pass/fail |
| `cargo xtask check-test-quality --report` | Touched-test quality | FACT touched findings only |

## Step Completion Expectations

- Land registry projection before deleting guest fallbacks; each guest batch must compile against a complete view.
- The no-drop oracle must pass in retained/warn mode before the same step flips unknown entries to drop.
- Preserve the baseline 89 count and category exclusions in the census test so a smaller population cannot self-certify.

## Context Discipline Notes

Do not load `resources/cube_4color.3mf`; delegate archive inspection. Guest edits are grouped into bounded batches, and cargo work must return only pass/fail with bounded failure snippets.
