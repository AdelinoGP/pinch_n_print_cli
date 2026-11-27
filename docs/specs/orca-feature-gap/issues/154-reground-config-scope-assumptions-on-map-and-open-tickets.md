# 154 — Re-ground config-scope assumptions (`ADRs 0067–0070`) across Map Notes and open tickets

Type: research
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

**How did `origin/master`'s `config-scope-resolution` series (packets `01`–`10`, `docs/adr/0067`–`0070`) change the factual premises of `map.md`'s standing config Notes, `118`'s per-tool inventory, and open tickets `125`, `128`, `132`, and `140`?**

Since the map paused on 2026-09-11, `origin/master` landed a unified config-schema and scope-resolution stack: `ConfigSchemaRegistry` and `expand_automatic_values` (`crates/slicer-config/src/lib.rs`), `ConfigIngestor` / `ScopedConfig` / `ScopeDelta` / `LayerConfigRange` (`crates/slicer-config/src/ingestion.rs`), `resolve_scope_stack` / `query_z_grid` / `query_layer_height_profile` (`crates/slicer-config/src/resolution.rs`), typed `ModifierKind` (`crates/slicer-ir/src/slice_ir.rs`), and scope-eligibility deny-lists. At filing (`f899cdf2`), `resolve_per_tool_configs` no longer exists anywhere in the tree, while `ConfigBoundsIndex` (`crates/slicer-scheduler/src/config_resolution.rs`) and `ConfigView::from_declared` / `ConfigView::get_abs_value` (`crates/slicer-ir/src/slice_ir.rs`) still exist.

Measure and verify every item below directly against the merged tree (do not infer from ADR prose alone):

1. **Map Note — "A manifest `default =` is DEAD for any plain-typed key that also exists as a `ResolvedConfig` field":** Does `ConfigSchemaRegistry` + `resolve_scope_stack` still drop manifest defaults behind `ResolvedConfig::default()`, or is this hazard retired / narrowed?
2. **Map Note — "`ConfigView::from_declared` whitelists by the module's schema keys":** How does `config-scope-resolution_06_resolved-config-view` bind and filter module configs now? Is the silent-drop hazard still live?
3. **Map Note & Ticket `118` / `125` — Per-tool config model (gates 12 open refiled tickets: `119`, `136`–`147`):**
   - Update the factual picture from [118-asset-per-tool-config-inventory.md](./118-asset-per-tool-config-inventory.md) against `ConfigIngestor`, `ScopedConfig`, `resolve_scope_stack`, and `ExpansionContext`: how does `tool_config:<u32>:<key>` work now, what changed for `overlay_resolved` (retired on `master`), how do per-tool bases interact with `expand_automatic_values`, and which of `125`'s sub-questions (Orca `coFloats` vector ingest, unpainted-region tool identity, module per-tool surface, separate extruder axis, `nozzle_diameter` home) are already answered vs still genuinely open?
4. **Open ticket [128 — Fix the units mismatch on `percent` / `float_or_percent` config keys](./128-percent-key-numeric-spelling-units-mismatch.md):** Trace all four JSON spellings (`100.0`, `1.0`, `100`, `"100%"`) through `ConfigIngestor`, `ConfigBoundsIndex`, `expand_automatic_values`, and `ConfigView::get_abs_value`. Did `config-scope-resolution` fix this class-wide, partially fix it (only for keys with `base_key: Some(_)`), or leave the bare-number vs percent-string mismatch intact?
5. **Open ticket [132 — Author packet — the CONFIG_BLOCK is a contract with OrcaSlicer's reader](./132-author-packet-config-block-reader-contract.md):** Does `ConfigSchemaRegistry` now supply the canonical type/default metadata Half A and Half B needed, changing `132`'s design and size?
6. **Open ticket [140 — e2e red: ticket 42's strict pressure-advance ingest rejects real Orca 3MF vectors](./140-e2e-pressure-advance-vector-ingest-regression.md):** Check whether commit `44e10bcb` and/or `ConfigIngestor` already resolved the 5 e2e failures, or what residual vector-ingest contract remains for [125](./125-rule-per-tool-config-model.md).
7. **Schema-registration contract for future packets:** Summarize the exact current rules a packet must follow when adding or modifying a config key (manifest vs `docs/config/host-keys.toml` vs `ConfigSchemaRegistry` vs `denied_scopes` vs `base_key` vs `ModifierKind` vs layer-range scope), so Phase 2 packet rewrites follow one authoritative checklist.

### Deliverable

- A linked markdown asset (`./154-asset-config-scope-regrounding.md`) giving verified symbol-level answers to items 1–7, exact dispositions for each config Map Note, and concrete premise updates for tickets `125`, `128`, `132`, and `140`.
- **Read-only research ticket:** do not edit `map.md` or close `125`/`128`/`132`/`140` here; [156 — Reconcile Map Notes, stale `⚠` badges, and remaining open tickets against current `master`](./156-reconcile-map-notes-stale-warnings-and-remaining-open-tickets.md) applies the Map Note updates and each open ticket resolves on its own session using this asset.

### Obligations

- Cite in-tree code by symbol name + crate-qualified path; never bare line numbers.
- Re-derive every symbol and behaviour from the merged tree at claim time.

## Answer
