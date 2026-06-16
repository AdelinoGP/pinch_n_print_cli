# Plan — OrcaSlicer Parity for MeshSegmentation, PaintSegmentation, RegionMapping

## Context

The current pinch_n_print paint pipeline has three correctness failures and one
performance failure, surfaced by the v2 audit and the cherry-pick `5c272ef`'s
RED test suite:

1. **PaintSegmentation broadcasts XY shadows.** `crates/slicer-core/src/algos/paint_segmentation.rs:298-362`
   drops Z, attaches each painted facet's projected triangle to every layer the
   object participates in, and never computes slice-plane intersections. There is
   no EdgeGrid, no Voronoi, no top/bottom propagation, no width limiting.
   Wrong tool/material assignments on any non-vertical painted facet.
2. **MeshSegmentation kernel is dead code.** `crates/slicer-core/src/algos/mesh_segmentation.rs:39-109`
   correctly normalizes sub-facet strokes into whole-triangle assignments but is
   never invoked from the prepass driver. Strokes parsed at load survive into
   `paint_data.layers[*].strokes` and are silently dropped by the paint kernel.
3. **RegionMapping paint overlay ignores geometry.** `overlapping_semantics_for_region`
   at `crates/slicer-core/src/algos/region_mapping.rs:286-319` hard-codes
   `return true`. The current "config overlay" stamps every paint semantic onto
   every region on the layer regardless of object identity or geometric overlap.
4. **Benchy fixtures dominate the test bench.** `benchy.stl` (11 MB),
   `benchy_4color.3mf` (2.6 MB), and `benchy_painted.3mf` (2.5 MB) drive ~50
   tests through full pipelines with weak assertions; cherry-pick `5c272ef`
   lands `cube_4color.3mf` (37 KB) and `cube_fuzzyPainted.3mf` (27 KB) with
   engineered per-face semantics and 24 deterministic tests as the targeted
   replacement, but the migration is not yet done.

Beyond fixing the bugs, this plan formalizes a region-splitting IR model that
matches OrcaSlicer's `PaintedRegion`/`FuzzySkinPaintedRegion` cross-product
expansion, generalized to an open-string namespace so community-authored
modules can declare new region-splitting paint semantics without IR schema
changes. The full design — 15 architectural decisions reached over an extended
design dialog — is recorded in the "Design Decisions" section below.

**Outcome of this plan**:

- All RED tests in `cube_4color_paint_tdd.rs` and `cube_fuzzy_painted_tdd.rs` GREEN.
- OrcaSlicer-parity multi-color slicing, including Phase 5 width limiting + interlocking.
- Mesh-segmentation host kernel wired into the prepass driver.
- RegionMapping config overlays geometrically isolated per object.
- Benchy fixtures retired (5.2 MB + 11 MB reclaimed; ~50 tests rewritten).
- Open-string variant-chain extensibility for community paint semantics, with
  built-in `material` and `fuzzy_skin` matching OrcaSlicer behavior.
- Doc/code drift eliminated.

---

## Authoritative References

- **`docs/specs/orca-paint-segmentation-parity.md`** — the team's 1,021-line
  handoff spec. The 7-phase paint pipeline, IR types, threading model, hazard
  list (H561–H567), and OrcaSlicer pseudocode line numbers are normative.
- **OrcaSlicer reference**: `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp`
  (2,539 LOC), `Print.hpp:243-289` (region-splitting data model), `PrintApply.cpp:1138-1156`
  (cross-product expansion), `PrintObjectSlice.cpp:924-1081` (`apply_mm_segmentation`).
- **RED tests as acceptance gate**: `crates/slicer-runtime/tests/executor/cube_4color_paint_tdd.rs`
  (12 tests) + `cube_fuzzy_painted_tdd.rs`. All RED tests must flip GREEN.
- **Coordinate hazard**: `docs/08_coordinate_system.md` — 1 unit = 100 nm.
  Every OrcaSlicer integer constant divides by 100.
- **Glossary**: `CONTEXT.md` (updated with `Variant chain`, `Painted variant`,
  `Region-split semantic`, `Segment annotation`, sharpened `"region"` ambiguity).

---

## Design Decisions

15 decisions reached during planning grilling. Each is referenced by `D<n>` in
the packet specs below.

| # | Decision area | Choice |
|---|---|---|
| D1 | Where paint-segmentation runs | Post-`host:slice`, between `host:shell_classification` and `host:support_geometry` (NOT in pre-slice prepass). |
| D2 | IR shape for paint-driven config splits | Region-splitting (one `RegionPlan` + one `SlicedRegion` per painted variant), mirroring OrcaSlicer's `PaintedRegion`/`FuzzySkinPaintedRegion`. |
| D3 | Variant discriminator on `RegionKey` | Parent-pointer chain with `parent_kind: String`; open namespace for community semantics. Canonical sequence is the **variant chain**. |
| D4 | Which semantics drive region-splitting | Opt-in per semantic via module manifest's `[[region_split]]` block. Support enforcer/blocker do NOT region-split (modifier-volume path; see D14). |
| D5 | Voronoi shape | Per-semantic Voronoi passes + geometric composition (`intersection_ex` / `difference_ex`) to derive each variant chain's exact polygons. |
| D6 | Variant-chain canonical order | Fixed core priorities (`material = 100`, `fuzzy_skin = 200`) + community tail `>= 1000`. Stable lex tiebreaker. |
| D7 | Manifest schema | Top-level `[[region_split]]` array. Each entry: `semantic`, `priority`, `value_type ∈ {flag, tool_index, custom_string}`. |
| D8 | Per-variant polygons | Inlined into `SliceIR.regions[*]` via `replace_slice_ir`. Each `SlicedRegion` carries `variant_chain`. `PaintRegionIR` is deleted. |
| D9 | Module dispatch | Host-filtered: layer executor reads each module's `[[region_split]]` and only invokes the module on regions whose `variant_chain` matches. Modules with no `[[region_split]]` are paint-transparent. |
| D10 | Config interning | `ConfigId(u32)` + `RegionMapIR.configs: Vec<ResolvedConfig>` lookup table. `RegionPlan.config: ConfigId`. Module accessors use a convenience helper. |
| D11 | `boundary_paint` disposition | Renamed to `segment_annotations`. `SliceIR` and `RegionMapIR` bumped to 2.0.0. |
| D12 | Foundation packet shape | Three sliced packets (P1a/P1b/P1c). Each leaves the workspace in a working state. |
| D13 | `PaintValue` in `variant_chain` | `Flag`, `ToolIndex`, `Custom(String)` allowed. `Scalar(f32)` rejected at manifest-load time (Scalar paints route to `segment_annotations` instead). |
| D14 | Modifier-volume support enforcer/blocker | Routes to `segment_annotations[SupportEnforcer/Blocker]`. NOT region-splitting. Matches OrcaSlicer modifier-volume + LayerRegion pattern. |
| D15 | Empty-polygon `RegionPlan` entries | Emit all cross-product entries unconditionally in RegionMapping (mirrors OrcaSlicer); Z-range pruning deferred as optional optimization. |

**Deferred (explicitly out of this plan; documented in `docs/specs/orca-paint-segmentation-parity.md` as follow-ups)**:

- **3MF parser extension hook for community paint channels** — built-in semantics
  (`material`, `fuzzy_skin`, support, seam) work end-to-end via the existing loader.
  Community semantics declarable in manifests but no ingestion path; first
  community semantic ships needs a follow-up.
