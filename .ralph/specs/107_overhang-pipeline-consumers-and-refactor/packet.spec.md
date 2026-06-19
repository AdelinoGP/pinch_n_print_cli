---
status: draft
packet: 107_overhang-pipeline-consumers-and-refactor
task_ids:
  - O-T030
  - O-T031
  - O-T032
  - O-T040
  - O-T041
  - O-T042
  - O-T050
  - O-T051
  - O-T052
  - O-T053
backlog_source: docs/specs/overhang-pipeline-restructuring.md
context_cost_estimate: M
---

# Packet Contract: 107_overhang-pipeline-consumers-and-refactor

## Goal

Land the consumer-side half of the overhang pipeline restructuring: add `SliceRegionView::overhang_quartile_polygons()` (and confirm `overhang_areas()` from P104 now returns non-empty data), refactor `overhang-classifier-default` from a wall-distance computer to a pure-consumer that reads per-vertex `overhang_quartile` from `LayerCollectionView` entities and applies speed factors only, register an end-to-end overhang-quartile propagation TDD, and close perimeter-roadmap deviations D-10 / D-12 / D-OVERHANG-QUARTILE-NONE while unblocking T-024 and T-077.

## Scope Boundaries

Touches `crates/slicer-sdk/src/views.rs` (add `overhang_quartile_polygons()` accessor; add `overhang_areas()` stub per P104's plan), `crates/slicer-schema/wit/deps/ir-types.wit` + `crates/slicer-wasm-host/src/host.rs` (WIT + populator), `modules/core-modules/overhang-classifier-default/src/{lib,classify,lines_distancer}.rs` (refactor + deletion), the module manifest (drop broad `LayerCollectionIR` read; add narrow `overhang_quartile` read declaration), a new end-to-end TDD, and `docs/01_system_architecture.md` + `docs/02_ir_schemas.md` + `docs/DEVIATION_LOG.md`. No perimeter module `lib.rs` changes — perimeter-side propagation of `overhang_quartile` per-vertex (T-024 from P104) requires follow-up after P104 ships; tracked separately.

## Prerequisites and Blockers

- Depends on (both `status: draft` — FORWARD-DEPs; do NOT treat as satisfied):
  - **FORWARD-DEP on draft P106 (overhang-pipeline-prepass-foundation)** — needs `SurfaceClassificationIR.overhang_quartile_polygons: HashMap<u32, Vec<QuartileBand>>` and `QuartileBand { quartile: u8, polygons: Vec<ExPolygon> }` (P106-produced). Also needs `OverhangRegion.xy_footprint: Vec<ExPolygon>` (already present in `slice_ir.rs:581` — P106 populates it at runtime). This packet's view accessors return empty data until P106 ships.
  - **FORWARD-DEP on draft P104 (perimeter-propagation-and-surface-rules)** — needs `SliceRegionView::overhang_areas(&self) -> &[ExPolygon]` and `SliceRegionView::surface_group(&self) -> Option<&SurfaceGroup>` (P104-produced). Neither exists in `crates/slicer-sdk/src/views.rs` yet. This packet's AC-2 cannot run until P104 ships these accessors.
- Activation blockers:
  - P106 must be `status: implemented` before `overhang_quartile_polygons()` returns non-empty data.
  - P104 must be `status: implemented` before AC-2 (`overhang_areas()` non-empty test) can run.
- Unblocks:
  - **P108 (perimeter special modes + seam)** — T-077 (`extra_perimeters_on_overhangs`) becomes a real consumer.
  - Future packets that wire P104's `overhang_quartile = None` shipping code to consume the new view data.

## Acceptance Criteria

- **AC-1. Given** the new view accessor, **when** `crates/slicer-sdk/src/views.rs` is inspected, **then** `SliceRegionView` exposes `pub fn overhang_quartile_polygons(&self) -> &[QuartileBand]` (per-layer quartile bands pre-filtered to this region's polygon area), the WIT mirror declares `overhang-quartile-polygons: func() -> list<quartile-band>;` on `slice-region-view`, and the host populator fills the field from `SurfaceClassificationIR.overhang_quartile_polygons` at view-construction. | `rg -q 'pub fn overhang_quartile_polygons\(&self\) -> &\[QuartileBand\]' crates/slicer-sdk/src/views.rs && rg -q 'overhang-quartile-polygons: func\(\) -> list<quartile-band>' crates/slicer-schema/wit/deps/ir-types.wit`
- **AC-2. Given** P106 has shipped (populated `OverhangRegion.xy_footprint`), **when** an overhang-ramp fixture is sliced and `region.overhang_areas()` is called on a layer containing overhang facets, **then** the returned slice is non-empty and contains the projected XY footprint of overhang facets covering that region. (Confirms P104's stub accessor now returns real data without P104 source changes.) | `cargo test -p slicer-runtime --test contract slice_region_view_overhang_areas_non_empty_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-3. Given** the refactored `overhang-classifier-default`, **when** `modules/core-modules/overhang-classifier-default/src/lib.rs` is inspected, **then** it reads `Point3WithWidth.overhang_quartile` from `LayerCollectionView` entities (NOT from a local `LinesDistancer2D` computation), applies `EntityMutation::SetSpeedFactor` based on read quartiles, the module shrinks to ≤ 80 LOC, and the auxiliary files `classify.rs` and `lines_distancer.rs` are deleted from the module directory. | `! ls modules/core-modules/overhang-classifier-default/src/classify.rs 2>/dev/null && ! ls modules/core-modules/overhang-classifier-default/src/lines_distancer.rs 2>/dev/null && [ $(wc -l < modules/core-modules/overhang-classifier-default/src/lib.rs) -le 80 ]`
- **AC-4. Given** the refactored module's manifest, **when** `modules/core-modules/overhang-classifier-default/overhang-classifier-default.toml` is inspected, **then** the `[ir-access].reads` declaration no longer contains `"LayerCollectionIR"` as a broad entry and instead declares a narrower `overhang_quartile`-only read annotation; the manifest also drops write access to `LayerCollectionIR` OR retains it only for the `SetSpeedFactor` mutation path. **Tree-verified baseline:** pre-refactor manifest has `reads = ["LayerCollectionIR"]` (line confirmed in tree). Post-refactor must change this. | `rg -q 'overhang_quartile' modules/core-modules/overhang-classifier-default/overhang-classifier-default.toml && ! rg -q '^reads\s*=\s*\["LayerCollectionIR"\]' modules/core-modules/overhang-classifier-default/overhang-classifier-default.toml`
- **AC-5. Given** the end-to-end overhang-quartile propagation pipeline, **when** an overhang-ramp mesh is sliced with the full stack (P106 PrePass → P104 perimeter-side `overhang_quartile = None` ship → this packet's view accessor) **then** the resulting `LayerCollectionIR` carries wall vertices with `overhang_quartile = Some(N)` (where N ∈ {1, 2, 3, 4}) on at least one wall in the overhang region, AND `overhang-classifier-default` applies the corresponding speed factor (`overhang_1_4_speed`…`overhang_4_4_speed` config keys, real names from manifest) to that wall entity. **Note:** if P104's perimeter-side propagation is still shipping `None` at the time this packet runs, this AC documents the gap and registers a follow-up task instead of failing; AC body covers both completion modes. **FORWARD-DEP blocker:** this AC cannot run until P106 and P104 are `status: implemented`. | `cargo test -p slicer-runtime --test integration overhang_pipeline_e2e_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-6. Given** the pre-refactor `overhang-classifier-default` behaviour (recorded reference G-code SHA for the benchy / standard test fixtures), **when** the same fixtures slice with the post-refactor module, **then** the resulting G-code differs from the pre-refactor reference within calibrated tolerance (speed factors may shift in the 3rd–6th decimal due to the algorithm change; documented in closure log) and no regression in slice success rate. | `cargo test -p slicer-runtime --test integration overhang_classifier_refactor_regression_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-7. Given** the deviation closure pass, **when** `docs/specs/perimeter-modules-orca-parity-roadmap.md` is inspected post-packet, **then** entries `D-10` and `D-12` (which live only in the roadmap — **not** in `DEVIATION_LOG.md`) carry updated closure notes referencing this packet; a new `D-104-OVERHANG-QUARTILE-NONE` entry (ID-conformant with the `D-<pkt>-<SLUG>` convention) is added to `docs/DEVIATION_LOG.md` and marked closed; and `docs/specs/perimeter-modules-orca-parity-roadmap.md` marks T-024 and T-077 as unblocked (preconditions met). **Note:** D-10 and D-12 are roadmap decision entries, not DEVIATION_LOG.md rows — AC-7 must grep the roadmap for these, and add a new log entry for OVERHANG-QUARTILE-NONE. | `rg -q 'D-10.*(closed\|resolved\|P107)' docs/specs/perimeter-modules-orca-parity-roadmap.md && rg -q 'D-12.*(closed\|resolved\|P107)' docs/specs/perimeter-modules-orca-parity-roadmap.md && rg -q 'D-104-OVERHANG-QUARTILE-NONE.*(closed\|resolved)' docs/DEVIATION_LOG.md && rg -q '(T-024\|T-077).*unblocked' docs/specs/perimeter-modules-orca-parity-roadmap.md`

## Negative Test Cases

- **AC-N1. Given** a flat-top cube mesh (no overhang facets — same as P106's AC-N1), **when** the full stack runs, **then** `overhang-classifier-default` emits zero `SetSpeedFactor` mutations (no false overhang detection), and `LayerCollectionIR` wall vertices carry `overhang_quartile = None`. | `cargo test -p slicer-runtime --test integration overhang_pipeline_e2e_tdd::no_overhang_case -- --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-runtime --test contract slice_region_view_overhang_areas_non_empty_tdd && cargo test -p slicer-runtime --test integration overhang_pipeline_e2e_tdd overhang_classifier_refactor_regression_tdd`

## Authoritative Docs

- `docs/specs/overhang-pipeline-restructuring.md` — Phase 3/4/5 task rows.
- `docs/adr/0008-overhang-as-finalization-module.md` — read full; speed-factor application stays here per the preserved part of ADR-0008.
- `docs/adr/0022-overhang-classification-at-prepass.md` (from P106 — FORWARD-DEP; ADR slot 0022 is the next free slot after 0021; slot 0012 is taken by `0012-spatial-indexing-as-reconstruction-only-companions.md`) — read full once P106 ships; supersedes the "unnecessary scope" caveat only.
- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — D-10, D-12 entries to close; T-024 / T-077 unblock markers.
- `docs/05_module_sdk.md` — `SliceRegionView` + `PaintRegionLayerView` accessor convention.
- `docs/DEVIATION_LOG.md` — closure entries.

## Doc Impact Statement (Required)

- `docs/05_module_sdk.md` §"SliceRegionView accessors" — document the new `overhang_quartile_polygons()` accessor — `rg -q 'overhang_quartile_polygons.*QuartileBand' docs/05_module_sdk.md`
- `docs/DEVIATION_LOG.md` — register and close new entry `D-104-OVERHANG-QUARTILE-NONE` (ID-conformant; the deviation originated in P104) — `rg -q 'D-104-OVERHANG-QUARTILE-NONE.*(closed|resolved)' docs/DEVIATION_LOG.md`
- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — update D-10 and D-12 closure notes to reference P107; these IDs live only in the roadmap — `rg -q 'D-10.*(P107\|closed\|resolved)' docs/specs/perimeter-modules-orca-parity-roadmap.md && rg -q 'D-12.*(P107\|closed\|resolved)' docs/specs/perimeter-modules-orca-parity-roadmap.md`
- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — mark T-024 and T-077 as unblocked — `rg -q 'T-024.*(unblocked|preconditions met)' docs/specs/perimeter-modules-orca-parity-roadmap.md && rg -q 'T-077.*(unblocked|preconditions met)' docs/specs/perimeter-modules-orca-parity-roadmap.md`
- `docs/01_system_architecture.md` §"Tier 3 PostPass" — note `overhang-classifier-default` now reads quartile from IR (no longer computes from wall geometry) — `rg -q 'overhang-classifier-default.*reads.*overhang_quartile' docs/01_system_architecture.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked).

Files to inspect for this packet:

- None new beyond P106's references. The OrcaSlicer `detect_steep_overhang` algorithm was captured in P106's SUMMARY; this packet's classifier is the consumer side which is workspace-internal.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
