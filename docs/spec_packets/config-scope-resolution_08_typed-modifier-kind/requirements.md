# Requirements: typed-modifier-kind

## Packet Metadata

- Grouped task IDs: `TASK-569`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The loader already parses modifier classification as `PartSubtype`, but `resolve_object` converts it to `config_delta.fields["subtype"]`; ten production sites then compare strings, and modifier settings bypass registry typing, bounds, and eligibility. Meanwhile `ModifierScope`/`ModifierVolume.applies_to` advertises feature restriction that production never reads. This coherent slice replaces both false contracts without changing modifier geometry semantics.

## In Scope

- Add public serde-capable `slicer_ir::ModifierKind` with exactly `ParameterModifier`, `NegativePart`, `SupportEnforcer`, and `SupportBlocker`, and add `ModifierVolume.kind: ModifierKind`.
- Keep host-local `PartSubtype::{NormalPart,ModifierPart,NegativePart,SupportEnforcer,SupportBlocker}` as the XML parser representation; `NormalPart` remains solid geometry, while the four non-normal variants map once in `resolve_object` to `ModifierKind`.
- Remove routing key `subtype` from newly built `ModifierVolume.config_delta`; metadata with the literal key is reserved and cannot overwrite `kind`.
- Replace all ten grounded production string classifications with exhaustive matches on `ModifierKind`: three in `region_mapping.rs`, two in paint segmentation, and five in runtime.
- Preserve exact behavior: parameter modifiers are stampable wall-less sub-regions; negative parts subtract; support enforcer/blocker become paint/support semantics, and only those two support kinds are skipped by the ordinary split/stamp/collect sites, so parameter-modifier and negative-part volumes keep their current non-support routes.
- Feed every modifier's non-routing metadata to packet 03's `ConfigIngestor::ingest_delta(ConfigScope::Modifier { object_id, modifier_id }, ..)` after registry assembly, retain it in `ScopedConfig`, and have packet 05's resolver apply that delta. Both `run_slice_with_collector` and `prepare_prepass_context` use the same composition helper.
- Enforce packet 07 admission before merge: denied keys and invalid typed/bounded values fail atomically, rather than being copied into `ResolvedConfig.extensions`.
- Delete `ModifierScope`, its public re-export, `ModifierVolume.applies_to`, all literals/imports/tests, and retire false tests in `acceptance_gate_gaps_tdd` that assert non-production per-feature scope behavior.
- At activation, re-ground `CURRENT_MESH_IR_SCHEMA_VERSION`, then increment its minor component exactly once with major unchanged and patch zero under the explicit owner decision; update its source comment and docs. The authoring-time value is 1.1.0, not a future target lock.
- Preserve deserialization of legacy MeshIR 1.1.0 modifier payloads by deriving kind from legacy `config_delta.fields["subtype"]` and dropping that routing entry. If correct legacy classification cannot be retained, stop for escalation.
- Cover the IR/serde contract, four-kind loader mapping, all behavior routes, registry admission through both runtime entry points, denied/invalid rejection, and geometry via a visual-debug manifest/image assertion.
- Update `docs/02_ir_schemas.md`, `docs/04_host_scheduler.md`, and ADR-0070 as specified by `AC-6`.

## Out of Scope

- Layer-range ingestion, interval matching, Z-grid re-derivation, or `ConfigScope::LayerRange`; queue row 9 owns those.
- New modifier kinds, changed priority values, perimeters at modifier boundaries, or a replacement per-feature modifier scope.
- WIT, guest, manifest-schema, CLI-wire, or non-MeshIR schema changes.
- Changes to global/object/paint/tool precedence or packet 06's resolved `ConfigView` work.
- Aliases for unrelated config names or automatic-value expansion.

## Authoritative Docs