- **`PaintValue::Vector(Vec<f32>)` IR addition** — for ergonomic multi-channel
  paints (CMYK, RGB, vector fields). Workaround today: 4 parallel single-channel
  semantics. Defer until 2+ multi-channel use cases land.
- **Promote paint-segmentation's internal slicing to `host:raw_slice`** if profiling
  demands. Today: paint-segmentation reads `SliceIR` directly (it runs post-slice).
- **Single-pass Voronoi over multi-color sites** (option Q from grilling) — defer
  unless per-layer Voronoi pass count becomes a profile hot spot.

---

## Reusable Building Blocks (already in the workspace)

| Need | Use | Path |
|---|---|---|
| Polygon union/intersection/difference (flat) | `union`, `intersection`, `difference`, `xor` | `crates/slicer-core/src/polygon_ops.rs:93-108` |
| Polygon offset | `offset(polygons, delta_mm, join, arc_tol)` | `crates/slicer-core/src/polygon_ops.rs:185` |
| Polygon simplicity validation | `validate_polygon_simplicity` | `crates/slicer-core/src/polygon_ops.rs:131` |
| Triangle mesh → 2D slice | `slice_mesh_ex(mesh, zs)` | `crates/slicer-core/src/triangle_mesh_slicer.rs:48` |
| Sliver removal | `apply_slice_closing_radius` | `crates/slicer-core/src/triangle_mesh_slicer.rs:394` |
| Coordinate conversion | `Point2::from_mm`, `mm_to_units`, `units_to_mm` | `crates/slicer-ir/src/slice_ir.rs` |
| R-tree spatial index | `rstar = "0.12"` already in `slicer-core` and `slicer-runtime` Cargo.toml |
| Blackboard atomic-replace pattern | `replace_slice_ir` at `blackboard.rs:276-290` |
| Process-local fixture cache | `cached_load_model`, `cached_run` | `crates/slicer-runtime/tests/common/model_cache.rs`, `slicer_cache.rs` |
| Modifier-volume slicing (will be repurposed) | `paint_segmentation.rs:374-417` |

**New polygon helpers needed (extend `slicer-core/src/polygon_ops.rs`)**:

- `union_ex`, `intersection_ex`, `difference_ex` — ExPolygon-aware variants that preserve holes
- `opening`, `closing_ex` — offset(-d)→offset(+d) and offset(+d)→offset(-d)
- `remove_small_and_small_holes(expolys, min_area_sqr_units)` — OrcaSlicer's `MultiMaterialSegmentation.cpp:2252`
- `expolygons_simplify(expolys, tolerance_units)` — Douglas-Peucker on contour + holes
- `remove_duplicates(expolys, eps_units, angle_eps_rad)` — collinear-point merge near each vertex
- `clip_line_with_bbox(line, bbox) -> Option<Line>` — bbox-clip a segment

**New crate dependency**: `boostvoronoi` (spec §2). Verify API surface (line-segment
sites, `vertex.color()` metadata, infinite-edge clipping via `is_primary()` / `twin()`)
as the first step of the paint-segmentation port (P3 sub-step 7).

---

## Packet Roadmap

```
P0a  Benchy 3MF retirement (independent, ships now)
P0b  benchy.stl → regression_wedge.stl swap (independent)
 │
 ▼  (foundations begin; serial)
P1a  Schema scaffolding (behavior-preserving — IR types only)
 │
P1b  Manifest schema + host-filtered dispatch (no module declares yet)
 │
P1c  RegionMapping cross-product expansion (variants populated in RegionKey, polygons empty)
 │
P2   host:mesh_segmentation kernel wiring + Blackboard::replace_mesh
 │
P3   Paint-segmentation port — Phases 1, 2, 3, 4, 6, 7 of the handoff spec
 │   (large packet; replaces `execute_paint_segmentation` and `run_paint_annotation`,
 │    deletes PaintRegionIR, populates per-variant polygons via replace_slice_ir)
 │
P4   Phase 5 — width limiting + interlocking
 │
P5a  WASM mesh-segmentation surface deletion (97 files)
P5b  Loader symmetry (paint_seam + paint_fuzzy_skin sub-facet decoding)
P5c  Doc updates (docs/01, /02, /03, /04, /07)
```

Total: 11 packets, all sequential. Each lands the workspace in a stable state.

---

### P0a — Benchy 3MF retirement (5.2 MB reclaim)

**Goal**: rewrite all tests using `resources/benchy_4color.3mf` or `resources/benchy_painted.3mf`
to target the engineered cube fixtures from cherry-pick `5c272ef`. Delete the
benchy 3MFs.

