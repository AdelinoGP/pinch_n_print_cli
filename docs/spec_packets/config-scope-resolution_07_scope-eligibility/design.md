# Design: scope-eligibility

## Controlling Code Paths

- Primary code path: packet 05's `resolve_scope_stack` validates each `ScopeDelta` against `ConfigSchemaRegistry::admission_set` before merge and expansion.
- Declaration paths: `HostConfigKey`/`HostRuntimeKey` and `declare_resolved_config!` (`crates/slicer-ir/src/resolved_config.rs`), `SPEED_KEYS`/`SPEED_META` (`crates/slicer-ir/src/feedrate.rs`), and module `[config.schema.<key>].denied_scopes` parsed by `read_config_schema` (`crates/slicer-scheduler/src/manifest.rs`).
- Legacy path: `LoadedModule::{overridable_per_region,overridable_per_layer}`, builder fields/methods, and required table reads in `ingest_manifest_text`.
- Neighboring tests: new `crates/slicer-config/tests/scope_eligibility_tdd.rs`, registry assembly/census tests, and scheduler `manifest_ingestion_tdd`.
- OrcaSlicer comparison: none; ADR-0069 and the approved owner direction govern this deliberately permissive policy.

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

## Code Change Surface

- Selected approach: enrich host declaration carriers/DSL with explicit static denial slices, author matching module fields, expose one registry admission query, validate deltas in the unified resolver, and retire module-level allow lists.
- Net-new public surface: `HostConfigKey.denied_scopes: &'static [&'static str]`; `ConfigSchemaRegistry::admission_set(&self, scope: &ConfigScope) -> BTreeSet<ConfigKey>`; `ResolutionError::ScopeDenied { key: ConfigKey, scope: ConfigScope }`.
- Speed declaration shape: add a positionally aligned `SPEED_DENIED_SCOPES` table typed by `SPEED_KEY_COUNT`; all 26 entries use the whole-print-only denial slice and a const assertion locks alignment with both `SPEED_KEYS` and `SPEED_META`.
- Mechanical derivation: test helper joins registry provenance with host channel consumption and loaded module execution granularity, then compares candidate pairs with authored registry denials. It must demonstrate independence by a negative control that removes one authored denial and observes a mismatch.
- Rejected alternatives: generated production denials violate owner direction; retaining legacy lists as optional aliases preserves two authorities; allow lists invert the accepted default; validating after merge risks partial state.

## Files in Scope (read + edit)

- `crates/slicer-ir/src/{resolved_config.rs,feedrate.rs}` and affected focused tests — host declarations and speed metadata.
- `crates/slicer-config/src/{lib.rs,resolution.rs}` plus `tests/scope_eligibility_tdd.rs` — admission and resolver enforcement (use packet 05's final actual path).
- `crates/slicer-scheduler/src/manifest.rs` and manifest ingestion tests — legacy model/parser retirement.
- All 24 `modules/core-modules/*/<module>.toml` files — remove both legacy sections; add denials only to roster declarers.
- `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md` — contract updates.

## Read-Only Context

- `docs/spec_packets/config-scope-resolution_05_scope-resolution-module/{packet.spec.md,requirements.md,design.md}` — resolver/error reconciliation.
- `crates/slicer-config/tests/{registry_assembly_tdd.rs,registry_census_tdd.rs}` — existing union/real-manifest fixtures.
- Module source call sites may be inspected only through bounded location dispatches to establish consumer reachability; they are not edit targets.

## Out-of-Bounds Files

- `docs/specs/config-scope-resolution-plan.md`, packet directories 01–06 and 08+, and unrelated docs.
- Guest Rust source, WIT, serialized IR, aliases, layer-range ingestion, modifier typing, and emission changes.
- `OrcaSlicerDocumented/**`, `target/`, `Cargo.lock`, generated code, vendored dependencies, and fixture contents.

## Expected Sub-Agent Dispatches

- Question: reconcile packet 05's final resolver module/path, delta iteration, and `ResolutionError`; scope: packet 05 exports and `slicer-config`; return: `FACT` ≤5 lines.
- Question: inventory every `HostConfigKey` literal/macro expansion input and `SPEED_KEYS` consumer affected by a denial field/table; scope: slicer-ir/config/scheduler tests; return: `LOCATIONS` ≤20 per type.
- Question: derive candidate unreachable `(key, scope)` pairs from current declaration/consumer/stage metadata and identify every declarer for AC-1 keys; scope: host channels and core manifests; return: `SUMMARY` ≤200 words plus `LOCATIONS` ≤20 per batch.
- Question: list the two retired section locations in all manifests; scope: `modules/core-modules/*/*.toml`; return: `LOCATIONS` in two ≤20 batches.
- Question: run exact cargo gates; scope: named command; return: `FACT` ≤5 lines or failure `SNIPPETS` ≤20 lines.

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

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (roster authorship across manifests)
- Highest-risk dispatch and required return format: derivation/declarer census, bounded `SUMMARY` plus `LOCATIONS` batches.

## Open Questions

- `[FWD]` Reconcile packet 05's final resolver and error types before activation; adapt names without weakening pre-merge atomic rejection.
- `[BLOCK]` None beyond the forward dependency.