- `docs/specs/config-scope-resolution-plan.md` — bounded Modifiers, owner decision, cross-cutting, and row-8 authority.
- `docs/adr/0070-typed-modifier-kind-replaces-modifier-scope.md` — accepted decision.
- `docs/adr/0030-modifier-splits-fill-not-perimeters.md` — retained wall-less geometry.
- `docs/02_ir_schemas.md` and `docs/04_host_scheduler.md` — bounded modifier/RegionMapping sections.
- `docs/19_visual_debug.md` — Request Shape and silhouette/RegionMapping semantics only.
- `docs/21_data_defaults_and_fixtures.md` and `docs/22_test_quality.md` — literal blast radius and truthful-test policy.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintApply.cpp` — confirm by function name that support modifier kinds remain excluded from ordinary per-volume config application and parameter modifiers retain wall-less region behavior; no C++ line-number citation may be emitted.

## Acceptance Summary

- Positive: `AC-1` through `AC-6` prove typed loader output, exhaustive behavior parity at all ten sites, registry routing through both entry points, the activation-derived MeshIR minor compatibility contract, visual geometry, and docs.
- Negative: `AC-N1` and `AC-N2` prove atomic rejection and complete retirement of string/scope surfaces and false tests.
- Cross-packet impact: consumes draft/forward contracts from packets 03, 05, and 07; exports `ModifierKind`, `ModifierVolume.kind`, and the next live MeshIR minor for later packets.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-ir --all-targets --test ir_tests typed_modifier_kind_mesh_ir_minor_contract -- --exact 2>&1 | tee target/test-output.log >/dev/null; rg -q "test typed_modifier_kind_mesh_ir_minor_contract .* ok" target/test-output.log'` | IR shape, activation-derived version, and legacy serde compatibility | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-model-io --all-targets --test threemf_sidecar_classification_tdd modifier_parts_cross_mesh_ir_with_typed_kinds -- --exact 2>&1 | tee target/test-output.log >/dev/null; rg -q "test modifier_parts_cross_mesh_ir_with_typed_kinds .* ok" target/test-output.log'` | Four XML subtype mappings | FACT pass/fail |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --all-targets --test algo_region_mapping_tdd 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` | Registry-backed parameter-modifier mapping behavior | FACT pass/fail |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p slicer-runtime --all-targets --test e2e typed_modifier_kind_tdd -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` | All four routes, both entry points, and rejection | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `bash -lc 'set -euo pipefail; mkdir -p target; cargo test -p pnp-cli --all-targets --test typed_modifier_kind_visual_debug_tdd 2>&1 | tee target/test-output.log >/dev/null; rg -q "test result: ok" target/test-output.log'` | Required geometry bundle gate | FACT pass/fail |
| `cargo xtask build-guests --check` | Guest-artifact freshness before diagnosis | FACT exit code and stale names only |
| `cargo check --workspace --all-targets` | Workspace type/target gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Required lint gate | FACT pass/fail |
| `cargo xtask check-literals` | Struct-literal discipline | FACT pass/fail |
| `cargo xtask check-test-quality --report` | Touched-test quality review | FACT findings in touched files only |

## Step Completion Expectations

- Introduce and test the enum plus a temporary compatibility constructor/accessor before migrating consumers; no step may add `ModifierVolume.kind` until every existing literal has been replaced by that constructor in preceding steps.
- The final field-removal/version step owns only the now-centralized constructor/serde implementation and IR assertions; it removes the temporary subtype-backed bridge in the same packet changeset.
- Runtime registry wiring precedes deletion of direct extension stamping, and behavior tests must remain green without reading a subtype string.

## Context Discipline Notes

- `crates/slicer-core/src/algos/paint_segmentation/mod.rs`, `crates/slicer-runtime/src/layer_executor.rs`, `crates/slicer-runtime/src/prepass.rs`, `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`, and both scheduler docs are long; read only named symbols/ranges.
- Never load binary 3MF fixtures directly. Build the focused fixture in test code or delegate ZIP inspection.
- The literal inventory is broad but mechanical; dispatch it as bounded `LOCATIONS` batches and do not browse unrelated tests.
