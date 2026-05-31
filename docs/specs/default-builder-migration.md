# Default + Builder Migration Spec

Status: ready for implementation
Approved: 2026-05-17 (user review on 2026-05-17 spec draft)
Date: 2026-05-17
Owner: see CODEOWNERS for `docs/`
Implementation tracking: `docs/07_implementation_status.md` → TASK-200
Author note: this is a specification, not a refactor. No `.rs` files are
changed by this document. Each chunk in the Migration Order section
(§9) is implemented as a separate PR; chunk-to-subtask mapping is
TASK-200a → Chunk 1, TASK-200b → Chunk 2, … TASK-200e → Chunk 5.

The next worker picks up TASK-200a (slicer-ir Bucket A POD leaf types)
unless §11 open questions 1, 6, or 11 require user input first.

## 1. Summary

Construction of multi-field structs is duplicated across this workspace.
Today, adding a field forces edits at every call site:
`GlobalLayer` (5 fields, 160 sites), `LoadedModule` (21 fields, 107 sites),
`ObjectMesh` (7 fields, 103 sites), `CompiledModule` (8 fields, 98 sites),
`LayerCollectionIR` (9 fields, 83 sites). This spec proposes which structs
get `Default`, which get a builder, and which stay untouched, plus the
order in which to land the changes.

**Workspace inventory (production `src/` only; tests, mocks, and
proc-macro generated tokens excluded):** 271 structs across 11 member
crates plus 27 `wit-guest` sub-crates and 13 `test-guests` crates.

| Bucket | Count | Action |
|---|---|---|
| A — `#[derive(Default)]` | 71 | Add (or confirm) `#[derive(Default)]` on the struct |
| B — manual `impl Default` | 28 | Hand-written `impl Default` with opinionated values; pairs with `..Default::default()` at call sites |
| C — builder | 5 | New consuming-style builder; `new()` removed; pub fields demoted to `pub(crate)` |
| D — untouched | 167 | Errors, unit structs, FFI, type aliases, enums-without-safe-default consumers, module entry types using `from_config`, glue Components |
| **Total** | **271** | |

Estimated diff size by chunk (lines added/removed, approximate):

| Chunk | Bucket(s) | Crates touched | Net LoC |
|---|---|---|---|
| 1 | A (slicer-ir POD types) | `slicer-ir` | +40 / -0 |
| 2 | B (slicer-ir schema-versioned IRs) | `slicer-ir`, plus call-site struct-update sweep in `slicer-host`, modules, tests | +280 / -1200 |
| 3 | C (slicer-helpers `DecimateConfig`) | `slicer-helpers` + downstream callers | +120 / -40 (MAJOR bump) |
| 4 | C (slicer-host `LoadedModule`, `CompiledModule`, `ConfigFieldSchema`) | `slicer-host` + downstream callers | +600 / -2800 (MAJOR bump) |
| 5 | C (slicer-host `HostExecutionContext`) + remaining A/B sweep | `slicer-host`, `slicer-sdk` | +220 / -700 (MAJOR bump) |

Net positive lines come from new `impl Default` bodies and builder types;
net negative lines come from collapsing repeated struct-literal fields
into `..Default::default()` at hundreds of call sites.

## 2. Test baseline

**Compile gate (every chunk):** `cargo check --workspace`. Verified clean
on `master` at the time this spec was written (commit `286022d`,
exit code 0, all 60 workspace members compile).

**Per-chunk narrow verification:** the exact `cargo test -p <crate>` (and
`--test <file>` where applicable) listed under each chunk in
§10 Test strategy.

**Workspace-wide acceptance gate (chunk close only):**
`cargo test --workspace`. Per CLAUDE.md test discipline, this is
dispatched to a sub-agent with a `FACT pass/fail` return rather than
absorbed into the main agent's context. Run only after the chunk's
narrow tests are already green.

**Pre-flight observation:** no test in the workspace currently relies
on the `new(...)` constructors that this spec deprecates beyond what
the migration tooling can rewrite. Tests rely on either struct-literal
syntax (`Foo { ... }`) or the fixture builders in `slicer-test`.
The Bucket C migrations explicitly carry call-site updates in the
same PR.

## 3. Builder and Default convention

This spec normalizes the workspace to **three conventions**, not four
or five. The orthogonal `from_config(&ConfigView) -> Result<Self,
ModuleError>` constructor on module-entry types is preserved and is
not in scope.

### 3.1 Convention 1 — `Default` (workhorse)

Every struct that has a sensible zero or opinionated default gets one,
derived or manual. Pairs with `..Default::default()` struct-update
syntax at call sites.

```rust
let cfg = ResolvedConfig {
    layer_height: 0.3,
    infill_density: 0.4,
    ..Default::default()
};
```

This is the single biggest ergonomic win in the spec. For 27-field
`ResolvedConfig` with `pub` fields, struct-update beats a builder on
every axis: shorter, no new types to maintain, fully idiomatic Rust.

### 3.2 Convention 2 — Builder (sparingly)

A struct gets a builder **only** when one of these is true:

1. The struct has 12+ fields **and** struct-update doesn't fit (private
   fields are needed for invariant protection).
2. Cross-field validation must run at construction (e.g. exactly-one-of
   pair, or mutually-required pair).
3. Protocol ordering exists (e.g. once-only setters that must `Err`
   on second call).

**Style — default:** consuming `mut self -> Self` with `#[must_use]` on
both the builder type and every setter; terminal `build(self) -> T`
(or `build(self) -> Result<T, BuildError>` for validating builders).
Matches the existing `slicer-test` fixture builders.

```rust
let module = LoadedModuleBuilder::new(id, version, stage, wit_world, wasm_path)
    .ir_reads(reads)
    .ir_writes(writes)
    .claims(claims)
    .requires_modules(deps)
    .build();
```

**Style — exception:** non-consuming `&mut self -> &mut Self` is permitted
**per struct** when (a) callers commonly apply 5+ setters via `for` loop
or `if let Some(_)` chains, **and** (b) the rebind cost
(`b = b.foo(x)`) materially hurts readability for that type. Spec
flags every such exception explicitly in §6. Default to consuming
unless flagged.

**`build()` returns `Result` only when validation runs.** Otherwise
`build()` returns `T` directly.

**Builder-only for migrated structs.** When a struct gets a builder:
- `pub` fields are demoted to `pub(crate)`.
- Existing `new()` factories are removed.
- All in-tree call sites are migrated in the same PR.
- The change is **MAJOR** semver for the affected re-export crate
  (`slicer-ir`, `slicer-host`, `slicer-helpers`). Tagged per type in §8.

### 3.3 Convention 3 — SDK output accumulator (untouched)

The `slicer-sdk` push-style builders
(`InfillOutputBuilder`, `PerimeterOutputBuilder`, `SupportOutputBuilder`,
`SlicePostprocessBuilder`, `GcodeOutputBuilder`, `FinalizationOutputBuilder`,
`LayerCollectionBuilder`, plus the prepass `*Output` types in
`crates/slicer-sdk/src/prepass_builders.rs`) follow a distinct pattern:

- `&mut self -> Result<(), String>` push methods
- No terminal `build()`; drained by the `#[slicer_module]` macro at
  dispatch time
- `impl Default` delegates to `Self::new()` (empty buffer)

This pattern is **structurally required by the WIT/dispatch contract**
and is not a builder in the configure-and-build sense. The spec leaves
all such types untouched and documents them here so future authors
do not confuse them with the configure-and-build builders introduced
in §3.2.

### 3.4 `from_config` constructors (untouched)

Every core module's
`fn from_config(config: &ConfigView) -> Result<Self, ModuleError>`
stays as-is. This is the canonical module-bootstrap path
(`modules/core-modules/*/src/lib.rs`). Replacing it with a builder is
out of scope for this migration and would touch every core module
simultaneously.

### 3.5 No builder-derivation crate

The workspace currently uses no `derive_builder`, `typed-builder`,
`bon`, or `buildstructor`. All builders proposed here are hand-rolled,
matching current practice. Trade-off discussed in §11 Risks.

### 3.6 Enum `#[default]` is permitted only when the variant is semantically safe

Some Bucket A migrations depend on adding `#[default]` to an enum
variant so the containing struct can `#[derive(Default)]`. This spec
permits it only for enums where the chosen variant is semantically
safe (i.e. picking the wrong variant by accident would not silently
miscategorize data downstream). Enabled enum defaults:

| Enum | File | Default variant | Justification |
|---|---|---|---|
| `WallGenerator` | `slicer-ir/src/slice_ir.rs:1130` | `Classic` | Already the `ResolvedConfig::default` value. |
| `InfillType` | `slicer-ir/src/slice_ir.rs:1139` | `Grid` | Already the `ResolvedConfig::default` value. |
| `SupportType` | `slicer-ir/src/slice_ir.rs:1158` | `Traditional` | Already the `ResolvedConfig::default` value. |
| `FacetClass` (SDK) | `slicer-sdk/src/prepass_types.rs:22` | `Normal` | "No classification" is the safe baseline. |

