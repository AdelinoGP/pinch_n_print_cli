---
status: implemented
packet: 51_paint-semantic-region-overrides
task_ids:
  - TASK-181
---

# 51_paint-semantic-region-overrides

## Goal

Close DEV-045 by making `RegionMap` paint-semantic-aware. Today the host built-in `crates/slicer-host/src/region_mapping.rs:103-248` contains zero "paint*"/"semantic" tokens, `RegionPlan` (`crates/slicer-ir/src/slice_ir.rs:1028-1033`) has no paint-semantic dimension, and `crates/slicer-host/src/config_resolution.rs` recognises only `object_config:<id>:<key>` (line 84, 195). A user config containing `paint_config:fuzzy_skin:perimeter_count=5` is silently dropped into `cfg.extensions` (`:169-171`, `:280`) with no diagnostic. Consequently `PaintSemantic::Custom("fuzzy_skin")` crosses IR via `PaintRegionLayerView` but cannot bind to per-region `ResolvedConfig` overrides on the live host scheduler — violating the RegionMap responsibility stated at `docs/01_system_architecture.md:107-114` ("decides modules + pre-filtered config + active claims per (layer, object, region)").

This packet wires three additive surfaces: (1) a new `paint_config:<semantic>:<key>` namespace in `config_resolution.rs` with a `resolve_per_paint_semantic_configs` function modelled on the existing `resolve_per_object_configs` (`:186-216`); (2) an additive `paint_overrides: BTreeMap<PaintSemantic, ResolvedConfig>` field on `RegionPlan` (minor schema bump 1.0.0 → 1.1.0); (3) `region_mapping.rs` learns to read `PaintRegionIR` (already available because `PrePass::PaintSegmentation` runs before `PrePass::RegionMapping` per `docs/04_host_scheduler.md:461-509`), compute per-region overlaps with each paint semantic, and stamp the effective overlay (per-object → per-paint-semantic, in that order) into `RegionPlan.config` while preserving the audit trail in `paint_overrides`.

**Crucial scope simplification:** the seven extrusion-emitting Layer-tier core modules (`arachne-perimeters`, `classic-perimeters`, `rectilinear-infill`, `gyroid-infill`, `lightning-infill`, `top-surface-ironing`, `traditional-support`/`tree-support`/`support-surface-ironing`, `fuzzy-skin`) need **zero changes**. They already read config via `ConfigView` (`crates/slicer-sdk/src/prelude.rs:43`, `traits.rs:158`); when the host passes a region's `RegionPlan.config` that already incorporates the paint-semantic overlay, the modules naturally honor it.

The failing TDD-RED test already committed at `crates/slicer-host/tests/benchy_painted_overrides_e2e_tdd.rs::paint_config_override_visibly_differs_gcode` (2026-05-10) goes GREEN at packet close.

## Problem Statement

DEV-045 (registered 2026-05-10, see `docs/DEVIATION_LOG.md`) records that `RegionMap` is paint-blind on the live host scheduler. Three coupled gaps make `PaintSemantic::Custom(...)` values useless beyond tool/material differentiation:

1. **Config namespace gap.** `crates/slicer-host/src/config_resolution.rs` (closed DEV-040 2026-05-04) recognises only `object_config:<id>:<key>` (line 84, 195). No `paint_config:<semantic>:<key>` namespace exists. Unknown keys silently fall into `cfg.extensions` (`:169-171`, `:280`) with no warning.

2. **IR shape gap.** `crates/slicer-ir/src/slice_ir.rs:1028-1033` declares `RegionPlan { config: ResolvedConfig, stage_modules: HashMap<StageId, Vec<ModuleInvocation>> }` — no paint-semantic dimension. `RegionKey` (`:1006-1015`) keys on `(global_layer_index, object_id, region_id)` only.

