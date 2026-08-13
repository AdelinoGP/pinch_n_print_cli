# Pinch 'n Print — Host Scheduler

**What this covers:** how the scheduler ingests manifests, builds and validates
the module DAG, freezes an execution plan, and runs the four execution phases
(prepass, per-layer, finalization, postpass) — plus RegionMapIR compilation and
claim resolution.

**Who it's for:** anyone changing scheduling, validation, or execution order, and
anyone debugging why a module was rejected or ordered a certain way.

**Prerequisites:** `01_system_architecture.md` for the stage tiers and
`03_wit_and_manifest.md` for the manifest fields the scheduler reads.

> **Reading this doc.** The Rust snippets below illustrate the scheduler's
> contracts and data flow. They are NOT literal copies of the production
> source — they elide error variants, instrumentation hooks, and lifetimes
> for clarity. For the authoritative implementation see:
>
> Planning (`slicer-scheduler` crate — wasmtime-free, extracted in packet 85):
>
> - `crates/slicer-scheduler/src/execution_plan.rs` — `ExecutionPlan`,
>   `CompiledStage`, `CompiledModuleStatic`, `CompiledModuleBuilder`.
> - `crates/slicer-scheduler/src/manifest.rs` — manifest parser + `LoadedModule`.
> - `crates/slicer-scheduler/src/validation.rs` — DAG validation passes.
> - `crates/slicer-scheduler/src/topology.rs` — `topological_sort`.
> - `crates/slicer-scheduler/src/dag.rs` — intra-stage DAG construction.
> - `crates/slicer-scheduler/src/dag_cli.rs` — `pnp_cli dag` introspection.
> - `crates/slicer-scheduler/src/config_resolution.rs` — config merge.
> - `crates/slicer-scheduler/src/stage_order.rs` — canonical `STAGE_ORDER`.
> - `crates/slicer-scheduler/src/module_search_path.rs` — manifest discovery.
> - `crates/slicer-scheduler/src/instrumentation.rs` — planning side (`EdgeReason`,
>   `SerialEdge`, `compute_serial_edges_for_stage`).
>
> Runtime / execution (`slicer-runtime` crate):
>
> - `crates/slicer-runtime/src/prepass.rs` — `execute_prepass` family.
> - `crates/slicer-runtime/src/layer_executor.rs` — `execute_per_layer` family.
> - `crates/slicer-runtime/src/layer_finalization.rs` — `execute_layer_finalization`.
> - `crates/slicer-runtime/src/postpass.rs` — `execute_postpass` family.
> - `crates/slicer-runtime/src/instrumentation.rs` — runtime side
>   (`PipelineInstrumentation` trait, `Phase`, `TierKind`,
>   `compute_serial_edges_from_compiled`).
>
> WASM hosting (`slicer-wasm-host` crate — extracted in packet 83):
>
> - `crates/slicer-wasm-host/src/traits.rs` — runner traits
>   (`PrepassStageRunner`, `LayerStageRunner`, `FinalizationStageRunner`,
>   `PostpassStageRunner`) and their `*StageInput<'a>` borrow-struct inputs.
> - `crates/slicer-wasm-host/src/host.rs` — the per-stage `bindgen!`
>   invocations (one per versioned stage package, remapped onto the
>   canonical shared type set per ADR-0002 and ADR-0045).
>
> Scheduler-no-wasmtime invariant (Packet 85): `slicer-scheduler` declares
> no dep on `slicer-wasm-host`, `slicer-runtime`, or `wasmtime`. Verify
> with `cargo tree -p slicer-scheduler --edges normal | grep wasmtime`
> (must be empty). This is what enables the ~6.8k LOC of planning logic
> to be unit-tested without instantiating any WASM component.

The scheduler has four phases, all completing before a single layer is sliced. Phases 1–3 are pure data transformation — no WASM executes until Phase 4.

```
Phase 1: Manifest Ingestion     (parse all .toml files)
Phase 2: DAG Construction       (build intra-stage dependency graphs)
Phase 3: DAG Validation         (claim conflicts, cycles, version checks)
Phase 4: Execution              (PrePass → Per-Layer parallel → PostPass)
```

---

## Phase 1 — Manifest Ingestion

```rust
pub struct LoadedModule {
    pub id:                    ModuleId,
    pub version:               SemVer,
    pub stage:                 StageId,
    pub ir_reads:              Vec<String>, // from manifest [ir-access].reads
    pub ir_writes:             Vec<String>, // from manifest [ir-access].writes
    pub claims:                Vec<String>, // from manifest [claims].holds
    pub requires_claims:       Vec<String>, // from manifest [claims].requires
    pub incompatible_with:     Vec<String>, // from manifest [compatibility].incompatible-with
    pub requires_modules:      Vec<ModuleId>, // from manifest [compatibility].requires
    pub min_host_version:      SemVer,      // from manifest [compatibility].min-host-version
    pub min_ir_schema:         SemVer,      // from manifest [compatibility].min-ir-schema
    pub max_ir_schema:         SemVer,      // from manifest [compatibility].max-ir-schema
    pub config_schema:         ConfigSchema,
    pub overridable_per_region:Vec<String>, // from manifest [config.overridable-per-region].keys
    pub overridable_per_layer: Vec<String>, // from manifest [config.overridable-per-layer].keys
    pub layer_parallel_safe:   bool,
    pub wasm_path:             PathBuf,
    pub placeholder_wasm:     bool,        // ≤8-byte stub; inert for dispatch (packet 181)
}
```

The authoritative definition is `LoadedModule` in
`crates/slicer-scheduler/src/manifest.rs`; the scheduler is wasmtime-free
(packet 85), so no `WasmInstance` is stored here — instances live in the
runtime's pools.

### Manifest ↔ Runtime Naming Map (Normative)

Manifest keys are kebab-case and table-scoped. `LoadedModule` stores normalized snake_case fields for runtime processing.

| Manifest key                           | Runtime field            |
|----------------------------------------|--------------------------|
| `[ir-access].reads`                    | `ir_reads`               |
| `[ir-access].writes`                   | `ir_writes`              |
| `[claims].holds`                       | `claims`                 |
| `[claims].requires`                    | `requires_claims`        |
| `[compatibility].incompatible-with`    | `incompatible_with`      |
| `[compatibility].requires`             | `requires_modules`       |
| `[config.overridable-per-region].keys` | `overridable_per_region` |
| `[config.overridable-per-layer].keys`  | `overridable_per_layer`  |

The manifest naming is canonical for author-facing docs and examples. Runtime field names are internal and must not appear in user-facing manifest examples.

Ingestion scans all module search paths and deserializes every `.toml`. TOML schema errors produce a structured `LoadError` with file path and field name. No module is silently skipped.

Ingestion is generalized over manifest source: a module may come from a disk
file or embedded TOML. `LoadedModule` carries a `ModuleProvenance` marker
(`External | Integrated`); claims and DAG machinery never inspects provenance.

### `[[region_split]]` Aggregation and Tied-Priority Diagnostic (Normative — Packet 92)

When ingestion completes, the scheduler aggregates the
`[[region_split]]` array entries from every loaded manifest into a single
canonical `BTreeMap<String, AggregatedRegionSplitEntry>` keyed by
semantic name and ordered by `(priority, name)`. The map is consumed by:

- `Phase 2` DAG construction (per-layer dispatch filter — see below).
- `PrePass::RegionMapping` builtin (cross-product expansion — see
  "RegionMapping (Builtin)" further down).

Per-manifest validation (Packet 92):

1. **Duplicate semantic within one manifest** → `LoadErrorKind::DuplicateRegionSplitSemantic`.
2. **`value_type = "scalar"`** → rejected at load time
   (`LoadErrorKind::ScalarValueTypeNotAllowedInRegionSplit`). Scalar
   paint values route through `segment_annotations` instead (see
   `docs/02_ir_schemas.md` IR 6 `SlicedRegion`).
3. **Community semantic with `priority < 1000`** → rejected
   (`LoadErrorKind::CommunityPriorityBelowFloor`). The `COMMUNITY_PRIORITY_FLOOR`
   is `1000`; core semantics (`material = 100`, `fuzzy_skin = 200`)
   are listed in `CORE_REGION_SPLIT_PRIORITIES`.
4. **Core semantic with `priority` ≠ registry value** → rejected
   (`LoadErrorKind::CorePriorityMismatch`).

Cross-manifest **tied-priority warning** (non-fatal): if two distinct
semantics from different manifests declare the same priority, a
`LoadDiagnostic { level: DiagnosticLevel::Warning, path, field, message }`
is appended to the diagnostics vec. The message names both semantics,
both manifest paths, the shared priority, and the lexicographic
tiebreaker order used to keep aggregation deterministic. Scheduler
operation continues; this is purely an author-facing nudge.

### IR Access Path Format (Normative)

`ir_reads` and `ir_writes` entries in manifests use dot-notation to name specific fields within an IR struct. The format is `<IRName>.<field>`, where `<IRName>` is the canonical IR short name (e.g., `PerimeterIR`, `LayerCollectionIR`) and `<field>` is a snake_case field name declared in the corresponding Rust struct in `crates/slicer-ir/src/`.

Examples:
- `PerimeterIR.regions` — the `regions` field of `PerimeterIR` (array of per-region slices)
- `PerimeterIR.resolved-seam` — the `resolved_seam` field of each `PerimeterRegion` (written by `seam-placer` via `push-resolved-seam`)
- `LayerCollectionIR.skirt-brim` — the `skirt-brim` field of `LayerCollectionIR` (written by skirt-brim finalization modules)
- `PerimeterIR.walls` — wall loop array within each perimeter region

Wildcards are not supported in this version. Each dot-terminated path is matched literally against runtime access audit paths generated at the WIT boundary.

Why sub-field specificity matters: declaring `PerimeterIR` as a whole grants access to every field in the struct, preventing other modules from writing non-overlapping sub-fields in the same stage without a claim conflict. Narrow declarations like `PerimeterIR.resolved-seam` let modules operate on non-overlapping fields within the same IR type without mutual exclusion.

Ingestion does **not** validate that a declared path exists in the IR schema — that check is performed by the IR schema itself at load time. Declaring a non-existent field path produces a manifest that passes ingestion but fails at Phase 3 DAG validation or at the WIT boundary runtime check when the module attempts the access.