Rejected enum defaults (kept as no-`Default` because no safe variant
exists): `PaintSemantic`, `PaintValue`, `ModifierScope`, `WallBoundaryType`,
`LoopType`, `ExtrusionRole`, `SeamReason` (IR enum), `LayerAnnotationKind`,
`GCodeCommand`, `FacetClass` (IR — has variants with fields, e.g.
`Overhang { angle_deg }`, where a `Normal` default could mask missing
angle data downstream). Structs whose fields use these enums stay in
Bucket D.

## 4. Bucket A — `#[derive(Default)]`

71 structs. Migration adds the derive on the struct (and, where listed
in §3.6, `#[default]` on a single enum variant in the same change).

### 4.1 `slicer-ir` (39)

| path:line | Struct | Justification |
|---|---|---|
| `crates/slicer-ir/src/slice_ir.rs:70` | `Point2` | 2× `i64`, Copy. Zero coord is the origin — semantically fine. |
| `crates/slicer-ir/src/slice_ir.rs:94` | `Point3` | 3× `f32`. Same rationale. |
| `crates/slicer-ir/src/slice_ir.rs:105` | `BoundingBox3` | 2× `Point3` once Point3 is Bucket A. |
| `crates/slicer-ir/src/slice_ir.rs:114` | `Transform3d` | `[f64; 16]` — `Default` is all-zeros. NOTE: zero matrix is degenerate as a transform; flag in §11. Recommend deriving anyway and adding `identity()` convenience constructor in a follow-up. |
| `crates/slicer-ir/src/slice_ir.rs:121` | `IndexedTriangleSet` | Vec + Vec. Empty mesh is valid. |
| `crates/slicer-ir/src/slice_ir.rs:130` | `SemVer` | 3× `u32`. Zero-version `0.0.0` is the documented sentinel; IRs that need a pinned schema override via Bucket B (§5). |
| `crates/slicer-ir/src/slice_ir.rs:167` | `ObjectConfig` | One HashMap; empty = no override. |
| `crates/slicer-ir/src/slice_ir.rs:225` | `FacetPaintData` | One `Vec<PaintLayer>`. |
| `crates/slicer-ir/src/slice_ir.rs:232` | `ConfigDelta` | One HashMap; empty = no delta. |
| `crates/slicer-ir/src/slice_ir.rs:269` | `ObjectMesh` | 7 fields, all stdlib once Transform3d is A. `world_z_extent: Option<(f32,f32)>` already defaults to None. |
| `crates/slicer-ir/src/slice_ir.rs:337` | `SurfaceGroup` | 7 stdlib fields. |
| `crates/slicer-ir/src/slice_ir.rs:356` | `BridgeRegion` | 8 stdlib fields. |
| `crates/slicer-ir/src/slice_ir.rs:382` | `OverhangRegion` | 4 stdlib fields. |
| `crates/slicer-ir/src/slice_ir.rs:395` | `ObjectSurfaceData` | 4× Vec. |
| `crates/slicer-ir/src/slice_ir.rs:609` | `NonPlanarShellRef` | 2× u32, Copy. |
| `crates/slicer-ir/src/slice_ir.rs:734` | `ActiveRegion` | 8 fields, all stdlib once `ResolvedConfig` (Bucket B) is Default. |
| `crates/slicer-ir/src/slice_ir.rs:755` | `GlobalLayer` | 5 stdlib fields. **Highest construction-site count in the workspace (160).** |
| `crates/slicer-ir/src/slice_ir.rs:770` | `ObjectLayerRef` | 3 stdlib fields. |
| `crates/slicer-ir/src/slice_ir.rs:817` | `SeamPlanEntry` | 3 fields once `SeamPosition` (Bucket A below) is Default. |
| `crates/slicer-ir/src/slice_ir.rs:895` | `SupportGeometryKey` | 3 stdlib fields. |
| `crates/slicer-ir/src/slice_ir.rs:936` | `LayerPaintMap` | u32 + HashMap. |
| `crates/slicer-ir/src/slice_ir.rs:976` | `FacetPaintMark` | 4 stdlib fields. |
| `crates/slicer-ir/src/slice_ir.rs:1008` | `RegionKey` | 3 stdlib fields. |
| `crates/slicer-ir/src/slice_ir.rs:1019` | `ModuleInvocation` | `ModuleId` + `ConfigView` (already has Default). |
| `crates/slicer-ir/src/slice_ir.rs:1028` | `RegionPlan` | 3 fields, all Default-able. |
| `crates/slicer-ir/src/slice_ir.rs:1052` | `Polygon` | One Vec. |
| `crates/slicer-ir/src/slice_ir.rs:1059` | `ExPolygon` | Polygon + Vec. |
| `crates/slicer-ir/src/slice_ir.rs:1068` | `SlicedRegion` | 12 stdlib fields — but every one is a Vec, Option, HashMap, f32, or bool. Empty region with 0.0 height is the natural default. |
| `crates/slicer-ir/src/slice_ir.rs:1194` | `WallFeatureFlags` | 6 fields, all Default-able (`Option<u32>`, bools, HashMap). |
| `crates/slicer-ir/src/slice_ir.rs:1211` | `WidthProfile` | One Vec. |
| `crates/slicer-ir/src/slice_ir.rs:1218` | `Point3WithWidth` | 6 fields, all f32 or Option, Copy. Zero point with zero width is a degenerate sentinel — flag in §11 if tests forget to override. |
| `crates/slicer-ir/src/slice_ir.rs:1346` | `SeamPosition` | `Point3WithWidth` + u32. |
| `crates/slicer-ir/src/slice_ir.rs:1355` | `PerimeterRegion` | 6 fields, all Default-able. |
| `crates/slicer-ir/src/slice_ir.rs:1387` | `InfillRegion` | 5 stdlib fields. |
| `crates/slicer-ir/src/slice_ir.rs:1438` | `ToolChange` | 3× u32. |
| `crates/slicer-ir/src/slice_ir.rs:1449` | `ZHop` | u32 + f32. |
| `crates/slicer-ir/src/slice_ir.rs:1458` | `TravelRetract` | 5 fields including `RetractMode` (already `#[default] = Gcode`). |
| `crates/slicer-ir/src/slice_ir.rs:1476` | `TravelMove` | 5 fields, all Option. |
| `crates/slicer-ir/src/slice_ir.rs:1637` | `PrintMetadata` | 4 stdlib fields. |

### 4.2 `slicer-host` (12)

All currently `#[derive(Default)]` in tree — listed for completeness and
confirmation that the bucket assignment is correct. No code change per
struct unless the spec moves it to a different bucket.

| path:line | Struct | Status |
|---|---|---|
| `crates/slicer-host/src/execution_plan.rs:664` | `IrAccessMask` | Existing derive, kept. |
| `crates/slicer-host/src/manifest.rs:71` | `ConfigFieldEntry` | Existing derive, kept. |
| `crates/slicer-host/src/manifest.rs:116` | `ConfigSchema` | Existing derive, kept. |
| `crates/slicer-host/src/manifest.rs:177` | `LoadModulesReport` | Existing derive, kept. |
| `crates/slicer-host/src/validation.rs` (DagValidationReport) | `DagValidationReport` | Existing derive, kept. |
| `crates/slicer-host/src/report/model.rs:19` | `MemDelta` | Existing derive, kept. |
| `crates/slicer-host/src/report/model.rs:117` | `ParallelismRecord` | Existing derive, kept. |
| `crates/slicer-host/src/report/model.rs:129` | `SliceMeta` | Existing derive, kept. |
| `crates/slicer-host/src/report/model.rs:146` | `Report` | Existing derive, kept. |
| `crates/slicer-host/src/wit_host.rs:899` | `FinalizationOutputBuilderData` | Existing derive, kept (SDK output-accumulator data holder). |
| `crates/slicer-host/src/blackboard.rs` (LayerArena) | `LayerArena` | Existing derive (per Phase-1 audit), kept. |
| `crates/slicer-host/src/progress_events.rs` (SliceEventCollector) | `SliceEventCollector` | Existing derive (per Phase-1 audit), kept. |

### 4.3 `slicer-sdk` (14)

Adding `Default` to enable struct-update at SDK consumer sites. All are
prepass/view types with pub fields and stdlib field types.