3. **Host built-in gap.** `crates/slicer-host/src/region_mapping.rs:103-248` contains zero "paint*"/"semantic" tokens. Configs are stamped per-object only (`:236-242`). `PaintRegionIR` is never read despite being available at this point in the pipeline (PaintSegmentation runs first per `docs/04_host_scheduler.md:461-509, :667`).

Consequence: a user passing `paint_config:fuzzy_skin:perimeter_count=5` cannot produce different GCode in fuzzy-skin-painted regions vs unpainted regions. The `fuzzy_skin` semantic crosses the IR via `PaintRegionLayerView` (`wit/deps/ir-types.wit:194-218`) to Layer modules, but each module must interpret it ad-hoc with no resolved-config plumbing — the "hand-tied config" anti-pattern the resolved-config layer was built to prevent. Per `docs/01_system_architecture.md:107-114`, RegionMap is responsible for "modules + pre-filtered config + active claims" per `(layer, object, region)`. This packet implements that responsibility for paint semantics.

**Crucial scope simplification:** Layer-tier core modules consume config via `ConfigView` (`crates/slicer-sdk/src/prelude.rs:43`, `traits.rs:158`), which the host stamps per-region. When the host overlays paint-semantic overrides into `RegionPlan.config` before dispatching the module, the module receives the correctly-overridden config naturally — no module-side change needed. This collapses what would otherwise be a 7-module change set into a 3-file host-side change.

## Architecture Constraints (Locked Assumptions)

1. **No Layer-module changes.** Override application happens entirely host-side via `RegionPlan.config` overlay. Modules see a `ConfigView` derived from the already-overlaid config.
2. **No SDK changes.** `crates/slicer-sdk/` is read-only in this packet.
3. **No WIT changes.** All paint data already crosses the WIT boundary via `PaintRegionLayerView` (Packet 43-rev1).
4. **No PaintSegmentation/dispatch changes.** PaintSegmentation produces `PaintRegionIR`; this packet only consumes it downstream in RegionMapping.
5. **Additive IR change only.** `RegionPlan.paint_overrides` is additive; `RegionMapIR.schema_version` bumps 1.0.0 → 1.1.0 per `docs/02_ir_schemas.md` minor-bump rule.
6. **Override precedence: global < per_object < per_paint_semantic.** Per-paint-semantic always wins over per-object. Documented in `docs/02_ir_schemas.md`.
7. **Multi-semantic overlap: deterministic lexicographic precedence.** When two semantics overlap a region, sort by `PaintSemantic` string representation; later semantics overlay later (so the lexicographically-LATER semantic wins). This is a **new** RegionMap-stage rule introduced by Packet 51 and documented in `docs/02_ir_schemas.md` under the RegionMap section (Step 6). It is distinct from `:436`, which is a `paint_order`-based rule governing `PrePass::PaintSegmentation`'s resolution of overlapping `Custom` paint values into a single `PaintRegionIR`.
8. **Unknown-semantic handling: warn but don't fail.** A `paint_config:UNKNOWN:key` produces a structured warning and is dropped. The slice succeeds.
9. **No-overlap regions are byte-identical pre/post packet.** A region with no overlapping paint semantics must produce a `RegionPlan` whose `config` field is byte-identical to the pre-packet `region_mapping.rs` output for the same input. This is the load-bearing backward-compat guarantee.
10. **The pre-committed failing test at `benchy_painted_overrides_e2e_tdd.rs::paint_config_override_visibly_differs_gcode` (RED 2026-05-10) must turn GREEN at packet close WITHOUT weakening its assertions.**

## Data and Contract Notes

