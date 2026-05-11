# Design: 51_paint-semantic-region-overrides

## Implementation Shape

Three coordinated host-side edits, ZERO module changes:

1. **`config_resolution.rs` namespace extension.** Add `paint_config:<semantic>:<key>` prefix recognition. New function `resolve_per_paint_semantic_configs(&[PaintSemantic]) -> BTreeMap<PaintSemantic, ResolvedConfig>` modelled on `resolve_per_object_configs` (`:186-216`). Unknown semantics emit a warning, not a fatal error.

2. **`slice_ir.rs::RegionPlan` additive field.** `paint_overrides: BTreeMap<PaintSemantic, ResolvedConfig>` added (additive → minor schema bump 1.0.0 → 1.1.0 per `docs/02_ir_schemas.md` versioning rules). The existing `config: ResolvedConfig` field continues to be the final stamped config that modules see; `paint_overrides` is the audit trail of which semantics contributed.

3. **`region_mapping.rs` paint-aware overlay.** Read `PaintRegionIR` (already produced by `PrePass::PaintSegmentation` per `docs/04_host_scheduler.md:461-509, :667`). For each `(layer, object, region_id)`, compute polygon overlap with each paint semantic via `slicer_core::intersection` (public symbol at `crates/slicer-core/src/polygon_ops.rs:98`). Apply precedence (global < per_object < per_paint_semantic, lexicographic tiebreak between semantics). Stamp the overlay into `RegionPlan.config`. Populate `RegionPlan.paint_overrides`.

The seven extrusion-emitting Layer-tier core modules (`arachne-perimeters`, `classic-perimeters`, `rectilinear-infill`, `gyroid-infill`, `lightning-infill`, `top-surface-ironing`, `traditional-support`/`tree-support`/`support-surface-ironing`, `fuzzy-skin`) are unchanged. They consume `ConfigView` derived from `RegionPlan.config`; the override is invisible to them and naturally applied.

Total churn estimate: ~ 300 LOC across the 3 host source files + 2 new test files + 3 doc edits.

## Controlling Code Paths and Surfaces

- **Primary edit surface 1: `crates/slicer-host/src/config_resolution.rs`**
  - Current parser at `:84, :195` recognizes `object_config:<id>:<key>`. Add a sibling parse path for `paint_config:<semantic>:<key>`.
  - New function: `pub fn resolve_per_paint_semantic_configs(raw_config: &Map<String, Value>, present_semantics: &[PaintSemantic]) -> (BTreeMap<PaintSemantic, ResolvedConfig>, Vec<UnknownSemanticWarning>)` modelled on `resolve_per_object_configs` (`:186-216`). The tuple second element carries unknown-semantic warnings for the host to forward to its progress-event sink.
  - Unknown-key fall-through to `cfg.extensions` (`:169-171, :280`) is preserved for non-`paint_config:` unknown keys; for known-prefix-but-unknown-semantic the warning surface is the new path.

- **Primary edit surface 2: `crates/slicer-ir/src/slice_ir.rs`**
  - `RegionPlan` struct at `:1028-1033`: add `paint_overrides: BTreeMap<PaintSemantic, ResolvedConfig>` field. Default `BTreeMap::new()` (empty) when no semantics apply.
  - Derive deterministic serialization order (the existing struct already does — `BTreeMap` is naturally ordered).
  - `RegionMapIR.schema_version` is constructed inline at `crates/slicer-host/src/region_mapping.rs:201-206` as `SemVer { major:1, minor:0, patch:0 }`. Bump to `minor:1`.

- **Primary edit surface 3: `crates/slicer-host/src/region_mapping.rs`**
  - `execute_region_mapping` (or whichever function builds `RegionMapIR` — the implementer locates via Step 1 grounding) currently does not consult `PaintRegionIR`. Add: accept `paint_regions: &PaintRegionIR` (or read from the host blackboard if available — Step 1 grounds the API).
  - For each region built, before stamping `RegionPlan`, compute overlap. `PaintRegionIR` exposes `pub fn get(&self, layer_index: u32, semantic: &PaintSemantic) -> &[SemanticRegion]` (`crates/slicer-ir/src/slice_ir.rs:951-961`), so iterate the known semantics (those present in `per_layer.get(layer).semantic_regions.keys()`) and call `get(layer, &semantic)`:
    ```
    let layer_map = paint_regions.per_layer.get(&global_layer_index);
    let overlapping_semantics: Vec<PaintSemantic> = match layer_map {
        None => Vec::new(),
        Some(layer_map) => layer_map.semantic_regions.keys()
            .filter(|semantic| {
                let semantic_regions = paint_regions.get(global_layer_index, semantic);
                semantic_regions.iter().any(|sr| {
                    !slicer_core::intersection(&region_expolys, &sr.polygons).is_empty()
                })
            })
            .cloned()
            .collect(),
    };
    ```
  - Resolve override:
    ```
    let semantic_configs = resolve_per_paint_semantic_configs(...);
    let mut effective_config = per_object_config.clone();
    for semantic in overlapping_semantics.iter().sorted() {  // lex order
        if let Some(override_cfg) = semantic_configs.get(semantic) {
            effective_config.overlay_mut(override_cfg);  // existing pattern from per-object overlay
        }
    }
    RegionPlan { config: effective_config, paint_overrides: <map of contributing>, stage_modules }
    ```