| path:line | Struct | Justification |
|---|---|---|
| `crates/slicer-sdk/src/prepass_types.rs:44` | `FacetAnnotation` | Adds `#[default = Normal]` to SDK `FacetClass` enum (§3.6). |
| `crates/slicer-sdk/src/prepass_types.rs:71` | `SurfaceGroupProposal` | 4 stdlib fields. Replaces hand-rolled `new()` (4 params). |
| `crates/slicer-sdk/src/prepass_types.rs:105` | `RegionLayerProposal` | 5 stdlib fields. Replaces `new()` (5 params). |
| `crates/slicer-sdk/src/prepass_types.rs:142` | `MeshObjectView` | 4 stdlib fields. |
| `crates/slicer-sdk/src/prepass_types.rs:155` | `PaintLayerView` | 3 fields, all stdlib (`String`, Vec, Vec). |
| `crates/slicer-sdk/src/prepass_types.rs:166` | `PaintValueView` | 4 Option fields + kind: String. |
| `crates/slicer-sdk/src/prepass_types.rs:179` | `PaintStrokeView` | 3 fields (Vec, String, PaintValueView). |
| `crates/slicer-sdk/src/prepass_types.rs:193` | `PaintSegmentationObjectView` | 6 stdlib fields. Replaces `new()` (6 params). |
| `crates/slicer-sdk/src/prepass_types.rs:266` | `LayerProposal` | 2 stdlib fields. |
| `crates/slicer-sdk/src/prepass_types.rs:282` | `SeamReason` | Single String field. |
| `crates/slicer-sdk/src/prepass_types.rs:289` | `ScoredSeamCandidate` | 3 stdlib fields. |
| `crates/slicer-sdk/src/prepass_types.rs:300` | `SeamPlanEntry` | 6 stdlib fields. |
| `crates/slicer-sdk/src/prepass_types.rs:317` | `SupportPlanEntry` | 4 stdlib fields (i32, String, String, Vec). |
| `crates/slicer-sdk/src/prepass_types.rs:333` | `LayerPlanViewEntry` | 3 stdlib fields. |
| `crates/slicer-sdk/src/prepass_types.rs:344` | `LayerPlanView` | One Vec. |

### 4.4 `slicer-helpers` (4)

| path:line | Struct | Justification |
|---|---|---|
| `crates/slicer-helpers/src/decimate.rs:34` | `DecimateResult` | 4 stdlib fields. Return value type; Default useful for tests. |
| `crates/slicer-helpers/src/repair.rs:12` | `RepairResult` | `MeshIR` (Bucket B) + `RepairStats` (existing derive). |
| `crates/slicer-helpers/src/import/step.rs` (NamedMesh) | `NamedMesh` | `Option<String>` + `MeshIR`. |
| `crates/slicer-helpers/src/import/step.rs` (StepImportResult) | `StepImportResult` | Stdlib fields. |

### 4.5 `slicer-core` (2)

| path:line | Struct | Justification |
|---|---|---|
| `crates/slicer-core/src/aabb_tree.rs:78` | `RayHit` | 3 stdlib fields. |
| `crates/slicer-core/src/aabb_tree.rs:87` | `ClosestPointHit` | 3 stdlib fields. |

## 5. Bucket B — manual `impl Default`

28 structs. For each, the spec gives a ready-to-paste `impl Default`
block. For structs that already have a hand-written `impl Default`,
the spec lists them with status `existing — kept` and no code block
(the existing impl is unchanged).

### 5.1 `slicer-ir` schema-versioned IRs (15)

The pattern: pin `schema_version` to the canonical `CURRENT_*_SCHEMA_VERSION`
const, default `global_layer_index` to 0, default `z` to 0.0, default
all collection fields to empty. For IRs that already define a schema
constant, use it. For the four IRs without a defined `CURRENT_*_SCHEMA_VERSION`
const (`LayerPlanIR`, `SeamPlanIR`, `SupportPlanIR`, `SupportGeometryIR`,
`PaintRegionIR`, `MeshSegmentationIR`, `RegionMapIR`, `PerimeterIR`,
`InfillIR`, `SupportIR`, `LayerCollectionIR`, `GCodeIR`), this spec
proposes adding the missing constants as part of Chunk 2 — flagged
in §11 as an open question.

**`MeshIR`** — `crates/slicer-ir/src/slice_ir.rs:299`. Schema constant
needed: `CURRENT_MESH_IR_SCHEMA_VERSION` (proposed `1.0.0`).

```rust
impl Default for MeshIR {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_MESH_IR_SCHEMA_VERSION,
            objects: Vec::new(),
            build_volume: BoundingBox3::default(),
        }
    }
}
```

**`SurfaceClassificationIR`** — existing, kept. See line 415.

**`LayerPlanIR`** — `slice_ir.rs:781`. Schema constant: `CURRENT_LAYER_PLAN_IR_SCHEMA_VERSION` (proposed `1.0.0`).

```rust
impl Default for LayerPlanIR {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_LAYER_PLAN_IR_SCHEMA_VERSION,
            global_layers: Vec::new(),
            object_participation: HashMap::new(),
        }
    }
}
```

**`SeamPlanIR`** — `slice_ir.rs:832`. Schema constant: `CURRENT_SEAM_PLAN_IR_SCHEMA_VERSION` (proposed `1.0.0`).

```rust
impl Default for SeamPlanIR {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SEAM_PLAN_IR_SCHEMA_VERSION,
            entries: Vec::new(),
        }
    }
}
```

**`SupportPlanIR`** — `slice_ir.rs:880`. Schema constant: `CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` (proposed `1.0.0`).

```rust
impl Default for SupportPlanIR {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION,
            entries: Vec::new(),
        }
    }
}
```

**`SupportGeometryIR`** — `slice_ir.rs:906`. Schema constant:
`CURRENT_SUPPORT_GEOMETRY_IR_SCHEMA_VERSION` (proposed `1.0.0`).

```rust
impl Default for SupportGeometryIR {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SUPPORT_GEOMETRY_IR_SCHEMA_VERSION,
            support_layer_height_mm: 0.0,
            support_top_z_distance_mm: 0.0,
            entries: HashMap::new(),
        }
    }
}
```

**`PaintRegionIR`** — `slice_ir.rs:945`. Schema constant proposed.

```rust
impl Default for PaintRegionIR {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_PAINT_REGION_IR_SCHEMA_VERSION,
            per_layer: HashMap::new(),
        }
    }
}
```

**`MeshSegmentationIR`** — `slice_ir.rs:995`. Schema constant proposed.

```rust
impl Default for MeshSegmentationIR {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_MESH_SEGMENTATION_IR_SCHEMA_VERSION,
            marks: Vec::new(),
        }
    }
}
```

**`RegionMapIR`** — `slice_ir.rs:1039`. Schema constant proposed.

```rust
impl Default for RegionMapIR {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_REGION_MAP_IR_SCHEMA_VERSION,
            entries: HashMap::new(),
        }
    }
}
```

**`SliceIR`** — existing, kept. See line 1113.

**`PerimeterIR`** — `slice_ir.rs:1372`. Schema constant proposed.

```rust
impl Default for PerimeterIR {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_PERIMETER_IR_SCHEMA_VERSION,
            global_layer_index: 0,
            regions: Vec::new(),
        }
    }
}
```

**`InfillIR`** — `slice_ir.rs:1402`. Schema constant proposed.

```rust
impl Default for InfillIR {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_INFILL_IR_SCHEMA_VERSION,
            global_layer_index: 0,
            regions: Vec::new(),
        }
    }
}
```

**`SupportIR`** — `slice_ir.rs:1417`. Schema constant proposed.

```rust
impl Default for SupportIR {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SUPPORT_IR_SCHEMA_VERSION,
            global_layer_index: 0,
            support_paths: Vec::new(),
            interface_paths: Vec::new(),
            raft_paths: Vec::new(),
            ironing_paths: Vec::new(),
        }
    }
}
```

**`LayerCollectionIR`** — `slice_ir.rs:1527`. Schema constant proposed.

```rust
impl Default for LayerCollectionIR {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION,
            global_layer_index: 0,
            z: 0.0,
            ordered_entities: Vec::new(),
            tool_changes: Vec::new(),
            z_hops: Vec::new(),
            annotations: Vec::new(),
            retracts: Vec::new(),
            travel_moves: Vec::new(),
        }
    }
}
```

**`GCodeIR`** — `slice_ir.rs:1650`. Schema constant proposed.

```rust
impl Default for GCodeIR {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_GCODE_IR_SCHEMA_VERSION,
            commands: Vec::new(),
            metadata: PrintMetadata::default(),
        }
    }
}
```

**`ResolvedConfig`** — existing, kept. See line 696. The 27-field
OrcaSlicer-aligned default is the canonical place for project defaults
and pairs with `..Default::default()` at every override site.

**`ConfigView`** — existing, kept. See line 601. Default delegates to
`new()` (empty). The private-field invariant is the reason this stays
in Bucket B with the existing impl rather than Bucket C — `new()` is
already the documented constructor and the type does not benefit from
a builder.

### 5.2 `slicer-host` (8) — existing manual `impl Default`, kept

All eight structs below already have hand-written `impl Default` blocks
with opinionated values. The spec keeps them as-is and lists them for
completeness so the chunk plans can audit drift.

