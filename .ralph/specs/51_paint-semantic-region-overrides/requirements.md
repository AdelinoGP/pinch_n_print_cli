# Requirements: 51_paint-semantic-region-overrides

## Packet Metadata

- Slug: `51_paint-semantic-region-overrides`
- Task IDs: `TASK-181`
- Backlog source: `docs/07_implementation_status.md`

## Problem Statement

DEV-045 (registered 2026-05-10, see `docs/DEVIATION_LOG.md`) records that `RegionMap` is paint-blind on the live host scheduler. Three coupled gaps make `PaintSemantic::Custom(...)` values useless beyond tool/material differentiation:

1. **Config namespace gap.** `crates/slicer-host/src/config_resolution.rs` (closed DEV-040 2026-05-04) recognises only `object_config:<id>:<key>` (line 84, 195). No `paint_config:<semantic>:<key>` namespace exists. Unknown keys silently fall into `cfg.extensions` (`:169-171`, `:280`) with no warning.

2. **IR shape gap.** `crates/slicer-ir/src/slice_ir.rs:1028-1033` declares `RegionPlan { config: ResolvedConfig, stage_modules: HashMap<StageId, Vec<ModuleInvocation>> }` — no paint-semantic dimension. `RegionKey` (`:1006-1015`) keys on `(global_layer_index, object_id, region_id)` only.

3. **Host built-in gap.** `crates/slicer-host/src/region_mapping.rs:103-248` contains zero "paint*"/"semantic" tokens. Configs are stamped per-object only (`:236-242`). `PaintRegionIR` is never read despite being available at this point in the pipeline (PaintSegmentation runs first per `docs/04_host_scheduler.md:461-509, :667`).

Consequence: a user passing `paint_config:fuzzy_skin:perimeter_count=5` cannot produce different GCode in fuzzy-skin-painted regions vs unpainted regions. The `fuzzy_skin` semantic crosses the IR via `PaintRegionLayerView` (`wit/deps/ir-types.wit:194-218`) to Layer modules, but each module must interpret it ad-hoc with no resolved-config plumbing — the "hand-tied config" anti-pattern the resolved-config layer was built to prevent. Per `docs/01_system_architecture.md:107-114`, RegionMap is responsible for "modules + pre-filtered config + active claims" per `(layer, object, region)`. This packet implements that responsibility for paint semantics.

**Crucial scope simplification:** Layer-tier core modules consume config via `ConfigView` (`crates/slicer-sdk/src/prelude.rs:43`, `traits.rs:158`), which the host stamps per-region. When the host overlays paint-semantic overrides into `RegionPlan.config` before dispatching the module, the module receives the correctly-overridden config naturally — no module-side change needed. This collapses what would otherwise be a 7-module change set into a 3-file host-side change.

## Task Mapping

- **TASK-181** (new — added to `docs/07_implementation_status.md` at Step 6):
  *"Make RegionMap paint-semantic-aware: add `paint_config:<semantic>:<key>` namespace, extend `RegionPlan` with `paint_overrides` (additive, minor schema bump), and make `region_mapping.rs` overlay per-semantic configs into `RegionPlan.config` via polygon overlap with `PaintRegionIR`. Covers DEV-045."*
  → Closes when AC-1 through AC-10 (positive) plus all three negative cases are green.

## In Scope

- `crates/slicer-host/src/config_resolution.rs`:
  - Recognize the `paint_config:<semantic>:<key>` key prefix in the raw config map.
  - New function `resolve_per_paint_semantic_configs(present_semantics: &[PaintSemantic]) -> BTreeMap<PaintSemantic, ResolvedConfig>` modelled on `resolve_per_object_configs` at `:186-216`.
  - Emit a structured warning (matching the existing paint-annotation warning surface) for `paint_config:UNKNOWN:` keys whose semantic is not in the model.
- `crates/slicer-ir/src/slice_ir.rs`:
  - `RegionPlan` gains `paint_overrides: BTreeMap<PaintSemantic, ResolvedConfig>` (additive).
  - `RegionMapIR.schema_version` bumped to 1.1.0 (minor, additive per `docs/02_ir_schemas.md` versioning rules).
- `crates/slicer-host/src/region_mapping.rs`:
  - Update `SemVer { major:1, minor:0, patch:0 }` at lines 201-206 → `minor:1`.
  - Read `PaintRegionIR` (host already has it post-PaintSegmentation).
  - For each `(layer, object, region_id)`:
    - Determine which paint semantics overlap that region's polygon (via `slicer_core::intersection` at `crates/slicer-core/src/polygon_ops.rs:98`).
    - Apply override precedence: global < per_object < per_paint_semantic. When multiple semantics overlap, resolve lexicographically by `PaintSemantic` string repr. This is a new RegionMap-stage rule introduced by Packet 51 and documented in `docs/02_ir_schemas.md` under the RegionMap section; it is distinct from the `paint_order`-based rule at `:436` which governs `PrePass::PaintSegmentation`.
    - Stamp the overlaid `ResolvedConfig` into `RegionPlan.config`.
    - Record the contributing semantics' configs in `RegionPlan.paint_overrides` for audit/test visibility.
- New tests:
  - `crates/slicer-host/tests/config_resolution_paint_semantic_tdd.rs` (positive + negative + warning).
  - `crates/slicer-host/tests/region_mapping_paint_semantic_tdd.rs` (overlap-applies / no-overlap-default / overlap-precedence-deterministic).