---
### Stage ID Validation (during ingestion)

`stage` is validated against the canonical `STAGE_ORDER` set before DAG construction.
Unknown or misspelled stage identifiers are fatal and must not be silently ignored.

```rust
fn validate_stage_ids(module: &LoadedModule) -> Result<(), SchedulerError> {
    if STAGE_ORDER.contains(&module.stage) {
        Ok(())
    } else {
        Err(SchedulerError::UnknownStage {
            module: module.id.clone(),
            declared_stage: module.stage.clone(),
        })
    }
}
```

---

## Phase 2 — DAG Construction

### Fixed Stage Order (never changes at runtime)

The canonical stage list is `STAGE_ORDER` — a `&[&str]` of stage-id strings, not
an enum — in `crates/slicer-scheduler/src/execution_plan.rs`. The scheduler's
validation passes use this declared order for cross-stage dependency and
ordering checks. In declaration order:

```rust
pub const STAGE_ORDER: &[&str] = &[
    "PrePass::MeshAnalysis",
    "PrePass::LayerPlanning",
    "PrePass::SeamPlanning",         // optional; runs when a seam-planner module is loaded
    "PrePass::PaintSegmentation",
    "PrePass::RegionMapping",        // host-built-in, not a module stage
    "PrePass::Slice",                // host-built-in
    "PrePass::OverhangAnnotation",   // host-built-in; derives overhang from the committed SliceIR
    "PrePass::ShellClassification",  // host-built-in; annotates the committed SliceIR
    "PrePass::SupportGeometry",      // host built-in always runs; guest optional
    "PrePass::LightningTreeGen",     // host-built-in; commits only for lightning sparse fill
    "Layer::PaintRegionAnnotation",  // host-built-in; a module claiming this stage runs instead of the host
    "Layer::SlicePostProcess",
    "Layer::Perimeters",
    "Layer::PerimetersPostProcess",
    "Layer::Infill",
    "Layer::InfillPostProcess",
    "Layer::Support",
    "Layer::SupportPostProcess",
    "Layer::PathOptimization",
    // ── rayon join happens here ──────────────────────────────────────────
    // PostPass tier — all stages below are sequential, whole-print.
    // Full Vec<LayerCollectionIR> visible. Never parallelized.
    "PostPass::LayerFinalization",
    "PostPass::GCodeEmit",           // host-built-in
    "PostPass::GCodePostProcess",
    "PostPass::TextPostProcess",
];
```

Two caveats a reader must know:

- **PathOptimization** (packet 33): nearest-neighbour entity ordering is owned
  entirely by `path-optimization-default`; the host carries no entity-ordering
  fallback. When no module claims it on a layer,
  `LayerCollectionIR.ordered_entities` keeps the order produced by upstream
  per-layer stages (no reorder).
- **Declared vs. executed order.** The list above is the declared order the
  scheduler validates against. The prepass host built-ins actually *execute* in
  a different sequence (`Slice` → `OverhangAnnotation` → `ShellClassification` →
  `PaintSegmentation` → `SupportGeometry` → `LightningTreeGen`) — see
  `01_system_architecture.md` §"PrePass Stage Order", which documents the
  executed chain and the known divergence between the two.

  `PrePass::LightningTreeGen` (packet 137, ADR-0029) executes **after** the
  stages producing sparse-infill outlines and **before** `Layer::Infill`
  dispatch. It consumes the committed sparse-infill outlines (via the committed
  `SliceIR`) and is **skipped** (no commit — the blackboard `LightningTreeIR`
  slot stays `None`) when no region's `sparse_fill_holder` resolves to
  `lightning-infill`: the zero-cost promise from ADR-0029.

`PrePass::OverhangAnnotation` — populates `SurfaceClassificationIR.overhang_quartile_polygons` by diffing consecutive-layer footprints, mirroring OrcaSlicer's `detect_overhangs_for_lift` (`PrintObject.cpp`) which diffs consecutive `lslices`. It runs **strictly after `PrePass::Slice`** and reads the committed `SliceIR` (each object's final per-layer region polygons) rather than re-slicing the mesh — the object meshes are sliced exactly once, in `PrePass::Slice`. Host built-in (`host:overhang_annotation`). Since packet 193 the same host built-in additionally writes
`SurfaceClassificationIR.prev_layer_boundaries` — a `HashMap<u32, Vec<ExPolygon>>`
keyed by global layer index exactly like `overhang_quartile_polygons`,
populated by `commit_overhang_annotation_builtin` from the previous-layer
contours `annotate_overhangs` already computes for the diff. This is the
carrier that packet 193's `signed_distance_to_boundary` (stamped into
`Point3WithWidth.overhang_distance_mm`) measures against; it is a host built-in
addition with **no manifest `[ir-access]`, `[claims]` or `[stage]` change** — no
DAG edge moves.

### Intra-Stage DAG (within one stage)

```rust
pub fn build_intra_stage_dag(
    stage: StageId,
    modules: &[LoadedModule],
) -> Result<Vec<ModuleNode>, SchedulerError> {
    let stage_modules: Vec<_> = modules.iter()
        .filter(|m| m.stage == stage)
        .collect();

    let mut nodes: HashMap<ModuleId, ModuleNode> = stage_modules.iter()
        .map(|m| (m.id.clone(), ModuleNode {
            module_id: m.id.clone(),
            ir_reads:  m.ir_reads.iter().cloned().collect(),
            ir_writes: m.ir_writes.iter().cloned().collect(),
            edges_to:  vec![],
        }))
        .collect();

    // Auto-derive edges: if A writes what B reads, A → B
    let ids: Vec<_> = nodes.keys().cloned().collect();
    for a_id in &ids {
        for b_id in &ids {
            if a_id == b_id { continue; }
            let a_writes = nodes[a_id].ir_writes.clone();
            let b_reads  = nodes[b_id].ir_reads.clone();
            if a_writes.iter().any(|w| b_reads.contains(w)) {
                nodes.get_mut(a_id).unwrap().edges_to.push(b_id.clone());
            }
        }
    }

    // Explicit requires edges from manifests
    for m in &stage_modules {
        for req in &m.requires_modules {
            if nodes.contains_key(req) {
                nodes.get_mut(req).unwrap().edges_to.push(m.id.clone());
            }
        }
    }

    Ok(nodes.into_values().collect())
}
```

### Per-Layer Region-Split Dispatch Filter (Normative — Packet 92)