| path:line | Struct | Notes |
|---|---|---|
| `crates/slicer-host/src/execution_plan.rs:576` | `ExecutionPlan` | Empty vecs and `Arc::new(empty)`. |
| `crates/slicer-host/src/gcode_emit.rs:92` | `FeedrateConfig` | OrcaSlicer-aligned speeds. |
| `crates/slicer-host/src/gcode_emit.rs:651` | `DefaultGCodeSerializer` | Delegates to `new()`; PLA filament 1.75 mm / 1.24 g/cm³. |
| `crates/slicer-host/src/mesh_analysis.rs:36` | `MeshAnalysisConfig` | Project policy: bridge length 10.0, anchor 0.5, expansion 1.0, overhang 45.0°. |
| `crates/slicer-host/src/wasm_instance.rs:154` | `WasmEngine` | Delegates to `new()`. |
| `crates/slicer-host/src/instance_pool.rs:` (WasmArtifactMetadata) | `WasmArtifactMetadata` | Existing derive. |
| `crates/slicer-host/src/config_schema.rs:128` | `FullConfigSchema` | ~200-line hand-built schema with OrcaSlicer-aligned numeric defaults. **Flagged in §11**: consider refactoring readability in a follow-up chunk; out of scope here. |

### 5.3 `slicer-sdk` (2)

**`SliceRegionView`** — `crates/slicer-sdk/src/views.rs:19` (12 fields,
several with non-zero defaults documented in `new()` body at line 55).

```rust
impl Default for SliceRegionView {
    fn default() -> Self {
        Self {
            object_id: ObjectId::default(),
            region_id: RegionId::default(),
            polygons: Vec::new(),
            infill_areas: Vec::new(),
            effective_layer_height: 0.0,
            z: 0.0,
            has_nonplanar: false,
            boundary_paint: HashMap::new(),
            needs_support: true,            // matches existing new() default
            is_top_surface: false,
            is_bottom_surface: false,
            is_bridge: false,
            bridge_areas: Vec::new(),
            bridge_orientation_deg: 0.0,
            held_claims: Vec::new(),
        }
    }
}
```

Bucket B (not A) because `needs_support: true` is the project's chosen
non-zero default, matching today's `new()` behaviour.

**`PerimeterRegionView`** — `crates/slicer-sdk/src/views.rs:317`.

```rust
impl Default for PerimeterRegionView {
    fn default() -> Self {
        Self {
            object_id: ObjectId::default(),
            region_id: RegionId::default(),
            wall_loops: Vec::new(),
            infill_areas: Vec::new(),
            seam_candidates: Vec::new(),
            resolved_seam: None,
        }
    }
}
```

### 5.4 `slicer-helpers` (1)

**`RepairStats`** — existing `#[derive(Default)]` (line 21). Listed in
Bucket B rather than A because the field semantics (counters that
should be zero, warnings that should be empty) match the manual-
defaults rationale even though the derive happens to produce the same
values. No change needed.

**`DecimateConfig`** — existing manual `impl Default` (line 21). See
§6.5 for Bucket C migration.

### 5.5 `slicer-host` re-exports already covered

`ConfigFieldSchema` (Bucket C in §6), `LoadedModule` (Bucket C), and
`CompiledModule` (Bucket C) are intentionally NOT in Bucket B.

## 6. Bucket C — builder pattern

5 structs total. Each section gives the builder type signature, setter
methods, validation points, and the terminal `build()` return type.
**No full implementations** — those are written during the chunk PR.

### 6.1 `LoadedModule` — `crates/slicer-host/src/manifest.rs:19`

21 fields, 107 construction sites. The private-only test fixture
builder at `tests/dag_validation_tdd.rs:556` graduates to production.

**Required at construction (positional args to `new()`):**
`id: ModuleId`, `version: SemVer`, `stage: StageId`, `wit_world: String`,
`wasm_path: PathBuf`. These five are the manifest-derived identity of
a module — no module exists without them.

**Builder type:** `pub struct LoadedModuleBuilder { /* 21 private fields */ }`.

**Setter convention:** consuming `mut self -> Self`, `#[must_use]`. All
setters return `Self`.

```rust
impl LoadedModuleBuilder {
    pub fn new(
        id: impl Into<ModuleId>,
        version: SemVer,
        stage: impl Into<StageId>,
        wit_world: impl Into<String>,
        wasm_path: impl Into<PathBuf>,
    ) -> Self;

    pub fn ir_reads(self, reads: Vec<String>) -> Self;
    pub fn ir_writes(self, writes: Vec<String>) -> Self;
    pub fn claims(self, claims: Vec<String>) -> Self;
    pub fn requires_claims(self, claims: Vec<String>) -> Self;
    pub fn incompatible_with(self, names: Vec<String>) -> Self;
    pub fn requires_modules(self, modules: Vec<ModuleId>) -> Self;
    pub fn min_host_version(self, v: SemVer) -> Self;
    pub fn min_ir_schema(self, v: SemVer) -> Self;
    pub fn max_ir_schema(self, v: SemVer) -> Self;
    pub fn config_schema(self, schema: ConfigSchema) -> Self;
    pub fn overridable_per_region(self, keys: Vec<String>) -> Self;
    pub fn overridable_per_layer(self, keys: Vec<String>) -> Self;
    pub fn layer_parallel_safe(self, safe: bool) -> Self;
    pub fn placeholder_wasm(self, placeholder: bool) -> Self;

    pub fn build(self) -> LoadedModule;
}
```

**Terminal:** `build(self) -> LoadedModule` (no `Result`; manifest
validation already lives in `manifest::ingest_manifest`, not in struct
construction).

**Defaults for unset fields:** empty `Vec` everywhere, `SemVer { 0, 0, 0 }`
for version fields not set, `ConfigSchema::default()`, `layer_parallel_safe: false`,
`placeholder_wasm: false`.

**Setter style:** consuming. Use case is always "build one module"; no
loop-conditional setting observed in 107 sites.

### 6.2 `CompiledModule` — `crates/slicer-host/src/execution_plan.rs:637`

8 fields, 98 construction sites. The struct holds `Arc<WasmInstancePool>`
and `Option<Arc<WasmComponent>>` for which there is no semantically-safe
"default empty" value — a `CompiledModule` with no instance pool is
broken. Builder is justified by the all-required-fields constraint plus
the high call-site count.

**Required at construction (positional):**
`module_id: ModuleId`, `instance_pool: Arc<WasmInstancePool>`.
Everything else is settable post-`new`.

```rust
impl CompiledModuleBuilder {
    pub fn new(module_id: impl Into<ModuleId>, instance_pool: Arc<WasmInstancePool>) -> Self;

    pub fn ir_read_mask(self, mask: IrAccessMask) -> Self;
    pub fn ir_write_mask(self, mask: IrAccessMask) -> Self;
    pub fn config_view(self, view: Arc<ConfigView>) -> Self;
    pub fn claims(self, claims: Vec<String>) -> Self;
    pub fn wasm_component(self, component: Option<Arc<WasmComponent>>) -> Self;
    pub fn requires_modules(self, modules: Vec<ModuleId>) -> Self;

    pub fn build(self) -> CompiledModule;
}
```

**Defaults for unset fields:** `IrAccessMask::default()` (empty paths),
`Arc::new(ConfigView::new())` for `config_view`, empty Vec for `claims`
and `requires_modules`, `None` for `wasm_component`.

**Setter style:** consuming. No loop-conditional setting observed.

### 6.3 `ConfigFieldSchema` — `crates/slicer-host/src/config_schema.rs:47`

17 fields, 31 construction sites (almost all in `FullConfigSchema::default`,
where each field is built in a `fields.insert(...)` loop with many
optional fields set to `None`).

**Required at construction (positional):**
`key: String`, `field_type: ConfigFieldType`. Every other field has a
documented "absent" default (`None`, `false`, or `ConfigUnit::None`).

```rust
impl ConfigFieldSchemaBuilder {
    pub fn new(key: impl Into<String>, field_type: ConfigFieldType) -> Self;

    pub fn default_value(&mut self, v: ConfigValue) -> &mut Self;
    pub fn display(&mut self, s: impl Into<String>) -> &mut Self;
    pub fn description(&mut self, s: impl Into<String>) -> &mut Self;
    pub fn group(&mut self, s: impl Into<String>) -> &mut Self;
    pub fn unit(&mut self, u: ConfigUnit) -> &mut Self;
    pub fn advanced(&mut self, b: bool) -> &mut Self;
    pub fn min(&mut self, v: f64) -> &mut Self;
    pub fn max(&mut self, v: f64) -> &mut Self;
    pub fn step(&mut self, v: f64) -> &mut Self;
    pub fn max_length(&mut self, n: usize) -> &mut Self;
    pub fn enum_values(&mut self, v: Vec<String>) -> &mut Self;
    pub fn min_list_length(&mut self, n: usize) -> &mut Self;
    pub fn max_list_length(&mut self, n: usize) -> &mut Self;
    pub fn validate(&mut self, expr: impl Into<String>) -> &mut Self;

    pub fn build(&self) -> ConfigFieldSchema;
}
```

**Setter style: non-consuming `&mut self -> &mut Self`.** This is the
documented exception (§3.2) — the existing `FullConfigSchema::default()`
body inserts ~30 schema fields in a procedural loop where each iteration
calls `ConfigFieldSchema { key, field_type, default, group, unit, ... }`
with a different mix of optionals set. Non-consuming setters let the loop
read more clearly:

