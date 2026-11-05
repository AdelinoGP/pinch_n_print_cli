---
status: implemented
packet: config-scope-resolution_07_scope-eligibility
task_ids:
  - TASK-568
---

# config-scope-resolution_07_scope-eligibility

## Goal

Make per-key `denied_scopes` the sole scope-admission authority, hand-author the initial machine/emitter denial roster across host and module declarations, pin it against a mechanical reachability derivation, and remove the two inert manifest allow-list sections.

## Problem Statement

Packet 01 provides validated, unioned `denied_scopes`, but broad host rows remain empty except `wall_generator`, module declarations do not yet carry the initial machine/emitter policy, and packet 05's resolver has no admission gate. Meanwhile two mandatory module-level allow-list sections remain in every core manifest even though 23 are empty and production never consumes either list. Eligibility therefore exists as metadata without complete authorship or enforcement.

## Architecture Constraints

- A missing denial means statable at every scope. Never synthesize a restrictive default.
- Multi-declarer `denied_scopes` remain a set union and deterministic regardless of declaration order.
- Scope validation occurs before mutation/merge/expansion; rejection returns no partial config.
- The mechanical derivation exists only in tests and reports candidate `(key, scope)` pairs. Production declarations are always hand-authored and reviewed.
- Exact whole-print denial order is `object`, `layer_range`, `modifier`, `paint_semantic`, `tool`; tool-capable denial order omits `tool`.
- Per-object admission is a registry query, not a new hard-coded roster.
- No IR schema, WIT, or CLI wire version changes.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Data and Contract Notes

- IR/manifest contracts: per-field optional snake_case `denied_scopes` remains the sole declaration; two module-level kebab-case section tables are deleted.
- WIT boundary: unchanged.
- Determinism/scheduler constraints: admission sets derive from reconciled registry entries; denial vectors normalize to canonical scope order; all-scope validation completes before merge.

## Locked Assumptions and Invariants

- Packet 01 landed `RegistryEntry.denied_scopes`, union reconciliation, unknown-scope validation, and `HostRuntimeKey.denied_scopes`; only `wall_generator` currently has host-runtime denials.
- FORWARD-DEP packet 05 exports `resolve_scope_stack`, `query_z_grid`, `ResolvedObjectLayerConfig { object_id: String, object_height: f64, layer_height: f64, first_layer_height: f64, support_raft_layers: u32 }`, `ResolutionError::InvalidObjectHeight`, and `prepass-layer-planning@2.0.0`.
- The two legacy sections exist in 24 manifests, are empty in 23, are required by ingestion, and have no production reader; the one populated manifest does not preserve authority through retirement.
- `nozzle_diameter` is declared by exactly 7 manifests in the grounded tree: classic-perimeters, machine-gcode-emit, traditional-support, tree-support, tree-support-planner, wave-overhangs, and arachne-perimeters (AC-1's mechanical filesystem discovery of `modules/core-modules/<id>/<id>.toml` confirms arachne's `[config.schema.nozzle_diameter]` carries the tool-capable `denied_scopes` — an earlier draft assumption that arachne does not declare the key is contradicted by the tree); each declaration receives the tool-capable denials. The tests are count-agnostic: the declarer set is derived from filesystem discovery, never a hand-listed roster.

## Risks and Tradeoffs

- A missing declarer annotation can be masked by union behavior; AC-1 checks every declarer, not only the aggregate.
- Reachability inference can become a disguised hand roster; derive from channel/stage metadata and include a mutation-style negative control.
- Deleting legacy fields can break builders/tests beyond manifest parsing; inventory methods and struct literals before editing.