**Scope by test classification (Finding #9 from v2 audit)**:

| File | Tests | Classification | Migration target |
|---|---|---|---|
| `benchy_4color_modifier_part_e2e_tdd.rs` | 7 | 6 STRUCTURAL + 1 SHAPE-DEPENDENT (duplicates an existing cube test) | `cube_4color.3mf` or a small `cube_with_modifier_part.3mf` |
| `benchy_painted_e2e_tdd.rs` | 2 | 1 CLI-SHAPE + 1 SHAPE-DEPENDENT (assert `painted != unpainted` gcode) | `cube_4color.3mf` vs no-paint cube |
| `benchy_painted_overrides_e2e_tdd.rs` | 1 | 1 CLI-SHAPE | `cube_4color.3mf` |
| `threemf_fixture_e2e_tdd.rs` (lines 540-924) | 13 refs | STRUCTURAL | `cube_4color.3mf` |
| `threemf_paint_drop_on_modifier_tdd.rs` | 1 | STRUCTURAL | `cube_with_modifier_part.3mf` (add if needed) |
| `threemf_transform_tdd.rs` | 4 refs | STRUCTURAL (rotated component) | tiny `cube_rotated_component.3mf` (add if needed) |
| `model_loader_tdd.rs` | 6 refs of 37 tests | STRUCTURAL | `cube_4color.3mf` |
| `threemf_sidecar_classification_tdd.rs` | 9 refs | STRUCTURAL | `cube_4color.3mf` |

**Steps**:

1. Audit each file's test bodies, classify each #[test], rewrite assertions to use the cube fixtures.
   Where assertions strengthen (e.g., "this face has ToolIndex 1" against a known cube face), strengthen them.
2. If `cube_with_modifier_part.3mf` and `cube_rotated_component.3mf` don't exist as
   sub-fixtures, author them as small derivatives of the cube_4color fixture.
3. Delete `resources/benchy_4color.3mf`, `resources/benchy_painted.3mf`, `resources/benchy_painted.README.md`.
4. Verify zero remaining references: `rg -nl 'benchy_4color\.3mf|benchy_painted\.3mf' crates/ modules/ docs/ .ralph/` returns no matches.

**Verification**:

```bash
cargo test -p slicer-model-io --test model_loader_tdd 2>&1 | tee target/test-output.log
cargo test -p slicer-runtime --test e2e 2>&1 | tee target/test-output.log
cargo clippy --workspace --all-targets -- -D warnings
rg -nl 'benchy_4color\.3mf|benchy_painted\.3mf' crates/ modules/ docs/ .ralph/  # expect: 0
```

---

### P0b — `benchy.stl` → `regression_wedge.stl` swap

**Goal**: retire `resources/benchy.stl` (11 MB, ~200k triangles) as a test fixture.
Replace with `resources/regression_wedge.stl` (~50 KB, purpose-built features:
top surface, bottom surface, a deliberate bridge, a 45° overhang, an ironable
top section). Update 42 tests in `benchy_end_to_end_tdd.rs` + a handful of refs
elsewhere.

**Test classification (v2 audit Finding #9)**:

- 22 CLI-SHAPE: `cached_run` against an STL, assert "exit 0", "output written",
  "byte-identical across runs". Any real STL works.
- 17 SHAPE-DEPENDENT: assert markers like `;TYPE:Top surface`, `;TYPE:Bridge`,
  `;TYPE:Ironing`, retract-pair counts, layer count > 100. Need a real-shape mesh
  with the relevant features — wedge provides all of them.
- 3 STRUCTURAL: stderr / module-discovery surface. Fixture-independent.

**Steps**:

1. Author `resources/regression_wedge.stl` (~50 KB) with: 40 mm tall, 45° overhang
   on one side, 5 mm flat top, 8 mm flat bottom, a 10 mm bridge gap in the middle
   front face, and an ironable top section ≥ 25 mm × 25 mm.
2. Rename `benchy_end_to_end_tdd.rs` → `slice_end_to_end_tdd.rs`. Function prefix
   `benchy_*` → `slice_*` (or `wedge_*` where assertion depends on shape features).
   Mechanical sed.
3. Update fixture references in `crates/slicer-runtime/tests/common/slicer_cache.rs:135`,
   `crates/slicer-model-io/tests/stl_roundtrip_tdd.rs:15-17`,
   `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs:332`,
   `crates/pnp-cli/tests/slice_instrumentation_fork_tdd.rs:32`.
4. Delete `resources/benchy.stl`.

**Verification**:

```bash
cargo test -p slicer-runtime --test e2e slice_end_to_end 2>&1 | tee target/test-output.log
rg -nl 'benchy\.stl' crates/ modules/ docs/ .ralph/  # expect: 0
# Wall-clock comparison (run before and after):
cargo clean -p slicer-runtime
time cargo test -p slicer-runtime 2>&1 | tee target/test-output.log
```

Expected wall-clock impact: a real-mesh slice on 11 MB benchy at ~200k triangles
takes seconds per `cached_run`; the wedge at ~50 KB / ~200 triangles takes
sub-second. With 29 distinct cache keys in the old `benchy_end_to_end_tdd.rs`,
total reduction is multi-minute.

---

### P1a — Schema scaffolding (D8, D10, D11)

**Goal**: land the IR type additions and renames REQUIRED by D8/D10/D11.
Behaviorally a no-op: every new field gets a default value (empty `variant_chain`,
`ConfigId(0)`, etc.), every existing test passes unchanged.

**IR changes**:

1. **`RegionKey` gains `variant_chain: Vec<(String, PaintValue)>`** in `slicer-ir/src/slice_ir.rs`.
   Empty vector = base region. Hash + Eq derived. `PaintValue::Scalar(f32)` uses
   `to_bits()` for hash (Scalar in variant_chain is forbidden at manifest-load time
   per D13, but Hash impl is still required).

2. **`ConfigId(u32)`** newtype in `slicer-ir`. Tuple struct, `Copy`. Nominal type
   `RegionMapConfigId` to prevent cross-IR ID confusion.

3. **`RegionMapIR.configs: Vec<ResolvedConfig>`** field added. `RegionPlan.config: ConfigId`
   replaces the inline `ResolvedConfig`. Lookup helper: `RegionMapIR::config_for(&RegionKey) -> &ResolvedConfig`.

4. **`SlicedRegion.boundary_paint` renamed to `segment_annotations`** — same shape
   (`HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>>`), narrower documented
   scope ("populated only for paint semantics NOT declared `[[region_split]]`").

5. **`SlicedRegion.variant_chain: Vec<(String, PaintValue)>`** field added. Empty
   on all existing test fixtures.

6. **Schema bumps**:
   - `SliceIR` 1.0.0 → 2.0.0 (breaking: field rename + variant_chain addition).
   - `RegionMapIR` 1.0.0 → 2.0.0 (breaking: RegionKey shape change + configs Vec).
   - Update `min_ir_schema` / `max_ir_schema` on every `BuiltinProducer` constant
     (`crates/slicer-runtime/src/builtins/*.rs`) to admit 2.x.

7. **`ResolvedConfig.extensions: HashMap → BTreeMap`** migration. Required for
   deterministic hash (D10), useful for snapshot determinism in general. Touch
   `crates/slicer-ir/src/resolved_config.rs` and every site that constructs/iterates
   the field.

8. **`Hash for ResolvedConfig`** derived (now possible after BTreeMap migration;
   float fields via `to_bits()`). Used by config interner; documented as
   "consistent within one process; not portable across architectures with
   different NaN bit-patterns" — acceptable since the interner doesn't outlive a
   single prepass invocation.

9. **`PaintValue` Hash impl** — derive `Eq` + `Hash`, `Scalar` via `to_bits()`,
   `Custom` via its `String`. All four variants hashable for use in
   `variant_chain`.

**Migration of existing call sites**:

- `paint_segmentation.rs:117` — `HashablePaintValue` wrapper becomes redundant
  (PaintValue now Hash). Delete the wrapper, use PaintValue directly.
- All 4 production sites that read `plan.config` (`prepass_slice.rs:275`,
  `slice_postprocess_prepass.rs:348,390`, `layer_executor.rs:783`, `dispatch.rs:1975,2009`)
  — update to use `region_map.config_for(&key)` or a convenience accessor.
- `~20 test files` reference `boundary_paint` — rewrite to `segment_annotations`.
  Mechanical sed.

**Critical files**:

- `crates/slicer-ir/src/slice_ir.rs` — `SliceIR`, `SlicedRegion`, `RegionKey`,
  `RegionMapIR`, `RegionPlan`, schema constants
- `crates/slicer-ir/src/resolved_config.rs` — extensions BTreeMap migration, Hash derive
- `crates/slicer-core/src/algos/region_mapping.rs` — RegionPlan/ConfigId construction
- `crates/slicer-runtime/src/builtins/*.rs` — schema-version bumps on producer constants
- `crates/slicer-runtime/src/blackboard.rs` — no shape change; commit/replace functions
  unchanged except `RegionMapIR` type signature

**Verification**:

```bash
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace 2>&1 | tee target/test-output.log
# Every existing test passes unchanged. Behavior is preserved.
```

---

### P1b — Manifest schema + host-filtered dispatch (D4, D6, D7, D9)

**Goal**: add the `[[region_split]]` manifest schema, the priority registry, and
the host-filtered dispatch hook. No core module declares `[[region_split]]` yet,
so behavior is still unchanged.

**Manifest schema additions** (in `crates/slicer-scheduler/src/`):

```toml
[[region_split]]
semantic = "fuzzy_skin"
priority = 200            # required; core slot
value_type = "flag"       # one of: flag | tool_index | custom_string
                          # value_type = "scalar" is rejected at manifest-load

[[region_split]]
semantic = "com.example.thermal:expansion"
priority = 1500           # community range >= 1000
value_type = "custom_string"
```

**Priority registry** in `crates/slicer-schema/`:

```rust
pub const CORE_REGION_SPLIT_PRIORITIES: &[(&str, u32)] = &[
    ("material", 100),
    ("fuzzy_skin", 200),
];
// Community range: priority >= 1000. Tied priorities broken by lex on semantic name.
```

**Manifest-load validation** (in `crates/slicer-scheduler/`):

- Per-manifest: reject duplicate `[[region_split]]` entries for the same semantic.
- Per-manifest: reject `value_type = "scalar"` (D13).
- Per-manifest: reject `priority < 1000` for a semantic name NOT in `CORE_REGION_SPLIT_PRIORITIES`.
- Cross-manifest: WARN on tied priorities across different semantics.

**Scheduler aggregation**:

- At startup, scheduler walks all loaded modules' `[[region_split]]` blocks.
- Computes `aggregated_region_split: BTreeMap<String_semantic, AggregatedRegionSplitEntry>`
  where the entry carries `{ priority, value_type, declaring_modules: Vec<ModuleId> }`.
- Sorted by `(priority, name)` — defines the canonical variant-chain order.

**Host-filtered dispatch in `layer_executor.rs`**:

- Per (layer × region), for each module:
  - If module declares `[[region_split]]` for some set S: invoke only if `region.variant_chain` contains at least one `(s, _)` with `s ∈ S`.
  - If module declares no `[[region_split]]`: invoke unconditionally (paint-transparent default).
- Add an empty-polygon guard: skip module invocation if `region.polygons.is_empty()`.
  Universal default; bug-class prevention even outside paint context.

**Critical files**:

- `crates/slicer-schema/src/` — priority registry constants
- `crates/slicer-scheduler/src/` — manifest TOML parser extension, validation
- `crates/slicer-runtime/src/layer_executor.rs:494-528` — host-filter hook
- `crates/slicer-runtime/tests/integration/` — new test: a synthetic manifest with
  `[[region_split]]`, assert dispatch honors it

**Verification**:

```bash
cargo test -p slicer-scheduler 2>&1 | tee target/test-output.log
cargo test -p slicer-runtime --test integration 2>&1 | tee target/test-output.log
cargo clippy --workspace --all-targets -- -D warnings
```

**Descoped at P92 refinement audit (2026-06-08)**: the "universal empty-polygon dispatch guard" originally planned alongside the host filter was REMOVED from P92. The codebase's per-(module × layer) dispatch leaves no per-region host invocation site at which the guard could fire (per-region iteration is module-internal). The guard is **owned by P95** (paint-segmentation port), which has the polygons in hand via `replace_slice_ir` — resolved during the P93 refinement pass (Audit 2). P93 keeps D15 unconditional emission: the cross-product kernel cannot predict polygon emptiness from `ActiveRegion` alone, so the empty-polygon check belongs at P95's paint-segmentation output boundary, not at P93's `RegionMapIR.entries` emission. P92's filter scope is per-(module × layer) only; per-(module × layer × region) is deferred to a candidate follow-up if P95 closure shows it's needed.

---

### P1c — RegionMapping cross-product expansion (D2, D3, D5, D6, D10, D15)

**Goal**: RegionMapping now expands one ActiveRegion into N variant entries per
declared region-split semantic × distinct paint value present on that object.
Variant chains populate `RegionKey.variant_chain`. Per-variant polygons remain
empty (they come from P3 paint-segmentation port). Cube tests start asserting
on the new shape.

**Pre-pass step inside `commit_region_mapping_builtin`**:

```rust
// Once per RegionMapping invocation:
let painting_variants_per_object: HashMap<ObjectId, HashMap<String, Vec<PaintValue>>> =
    scan_paint_data(&mesh.objects, &aggregated_region_split_semantics);
```

The scan iterates each `object.paint_data.layers[*].facet_values`, collects the
distinct paint values per opted-in semantic. Cheap; one pass over paint data per object.

**Expansion algorithm**:

```rust
for layer in layer_plan.global_layers {
    for active_region in layer.active_regions {
        let variants = painting_variants_per_object
            .get(&active_region.object_id)
            .cloned()
            .unwrap_or_default();
        let chains = enumerate_canonical_chains(variants, &canonical_order);
        // chains[0] is always the empty (base) chain.

        for chain in chains {
            let key = RegionKey {
                global_layer_index: layer.index,
                object_id: active_region.object_id.clone(),
                region_id: active_region.region_id,
                variant_chain: chain.clone(),
            };
            let plan_config = derive_resolved_config(
                &active_region.resolved_config,
                &chain,
                &paint_semantic_configs,
            );
            let config_id = interner.intern(plan_config);
            let stage_modules = derive_stage_modules(&active_region, &chain, &stage_invocations);
            entries.insert(key, RegionPlan { config: config_id, stage_modules });
        }
    }
}
```

**`enumerate_canonical_chains` semantics**:

- Generate every subset of (semantic, value) pairs across all opted-in semantics
  that the object has any paint for.
- For each subset, order the pairs by `canonical_order` (from P1b aggregation:
  priority ascending, lex tiebreak).
- The empty subset = base region (variant_chain = []).

For a 16-color object with FuzzySkin paint and 5 base regions, expansion produces
17 * 2 = 34 variants per (layer, region) — empty + 16 Material variants × {with FuzzySkin, without}.

Note (D15): empty-polygon variants are emitted unconditionally. The cube_4color
test suite will see `RegionMapIR.entries.len()` > the count of geometric-coverage
variants. Tests assert on `RegionKey.variant_chain` presence, not on `entries.len()`.

**Modifier-volume config stamping interacts with interning**: each modifier-stamped
`ResolvedConfig` produces its own `ConfigId` via the hash-intern mechanism. The
interner Vec grows by one entry per distinct (base × stamp set × variant chain) config.

**`DEFAULT_REGION_MAP_CAP`** raised from current value to **750_000** with the
existing top-contributor diagnostic active for overflow. The headroom admits
~16-color × 1000-layer × 16-region × 3-modifier scenes; the diagnostic message
points at the most-contributing object so pathological cases are debuggable.

**Cube test retargeting**:

- 5 GREEN cube_4color tests: assert that `RegionMapIR.entries` contains the
  expected variant_chain entries. Assertions strengthen (region identity is the
  direct subject of the test now).
- 7 RED cube_4color tests: still RED here — they assert on polygon geometry that
  P3 will populate. Tests are kept; their failure messages are updated to refer
  to "variant polygons" rather than "boundary_paint contour points."

**Critical files**:

- `crates/slicer-core/src/algos/region_mapping.rs` — full kernel rewrite for
  cross-product expansion + ConfigId interning
- `crates/slicer-runtime/src/builtins/region_mapping_producer.rs` — wrapper
  receives aggregated_region_split_semantics from scheduler
- `crates/slicer-ir/src/region_split_registry.rs` — new module for
  `enumerate_canonical_chains` helper

**Verification**:

```bash
cargo test -p slicer-core region_mapping 2>&1 | tee target/test-output.log
cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 | tee target/test-output.log
#   expected: 5 GREEN cube tests pass; 7 RED still fail (P3 territory)
cargo clippy --workspace --all-targets -- -D warnings
```

---

### P2 — `host:mesh_segmentation` kernel wiring

**Goal**: insert a host built-in `PrePass::MeshSegmentation` stage that runs
FIRST in the prepass driver (before `host:mesh_analysis`). The kernel
(`crates/slicer-core/src/algos/mesh_segmentation.rs:39-109`) already exists but
is never invoked. After this packet, sub-facet strokes from any object's
`paint_data.layers[*].strokes` are normalized into whole-triangle `facet_values`
splits before any downstream stage sees the mesh.

**Change surface**:

1. **`Blackboard::replace_mesh`** added in `crates/slicer-runtime/src/blackboard.rs`.
   Sibling of `replace_slice_ir:276-290`. Same `debug_assert!` that no Tier 2
   layer outputs exist; same `MissingRequiredPrepass` guard for "mesh slot was
   never committed in the first place" (vacuous in production — mesh is always
   committed at construction).

2. **`MESH_SEGMENTATION_PRODUCER`** in `crates/slicer-runtime/src/builtins/mesh_segmentation_producer.rs`
   (new file mirroring `mesh_analysis_producer.rs`):
   ```
   id:        "host:mesh_segmentation"
   stage:     "PrePass::MeshSegmentation"
   ir_writes: &["MeshIR"]
   ir_reads:  &[]
   ```

3. **Wire into `crates/slicer-runtime/src/builtins/mod.rs`** — `pub mod mesh_segmentation_producer;`.

4. **Insert into prepass driver** at `crates/slicer-runtime/src/prepass.rs:374` (before `host:mesh_analysis`):
   ```rust
   run_builtin_stage(
       blackboard, instrumentation,
       "PrePass::MeshSegmentation", "host:mesh_segmentation",
       |bb| has_subfacet_strokes(bb.mesh()),
       |bb| {
           let normalized = execute_mesh_segmentation(bb.mesh().clone())
               .map_err(PrepassExecutionError::MeshSegmentation)?;
           bb.replace_mesh(normalized).map_err(|source| {
               PrepassExecutionError::Blackboard {
                   stage_id: "PrePass::MeshSegmentation".to_string(),
                   module_id: "host:mesh_segmentation".to_string(),
                   source,
               }
           })
       },
   )?;
   ```

5. **`PrepassExecutionError::MeshSegmentation { source: MeshSegmentationError }`** variant.

6. **`required_slots(StageId)` table** at `prepass.rs:680-708` — add `"PrePass::MeshSegmentation" => &[]`.

**Test additions**:

- Integration: load `cube_4color.3mf` (which has sub-facet hex paint), run prepass,
  assert `mesh.objects[0].paint_data.layers[*].strokes.is_empty()` after
  PrePass::MeshSegmentation runs, AND `facet_values.len()` exceeds the original
  triangle count (proving splits occurred).
- Determinism: re-run, byte-compare normalized mesh across runs.
- No-op short-circuit: load benchy.stl-equivalent unpainted mesh, verify
  `has_subfacet_strokes` returns false and `replace_mesh` is NOT called.

**Verification**:

```bash
cargo test -p slicer-core --test algo_mesh_segmentation_tdd 2>&1 | tee target/test-output.log
cargo test -p slicer-runtime --test executor mesh_segmentation 2>&1 | tee target/test-output.log
cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 | tee target/test-output.log
#   expected: cube tests assert paint_data.strokes empty after prepass
cargo clippy --workspace --all-targets -- -D warnings
```

---

**SUPERSEDED 2026-06-10 — TASK-250 architectural finding.** P2 as scoped above was implemented under TASK-244 (commits `3113083` + `89b3517`), then **retired in packet P94r (`.ralph/specs/94_host-mesh-segmentation-wiring/` rewritten in place)**. The retirement decision was driven by three structural findings from the post-implementation investigation:

1. **The loader already does this work.** `crates/slicer-model-io/src/loader.rs:1900-1961` (`split_triangle_strokes` + `walk_triangle_selector_strokes`) reproduces OrcaSlicer's `TriangleSelector` recursive subdivision exactly. `PaintLayer.strokes` arrives at the prepass blackboard in OrcaSlicer's flat-leaf form. A second normalization stage duplicates the loader's output.
2. **OrcaSlicer has no `stroke` abstraction.** `FacetsAnnotation::get_facets()` (Model.cpp:3806) reconstructs a transient per-extruder flat list from the hex bitstream on demand. Our `PaintLayer.strokes` is the IR-resident equivalent — keeping it is the parity-honoring choice; flattening it into the mesh IR would diverge from OrcaSlicer's actual operational shape.
3. **The kernel structurally fails on OrcaSlicer-pattern leaves.** 12+ `TangentToFacetEdge` raise sites at `crates/slicer-core/src/algos/mesh_segmentation.rs` reflect the kernel's clean-bisection template not fitting arbitrary-depth subdivisions. `cube_4color.3mf` triggered this on every paint-segmentation integration test in P94's original framing.

Post-P94r outcome: the host stage is gone; the kernel + producer constant + Blackboard::replace_mesh + the four P94 integration tests are deleted; `cube_4color.3mf` slices end-to-end. P95's `collect_facets()` (parity doc §Phase 3 lines 140-141) reads `PaintLayer.facet_values` + `PaintLayer.strokes` directly — the data-model fork is intentional and locks in OrcaSlicer parity. P97 (WASM mesh-segmentation deletion) loses its "kernel survives" survival claim but is otherwise unchanged.

---

### P3 — Paint-segmentation port (Phases 1, 2, 3, 4, 6, 7)

**Goal**: replace `execute_paint_segmentation` and `execute_slice_postprocess_paint_annotation`
with the OrcaSlicer-parity pipeline per `docs/specs/orca-paint-segmentation-parity.md`.
The new kernel runs post-`host:shell_classification`, before `host:support_geometry`,
reading `SliceIR` and producing per-variant `SlicedRegion`s via `replace_slice_ir`.
`PaintRegionIR` is deleted.

This is the largest packet in the plan. Sub-step order matches spec §10:

| # | Step | Output | Spec ref |
|---|---|---|---|
| 0 | Polygon helper expansion (`union_ex`, `intersection_ex`, `difference_ex`, `opening`, `closing_ex`, `remove_small_and_small_holes`, `expolygons_simplify`, `remove_duplicates`, `clip_line_with_bbox`) | New helpers in `crates/slicer-core/src/polygon_ops.rs` | §5 constants |
| 1 | `triangle_z_intersection(p0,p1,p2,z) -> Option<Line>` pure math | `crates/slicer-core/src/algos/paint_segmentation/triangle_intersect.rs` | §3 Phase 3 |
| 2 | `EdgeGrid` data structure + `visit_cells_intersecting_line` | `crates/slicer-core/src/algos/paint_segmentation/edge_grid.rs` | §3 Phase 2, §4 |
| 3 | `PaintedLineVisitor` + `PaintedLine` private type | `…/paint_segmentation/painted_line.rs` | §4 |
| 4 | Phase 1 slice preprocessing — operates on `SliceIR.regions` (already on blackboard) | `…/paint_segmentation/preprocess.rs` | §3 Phase 1 |
| 5 | Phase 3 driver — object × extruder × facet → painted_lines | `…/paint_segmentation/phase3.rs` | §3 Phase 3 |
| 6 | `post_process_painted_lines` + `colorize_contours` + `ColoredLine` private type | `…/paint_segmentation/colorize.rs` | §3 Phase 4a/4b |
| 7 | `boostvoronoi` dep + API verification spike + `MMU_Graph` | `crates/slicer-core/Cargo.toml`, `…/paint_segmentation/voronoi_graph.rs` | §3 Phase 4c |
| 8 | `remove_multiple_edges_in_vertices` + `remove_nodes_with_one_arc` | `…/paint_segmentation/voronoi_prune.rs` | §3 Phase 4d/4e |
| 9 | `extract_colored_segments` (leftmost-arc walk + repair path with `Option<usize>` sentinel) | `…/paint_segmentation/extract_segments.rs` | §3 Phase 4f |
| 10 | `slice_mesh_slabs` — new helper in `slicer-core` for top/bottom classification | `crates/slicer-core/src/triangle_mesh_slicer.rs` (extend) | §3 Phase 6 |
| 11 | Phase 6 top/bottom propagation | `…/paint_segmentation/top_bottom.rs` | §3 Phase 6 |
| 12 | Phase 7 merge + variant-chain composition (per-semantic outputs → variant-chain ExPolygon map via geometric composition with `intersection_ex` / `difference_ex`) | `…/paint_segmentation/compose_variants.rs` | §3 Phase 7 + D5 |
| 13 | New driver `execute_paint_segmentation_v2(mesh, slice, layer_plan, region_map, config)` — produces `Arc<Vec<SliceIR>>` with per-variant `SlicedRegion` entries | `crates/slicer-core/src/algos/paint_segmentation/mod.rs` | §7 |
| 14 | Modifier-volume sub-pipeline preserved (D14) — slice `mv.mesh` per layer, route to `segment_annotations[SupportEnforcer/Blocker]` on the BASE variant `SlicedRegion`. No region-split. | `…/paint_segmentation/modifier_volumes.rs` | D14 |
| 15 | Wire into prepass driver: new built-in `host:paint_segmentation` between `host:shell_classification` (`prepass.rs:561-571`) and `host:support_geometry` (`prepass.rs:576-588`). Reads `mesh`, `slice_ir`, `layer_plan`, `region_map`. Writes via `replace_slice_ir`. Stage id stays `PrePass::PaintSegmentation`. | `crates/slicer-runtime/src/prepass.rs` | D1 |
| 16 | Delete `execute_paint_segmentation` (old) and `PaintRegionIR` entirely. Drop `Blackboard::paint_regions()` + `commit_paint_regions` + `PaintRegionRTreeEntry/Index` + `point_in_paint_region` in `crates/slicer-core/src/paint_region.rs`. | various | D8 |
| 17 | Replace `run_paint_annotation` body at `layer_executor.rs:494-528` — drop its work (annotation is now intrinsic to `SlicedRegion.variant_chain` + segment_annotations). The function may degenerate to a no-op or be removed entirely if no per-layer paint work remains. Keep `execute_slice_postprocess_paint_annotation` as a thin shim during the transition; delete at packet close. | `crates/slicer-runtime/src/slice_postprocess.rs:302`, `layer_executor.rs:494-528` | D8 + §7 |

**Slice ordering (D1) revisited explicitly**:

Old prepass order (before this packet):
```
mesh_analysis → user-early → paint_segmentation → region_mapping → slice → shell_classification → support_geometry
```

New prepass order (after this packet):
```
mesh_segmentation (from P2) → mesh_analysis → user-early → region_mapping (from P1c) → slice → shell_classification → paint_segmentation (NEW position) → support_geometry → user-late
```

Paint-segmentation now reads `SliceIR` (committed by `host:slice`) and writes
back via `replace_slice_ir`, splitting each variant region into its own
`SlicedRegion` entry with its own polygons. This matches `replace_slice_ir`'s
existing contract.

**OrcaSlicer parity hazards to mitigate (spec §8)**:

- H561 `vertex.color()` dual-use → typed-state wrappers `VoronoiVertex` / `GraphVertex`.
- H562 repair sentinel → `Option<usize>` not `usize::MAX`.
- H563 heuristic prefilter → port as-is, document the AND-logic conservatism.
- H564 two-array interleave → Rayon `par_chunks_mut` with non-overlapping layer bands.
- H565 hardcoded extruder 0 nozzle → fix on the way past; read each extruder's own nozzle config. Do NOT replicate the OrcaSlicer bug.
- H566 O(degree) dedup → `HashSet` + `debug_assert!(degree <= 20)`.
- H567 force-edge pointer arithmetic → explicit index tracking.

**Threading**: Rayon `par_iter`/`par_iter_mut` everywhere TBB is used in
OrcaSlicer (spec §6). 64-mutex bucket: `Vec<Mutex<()>>` of length 64, indexed by
`layer_idx & 63`.

**RED test acceptance**: at packet close, all 12 cube_4color tests + all 12
cube_fuzzy_painted tests are GREEN.

**Critical files** (representative — many new files; see sub-step table):

- `crates/slicer-core/src/algos/paint_segmentation/` — new module directory; replaces the file `paint_segmentation.rs`
- `crates/slicer-core/src/polygon_ops.rs` — helper additions (sub-step 0)
- `crates/slicer-core/src/triangle_mesh_slicer.rs` — `slice_mesh_slabs` (sub-step 10)
- `crates/slicer-core/Cargo.toml` — `boostvoronoi` dep (sub-step 7)
- `crates/slicer-runtime/src/prepass.rs` — wire new built-in position, delete old position
- `crates/slicer-runtime/src/blackboard.rs` — drop `paint_regions`, `commit_paint_regions`, `paint_region_rtree`
- `crates/slicer-runtime/src/layer_executor.rs:494-528` — `run_paint_annotation` body change
- `crates/slicer-runtime/src/slice_postprocess.rs:24, 302` — drop rtree field, simplify annotation
- `crates/slicer-core/src/paint_region.rs:22-93` — delete entire file
- `crates/slicer-ir/src/slice_ir.rs` — delete `PaintRegionIR`, `LayerPaintMap`, `SemanticRegion`

**Verification**:

```bash
# Per sub-step — unit tests alongside each new file
cargo test -p slicer-core paint_segmentation 2>&1 | tee target/test-output.log

# Acceptance — every cube RED test GREEN:
cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 | tee target/test-output.log
cargo test -p slicer-runtime --test executor cube_fuzzy_painted_tdd 2>&1 | tee target/test-output.log
grep "^test result" target/test-output.log
#   expected: 12 + 12 = 24 GREEN

# Full bench:
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask build-guests --check
cargo test --workspace 2>&1 | tee target/test-output.log

# Visual: render an HTML report on cube_4color and inspect a mid-layer's
# Material regions across the four painted faces:
cargo run --bin pnp_cli --release -- slice \
    --model resources/cube_4color.3mf \
    --module-dir modules/core-modules \
    --output /tmp/cube.gcode \
    --report /tmp/cube-report.html
```

---

### P4 — Phase 5: width limiting + interlocking

**Status**: implemented — packets P95 (`cut_segmented_layers` width-limit + interlocking, TASK-246) + P96 (AC-22b bisector-edge dedup via `SlicedRegion.external_contour`, TASK-246-BISECTOR). **D-95-AC22-BISECTOR-DEDUP**: resolved — P95-deferred bisector-edge test is GREEN in P96 via the `external_contour` per-object model-boundary mechanism.

**Goal**: implement OrcaSlicer's `cut_segmented_layers` per spec §3 Phase 5 so
`mmu_segmented_region_max_width` and `mmu_segmented_region_interlocking_depth`
config keys take geometric effect. User explicitly opted into full parity.

**Change surface**:

- New `crates/slicer-core/src/algos/paint_segmentation/width_limit.rs`:
  - For each layer, for each variant, erode the variant's polygons by
    `diff_ex(variant_polygons, input_expolygons offset_inward by region_width)`.
  - Alternate depth between even/odd layers when `interlocking_beam == false`;
    constant depth when `true`.
- Wire after sub-step 12 (compose_variants) inside `execute_paint_segmentation_v2`.
- Read config keys from `RegionPlan.config` via the interning helper.
- Add config-schema TOML entries to affected core modules / host config.

**Test additions** (extending `cube_4color_paint_tdd.rs`):

- A tall cube with width-limit=2.0 mm produces banded extruder regions vertically.
- Interlocking_depth=0.5 mm produces alternating bands across adjacent layers.

**Critical files**:

- `crates/slicer-core/src/algos/paint_segmentation/width_limit.rs` — new file
- `crates/slicer-runtime/src/builtins/` — config-schema for width-limit keys

**Verification**:

```bash
cargo test -p slicer-core --lib paint_segmentation::width_limit 2>&1 | tee target/test-output.log
cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 | tee target/test-output.log
cargo clippy --workspace --all-targets -- -D warnings
```

---

### P5a — WASM mesh-segmentation surface deletion

**Goal**: remove all infrastructure for a "guest can override mesh-segmentation"
path. The user clarified mesh-segmentation is a host responsibility. With P2
wiring the host kernel, the WASM module is pure debt. **Blast radius: 97 files**.
Ship as a single packet for atomicity.

**Deletions**:

- `modules/core-modules/mesh-segmentation/` — entire directory
- `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit:46-54` — drop `mesh-segmentation-output` resource and `run-mesh-segmentation` export
- `crates/slicer-wasm-host/src/host.rs:3588-3622, 767, 1042-1043` — drop the resource impl, the `mesh_segmentation_marks` field, the accessor
- `crates/slicer-wasm-host/src/dispatch.rs:1700-1727, 818, 1906-1908` — drop harvest and dispatch arm
- `crates/slicer-macros/src/lib.rs:452, 1439-1480` — drop the macro arm. Triggers guest rebuild.
- `crates/slicer-runtime/src/blackboard.rs:159-172` — drop `commit_mesh_segmentation` + `mesh_segmentation()` accessor (the host built-in uses `replace_mesh` instead, from P2)
- `crates/slicer-runtime/src/prepass.rs:280, 656, 730` — drop dispatcher-output handling and `BlackboardPrepassSlot::MeshSegmentation`
- `crates/slicer-ir/src/slice_ir.rs:1053-1086, 238-…` — drop `FacetPaintMark`, `MeshSegmentationIR`, schema constant
- `crates/slicer-ir/src/stage_io.rs:30-31, 262-…` — drop `PrepassStageOutput::MeshSegmentation`
- Test deletions:
  - unit: `crates/slicer-core/tests/algo_mesh_segmentation_tdd.rs` — KEEP (tests the host kernel from P2)
  - contract: `crates/slicer-runtime/tests/contract/macro_mesh_segmentation_output_roundtrip_tdd.rs` — DELETE
  - executor: `crates/slicer-runtime/tests/executor/mesh_segmentation_executor_tdd.rs` — KEEP, rewire to test host built-in (no WASM roundtrip)
  - integration: `crates/slicer-runtime/tests/integration/macro_mesh_segmentation_geometry_tdd.rs` — DELETE
  - module-local: `modules/core-modules/mesh-segmentation/tests/mesh_segmentation_tdd.rs` — deleted with the module
  - dispatch contract surface in `crates/slicer-runtime/tests/contract/dispatch_tdd.rs:282, 4771-5074, 6187-…` — delete those arms
- `crates/pnp-cli/src/module_new.rs:388, 521, 569, 571, 681` — drop scaffolder template arm
- `crates/pnp-cli/tests/module_new_tdd.rs:136` — drop test asserting scaffolder accepts `PrePass::MeshSegmentation`
- `crates/slicer-scheduler/tests/contract/core_module_ir_access_contract_tdd.rs:43, 233` and `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs:653` — drop stage from canonical-stages table
- `crates/slicer-runtime/benches/wasm_modules.rs:89` — drop bench entry

**What survives**: the `host:mesh_segmentation` stage_id, the `MESH_SEGMENTATION_PRODUCER`
constant from P2, the host kernel `execute_mesh_segmentation` in slicer-core,
and the existing tests for them.

**Verification**:

```bash
# After deleting macro arm, force guest rebuild:
cargo xtask build-guests
cargo xtask build-guests --check  # must report fresh

cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace 2>&1 | tee target/test-output.log

# Confirm only intended `MeshSegmentation` references survive:
rg -n 'MeshSegmentation' crates/ modules/
#   expect: only host BuiltinProducer + host:mesh_segmentation stage_id + kernel fn

# Confirm WIT no longer exports the resource:
rg -n 'mesh-segmentation-output|run-mesh-segmentation' crates/slicer-schema/
#   expect: 0 matches
```

---

### P5b — Loader symmetry: paint_seam + paint_fuzzy_skin sub-facet decoding

**Goal**: the 3MF loader at `crates/slicer-model-io/src/loader.rs:1119-1295`
decodes sub-facet strokes for `paint_color` (Material) and `paint_supports`
(SupportEnforcer/Blocker) but NOT for `paint_seam` or `paint_fuzzy_skin`. Same
hex codec applies. Fix the asymmetry.

**Change surface**:

- Hoist the stroke-decoding block from `loader.rs:1237-1295` into a
  `decode_strokes_for_channel(hex, semantic_mapper, tri_verts, byte_offset) -> Vec<PaintStroke>` helper.
- Call it for all four channels (`paint_color`, `paint_supports`, `paint_seam`, `paint_fuzzy_skin`)
  with the appropriate `PaintSemantic` mapping.
- After P2 is wired, strokes from any channel are normalized into `facet_values`
  by `host:mesh_segmentation` before paint-segmentation sees them.

**Test additions**: per-channel stroke tests in `model_loader_tdd.rs`. Use
existing cube fixtures or author tiny per-channel sub-facet fixtures if needed.

**Critical files**:

- `crates/slicer-model-io/src/loader.rs:1119-1295` — hoist + symmetrize
- `crates/slicer-model-io/tests/model_loader_tdd.rs` — per-channel stroke tests

**Verification**:

```bash
cargo test -p slicer-model-io 2>&1 | tee target/test-output.log
cargo test -p slicer-runtime --test executor cube_fuzzy_painted_tdd 2>&1 | tee target/test-output.log
```

---

### P5c — Doc updates

**Goal**: bring `docs/` into line with the new pipeline shape. Run last; preceding
packets leave docs intentionally out-of-sync during implementation.

**Updates**:

- `docs/01_system_architecture.md` — rewrite the prepass-order section to describe
  the new sequence (mesh_segmentation → mesh_analysis → user → region_mapping → slice → shell_classification → paint_segmentation → support_geometry).
  Add the variant-chain region-splitting model. Remove the obsolete
  `PrePass::MeshSegmentation [new — runs first]` block that described an unwired stage.
- `docs/02_ir_schemas.md` — bump `SliceIR` and `RegionMapIR` to 2.0.0.
  Document the variant_chain field on `RegionKey` and on `SlicedRegion`.
  Document `segment_annotations` (replaces `boundary_paint`).
  Document `ConfigId(u32)` + the `configs: Vec<ResolvedConfig>` interner.
  Remove `PaintRegionIR`, `LayerPaintMap`, `SemanticRegion`, `PaintRegionRTreeIndex`,
  `MeshSegmentationIR`, `FacetPaintMark`. Note the `PaintValue::Vector` deferred follow-up.
- `docs/03_wit_and_manifest.md` — add the `[[region_split]]` manifest schema
  section with the priority registry, the value-type validation rules, and
  the cross-manifest aggregation behavior. Remove the obsolete
  `mesh-segmentation-output` WIT resource documentation.
- `docs/04_host_scheduler.md` — update the stage-prerequisites table:
  - `PrePass::MeshSegmentation` → no prerequisites; produces MeshIR via replace_mesh
  - `PrePass::PaintSegmentation` → SliceIR, RegionMapIR; produces split SliceIR
    via replace_slice_ir
  Document the host-filtered dispatch contract. Remove the "guard-based
  fallback contract" sentence for paint-segmentation (the guest path is
  deleted in P5a).
