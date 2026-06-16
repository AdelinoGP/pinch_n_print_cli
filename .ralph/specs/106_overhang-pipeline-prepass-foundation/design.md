# Design: 106_overhang-pipeline-prepass-foundation

## Controlling Code Paths

- Primary code path: `mesh_analysis.rs::OverhangRegion` construction site is extended to populate `xy_footprint` by reusing the existing facet-cluster pattern (same shape as `BridgeRegion.xy_footprint` at line ~616 per packet metadata). `support_geometry.rs`'s plane-triangle intersection helper is extracted to `mesh_cross_section.rs` and consumed by both `support_geometry` (existing) and `overhang_annotation` (new). `overhang_annotation.rs` computes per-layer quartile polygons: for each consecutive layer pair `(layer_n, layer_{n-1})`, compute cross-sections via the promoted helper, derive distance-from-previous-slice field, partition into 4 quartile bands by `line_width × {0.5, 1.0, 1.5, 2.0}` thresholds. The stage runs after `MeshAnalysis` + `LayerPlanning` and writes to the Blackboard's `SurfaceClassificationIR.overhang_quartile_polygons` HashMap.
- Neighboring tests / fixtures: 4 new TDD files. Existing `prepass_support_geometry_tdd` + `mesh_analysis_tdd` regression tests must stay green.
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations (delegate; never load).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- ADR-0008 invariant preserved: speed-factor application stays a `PostPass::LayerFinalization` concern. This packet does NOT touch `overhang-classifier-default` (P107 work).
- ADR-0012 invariant introduced: classification uses mesh cross-sections, not wall geometry. The classifier reads only `MeshIR` + `LayerPlanIR` — no per-layer slice data, no Tier 2 per-region data.
- Schema-version contract: additive bump (likely 4.3.0 → 4.4.0; coordinate with P105 if it lands first). `#[serde(default)]` on the new fields preserves old fixtures.
- WIT type identity: `OverhangRegion` record gains `xy-footprint` field; `SurfaceClassificationIR` gains `overhang-quartile-polygons` accessor (host-only — no guest reads of this top-level IR; guests get the per-region projection via P107's view accessors).
- Stage ordering invariant: `PrePass::OverhangAnnotation` MUST execute after both `MeshAnalysis` and `LayerPlanning` commit. The scheduler validates this; AC-N2 covers the violation case.
- The classifier is deterministic: same `MeshIR` + same `LayerPlanIR` + same threshold config → same `overhang_quartile_polygons`.

## Code Change Surface

- Selected approach: extend the existing `MeshAnalysis` construction path for `xy_footprint` (one-line addition mirroring the bridge pattern); promote the plane-triangle intersection to a shared module so both `SupportGeometry` and `OverhangAnnotation` consume it (avoids duplication and keeps both in lock-step on coordinate conventions); implement the classifier as a standalone pure function in a new file (no host-services dependency, deterministic); declare the stage in the scheduler with explicit precondition declarations on MeshAnalysis + LayerPlanning so the validator catches ordering violations.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `docs/adr/0012-overhang-classification-at-prepass.md` (NEW).
  - `docs/specs/overhang-pipeline-restructuring.md` — close O-1..O-8 entries.
  - `crates/slicer-ir/src/slice_ir.rs` — `OverhangRegion.xy_footprint`, `QuartileBand`, `SurfaceClassificationIR.overhang_quartile_polygons`, schema bump.
  - `crates/slicer-schema/wit/deps/ir-types.wit` — WIT mirrors.
  - `crates/slicer-core/src/algos/mesh_cross_section.rs` (NEW) — promoted helper.
  - `crates/slicer-core/src/algos/support_geometry.rs` — consume promoted helper.
  - `crates/slicer-core/src/algos/mesh_analysis.rs` — populate `xy_footprint`.
  - `crates/slicer-core/src/algos/overhang_annotation.rs` (NEW) — classifier.
  - `crates/slicer-core/src/algos/mod.rs` — `pub mod` declarations.
  - `crates/slicer-scheduler/src/execution_plan.rs` (or analogous) — stage declaration.
  - `crates/slicer-runtime/src/prepass.rs` (or analogous) — stage runner.
  - `crates/slicer-runtime/src/builtins/` — possibly a new `overhang_annotation_producer.rs` mirroring `region_mapping_producer.rs` pattern (verify in implementation).
  - 4 new TDD files.
  - `docs/01_system_architecture.md`, `docs/02_ir_schemas.md` per Doc Impact Statement.
- Rejected alternatives that were considered and why they were not chosen:
  - Distance field instead of polygon partition: rejected per O-7 default — matches existing IR style (polygons, not fields); cheaper for guest consumers (point-in-polygon vs distance-field sampling).
  - Independent mesh cross-section implementation per stage: rejected per O-3 default — duplication; risks drift between SupportGeometry and OverhangAnnotation conventions.
  - Keep `xy_footprint` computation inside `OverhangAnnotation` rather than at `MeshAnalysis`: rejected — `MeshAnalysis` already constructs `OverhangRegion`; computing the XY footprint at the same site is one-line and matches the `BridgeRegion.xy_footprint` pattern.

## Files in Scope (read + edit)

Primary edit surface exceeds 3 files because the packet covers 9 tasks spanning IR + 2 new modules + stage wiring. The **three highest-LOC-delta** files are listed first; the rest are justified.

- `crates/slicer-core/src/algos/overhang_annotation.rs` (NEW) — classifier; expected change: ~200 LOC.
- `crates/slicer-core/src/algos/mesh_cross_section.rs` (NEW) — promoted helper; expected change: ~120 LOC moved from `support_geometry.rs`.
- `crates/slicer-ir/src/slice_ir.rs` — IR additions; expected change: ~30 LOC.
- `crates/slicer-core/src/algos/mesh_analysis.rs` — `xy_footprint` populate (~10 LOC).
- `crates/slicer-core/src/algos/support_geometry.rs` — consume promoted helper (~20 LOC delta — delete inline impl, replace with call).
- `crates/slicer-core/src/algos/mod.rs` — pub mod declarations (~3 LOC).
- `crates/slicer-schema/wit/deps/ir-types.wit` — ~10 LOC.
- `crates/slicer-scheduler/src/execution_plan.rs` (or analogous) — ~15 LOC for stage declaration.
- `crates/slicer-runtime/src/prepass.rs` (or analogous) + possibly a new builtin producer file — ~30 LOC.
- 4 new TDD files.
- `docs/adr/0012-overhang-classification-at-prepass.md` (NEW), `docs/specs/overhang-pipeline-restructuring.md` (close O-decisions), `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`.

## Read-Only Context

- `docs/adr/0008-overhang-as-finalization-module.md` — read full — purpose: align ADR-0012's supersession language; preserve the speed-factor decision.
- `docs/specs/overhang-pipeline-restructuring.md` — read full — purpose: scope confirmation per phase + default-if-unanswered for O-1..O-8.
- `docs/01_system_architecture.md` — range-read §"Tier 1 — PrePass" — purpose: match existing stage-block format when adding `PrePass::OverhangAnnotation`.
- `docs/02_ir_schemas.md` — delegate SUMMARY for `SurfaceClassificationIR`, `OverhangRegion`, `BridgeRegion`.
- `CLAUDE.md` — §"Guest WASM Staleness" + §"WIT/Type Changes Checklist".
- `crates/slicer-core/src/algos/support_geometry.rs` — range-read the plane-triangle intersection function being promoted.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate.
- `target/`, `Cargo.lock`, generated bindgen output — never load.
- Vendored deps — never load.
- `modules/core-modules/overhang-classifier-default/src/{lib,classify,lines_distancer}.rs` — out of scope; refactor lands in P107.
- All perimeter module `lib.rs` files — out of scope; consumption lands in P108 (post-rename).
- `crates/slicer-sdk/src/views.rs` — out of scope; view accessor for `overhang_quartile_polygons` is P107 (O-T031).
- All other `slicer-core` algos except the 4 named files — out of scope.
- All other crates not in §Files in Scope.

## Expected Sub-Agent Dispatches

- "Summarize OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:159-199 for `detect_steep_overhang` algorithm; return SUMMARY ≤ 150 words confirming: (a) input is slice polygons (Polygons type), (b) band threshold formula uses `extrusion_width × multiplier`, (c) quartile band count is 4." — Step 4.
- "Find the plane-triangle intersection function in `crates/slicer-core/src/algos/support_geometry.rs`; return LOCATIONS ≤ 5 entries (function name + line range)." — Step 3.
- "Find the existing OverhangRegion construction site at `crates/slicer-core/src/algos/mesh_analysis.rs:206`; confirm field set matches packet metadata; return FACT (field list)." — Step 2.
- "Find an existing prepass stage's scheduler declaration + builtin producer wiring (e.g., `RegionMapping` producer at `crates/slicer-runtime/src/builtins/region_mapping_producer.rs`); return LOCATIONS ≤ 5 entries (the pattern to mirror)." — Step 5.
- "Run `cargo check --workspace --all-targets`; return FACT pass/fail + SNIPPETS ≤ 20 lines on fail." — after each step.
- "Run targeted TDD per AC; return FACT pass/fail + assertion text on fail."
- "Run `cargo xtask build-guests --check`; return FACT (clean / STALE list)." — Step 2 closure gate.

## Data and Contract Notes

- IR or manifest contracts touched: `OverhangRegion` gains a field; `SurfaceClassificationIR` gains a field. Additive via `#[serde(default)]`. Schema version bumps minor.
- WIT boundary considerations: `OverhangRegion` is a leaf record inside `surface-classification-ir`; the `xy-footprint` field add is backward-compatible. `overhang-quartile-polygons` is a HashMap accessor — pick the WIT representation (likely `list<tuple<u32, list<quartile-band>>>` or a dedicated record type).
- Determinism or scheduler constraints: `overhang_annotation` is a pure function over `MeshIR` + `LayerPlanIR` + threshold config. Deterministic. The new stage has explicit precondition declarations on `MeshAnalysis` + `LayerPlanning`; the scheduler validates ordering.
- Quartile threshold defaults: `line_width × {0.5, 1.0, 1.5, 2.0}` per O-4 default. `line_width` comes from the `outer_wall_line_width` config (or `line_width` fallback if outer/inner widths are not registered yet — i.e., before P105 ships its width split).

## Locked Assumptions and Invariants

- `OverhangRegion.xy_footprint` mirrors `BridgeRegion.xy_footprint`: per-region 2D projection of the underlying facets, populated at `MeshAnalysis`.
- `overhang_quartile_polygons` outer key is the global layer index (`u32`). Inner Vec carries one `QuartileBand` per quartile (1, 2, 3, 4); a layer with no overhang has an empty Vec at that key (or the key is absent — both semantics are valid; pick one and document).
- The classifier classifies based on distance to **previous-layer cross-section**, NOT to previous-layer walls. The threshold band formula uses `outer_wall_line_width × {0.5, 1.0, 1.5, 2.0}` (or `line_width` if width split isn't shipped yet).
- The new stage runs strictly after `MeshAnalysis` + `LayerPlanning` and strictly before any Tier 2 stage. It does not depend on `PaintSegmentation` or any other PrePass stage.
- ADR-0012 supersedes only ADR-0008's "unnecessary scope" caveat — NOT ADR-0008 in its entirety. The speed-factor-application-at-finalization decision stays.

## Risks and Tradeoffs

- Schema-bump race with P105: if P105 lands first at 4.3.0, this packet bumps to 4.4.0. If this packet lands first (less likely given dependency on P105's MMU foundation work — verify), it bumps to 4.3.0 and P105's bump becomes 4.4.0. Doc-impact greps allow either ordering; document in closure log.
- Mesh cross-section helper signature: the implementer must preserve the exact semantics of `support_geometry.rs`'s existing implementation. AC-4's `prepass_support_geometry_tdd` is the regression bed; if it fails after promotion, the implementer halts and diagnoses before continuing.
- `line_width` vs `outer_wall_line_width` for threshold formula: depends on P105 sequencing. If P105 ships first with the outer/inner width split, this packet uses `outer_wall_line_width`. Otherwise, fallback to `line_width`. Documented in `OverhangAnnotation` config-read code.
- Quartile threshold formula deviation: if OrcaSlicer SUMMARY surfaces a formula different from `× {0.5, 1.0, 1.5, 2.0}`, the implementer documents the deviation in the closure log and the threshold-defaulting code; AC-5's tolerance covers small differences.

## Context Cost Estimate

- Aggregate (sum across all steps): `M`
- Largest single step: `M` (Step 4 — classifier algorithm + reference-fixture TDD; longest LOC delta).
- Highest-risk dispatch: OrcaSlicer SUMMARY (≤ 150 words). Re-dispatch if pseudocode appears.

## Open Questions

- `[FWD]` Mesh cross-section helper exact signature: `cross_section_at_z(mesh, z) -> Vec<ExPolygon>` is the documented default. If `support_geometry.rs`'s existing function returns a different type (`Vec<Polygon>` rather than `Vec<ExPolygon>`), the implementer harmonises the signature; AC-4's grep matches the chosen name.
- `[FWD]` `overhang_quartile_polygons` HashMap empty-layer semantics: key absent vs key present with empty Vec. Pick one in implementation; document in the IR doc-comment.
- `[FWD]` WIT representation of `HashMap<u32, Vec<QuartileBand>>`: likely a `list<tuple<u32, list<quartile-band>>>` or a dedicated record type with an indexed accessor. Implementer picks the WIT shape that compiles cleanly and matches existing HashMap-in-WIT conventions (verify against `paint_region_ir` HashMap handling if precedent exists).
