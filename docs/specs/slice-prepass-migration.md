# Plan v4 (merged + verified): Promote Slicing to PrePass + Cross-Layer Shell Classification

> **Merge note**: This document combines Plan v3's full implementation detail (algorithm
> pseudocode, exact code snippets, complete test enumerations) with Plan v4's working-tree
> verification (corrected line numbers, three resolved decisions). Where v3 and v4 conflict,
> **v4's verified facts win**. The three resolved decisions — closing-radius wiring,
> `Option<u8>` shell index, and bottom-first precedence — are applied throughout, replacing
> v3's original `Option<u16>` / top-first / "slice_mesh_ex accepts closing radius" assumptions.

---

## Context

The current Tier 2 pipeline runs `Layer::Slice` and `Layer::SlicePostProcess` per-layer with no cross-layer visibility, by architectural design (`docs/01_system_architecture.md:175` — "Layers share no mutable state. The Blackboard is read-only during this tier."). This prevents OrcaSlicer-style top/bottom surface classification (a polygon-`diff` between adjacent layers, plus a depth-K "shrinking shadow" projection per `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp:1541,3928`) and forces cross-layer features into `PostPass::LayerFinalization` workarounds — see packet 38-rev1 `top-surface-ironing`, which currently scans `0..layers.len()` and falls back to bounding-box approximation at `top-surface-ironing/src/lib.rs:276-302`.

**Execution model**: solo developer, complete-run rewrite. No transitional state, no `#[deprecated]` migration, no incremental commits with broken pipeline output. Each commit lands a coherent, complete change. **Runs after packets 59 (`active`, `GCodeCommand::ExtrusionMode`) and 60 (`implemented`, `slice_closing_radius` round-trip + precision knobs)** — both confirmed landed and committed (Pre-Flight verified). Packet 60 added a separate `apply_slice_closing_radius(polygons, r)` helper but did **not** wire it into the slice path; this plan closes that gap.

This refactor:

- Moves slicing and cross-layer shell classification into PrePass via two new host built-ins.
- Replaces `is_top_surface: bool` / `is_bottom_surface: bool` on `SlicedRegion` with `top_shell_index: Option<u8>` / `bottom_shell_index: Option<u8>` plus polygon-precise `top_solid_fill: Vec<ExPolygon>` / `bottom_solid_fill: Vec<ExPolygon>`. Bool fields removed atomically — no deprecation window.
- Moves `top-surface-ironing` to `Layer::Infill` (the only stage exposing `slice-region-view` AND `infill-output-builder.push-ironing-path`).
- Replaces `PrePass::SupportGeometry`'s `collect_polygons_at_z` stub with `SliceIR` consumption.
- No stage rename. `Layer::SlicePostProcess` keeps its current name; new prepass stage is named `PrePass::ShellClassification` (semantic — describes what it computes; avoids the WIT-divergence half-rename trap from v2).

---

## Pre-Flight Findings (verified against working tree)

These corrections were ground-truthed against the actual codebase. All `file:line` references below use these verified values.

| Plan v3 claim | Reality | Action |
|---|---|---|
| `slice_mesh_ex` accepts `closing_radius` | WRONG. Signature is `slice_mesh_ex(mesh, zs) -> Vec<Vec<ExPolygon>>` at `crates/slicer-core/src/triangle_mesh_slicer.rs:48`. Packet 60 added a separate `apply_slice_closing_radius(polygons, r)` at line 394, but `execute_layer_slice` does NOT call it. | Wire `apply_slice_closing_radius` into the new prepass slice path (decision 1). |
| `top_shell_layers` / `bottom_shell_layers` are `u8` | WRONG. They are `u32` in `crates/slicer-ir/src/resolved_config.rs:433-435`. | Use `Option<u8>` for shell index; saturating cast on >255 (decision 2). |
| `polygon_opening` exists | WRONG. Compose via `offset(-r, Round, arc_tol)` then `offset(+r, Round, arc_tol)`. `offset` signature confirmed: `offset(&[ExPolygon], f32, OffsetJoinType, f32) -> Vec<ExPolygon>` at `polygon_ops.rs:185`. | Implement `apply_opening` helper inline. |
| `SlicedRegion` at slice_ir.rs:1145-1177 | WRONG. Struct at **1168-1199**; `is_top_surface` at **1186**, `is_bottom_surface` at **1189**. | Use real lines. |
| `CURRENT_SLICE_IR_SCHEMA_VERSION` at 165-169 | WRONG. Const at **184-188**; current value `2.1.0`. | Bump to `3.0.0`. |
| `__slicer_adapt_slice_regions` at 2494-2526 | WRONG. Function at **line 2515**; setter calls at **2541, 2542**. | Use real lines. |
| `SliceRegionData` field range at wit_host.rs:130-153 | Approximate. Struct keyword at **122**; trait accessors `is_top_surface`/`is_bottom_surface` at **3357-3364** (NOT 3387-3402). | Use real lines. |
| `RegionMapIR` types at slice_ir.rs:1077-1114 | WRONG range. `RegionKey` at **1099-1107**, `RegionPlan` at **1118-1127**, `entries: HashMap<RegionKey, RegionPlan>` at **1135**. | Use real lines. |
| `STAGE_ORDER` shape | CONFIRMED at execution_plan.rs:27-49; SupportGeometry at index 4. | Reorder as planned. |
| `BlackboardPrepassSlot` at 137-154 | Off-by-two. Enum at **139-156**; Display impl at **158-173**; Blackboard struct fields at **57-69**; `Blackboard::new` body at **187**. No `slice_ir` slot exists. | Use real lines. |
| `required_slots` at prepass.rs:562-580 | WRONG. Real location **658-676**. | Use real lines. |
| `ensure_stage_prerequisites` at prepass.rs:533-560 | WRONG. Real location **629-656**. | Use real lines. |
| SupportGeometry commit site at prepass.rs:485-512 | WRONG. `commit_support_geometry_builtin` invocation at **530-542**; early-guard region-mapping block at **483-507**, phase-2 region-mapping at **543-567**. | Insert Slice then ShellClassification between region-mapping and support-geometry. |
| `layer_executor.rs` per-layer slice block at 356-389 | Off-by-four. Real block at **360-400**; `execute_layer_slice` call at 373-381. | Use real lines. |
| `FILL_CLAIM_IDS` at execution_plan.rs:429-471 | WRONG file. Real location **validation.rs:11-16**; dedup keyed by `(stage_id, claim)`, so `claim:ironing` is collision-free. | Note correct file. |
| `classify_region_surfaces` bridge coupling | LOOSER than v3 claimed. Bridge detection uses `bridge_set` (facet index from prepass) at layer_slice.rs:224-238, NOT Z-window lookahead. Top/bottom logic at 180-222 is fully separable. | Slim cleanly — no special-case preservation. |
| `apply_slice_closing_radius` integrated | NO. Only called from tests. New prepass must wire it. | See Commit 2. |
| `support_layer_height_mm` validation exists | NO. None present in support_geometry.rs. | Add at config_schema.rs (Commit 4). |
| `StageInstrumentationGuard` exists | NO. Must be created in Commit 2. | Implement as RAII guard. |
| Files with `is_top_surface:` literal | **16** live files (v3 said 17). Includes `docs/specs/default-builder-migration.md`. `.ralph/specs/_OLD/12*/...` IGNORED (frozen). | Update all 16. |
| Packets 59 + 60 status | CONFIRMED `implemented` and committed. | No blocker. |
| OrcaSlicer line 4001 "goto EXTERNAL" | WRONG. Line 4001 is a comment inside `discover_horizontal_shells`; no goto. The break-on-empty semantic is still in that function, just at a different line. | Re-derive break point on read. |
| `OrcaSlicerDocumented/` vendored tree | CONFIRMED in-repo at the cited paths. | OK. |

**Degenerate polygon behavior** (write three throwaway unit tests against the actual polygon-op functions BEFORE A6): `intersection(&[], &[poly])` returns `Vec::new()`; `difference(&[poly], &[])` returns `vec![poly]`; `union(&[], &[])` returns `Vec::new()`. These guard A6's algorithm correctness; if any deviates, the algorithm needs special-casing.

**Re-verify before edit**: even with these corrections, intervening commits may shift lines. Re-read each target file BEFORE editing it.

---

## Resolved Decisions (from clarifying questions)

1. **Closing radius**: Wire `apply_slice_closing_radius` into the new prepass slice path in Commit 2. Closes a real packet-60 gap.
2. **Shell-index type**: `Option<u8>`. Config values >255 saturate (extreme edge case; document in `DEVIATION_LOG.md`).
3. **Top/bottom precedence on overlap**: Flip `rectilinear-infill` and `gyroid-infill` to bottom-first (OrcaSlicer parity). Log behavior change in `DEVIATION_LOG.md`.

---

## Architectural Decisions