```rust
let mut b = ConfigFieldSchemaBuilder::new(key, ConfigFieldType::Float);
b.default_value(ConfigValue::Float(default_val)).group("Speed").unit(ConfigUnit::MillimetersPerSecond).min(0.0);
fields.insert(key.to_string(), b.build());
```

**Terminal:** `build(&self) -> ConfigFieldSchema` (no `Result`; no
cross-field validation runs at construction — validation against actual
values lives in `validate_config`).

### 6.4 `HostExecutionContext` — `crates/slicer-host/src/wit_host.rs:1358`

25 fields, 45 construction sites. Holds the per-dispatch state for one
module call: module_id, layer_z, layer_height, several `Option<Arc<MeshIR>>`
slots for the various prepass outputs visible at this dispatch point,
and the per-call output accumulators.

**Required at construction (positional):**
`module_id: String`, `layer_z: f32`, `effective_layer_height: f32`.

**Setter style:** consuming (the single-call build use case fits the
fluent chain). Setters cover each of the 22 optional fields:

```rust
impl HostExecutionContextBuilder {
    pub fn new(
        module_id: impl Into<String>,
        layer_z: f32,
        effective_layer_height: f32,
    ) -> Self;

    pub fn catchup_z_bottom(self, v: Option<f32>) -> Self;
    pub fn mesh_ir(self, v: Option<Arc<MeshIR>>) -> Self;
    pub fn surface_classification(self, v: Option<Arc<SurfaceClassificationIR>>) -> Self;
    pub fn layer_plan(self, v: Option<Arc<LayerPlanIR>>) -> Self;
    pub fn seam_plan(self, v: Option<Arc<SeamPlanIR>>) -> Self;
    pub fn support_plan(self, v: Option<Arc<SupportPlanIR>>) -> Self;
    pub fn paint_regions(self, v: Option<Arc<PaintRegionIR>>) -> Self;
    pub fn region_map(self, v: Option<Arc<RegionMapIR>>) -> Self;
    pub fn mesh_segmentation(self, v: Option<Arc<MeshSegmentationIR>>) -> Self;
    pub fn support_geometry(self, v: Option<Arc<SupportGeometryIR>>) -> Self;
    // ... one setter per remaining optional field ...

    pub fn build(self) -> HostExecutionContext;
}
```

**Terminal:** `build(self) -> HostExecutionContext` (no `Result`).

**Defaults for unset fields:** all `None`, empty output accumulator
defaults via `Default::default()` (the SDK output accumulators already
have `Default`).

### 6.5 `DecimateConfig` — `crates/slicer-helpers/src/decimate.rs:8`

4 fields. Bucket C (rather than B) because the validation rule
"exactly one of `target_count` and `target_ratio` is set" is currently
deferred to `decimate()` (line 75–87) and surfaces as
`DecimateError::InvalidConfig`. Migrating to a validating builder
moves this check earlier — at construction — so misuse fails before
the mesh data is loaded.

**Builder:**

```rust
pub struct DecimateConfigBuilder {
    target_count: Option<usize>,
    target_ratio: Option<f32>,
    max_error: f32,
    aggressive: bool,
}

impl DecimateConfigBuilder {
    pub fn new() -> Self;

    pub fn target_count(self, n: usize) -> Self;
    pub fn target_ratio(self, ratio: f32) -> Self;
    pub fn max_error(self, e: f32) -> Self;
    pub fn aggressive(self, b: bool) -> Self;

    pub fn build(self) -> Result<DecimateConfig, DecimateError>;
}

impl Default for DecimateConfigBuilder {
    fn default() -> Self {
        Self { target_count: None, target_ratio: None, max_error: 0.01, aggressive: false }
    }
}
```

**Validation in `build`:**
- `(target_count, target_ratio) == (None, None)` → `Err(InvalidConfig("neither target set"))`
- `(target_count, target_ratio) == (Some, Some)` → `Err(InvalidConfig("both targets set"))`
- `max_error <= 0.0` → `Err(InvalidConfig("max_error must be > 0"))` (new check, currently absent)

**Setter style:** consuming. Call sites build the config once at the
top of a slice run; no loop-conditional pattern observed.

**`decimate()` impact:** the function continues to accept `DecimateConfig`
by value, but the validation block at lines 75–87 is deleted (validation
now happens at construction).

## 7. Bucket D — untouched

167 structs total. Categories below; each category has one bullet
explaining why no migration is appropriate, followed by the per-struct
list.

### 7.1 Error enums and structs (~38)

Errors are constructed at exactly one site each (the `Err(...)` they
emit) and consumed once at the caller. Adding `Default` produces no
useful default value — every error variant carries situation-specific
context. No builder needed.

`slicer-host`:
`BlackboardError`, `LayerArenaError`, `CliError`, `ConfigResolutionError`,
`ConfigValidationError`, `ConfigValidationErrorKind`, `ConfigSchemaParseError`,
`ConfigSchemaParseErrorKind`, `DispatchError`, `ConfigSourceParseError`,
`LiveModuleLoadError`, `ExecutionPlanError`, `LayerStageError`,
`LayerExecutionError`, `FinalizationError`, `LayerSliceError`,
`LoadError`, `LoadErrorKind`, `MeshAnalysisError`, `ModelLoadError`,
`PipelineError`, `PostpassError`, `PrepassExecutionError`,
`ProgressError`, `SchedulerError`, `WasmLoadError`,
`WasmCallError`, `InstancePoolError`, `MeshSegmentationError`,
`PaintSegmentationError`, `RegionMappingError`, `RegionMappingBuiltinError`,
`SlicePostProcessPaintAnnotationError`, `SupportGeometryBuiltinError`,
`UnknownSemanticWarning`.

`slicer-sdk`: `ModuleError` (`src/error.rs:12`), `HostUnavailable`
(`src/host.rs:89`).

`slicer-helpers`: `DecimateError`, `RepairError`, `StepImportError`.

`slicer-core`: `PolygonSimplicityError` plus internal triangle-slicer error enum.

`slicer-ir`: 0 — IR is a pure data crate, no errors defined.

### 7.2 Unit / ZST glue structs (~32)

Constructed once globally (or via `<Type>::default()` returning the
type itself). Already trivially Default-able; no migration warranted.

`slicer-host`:
- `NoopInstrumentation`, `NoopLayerProgressSink` (unit; trait-witness stubs)
- 24 `*OutputBuilderData` / `*OutputData` ZSTs in `crates/slicer-host/src/wit_host.rs:171–1066` (dispatch-internal accumulators driven by the WIT codegen). Bucket D blanket assignment.
- Private `PeakCounter(AtomicU32)` and `AtomicCounter(AtomicU32)` newtypes in `crates/slicer-host/src/report/collector.rs:75,99`
- Private `*Runner` unit structs in `crates/slicer-host/src/main.rs` (4)

`modules/core-modules`:
- `MeshSegmentation;` (`mesh-segmentation/src/lib.rs`)
- `PaintRegionAnnotator;` (`paint-region-annotator/src/lib.rs`)
- `PaintSegmentation;` (`paint-segmentation/src/lib.rs`)

`test-guests` / `wit-guest`: every `Component;` glue struct (~17) in
`test-guests/*/src/lib.rs` and `modules/core-modules/*/wit-guest/src/lib.rs`.

### 7.3 Module-entry types using `from_config` (28 structs)

Every core-module's top-level struct
(`ArachnePerimeters`, `ClassicPerimeters`, `FuzzySkinModule`,
`GyroidInfill`, `DefaultLayerPlanner`, `LightningInfill`, `PartCooling`,
`PathOptimizationDefault`, `RectilinearInfill`, `SeamPlacer`,
`SeamPlannerDefault`, `SkirtBrim`, `SupportPlanner`, `SupportSurfaceIroning`,
`TopSurfaceIroning`, `TraditionalSupport`, `TreeSupport`, `WipeTower`)
is constructed exclusively by the host via the orthogonal
`from_config(config: &ConfigView) -> Result<Self, ModuleError>` path.
Adding `Default` would create an invalid module state; adding a builder
would duplicate the `from_config` API. Out of scope (§3.4).

Plus private internal helpers inside those modules:
- `Ray` (`arachne-perimeters/src/lib.rs`)
- `Rng` (`fuzzy-skin/src/lib.rs`)
- `BBox2D` (`skirt-brim/src/lib.rs`, `top-surface-ironing/src/lib.rs`)
- `ObjectPlan`, `MergedLayer` (`layer-planner-default/src/lib.rs`)
- `ParsedMark` (`mesh-segmentation/src/lib.rs`)
- `ToolChangeRecord` (`path-optimization-default/src/lib.rs`)
- `PlannedSupportNode`, `LayerCollisionCache` (`support-planner/src/lib.rs`)

### 7.4 Type aliases (not structs)

For completeness: the `slicer-ir` `pub type` aliases
(`ObjectId`, `ModifierId`, `ModuleId`, `SurfaceGroupId`,
`BridgeRegionId`, `OverhangRegionId`, `RegionId`, `StageId`, `ConfigKey`)
are aliases for `String` or `u64`. They are not structs; they take
their `Default` from the underlying type. Not counted in the 271 total.