- `docs/07_implementation_status.md` — mark paint-segmentation parity, mesh-segmentation
  wiring, region-splitting IR, and Phase 5 as implemented. Flag the three
  follow-ups (community paint ingestion, PaintValue::Vector, host:raw_slice).
- `docs/08_coordinate_system.md` — add the constant-conversion table from spec §5.
- `docs/specs/orca-paint-segmentation-parity.md` — flip `Status:` from
  `awaiting Slice Rework` to `implemented`. Don't delete; it's the historical record.
- `CONTEXT.md` — already updated during planning (variant chain, painted variant,
  region-split semantic, segment annotation; "region" ambiguity expanded).

---

## Risks and Mitigations

| Risk | Likelihood | Mitigation |
|---|---|---|
| `boostvoronoi` API doesn't match spec assumptions | Medium | P3 sub-step 7 starts with a 1-day API spike; if it fails, fall back to `spade` + custom Voronoi-graph wrapper, or cxx-bridge to OrcaSlicer's boost::polygon::voronoi. Document in `docs/specs/orca-paint-segmentation-parity.md` open Q5. |
| Per-semantic Voronoi pass count balloons on contrived inputs | Low | Spec §6 threading model + Rayon par_iter limits real-world overhead. Hard cap not needed; document expected scaling. |
| Modifier-volume short-circuit path (D14) diverges from main paint pipeline | Medium | Bind modifier-volume polygons into `segment_annotations` using the same per-contour-point shape downstream readers already use. Unit test mixing modifier-volume SupportEnforcer + facet Material on the same layer. |
| Phase 6 `slice_mesh_slabs` more involved than expected | Medium | Spec §3 Phase 6 specifies the algorithm. Sub-step 10 is separately landable; verifiable with `cube_4color_top_face_two_tool_indices_requires_projection_coverage` RED test. |
| Phase 5 width-limiting interacts badly with downstream perimeter generation | Medium | Visual-inspection step via `pnp_cli --report` on cube_4color + `regression_wedge.stl` during P4 verification. |
| Removing `PaintRegionIR` breaks a consumer I missed | Low | Exploration mapped consumers: only `slice_postprocess.rs:173-220`. P3 sub-step 17 deletes that call site alongside the type. Run `rg -nl 'PaintRegionIR\|PaintRegionRTreeIndex\|point_in_paint_region' crates/` after the delete; expect 0. |
| P0a/P0b fixture migration misses a test whose assertion needs benchy shape | Low | Audit Finding #9 classified all 42 benchy.stl tests. The new `regression_wedge.stl` is engineered to satisfy each SHAPE-DEPENDENT class. If any test genuinely breaks, keep one `#[ignore = "manual real-world regression"]` benchy holdout rather than weakening assertions. |
| `Blackboard::replace_mesh` changes the determinism contract | Low | `replace_slice_ir` precedent at `blackboard.rs:276-290` establishes the pattern: `debug_assert!` no downstream Tier 2 slot is committed. Mesh-segmentation runs first, guard is vacuous in production but catches reordering bugs at debug time. |
| WASM mesh-segmentation deletion (97 files) misses a hidden reference | Medium | Sub-step verification greps the whole tree post-delete: `rg -n 'MeshSegmentation\|mesh-segmentation\|mark_triangle_paint\|MeshSegmentationIR\|FacetPaintMark'`. Anything surviving is either intentional (host stage_id, kernel fn names) or a missed ref. Add a CI grep test if needed. |
| Config interning (D10) creates a subtle pointer-equality bug | Low | `ConfigId(u32)` is a value type, not a pointer; no aliasing risk. `Hash for ResolvedConfig` documented as "consistent within one process; cross-architecture not portable" (only matters for serialized snapshots). |
| Test cardinality grows under D15 (emit-empty-variants) | Low | Empty-variant `RegionPlan` entries cost ~100 bytes each with config interning. 170k empty entries = ~17 MB; acceptable. Tests assert on variant_chain presence, not entry count. `DEFAULT_REGION_MAP_CAP` raised to 750k with overflow diagnostics. |

