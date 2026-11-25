---
status: implemented
packet: config-scope-resolution_08_typed-modifier-kind
task_ids:
  - TASK-569
---

# config-scope-resolution_08_typed-modifier-kind

## Goal

Replace stringly modifier subtype routing with an exhaustive `ModifierKind` carried by `ModifierVolume`, validate modifier settings through the registry-backed modifier scope, and retire the inert `ModifierScope`/`applies_to` contract with the owner-approved one-minor MeshIR bump derived from the live constant at activation.

## Problem Statement

The loader already parses modifier classification as `PartSubtype`, but `resolve_object` converts it to `config_delta.fields["subtype"]`; ten production sites then compare strings, and modifier settings bypass registry typing, bounds, and eligibility. Meanwhile `ModifierScope`/`ModifierVolume.applies_to` advertises feature restriction that production never reads. This coherent slice replaces both false contracts without changing modifier geometry semantics.

## Architecture Constraints

- `PartSubtype` is host-local XML syntax; `ModifierKind` is the IR-domain classification. `NormalPart` has no `ModifierKind` variant because it never creates a `ModifierVolume`.
- `ModifierKind` has exactly four variants: `ParameterModifier`, `NegativePart`, `SupportEnforcer`, and `SupportBlocker`. Production classification uses exhaustive `match` without wildcard arms, string conversion, or helper predicates that hide new variants from the ten consumers.
- `config_delta` remains authored settings only. The XML routing attribute `subtype` is not a config key, cannot be supplied as an override, and is removed from newly emitted deltas.
- Modifier delta typing and admission happen before resolver mutation. Rejection produces no partial `ResolvedConfig` or interner entry.
- Existing priority values remain 0, 100, 200, and 300 for parameter, negative, enforcer, and blocker kinds respectively.
- Parameter modifiers remain wall-less fill sub-regions under ADR-0030; support kinds stay on paint/support paths; negative parts stay on subtract.
- The owner decision mandates a minor MeshIR bump despite removing `applies_to`. Legacy 1.1.0 deserialization must recover kind from the old subtype entry or implementation escalates.
- Every existing `ModifierVolume` literal is part of the field-change blast radius. Use constructors/helpers or edit each literal in the field-change step; do not wait for `cargo check` to discover one.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Data and Contract Notes

- IR/manifest contracts: the next live MeshIR minor adds `ModifierVolume.kind` and removes `applies_to`; no manifest change. `ConfigDelta` continues storing authored config values, never routing metadata.
- WIT boundary: unchanged; MeshIR is host-owned and guest modules see resolved per-region config rather than `ModifierVolume`.
- Determinism/scheduler constraints: priorities and stable tie ownership do not change; registry normalization/admission precedes resolution; both runtime entry points share one modifier-ingestion implementation.
- Compatibility: legacy read is one-way. Pre-change subtype/applies-to input may be accepted, but all post-bump serialization uses typed kind and omits both legacy surfaces.

## Locked Assumptions and Invariants

- FORWARD-DEP packet 03 exports `ConfigScope::Modifier { object_id, modifier_id }`, `ConfigIngestor::ingest_delta`, `ScopeDelta`, `ScopedConfig`, and `IngestionOutcome` with the shapes recorded in its design.
- FORWARD-DEP packet 05 exports `resolve_scope_stack`, `ResolutionTarget`, and `ResolutionError`; modifier precedence remains above object/future layer range and below paint/tool.
- FORWARD-DEP packet 07 exports `ConfigSchemaRegistry::admission_set(scope)` and `ResolutionError::ScopeDenied { key, scope }` and removes the inert manifest sections.
- Current MeshIR constant is re-grounded as 1.1.0 in `CURRENT_MESH_IR_SCHEMA_VERSION`; activation must re-ground it and compute one minor increment rather than trust this mutable ledger fact.
- The production subtype inventory is exactly five `slicer-core` reads and five `slicer-runtime` reads, in the ten symbols listed above.
- `ModifierScope` is read by no production code; `acceptance_gate_gaps_tdd` contains synthetic tests that claim behavior production never implements and must be retired.
- `PartSubtype::NormalPart` never becomes a `ModifierVolume`; every `ModifierVolume.kind` is one of the four non-normal variants.

## Risks and Tradeoffs

- A wildcard enum arm would preserve compilation but lose ADR-0070's compile-time exhaustiveness; static review and behavior tests require explicit arms.
- Legacy serde can misclassify old support/negative fixtures if it defaults kind rather than reading subtype; AC-4 locks representative legacy values.
- Routing modifier values through the registry can expose formerly tolerated invalid metadata. This is intended, but errors must name key/scope and remain atomic.
- The struct-literal blast radius is broad. Mechanical constructor/literal migration must be split into small steps without omitting embedded test modules in production files.
- Geometry behavior is intentionally unchanged but config delivery changes; the visual-debug bundle catches silent shape changes not visible in unit values.