### 7.5 Structs with no-safe-default enum field (11 in slicer-ir)

`PaintStroke` (`PaintSemantic`, `PaintValue`),
`PaintLayer` (`PaintSemantic`),
`ModifierVolume` (`ModifierScope` — `AllFeatures` arguably safe, but
default-set modifier would silently affect every feature; flag in §11),
`ScoredSeamCandidate` (`SeamReason` enum),
`SeamCandidate` (`SeamReason` enum),
`SupportPlanEntry` (carries `ExtrusionPath3D` with no-safe-default `ExtrusionRole`),
`SemanticRegion` (`PaintValue`),
`ExtrusionPath3D` (`ExtrusionRole`),
`WallLoop` (`LoopType`, `WallBoundaryType`),
`PrintEntity` (`ExtrusionRole`, `ExtrusionPath3D`),
`LayerAnnotation` (`LayerAnnotationKind`).

These structs stay Bucket D until either (a) the enum receives `#[default]`
per §3.6 (rejected for these enums), or (b) a follow-up migration
introduces test-helper builders in `slicer-test`.

### 7.6 Held-by-pattern: contain trait objects (3 in slicer-host)

`Blackboard` (`crates/slicer-host/src/blackboard.rs:56`) — has private
fields and is constructed via `new(mesh_ir: Arc<MeshIR>, layer_count: usize)`.
Documented constructor matches the type's role (host-owned blackboard
created once per slice). `Default` would require an `Arc<MeshIR>`
from somewhere — no sensible empty value exists.

`PipelineStageRunners` (`pipeline.rs:25`), `PipelineConfig` (`pipeline.rs:41`)
— hold trait objects (`Box<dyn LayerStageRunner + Sync>`, etc.). Trait
objects have no `Default`. These structs are constructed in `main.rs`
once per run; adding a builder would multiply the trait-object plumbing
without removing a real pain point.

### 7.7 `slicer-schema` const-only structs (3)

`StageSpec`, `ExportBinding`, `SlicerModuleSchema` —
constructed exclusively as `pub const` arrays in `crates/slicer-schema/src/lib.rs`
and from inside the `#[slicer_module]` proc-macro expansion. Never
constructed at runtime. `Default` is meaningless; builder would be
unused.

### 7.8 `slicer-test` test crate (8 structs)

Per the brief's scope rule, test crate structs are inspected for
context but are not migration candidates. The existing fixture
builders (`ConfigViewBuilder`, `SliceRegionViewBuilder`,
`PerimeterRegionViewBuilder`, etc.) already follow the convention this
spec proposes to adopt elsewhere.

### 7.9 `slicer-cli` (1 struct)

`Cli` (`cli/slicer-cli/src/main.rs:18`) — clap-driven, single field
(`#[command(subcommand)]`). No migration warranted.

### 7.10 `slicer-host` data-class structs not yet migrated (~13)

Listed for completeness; left in Bucket D because each is constructed
at <2 sites or has no clear ergonomic win:

`HostRunOptions` (clap-derived; single construction site at CLI parse),
`HostCli`, `HostCommands` enum, `LiveModuleBinding`, `LiveModuleLoadOutput`,
`ExecutionPlanRequest`, `ExecutionModuleBinding`, `SortedStageModules`,
`SerialEdge`, `ClaimHolder`, `ModuleAccessAudit`,
`SlicePostProcessPaintAnnotationRequest`, `SlicePostProcessPaintAnnotationResult`,
`SlicePostProcessPaintAnnotationWarning`, `FacetAnnotationRecord`,
`SurfaceGroupRecord`, `MeshAnalysisAuxiliary`,
`DagValidationRequest`, `DagValidationDiagnostic`, `FillHolders`, `StageDag`,
`ProgressEvent`, `TopContributor`, `EdgeTo`, `ModuleNode`,
`DispatchPhase`, `TierKind`, `Phase`,
`JsonLinesEmitter`, `RuntimeProgressSink`,
`LayerArenaSlot`, `BlackboardPrepassSlot`, `WasmInstance`, `WasmComponent`,
`WasmInstanceLease`, `MemTracker`, `AccountingAllocator`, `MemStats`,
`InfillOutputCollected`, `PerimeterOutputCollected`, `SupportOutputCollected`,
`GcodeOutputCollected`, `SlicePostprocessCollected`, `ThumbnailAwareSerializer`,
`LayerCollectionViewData`. (Some are `#[derive(Default)]` already; spec
keeps the current state.)

If any of these surfaces a real pain point during the migration, the
chunk PR proposing it can move it from D to A/B in-place with a one-line
spec addendum — these moves are non-breaking.

### 7.11 `slicer-helpers` (2)

`Vertex([f32; 3])` (private newtype) — Bucket D (newtype, single use).
The error types in §7.1 cover the rest.

### 7.12 `slicer-core` internal (5)

`AabbTree` private fields (used via `pub fn new`), `LinesDistancer2D`
internals, private structs in `triangle_mesh_slicer.rs` and `polygon_ops.rs`.

### 7.13 SDK output accumulators (already untouched per §3.3)

`InfillOutputBuilder`, `PerimeterOutputBuilder`, `SupportOutputBuilder`,
`SlicePostprocessBuilder`, `GcodeOutputBuilder`, `FinalizationOutputBuilder`,
`LayerCollectionBuilder`, plus prepass `*Output` types
(`MeshAnalysisOutput`, `LayerPlanOutput`, `MeshSegmentationOutput`,
`PaintSegmentationOutput`, `SeamPlanningOutput`, `SupportGeometryOutput`).

## 8. API impact (semver)

Adding `Default` is non-breaking (patch). Removing public fields,
removing public `new()` factories, and adding terminal `build()`
methods are breaking. Per §3.2 the Bucket C migrations are intentional
MAJOR bumps; no Bucket A/B migration is breaking.

### 8.1 `slicer-ir` (re-exports 67 structs at crate root)

| Struct | Change | Semver |
|---|---|---|
| All 39 Bucket A structs in §4.1 | Add `#[derive(Default)]` | patch |
| 13 Bucket B structs in §5.1 needing new manual `Default` impls | Add `impl Default` | patch |
| 4 existing-impl Bucket B structs (`SurfaceClassificationIR`, `SliceIR`, `ResolvedConfig`, `ConfigView`) | No change | none |
| `RetractMode` enum | No change (already has `#[default]`) | none |
| `WallGenerator`, `InfillType`, `SupportType` enums | Add `#[default]` per §3.6 | patch (new trait impl on existing public enum) |
| 11 Bucket D structs (§7.5) | No change | none |
| **Net** | | **patch** |

**No MAJOR semver bump on `slicer-ir`.** Every IR-crate change is
additive — `Default` impls and `#[default]` enum variants.

### 8.2 `slicer-host` (re-exports ~68 structs at crate root)

| Struct | Change | Semver |
|---|---|---|
| 12 Bucket A structs (§4.2) | No change (existing derives) | none |
| 8 Bucket B structs (§5.2) | No change (existing impls) | none |
| **`LoadedModule`** (§6.1) | Pub fields → `pub(crate)`; `new` removed; `LoadedModuleBuilder` added | **MAJOR** |
| **`CompiledModule`** (§6.2) | Pub fields → `pub(crate)`; `new` removed; `CompiledModuleBuilder` added | **MAJOR** |
| **`ConfigFieldSchema`** (§6.3) | Pub fields → `pub(crate)`; `ConfigFieldSchemaBuilder` added | **MAJOR** |
| **`HostExecutionContext`** (§6.4) | Pub fields → `pub(crate)`; `new` removed; `HostExecutionContextBuilder` added | **MAJOR** |
| All Bucket D | No change | none |

**`slicer-host` next release is MAJOR.** The host crate is consumed
within this workspace only (no public Cargo.io publish today), so the
blast radius is bounded by in-tree call sites updated in the same PRs.

### 8.3 `slicer-sdk` (re-exports 3 structs at root)

| Struct | Change | Semver |
|---|---|---|
| 14 Bucket A structs (§4.3) | Add `#[derive(Default)]` + remove `new()` factories | **MAJOR** |
| 2 Bucket B structs in §5.3 (`SliceRegionView`, `PerimeterRegionView`) | Add `impl Default` + remove `new()` and `with_boundary_paint()` factories | **MAJOR** |
| SDK FacetClass enum gets `#[default = Normal]` | New trait impl | patch |
| All Bucket D | No change | none |

**Decision resolved in §11 Q6 (2026-05-17):** SDK `new()` factories
across `slicer-sdk/src/views.rs` and `slicer-sdk/src/prepass_types.rs`
are **removed** in TASK-200e, matching the global locked rule that
no positional factory survives alongside `Default + struct-update`.
Pub fields stay pub so callers migrate by editing to
`{ field, ..Default::default() }`. Every core-module test fixture
and every in-tree caller is updated in the same PR. **`slicer-sdk`
next release: MAJOR.**

### 8.4 `slicer-helpers` (re-exports 6 structs at crate root)