## Neighboring Tests and Fixtures

- **Failing E2E target (already RED, must turn GREEN at packet close; gated on Packet 50 closure):**
  - `crates/slicer-host/tests/benchy_painted_overrides_e2e_tdd.rs::paint_config_override_visibly_differs_gcode`
- **New tests to author (this packet):**
  - `crates/slicer-host/tests/config_resolution_paint_semantic_tdd.rs`:
    - `resolves_paint_config_namespace` (positive)
    - `unknown_semantic_warns_then_ignores` (negative)
  - `crates/slicer-host/tests/region_mapping_paint_semantic_tdd.rs`:
    - `region_overlap_applies_override` (positive)
    - `no_overlap_keeps_object_config` (negative — no-overlap default)
    - `overlap_precedence_is_deterministic` (negative — multi-semantic deterministic tiebreak)
- **Regression-defense targets (must stay green):**
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs::benchy_e2e_real_pipeline_produces_gcode`
  - `crates/slicer-host/tests/benchy_painted_e2e_tdd.rs` (Packet 50 tests)
  - `crates/slicer-host/tests/macro_paint_segmentation_output_roundtrip_tdd.rs`
  - `crates/slicer-host/tests/macro_mesh_segmentation_output_roundtrip_tdd.rs`
  - `crates/slicer-host/tests/dispatch_tdd.rs` macro_path tier
  - `crates/slicer-host/tests/macro_all_worlds_roundtrip_tdd.rs` prepass tier
  - `crates/slicer-host/tests/guest_fixture_freshness_tdd.rs`

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

## Selected Approach

**Path A: Host-side overlay in `region_mapping.rs`; `RegionPlan` carries audit-trail `paint_overrides` field.**

- Override resolution happens at PrePass::RegionMapping time, not at module-dispatch time.
- The effective `ResolvedConfig` is computed once per region; cached in `RegionPlan.config`; passed to modules unchanged.
- The `paint_overrides` field exists for audit/test visibility — region-mapping tests can verify which semantics contributed without parsing GCode.

### Rejected Alternatives

- **Path B: Defer override resolution to module-dispatch time (per-stage).** Rejected because (a) duplicates work across stages on the same region, (b) makes the override behavior invisible at the IR layer, (c) every module would need an awareness of the override mechanism. Path A is strictly better.
- **Path C: No `paint_overrides` audit field; just stamp the effective `config` and discard the trail.** Rejected because audit/test visibility is load-bearing for the negative tests. Without the field, `overlap_precedence_is_deterministic` cannot verify which semantics actually contributed without parsing GCode.
- **Path D: Modify all seven Layer-tier modules to accept a `paint_overrides` parameter on their stage functions.** Rejected — massive scope blowup with zero benefit because (a) modules already get the resolved `ConfigView`, (b) WIT shape changes are out of scope, (c) the project explicitly designed `ConfigView` to abstract config resolution away from modules.
- **Path E: Patch `ResolvedConfig` post-hoc inside `dispatch.rs` per-stage.** Rejected for the same reasons as Path B + adds a per-module dispatch overhead and a second mutation site for `ResolvedConfig`.

## Code Change Surface (authoritative files-in-scope list)

Primary editing surfaces:

1. `crates/slicer-host/src/config_resolution.rs` (extend prefix parser; add `resolve_per_paint_semantic_configs`).
2. `crates/slicer-ir/src/slice_ir.rs` (add `paint_overrides` field to `RegionPlan`; export `BTreeMap` import if not already).
3. `crates/slicer-host/src/region_mapping.rs` (read `PaintRegionIR`; overlap loop; overlay; schema-version bump).
4. `crates/slicer-host/tests/config_resolution_paint_semantic_tdd.rs` (new test file).
5. `crates/slicer-host/tests/region_mapping_paint_semantic_tdd.rs` (new test file).
6. `docs/01_system_architecture.md` (RegionMapping bullet update).
7. `docs/02_ir_schemas.md` (paint_config namespace; schema bump; precedence rules; paint_overrides field).
8. `docs/07_implementation_status.md` (add + close TASK-181 — via worker dispatch).
9. `docs/DEVIATION_LOG.md` (flip DEV-045 — via worker dispatch).
10. `docs/14_deviation_audit_history.md` (chronology entry — via worker dispatch).

No step opens more than 3 of these files at once.

## Read-Only Context the Implementer Needs

- `crates/slicer-host/src/config_resolution.rs` — full file (expected ≤ 350 lines; read directly with focus on `:77-216`).
- `crates/slicer-host/src/region_mapping.rs` — full file (expected ≤ 250 lines; read directly).
- `crates/slicer-ir/src/slice_ir.rs:1006-1080` — only the RegionKey/RegionPlan/RegionMapIR section (≤ 80 lines).
- `crates/slicer-ir/src/slice_ir.rs:172-184` — only the PaintSemantic section (≤ 20 lines).
- `crates/slicer-host/src/paint_segmentation.rs:70-130` — only the harvest path producing `PaintRegionIR` (read to understand the shape consumed).
- `crates/slicer-core/src/polygon_ops.rs` — only the `intersection` function signature at `:98` (public symbol re-exported as `slicer_core::intersection`; `slicer-helpers` does NOT expose this function).
- `crates/slicer-host/tests/benchy_painted_overrides_e2e_tdd.rs` — full file (≤ 200 lines); the AC contract.

## Out-of-Bounds Files (forbidden direct reads)

- `crates/slicer-macros/src/lib.rs` — out of scope, > 2 300 lines, no edit.
- `crates/slicer-sdk/` — out of scope (no trait/builder/ConfigView changes).
- All `modules/core-modules/*` Layer-tier crates — out of scope (zero module changes).
- `crates/slicer-host/src/paint_segmentation.rs` outside `:70-130` — read-only context.
- `crates/slicer-host/src/dispatch.rs`, `wit_host.rs`, `model_loader.rs` — out of scope.
- `OrcaSlicerDocumented/` — no parity obligation; do not read.
- `wit/` and inline-WIT blocks — no WIT changes.
- `target/` — generated artifacts.
- Other `.ralph/specs/` packet directories (the cross-packet mutation rule).

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

## Open Questions

These must be resolved before activation (status: draft → active):

- **Q1**: confirm schema-version bump from 1.0.0 → 1.1.0 for `RegionMapIR`. Recommend yes (minor, additive per `docs/02_ir_schemas.md` versioning rules).
- **Q2**: confirm override precedence: `global < per_object < per_paint_semantic`. Recommend yes.
- **Q3**: confirm unknown-semantic emits warning vs silent drop. Recommend warn. Which progress-event code? (Reuse paint-annotation warning surface; code TBD by Step 1 grounding of the existing paint-annotation event family.)
- **Q4**: confirm multi-semantic overlap precedence: lexicographic by `PaintSemantic` string repr, later overlay wins. This is a **new** RegionMap-stage rule (not the `:436` rule, which is `paint_order`-based and applies to PaintSegmentation). Step 6 commits to adding the new rule to `docs/02_ir_schemas.md` under the RegionMap section. Recommend yes.

## Locked Assumptions and Invariants

The implementation must preserve these invariants. If any one is violated, the change is rejected.

1. `crates/slicer-macros/src/lib.rs`, `crates/slicer-sdk/`, all `modules/core-modules/*` Layer-tier crates, `crates/slicer-host/src/paint_segmentation.rs`, `dispatch.rs`, `wit_host.rs`, `model_loader.rs`, and all `wit/` files are unchanged after this packet.
2. `RegionPlan.paint_overrides` is the ONLY new field on `RegionPlan`; no existing field is removed or renamed.
3. `RegionMapIR.schema_version` bumps to 1.1.0 minor; no other version bump.
4. A region with no overlapping paint semantics produces a `RegionPlan` whose `config` is byte-identical to the pre-packet output for the same input.
5. The pre-committed failing test at `benchy_painted_overrides_e2e_tdd.rs::paint_config_override_visibly_differs_gcode` (RED 2026-05-10) turns GREEN WITHOUT weakening its assertions. The assertion text MUST NOT be edited in this packet.
6. No existing passing test is weakened (no assertion removed; no `#[ignore]` added).
7. Test discipline: targeted `cargo test -p <crate> --test <file>` only; never `cargo test --workspace`.
8. The unknown-semantic warning path NEVER fails the slice — only emits a warning event.
