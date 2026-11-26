# 162 — Delete the production-dead overlay-composition engine

Type: task
Status: resolved
Assignee: wayfinder session (2026-11-26)
Blocked by: —
Map: ../map.md

## Question

Ticket 152 classified `202f812d` as superseded-by-master: production always
passes a host config authority, and `resolve_scope_stack` owns precedence.
`ResolvedConfig::overlay_onto` / `overlay_resolved` are reachable only from the
`host_config: None` compatibility path.

Deleting them is not a dead-code deletion:

- `crates/slicer-core/tests/property_prepass_common/model.rs` (the H-REGIONMAP
  witness) drives `execute_region_mapping_inner` with `None` plus explicit
  `tool_configs`, and asserts the overlay result.
- `crates/slicer-core/tests/algo_region_mapping_tdd.rs` has two precedence
  tests over the same path (per-tool over paint; paint and tool on one key).
- `tool_configs` threads through `crates/slicer-runtime/src/prepass.rs` ->
  `crates/slicer-runtime/src/builtins/region_mapping_producer.rs` ->
  `execute_region_mapping_inner`.

## Acceptance

- Delete `overlay_onto`, `__drc_overlay_arms!`, `overlay_resolved`, and the
  `host_config: None` branches, or rebuild the compatibility entry point on
  `ScopedConfig` + `resolve_scope_stack`.
- Rewrite or delete the H-REGIONMAP compatibility witness and the two
  precedence tests; do not add new tests against the overlay path.
- Remove the now-dead `tool_configs` parameter from the region-mapping call
  chain, or record why it stays.
- Precedence stays covered by
  `crates/slicer-config/tests/resolved_config_view_tdd.rs` and
  `crates/slicer-config/tests/automatic_value_expansion_tdd.rs`.
- `cargo build --workspace`, the narrow region-mapping tests, and
  `cargo clippy --workspace --all-targets -- -D warnings` green.

## Notes

User ruling 2026-11-26 (ticket 153): delete/unify, but land it immediately
after the `orca-feature-gaps` merge rather than blocking a 140-commit
integration on a production-dead path. The engine cannot change slice output.

## Answer

Resolved 2026-11-26. The production-dead overlay-composition engine is deleted
and all remaining precedence witnesses run through the real typed scope
resolver (`ScopedConfig` + `resolve_scope_stack` + `resolved_target_key`):

- **Deleted overlay engine (`crates/slicer-ir/src/resolved_config.rs`,
  `crates/slicer-core/src/algos/region_mapping.rs`):** removed
  `__drc_overlay_arms!`, `ResolvedConfig::overlay_onto`,
  `overlay_onto_drift_guard`, `overlay_resolved`, `overlay_origin`, and all
  three `if host_config.is_none()` overlay blocks in
  `execute_region_mapping_inner`. Non-authority calls
  (`execute_region_mapping` / `execute_region_mapping_with_cap`) now keep each
  region's `ActiveRegion.resolved_config` (plus modifier delta extensions on
  minted sub-regions) and record `RegionPlan.paint_overrides` metadata without
  performing ad-hoc config composition.
- **Removed dead `tool_configs` parameter from the region-mapping call chain:**
  dropped from `execute_region_mapping_inner`
  (`crates/slicer-core/src/algos/region_mapping.rs`),
  `commit_region_mapping_builtin`
  (`crates/slicer-runtime/src/builtins/region_mapping_producer.rs`), and the
  `RESOLVED_TOOL_PREFIX` extraction in `crates/slicer-runtime/src/prepass.rs`.
  `RESOLVED_TOOL_PREFIX` in `crates/slicer-runtime/src/run.rs` is narrowed to
  private `const` (retained solely so `validate_support_layer_heights` and
  `configured_fill_holders` continue to validate tool-scoped entries in
  `target_configs`).
- **Migrated precedence witnesses off `host_config: None`:**
  - `crates/slicer-core/tests/algo_region_mapping_tdd.rs`: rewrote
    `region_mapping_applies_per_tool_config_overlay_to_painted_tool` and
    `region_mapping_per_tool_config_overrides_paint_semantic_on_same_key` to
    resolve targets via `resolve_scope_stack` and pass `host_config: Some(...)`
    to `execute_region_mapping_inner`.
  - `crates/slicer-core/tests/property_prepass_common/{model,controls}.rs` and
    `docs/testing/properties.md#h-regionmap`: removed `expected_compat_config`
    from `H-REGIONMAP` (`execute_region_mapping_with_cap` keeps parent base
    configs) and replaced `lexical_overlay_witness` with
    `authority_precedence_witness` over `ScopedConfig` + `resolve_scope_stack`
    + `host_config: Some(...)`.
  - `crates/slicer-runtime/tests/integration/region_mapping_paint_semantic_tdd.rs`:
    migrated `region_overlap_applies_override` and
    `overlap_precedence_is_deterministic` to resolve paint-semantic targets via
    `resolve_scope_stack` and call `execute_region_mapping_inner` with
    `host_config: Some(...)`.
- **Verification:** `cargo test -p slicer-ir`, `cargo test -p slicer-core
  --features host-algos --test algo_region_mapping_tdd` (24/24), `cargo test -p
  slicer-core --features "host-algos property_testing" --test
  property_prepass_controls` (8/8), `cargo xtask test --summary --properties
  --property H-REGIONMAP --tier routine` (PASS, guests rebuilt and verified
  clean via `cargo xtask build-guests --check`), `cargo test -p slicer-config
  --test resolved_config_view_tdd --test automatic_value_expansion_tdd --test
  scope_resolution_tdd` (23/23), `slicer-runtime` region-mapping unit/integration/e2e/controls
  tests (49/49), `cargo build --workspace`, `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo xtask check-literals` (0 violations),
  `cargo xtask check-fmt` (62/62 clean), and `cargo xtask check-test-quality
  --report` (0 findings in touched files).