---

## End-to-End Verification

After all packets land:

```bash
# 1. Workspace builds clean
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings

# 2. Guest WASMs fresh (no PrePass::MeshSegmentation refs remain)
cargo xtask build-guests --check

# 3. Acceptance gate: every RED test in the cherry-pick is GREEN
cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 | tee target/test-output.log
cargo test -p slicer-runtime --test executor cube_fuzzy_painted_tdd 2>&1 | tee target/test-output.log
grep "^test result" target/test-output.log
#   expected: 24 passed total

# 4. Stroke consumption: sub-facet hex in 3MF normalizes via mesh-segmentation
cargo test -p slicer-runtime --test executor mesh_segmentation 2>&1 | tee target/test-output.log

# 5. RegionMapping isolation: per-object AABB gates paint overlay
cargo test -p slicer-core --lib region_mapping 2>&1 | tee target/test-output.log

# 6. Phase 5 width limiting: banded extruder regions at the configured width
cargo test -p slicer-runtime --test executor width_limit 2>&1 | tee target/test-output.log

# 7. Variant-chain region-splitting behaves end-to-end
cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 | tee target/test-output.log
#   each painted face produces its own RegionKey variant entry

# 8. Modifier-volume + facet-paint co-existence
cargo test -p slicer-runtime --test executor modifier_volume 2>&1 | tee target/test-output.log

# 9. Wall-clock improvement from P0a + P0b
cargo clean -p slicer-runtime
time cargo test -p slicer-runtime 2>&1 | tee target/test-output.log

# 10. Workspace acceptance gate
cargo test --workspace 2>&1 | tee target/test-output.log
grep "^test result" target/test-output.log

# 11. No surviving benchy references
rg -n 'benchy_4color\.3mf|benchy_painted\.3mf|benchy\.stl' crates/ modules/ docs/ .ralph/  # expect: 0

# 12. No surviving WASM mesh-segmentation surface
rg -n 'mark_triangle_paint|MeshSegmentationIR|FacetPaintMark|run-mesh-segmentation' crates/ modules/  # expect: 0

# 13. Visual: HTML report on cube_4color shows the 4 colors on the 4 painted faces
cargo run --bin pnp_cli --release -- slice \
    --model resources/cube_4color.3mf \
    --module-dir modules/core-modules \
    --output /tmp/cube.gcode \
    --report /tmp/cube-report.html
```