After the intra-stage DAG is sorted, each `LoadedModule` carries a
cached `region_split_semantics: HashSet<String>` on its
`CompiledModuleStatic` descriptor (the set of semantic names declared
in the module's `[[region_split]]` array). The host applies a per-layer
filter at dispatch time using this set; the granularity is per-(module
× layer), NOT per-(module × region):

- A module whose `region_split_semantics` is **empty** runs
  unconditionally (paint-transparent default — preserves pre-packet-92
  behaviour for every existing module).
- A module with a non-empty set `S` is **skipped on layer `L`** if NO
  region in `L`'s `RegionMapIR` entries has a `variant_chain` whose
  semantic ∈ `S`.
- **Conservative-allow edge case:** if the slice for `L` is `None`
  (rare; layer not yet sliced or filter consulted out of order), the
  filter conservatively allows the module to run rather than skipping
  it. This is the safe default; missing the run would silently drop
  output, missing the skip wastes a no-op call.

The filter helper is `module_invocation_allowed_on_layer(...)` (called
from `module_invocation_allowed_on_layer` in `crates/slicer-runtime/src/layer_executor.rs`). Filter cost is `O(|regions| × |S|)` per
dispatch decision; the `region_split_semantics` HashSet keeps the
inner check at O(1).

---

## Phase 3 — DAG Validation

All validation errors are structured and collected before any are surfaced to the user.

```rust
pub enum SchedulerError {
    NotImplemented,
    UnknownStage {
        module: ModuleId,
        declared_stage: StageId,
    },
    ClaimConflict {
        claim: String, module_a: ModuleId, module_b: ModuleId, scope: ConflictScope,
    },
    IncompatibleModules {
        declared_by: ModuleId, conflicting: ModuleId, reason: String,
    },
    MissingDependency {
        module: ModuleId, requires: ModuleId,
    },
    CyclicDependency {
        cycle: Vec<ModuleId>,
    },
    UnfulfilledRead {
        module: ModuleId, field: String, suggestion: Option<String>,
    },
    IrVersionIncompatible {
        module: ModuleId, ir_type: String, required: SemVer, available: SemVer,
    },
    HostVersionIncompatible {
        module: ModuleId, required: SemVer, available: SemVer,
    },
    StageMismatch {
        module: ModuleId, declared_stage: StageId, exported_fn: String,
    },
    WriteConflict {
        field:    String,
        module_a: ModuleId,
        module_b: ModuleId,
        stage:    StageId,
        /// True if an ordering could in principle be established by having
        /// one module declare a read on the conflicting field. Hints to the
        /// user which resolution option to use.
        orderable: bool,
    },
    // Non-fatal — logged as warning, does not block slicing
    DeadWrite {
        module: ModuleId, field: String,
    },
    UndeclaredAccess {
        module: ModuleId, access: AccessKind, path: String,
    },
    CrossStageDependency {
        module: ModuleId, requires: ModuleId,
    },
    TransitiveStageDependency {
        module: ModuleId, requires: ModuleId,
    },
}
```

The authoritative definition is `SchedulerError` in
`crates/slicer-scheduler/src/validation.rs`; the `WriteConflict` resolution
options for module authors are: (A) declare one module incompatible with the
other, (B) have module B declare it reads the field that module A writes,
establishing an explicit ordering, or (C) use a claim so only one can be
active per region at a time.

### Validation Passes (in order)

1. **Stage ID validation** — manifest `stage` must exist in `STAGE_ORDER`
2. **Global claim conflicts** — two enabled modules hold the same claim globally. For the four fill-role claims (`top-fill`, `bottom-fill`, `bridge-fill`, `sparse-fill`, introduced in packet 37) the pass rejects two modules holding the same fill-role claim for the same `(layer, object, region)` triple. A single module may hold multiple fill-role claims (e.g. `rectilinear-infill` holds all four by default). Per-region overrides may transfer a fill-role claim to a different module. **For symmetry, startup module dedup (`dedup_same_claim_modules` in `crates/slicer-scheduler/src/execution_plan.rs`) and the *global* arm of this pass both skip the four fill-role claim IDs (`validation::FILL_CLAIM_IDS`):** multiple modules legitimately declare the same fill claim and per-region resolution at dispatch time picks the active holder. The *per-region* arm (pass 3 below) still flags genuine `(layer, object, region)`-level collisions. See DEV-065 (2026-06-09) for the regression history.
3. **Per-region claim conflicts** — same claim remains after region-level filtering
4. **Incompatibility declarations** — explicit `incompatible-with` pairs
5. **Missing dependencies** — `requires` modules absent or disabled
6. **IR version compatibility** — module requires newer IR schema than host provides
7. **Cycle detection** — Kahn's algorithm per stage DAG
8. **Write conflicts** — two modules in the same stage both write the same IR field with no read-after-write ordering edge between them (see below)
9. **Unfulfilled reads** — module reads a field no upstream module or host writes
10. **Dead writes** — module writes a field no downstream module reads (warning only)
11. **Undeclared access** — module runtime read/write masks must be strict subsets of manifest declarations
12. **Cross-stage dependency legality** — module may not require a module from a later stage
13. **Transitive dependency legality** — transitive `requires` closure may not include later-stage modules
14. **Host version compatibility** — module's declared `min-host-version` must be `<=` the running host version (`env!("CARGO_PKG_VERSION")` of `slicer-runtime`). Fatal, same blocking tier as pass 6 (IR version compatibility); see `SchedulerError::HostVersionIncompatible` and `docs/11_operational_governance_and_acceptance_gate.md` §2 "Compatibility Policy" dimension 1. Closes DEV-026 gap (1).

### Call-Time Access Enforcement (Normative)

Validation pass 11 verifies declared masks statically. Runtime calls are still revalidated at the WIT boundary.

Runtime enforcement requirements:

- Every host read call checks requested path/semantic against `module.ir_reads`.
- Every output-builder commit checks written path against `module.ir_writes`.
- Violations are fatal contract errors and are emitted as `module_error(status=fatal_error)`.
- Enforcement must be identical for SDK-based modules and raw WIT callers.

This dual-layer design prevents privilege escalation through custom bindings while preserving startup diagnostics quality.

### Claim Resolution with Runtime Disable Rules

> This is the authoritative reference for runtime claim resolution. The claim
> concept and the normative Allowed Claim Transition Matrix live in
> `docs/01_system_architecture.md` § "Claim System"; the known-claim catalog and
> manifest `[claims]` syntax live in `docs/03_wit_and_manifest.md`.

Claims are evaluated only over modules that remain enabled after config filtering.

```rust
// Illustrative. The real claim-conflict validation pass is
// `validate_claim_conflicts` in `crates/slicer-scheduler/src/validation.rs`.
fn effective_claim_holders(
    claim: &ClaimId,
    modules: &[LoadedModule],
    cfg: &ResolvedConfig,
) -> Vec<ModuleId> {
    modules.iter()
        .filter(|m| m.claims.contains(claim))
        .filter(|m| !config_disables_module(cfg, &m.id))
        .map(|m| m.id.clone())
        .collect()
}
```

Rules:

- Global validation fails only if `effective_claim_holders(claim).len() > 1`.
- A disabled module does not participate in claim conflicts.
- Region overrides may disable one holder and enable another; the region-level result must still be unique.
- If no holder remains for a required claim, this is a configuration error (`MissingDependency`/unfulfilled capability).
- Claim holder consistency is required per `(object_id, claim)` across all global layers.
- If region overrides produce claim holder transitions across layers for the same object, validation fails as non-deterministic.

Cross-stage transitive rule:

- If module `A` requires `B`, and `B` (directly or transitively) requires `C`, then `stage(C) <= stage(A)` must hold.
- Any violation is fatal even when the direct dependency appears legal.

### Perimeter-generator selection (`wall_generator` dedup + spiral-vase fallback)

Both `com.core.classic-perimeters` and `com.core.arachne-perimeters` declare
`holds = ["perimeter-generator"]` and are mutually `incompatible-with` each
other. Two modules holding the same non-fill claim would normally be a fatal
startup conflict, but the `perimeter-generator` claim is resolved *before*
`validate_startup_dag` runs, at module-load dedup time, by
`dedup_same_claim_modules_with_wall_generator`
(`crates/slicer-scheduler/src/execution_plan.rs`, called from
`crates/slicer-runtime/src/run.rs`). Dedup keeps exactly one holder, so
`incompatible-with` never has a chance to fire.

Selection rules, in order:

1. **`wall_generator` config key** — read directly from the raw config source
   at module-load time (before `ResolvedConfig` exists) via
   `WALL_GENERATOR_CONFIG_KEY` / `DEFAULT_WALL_GENERATOR`. Values: `"classic"`
   (default) or `"arachne"`. `dedup_same_claim_modules_with_wall_generator`
   resolves the `perimeter-generator` claim by this key instead of alphabetical
   order, falling back to `classic` if the preferred module is not among the
   loaded candidates or the value is unrecognised. This closes
   the wall-generator selection record (before it, dedup silently kept the
   alphabetically-first candidate — `arachne-perimeters` — with no way for a
   user to express intent).

2. **Spiral-vase fallback (packet 151)** — when `spiral_vase = true`, the
   scheduler forces `com.core.classic-perimeters` as the `perimeter-generator`
   holder regardless of `wall_generator`. This mirrors OrcaSlicer, which gates
   Arachne dispatch on `wall_generator == Arachne && !spiral_mode`
   (canonical `LayerRegion.cpp`): spiral / vase
   mode produces a single continuous Z-ramped wall that the Arachne
   variable-width beading pipeline is not designed to emit, so upstream always
   falls back to the classic perimeter generator in vase mode. The fallback
   lives in the scheduler/runtime selection path, not in either perimeter
   module (gap G8, closed by packet 151).

Unlike the fill claims, `perimeter-generator` is a stable single-owner claim
(see the Allowed Claim Transition Matrix in `docs/01_system_architecture.md`):
the resolved holder must remain constant across every layer for a given object.

### Support-generator selection (`support_type` dedup)

Both `com.core.traditional-support` and `com.core.tree-support` declare
`holds = ["support-generator"]`. Like `perimeter-generator`, the claim is
resolved at module-load dedup time (before `validate_startup_dag` runs) by
`dedup_same_claim_modules_with_wall_generator`
(`crates/slicer-scheduler/src/execution_plan.rs`), so the two support modules
never trip the startup conflict for each other.

Selection rules:

1. **`support_type` config key** — read directly from the raw config source
   at module-load time (before `ResolvedConfig` exists) via
   `SUPPORT_GENERATOR_CONFIG_KEY` (`"support_type"`). Values are OrcaSlicer's
   raw spellings, carried in the 3MF sidecar next to the raw `enable_support`
   key: values starting with `tree` (`tree(auto)`, `tree(manual)`) or with
   `hybrid` (legacy `hybrid(auto)`, which OrcaSlicer itself migrates to
   `tree(auto)` at config load) select `com.core.tree-support`; absent values
   and everything else select `com.core.traditional-support`. Falling back to
   traditional matches the historical alphabetical winner, so configs without
   the key slice exactly as before. Manual (enforcer-only) variants select the
   same holder as their auto counterpart — pnp has no enforcer-only concept.

2. **Alphabetical fallback** — when the preferred module is not among the
   loaded candidates (e.g. a community module reusing the claim name), the
   first-winner alphabetical default applies, as for `perimeter-generator`.

Unlike the fill claims, `support-generator` is a stable single-owner claim:
the resolved holder must remain constant across every layer for a given object
(see the Allowed Claim Transition Matrix in `docs/01_system_architecture.md`).

### Write Conflict vs Claim Conflict — Enforcement Level Summary

These two mechanisms are complementary, not redundant. Understanding the
difference is important when designing modules that share IR fields.

|                      | Claim Conflict                                                   | Write Conflict                                                                                                           |
|----------------------|------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------|
| **What it detects**  | Two modules both intend to be the primary generator of a feature | Two modules both write the same IR field with no ordering between them                                                   |
| **Granularity**      | One claim name per feature category (coarse)                     | Per IR access path (fine)                                                                                                |
| **Caught at**        | Startup, validation pass 1–2                                     | Startup, validation pass 6b                                                                                              |
| **Typical cause**    | User enables two infill modules simultaneously                   | Developer adds a new PostProcess module that overwrites a field another module already modifies                          |
| **Resolution**       | Region overrides; disable one module                             | Declare `incompatible-with`, OR have one module read the field it will overwrite (establishing ordering), OR use a claim |
| **Runtime fallback** | None — fatal startup error                                       | None — fatal startup error                                                                                               |

A claim conflict always implies a write conflict on the claim's primary output
field. A write conflict does not always imply a claim conflict — two
PostProcess modules that both transform `PerimeterIR.walls.path` may both
legitimately run, but only if one reads the other's output first.

### Composable Multi-Writer Patterns (Normative)

To avoid tight coupling while keeping determinism:

- Prefer **transform chains** over exclusivity when modules are semantically additive.
- Use claims only for true single-owner generators (for example infill generator).
- A valid transform chain is: module A writes `F`, module B declares read `F` and write `F`, producing deterministic `A → B` ordering.
- If two modules are alternatives rather than transforms, use `incompatible-with` or a shared claim.
- Modules must not declare synthetic reads solely to force order unless they semantically consume the prior value.

### `DeadWrite` vs `WriteConflict`

| | `DeadWrite` | `WriteConflict` |
|---|---|---|
| **Severity** | Warning — does not block slicing | Error — blocks slicing |
| **Meaning** | A module writes a field no downstream module reads. The write has no effect on the output. Likely a manifest declaration error. | Two modules write the same field with no ordering. The result is non-deterministic. Always a bug. |
| **Common cause** | Module updated its implementation but forgot to update its manifest `ir-access.writes` | Two independently developed PostProcess modules targeting the same output field without knowing about each other |

Summary of Changes

### Topological Sort (Kahn's Algorithm)

```rust
pub fn topological_sort(
    nodes: &[ModuleNode],
) -> Result<Vec<ModuleId>, Vec<ModuleId>> {
    let mut in_degree: HashMap<ModuleId, usize> = nodes.iter()
        .map(|n| (n.module_id.clone(), 0usize))
        .collect();

    for node in nodes {
        for dep in &node.edges_to {
            *in_degree.get_mut(dep).unwrap() += 1;
        }
    }

    let mut queue: VecDeque<ModuleId> = in_degree.iter()
        .filter(|(_, &d)| d == 0)
        .map(|(id, _)| id.clone())
        .collect();

    let mut sorted = vec![];
    while let Some(id) = queue.pop_front() {
        sorted.push(id.clone());
        let node = nodes.iter().find(|n| n.module_id == id).unwrap();
        for dep in &node.edges_to {
            let d = in_degree.get_mut(dep).unwrap();
            *d -= 1;
            if *d == 0 { queue.push_back(dep.clone()); }
        }
    }

    if sorted.len() == nodes.len() {
        Ok(sorted)
    } else {
        let visited: HashSet<_> = sorted.iter().cloned().collect();
        Err(nodes.iter()
            .map(|n| n.module_id.clone())
            .filter(|id| !visited.contains(id))
            .collect())
    }
}

/// Validation pass 7: detect write conflicts within a stage.
///
/// A write conflict exists when modules A and B both write field F in the
/// same stage, and there is no directed path A→B or B→A in the stage DAG.
/// Without an ordering edge, the second writer silently overwrites the first
/// and the result is implementation-defined.
///
/// A valid multi-writer scenario: A writes F, B reads F and writes F.
/// This creates edge A→B (A's write satisfies B's read), establishing a
/// deterministic transformation chain. This is NOT a conflict.
fn validate_write_conflicts(
    nodes: &[ModuleNode],
    errors: &mut Vec<SchedulerError>,
    stage: StageId,
) {
    // Build a reachability map: can_reach[a][b] = true if there is a
    // directed path from a to b in the DAG.
    let reachability = compute_reachability(nodes);

    // For every pair of distinct modules (A, B) in this stage:
    for i in 0..nodes.len() {
        for j in (i + 1)..nodes.len() {
            let a = &nodes[i];
            let b = &nodes[j];

            // Find fields written by both A and B.
            let shared_writes: Vec<IrAccessPath> = a.ir_writes.iter()
                .filter(|w| b.ir_writes.contains(w))
                .cloned()
                .collect();

            for field in shared_writes {
                // Check if an ordering exists between A and B.
                let a_before_b = reachability[&a.module_id][&b.module_id];
                let b_before_a = reachability[&b.module_id][&a.module_id];

                if !a_before_b && !b_before_a {
                    // No ordering — this is a conflict.
                    // Determine if it is orderable: would B reading the field
                    // establish an A→B edge?
                    let orderable = b.ir_reads.contains(&field)
                        || a.ir_reads.contains(&field);

                    errors.push(SchedulerError::WriteConflict {
                        field,
                        module_a: a.module_id.clone(),
                        module_b: b.module_id.clone(),
                        stage,
                        orderable,
                    });
                }
                // If a_before_b or b_before_a: ordering exists, no conflict.
                // The later module's write is an intentional transformation
                // of the earlier module's output.
            }
        }
    }
}

fn compute_reachability(
    nodes: &[ModuleNode],
) -> HashMap<ModuleId, HashMap<ModuleId, bool>> {
    // Floyd-Warshall over the DAG adjacency list.
    // O(N³) but N (modules per stage) is always small (< 20 in practice).
    let mut reach: HashMap<ModuleId, HashMap<ModuleId, bool>> = nodes.iter()
        .map(|n| {
            let row = nodes.iter()
                .map(|m| (m.module_id.clone(), n.edges_to.contains(&m.module_id)))
                .collect();
            (n.module_id.clone(), row)
        })
        .collect();

    let ids: Vec<ModuleId> = nodes.iter().map(|n| n.module_id.clone()).collect();
    for k in &ids {
        for i in &ids {
            for j in &ids {
                if reach[i][k] && reach[k][j] {
                    *reach.get_mut(i).unwrap().get_mut(j).unwrap() = true;
                }
            }
        }
    }
    reach
}
```

---

## RegionMapIR Compilation (PrePass::RegionMapping)

> This section owns how `RegionMapIR` is **built and bounded**. The struct shape
> and field semantics (`RegionPlan`, `RegionKey`, config-key namespaces, override
> precedence) are defined in `docs/02_ir_schemas.md` IR 5; `docs/01_system_architecture.md`
> describes why the stage exists.

`PrePass::RegionMapping` is host-built-in and precomputes per-region execution context so Tier 2 has no config or claim resolution overhead.

During region mapping, modifier volume `config_delta.fields` from every `modifier_volume` attached to a region's parent `ObjectMesh` are stamped into `RegionPlan.config.extensions` via `overlay_resolved` (priority-ascending, last-writer-wins), with `support_enforcer` and `support_blocker` subtypes filtered out for OrcaSlicer parity (canonical `PrintApply.cpp`). Implemented modifier-volume splits (packets 131/132) bind configuration to geometric sub-regions: modifier meshes are sliced per layer during prepass, the cross-sections are intersected with the owning region's partitioned fill polygons at partition time, and each resulting wall-less sub-region carries its own `region_id` + config binding (per-region delivery through the region-view config accessor). The support enforcer/blocker subtypes remain filtered and never produce sub-regions; paint variant-splits remain the other per-region producer.

### RegionMapping (Builtin) — `aggregated_region_split` Threading (Normative — Packet 93)

The region-mapping kernel signature is extended to consume
`aggregated_region_split: &BTreeMap<String, AggregatedRegionSplitEntry>`
from the execution plan. This map is the canonical aggregator output
populated by `slicer-scheduler::region_split::aggregate_region_splits`
at plan construction (see Phase 1 §`[[region_split]]` aggregation
above). The producer wrapper
(`crates/slicer-runtime/src/builtins/region_mapping_producer.rs`)
threads it from `ExecutionPlan` into `execute_region_mapping`. The
kernel uses it to:

1. Determine which paint semantics are opted-in per region (filter
   `MeshIR.objects[].paint_data.layers` against the keyset).
2. Drive cross-product expansion of `variant_chain` per `(layer,
   ActiveRegion)` — see `docs/02_ir_schemas.md` IR 5 § Config Interner.
3. Detect Scalar paint values defensively: any `PaintValue::Scalar`
   encountered in a region-split path becomes
   `RegionMappingError::ScalarInRegionSplitFacetValue` (Packet 93 guard, the
   manifest validator from Packet 92 normally catches it first).

`enumerate_canonical_chains` produces chains deterministically in
BTreeMap (semantic-name) order with `PaintValue` ordered as
`Flag < ToolIndex(0) < ToolIndex(1) < … < Custom(s_lex)`. This order
is contract — test fixtures and integration tests lock it.

**Cap and overflow:** `DEFAULT_REGION_MAP_CAP = 750_000` (raised from
`1_000` in Packet 93 to accommodate worst-realistic envelopes of 16
colors × 1000 layers × 16 regions × ~3 modifier subtypes). Overflow
surfaces `RegionMappingError::CapExceeded` naming
`top_contributor_object_id` so callers can diagnose which object
exploded the cross-product.

**Cross-crate dependency:** `slicer-core` depends on `slicer-scheduler`
for the `AggregatedRegionSplitEntry` type. Relocating the type to
`slicer-ir` to clean up the edge is a deferred follow-up; verify with
`cargo tree -p slicer-core --edges normal`.

```rust
// Illustrative. The real entry point is `execute_region_mapping` in
// `crates/slicer-core/src/algos/region_mapping.rs`, invoked from the
// `host:region_mapping` built-in producer.
fn build_region_map(
    layer_plan: &LayerPlanIR,
    modules: &[LoadedModule],
) -> RegionMapIR {
    let mut entries = HashMap::new();

    for layer in &layer_plan.global_layers {
        for region in &layer.active_regions {
            let key = RegionKey {
                global_layer_index: layer.index,
                object_id: region.object_id.clone(),
                region_id: region.region_id.clone(),
            };

            let mut stage_modules = HashMap::new();
            for stage in STAGE_ORDER.iter().copied() {
                if is_host_builtin(stage) { continue; }

                let active_for_stage = modules.iter()
                    .filter(|m| m.stage == stage)
                    .filter(|m| !config_disables_module(&region.resolved_config, &m.id))
                    .filter(|m| claims_allow_module(&region.resolved_config, &m.id, modules))
                    .map(|m| ModuleInvocation {
                        module_id: m.id.clone(),
                        config_view: build_config_view(m, &region.resolved_config),
                    })
                    .collect::<Vec<_>>();

                stage_modules.insert(stage, active_for_stage);
            }

            entries.insert(key, RegionPlan {
                config: region.resolved_config.clone(),
                stage_modules,
            });
        }
    }

    RegionMapIR {
        schema_version: CURRENT_IR_VERSION,
        entries,
    }
}
```

### `resolve_active_regions` Complexity Contract (Normative)

Per-layer execution must not rescan global config or claims. Region activation is O(1) lookup by `(global_layer_index, module_id)` into precomputed RegionMap indexes.

```rust
fn resolve_active_regions(
    layer: &GlobalLayer,
    module: &CompiledModule,
    blackboard: &Blackboard,
) -> &[ActiveRegionRef] {
    blackboard.region_map
        .module_region_index
        .get(&(layer.index, module.module_id.clone()))
        .map(Vec::as_slice)
        .unwrap_or(&[])
}
```

Any implementation with per-call filtering over all regions (`O(n_regions)`) is non-compliant.

### Layer Stage Dispatch ConfigView Sourcing (Normative — Packet 51)

`dispatch_layer_call` constructs a fresh `ConfigView` for each module
invocation by reading the **per-region `RegionPlan.config`** via
`blackboard.region_map()` and the current `(layer, object, region_id)`,
NOT from the module's `module.config_view` field that was bound at
load time. This ensures per-paint-semantic config overlays stamped
into `RegionPlan.config` during `PrePass::RegionMapping` are visible to
dispatched Layer-tier modules. The frozen-at-load `module.config_view`
is retained only for prepass and finalization stages where there is no
region-level overlay.

### PrePass Config-View Plumbing (Normative — Packet 73)

Every PrePass export (`layer-planning`, `seam-planning`,
`support-geometry`, `paint-segmentation`) receives a `config-view`
parameter providing read-only access to declared config keys, normalised
across stages by Packet 73 (the `support-geometry` runner was the final
holdout). Modules declaring no `[config.schema]` receive an empty
`ConfigView`. Config keys are looked up by string name; absent keys
return `None`. The `support-geometry` runner specifically:

- Now honours `enable_support` (false → planner is invoked but emits
  no plan; was previously discarded by an empty `ConfigView` injection).
- Surfaces planner fatals as `DispatchError` instead of swallowing them
  inside the macro/host glue (required by Packet 73 AC-N2).

### Required-Tool Fallback (Normative — Packet 68)

When the layer executor resolves required tool assignment for a region
and `dominant_tool_index()` returns `None` (no Material paint touches
the region's walls), the executor reads
`RegionPlan.config.extensions["extruder"]` as a `ConfigValue::Int(tool)`
fallback and uses that as `Some(tool)`. This is what makes
`extruder = N` in a 3MF `<object>`/`<modifier>` metadata block reach
G-code emit even on unpainted geometry — the value flows through:

1. 3MF sidecar metadata → `ObjectConfig.data` / `ModifierVolume.config_delta`.
2. `RegionMapping` stamps `extruder` into `RegionPlan.config.extensions`
   (subtype-key exclusion still applies to `support_enforcer` /
   `support_blocker` modifier subtypes; see `docs/02_ir_schemas.md`).
3. Layer executor's fallback reads `extensions["extruder"]` when
   `dominant_tool_index()` returns `None`.

Paint-derived `dominant_tool_index()` always wins when present (priority
order: `Material` paint > `extensions.extruder` > default `0`).

### RegionMapIR Memory Budget Contract (Normative)

Required bounds:

- Host must enforce a configurable cap on RegionMapIR entry count.
- Default cap: `DEFAULT_REGION_MAP_CAP = 750_000` entries (`crates/slicer-ir/src/slice_ir.rs`; raised from `1_000` in packet 93).
- Exceeding cap is a fatal planning error with actionable diagnostics.

Required representation guidance:

- `RegionPlan.config` should be shared (`Arc`/interned) when identical across entries.
- Implementations should avoid full config cloning per entry when equivalent views can be reused.

Minimum diagnostics on overflow:

- computed entry count
- configured cap
- top contributing `(object_id, region_count, layer_count)` tuples
- remediation hint (`reduce region granularity`, `raise cap`, or `split job`)

### LayerCollectionIR Lifecycle & Memory Strategy (Normative)

Required lifecycle:

1. Per-layer worker builds intermediate IRs in `LayerArena`.
2. Worker commits exactly one `LayerCollectionIR` into slot storage.
3. After rayon join, slot storage is drained into `Vec<LayerCollectionIR>`.
4. `LayerArena` memory is released before PostPass starts.

Memory policy requirements:

- Implementations must support a bounded in-memory mode and a spill-capable mode.
- In bounded mode, if projected peak memory exceeds configured limit, host must fail early with diagnostics.
- In spill-capable mode, completed layer outputs may be persisted to temporary storage and reloaded before PostPass.

Projection minimum inputs:

- `global_layer_count`
- `active_region_count`
- `configured parallel worker count`
- rolling average `LayerCollectionIR` size sample from early layers

### WASM Host-Call Batching Contract (Normative)

To keep boundary overhead proportional to region complexity:

- Host services must provide batch operations for geometry and paint queries.
- Module guidance is normative for hot paths: prefer one batched call per region over per-vertex calls.
- Scheduler diagnostics should include per-module host-call counts per stage.

Default soft budgets:

- target host-call count per module invocation: `<= 16`
- warning threshold: `> 64`
- error threshold (contract breach for performance gate fixtures): `> 256`

### Proactive Validation Points (Normative)

Validation must happen before expensive stage work whenever possible:

- Catch-up Z envelope compatibility checks at stage entry for Z-writing modules.
- Config type/range validation at planning time (not first failing layer).
- Coordinate precision guardrails on output commit for geometry-writing modules.

---

## Phase 4 — Execution

### Compiled Execution Plan (frozen, shared read-only across threads)

```rust
pub struct ExecutionPlan {
    pub prepass_stages:   Vec<CompiledStage>,
    pub per_layer_stages: Vec<CompiledStage>,
    pub layer_finalization_stage: Option<CompiledStage>,
    pub postpass_stages:  Vec<CompiledStage>,
    pub global_layers:    Arc<Vec<GlobalLayer>>,
    pub region_plans:     Arc<HashMap<RegionKey, RegionPlan>>,
    pub module_region_index: HashMap<(u32, ModuleId), Vec<ActiveRegion>>,
    // + aggregated region-split map (see Phase 1 §[[region_split]])
}

pub struct CompiledStage {
    pub stage_id: StageId,
    pub modules:  Vec<CompiledModuleStatic>,  // topologically sorted, iterate directly
}

pub struct CompiledModuleStatic {
    pub module_id:     ModuleId,
    pub ir_read_mask:  IrAccessMask,
    pub ir_write_mask: IrAccessMask,
    pub config_view:   Arc<ConfigView>,
    pub claims:        Vec<String>,   // frozen [claims].holds; feeds resolve_held_claims
    // + requires_modules, region_split_semantics, layer_parallel_safe
}
```

The authoritative definitions are `ExecutionPlan`, `CompiledStage`, and
`CompiledModuleStatic` in `crates/slicer-scheduler/src/execution_plan.rs`.
The scheduler is wasmtime-free (packet 85): no `WasmInstancePool` lives on
the plan — WASM instance pools are owned by the runtime
(`slicer-wasm-host`), keyed by module, sized by `layer_parallel_safe`
(N instances for parallel-safe modules, 1 for sequential).

### Runner-Trait Input Borrow Structs (Normative — Packet 83)

Runner trait signatures (`PrepassStageRunner::run_prepass`,
`LayerStageRunner::run_layer`,
`FinalizationStageRunner::run_finalization`,
`PostpassStageRunner::run_postpass`) accept IR-typed `*StageInput<'a>`
borrow structs rather than raw `&Blackboard` or `&LayerArena`. This
decouples the dispatcher (which lives in `slicer-wasm-host`) from
runtime-owned aggregates (which stay in `slicer-runtime`):

```rust
pub struct LayerStageInput<'a> {
    pub stage_id:    StageId,
    pub layer_index: u32,
    pub region:      &'a ActiveRegion,
    pub slice:       &'a SliceIR,
    pub perimeter:   Option<&'a PerimeterIR>,
    // … other field-level borrows the dispatcher reads
}

// PrepassStageInput<'a>, FinalizationStageInput<'a>,
// PostpassStageInput<'a> follow the same pattern.
```

Dispatch is provenance-routed: `CompiledModuleLive.native_entry` decides
whether a stage uses the native call path or WASM instantiation. The marshalling
boundary is shared between transports; the native path re-enters at the
`*OutputCollected` accumulator layer, so the `out.rs` converters and
`origin.rs` `OriginBucket` re-attribution run unchanged. Only the input-view
leg differs. Module logic is single-threaded on both paths (ADR-0056 Decision
item 5).

The orchestrator constructs the input struct at each dispatch call
site by projecting field-level borrows from `Blackboard` / `LayerArena`,
then hands it to the wasm-host's `instance.call_*` path. Errors from
the runner narrow to crate-local enums (e.g. `PrepassRunnerError`) in
`slicer-ir`; the broader `PrepassExecutionError` in `slicer-runtime`
implements `From<PrepassRunnerError>` with lossless variant remap.

Concurrent with Packet 83, `CompiledModule` was renamed
`CompiledModuleStatic` and a `CompiledModuleLive<'s>` borrow type was
introduced; Packet 85 completed the migration of wasmtime fields out
of Static and dropped the transitional `pub type CompiledModule =
CompiledModuleStatic` alias. See ADR-0005 and ADR-0007.

### Multiple `PostPass::LayerFinalization` Modules (Normative — Packet 88)

`PostPass::LayerFinalization` admits multiple modules in the same
stage (e.g. `overhang-classifier-default` + `part-cooling` +
`skirt-brim` + `wipe-tower`). Modules execute SEQUENTIALLY, ordered by
their claims' topological sort. Two modules MUST NOT claim the same
role (claim conflict → DAG validation failure). Example role split:

| Module                        | Holds claim                       |
|-------------------------------|-----------------------------------|
| `overhang-classifier-default` | `overhang-speed-factor`           |
| `part-cooling`                | `layer-cooling`                   |
| `skirt-brim`                  | `skirt`, `brim`                   |
| `wipe-tower`                  | `wipe-tower`, `prime-tower`       |
| `top-surface-ironing`         | `ironing` (`PostPass::Finalization` since packet 38-rev1) |

A finalization module is permitted to be unconditionally `layer_parallel_safe = false` (enforced by Phase 2 DAG construction); modules in the same stage execute in dependency order without any mutual-exclusion machinery. `wipe-tower`'s manifest declares `[compatibility].requires = ["skirt-brim", "part-cooling", "top-surface-ironing"]` to force itself last.

### Model Loading — 3MF Sidecar Parse Order (Normative — Packet 56)

Inside `load_3mf` the host opens the 3MF ZIP archive, calls
`parse_3mf_model_xml`, and then invokes
`parse_3mf_sidecar(&mut zip)` BEFORE the `ZipArchive` is dropped.
The resulting `HashMap<u32, ObjectSidecarInfo>` is threaded through
`parse_3mf_model_xml` to `resolve_object` as an additional parameter
(unused in Packet 56 — branched only in Packets 56b/56c). Missing
sidecar files return an empty map silently; malformed XML returns an
empty map plus a `log::warn!` on the `slicer_model_io::sidecar` target.
Either way `load_model` returns `Ok(MeshIR)` — sidecar failure is
non-fatal and falls back to treating all parts as `NormalPart`. See
`docs/02_ir_schemas.md` § Host-Local Sidecar Types for the exact
return types.

### Modifier-Part and Negative-Volume Routing (packets 56b / 56c)

Modifier parts (3MF `Metadata/model_settings.config`) are routed into `MeshIR.objects[].modifier_volumes` by the host loader (packet 56b). Negative-volume and support-subtype modifiers (`ModifierScope::Support`, negative-volume difference) are applied by the per-layer negative-part subtract host stage described in the next section (packet 56c): the host subtracts negative-volume geometry per layer and routes support-subtype modifiers into the Support claim's per-region override stream.

#### Negative-Part Per-Layer Subtract (Normative — Packet 56c)

Negative-part subtract is a **per-layer host stage** inserted inside
`run_paint_annotation` (`crates/slicer-runtime/src/layer_executor.rs`), after `arena.take_slice()`
returns the layer's `SliceIR` and BEFORE the paint annotation loop
begins. This insertion point is binding (see proposed ADR-0012):

- Earlier designs put the subtract in a prepass phase-0 built-in or in
  `crates/slicer-runtime/src/pipeline.rs`; both were infeasible because `Vec<SliceIR>` is
  produced per-layer during execution, not during prepass.
- The per-layer seam guarantees paint annotation and all downstream
  per-layer consumers (perimeters, infill, support) see post-subtract
  polygons.

Per-layer call order is locked:
`arena.take_slice()` → `apply_negative_part_subtract(...)` →
`run_paint_annotation` loop → downstream per-layer stages.

For each `ModifierVolume` whose
`config_delta.fields["subtype"] == "negative_part"`, the stage
projects the modifier mesh at `slice_ir.z` via
`slicer_core::slice_mesh_ex(&mv.mesh, &[slice_ir.z])` and applies
`slicer_core::polygon_ops::difference` to each
`slice_ir.regions[ri].polygons`. Modifiers whose Z extent does not
contain `slice_ir.z` are skipped. The function has no global state.

### PrePass Execution (sequential)

```rust
// Illustrative. The real entry points are the `execute_prepass` family in
// `crates/slicer-runtime/src/prepass.rs`; instance acquisition goes through
// the runtime's pools (`slicer-wasm-host`), not the plan.
pub fn execute_prepass(
    plan: &ExecutionPlan,
    blackboard: &mut Blackboard,
) -> Result<(), SlicerError> {
    for stage in &plan.prepass_stages {
        // Stage prerequisites are checked once per stage, before any module
        // runs. The check returns `MissingRequiredPrepass { slot }` when a
        // prerequisite IR slot is uncommitted — see required_slots() for
        // the per-stage table.
        ensure_stage_prerequisites(&stage.stage_id, blackboard)?;
        for module in &stage.modules {
            let ir_views = blackboard.build_read_views(&module.ir_read_mask);
            let output   = blackboard.build_output_builder(&module.ir_write_mask);
            let instance = module.instance_pool.acquire();
            instance.call_prepass(&stage.stage_id, ir_views, output,
                                  Arc::clone(&module.config_view))?;
            blackboard.commit_output(output);
        }
    }
    Ok(())
}
```

#### Stage Prerequisites (Normative)

Each PrePass stage declares which already-committed Blackboard slots it
requires. The `required_slots()` table is the single source of truth — modules
must not run their own ad-hoc presence checks for these slots.

| Stage                              | Required Slots                                                            |
|------------------------------------|---------------------------------------------------------------------------|
| `PrePass::LayerPlanning`           | `SurfaceClassification`                                                   |
| `PrePass::OverhangAnnotation`      | reads `SliceIR` (+ `LayerPlanIR`, `SurfaceClassificationIR`); writes `overhang_quartile_polygons` into `SurfaceClassificationIR`. Runs after `PrePass::Slice`. |
| `PrePass::SeamPlanning`            | `LayerPlan`, `SliceIR`, `RegionMap` — reads the committed `LayerPlan` plus per-region `SliceIR` geometry and annotations (projected via `SeamPlanningView`, packet 178); writes `SeamPlanIR`. Dispatch occurs only after these products are committed. |
| `PrePass::PaintSegmentation`       | `SliceIR`, `RegionMap`; produces split `SliceIR` via `replace_slice_ir`  |
| `PrePass::RegionMapping`           | `LayerPlan`                                                               |
| `PrePass::SupportGeometry`         | `MeshIR`, `LayerPlan`, `RegionMap`, `SupportGeometry` (committed by the host built-in within this stage before the guest runs) |

A stage scheduled before its prerequisites are committed produces
`PrepassExecutionError::MissingRequiredPrepass { stage_id, slot }` and aborts
the prepass without invoking any module. This guard short-circuits before
dispatch so module-side error handling for "the IR I need wasn't committed"
is unnecessary.



#### Precision-Key Touch Points (packet 60)

`Layer::Slice` (host-built-in): reads `slice_closing_radius` from `ResolvedConfig`; this key is consumed by `slicer_core::triangle_mesh_slicer` to close open contours at the slice plane.

`PostPass::GCodeEmit` (host-built-in): reads seven precision keys from `ResolvedConfig` (see `docs/02_ir_schemas.md` "Polyline simplification and precision" subsection). Key routing:
- `gcode_resolution`, `infill_resolution`, `support_resolution`, `min_segment_length`, `gcode_xy_decimals` — consumed inside `DefaultGCodeEmitter` during G-code serialization.
- `perimeter_arc_tolerance` — read by perimeter modules at module-load time and threaded into every `slicer_core::polygon_ops::offset(...)` call.
- `slice_closing_radius` — consumed by `slicer_core::triangle_mesh_slicer` at the host-built-in `Layer::Slice` stage (see above).

#### Layer::PaintRegionAnnotation Stage (packet 64)

`Layer::PaintRegionAnnotation` sits between `Layer::Slice` and `Layer::SlicePostProcess` in the per-layer stage order. The host handler `run_paint_annotation` (`crates/slicer-runtime/src/layer_executor.rs`) is a **no-op stub** since packet 95: `PrePass::PaintSegmentation` writes `segment_annotations` (and per-variant geometry) directly into the committed `SliceIR` during prepass. The stage boundary is retained for plan wiring; any WASM module claiming `Layer::PaintRegionAnnotation` in its manifest runs instead of the host built-in, providing a full override contract. When no module claims the stage, the host built-in (no-op) handles it.

The annotation loop processes contour points in **parallel chunks of
32** (`par_chunks(32)`, rayon). Results are byte-identical to serial
execution — per-point paint queries are order-independent, so the
chunked schedule is purely a wall-clock optimisation. Thread-local
warnings and `DeterministicConflict` detection flags are merged at
the end of the layer; cross-thread state contention is zero. Observed
multi-thread utilisation is exposed via report wall-clock timing
(non-gating).
<!-- VERIFY: the par_chunks(32) annotation loop described here predates the
     packet-95 move of paint annotation into PrePass::PaintSegmentation; the
     per-layer host built-in is now a no-op (run_paint_annotation in
     crates/slicer-runtime/src/layer_executor.rs), so this paragraph describes
     retired behaviour unless a WASM module claims the stage. -->

`DeterministicConflict` Timing (Normative — Packet 64): overlapping
`Custom` paint regions with equal `paint_order` are detected at
`PrePass::PaintSegmentation` time and surfaced as a fatal prepass
error (`PaintSegmentationError::DeterministicConflict`). This is a
correctness improvement over the pre-Packet-64 path where the same
   conflict failed per-layer at query time.

### Per-Layer Execution (rayon parallel)

```rust
/// Execute the PostPass::LayerFinalization stage.
///
/// Ownership model:
/// `layer_irs` is a plain `Vec` taken by mutable reference. By the time
/// this function is called, the rayon join has completed and the
/// Blackboard's `layer_outputs: Vec<Option<LayerCollectionIR>>` has been
/// drained into this Vec. The Blackboard no longer holds any reference to
/// these values — there is no concurrent access, and no RwLock is needed.
///
/// After this function returns, the Vec is passed as `&[LayerCollectionIR]`
/// to `execute_postpass`. It is never re-entered into the Blackboard.
fn execute_layer_finalization(
    plan:       &ExecutionPlan,
    layer_irs:  &mut Vec<LayerCollectionIR>,  // exclusively owned, single-threaded
    blackboard: &Blackboard,                   // read-only; mesh, layer plan, etc.

) -> Result<(), SlicerError> {
    // Always sequential — pool size 1 for all finalization modules.
    for module in &plan.layer_finalization_stage.modules {
        let layer_views: Vec<LayerCollectionView> = layer_irs.iter()
            .map(|l| LayerCollectionView::from(l, &module.ir_read_mask))
            .collect();

        let mut output = FinalizationOutputBuilder::new(layer_irs);

        instance.call_finalization(
            layer_views,
            output,
            Arc::clone(&module.config_view),
        ).map_err(|e| handle_module_error(e, &module.module_id, 0))?;

        output.commit(layer_irs);
        validate_finalization_state(layer_irs)?;
    }
    Ok(())
}

fn validate_finalization_state(
    layer_irs: &[LayerCollectionIR],
) -> Result<(), SlicerError> {
    let mut seen = HashSet::new();
    let mut prev = None;

    for layer in layer_irs {
        if !seen.insert(layer.global_layer_index) {
            return Err(SlicerError::InvalidSyntheticLayer {
                reason: format!("duplicate layer index {}", layer.global_layer_index),
            });
        }
        if let Some(p) = prev {
            if layer.global_layer_index < p {
                return Err(SlicerError::InvalidSyntheticLayer {
                    reason: "layer indices must be monotonic".into(),
                });
            }
        }
        prev = Some(layer.global_layer_index);
    }
    Ok(())
}


Finalization ordering guarantees:
- Modules execute sequentially in stage order.
- Module B always sees the fully committed output of module A.
- If two modules insert at the same position, order is deterministic by module execution order.

Top-surface ironing is performed at `PostPass::LayerFinalization` (not at `Layer::InfillPostProcess`) so the module sees the full layer sequence and can detect the topmost-layer index via the multi-layer `top_solid_layers` window (packet 38-rev1). The module appends `Ironing`-role entities via the finalization builder; ordering uses the role's default priority `900` (Ironing prints last on its layer).

#### Post-Finalization Travel Reconciliation (packet 20)

After `execute_layer_finalization` returns and before `execute_postpass` runs,
the host performs a built-in travel-reconciliation pass. Skirt, brim, wipe-
tower, and prime-tower entities inserted by finalization modules have
endpoints the per-layer `Layer::PathOptimization` could not have seen, so the
host recomputes travel transitions against those new endpoints.

Reconciliation contract (normative):

- Walks `layer_irs` once and recomputes `TravelMove.entity_id` and endpoint XY
  against the post-finalization entity sequence.
- **Model extrusion entity ordering is invariant** — only travel anchors and
  endpoints change. The reconciliation must not reorder, drop, or rewrite any
  `PrintEntity`.
- Retract/unretract pairing and Z-hop matching are re-validated; mismatches
  surface as fatal `RECONCILED_TRAVEL_INCONSISTENT` errors.
- No module-visible surface — this is a host built-in tucked between
  `PostPass::LayerFinalization` and `PostPass::GCodeEmit`.

The reconciled `Vec<LayerCollectionIR>` is then handed to `execute_postpass`
as the immutable slice argument.
pub fn execute_per_layer(
    plan: &ExecutionPlan,
    blackboard: &Blackboard,  // read-only after PrePass
) -> Result<Vec<LayerCollectionIR>, SlicerError> {
    plan.global_layers
        .par_iter()
        .map(|layer| execute_single_layer(layer, &plan.per_layer_stages, blackboard))
        .collect()
}

fn execute_single_layer(
    layer: &GlobalLayer,
    stages: &[CompiledStage],
    blackboard: &Blackboard,
) -> Result<LayerCollectionIR, SlicerError> {
    // Per-layer arena — freed entirely when this function returns.
    let mut arena = LayerArena::new();
    let mut layer_ir = LayerIrState::new(layer, &mut arena);

    for stage in stages {
        for module in &stage.modules {
            let active_regions = resolve_active_regions(layer, module, blackboard);
            if active_regions.is_empty() { continue; }

            let instance = module.instance_pool.acquire();
            let ir_views = layer_ir.build_read_views(&module.ir_read_mask);
            let output   = layer_ir.build_output_builder(&module.ir_write_mask);

            instance.call_layer(
                &stage.stage_id,
                layer.index,
                ir_views,
                output,
                Arc::clone(&module.config_view),
            ).map_err(|e| handle_module_error(e, &module.module_id, layer.index))?;

            layer_ir.commit_output(output);
        }
    }
    Ok(layer_ir.finalize())
}
```

#### Cooperative Cancellation (packet 174)

Cancellation is cooperative, not preemptive. The CLI owns a shared
`AtomicBool` (`SliceRunOptions.cancel_flag` → `PipelineConfig.cancel_flag`,
populated by signal handlers and an opt-in stdin watcher behind
`--cancel-on-stdin-eof`); the runtime checks the flag at phase boundaries
(before PrePass, PerLayer, and PostPass phase starts) and **before per-layer
execution** (inside the per-layer rayon closure, where a set flag returns
`LayerExecutionError::Cancelled` instead of dispatching the layer).

- A WASM module already in dispatch is **not interrupted** — there is no
  cancellation inside a running module call.
- Already-scheduled layers (the in-flight rayon batch) **may finish**; cancel
  latency is bounded by the in-flight layer batch.
- Cancellation therefore takes effect at the next available checkpoint: a set
  flag is only observed at the next phase boundary or the next layer dispatch.
- The pipeline returns `LayerExecutionError::Cancelled` (mapping to
  `PipelineError::LayerExecution`); the CLI records a `cancelled` progress
  event, guarantees a absent output file, and exits with code 130.
- `cancel_flag: None` (or a present-but-unset flag) reproduces today's
  behaviour bit-for-bit — no change to layer ordering or module scheduling.

### Error Handling Policy

```rust
pub enum LayerErrorAction {
    ContinueDegraded,
    Abort(SlicerError),
}

fn handle_module_error(
    error: ModuleError,
    module_id: &ModuleId,
    layer: u32,
) -> LayerErrorAction {
    if error.fatal {
        emit_progress_event(ProgressEvent::module_error(
            module_id,
            layer,
            "fatal_error",
            &error.message,
        ));
        LayerErrorAction::Abort(SlicerError::ModuleFatal {
            module: module_id.clone(), layer, message: error.message,
        })
    } else {
        emit_progress_event(ProgressEvent::module_error(
            module_id,
            layer,
            "non_fatal_error",
            &error.message,
        ));
        log::warn!("[{}] layer {}: non-fatal — {}. Using unmodified IR.",
                   module_id, layer, error.message);
        LayerErrorAction::ContinueDegraded
    }
}
```

Normative behavior:

- `fatal=true` aborts the slice command immediately.
- `fatal=false` continues with pre-stage IR for that module only; downstream stages process degraded state.
- Every non-fatal or fatal module error must emit a structured progress event (`module_error`).
- Slice result metadata must include `degraded=true` if any non-fatal error occurred.
- An absent compiled component is always fatal: load rejects it via `LiveModuleLoadError::Component`, and dispatch rejects it via the phase-appropriate fatal error in `dispatch.rs`. The previous graceful-stage-skip behavior and the placeholder-skip affordance documented in `manifest.rs` are retired.

#### Non-Fatal → FatalModule Host Limitation (Open — packet 180)

The host **currently maps every module-returned `ModuleError` to a fatal
error regardless of the `fatal` field**. In `crates/slicer-wasm-host/src/dispatch.rs`,
`dispatch_layer_call` converts a module-returned `ModuleError` into a
`DispatchError` whose `reason` records `code`/`fatal`/`message` but whose
variant does not branch on `fatal`; the caller then escalates to
`LayerStageError::FatalModule` unconditionally (likewise
`PrepassRunnerError::FatalModule` / `FinalizationError::FatalModule` /
`PostpassError::FatalModule` at the other three phase boundaries). This
contradicts the documented degraded-success propagation contract above —
`fatal=false` should `ContinueDegraded` with the pre-stage IR intact — and
is a **known open host limitation** (tracked separately by packet 180;
cross-reference a potential future `DEVIATION_LOG` row for
`dispatch.rs::dispatch_layer_call`'s fatal-only mapping). The seam-placer's
packet-180 degraded fallback (missing `SeamPlanIR` entry → `ModuleError`
`fatal: false`) is verified at the module boundary (in-process tests), not
through the WASM dispatch path.

See progress event schema: `09_progress_events.md`.

### PostPass::GCodeEmit Emission Contract (packet 11, Normative)

`PostPass::GCodeEmit` is the **sole owner of final G-code text formatting**;
modules are forbidden from producing OrcaSlicer-specific strings (`;LAYER_CHANGE`,
`;TYPE:`, `;Z:`, `;HEIGHT:`) themselves. The host emits these per-layer in
exactly this order before the first extrusion entity on each layer:

#### `GCodeEmitter` Trait Signature (Normative — Packet 86)

The G-code emission machinery lives in the `slicer-gcode` crate
(extracted in Packet 86). The traits accept IR-typed inputs only — no
`&Blackboard` parameter — and return errors in `GCodeEmitError`
(crate-local):

```rust
pub trait GCodeEmitter {
    fn emit_gcode(&self, layers: &[LayerCollectionIR])
        -> Result<GCodeIR, GCodeEmitError>;
}

pub trait GCodeSerializer {
    fn serialize_gcode(&self, gcode_ir: &GCodeIR)
        -> Result<String, GCodeEmitError>;
}
```

`PostPass::GCodeEmit` is implemented in
`slicer-runtime/src/builtins/gcode_emit_producer.rs` as a metadata-only
`BuiltinProducer` descriptor (~42 LOC). The actual call site lives in
`crates/slicer-runtime/src/run.rs` / `crates/slicer-runtime/src/postpass.rs` and wraps `DefaultGCodeEmitter::emit_gcode`,
converting `GCodeEmitError` → `PostpassError` at the boundary via a free
function (not a `From` impl — orphan rule prevents that). This
preserves ADR-0001's in-stage-commit pattern without introducing a
`slicer-gcode` → `slicer-runtime` circular dependency.

#### Overhang Classification (Normative — Packets 88 / 106 / 107, ADR-0008 / ADR-0031)

> **Superseded:** the Packet-57 embedded `emit_gcode` prepass and
> `slicer_core::algos::overhang_classifier::classify_layers` were **deleted**
> (Packet 88 relocated the algorithm out of `slicer-core`; there is no
> `overhang_classifier.rs` today). Overhang classification now runs in two
> pieces:

- **PrePass::OverhangAnnotation** (Packet 106/107, ADR-0031) stamps
  `Point3WithWidth.overhang_quartile` per wall-family vertex. Since the
  2026-07-10 inversion (see the overhang-after-slice inversion record) the stage
  runs after `PrePass::Slice` and derives bands by diffing consecutive
  `SliceIR` footprints rather than computing mesh cross-sections.
- **overhang-classifier-default** (Packet 88, ADR-0008), a `FinalizationModule`
  at `PostPass::LayerFinalization`, consumes the per-vertex `overhang_quartile`
  gate and resolves a per-point speed by interpolating the `speed_sections` table
  from the prepass-stamped `overhang_distance_mm`, emitting
  `EntityMutation::SetPointSpeedFactors`. Users opt out by curating their module dir without it; with all
  four keys zero the module short-circuits (byte-identical to pre-Packet-57).

##### ADD_INTERSECTIONS Contract (Normative — Packet 191, ADR-0053)

Since packet 191 the same module is also a **geometry mutator**: it ports
canonical `estimate_points_properties` (`ExtrusionProcessor.hpp`) mid-segment
vertex insertion via a new `EntityMutation::SetPathPoints(Vec<Point3WithWidth>)`
channel. The contract (ADR-0053; distances are the packet-193 signed,
`boundary_offset`-normalised `overhang_distance_mm` carrier):

- **Strictly interior insertion.** Synthetic vertices lie on the original
  polyline; the loop's closing repeat is preserved and the first/last points
  stay bit-identical to the originals (post-finalization travel reconciliation
  fails fatally with `RECONCILED_TRAVEL_INCONSISTENT` if an entity's endpoints
  move). `SetPathPoints`' `apply_to` rejects an empty vector and any list that
  breaks a loop's closing repeat (`ExtrusionRole::is_loop()`).
- **Threshold-crossing branch.** XOR of the two endpoints' side tests against
  `boundary_offset + EPSILON` (`boundary_offset = 0.5 × flow_width`); one
  synthetic vertex per crossing of the segment with the previous-layer
  boundary polyline, recorded distance **exactly `boundary_offset`** (assigned,
  not re-measured); two-sided `min_spacing = flow_width × 0.25` filter.
- **Segmentation branch.** Outer proximity test
  (`> -boundary_offset && < boundary_offset + 2.0`), then
  `min_distance > 0 && (|d_curr| > min_distance || |d_next| > min_distance) &&
  line_len >= 2.0` **or** `min_distance <= 0 && line_len > 4.0`; interior
  parameters `a0 = clamp((d_curr + 3×boundary_offset)/line_len, 0, 1)`,
  `a1 = clamp(1.0 - (d_next + 3×boundary_offset)/line_len, 0, 1)` (the
  `1.0 -` is load-bearing — do not "tidy" it into symmetry), `t0 = min(a0,a1)`,
  `t1 = max(a0,a1)`. `min_distance` is canonical's
  `smallest_distance_with_lower_speed` from packet 190's `speed_sections`,
  `-1.0` when no section is slower than `original_speed`. Each segmentation
  candidate's `overhang_distance_mm` is **linearly interpolated** between the
  segment endpoints at its own `t` (option-C divergence from canonical's
  re-measurement — see `docs/adr/0053-overhang-emission-time-speed-sections.md`; the pre-purge closed record was DEV-108).
- **Unmeasured distances rejected.** `overhang_distance_mm: Option<f32>` is
  `None` when there is no previous layer or the previous layer's slice
  boundary is empty (packet 193 AC-N1); a `None` endpoint takes the
  no-insertion path in both branches and forbids `0.0` / `f32::MAX` / `-1.0`
  substitutes.
- **Canonical spacing/length gates.** `min_spacing = flow_width × 0.25`
  (two-sided) and the `2.0`/`4.0` mm length gates are canonical constants, not
  tunables.
- **Emission order.** `SetPathPoints(new_points)` **then**
  `SetPointSpeedFactors(new_factors)` with `new_factors.len() == new_points.len()`
  for the same entity, same `merge_ops` sequence (the profile branch
  length-checks against the entity's *current* point count).
- **No mutations when** all `overhang_*_4_speed` keys are zero (default),
  `enable_overhang_speed = false`, or distances are unavailable — `AC-N2`
  (unchanged list) and `AC-N3` (`None` distances) emit no `SetPathPoints` at
  all, and the all-zero config emits no mutations of any kind.

```
;LAYER_CHANGE
;Z:<value>
;HEIGHT:<value>
```

Field derivation:

- `;Z:<value>` — `LayerCollectionIR.z` formatted with `gcode_xy_decimals`
  (packet 60).
- `;HEIGHT:<value>` — derived from the difference between consecutive
  `LayerCollectionIR.z` values: `height_i = z_{i+1} - z_i`. The first layer
  uses `z_0` directly. The **terminal layer falls back to the last non-zero
  delta** (`height_N = height_{N-1}`) — never zero, because OrcaSlicer
  post-processors reject zero-height comments.

`ExtrusionRole` → `;TYPE:` label mapping (host-canonical, OrcaSlicer parity):

| `ExtrusionRole`        | `;TYPE:` label      |
|------------------------|---------------------|
| `OuterWall`            | `Outer wall`        |
| `InnerWall`            | `Inner wall`        |
| `ThinWall`             | `Thin wall`         |
| `TopSolidInfill`       | `Top surface`       |
| `BottomSolidInfill`    | `Bottom surface`    |
| `SparseInfill`         | `Sparse infill`     |
| `BridgeInfill`         | `Bridge`            |
| `SupportMaterial`      | `Support`           |
| `SupportInterface`     | `Support interface` |
| `Skirt`                | `Skirt`           |
| `Brim`                 | `Brim`            |
| `WipeTower`            | `Prime tower`       |
| `PrimeTower`           | `Prime tower`       |
| `Ironing`              | `Ironing`           |
| `Custom(s)`            | `s` verbatim        |

Modules that attempt to emit any of these strings via `Raw(text)` are accepted
(the escape hatch is intentional) but doing so duplicates the host-emitted
markers and is logged as a `MUDDIED_GCODE_PREAMBLE` warning.

### Deferred Tool-Change Queue (packet 19)

`gcode-output-builder.push-tool-change(from_tool, to_tool)` is the canonical
surface for inserting `ToolChange { from, to }` commands. Calls at
`Layer::PathOptimization` are queued and deferred — they are *not* committed
mid-layer. The host drains the queue at `PostPass::LayerFinalization` time,
inserting the `ToolChange` commands at the appropriate entity boundaries based
on per-region `tool_index` transitions.

Host-side tool-grouping in `crates/slicer-runtime/src/layer_executor.rs` is intentionally absent;
re-ordering entities to consolidate same-tool runs is the path-optimization
module's responsibility (via `LayerCollectionBuilder::set_entity_order`). The
host neither sorts by tool nor synthesises tool-change records — both are
data-driven from module output.

### PostPass Execution (sequential)

```rust
pub fn execute_postpass(
    plan: &ExecutionPlan,
    layer_irs: &[LayerCollectionIR],  // immutable ref — LayerFinalization already ran
    blackboard: &Blackboard,
) -> Result<String, SlicerError> {
    let mut gcode_ir = emit_gcode(layer_irs, blackboard)?;

    for stage in &plan.postpass_stages {
        for module in &stage.modules {
            let instance = module.instance_pool.acquire();
            match stage.stage_id {
                StageId::PostPassGCodePostProcess => {
                    let output = GCodeOutputBuilder::new(&mut gcode_ir);
                    instance.call_gcode_postprocess(
                        &gcode_ir.commands, output,
                        Arc::clone(&module.config_view))?;
                }
                StageId::PostPassTextPostProcess => {
                    let text   = serialize_gcode(&gcode_ir);
                    let result = instance.call_text_postprocess(
                        text, Arc::clone(&module.config_view))?;
                    return Ok(result);
                }
                _ => unreachable!()
            }
        }
    }
    Ok(serialize_gcode(&gcode_ir))
}
```

---

## Blackboard Structure

```rust
pub struct Blackboard {
    // Immutable after loading
    pub mesh_ir: Arc<MeshIR>,

    // Written by PrePass, immutable during per-layer
    pub surface_class: Arc<SurfaceClassificationIR>,
    pub layer_plan:    Arc<LayerPlanIR>,
    pub region_map:    Arc<RegionMapIR>,

    // Written by per-layer (one slot per layer, written once, read after join)
    pub layer_outputs: Vec<Option<LayerCollectionIR>>,
}
```

The authoritative definition is `Blackboard` in
`crates/slicer-runtime/src/blackboard.rs`; per-layer slots are
`Vec<Option<LayerCollectionIR>>` (not a `SlotVec` type — that name does not
exist in the codebase).

---

## Full Lifecycle

```
startup
  ├─ scan module directories → parse all .toml manifests
  ├─ build intra-stage DAGs
  ├─ validate: claim conflicts, incompatibilities, cycles, unfulfilled reads, IR versions
  │    ├─ fatal errors → print diagnostics, exit(1)
  │    └─ warnings     → print, continue
  ├─ topological sort each stage DAG
  ├─ instantiate WASM modules + build instance pools
  └─ freeze ExecutionPlan

slice command
  ├─ load model → MeshIR (paint normalized at load via split_triangle_strokes)
  ├─ execute_prepass()
    │    ├─ PrePassMeshAnalysis          → SurfaceClassificationIR   → Blackboard
    │    ├─ PrePassLayerPlanning         → LayerPlanIR               → Blackboard
    │    ├─ PrePassSeamPlanning          → SeamPlanIR                → Blackboard  (optional guest)
    │    ├─ PrePassRegionMapping         → RegionMapIR               → Blackboard
    │    ├─ PrePassSlice                 → SliceIR                   → Blackboard
    │    ├─ PrePassOverhangAnnotation    → SurfaceClassificationIR (overhang_quartile_polygons, from SliceIR) → Blackboard  (after Slice)
    │    ├─ PrePassShellClassification   → SliceIR (shell indices, solid fill) → Blackboard
    │    ├─ PrePassPaintSegmentation     → SliceIR (per-variant regions)       → Blackboard
    │    └─ PrePassSupportGeometry       → SupportGeometryIR+SupportPlanIR      → Blackboard  (guest optional)
  │    └─ PrePassLightningTreeGen      → LightningTreeIR                      → Blackboard  (only when sparse_fill_holder = lightning-infill)
  ├─ execute_per_layer()  [rayon::par_iter]
  │    └─ per layer (parallel):
  │         ├─ LayerSlice              (host-built-in)
  │         ├─ LayerPaintRegionAnnotation  (host-built-in; WASM override)
  │         ├─ LayerSlicePostProcess
  │         ├─ LayerPerimeters
  │         ├─ LayerPerimetersPostProcess
  │         ├─ LayerInfill
  │         ├─ LayerInfillPostProcess
  │         ├─ LayerSupport
  │         └─ LayerPathOptimization
  │              └─ writes complete LayerCollectionIR into Blackboard layer_outputs[layer_idx]
  │                 (written once per slot; no mutex required)
  ├─ rayon join
  │    └─ drain Blackboard layer_outputs → plain Vec<LayerCollectionIR>
  │       (Blackboard no longer holds these values after this point)
  ├─ execute_layer_finalization()    [single-threaded, owns Vec<LayerCollectionIR>]
  │    └─ PostPassLayerFinalization modules may append or insert synthetic layers
  └─ execute_postpass()
       ├─ PostPassGCodeEmit         (host-built-in serializer)
       ├─ PostPassGCodePostProcess  (optional modules)
       └─ PostPassTextPostProcess   (optional, last resort)
       └─ write .gcode / .bgcode file
```