| Struct | Change | Semver |
|---|---|---|
| `RepairResult`, `DecimateResult`, `NamedMesh`, `StepImportResult` | Add `#[derive(Default)]` | patch |
| `RepairStats` | No change (existing derive) | none |
| **`DecimateConfig`** (§6.5) | Pub fields → `pub(crate)`; existing `impl Default` removed; `DecimateConfigBuilder` added; `decimate()` validation moves to `build()` | **MAJOR** |

**`slicer-helpers` next release is MAJOR.** One breaking type. The
alternative (keep `DecimateConfig` in Bucket B, leave validation in
`decimate()`) is non-breaking but loses the earlier-failure benefit;
spec **commits** to the major bump per §3.2.

### 8.5 Non-breaking-alternative summary

The locked decision is "builder-only for migrated structs" — every
Bucket C is MAJOR. For each MAJOR struct, the spec acknowledges the
patch-semver alternative (keep `new()` alive alongside the builder)
and explicitly rejects it to preserve a clean end state. The Risks
section §11 captures the trade-off so future maintainers can revisit
on a per-struct basis if migration churn turns out to be heavier than
projected.

## 9. Migration order

5 PR-sized chunks. Each chunk is independently mergeable; each chunk
leaves the workspace green on `cargo check --workspace` and the chunk's
narrow tests; full `cargo test --workspace` runs at chunk close via
sub-agent dispatch.

Dependencies between buckets:
- Chunk 1 (slicer-ir Bucket A POD) is a prerequisite for Chunk 2
  (slicer-ir Bucket B), because the latter's manual `Default` impls
  call `<ChildType>::default()`.
- Chunk 3 (slicer-helpers `DecimateConfig` builder) is independent of
  Chunks 1–2 (slicer-helpers does not depend on the slicer-ir
  schema-pinned IRs at construction time).
- Chunks 4 and 5 (slicer-host builders) can run after Chunks 1–2 land,
  in either order. The spec orders them by call-site count
  (LoadedModule first) to maximise compound velocity.

### Chunk 1 — `slicer-ir` Bucket A POD types (~15 structs, leaves only)

Add `#[derive(Default)]` to the field-less and small leaf structs in
`crates/slicer-ir/src/slice_ir.rs`: `Point2`, `Point3`, `BoundingBox3`,
`Transform3d`, `IndexedTriangleSet`, `SemVer`, `ObjectConfig`,
`FacetPaintData`, `ConfigDelta`, `SupportGeometryKey`, `RegionKey`,
`Polygon`, `ExPolygon`, `WidthProfile`, `ToolChange`, `ZHop`,
`PrintMetadata`, `NonPlanarShellRef`, `ObjectLayerRef`,
`LayerPaintMap`, `FacetPaintMark`, `OverhangRegion`,
`ObjectSurfaceData`, `Point3WithWidth`, `WallFeatureFlags`,
`SeamPosition`, `TravelMove`, `TravelRetract`, `InfillRegion`,
`ModuleInvocation`, `RegionPlan`, plus add `#[default]` to
`WallGenerator::Classic`, `InfillType::Grid`, `SupportType::Traditional`.

This unblocks Chunk 2's manual `impl Default` for composite types.

**Estimated diff:** +40 / -0 LoC.

### Chunk 2 — `slicer-ir` Bucket B (schema-versioned IRs) + slicer-ir Bucket A composites

Two halves:

**2a.** Add missing `CURRENT_*_SCHEMA_VERSION` constants and manual
`Default` impls for the 13 schema-versioned IRs in §5.1
(`MeshIR`, `LayerPlanIR`, `SeamPlanIR`, `SupportPlanIR`, `SupportGeometryIR`,
`PaintRegionIR`, `MeshSegmentationIR`, `RegionMapIR`, `PerimeterIR`,
`InfillIR`, `SupportIR`, `LayerCollectionIR`, `GCodeIR`).

**2b.** Add `#[derive(Default)]` to the composite IR structs that
depend on Chunk 1 leaves: `ObjectMesh`, `SurfaceGroup`, `BridgeRegion`,
`ActiveRegion`, `GlobalLayer`, `SeamPlanEntry`, `SlicedRegion`,
`PerimeterRegion`.

**Then sweep call sites** in `slicer-host`, `core-modules`, `slicer-test`,
and tests to replace verbose struct literals with `..Default::default()`.
This is where the 1200-line reduction comes from. Use a focused codemod
script or hand edits per call-site cluster.

**Estimated diff:** +280 / -1200 LoC (net negative).

### Chunk 3 — `slicer-helpers` `DecimateConfig` builder

Standalone, independent of slicer-ir changes. Smallest builder to
validate the convention end-to-end before tackling the larger
`slicer-host` builders.

Add `DecimateConfigBuilder` per §6.5. Remove `DecimateConfig::default`,
demote pub fields to `pub(crate)`. Move validation from `decimate()`
into `build() -> Result<_, DecimateError>`. Update all in-tree call
sites in `slicer-helpers/tests/*`. Major-bump `slicer-helpers`.

**Estimated diff:** +120 / -40 LoC. **Major bump.**

Also in this chunk: add `Default` derives to `DecimateResult`,
`RepairResult`, `NamedMesh`, `StepImportResult` (patch).

### Chunk 4 — `slicer-host` builders: `LoadedModule`, `CompiledModule`, `ConfigFieldSchema`

The high-density-of-call-sites cluster. Order within the chunk: build
each builder, then sweep all call sites in:

- `crates/slicer-host/tests/*` (107 LoadedModule + 98 CompiledModule sites)
- `crates/slicer-host/src/manifest.rs` (graduate the `LoadedModule` builder)
- `crates/slicer-host/src/execution_plan.rs` (CompiledModule build sites)
- `crates/slicer-host/src/config_schema.rs` (`FullConfigSchema::default` uses ConfigFieldSchema 30+ times)

Each struct's pub fields become `pub(crate)`; `new()` factories
removed; builders are the only path to construct. Major-bump `slicer-host`.

**Estimated diff:** +600 / -2800 LoC. **Major bump.**

### Chunk 5 — `slicer-host` `HostExecutionContext` builder + remaining sweep

`HostExecutionContextBuilder` per §6.4. Sweep the 45 construction sites
in `crates/slicer-host/src/wit_host.rs` and related dispatch code.

Plus the SDK-side `slicer-sdk/src/prepass_types.rs` Bucket A additions
(§4.3) — adding `Default` and removing redundant `new()` constructors
in favour of struct-update at call sites. Per §8.3, `slicer-sdk`
`SliceRegionView::new` and `with_boundary_paint` get
`#[deprecated(note = "use Default::default() + struct update")]`
rather than full removal, to keep the SDK semver at patch+deprecation.

**Estimated diff:** +220 / -700 LoC. **Major bump for slicer-host;
patch for slicer-sdk.**

## 10. Test strategy per chunk

Each chunk closes with `cargo test --workspace` dispatched to a
sub-agent per CLAUDE.md test discipline; narrow tests run during
implementation iterations.

### Chunk 1 verification

**Existing coverage:**
- `crates/slicer-ir/src/slice_ir.rs` doctests
- `cargo check --workspace` (every derive change must keep compile clean)

**New tests required before refactor:** none. Adding derives can be
verified by `cargo build --tests`.

**Narrow command:**
```
cargo test -p slicer-ir
cargo check --workspace
```

### Chunk 2 verification

**Existing coverage:**
- `crates/slicer-host/tests/layer_collection_builder_tdd.rs` exercises
  `LayerCollectionIR` construction
- `crates/slicer-ir/src/slice_ir.rs` doctests for `SliceIR::default`,
  `SurfaceClassificationIR::default`, `ResolvedConfig::default`
- Schema-version round-trip tests in `slicer-host/tests/*`

**New tests required before refactor:** one test per new `CURRENT_*_SCHEMA_VERSION`
constant asserting that `<IR>::default().schema_version ==
CURRENT_<IR>_SCHEMA_VERSION`. Add to `crates/slicer-ir/src/slice_ir.rs`
inside `#[cfg(test)] mod tests`.

**Narrow command:**
```
cargo test -p slicer-ir
cargo test -p slicer-host --test layer_collection_builder_tdd
cargo test -p slicer-host --test pipeline_tdd
cargo check --workspace
```

### Chunk 3 verification

**Existing coverage:**
- `crates/slicer-helpers/src/lib.rs:30-65` smoke tests for `DecimateConfig::default`

**New tests required before refactor:**
- `build()` validation tests: neither target set → `Err`; both targets set → `Err`; valid count → `Ok`; valid ratio → `Ok`; `max_error <= 0.0` → `Err`.
- Add to `crates/slicer-helpers/src/decimate.rs` in `#[cfg(test)] mod tests`.

**Narrow command:**
```
cargo test -p slicer-helpers
cargo check --workspace
```

### Chunk 4 verification

**Existing coverage:**
- `crates/slicer-host/tests/dag_validation_tdd.rs` (uses the test-private `LoadedModuleBuilder`)
- `crates/slicer-host/tests/manifest_ingestion_tdd.rs`
- `crates/slicer-host/tests/dispatch_tdd.rs`
- `crates/slicer-host/tests/runtime_wiring_tdd.rs`
- Round-trip tests through `crates/slicer-host/src/execution_plan.rs::dedup_tests`