1. **Two new PrePass host built-ins**: `PrePass::Slice` (was `Layer::Slice`) and `PrePass::ShellClassification` (NEW — OrcaSlicer-style two-pass top/bottom shell classification).
2. **No stage rename**: `Layer::SlicePostProcess` stays. New stage is `PrePass::ShellClassification` (not `PrePass::SlicePostProcess`) — avoids the v2 half-rename WIT divergence entirely.
3. **Atomic IR field swap**: `is_top_surface: bool` / `is_bottom_surface: bool` REMOVED in the same commit that adds shell-index/solid-fill fields. Two consumer modules (gyroid-infill, rectilinear-infill) plus all `SlicedRegion` struct-literal test fixtures migrate in the same commit. No `#[deprecated]`, no `#![allow(deprecated)]` crate-root annotations, no legacy bool maintenance pass.
4. **Algorithm authority**: `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp:1541-1892` (`detect_surfaces_type` — pass 1: depth-0 diff + opening anti-sliver) and lines 3928-4132 (`discover_horizontal_shells` — pass 2: shrinking-shadow projection at depths 1..K-1). Unit conversion: 1 unit = 100 nm (`docs/08_coordinate_system.md`); divide OrcaSlicer's `scaled_*` values by 100.
5. **A6 write model = build-immutably + commit-atomically**:
   - `PrePass::Slice` builds a `Vec<SliceIR>` from reads only, commits via `Blackboard::commit_slice_ir(Arc::new(vec))`.
   - `PrePass::ShellClassification` reads the committed Vec immutably, builds a NEW `Vec<SliceIR>` with shell-index/solid-fill populated, atomically replaces via `Blackboard::replace_slice_ir(Arc::new(new_vec))`.
   - **No `Arc::get_mut`. No `Arc::make_mut`. No in-place mutation.** Transactional rollback is free (partial new-vec is dropped on error; old vec stays committed). Per-region rayon parallelism is enabled naturally (disjoint writes to slots in the new vec).
   - No `slice_classification_complete` flag. Completeness is implicit in the data (replacement happened or it didn't).
6. **Cross-layer walk iterates per-region-timeline**, not per-global-layer. For each `(object_id, region_id)`, compute the ordered list of `global_layer_index` where the region is active. Catch-up layers and per-object layer-height differences resolve naturally because the walk follows the region's own occurrences.
7. **`Blackboard` extension**:
   - `slice_ir: Option<Arc<Vec<SliceIR>>>` slot.
   - `commit_slice_ir`: dup-commit error on second call (no idempotent early-return — bugs surface loudly).
   - `replace_slice_ir`: legal only when `slice_ir` is already `Some` AND no layer output has been committed yet (debug-asserted via `layer_outputs.as_ref().is_some_and(|v| v.iter().all(Option::is_none))`).
   - New `BlackboardPrepassSlot::SliceIR` variant.
8. **STAGE_ORDER (final shape, landed in one shot — no intermediate state)**:
   ```
   MeshSegmentation, MeshAnalysis, LayerPlanning, SeamPlanning, PaintSegmentation,
   RegionMapping, Slice, ShellClassification, SupportGeometry,
   Layer::Slice REMOVED, Layer::SlicePostProcess unchanged, ...
   ```
   SupportGeometry moves from index 4 to the end of prepass in the same commit that adds Slice/ShellClassification. No "reorder later" step.
9. **`PrePass::Slice` does NOT list `SliceIR` as its own prerequisite** in `required_slots()`. The self-slot pattern in `PrePass::SupportGeometry` exists for downstream user-module satisfaction (built-in commits before user modules run) and is a deliberate guard that blocks WASM override — **leave it intact**. `PrePass::Slice` and `PrePass::ShellClassification` are host-only — no user modules register on them. Listing self creates a false-prerequisite that confuses `ensure_stage_prerequisites`.
10. **`top-surface-ironing` → `Layer::Infill`**:
    - Declares `claim:ironing` (NEW string; not in `FILL_CLAIM_IDS` at `validation.rs:11-16`; dedup keyed by `(stage_id, claim)` so no collision).
    - Does NOT declare `infill-generator` (would dedup-collide with the surviving infill module).
    - Implements `run_infill` (takes `slice-region-view`, writes to `infill-output-builder.push_ironing_path`).
    - Self-gates per-region: `if !config.ironing_enabled || region.top_shell_index() != Some(0) || region.top_solid_fill().is_empty() { continue; }`. Module runs on every layer; gating is internal.
    - `ExtrusionRole::Ironing` is ungated by `should_emit` (`crates/slicer-sdk/src/views.rs:312-324` — falls through `_ => true`); `claim:ironing` is dedup-only.
11. **`support_layer_height_mm` validation**:
    - Per-object: compare against THAT object's minimum effective layer height, not the global min. Multi-object prints with different layer heights validate independently.
    - Validation at `config_schema.rs` only. `support_geometry.rs` trusts sanitized input.
    - Hard-reject finer-than-model with structured error: `"support_layer_height_mm={resolved} is finer than object '{object_id}' minimum layer height {object_min}; must be >= {object_min}. Use 0.0 to match per-object layer height."`. No misleading `support_top_z_distance_mm` suggestion.
12. **`collect_polygons_at_z` returns upper-layer polygons** (not union) for non-aligned Z. Union overbuilds support on tapered walls. Upper-layer-only is conservative for support pillars (catches the overhang above). Documented as deviation in `docs/DEVIATION_LOG.md`.
13. **Schema version**: `CURRENT_SLICE_IR_SCHEMA_VERSION` bumps to `3.0.0` in Commit 1 (single bump). Breaking change is intentional (bool fields removed).
14. **Instrumentation**: both new built-ins emit `on_stage_start` / `on_module_start` / `on_module_end` / `on_stage_end` via a new `StageInstrumentationGuard` RAII guard (ensures `*_end` fires on error paths). Unique module IDs: `host:slice` and `host:shell_classification`. `support_geometry` built-in also gets a unique ID (`host:support_geometry`) in the same commit to avoid `<host-built-in>` collision in the slicer-report HTML.
15. **Shell-depth type**: `Option<u8>` per resolved decision 2. Config `top_shell_layers`/`bottom_shell_layers` are `u32`; cast saturates at `u8::MAX` (255). Config validation should reject/warn on saturation; the IR carries the saturated value. (v3 originally proposed `u16`; superseded.)

---

## Commit 1: IR Schema + Scaffolding (atomic)

Single coherent change. Build must compile and all tests pass at HEAD. Pipeline produces sparse-only infill (no top/bottom solid) between Commit 1 and Commit 2 — acceptable for the complete-run, no end-user impact.

### Scope

**IR** (`crates/slicer-ir/src/slice_ir.rs`):
- `SlicedRegion` struct at lines **1168-1199**: **delete** `is_top_surface: bool` (line 1186) and `is_bottom_surface: bool` (line 1189). Add:
  ```rust
  #[serde(default)]
  pub top_shell_index: Option<u8>,
  #[serde(default)]
  pub bottom_shell_index: Option<u8>,
  #[serde(default)]
  pub top_solid_fill: Vec<ExPolygon>,
  #[serde(default)]
  pub bottom_solid_fill: Vec<ExPolygon>,
  ```
- Bump `CURRENT_SLICE_IR_SCHEMA_VERSION` at lines **184-188** to `SemVer { major: 3, minor: 0, patch: 0 }`.

**SDK** (`crates/slicer-sdk/src/views.rs`):
- `SliceRegionView` struct (**19-50**): delete `is_top_surface`, `is_bottom_surface` fields. Add the four new fields (shell indices as `Option<u8>`).
- `Default` impl (**52-76**): drop the two bool defaults, add the four new defaults (`None`, `None`, `Vec::new()`, `Vec::new()`).
- Setters (**148-160**): delete `set_is_top_surface` / `set_is_bottom_surface`. Add `set_top_shell_index`, `set_bottom_shell_index`, `set_top_solid_fill`, `set_bottom_solid_fill`.
- Readers (**197-207**): delete `is_top_surface()` / `is_bottom_surface()`. Add `top_shell_index() -> Option<u8>`, `bottom_shell_index() -> Option<u8>`, `top_solid_fill()`, `bottom_solid_fill()`.

**Macro bridge** (`crates/slicer-macros/src/lib.rs`):
- `__slicer_adapt_slice_regions` (function at **line 2515**): delete `set_is_top_surface(r.is_top_surface())` (line 2541) and `set_is_bottom_surface(r.is_bottom_surface())` (line 2542). Add four new setter calls for the new fields.

**WIT** (`wit/deps/ir-types.wit`):
- `resource slice-region-view` (**65-92**): delete `is-top-surface` and `is-bottom-surface` functions. Add:
  ```wit
  top-shell-index:    func() -> option<u8>;
  bottom-shell-index: func() -> option<u8>;
  top-solid-fill:     func() -> list<ex-polygon>;
  bottom-solid-fill:  func() -> list<ex-polygon>;
  ```
- `push-ironing-path` already exists at **wit/deps/ir-types.wit:113**; no WIT changes needed for ironing.

**WIT host bridge** (`crates/slicer-host/src/wit_host.rs`):
- `SliceRegionData` struct (starts line **122**): delete two bool fields. Add four new fields with matching types (`Option<u8>` for shell indices).
- IR → WIT struct literal: search for the `is_top_surface:` copy site and replace with four new field copies.
- Trait accessor impls at lines **3357-3364**: delete `is_top_surface` and `is_bottom_surface` method bodies. Add four new method bodies. Each pushes `"SliceIR"` to `runtime_reads` and returns the field.

**Scheduler scaffolding**:
- `crates/slicer-host/src/execution_plan.rs:27-49` (`STAGE_ORDER`): final shape (one edit, no intermediate state):
  ```rust
  pub const STAGE_ORDER: &[&str] = &[
      "PrePass::MeshSegmentation",
      "PrePass::MeshAnalysis",
      "PrePass::LayerPlanning",
      "PrePass::SeamPlanning",
      "PrePass::PaintSegmentation",
      "PrePass::RegionMapping",
      "PrePass::Slice",                 // NEW
      "PrePass::ShellClassification",   // NEW
      "PrePass::SupportGeometry",       // MOVED from index 4 to here
      "Layer::Perimeters",              // Layer::Slice REMOVED
      "Layer::SlicePostProcess",        // unchanged
      "Layer::PerimetersPostProcess",
      "Layer::Infill",
      "Layer::InfillPostProcess",
      "Layer::Support",
      "Layer::SupportPostProcess",
      "Layer::PathOptimization",
      "PostPass::LayerFinalization",
      "PostPass::GCodeEmit",
      "PostPass::GCodePostProcess",
      "PostPass::TextPostProcess",
  ];
  ```
- `crates/slicer-host/src/dispatch.rs:46-68` (`export_name_for_stage()`): add `"PrePass::Slice" => None` and `"PrePass::ShellClassification" => None` (host built-ins). Remove `"Layer::Slice" => Some("run-slice")` mapping (line 54). `"Layer::SlicePostProcess" => Some("run-slice-postprocess")` stays.
- `crates/slicer-host/src/blackboard.rs`:
  - `BlackboardPrepassSlot` enum (**139-156**): add `SliceIR` variant. Update `Display` impl (**158-173**) with `SliceIR => "slice-ir"`.
  - Struct fields (**57-69**): add `slice_ir: Option<Arc<Vec<SliceIR>>>`.
  - `Blackboard::new` body (**178-192**, init at ~187): init `slice_ir: None`.
  - Add accessors `slice_ir(&self) -> Option<Arc<Vec<SliceIR>>>` and `commit_slice_ir(&mut self, ir) -> Result<(), BlackboardError>`. (`replace_slice_ir` lands in Commit 2 — see its signature there. Commit 1 only needs `commit_slice_ir`, used in Commit 2's prepass executor.)
- `crates/slicer-host/src/prepass.rs`:
  - `required_slots()` (**658-676**): add
    ```rust
    "PrePass::Slice" => &[
        BlackboardPrepassSlot::SurfaceClassification,
        BlackboardPrepassSlot::LayerPlan,
        BlackboardPrepassSlot::RegionMap,
        // SliceIR NOT listed — this stage WRITES it, no self-prereq.
    ],
    "PrePass::ShellClassification" => &[
        BlackboardPrepassSlot::SurfaceClassification,
        BlackboardPrepassSlot::LayerPlan,
        BlackboardPrepassSlot::RegionMap,
        BlackboardPrepassSlot::PaintRegions,
        BlackboardPrepassSlot::SliceIR,
    ],
    ```
  - `ensure_stage_prerequisites` (**629-656**): add `BlackboardPrepassSlot::SliceIR => blackboard.slice_ir().is_some()` to the match.

**Consumer modules** (must compile after Commit 1) — **bottom-first precedence (decision 3)**:
- `modules/core-modules/rectilinear-infill/src/lib.rs:142-148`: flip to bottom-first:
  ```rust
  let role = if region.bottom_shell_index().is_some() {
      ExtrusionRole::BottomSolidInfill
  } else if region.top_shell_index().is_some() {
      ExtrusionRole::TopSolidInfill
  } else {
      ExtrusionRole::SparseInfill
  };
  ```
- `modules/core-modules/gyroid-infill/src/lib.rs:~132-134`: same bottom-first flip pattern.
- `modules/core-modules/rectilinear-infill/tests/top_bottom_fill_tdd.rs:~56,57`: `set_is_top_surface(true)` → `set_top_shell_index(Some(0))` + `set_top_solid_fill(vec![<fixture polygon>])`. Where tests assert role for top-only fixtures, also set non-empty `top_solid_fill`. **Add a test case asserting bottom-first precedence on overlap** (region with both indices `Some(0)` → expects `BottomSolidInfill`).

**Test fixtures** — all **16** live files identified in Pre-Flight update struct literals. Pattern: remove `is_top_surface: ...,` and `is_bottom_surface: ...,` lines. New fields default via `..Default::default()` if used, else explicit `top_shell_index: None,` etc. The 16 live files:
- `crates/slicer-ir/src/slice_ir.rs` (struct def)
- `crates/slicer-ir/tests/ir_tests.rs`
- `crates/slicer-sdk/src/views.rs`
- `crates/slicer-host/src/wit_host.rs`
- `crates/slicer-host/tests/dispatch_tdd.rs`
- `crates/slicer-host/tests/macro_all_worlds_roundtrip_tdd.rs`
- `crates/slicer-host/tests/wit_boundary_tdd.rs`
- `crates/slicer-host/tests/live_layer_support_tdd.rs`
- `crates/slicer-host/tests/benchy_4color_modifier_part_e2e_tdd.rs`
- `crates/slicer-host/tests/threemf_subtypes_synthetic_e2e_tdd.rs`
- `crates/slicer-host/tests/slice_postprocess_paint_annotation_tdd.rs`
- `crates/slicer-host/tests/bridge_detector_tdd.rs`
- `docs/02_ir_schemas.md` (doc reference)
- `docs/specs/default-builder-migration.md` (doc reference)
- `modules/core-modules/rectilinear-infill/tests/top_bottom_fill_tdd.rs`
- (struct-def site within slice_ir.rs counted once)
- `.ralph/specs/_OLD/12*/...` — **IGNORE** (historical packets, frozen).

**Scheduler contract test** (`crates/slicer-host/tests/core_module_ir_access_contract_tdd.rs:39-73`): add
```rust
"PrePass::Slice" => Some((
    &["MeshIR", "SurfaceClassificationIR", "LayerPlanIR", "RegionMapIR"],
    &["SliceIR"],
)),
"PrePass::ShellClassification" => Some((
    &["SliceIR", "RegionMapIR", "PaintRegionIR", "LayerPlanIR"],
    &["SliceIR"],
)),
```
Remove `"Layer::Slice"` (no longer exists).

**Validation/manifest audit**:
- `crates/slicer-host/src/validation.rs:887` — drop `"Layer::Slice"` from `stage_order_index`; keep `"Layer::SlicePostProcess"` at 888. (`FILL_CLAIM_IDS` at 11-16 and dedup logic at 430-473 confirm `claim:ironing` is collision-free — audit only, no edit.)
- `crates/slicer-host/src/manifest.rs:~1041` — drop `"Layer::Slice"` from `known_stage_ids`. Test occurrences at **1144, 1152** update or delete as appropriate. Keep `"Layer::SlicePostProcess"`.

**Schema version assertion** (`crates/slicer-ir/tests/ir_tests.rs:647-655`): bump literal from `SemVer { major: 2, minor: 1, patch: 0 }` to `SemVer { major: 3, minor: 0, patch: 0 }`. Add roundtrip assertions for the four new fields.

**Deviation log** (`docs/DEVIATION_LOG.md`): bottom-first precedence flip; `Option<u8>` saturation at 255.

**Doc updates** (`docs/02_ir_schemas.md`): update `SlicedRegion` field list.

### Guest WASM

WIT changed → STALE for ALL guests. Run `cargo xtask build-guests` (full build, NOT `--check`). Inspect output for type errors from the WIT cascade.

### Verify

```
cargo build --workspace
cargo clippy --workspace -- -D warnings
cargo test -p slicer-ir -p slicer-sdk -p slicer-macros
cargo test -p slicer-host --test core_module_ir_access_contract_tdd --test prepass_executor_tdd
cargo test -p gyroid-infill -p rectilinear-infill
cargo xtask build-guests --check
```

---

## Commit 2: PrePass Slicing + Shell Classification (atomic)

Adds the two new host built-ins and removes the Tier-2 `Layer::Slice` built-in dispatch. Pipeline produces correct top/bottom solid infill after this commit lands.

### Scope

**File rename**: `git mv crates/slicer-host/src/layer_slice.rs crates/slicer-host/src/prepass_slice.rs`. Update `crates/slicer-host/src/lib.rs` module declaration.

**`prepass_slice.rs`** (renamed from `layer_slice.rs`):
- Rename `execute_layer_slice` (**lines 337-345**) → `execute_prepass_slice_single_layer`. Make `layer_plan` mandatory (drop the `Option` and the caller-supplied `next_layer_z`/`prev_layer_z` fallback path).
- **Wire closing radius (decision 1)** — packet-60 gap closure:
  ```rust
  let mut sliced = slice_mesh_ex(&object.mesh, &[layer.z]);
  let raw_polys = sliced.pop().unwrap_or_default();
  let polygons = if resolved.slice_closing_radius > 0.0 {
      apply_slice_closing_radius(raw_polys, resolved.slice_closing_radius)
  } else {
      raw_polys
  };
  ```
  Import `apply_slice_closing_radius` from `slicer_core::triangle_mesh_slicer` (defined at `triangle_mesh_slicer.rs:394`). Reads `slice_closing_radius` from `RegionPlan.config`; falls back to default when no region plan.
- **Slim `classify_region_surfaces`** (**lines 123-242**): drop the top/bottom Z-plane lookahead logic at **180-222** entirely. The function NOW returns `(bool /*is_bridge*/, Vec<ExPolygon> /*bridge_areas*/)`. Bridge detection at **224-238** stays untouched — it uses `bridge_set` (facet indices from prepass), NOT Z-window lookahead, so it is fully separable. No special-case preservation needed (Pre-Flight confirmed the coupling is looser than v3 feared).
- Add wrapper:
  ```rust
  pub fn execute_prepass_slice_all_layers(blackboard: &Blackboard) -> Result<Vec<SliceIR>, LayerSliceError> {
      let mesh = blackboard.mesh();
      let layer_plan = blackboard.layer_plan().ok_or(LayerSliceError::NoLayerPlan)?;
      let surface_class = blackboard.surface_classification().map(|a| a.as_ref());
      let region_map = blackboard.region_map().map(|a| a.as_ref());
      layer_plan.global_layers.iter()
          .map(|gl| execute_prepass_slice_single_layer(
              mesh.as_ref(), gl, surface_class, region_map, layer_plan.as_ref()
          ))
          .collect()
      // Per-layer rayon if profiling shows it's needed.
  }
  ```
- Add:
  ```rust
  pub fn commit_slice_builtin(blackboard: &mut Blackboard) -> Result<(), LayerSliceError> {
      let slices = execute_prepass_slice_all_layers(blackboard)?;
      blackboard.commit_slice_ir(Arc::new(slices))
          .map_err(LayerSliceError::Blackboard)
  }
  ```
  No idempotent early-return. Dup-commit surfaces as `BlackboardError::DuplicatePrepassCommit` — a bug, not a no-op.
- New error variant `LayerSliceError::Blackboard(#[from] BlackboardError)`. Existing `LayerSliceError` gets `#[derive(thiserror::Error, Debug)]` if not already; new variant has `#[error("blackboard error: {0}")]`.

**Memory note**: building all `Vec<SliceIR>` upfront costs ~50-100 KB per region per layer × thousands of (region × layer) pairs = tens of MB for typical prints. Acceptable on modern hardware. If problematic on large multi-object prints, future packets can introduce streaming or polygon pooling.

**`slice_postprocess_prepass.rs`** (NEW file, distinct from `slice_postprocess.rs` which stays unchanged):

```rust
//! Shell classification (PrePass::ShellClassification host built-in).
//!
//! Ports OrcaSlicer PrintObject.cpp:1541-1892 (detect_surfaces_type) and
//! :3928-4132 (discover_horizontal_shells). See docs/DEVIATION_LOG.md for
//! the documented divergences (hollow-object continue path, etc).

#[derive(thiserror::Error, Debug)]
pub enum ShellClassificationError {
    #[error("SliceIR not committed before shell classification")]
    SliceIRNotCommitted,
    #[error("RegionMap not committed before shell classification")]
    RegionMapNotCommitted,
    #[error("LayerPlan not committed before shell classification")]
    LayerPlanNotCommitted,
    #[error("blackboard error: {0}")]
    Blackboard(#[from] BlackboardError),
}

pub fn commit_shell_classification_builtin(
    blackboard: &mut Blackboard,
) -> Result<(), ShellClassificationError>;
```

**Algorithm** (two-pass, OrcaSlicer-faithful). Note shell indices are `u8`; the `k_top`/`k_bot` loop bounds derive from a saturating cast of the `u32` config:

```
fn commit_shell_classification_builtin(bb: &mut Blackboard):
  let old_vec = bb.slice_ir().ok_or(SliceIRNotCommitted)?.as_ref();
  let region_map = bb.region_map().ok_or(RegionMapNotCommitted)?;
  let layer_plan = bb.layer_plan().ok_or(LayerPlanNotCommitted)?;

  let mut new_vec: Vec<SliceIR> = old_vec.iter().cloned().collect();  // start from raw slices

  // For each (object_id, region_id), compute the timeline of layers where it's active.
  let region_timelines: HashMap<(ObjectId, RegionId), Vec<usize>> =
      build_region_timelines(layer_plan, old_vec);

  // Parallelize per (object, region) pair — disjoint writes into new_vec[i].regions[j].
  for ((obj, region_id), timeline) in region_timelines {
      // Saturating cast from u32 config to u8 shell depth (decision 2).
      let k_top: u8 = lookup_config_u32(region_map, &obj, region_id, "top_shell_layers")
                          .try_into().unwrap_or(u8::MAX);
      let k_bot: u8 = lookup_config_u32(region_map, &obj, region_id, "bottom_shell_layers")
                          .try_into().unwrap_or(u8::MAX);
      let ext_width_mm = lookup_extrusion_width_mm(region_map, &obj, region_id);
      let opening_offset_mm = ext_width_mm / 10.0;  // OrcaSlicer's "/ 10.f" on mm width

      // PASS 1: depth-0 diff classification (port of detect_surfaces_type:1577-1623).
      for (i_in_timeline, layer_idx) in timeline.iter().enumerate() {
          let r_polys = get_region_polys(&new_vec[*layer_idx], &obj, region_id);

          // Upper = next layer in this region's timeline (NOT slice_vec[layer_idx+1]).
          let upper_polys = timeline.get(i_in_timeline + 1)
              .map(|&up_idx| get_region_polys(&new_vec[up_idx], &obj, region_id))
              .unwrap_or_default();  // print-top: empty = fully exposed.

          let lower_polys = if i_in_timeline > 0 {
              get_region_polys(&new_vec[timeline[i_in_timeline - 1]], &obj, region_id)
          } else {
              Vec::new()  // print-bottom: fully exposed.
          };

          if k_top > 0 {
              let top_diff = polygon_ops::difference(&r_polys, &upper_polys);
              let top_solid_fill_0 = if opening_offset_mm > 0.0 {
                  apply_opening(&top_diff, opening_offset_mm)  // erode then dilate
              } else {
                  top_diff
              };
              if !top_solid_fill_0.is_empty() {
                  let r = get_region_mut(&mut new_vec[*layer_idx], &obj, region_id);
                  r.top_shell_index = Some(0);
                  r.top_solid_fill = top_solid_fill_0;
              }
          }

          // Symmetric bottom.
          if k_bot > 0 {
              let bot_diff = polygon_ops::difference(&r_polys, &lower_polys);
              let bot_solid_fill_0 = if opening_offset_mm > 0.0 {
                  apply_opening(&bot_diff, opening_offset_mm)
              } else {
                  bot_diff
              };
              if !bot_solid_fill_0.is_empty() {
                  let r = get_region_mut(&mut new_vec[*layer_idx], &obj, region_id);
                  r.bottom_shell_index = Some(0);
                  r.bottom_solid_fill = bot_solid_fill_0;
              }
          }
      }

      // PASS 2: shrinking-shadow projection (port of discover_horizontal_shells).
      // The break-on-empty check lives INSIDE the loop body (v3's "line 4001" cite was a
      // comment, not a goto — re-derive exact location on read). For each layer with
      // top_shell_index == Some(0), walk backwards along the timeline and stamp interior
      // shell layers with min-depth.
      for (i_in_timeline, layer_idx) in timeline.iter().enumerate() {
          let r = get_region(&new_vec[*layer_idx], &obj, region_id);
          if r.top_shell_index != Some(0) { continue; }

          let mut current_shadow = r.top_solid_fill.clone();
          for offset in 1..k_top.min(i_in_timeline as u8 + 1) {
              let n_in_timeline = i_in_timeline - offset as usize;
              let n_layer_idx = timeline[n_in_timeline];

              let neighbor_polys = get_region_polys(&new_vec[n_layer_idx], &obj, region_id);
              let new_shadow = polygon_ops::intersection(&current_shadow, &neighbor_polys);
              if new_shadow.is_empty() {
                  break;  // matches OrcaSlicer break-on-empty for hollow-object case.
                  // DEVIATION: we don't implement the "continue" path for sparse_infill_density > 0.
                  // Documented in DEVIATION_LOG.md.
              }

              // Always union into top_solid_fill[n] (multi-source contributions, flattened IR).
              // Update top_shell_index only when our depth improves the minimum.
              let n_r = get_region_mut(&mut new_vec[n_layer_idx], &obj, region_id);
              n_r.top_solid_fill = polygon_ops::union(&n_r.top_solid_fill, &new_shadow);
              n_r.top_shell_index = Some(match n_r.top_shell_index {
                  None => offset,
                  Some(existing) => existing.min(offset),
              });

              current_shadow = new_shadow;
          }
      }

      // Symmetric bottom projection: walk forward through timeline, same logic.
  }

  bb.replace_slice_ir(Arc::new(new_vec))?;
  Ok(())
```

Rayon: parallelize the outer per-`(object, region)` loop — writes are disjoint across `(slice_index, region_index)` slots. Use `par_iter` over a slot-index decomposition that pre-computes write targets; if safe disjointness is hard to express, accept a sequential outer pass with rayon at the inner polygon-op level.

**`Blackboard::replace_slice_ir`** signature (added in Commit 2):
```rust
pub(crate) fn replace_slice_ir(
    &mut self,
    ir: Arc<Vec<SliceIR>>,
) -> Result<(), BlackboardError> {
    if self.slice_ir.is_none() {
        return Err(BlackboardError::MissingRequiredPrepass {
            slot: BlackboardPrepassSlot::SliceIR,
        });
    }
    debug_assert!(
        self.layer_outputs.as_ref().is_some_and(|v| v.iter().all(Option::is_none)),
        "replace_slice_ir called after Tier 2 wrote a layer slot"
    );
    self.slice_ir = Some(ir);
    Ok(())
}
```

**Extrusion-width lookup fallback chain** (`lookup_extrusion_width_mm`):
1. Read `RegionPlan.config.perimeter_extrusion_width` (mm). If > 0, use it.
2. Else read `RegionPlan.config.line_width` (or `extrusion_width` — verify exact ResolvedConfig field name).
3. Else use `effective_layer_height * 1.2` (heuristic: nozzle ≈ layer_height for fine prints, 1.2× for typical).
4. Else 0.4 mm (standard nozzle).

Returns mm directly (the opening offset is `ext_width_mm / 10.0`, also mm). Document the fallback chain in `docs/DEVIATION_LOG.md` — OrcaSlicer reads `flow(frExternalPerimeter).scaled_width()` which has different fallback semantics.

**`apply_opening`** helper (since `polygon_opening` doesn't exist in `slicer-core::polygon_ops`):
```rust
fn apply_opening(polys: &[ExPolygon], offset_mm: f32) -> Vec<ExPolygon> {
    // Erode then dilate. Eliminates slivers narrower than 2×offset_mm.
    let arc_tol = 0.0125_f32;  // matches resolved_config default (packet-60 perimeter_arc_tolerance)
    let eroded = polygon_ops::offset(polys, -offset_mm, OffsetJoinType::Round, arc_tol);
    polygon_ops::offset(&eroded, offset_mm, OffsetJoinType::Round, arc_tol)
}
```
`polygon_ops::offset` takes `delta_mm: f32` (signature confirmed at `polygon_ops.rs:185`). The OrcaSlicer `/ 10.f` factor is on a scaled (nanometer) width; converting cleanly, `offset_mm = extrusion_width_mm / 10.0` is the mm offset directly. Add a unit test in `slicer-core::polygon_ops` against a sliver-fixture (width comparable to `external_perimeter_width / 5`) to confirm collapse behavior.

**Prepass executor wiring** (`crates/slicer-host/src/prepass.rs`): the existing `commit_support_geometry_builtin` invocation (**lines 530-542**) moves AFTER the two new built-ins. Insert between phase-2 region-mapping (**543-567**) and the support-geometry commit. Use the RAII guard so `on_stage_end` fires on the error path too:

```rust
{
    let _guard = StageInstrumentationGuard::new(instrumentation, "PrePass::Slice", "host:slice");
    if blackboard.slice_ir().is_none() {
        prepass_slice::commit_slice_builtin(blackboard)
            .map_err(|source| PrepassExecutionError::Slice { source })?;
    }
}
{
    let _guard = StageInstrumentationGuard::new(instrumentation, "PrePass::ShellClassification", "host:shell_classification");
    slice_postprocess_prepass::commit_shell_classification_builtin(blackboard)
        .map_err(|source| PrepassExecutionError::ShellClassification { source })?;
}
{
    let _guard = StageInstrumentationGuard::new(instrumentation, "PrePass::SupportGeometry", "host:support_geometry");
    if blackboard.support_geometry().is_none() && blackboard.layer_plan().is_some() {
        support_geometry::commit_support_geometry_builtin(blackboard)
            .map_err(/* existing error map */)?;
    }
}
```

The `support_geometry` commit also adopts the RAII guard with module ID `"host:support_geometry"` (replaces the bare `<host-built-in>` ID for consistent slicer-report attribution).

**`StageInstrumentationGuard`** (NEW): lightweight RAII wrapper. `new` fires `on_stage_start` + `on_module_start`; `Drop` checks `std::thread::panicking()` and fires `on_module_end(success = !panicking)` + `on_stage_end`. Confine to `prepass.rs` if it's the only user; promote to a shared util if `layer_executor.rs` adopts the pattern later.

Add error variants on `PrepassExecutionError`:
- `Slice { source: LayerSliceError }` — `#[error("PrePass::Slice failed: {source}")]`
- `ShellClassification { source: ShellClassificationError }` — `#[error("PrePass::ShellClassification failed: {source}")]`

**Layer executor cleanup** (`crates/slicer-host/src/layer_executor.rs:360-400`): delete the per-layer `Layer::Slice` built-in block (the `execute_layer_slice` call at 373-381 wrapped in `if arena.slice().is_none()`). Replace with a blackboard read:

```rust
if arena.slice().is_none() {
    let slice_vec = blackboard.slice_ir().ok_or_else(|| LayerExecutionError::FatalLayer {
        layer_index: layer.index,
        stage_id: "PrePass::Slice".to_string(),
        module_id: "host:slice".to_string(),
        message: "blackboard slice_ir empty when Tier 2 started".to_string(),
    })?;
    let slice = slice_vec.get(layer.index as usize).cloned().ok_or_else(|| LayerExecutionError::FatalLayer {
        layer_index: layer.index,
        stage_id: "PrePass::Slice".to_string(),
        module_id: "host:slice".to_string(),
        message: format!("slice_ir Vec missing entry for layer index {}", layer.index),
    })?;
    arena.set_slice(slice).map_err(|_| LayerExecutionError::FatalLayer {
        layer_index: layer.index,
        stage_id: "PrePass::Slice".to_string(),
        module_id: "host:slice".to_string(),
        message: "arena slice slot already occupied".to_string(),
    })?;
}
```

### Tests

New file `crates/slicer-host/tests/prepass_slice_tdd.rs`:
1. **Empty mesh** → `slice_ir.unwrap().len() == 0`, no panic.
2. **Single-layer mesh** → `len() == 1`, one SliceIR per region.
3. **Multi-object mixed layer heights** (0.2 + 0.3, catch-up at Z=0.6) → `len() == global_layers.len()`, indexing follows global position.
4. **Bridge preserved** — overhang fixture: `is_bridge = true`, non-empty `bridge_areas`, shell-index fields all `None`.
5. **Pre-Tier-2 commit** — `blackboard.slice_ir()` is `Some` immediately after `execute_prepass_with_builtins` returns.
6. **Dup-commit error** — calling `commit_slice_builtin` twice returns `LayerSliceError::Blackboard(DuplicatePrepassCommit)`.
7. **Closing-radius wired** (decision 1) — slice with `slice_closing_radius = 0.04`, two near-touching contours → fused into one ExPolygon. With radius `= 0.0`, not fused.

New file `crates/slicer-host/tests/prepass_shell_classification_tdd.rs` (15 cases):
1. **Stepped pyramid (K_top=3)** — top step `top_shell_index = Some(0)`; one below `Some(1)`; one below that `Some(2)`; deeper `None`. Polygon-area equality with 1e-6 mm² tolerance.
2. **Shrinking shadow** — wedge: `top_solid_fill[n+1].area < top_solid_fill[n].area` in the shell zone.
3. **K_top = 0** — classification no-op for top; bottom unaffected.
4. **Single-layer print** — single layer has BOTH `top_shell_index = Some(0)` AND `bottom_shell_index = Some(0)`; both fills equal `polygons`. Verify the downstream **bottom-wins** convention (decision 3) via a separate role-emission test on rectilinear-infill.
5. **K_top + K_bot > total_layers** — every layer has both indices set; no precedence at IR level.
6. **Inactive region** — region active at layers 0-3, absent at layer 4. Layer 3 sees upper-region as empty → `top_solid_fill = polygons(3)`. PASS.
7. **Print-top** — top layer has `top_shell_index = Some(0)`, `top_solid_fill = polygons(L)`.
8. **Catch-up timeline** — multi-object mixed heights; region A at Z=0.2, 0.4, 0.6 (skips global Z=0.3 catch-up); A's "upper" at Z=0.2 is at Z=0.4 (next active in timeline), NOT Z=0.3. Assert no spurious top classification at Z=0.2.
9. **Donut polygon** — region with a hole. `top_solid_fill` preserves the hole (ExPolygon handles hole geometry natively).
10. **Multi-island region** — region with 2 disconnected polygons at layer N, splits to 3 at layer N+1. Both source polygons project shadows independently.
11. **Region split** — single polygon at L splits to two at L-1. `top_solid_fill` at L-1 reflects the split shadow.
12. **Region merge** — two polygons at L+1 merge to one at L. Top diff at L (against unioned upper) yields any exposed sliver.
13. **Cross-region shadow boundary** — region A's shadow projects into B's polygon area at L-1. Verify A's shadow stamps only A's region in L-1 (no cross-region contamination).
14. **Atomic replace** — mid-pass panic injection (fault-injection feature or a test-only panic hook in `apply_opening`): assert `blackboard.slice_ir()` still returns the pre-A6 Vec, NOT a partially-mutated state.
15. **Missing PaintRegions** — omit PaintRegions commit; `ensure_stage_prerequisites` returns `MissingRequiredPrepass { slot: PaintRegions }`.

> Test fixture construction across these 15 cases is significant labor — allocate explicit implementation time. Test #14 needs a feature-gate or test-only panic hook in `apply_opening`; design before implementation.

### Verify

```
cargo build --workspace
cargo clippy --workspace -- -D warnings
cargo test -p slicer-host --test prepass_slice_tdd --test prepass_shell_classification_tdd --test prepass_executor_tdd --test prepass_support_geometry_tdd
cargo xtask build-guests --check
```

(`prepass_support_geometry_tdd` runs unchanged here — SupportGeometry still uses the old stub. Commit 4 fixes it.)

---

## Commit 3: Move `top-surface-ironing` to `Layer::Infill`

Full module rewrite. Removes `PostPass::LayerFinalization` registration; ironing now emits during `Layer::Infill` per-layer parallel dispatch.

### `modules/core-modules/top-surface-ironing/top-surface-ironing.toml`

Full rewrite:
```toml
[module]
id           = "com.core.top-surface-ironing"
version      = "0.2.0"
display-name = "Top Surface Ironing"
description  = "Emits ironing strokes over polygon-precise top solid fill areas at the topmost exposed surface of each region."
author       = "modular-slicer"
license      = "MIT"
wit-world    = "slicer:world-layer@1.0.0"

[stage]
id = "Layer::Infill"

[ir-access]
reads  = ["SliceIR"]
writes = ["InfillIR"]

[claims]
holds = ["claim:ironing"]

[hints]
estimated-ms-per-layer = 10    # to be benchmarked; see Risks.
layer-parallel-safe    = true

[config.schema.ironing_enabled]
type    = "bool"
default = true

[config.schema.ironing_pattern]
type    = "string"
default = "zigzag"

[config.schema.ironing_spacing_mm]
type    = "float"
default = 0.1
min     = 0.01
max     = 1.0

[config.schema.ironing_flow]
type    = "float"
default = 0.1
min     = 0.0
max     = 1.0

[config.schema.ironing_speed]
type    = "float"
default = 15.0
min     = 1.0
max     = 100.0
```

### `modules/core-modules/top-surface-ironing/src/lib.rs`

Full rewrite (~150 lines). Template: `modules/core-modules/gyroid-infill/src/lib.rs`. Skeleton:

```rust
use slicer_macros::slicer_module;
use slicer_sdk::traits::*;
use slicer_sdk::views::*;
use slicer_ir::ExtrusionRole;

pub struct TopSurfaceIroning;

#[slicer_module]
impl InfillModule for TopSurfaceIroning {
    fn on_print_start(&self, config: &ConfigView) -> Result<(), ModuleError> {
        let flow = config.get_float("ironing_flow")?;
        if flow <= 0.0 {
            return Err(ModuleError::config("ironing_flow must be > 0.0"));
        }
        let pattern = config.get_string("ironing_pattern")?;
        if pattern != "zigzag" {
            return Err(ModuleError::config(format!("ironing_pattern '{}' unsupported (only 'zigzag')", pattern)));
        }
        Ok(())
    }

    fn run_infill(
        &self,
        _layer_index: u32,
        regions: &[SliceRegionView],
        output: &mut InfillOutputBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        if !config.get_bool("ironing_enabled")? { return Ok(()); }
        let spacing = config.get_float("ironing_spacing_mm")? as f32;
        let flow = config.get_float("ironing_flow")? as f32;
        let speed = config.get_float("ironing_speed")? as f32;

        for region in regions {
            if region.top_shell_index() != Some(0) { continue; }
            let fill_polys = region.top_solid_fill();
            if fill_polys.is_empty() { continue; }
            let z = region.z();

            let unioned = polygon_ops::union(&fill_polys, &[]);
            for poly in &unioned {
                for stroke in generate_zigzag_strokes_for_polygon(poly, z, spacing, flow, speed) {
                    output.push_ironing_path(stroke)?;
                }
            }
        }
        Ok(())
    }
}

fn generate_zigzag_strokes_for_polygon(
    poly: &ExPolygon,
    z: f32,
    spacing_mm: f32,
    flow: f32,
    speed: f32,
) -> Vec<ExtrusionPath3D> {
    // 1. Compute axis-aligned bounding box of the polygon contour.
    // 2. Generate horizontal zigzag scan lines at `spacing_mm` intervals across the bbox.
    // 3. For each scan-line segment, polygon_ops::intersection(segment, poly) to clip
    //    to the polygon interior (handles concave / multi-island polygons).
    // 4. Each clipped segment becomes an ExtrusionPath3D with role = Ironing,
    //    flow_factor = flow, speed_factor = speed, points = [start_3d, end_3d].
    // PERFORMANCE NOTE: per-stroke clip is O(N * M). Acceptable for typical fixtures;
    // revisit with scanline approach if benchmarks show >50ms/layer (see Risks).
    todo!("implementation")
}
```

Delete: `topmost: BTreeMap<RegionIdent, usize>` cross-layer scan; `BBox2D::from_points` bounding-box approach (current fallback at lib.rs:276-302); `FinalizationModule` impl and `run_finalization` entry point (current impl at lib.rs:178); all `LayerCollectionView`-based traversal; unused imports.

### `modules/core-modules/top-surface-ironing/Cargo.toml`

Verify the dependency on `slicer-sdk` exposes the `InfillModule` trait (gyroid-infill is a working precedent). No `wit-bindgen` direct dep changes expected.

### `modules/core-modules/top-surface-ironing/wit-guest/src/lib.rs`

No change. `pub use top_surface_ironing::TopSurfaceIroning;` works for any wit-world.

### Tests

`modules/core-modules/top-surface-ironing/tests/top_surface_ironing_emission_tdd.rs` — full rewrite, 10 tests:
1. **Topmost-layer ironing emission** — `top_shell_index = Some(0)`, `top_solid_fill = vec![10mm × 10mm square]` → ironing pushed; verify `role = Ironing`, `flow_factor = 0.1`.
2. **No-shell-index emission skip** — `top_shell_index = None` → zero pushes.
3. **Interior-shell emission skip** — `top_shell_index = Some(1)` → zero pushes.
4. **Disabled config** — `ironing_enabled = false` → zero pushes.
5. **Spacing stroke count** — 10×10 mm polygon at spacing 0.1mm → at least 100 strokes.
6. **Bottom-only region** — `bottom_shell_index = Some(0)`, `top_shell_index = None` → zero ironing.
7. **Zero flow config error** — `ironing_flow = 0.0` → `on_print_start` rejects.
8. **Unsupported pattern config error** — `ironing_pattern = "concentric"` → `on_print_start` rejects.
9. **Polygon-precision (L-shape)** — all emitted stroke endpoints fall INSIDE the L (no strokes cross the concave notch). Point-in-polygon test.
10. **Cross-region independence** — two regions on the same layer, only region A has `top_shell_index = Some(0)`. Ironing emitted only on region A; no leak into region B.

### Guest WASM

Module's stage and wit-world changed → full rebuild required: `cargo xtask build-guests` (not `--check`).

### Verify

```
cargo test -p top-surface-ironing
cargo xtask build-guests --check
cargo test -p slicer-host --test dag_validation_tdd  # confirms two Layer::Infill modules coexist
```

---

## Commit 4: SupportGeometry Rewrite + Per-Object Validation

Replaces the `collect_polygons_at_z` stub at `support_geometry.rs:144-152` with `SliceIR` consumption. Adds per-object `support_layer_height_mm` validation.

### `crates/slicer-host/src/config_schema.rs`

Add validation rule (single authoritative source — runtime support_geometry trusts sanitized input):

```rust
// At config resolution time, after per-object resolved configs are built:
for (object_id, resolved) in &per_object_configs {
    let slh = resolved.support_layer_height_mm;
    if slh > 0.0 {
        let object_min_lh = layer_plan.global_layers.iter()
            .filter_map(|gl| gl.active_regions.iter().find(|r| &r.object_id == object_id))
            .map(|r| r.effective_layer_height)
            .fold(f32::INFINITY, f32::min);
        if slh < object_min_lh - 1e-6 {
            return Err(ConfigValidationError::SupportLayerHeightTooFine {
                object_id: object_id.clone(),
                resolved_mm: slh,
                object_min_mm: object_min_lh,
            });
        }
    }
}
```

Error variant:
```rust
#[error("support_layer_height_mm={resolved_mm:.4} is finer than object '{object_id}' minimum layer height {object_min_mm:.4}; must be >= {object_min_mm:.4}. Use 0.0 to match per-object layer height.")]
SupportLayerHeightTooFine { object_id: String, resolved_mm: f32, object_min_mm: f32 },
```

### `crates/slicer-host/src/support_geometry.rs`

- Rewrite `collect_polygons_at_z` (**lines 144-152**) — return UPPER-layer polygons (not union):
  ```rust
  fn collect_polygons_at_z(
      slice_vec: &[SliceIR],
      layer_plan: &LayerPlanIR,
      object_id: &ObjectId,
      region_id: RegionId,
      z: f32,
  ) -> Vec<ExPolygon> {
      let eps = 1e-6_f32;
      let pos = layer_plan.global_layers.binary_search_by(|gl| {
          if gl.z < z - eps { std::cmp::Ordering::Less }
          else if gl.z > z + eps { std::cmp::Ordering::Greater }
          else { std::cmp::Ordering::Equal }
      });
      match pos {
          Ok(idx) => extract_region_polys(&slice_vec[idx], object_id, region_id),
          Err(idx) => {
              // z falls between global_layers[idx-1] and global_layers[idx].
              // Return UPPER layer (idx) polygons — conservative for support pillars.
              // Documented deviation; see DEVIATION_LOG.md.
              if idx >= slice_vec.len() {
                  Vec::new()  // z above print top.
              } else {
                  extract_region_polys(&slice_vec[idx], object_id, region_id)
              }
          }
      }
  }

  fn extract_region_polys(slice: &SliceIR, object_id: &ObjectId, region_id: RegionId) -> Vec<ExPolygon> {
      slice.regions.iter()
          .filter(|r| &r.object_id == object_id && r.region_id == region_id)
          .flat_map(|r| r.polygons.clone())
          .collect()
  }
  ```
- Thread `slice_vec: &[SliceIR]` through `execute_support_geometry` (**lines 57-138**) and `add_intermediate_model_layers` (**158-192**). Both call sites pull `slice_vec` from blackboard.
- `commit_support_geometry_builtin` (**195-207**): replace `expect()`-style retrieval with structured error:
  ```rust
  pub fn commit_support_geometry_builtin(
      blackboard: &mut Blackboard,
  ) -> Result<(), SupportGeometryBuiltinError> {
      let layer_plan = blackboard.layer_plan()
          .ok_or(SupportGeometryBuiltinError::NoLayerPlan)?;
      let slice_vec = blackboard.slice_ir()
          .ok_or(SupportGeometryBuiltinError::MissingSliceIR)?;
      let mesh = blackboard.mesh();
      let ir = execute_support_geometry(layer_plan.as_ref(), mesh.as_ref(), slice_vec.as_ref())?;
      blackboard.commit_support_geometry(Arc::new(ir))?;
      Ok(())
  }
  ```
- New error variant `SupportGeometryBuiltinError::MissingSliceIR` with `#[error("PrePass::Slice must commit SliceIR before PrePass::SupportGeometry")]`.
- The validation-at-runtime check inside `execute_support_geometry` (per v2's plan): **removed**. Validation lives entirely at config-resolution time.

### Tests

`crates/slicer-host/tests/support_layer_height_validation_tdd.rs` (NEW):
1. `support_layer_height_mm = 0.1`, object min = 0.2 → `SupportLayerHeightTooFine`.
2. `support_layer_height_mm = 0.2`, object min = 0.2 → accepted.
3. `support_layer_height_mm = 0.4`, object min = 0.2 → accepted (coarsening).
4. `support_layer_height_mm = 0.0` (sentinel) → accepted.
5. **Multi-object mixed heights**: object A min = 0.2, object B min = 0.3 — per-object validation passes/fails appropriately. Demonstrates per-object resolution.

`crates/slicer-host/tests/prepass_support_geometry_tdd.rs` (UPDATE):
- Helper `blackboard_with_layer_plan` becomes `blackboard_with_layer_plan_and_slice`. Constructs `SliceIR` by calling `prepass_slice::execute_prepass_slice_all_layers` on the test mesh + layer plan (real path) rather than hand-constructing. Non-trivial helper rewrite — allocate explicit time.
- 8 existing test bodies updated to use the new helper.
- Add coarsening test: `support_layer_height_mm = 0.4`, model = 0.2. Support emits at every other model layer. Geometry at interpolated Z = upper layer's polygons.

`crates/slicer-host/tests/blackboard_support_geometry_slot_tdd.rs` — confirms ordering: `slice_ir` committed before `support_geometry`.

### Verify

```
cargo test -p slicer-host --test support_layer_height_validation_tdd --test prepass_support_geometry_tdd --test blackboard_support_geometry_slot_tdd
cargo clippy --workspace -- -D warnings
```

---

## Commit 5: E2E Regression Smoke

New test `crates/slicer-host/tests/slicing_promotion_e2e_regression_tdd.rs`. Production-grade verification — strict polygon containment, fixed indices, parsed G-code roles.

### Stepped Pyramid Fixture

Construct in-test (no committed STL):
- 4 steps stacked at Z = 0, 0.2, 0.4, 0.6.
- Each step a smaller axis-aligned square: 20×20, 16×16, 12×12, 8×8 mm centered on origin.
- Layer height 0.2mm → 4 global layers.
- `top_shell_layers = 3`, `bottom_shell_layers = 3`.

### Assertions

1. **Per-layer shell index** — after pipeline run, parse `SliceIR` per layer. Shell indices vary per region (the pyramid steps create multiple regions). Verify per `(object_id, region_id, layer_index)` using the actual region structure produced by the slicer (e.g. layer 0 print-bottom `bottom_shell_index = Some(0)`; top-exposed slivers `top_shell_index = Some(0)`; interior depths `Some(1)`/`Some(2)`).
2. **G-code role markers** — parse `out.gcode`. Format confirmed as `";TYPE:Ironing"` (per `crates/slicer-host/src/gcode_emit.rs:289` via `orca_type_label` at 275-292). Assert at least one `Ironing` block exists per step's topmost layer.
3. **Polygon containment** — for each `G1` line within an `Ironing` block, extract `X`/`Y`. Compute `(x, y) ∈ corresponding step's top_solid_fill polygon` via point-in-polygon. Assert 100% containment with 1e-3 mm tolerance.
4. **Stroke count bounds** — top step (8×8 mm at spacing 0.1) → at least 80 ironing strokes (loose lower bound). Avoids brittle exact-count assertions.
5. **Determinism under parallel layer iteration** — run the pipeline twice with rayon parallel layer iteration enabled. Diff G-code byte-by-byte. Must be IDENTICAL. Validates `layer-parallel-safe = true` (cross-layer state frozen in prepass; Tier-2 reads are read-only).
6. **No support generated** — flat-bed fixture → `SupportGeometryIR.entries` empty after prepass.
7. **Sparse infill on non-shell layers** — verifies the inverse path (where applicable; in a 4-layer all-shell fixture this confirms the role-selection branch).

### Verify

```
cargo test -p slicer-host --test slicing_promotion_e2e_regression_tdd
```

### Visual Sanity (manual, not gated)

```
cargo run --bin slicer-host --release -- run \
  --model resources/benchy.stl \
  --module-dir modules/core-modules \
  --output /tmp/out.gcode \
  --report /tmp/report.html
```
Inspect `/tmp/report.html` for `PrePass::Slice` and `PrePass::ShellClassification` stages in the timeline with distinct timing entries (per `host:slice` / `host:shell_classification` IDs), and ironing strokes visible on Benchy's top deck.

---

## Critical Files (with verified line citations)

### Commit 1 (schema + scaffolding)
- `crates/slicer-ir/src/slice_ir.rs` — `SlicedRegion` (1168-1199), `CURRENT_SLICE_IR_SCHEMA_VERSION` (184-188), `RegionKey` (1099-1107), `RegionPlan` (1118-1127), `entries` (1135).
- `crates/slicer-sdk/src/views.rs` — `SliceRegionView` (19-50), `Default` (52-76), setters (148-160), readers (197-207), `should_emit` (312-324).
- `crates/slicer-macros/src/lib.rs` — `__slicer_adapt_slice_regions` (2515 start; setters 2541-2542).
- `wit/deps/ir-types.wit` — `slice-region-view` (65-92); `push-ironing-path` already at 113.
- `crates/slicer-host/src/wit_host.rs` — `SliceRegionData` (122 start), trait accessors (3357-3364).
- `crates/slicer-host/src/execution_plan.rs` — STAGE_ORDER (27-49).
- `crates/slicer-host/src/dispatch.rs` — `export_name_for_stage` (46-68; Layer::Slice at 54).
- `crates/slicer-host/src/blackboard.rs` — fields (57-69), `Blackboard::new` (178-192), `BlackboardPrepassSlot` (139-156), `Display` (158-173).
- `crates/slicer-host/src/prepass.rs` — `required_slots` (658-676), `ensure_stage_prerequisites` (629-656).
- `crates/slicer-host/src/validation.rs` — stage strings (887-888); `FILL_CLAIM_IDS` (11-16), dedup (430-473) for audit.
- `crates/slicer-host/src/manifest.rs` — `known_stage_ids` (~1041); test refs (1144, 1152).
- `crates/slicer-host/tests/core_module_ir_access_contract_tdd.rs` — contract table (39-73).
- `crates/slicer-ir/tests/ir_tests.rs` — version literal (647-655).
- `modules/core-modules/rectilinear-infill/src/lib.rs` — precedence flip (142-148) + tests.
- `modules/core-modules/gyroid-infill/src/lib.rs` — precedence flip (~132-134).
- 16 live files with `is_top_surface:` literals.

### Commit 2 (slicing + classification)
- `crates/slicer-host/src/layer_slice.rs` → renamed `prepass_slice.rs`. `execute_layer_slice` (337-345), `classify_region_surfaces` (123-242; top/bottom lookahead 180-222 dropped; bridge 224-238 preserved).
- `crates/slicer-core/src/triangle_mesh_slicer.rs` — `slice_mesh_ex` (48), `apply_slice_closing_radius` (394).
- `crates/slicer-core/src/polygon_ops.rs` — `union` (93), `intersection` (98), `difference` (103), `offset` (185).
- `crates/slicer-host/src/slice_postprocess_prepass.rs` (NEW).
- `crates/slicer-host/src/prepass.rs` — executor wiring (support-geometry commit 530-542; insert Slice/ShellClassification before) + `StageInstrumentationGuard`.
- `crates/slicer-host/src/blackboard.rs` — `replace_slice_ir`.
- `crates/slicer-host/src/layer_executor.rs` — replace per-layer slice block (360-400; call 373-381).
- New tests `prepass_slice_tdd.rs`, `prepass_shell_classification_tdd.rs`.

### Commit 3 (ironing move)
- `modules/core-modules/top-surface-ironing/top-surface-ironing.toml` — full rewrite.
- `modules/core-modules/top-surface-ironing/src/lib.rs` — full rewrite (FinalizationModule impl at 178; bbox fallback 276-302 deleted).
- `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_emission_tdd.rs` — full rewrite (10 tests).
- `modules/core-modules/top-surface-ironing/Cargo.toml`, `wit-guest/src/lib.rs` — verify.

### Commit 4 (SupportGeometry)
- `crates/slicer-host/src/config_schema.rs` — per-object validation.
- `crates/slicer-host/src/support_geometry.rs` — `collect_polygons_at_z` (144-152), `execute_support_geometry` (57-138), `add_intermediate_model_layers` (158-192), `commit_support_geometry_builtin` (195-207).
- `crates/slicer-host/tests/support_layer_height_validation_tdd.rs` (NEW), `prepass_support_geometry_tdd.rs` (UPDATE), `blackboard_support_geometry_slot_tdd.rs`.

### Commit 5 (E2E)
- `crates/slicer-host/tests/slicing_promotion_e2e_regression_tdd.rs` (NEW).
- Reference: `crates/slicer-host/src/gcode_emit.rs:275-292` (`orca_type_label` → `";TYPE:Ironing"` at 289).

### Docs
- `docs/01_system_architecture.md` — Tier 2 description (no more `Layer::Slice`) + new prepass stages (end of Commit 2).
- `docs/02_ir_schemas.md` — `SlicedRegion` field list (Commit 1).
- `docs/DEVIATION_LOG.md` — see Documented Deviations below.

---

## OrcaSlicer Algorithm References (verbatim, in-repo)

All reads delegated per `CLAUDE.md` context-discipline.

- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp:1541-1892` — `detect_surfaces_type`. Pass 1 reference (depth-0 diff + opening anti-sliver).
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp:3928-4132` — `discover_horizontal_shells`. Pass 2 reference. The break-on-empty check is INSIDE the loop body — v3's "line 4001 goto EXTERNAL" cite was wrong (4001 is a comment, no goto). Re-derive the exact break location on read.
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp:3955, 2221, 2250` — `top_shell_layers` / `bottom_shell_layers` config reads.
- `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp:192, 1393` — `slice_closing_radius` round-trip. Now wired into the production slice path (Commit 2, decision 1).

**Unit conversion**: OrcaSlicer's `scaled_*` is in nanometers; our 100 nm/unit means divide by 100 when porting any `scaled_width()` value. The opening offset uses mm directly: `offset_mm = extrusion_width_mm / 10.0`.

---

## Documented Deviations (write to `docs/DEVIATION_LOG.md`)

1. **`collect_polygons_at_z` upper-only**: For interpolated Z planes in support generation, we return the upper-bracket layer's polygons (not interpolated, not unioned). OrcaSlicer uses slice contour interpolation; we approximate. Quality trade-off; revisit if support adherence issues surface. (Commit 4)
2. **Hollow-object `continue` path NOT ported**: OrcaSlicer's `discover_horizontal_shells` has a `continue` path for non-hollow objects (`sparse_infill_density > 0`) that keeps walking shell projection even when the intersection is empty. We break on empty unconditionally. Affects hollow objects with internal cavities where the shell should "reconnect" through gaps. (Commit 2)
3. **Extrusion-width fallback chain**: When `perimeter_extrusion_width` is missing/zero, we fall back to `line_width` → `effective_layer_height * 1.2` → 0.4 mm. OrcaSlicer reads `flow(frExternalPerimeter).scaled_width()` (flow-driven). Ours is config-driven. (Commit 2)
4. **`top_solid_fill` flattened across shell sources**: OrcaSlicer stores separate `Surface` entries per source layer. We flatten via `polygon_ops::union` into a single `top_solid_fill: Vec<ExPolygon>` per layer; `top_shell_index` carries only the minimum depth. Lossy for consumers needing per-source attribution (none currently). (Commit 2)
5. **Bottom-first precedence on overlap** (decision 3): For regions where both `top_shell_index = Some(0)` and `bottom_shell_index = Some(0)` (single-layer prints, thin overhangs), rectilinear-infill and gyroid-infill now resolve **bottom-wins** (OrcaSlicer parity). Behavior change vs. prior output. (Commit 1)
6. **Shell-index `Option<u8>` saturation** (decision 2): Config `top_shell_layers`/`bottom_shell_layers` are `u32`; the IR shell index is `Option<u8>` and saturates at depth 255. Config validation should reject/warn on saturation. Practical impact zero. (Commit 1)
7. **`apply_slice_closing_radius` now wired** (decision 1): Closes a packet-60 gap — the helper existed but was never called from the production slice path. Production slice output may change shape vs. pre-refactor for nonzero closing radius. (Commit 2)

---

## Verification (rollup)

### Per-commit gate
Each commit verifies independently. All commits must pass `cargo build --workspace` and `cargo clippy --workspace -- -D warnings`.

| Commit | Gate |
|---|---|
| 1 | `cargo build --workspace && cargo clippy --workspace -- -D warnings && cargo test -p slicer-ir -p slicer-sdk -p slicer-macros && cargo test -p slicer-host --test core_module_ir_access_contract_tdd --test prepass_executor_tdd && cargo test -p gyroid-infill -p rectilinear-infill && cargo xtask build-guests --check` |
| 2 | `cargo test -p slicer-host --test prepass_slice_tdd --test prepass_shell_classification_tdd --test prepass_executor_tdd` + clippy |
| 3 | `cargo test -p top-surface-ironing && cargo test -p slicer-host --test dag_validation_tdd && cargo xtask build-guests --check` |
| 4 | `cargo test -p slicer-host --test support_layer_height_validation_tdd --test prepass_support_geometry_tdd --test blackboard_support_geometry_slot_tdd` + clippy |
| 5 | `cargo test -p slicer-host --test slicing_promotion_e2e_regression_tdd` |

### Final-acceptance gate (after Commit 5)
Per `CLAUDE.md` Test Discipline — dispatched to sub-agent with FACT pass/fail:
1. `cargo build --workspace`
2. `cargo clippy --workspace -- -D warnings`
3. `cargo xtask build-guests --check`
4. `cargo test --workspace`

---

## Risks

1. **`apply_slice_closing_radius` wiring side-effects** (NEW) — production slice output may change shape vs. pre-refactor since closing radius was previously unused. Slicer-report visual sanity on benchy.stl is the canonical check; dimensional-fidelity regressions should be visible there.
2. **`apply_opening` composition + unit conversion** — `offset(-r)` then `offset(+r)` should produce OrcaSlicer's `opening_ex` semantics. `polygon_ops::offset` takes `delta_mm: f32`; the OrcaSlicer `scaled_width / 10` factor maps cleanly to `offset_mm = extrusion_width_mm / 10.0`. Polygons narrower than `2*r` collapse entirely — desired for anti-sliver, but verify with a unit test against sliver fixtures (width ≈ `external_perimeter_width / 5`).
3. **`Option<u8>` saturation at depth 255** (NEW) — practical impact zero, but `config_schema.rs` should warn if `top_shell_layers > 255` (or saturate silently with a deviation-log entry).
4. **Bottom-first precedence flip** (NEW) — test fixtures that previously expected `TopSolidInfill` on single-layer regions now expect `BottomSolidInfill`. Audit `rectilinear-infill/tests/top_bottom_fill_tdd.rs` and gyroid-infill tests carefully.
5. **`generate_zigzag_strokes_for_polygon` performance** — `O(N strokes × M polygon vertices)` per region per layer. 10mm² @ 0.1mm = 100 strokes × ~10 vertices = 1000 ops. Sub-millisecond expected. If polygon is complex (>100 vertices) or fill area large, costs scale. Benchmark on the e2e fixture; if >50 ms/layer, switch to a scanline implementation (use packet 60's `simplify_polyline_mm` to pre-decimate the contour).
6. **Memory cost for `top_solid_fill` / `bottom_solid_fill`** — typical 100-500 KB per region per layer; 1000-layer × 50-region worst case ~10 GB. Real-world prints (single-object, <100 layers, <5 regions) ≈ 50 MB. Watch for OOM on extreme multi-object prints. Streaming/pooling is a future optimization.
7. **Catch-up timeline correctness under unusual layer plans** — multi-object prints with extreme layer-height ratios (0.05 + 0.3 = mostly catch-ups for the fine object) stress the timeline walk. Test #8 covers the canonical case; add stress fixtures if real prints surface issues.
8. **Two modules at `Layer::Infill` DAG ordering** — `top-surface-ironing` + surviving infill module (gyroid alphabetically wins). Both write `InfillIR`. Per `crates/slicer-host/src/dag.rs:42-114`, edges form by IR write/read overlap. Verify the DAG accepts disjoint sub-writes (`paths` vs `ironing_paths`) without false cycles. If coarse `InfillIR` write conflict triggers, alphabetical ordering applies (`com.core.gyroid-infill` < `com.core.top-surface-ironing`). Validate via `dag_validation_tdd`.
9. **`ExtrusionRole::Ironing` G-code marker format** — Commit 5 assertion #2 confirmed as `";TYPE:Ironing"` (gcode_emit.rs:289). Re-read on implementation if the emitter changed.
10. **`SliceIR.regions` index stability** — `get_region_mut` needs a stable `(object_id, region_id)` lookup. `regions: Vec<SlicedRegion>` is `O(N)` per access. For typical N=5, fine. Build a `HashMap<(ObjectId, RegionId), usize>` index per layer at the start of Pass 1 if profiling shows hot lookups.
11. **Schema version 3.0.0 cache invalidation** — persisted `SliceIR` with version 2.x fails to deserialize after Commit 1 (no fallback). Audit `crates/slicer-host/tests/fixtures/` for cached IR artifacts; per grep, only `ir_tests.rs:647-655` hardcodes 2.1.0 — single update.
12. **`Arc::strong_count` between `commit_slice_builtin` and `replace_slice_ir`** — the executor borrows immutably (no `Arc::clone`) between the two calls, so refcount stays at 1. If instrumentation hooks clone the Arc, `replace_slice_ir` still works (it overwrites the slot; old Arc drops when other holders release). This is why we use `replace_slice_ir` over `Arc::get_mut` (which would panic on shared).
13. **`StageInstrumentationGuard` panic-end correctness** (NEW) — `Drop` using `std::thread::panicking()` works only on the panicking thread. Verify behavior under nested panics (rare).
14. **Test fixture construction labor in Commit 2's 15 cases** — significant work. Allocate explicit time. Test #14 (panic-injection for atomic-replace) needs a feature-gate or test-only panic hook in `apply_opening`; design before implementation.
15. **Line numbers re-verify before edit** — even with v4's corrections, intervening commits may shift lines. Re-read each target file BEFORE editing it.