**A successful close produces**:

- 24 cube RED tests as canonical paint regression coverage.
- Multi-minute reduction in workspace test wall-clock.
- `docs/specs/orca-paint-segmentation-parity.md` flipped `Status: implemented`.
- `CONTEXT.md` carrying the variant-chain, segment-annotation, painted-variant
  vocabulary.
- A consistent post-slice paint pipeline with no surviving placeholder kernels,
  dead WASM surfaces, or doc/code drift.

---

## Post-roadmap obligation — perimeter-module OrcaSlicer parity

The 11-packet roadmap closes with `external_contour`-driven single-trace outer walls for painted regions (D-96-AC22-EXTERNAL-CONTOUR). This is a parity-incomplete simplification — OrcaSlicer's MMU produces per-color outer wall fragments with tool changes at color transitions, not a union-traced single wall per object.

The reshape obligation is owned by [`docs/specs/perimeter-modules-orca-parity-roadmap.md`](./perimeter-modules-orca-parity-roadmap.md) under the "Inherited from P96 — AC-22b reshape obligation" section. Tasks T-P96-A through T-P96-F supersede this mechanism with per-color outer-wall fragmentation + deterministic per-edge bisector ownership.

Closing the 11-packet roadmap with status `implemented` on P96 is correct — Phase 5 work itself is real and complete. The AC-22b mechanism is documented debt with a binding cross-reference.