**New tests required before refactor:**
- `LoadedModuleBuilder::new(...).build()` round-trips a sample manifest
  to confirm field parity with the prior struct literal pattern.
- `CompiledModuleBuilder::new(id, pool).build()` produces a working
  `CompiledModule` that round-trips through `build_execution_plan`.
- `ConfigFieldSchemaBuilder` smoke test: build a Float-with-Min-Max
  schema, assert all fields populated identically to the prior literal.

**Narrow command:**
```
cargo test -p slicer-host --test dag_validation_tdd
cargo test -p slicer-host --test manifest_ingestion_tdd
cargo test -p slicer-host --test dispatch_tdd
cargo test -p slicer-host --test runtime_wiring_tdd
cargo test -p slicer-host --lib execution_plan::dedup_tests
cargo check --workspace
```

### Chunk 5 verification

**Existing coverage:**
- `crates/slicer-host/tests/e2e_integration_tdd.rs`
- `crates/slicer-host/tests/layer_executor_tdd.rs`
- `crates/slicer-host/tests/prepass_executor_tdd.rs`
- `crates/slicer-host/tests/postpass_executor_tdd.rs`
- SDK prepass tests via `crates/slicer-sdk/tests/prepass_module_tdd.rs`

**New tests required before refactor:**
- `HostExecutionContextBuilder::new(id, z, h).build()` produces a
  context byte-equivalent to the prior literal at one canonical
  fixture site.
- SDK `FacetAnnotation::default()` smoke: confirms `Normal` classification.

**Narrow command:**
```
cargo test -p slicer-host --test e2e_integration_tdd
cargo test -p slicer-host --test layer_executor_tdd
cargo test -p slicer-sdk
cargo check --workspace
```

## 11. Risks and open questions

1. **Schema-version constants are missing for 13 IR types. RESOLVED 2026-05-17:**
   audit per-IR; pin each new constant to the IR's actual current
   effective version, not a blanket `1.0.0`. Concretely: TASK-200b
   greps the codebase (`*_tdd.rs` fixtures, `docs/02_ir_schemas.md`,
   `docs/14_deviation_audit_history.md`, `docs/DEVIATION_LOG.md`) for
   each IR's most recent version literal and pins the new `CURRENT_*_IR_SCHEMA_VERSION`
   to that value. Known non-`1.0.0` cases: `MeshIR` → `1.1.0` (per
   TASK-191 modifier-part routing), `RegionMapIR` → `1.1.0` (per
   TASK-181 paint-overrides additive field). The remaining 11 IRs
   default to `1.0.0` if the audit finds no explicit bump. Fixtures
   that hard-code mismatched literals are corrected in the same chunk.

2. **`Transform3d::default()` is a zero matrix, not an identity matrix.**
   `[f64; 16]: Default` produces 16 zeros. A degenerate transform.
   Spec recommends deriving `Default` anyway for ergonomics, plus adding
   `Transform3d::identity()` as a follow-up convenience. Tests that
   construct a `Transform3d` must explicitly use `identity()` or set
   the matrix — flag in chunk 1 PR description.

3. **`Point3WithWidth::default()` is `(0,0,0)` with zero width and zero
   flow factor.** Zero-width extrusion is semantically meaningless.
   Acceptable as a placeholder for test setup, but production code
   should never rely on the default. Add a doc note on the type
   warning future maintainers.

4. **`SemVer { 0, 0, 0 }` is a sentinel.** `SemVer::default()` produces
   it. IRs that pin to a real `CURRENT_*` constant override correctly,
   but raw construction (e.g. test fixtures asserting on version) may
   silently use the sentinel. Document in the doc comment.

5. **`FullConfigSchema::default` complexity (~200 lines).**
   `crates/slicer-host/src/config_schema.rs:128` builds the full
   project schema field-by-field. It works, but is hard to read.
   **Open question:** is a follow-up chunk worth refactoring it to
   use `ConfigFieldSchemaBuilder` (defined in §6.3) loop-bodied per
   schema group? Out of scope for this migration; flagged as a
   future packet.

6. **SDK `SliceRegionView::new` / `with_boundary_paint` deprecation
   vs. removal. RESOLVED 2026-05-17:** remove immediately, MAJOR bump
   `slicer-sdk`. Matches the global locked decision §3.2 ("builder-only
   for migrated structs" generalised to "no positional factory survives
   alongside `Default + struct-update`"). TASK-200e deletes
   `SliceRegionView::new`, `SliceRegionView::with_boundary_paint`,
   `PerimeterRegionView::new`, and every prepass-type `new(...)`
   constructor under `slicer-sdk/src/prepass_types.rs`; pub fields
   stay pub where they are today so callers migrate by editing to
   `SliceRegionView { object_id, region_id, polygons, ..Default::default() }`.
   Every core-module test fixture and every in-tree caller is updated
   in the same PR. **slicer-sdk next release: MAJOR.** §8.3 is updated
   to reflect this.

7. **No builder-derivation crate.** Every Bucket C builder is hand-rolled
   (~50–100 LoC each). Trade-off:
   - **Pro of hand-rolled:** no new dependency, full control over
     `#[must_use]` placement, full control over which setters take
     `impl Into<>`.
   - **Con of hand-rolled:** ~250 LoC of boilerplate across the 5
     builders; if a future Bucket C entry is added, that's another
     50–100 LoC.
   - **Alternative:** adopt `bon` (smallest, most idiomatic of the three
     popular crates today) workspace-wide. Adds one dependency. Not
     proposed in this spec because the user locked it out; flagged
     here as a possible future revisit if Bucket C grows.

8. **`ExtrusionPath3D` and `PrintEntity` stay Bucket D.** These are
   constructed in hot paths (the WIT output accumulators emit them).
   Test fixtures complain loudest about this — they're forced to spell
   out `role: ExtrusionRole::OuterWall` even when role is irrelevant
   to the test. **Recommended follow-up (not in this spec):** add
   `slicer-test::fixtures::extrusion_path()` / `print_entity()`
   convenience constructors that pick canonical placeholder values
   for the no-safe-default enum fields. Keeps the Bucket D assignment
   intact for production paths while reducing test pain.

9. **`Blackboard::new(mesh_ir, layer_count)` stays.** Blackboard is
   constructed once per slice run; the constructor pattern is fine.
   **Risk:** if a future packet wants per-layer parallel blackboards
   or a sparse layer-output mode, the 2-param constructor will run out
   of arity. Flag for revisit if/when that comes up; not migrating
   pre-emptively.

10. **`HostRunOptions` (5 fields, clap-derived) stays Bucket D.** Spec
    classifies it as D because clap already does the construction work.
    If we move it to A/B, the clap derive and the `Default` derive would
    fight over whose default takes precedence. Conservative call;
    revisit if it becomes a pain point.

11. **No regression on hot paths. RESOLVED 2026-05-17:** TASK-200d's
    acceptance gate runs `cargo bench -p slicer-host --bench pipeline`
    once on the chunk's base commit and once on the migration head;
    PR description captures both numbers. A regression > 5% blocks
    merge until investigated. The `compiled-module` and
    `HostExecutionContext` builders are on the per-dispatch hot path
    and should compile to identical assembly to today's struct
    literals under release LTO; the bench guards the assumption.
    Same protocol applies to TASK-200e (HostExecutionContext) — bench
    both before and after, capture both numbers in the PR.

12. **The 24 `*OutputBuilderData` ZSTs in `wit_host.rs` are dispatch-
    internal.** Confirmed Bucket D in §7.2. They are derived from the
    WIT codegen and not intended for direct construction. If the WIT
    codegen ever stops emitting them as `#[derive(Default)]`-friendly
    ZSTs, that's a separate fix.

13. **`SDK` `FacetClass` and `IR` `FacetClass` are different types.**
    SDK `FacetClass` (`slicer-sdk/src/prepass_types.rs:22`) is a flat
    enum with no field-bearing variants — gets `#[default = Normal]`
    safely. IR `FacetClass` (`slicer-ir/src/slice_ir.rs:313`) has
    variants `NearHorizontal { slope_angle_deg }` and `Overhang { angle_deg }`
    — defaulting to `Normal` would discard the angle data on a buggy
    miscategorisation. Spec keeps IR `FacetClass` without `#[default]`.
    No struct currently in Bucket A depends on this enum's Default,
    so the asymmetry is harmless.

14. **`compile_module_component` failure paths are unchanged.**
    `crates/slicer-host/src/execution_plan.rs:479` emits a warning
    diagnostic when a `.wasm` fails to compile and returns `None`. The
    new `CompiledModuleBuilder::wasm_component(None)` path stays
    compatible with this; verified in §6.2.

---

End of spec. Future maintainers: when a struct is added to a public
module, decide its bucket against §3 and append to §4–§7. The spec is
a living document; per-PR addenda are welcomed.
