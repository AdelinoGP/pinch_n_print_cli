# 162 — Delete the production-dead overlay-composition engine

Type: task
Status: open
Assignee: —
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