- `docs/01_system_architecture.md` — extend the RegionMapping bullet to declare paint-semantic awareness.
- `docs/02_ir_schemas.md` — document `RegionPlan.paint_overrides`; document `paint_config:<semantic>:<key>` namespace; document the override precedence rule; document the schema bump.
- `docs/07_implementation_status.md` — add + close TASK-181.
- `docs/DEVIATION_LOG.md` — flip DEV-045 to Closed.
- `docs/14_deviation_audit_history.md` — chronology entry.

## Out of Scope

- The seven extrusion-emitting Layer-tier core modules. Zero changes by design.
- `crates/slicer-sdk/` — no trait, ConfigView, or builder changes.
- `crates/slicer-host/src/paint_segmentation.rs`, `dispatch.rs`, `wit_host.rs`, `model_loader.rs` — no edits.
- Any change to WIT files under `wit/`.
- Any change to `crates/slicer-macros/src/lib.rs`.
- New paint semantics (this packet only adds the override mechanism for existing values).
- Cross-object paint overrides.
- Tool/material switching (already solved via `ActiveRegion.tool_index`).
- 3MF input ingestion (Packet 50's scope).

## Authoritative Docs

- `docs/01_system_architecture.md:107-114` — RegionMapping responsibility (edited).
- `docs/02_ir_schemas.md:103-122` PaintSemantic; `:306-364` ResolvedConfig; `:436` overlap precedence; `:451-480` RegionPlan (edited).
- `docs/04_host_scheduler.md:461-509` RegionMap host built-in; `:667` PaintSegmentation-before-RegionMapping ordering.
- `docs/07_implementation_status.md` — delegate ALL reads/edits.
- `docs/DEVIATION_LOG.md` — delegate SNIPPET fetch.
- `docs/14_deviation_audit_history.md` — delegate SNIPPET fetch.

## OrcaSlicer Reference Obligations

- None. This is host-scheduler + IR-shape work, not a geometric-algorithm port. Polygon overlap uses the existing Clipper2-backed `slicer_core::intersection` (public re-export from `crates/slicer-core/src/polygon_ops.rs:98`).

## Acceptance Summary

The packet is complete when:

1. `paint_config:<semantic>:<key>` namespace is recognized by `config_resolution.rs`; `resolve_per_paint_semantic_configs` exists and behaves like `resolve_per_object_configs`.
2. `RegionPlan` carries an additive `paint_overrides` field; `RegionMapIR.schema_version` bumped to 1.1.0.
3. `region_mapping.rs` reads `PaintRegionIR` and stamps overlay-resolved configs into `RegionPlan.config` and per-semantic audit data into `RegionPlan.paint_overrides`.
4. The pre-committed failing E2E test `paint_config_override_visibly_differs_gcode` (RED 2026-05-10) goes GREEN (gated on Packet 50 closure for the painted fixture).
5. Backward-compat: `benchy_e2e_real_pipeline_produces_gcode` stays GREEN; Packet 50's tests stay GREEN; the five Packet-43-rev1 regression-defense commands stay GREEN.
6. Three negative tests pass: unknown semantic warns then ignores; overlap precedence is deterministic; no-overlap region preserves object-only config.
7. `docs/01` and `docs/02` document the new mechanism; schema bump recorded.
8. `cargo clippy --workspace -- -D warnings` green.
9. DEV-045 flipped to Closed; TASK-181 closed; chronology entry added.

## Cross-Packet Dependencies

- **Depends on:** Packet 50 (`paint-input-3mf-ingestion`, DEV-044). End-to-end test (AC-4) is gated on the painted-Benchy fixture. Steps 1-3 (config_resolution, IR shape, region_mapping overlap) can proceed in parallel using synthetic in-memory `paint_data`.
- **Unblocks:** Future packets that add new paint semantics (e.g. `Custom("ironing")`) — they inherit a working override mechanism.

## Verification Commands

Targeted verification (use these for per-step adjudication):

- `cargo build --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo test -p slicer-host --test config_resolution_paint_semantic_tdd`
- `cargo test -p slicer-host --test region_mapping_paint_semantic_tdd`
- `cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd`
- `cargo test -p slicer-host --test benchy_painted_e2e_tdd` (Packet 50 regression-defense)
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_e2e_real_pipeline_produces_gcode -- --exact`
- Packet-43-rev1 regression battery (five commands; see packet.spec.md AC-7).

`cargo test --workspace` is **not** required at packet close.

## Step Completion Expectations

Each implementation step in `implementation-plan.md` declares files-allowed-to-read, files-allowed-to-edit (≤ 3), expected sub-agent dispatches, context cost (S/M; never L), and a falsifying check or exit condition.

## Context Discipline Notes

- Read budget: 60% (≈ 120k). Stop reading at 60%, hand off at 85%.
- `crates/slicer-macros/src/lib.rs`, `crates/slicer-sdk/`, and all Layer-tier core-module crates are out of bounds for direct reading.
- `docs/07_implementation_status.md` and `docs/DEVIATION_LOG.md` are large; delegate all reads.
- Polygon-overlap computation in region_mapping.rs uses existing `slicer_helpers` APIs; do not reimplement Clipper2 plumbing.