- `PaintSemantic` values per `docs/02_ir_schemas.md:103-122`: `Custom(String)` is the only variant carrying user-defined semantics. Built-in variants are tool-index-aligned and not the target of this packet. The `paint_config:` namespace serializes `PaintSemantic::Custom("fuzzy_skin")` as `paint_config:fuzzy_skin:<key>` (the string repr, no prefix).
- `RegionPlan.config` is the final effective config after overlay. `RegionPlan.paint_overrides` is the per-semantic subset that contributed.
- Override precedence: global → per_object → per_paint_semantic (later overlay wins). Within paint semantics overlapping the same region, sort lex by `PaintSemantic` string repr; later semantics in sort order overlay later.
- Polygon overlap: use `slicer_core::intersection` (public re-export from `crates/slicer-core/src/polygon_ops.rs:98`; signature `intersection(subject: &[ExPolygon], clip: &[ExPolygon]) -> Vec<ExPolygon>`). Any non-empty intersection counts as overlap (even a single point); this matches existing region-overlap semantics in `region_mapping.rs`.
- The host already has `PaintRegionIR` at this point in the scheduler — confirmed by `docs/04_host_scheduler.md:461-509, :667`. The implementer must locate the exact field/parameter where it's available to `region_mapping.rs::execute_region_mapping` (Step 1 grounding).

## Risks and Tradeoffs

- **Risk: `PaintRegionIR` availability inside region_mapping.rs.** The doc says PaintSegmentation runs first, but the current `execute_region_mapping` function signature may not have access to `PaintRegionIR`. Step 1 must confirm; if the signature requires extension, the change is bounded to that function plus its caller in `prepass.rs` or similar. NOT an out-of-scope expansion; this is a host-internal plumbing change.
- **Risk: polygon overlap computation cost.** For models with many paint regions and many slice regions per layer, the N*M overlap loop can be expensive. Mitigation: bail early once any overlap is found (we just need to know *which* semantics overlap, not the intersection polygon); index paint regions by bounding box if hot. Initial implementation can be naive; optimize if benchmarks show pain.
- **Tradeoff: schema_version bump.** Minor bump is correct per the additive-field rule, but consumers reading old `RegionMapIR` snapshots will see `paint_overrides: BTreeMap::new()` (default) — no breakage, but tests/fixtures that hash the full `RegionPlan` value need re-blessing. Step 1 inventories these.
- **Tradeoff: deterministic precedence by lex order.** Some users may expect a "first paint wins" or "last paint wins" semantic. Lex order is the simplest deterministic rule and matches the spirit of `docs/02_ir_schemas.md:436`. Documented explicitly so future packets don't second-guess.

## Locked Assumptions and Invariants

The implementation must preserve these invariants. If any one is violated, the change is rejected.

1. `crates/slicer-macros/src/lib.rs`, `crates/slicer-sdk/`, all `modules/core-modules/*` Layer-tier crates, `crates/slicer-host/src/paint_segmentation.rs`, `wit_host.rs`, `model_loader.rs`, and all `wit/` files are unchanged after this packet. Note: `dispatch.rs` and `prepass.rs` WERE edited as structural necessities — the original design's claim that "the host passes a region's `RegionPlan.config` that already incorporates the paint-semantic overlay" and "modules naturally honor it" required explicit wiring in `dispatch_layer_call` (ConfigView was frozen at module-bind time, not sourced per-region) and a timing fix in `prepass.rs` (paint_semantic_configs computed before paint regions were available). These are bounded host-internal plumbing changes consistent with the no-module-changes intent of this assumption; see Implementation Notes below.
2. `RegionPlan.paint_overrides` is the ONLY new field on `RegionPlan`; no existing field is removed or renamed.
3. `RegionMapIR.schema_version` bumps to 1.1.0 minor; no other version bump.
4. A region with no overlapping paint semantics produces a `RegionPlan` whose `config` is byte-identical to the pre-packet output for the same input.
5. The pre-committed failing test at `benchy_painted_overrides_e2e_tdd.rs::paint_config_override_visibly_differs_gcode` (RED 2026-05-10) turns GREEN WITHOUT weakening its assertions. The assertion text MUST NOT be edited in this packet.
6. No existing passing test is weakened (no assertion removed; no `#[ignore]` added).
7. Test discipline: targeted `cargo test -p <crate> --test <file>` only; never `cargo test --workspace`.
8. The unknown-semantic warning path NEVER fails the slice — only emits a warning event.
