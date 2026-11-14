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
> - `crates/slicer-scheduler/src/stage_order.rs` — stage-order helpers derived from the canonical list (the list itself lives in `execution_plan.rs`).
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
> (must be empty). This is what enables the ~7.9k LOC of planning logic
> to be unit-tested without instantiating any WASM component.

The scheduler has four phases. Phases 1–3 are pure data transformation that complete before a single layer is sliced — no WASM executes until Phase 4.

```text
Phase 1: Manifest Ingestion     (parse all .toml files)
Phase 2: DAG Construction       (build intra-stage dependency graphs)
Phase 3: DAG Validation         (claim conflicts, cycles, version checks)
Phase 4: Execution              (PrePass → Per-Layer parallel → PostPass)
```

---

## Phase 1 — Manifest Ingestion

```rust
// Simplified sketch — field visibility and accessors elided. The authoritative
// definition is `LoadedModule` in `crates/slicer-scheduler/src/manifest.rs`.
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
    pub layer_parallel_safe:   bool,
    pub wasm_path:             PathBuf,
    pub provenance:            ModuleProvenance, // External | Integrated (packet 85/ADR-0056)
    pub region_splits:         Vec<RegionSplitDeclaration>, // from manifest [[region_split]] (packet 92)
    pub paint_only:            bool,       // top-level paint_only = true (ADR-0071); gates per-layer dispatch
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

#### Preparation metadata naming exception (Normative)

The new `[preparation]` table maps `[preparation].interface`,
`[preparation].input_reads`, and `[preparation].views` to the companion's
Rust `interface()`, `input_reads()`, and `views()` methods, respectively. A
kebab-case `input-reads` alias is rejected as an unknown key.

Only new [preparation] metadata keys use snake_case; legacy manifest/config naming remains unchanged.

This exception applies only to the new `[preparation]` table; no existing
manifest key is renamed, existing mapping rows remain unchanged, and the
config naming policy is unchanged.

Per-key scope eligibility is not a manifest-table list: each key's
`denied_scopes` entries live on its own `[config.schema.<key>]` declaration and
are parsed into the key's schema entry (`config_schema`), where the registry
consumes them at resolution (see "Config scope admission (ADR-0069)" below).

The manifest naming is canonical for author-facing docs and examples. Runtime field names are internal and must not appear in user-facing manifest examples.

Ingestion scans all module search paths and deserializes every `.toml`. TOML schema errors produce a structured `LoadError` with file path and field name. No module is silently skipped.

Ingestion is generalized over manifest source: a module may come from a disk
file or embedded TOML. `LoadedModule` carries a `ModuleProvenance` marker
(`External | Integrated`); claims and DAG machinery never inspects provenance.

### `[[region_split]]` Aggregation and Tied-Priority Diagnostic (Normative — Packet 92, ADR-0071)

When ingestion completes, the scheduler aggregates the
`[[region_split]]` array entries from every loaded manifest into a single
`BTreeMap<String, AggregatedRegionSplitEntry>` keyed and iterated by
**semantic name**. The map is not a priority-ordered sequence: the separate
`slicer_scheduler::region_split::canonical_variant_chain_order` helper returns
the `(priority, name)` sequence that canonical priority ordering calls for, and
raw map iteration is not that sequence in general. The aggregate is exactly the
union of the loaded modules' declarations — no semantics are seeded implicitly.
`material` and `fuzzy_skin` are present in a real slice because the core
perimeter and fuzzy-skin manifests declare them. The map is consumed by:

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
5. **Top-level `paint_only = true` with no `[[region_split]]` entry** →
   rejected (`LoadErrorKind::PaintOnlyWithoutRegionSplit`); a non-boolean
   `paint_only` is a `Schema` error.

Cross-manifest **tied-priority warning** (non-fatal): one
`LoadDiagnostic { level: DiagnosticLevel::Warning, path, field, message }` is
appended per priority shared by two or more **distinct semantics** — including
two semantics declared by the same module. The message names the shared
priority, the tied semantic names, each semantic's declaring module IDs, and
the lexicographic tiebreaker order used to keep aggregation deterministic. Its
`path` is an empty `PathBuf` sentinel (no single manifest is authoritative) and
its `field` is `Some("region_split")`; it names module IDs, not both manifest
paths. Scheduler operation continues; this is purely an author-facing nudge.

Cross-manifest **conflicting declaration** (non-fatal): when two manifests
declare the same semantic with different priority or value type, the first one
seen wins and one warning is appended per later incompatible declaration,
carrying that later module's **WASM path** (`LoadedModule::wasm_path` in
`crates/slicer-scheduler/src/manifest.rs`, the module's built artifact) rather
than its manifest TOML path; for integrated modules that field carries the
registration origin label. Declaration-order invariance is therefore claimed
only for compatible declarations.

**Where the two orders apply.** `build_execution_plan`
(`crates/slicer-scheduler/src/execution_plan.rs`) calls the aggregator and
stores its output in `ExecutionPlan.aggregated_region_split`. The RegionMapping
builtin then passes that map to the kernel, which currently supplies the map's
**name-ordered keys** to `enumerate_canonical_chains`
(`crates/slicer-ir/src/region_split_registry.rs`); the enumerator preserves the
supplied axis order. `canonical_variant_chain_order` has no production caller in
the current Rust tree. See ADR-0018's clarifying amendment; the driver-level
audit belongs to a separate pre-pass property packet.

Property coverage: `H-REGISTRY` (`docs/testing/properties.md#h-registry`,
evidence in
[proptest-scheduler-evidence.md](testing/proptest-scheduler-evidence.md))
drives both real functions — the aggregate plus the separate helper sequence —
on compatible, conflicting and tied declarations, and it asserts the plan-level
aggregate so a missing plan-to-aggregator call cannot pass. It does **not**
prove helper integration with RegionMapping.

### IR Access Path Format (Normative)

`ir_reads` and `ir_writes` entries in manifests use dot-notation to name specific fields within an IR struct. The format is `<IRName>.<field>`, where `<IRName>` is the canonical IR short name (e.g., `PerimeterIR`, `LayerCollectionIR`) and `<field>` is a snake_case field name declared in the corresponding Rust struct in `crates/slicer-ir/src/`.

Examples:
- `PerimeterIR.regions` — the `regions` field of `PerimeterIR` (array of per-region slices)
- `PerimeterIR.resolved-seam` — the `resolved_seam` field of each `PerimeterRegion` (written by `seam-placer` via `push-resolved-seam`)
- `LayerCollectionIR.skirt-brim` — the `skirt-brim` field of `LayerCollectionIR` (written by skirt-brim finalization modules)
- `PerimeterIR.walls` — wall loop array within each perimeter region

Wildcards are not supported in this version. Each dot-separated path is matched literally against runtime access audit paths generated at the WIT boundary.

Why sub-field specificity matters: declaring `PerimeterIR` as a whole grants access to every field in the struct, preventing other modules from writing non-overlapping sub-fields in the same stage without a claim conflict. Narrow declarations like `PerimeterIR.resolved-seam` let modules operate on non-overlapping fields within the same IR type without mutual exclusion.

Ingestion does **not** validate that a declared path exists in the IR schema — that check is performed by the IR schema itself at load time. Declaring a non-existent field path produces a manifest that passes ingestion but fails at Phase 3 DAG validation or at the WIT boundary runtime check when the module attempts the access.

---
### Stage ID Validation (during ingestion)

`stage` is validated against the canonical `STAGE_ORDER` set before DAG construction.
Unknown or misspelled stage identifiers are fatal and must not be silently ignored.

```rust
// Simplified sketch of the StageIdValidation pass. The real
// `validate_stage_ids` in `crates/slicer-scheduler/src/validation.rs`
// accumulates into a `DagValidationReport` instead of returning `Result`.
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
    "PrePass::SupportAnalysis",      // host-built-in; derives support candidates from the colour-resolved SliceIR
    "PrePass::SupportGeometry",      // host built-in always runs; guest optional
    "Layer::PaintRegionAnnotation",  // host-built-in; a module claiming this stage runs instead of the host
    "Layer::SlicePostProcess",
    "Layer::Perimeters",
    "Layer::PerimetersPostProcess",
    "Layer::Infill",
    "Layer::InfillPostProcess",
    "Layer::Support",
    "Layer::SupportPostProcess",
    "Layer::AnchoredEvents",
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
  `PaintSegmentation` → `SupportAnalysis` → `SupportGeometry`) — see
  `01_system_architecture.md` §"PrePass Stage Order", which documents the
  executed chain and the known divergence between the two. `PrePass::SupportAnalysis`
  runs strictly after the Slice/paint stages (it derives support candidates from
  the committed, colour-resolved `SliceIR`) and before `PrePass::SupportGeometry`
  (which consumes that analysis), matching its declared `STAGE_ORDER` position.

`PrePass::OverhangAnnotation` — populates `SurfaceClassificationIR.overhang_quartile_polygons` by diffing consecutive-layer footprints, mirroring OrcaSlicer's `detect_overhangs_for_lift` (`PrintObject.cpp`) which diffs consecutive `lslices`. It runs **strictly after `PrePass::Slice`** and reads the committed `SliceIR` (each object's final per-layer region polygons) rather than re-slicing the mesh — the object meshes are sliced exactly once, in `PrePass::Slice`. Host built-in (`host:overhang_annotation`). Since packet 193 the same host built-in additionally writes
`SurfaceClassificationIR.prev_layer_boundaries` — a
`HashMap<ObjectId, HashMap<u32, Vec<ExPolygon>>>` keyed by object id first and
global layer index second, like `overhang_quartile_polygons`,
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

### Per-Layer Region-Split Dispatch Filter (Normative — Packet 92, ADR-0071)

After the intra-stage DAG is sorted, each `LoadedModule` carries a
cached `region_split_semantics: HashSet<String>` on its
`CompiledModuleStatic` descriptor. The set is populated **only** when the
module's manifest opted into paint-only dispatch with top-level
`paint_only = true`, in which case it holds exactly the semantic names
declared in the module's `[[region_split]]` array. A module that declares
`[[region_split]]` semantics without `paint_only = true` — the core
`classic-perimeters`, `arachne-perimeters`, and `fuzzy-skin` modules — has an
**empty** dispatch set and runs on every layer; its declarations still
register the semantics in the cross-manifest aggregate. The host applies a
per-layer filter at dispatch time using this set; the granularity is
per-(module × layer), NOT per-(module × region):

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

The filter helper is `module_invocation_allowed_on_layer(...)` in `crates/slicer-runtime/src/layer_executor.rs`, consulted at per-layer dispatch. Filter cost is `O(|regions| × |S|)` per
dispatch decision; the `region_split_semantics` HashSet keeps the
inner check at O(1).

---

### Lightning selection-driven preparation

The legacy path is retired.
The committed selection index decides participation via `selection.invoke && !selection.targets.is_empty()`;
the old `sparse_fill_holder` string gate is no longer the scheduler's predicate.
The selected `com.core.lightning-infill` artifact plans during `prepare_print` from typed committed
regions, region configs, and layer plan, publishing one private `lightning-layer-{global_layer_index}` piece per schedule layer.
Ordinary `Layer::Infill` consumers require and decode their own layer's piece through the module's decoder.
Deleted surfaces are `commit_lightning_tree_ir_builtin`, `PrePass::LightningTreeGen` and its `STAGE_ORDER` entry,
`BlackboardPrepassSlot::LightningTreeIR`, `BlackboardError::LightningTreeGeneration`,
public `LightningTreeIR`/`LightningTreeEntry`/`CURRENT_LIGHTNING_TREE_IR_SCHEMA_VERSION`,
the `lightning-tree-segments` WIT method and whole `slicer:lightning-queries` package,
and SDK `lightning_tree_segments_for`/`with_lightning_tree_ir` accessors.
Only module-private `KernelPlan`/`PlanEntry` remain, never public IR.
This records behavioural equivalence to the pre-migration producer, moving ownership with no intended output change;
it is not a canonical-parity claim, and the recorded Gate A/B outcomes remain unchanged.

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
2. **Global claim conflicts** — two effective modules hold the same claim with `ConflictScope::Global`. The four configured fill-role claims (`top-fill`, `bottom-fill`, `bridge-fill`, `sparse-fill`, introduced in packet 37) are exempt from this arm: multiple modules legitimately declare the same fill claim and per-region resolution at dispatch time picks the active holder. `claim:raft-fill` is **ordinary** here — two effective holders conflict. The same exemption covers exactly four family-scoped support claims (`support-generator`, `support-planner`, `support-family:traditional`, `support-family:tree`); it is a finite table, not a `support-family:*` wildcard. Absence of any holder for one of the four fill claims is a `MissingDependency { module: "fill-role-claim:<claim>", requires: "no module holds <claim>" }`. See `FILL_CLAIM_IDS` and `validate_claim_conflicts` in `crates/slicer-scheduler/src/validation.rs`, and property `H-CLAIMS` (`docs/testing/properties.md#h-claims`).
3. **Per-region claim conflicts** — the same claim with `ConflictScope::Region { object_id, region_id, global_layer_index }`. Holders are grouped by a typed key, never a concatenated string: ordinary and transition-capable claims key on the full `(object, region, layer)` triple, while the four non-transitionable claims (`perimeter-generator`, `seam-placer`, `layer-planner`, `mesh-analyzer`) key on the object alone, because `docs/01_system_architecture.md` §"Allowed Claim Transition Matrix" requires one holder across every layer **and** region of one object. A same-region cross-layer conflict reports the original region with `global_layer_index = None`.
4. **Incompatibility declarations** — explicit `incompatible-with` pairs
5. **Missing dependencies** — `requires` modules absent or disabled
6. **IR version compatibility** — module requires newer IR schema than host provides
7. **Cycle detection** — Kahn's algorithm per stage DAG. The reported `CyclicDependency.cycle` is the entire blocked remainder: every node on a cycle plus every node reachable from a cycle, including an acyclic downstream tail, and it excludes independently sortable nodes. It is not a simple-cycle route and the private sorter promises no order, so consumers compare it as a set. Property `H-DAG` compares both this set and the public sorter's lexically sorted `Err` vector to a nonempty-path model.
8. **Write conflicts** — two modules in the same stage both write the same IR field with no read-after-write ordering edge between them (see below)
9. **Unfulfilled reads** — module reads a field no upstream module or host writes
10. **Dead writes** — module writes a field no downstream module reads (warning only)
11. **Undeclared access** — module runtime read/write masks must be strict subsets of manifest declarations
12. **Cross-stage dependency legality** — module may not require a module from a later stage
13. **Transitive dependency legality** — transitive `requires` closure may not include later-stage modules
14. **Host version compatibility** — module's declared `min-host-version` must be `<=` the running host version (`env!("CARGO_PKG_VERSION")` of `slicer-runtime`). Fatal, same blocking tier as pass 6 (IR version compatibility); see `SchedulerError::HostVersionIncompatible` and `docs/11_operational_governance_and_acceptance_gate.md` §2 "Compatibility Policy" dimension 1. Closes DEV-026 gap (1).

**Production scope today.** `DagValidationRequest` has no module-disable field and production
constructs only `ConflictScope::Global` claim holders
(`crates/slicer-runtime/src/run.rs`), so pass 3 always runs on an empty set in the live path.
Region-scoped uniqueness for support is enforced at the support-plan commit seam (host aggregation
in `crates/slicer-wasm-host/src/support_aggregation.rs`), not by this pass. Synthetic
`ConflictScope::Region` observations exercise the public validator contract — the typed-scope
stability and exception rules — and are **not** evidence of live region-scoped enforcement.

For the `claim:raft-fill` role, duplicate active holders are rejected at the
applicable scope as `SchedulerError::ClaimConflict`. The resolved holder is
deterministic for each region and layer; no ambiguous raft-fill dispatch is
permitted.

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

The validator itself has **no** module-disable input: `DagValidationRequest` carries `modules`,
`stage_dags`, `host_ir_schema_version`, `host_version`, `claim_holders` and `access_audits`, and
nothing else. Callers supply the effective universe; a locally disabled module and its holder
observations must be removed before the request is built, which is what the `H-CLAIMS`
disabled-mode property models.

```rust
// Illustrative. The real claim-conflict pass is `validate_claim_conflicts` in
// `crates/slicer-scheduler/src/validation.rs`; it groups the caller-supplied
// effective observations, it does not re-filter modules.
fn effective_claim_holders(claim: &ClaimId, request: &DagValidationRequest) -> Vec<ModuleId> {
    request.claim_holders.iter()
        .filter(|holder| holder.claim == *claim)
        .map(|holder| holder.module_id.clone())
        .collect()
}
```

Rules:

- Global validation fails only if the effective holder set for a claim in one scope has more than one distinct module.
- A module removed from the effective universe (disabled, not loaded, or shadowed) does not participate in claim conflicts, and its holder observations must not be retained.
- If no holder remains for a required claim or fill role, this is a configuration error (`MissingDependency`).
- Stable claims are the four non-transitionable rows of the Allowed Claim Transition Matrix. Their holder must not change across layers **or regions** of one `(object_id, claim)`; the validator keys those by object alone and reports a conflict with the original object, the first holder's original region, and `global_layer_index = None`.
- Transition-capable claims (for example `claim:sparse-fill`, `infill-generator`, `support-generator`) may vary per layer, and ordinary claims are compared only inside the same typed `(object, region, layer)` scope. `support-planner` is retained per support family at dispatch time, so family-scoped planners coexist rather than being deduplicated; no blanket cross-layer temporal policy is imposed on other claim ids.

Cross-stage transitive rule:

- If module `A` requires `B`, and `B` (directly or transitively) requires `C`, then `stage(C) <= stage(A)` must hold.
- Any violation is fatal even when the direct dependency appears legal.

### Perimeter-generator selection (`wall_generator` dedup + spiral-vase fallback)

Both `com.core.classic-perimeters` and `com.core.arachne-perimeters` declare
`holds = ["perimeter-generator"]` and are mutually `incompatible-with` each
other. Two modules holding the same non-fill claim would normally be a fatal
startup conflict, but the `perimeter-generator` claim is resolved *before*
`validate_startup_dag` runs, at module-load dedup time, by
`dedup_same_claim_modules_with_typed_wall_generator` (the `ConfigValue`-typed
twin of `dedup_same_claim_modules_with_wall_generator`; both in
`crates/slicer-scheduler/src/execution_plan.rs`, called from the
manifest-first live loaders in
`crates/slicer-wasm-host/src/execution_plan_live.rs`, which
`crates/slicer-runtime/src/run.rs` drives). Dedup keeps exactly one holder, so
`incompatible-with` never has a chance to fire.

Selection rules, in order:

1. **`wall_generator` typed global selector** — the value arrives as a typed
   global value decoded once by `slicer-config`'s ingestion
   (`ConfigIngestor` in `crates/slicer-config/src/ingestion.rs`): the
   assembled registry marks `wall_generator` as the only current startup
   claim selector (`selector = true` on its `HOST_RUNTIME_KEYS` row,
   ordinary host rows carrying `selector = false`, the `spiral_vase`
   manifest declarations omitting selector metadata so it defaults false,
   and `support_family` undeclared), so `finish()` derives
   `IngestionOutcome.selector_values` containing exactly
   `wall_generator`. Selection reads it from that map — not from the raw
   config source map — as a `ConfigScope::Global`-scope value via
   `dedup_same_claim_modules_with_typed_wall_generator`
   (`crates/slicer-scheduler/src/execution_plan.rs`, called from
   `load_live_modules_for_plan_manifest_first` /
   `load_live_modules_for_plan_with_integrated`
   in `crates/slicer-wasm-host/src/execution_plan_live.rs`). Values:
   `"classic"` (default) or `"arachne"`; an absent or unrecognised value
   falls back to `DEFAULT_WALL_GENERATOR`
   (`crates/slicer-ir/src/resolved_config.rs`, `"classic"`), and when the
   preferred module is not among the loaded candidates the dedup falls
   through to its alphabetical default. Because only a registry entry
   declared `selector = true` enters the selector channel, `spiral_vase` and
   `support_type` remain ordinary typed global values (read from the global
   `ScopeDelta`), and undeclared `support_family` is warn-and-kept. This
   closes the wall-generator selection record (before it, dedup silently
   kept the alphabetically-first candidate — `arachne-perimeters` — with no
   way for a user to express intent). The production composition roots
   thread the same typed ingestion result (`IngestionOutcome`) through plan
   binding and config resolution (`crates/slicer-runtime/src/run.rs`).

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

### Support-generator selection (retained family candidates, per-region pairing)

Both `com.core.traditional-support` and `com.core.tree-support` declare
`holds = ["support-generator"]`, and each pairs with its planner by holding a
shared `support-family:<id>` claim (see `docs/03_wit_and_manifest.md` §
"Support-family claims" and the Planner-Renderer Pairing section below).

Unlike `perimeter-generator`, `support-generator` is **not** resolved to a
stable single owner by global startup dedup. It is excluded from
`dedup_same_claim_modules_with_wall_generator`
(`crates/slicer-scheduler/src/execution_plan.rs`), exactly like the fill-role
claims: every family candidate is **retained** through load and dispatch, and
the active renderer is selected **per region** at dispatch time via the
planner-renderer family pairing. There is no global "first winner"
`support-generator` dedup that drops losing families.

Selection rules:

1. **`support_family` config key** — the canonical per-region selector
   (`SUPPORT_FAMILY_CONFIG_KEY`). `support_type` (`SUPPORT_GENERATOR_CONFIG_KEY`,
   `"support_type"`) is a **compatibility alias** that, when present, overrides
   the canonical value. Values are OrcaSlicer's raw spellings carried in the 3MF
   sidecar next to the raw `enable_support` key: values starting with `tree`
   (`tree(auto)`, `tree(manual)`) or with `hybrid` (legacy `hybrid(auto)`,
   which OrcaSlicer itself migrates to `tree(auto)` at config load) select the
   tree family; values starting with `normal*` or `classic*` select the
   traditional family; absent values default to traditional. Manual
   (enforcer-only) variants select the same family as their auto counterpart —
   pnp has no enforcer-only concept. Resolution is `select_support_family`
   (`crates/slicer-scheduler/src/execution_plan.rs`), applied per region, and
   drives per-region dispatch: `module_receives_slice_region`
   (`crates/slicer-wasm-host/src/dispatch.rs`) checks a module holds a
   `support-family:` claim and, if so, only routes it a region when
   `module_claims_match_active_region`
   (`crates/slicer-scheduler/src/execution_plan.rs`) confirms the module's
   `support-family:<id>` matches the region's resolved family. Modules with no
   `support-family:` claim receive every region unconditionally.

2. **Pairing validation is warning-only** — a planner or renderer whose
   `support-family:<id>` has no matching counterpart holding the same `<id>`
   produces a structured warning diagnostic at load
   (`validate_support_family_pairing`); it does not abort startup. The
   incomplete family's regions simply have no complete planner/renderer route
   and produce no support plan (degraded) until the pair is completed. There
   is no alphabetical fallback that promotes a lone `support-generator`
   candidate.

Because selection is per region and family-atomic, `support-generator` is
**not** a stable single-owner claim: the Allowed Claim Transition Matrix in
`docs/01_system_architecture.md` permits it to transition across geometry
phases, and the retained-candidate / per-region pairing semantics are
normative in the Planner-Renderer Pairing section below.

### Planner-Renderer Pairing (Normative — packet 220)

Support family selection is **atomic per region**: the planner and the
renderer are resolved together from one family, never independently. This
`planner-renderer` pairing is the unit of family selection. For a
given region, the selected `support_family` determines both the
`support-planner` module (PrePass) and the `support-generator` renderer
(`Layer::Support`) that serve it — they are paired by the shared
`support-family:<id>` claim (see `docs/03_wit_and_manifest.md` §
"Support-family claims"). There is no global "first winner" `support-generator`
dedup that drops losing families.

**Tree family dispatch (packet 221).** When `support_family` resolves to the
tree family for a region, the host dispatches the tree pair —
`tree-support-planner` (PrePass) + `tree-support` (renderer) — via the shared
`support-family:tree` claim, per region.

**Traditional family dispatch (packet 222).** When `support_family` resolves to
the traditional family for a region, the host dispatches the traditional pair —
`traditional-support-planner` (PrePass) + `traditional-support` (renderer) — via
the shared `support-family:traditional` claim, per region. The renderer emits
only the planner's polygons; disabled or declined inputs produce no fallback
support paths.

- **Per-region candidate retention.** All family candidates are retained
  through load and dispatch. The host does not globally dedup away a
  `support-generator`; each region keeps its own candidate set so a family
  can be selected per region.
- **Pairing validation is warning-only.** A planner or renderer whose
  `support-family:<id>` has no matching counterpart holding the same `<id>`
  (missing or mismatched pair) yields a structured warning diagnostic at load,
  not a fatal startup error and not a runtime fallback. The incomplete
  family's regions produce no support plan (degraded) until the pair is
  completed; complete pairs such as `tree-support-planner` + `tree-support`
  dispatch normally.
- **Host aggregation as the sole multi-writer merge point.** The host is the
  only place that merges output from multiple family writers. Aggregation
  merges on declared identity (`family_id`, `global_layer_index`, `object_id`,
  `region_id`, `anchor_z`) and checks each entry's ownership against
  `family_assignments` plus the producing module's `support-family:<id>` claim,
  default-deny. Ownership is therefore a pure function of the entry and the
  assignments, so the merged result is reproducible regardless of which family
  produced which body and independent of plan arrival order.
- **Complete-body validation.** A complete body is validated against the
  exact-Z occupancy and a maximum body extent bound (`MAX_BODY_EXTENT_UNITS`,
  `1 << 22` units = 419.43 mm on each axis — the build-plate bound from the
  ADR-0059 Ruling 3 amendment; measured per body cross-section, not per
  identity aggregate. See the host `exact_z_query` service in
  `crates/slicer-wasm-host/src/exact_z_query.rs`). The extent bound is a pure
  check — it neither assigns nor partitions. An invalid complete body is
  dropped, not clipped or replaced by a fallback filler.
- **Structured unmet diagnostics with degraded continuation.** A declined or
  unroutable candidate surfaces a structured unmet diagnostic and the pipeline
  continues in a degraded state; it is not silently filled.
- **No missing-plan fallback filler.** When a family's plan is missing, the
  host does not substitute a fallback filler — the region is left without
  that family's output and the unmet condition is reported.

Aggregation keys on declared identity, applies default-deny family ownership,
enforces `MAX_BODY_EXTENT_UNITS`, and applies the documented own/foreign
territory trim rule. Routing-cell ownership is not itself a contract, and
positive-area cross-family swept-path conflicts drop both complete bodies and
mark their demands unmet; tolerance-only boundary contact is retained. Such
diagnostics carry the family, body, demand, and conflict reason.

Anchored-event swept-path checks are normative: a positive-area cross-family
swept-path conflict drops the complete bodies and marks their demands unmet,
while tolerance-only boundary contact is retained. Diagnostics identify the
family, body, demand, and reason.

The host-owned Exact-Z Support Query is keyed by `(object_id, region_id,
physical Z)`. Coordinates are normalized to repository units; results are
immutable and cached. Each result contains occupancy, blockers, eligible
termination geometry, and the baseline feasible envelope.

### Write Conflict vs Claim Conflict — Enforcement Level Summary

These two mechanisms are complementary, not redundant. Understanding the
difference is important when designing modules that share IR fields.

|                      | Claim Conflict                                                   | Write Conflict                                                                                                           |
|----------------------|------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------|
| **What it detects**  | Two modules both intend to be the primary generator of a feature | Two modules both write the same IR field with no ordering between them                                                   |
| **Granularity**      | One claim name per feature category (coarse)                     | Per IR access path (fine)                                                                                                |
| **Caught at**        | Startup, validation pass 1–2                                     | Startup, validation pass 8                                                                                              |
| **Typical cause**    | User enables two infill modules simultaneously                   | Developer adds a new PostProcess module that overwrites a field another module already modifies                          |
| **Resolution**       | Region overrides; disable one module                             | Declare `incompatible-with`, OR have one module read the field it will overwrite (establishing ordering), OR use a claim |
| **Runtime fallback** | None — fatal startup error                                       | None — fatal startup error                                                                                               |

A claim conflict always implies a write conflict on the claim's primary output
field. A write conflict does not always imply a claim conflict — two
PostProcess modules that both transform `PerimeterIR.regions.walls` may both
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

### Topological Sort (Kahn's Algorithm)

Two different sorts exist, and they promise different things:

- **Public sorter.** `slicer_scheduler::topological_sort`
  (`crates/slicer-scheduler/src/topology.rs`) is Kahn's algorithm over a `BTreeSet` of ready
  modules and takes `pop_first()` at every step, so the emitted sequence is the
  **lexicographically minimum** module ID among the currently ready nodes — not FIFO or insertion
  order. Each node's destinations are collapsed into a `BTreeSet`, so a destination repeated with
  several reasons increments in-degree once. On success the result is a bijection over the declared
  nodes that satisfies every edge.
- **Validator sorter.** `validate_startup_dag` uses its own private Kahn pass (a `VecDeque`
  in-degree-0 queue) to fill `SchedulerError::CyclicDependency.cycle`. It makes no ordering
  promise; only the set matters. Consumers must not assume a simple-cycle route or lexical order
  from it.

**Blocked remainder, not cycle members.** Both sorts return the same set, and it is larger than
"the cycle": a node never reaches in-degree zero exactly when it lies on a cycle **or** is
reachable from one. So the reported blocked set is the cycle plus every descendant — including an
acyclic downstream tail — and it excludes nodes that are independently sortable. The public
`Err` vector is lexically sorted because it is collected from a `BTreeMap`; the private one is in
node order.

Property `H-DAG` (`docs/testing/properties.md#h-dag`, evidence in
[proptest-scheduler-evidence.md](testing/proptest-scheduler-evidence.md)) compares the public
sequence to an exhaustive lexical-minimum linear extension over all node permutations, and compares
both the public `Err` set and the `CycleDetection` set to
`slicer_sdk::test_support::property_testing::oracles::graph_cycle_remainder`, which is a
nonempty-path reachability model rather than a second sorter.

### Write Conflicts (validation pass 8)

`validate_write_conflicts` (`crates/slicer-scheduler/src/validation.rs`) compares every unordered
pair of nodes in one stage DAG, per shared **exact** write path, and skips the pair only when an
ordering genuinely resolves the shared field:

- the other module declares that exact path in its `ir_reads` **and** a directed path exists from
  the upstream writer to it (a consumed-field transform chain), or
- both modules are family-support holders and the shared path is exactly `SupportPlanIR` or
  `SupportIR` (or a dot-descendant of either). `SupportIRx`, unrelated fields and a bare
  `requires_modules` edge are not exempt: ordering without consumption is not a transform.

Everything else is a fatal `WriteConflict { field, module_a, module_b, stage, orderable: false }`,
reported as a normalized set because request node order can orient the pair. The `orderable` hint
in the documented enum describes a *possible* repair (declare the read), not an observed
exemption; the validating pass reports `false` for conflicts it emits. Property `H-CLAIMS`
(`docs/testing/properties.md#h-claims`) models reachability and exact shared-path consumption
independently and also pins the negative mutations (ordering without consumption, the `SupportIRx`
pseudo-exemption, an unordered pair) in
`scheduler_write_dependency_and_disabled_controls`.

---

## RegionMapIR Compilation (PrePass::RegionMapping)

> This section owns how `RegionMapIR` is **built and bounded**. The struct shape
> and field semantics (`RegionPlan`, `RegionKey`, config-key namespaces, override
> precedence) are defined in `docs/02_ir_schemas.md` IR 4; `docs/01_system_architecture.md`
> describes why the stage exists.

`PrePass::RegionMapping` is host-built-in and precomputes per-region execution context so Tier 2 has no config or claim resolution overhead.

### Unified config-resolution entry points (Normative — TASK-566)

The `slicer-config` resolution module exposes two unified entry points, replacing
the scattered resolvers and `overlay_resolved`:

- `query_z_grid` is called by `PrePass::LayerPlanning` to resolve the inputs
  that determine the layer Z grid and the typed per-object planning records.
  Each record carries `layer_z_tops`, the explicit object-local top-Z schedule
  derived from the object's composed layer-height profile, which
  `run.rs::layer_planning_objects` carries across the WIT seam as `layer-zs`.
- `query_layer_height_profile` is the shared producer of the canonical
  layer-height profile: it composes the resolved object base height, the fixed
  first-layer interval, and every matching **layer range**'s `layer_height`
  into literal `(z_start, z_end, height)` segments. Range endpoints are authored
  world-space Z: `query_z_grid` resolves all objects first and subtracts the
  shared highest raft top before composing each object-local profile. The fixed
  first-layer interval retains world Z `[raft_offset, raft_offset + first_layer_height)`;
  ranges below the object are skipped or clamped at local zero. The guest adds
  that same shared offset to every object's schedule. `layer_top_zs` evaluates
  that profile into the object-local top-Z schedule, so the schedule the guest
  plans against and the profile the host composed can never disagree.
- `resolve_scope_stack` is called by `PrePass::RegionMapping` to resolve the
  applicable typed scope deltas for each active region. The same **layer range**
  set feeds it: when a range covers the region's layer top Z
  (`ResolutionTarget.layer_top_z`), its typed values apply between the object
  and modifier scopes. Range membership is decided by `LayerConfigRange::covers`
  in `slicer-config`, which the runtime kernel also calls before re-resolving a
  layer, so both consumers agree by construction — no caller re-derives range
  precedence locally and no runtime path re-parses XML.

### Bounds-only scalar validation (Normative)

Bounds-only scalar validation is a separate, narrower seam from scope admission:
`ConfigBoundsIndex::{from_declarations, check}` in
`crates/slicer-scheduler/src/config_resolution.rs` intersects every module's declared
`[min, max]` per key and rejects a numeric scalar or list element that falls outside the
intersection, reporting the first offending list index. Its scope is exactly that:

- an entirely absent key returns `Ok(())`;
- a non-numeric (`Bool`/`String`) value returns `Ok(())` — variant mismatch is
  `ResolvedConfig::apply_cli_key`'s job, not this API's;
- a present declaration with neither min nor max still reaches the finiteness check, so
  NaN/infinity is rejected there;
- `Percent` and percent-authored `FloatOrPercent` are checked for finiteness and the **min**
  only, because a declared max is expressed in the key's absolute unit while a raw percentage's
  absolute equivalent is only known after Phase-B expansion. Absolute `FloatOrPercent` uses the
  full bounds.

No cross-field or whole-config validity is inferred, and the private `check_scalar` /
`check_value` helpers stay private. The public behaviour is pinned by the
[`H-CONFIG-BOUNDS`](testing/properties.md#h-config-bounds) property charter against an
independent all-declarations predicate, including inclusive edges, empty intersections,
one-sided bounds, first-invalid-index and percent-versus-absolute distinctions.

### Config scope admission (Normative — ADR-0069)

Scope eligibility is derived from the registry, not from any manifest-table
allow list. The registry's admission state is built from every declaring
manifest's per-key `denied_scopes` entries via
`ConfigSchemaRegistry::admission_set`, and resolution consults that admission
state: a key stated at a scope it denies is rejected loudly wherever it is
stated, surfacing `ResolutionError::ScopeDenied` (naming key and scope) rather
than being silently ignored. `denied_scopes` is the sole per-key eligibility
mechanism for both host and module-declared keys (see "Common per-field keys"
in `docs/03_wit_and_manifest.md`); when a key is declared by several declarers,
the denials union across them — a scope denied by any declarer is denied.

During region mapping, modifier volume `config_delta.fields` from every
`modifier_volume` attached to a region's parent `ObjectMesh` are consumed by
`resolve_scope_stack` in priority-ascending, last-writer-wins order, admitted
through the config schema registry (type-checked, bounds-checked, and subject
to the ADR-0069 scope admission above), and stamped into
`RegionPlan.config.extensions`. Modifier volumes whose typed kind is
`ModifierKind::SupportEnforcer` or `ModifierKind::SupportBlocker` are filtered
out for OrcaSlicer parity (canonical `PrintApply.cpp`). Implemented
modifier-volume splits (packets 131/132) bind configuration to geometric
sub-regions: modifier meshes are sliced per layer during prepass, the
cross-sections are intersected with the owning region's partitioned fill
polygons at partition time, and each resulting wall-less sub-region carries its
own `region_id` + config binding (per-region delivery through the region-view
config accessor). The `SupportEnforcer` / `SupportBlocker` kinds remain
filtered and never produce sub-regions; paint variant-splits remain the other
per-region producer.

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
   ActiveRegion)` — see `docs/02_ir_schemas.md` IR 4 § Config Interner.
3. Detect Scalar paint values defensively: any `PaintValue::Scalar`
   encountered in a region-split path becomes
   `RegionMappingError::ScalarInRegionSplitFacetValue` (Packet 93 guard, the
   manifest validator from Packet 92 normally catches it first).

`enumerate_canonical_chains` produces chains deterministically from the axis order it is
**given**, and preserves the caller's value vectors verbatim rather than sorting them. The
`Flag < ToolIndex(0) < ToolIndex(1) < … < Custom(s_lex)` value order is supplied by
`scan_paint_data` (`crates/slicer-core/src/algos/region_mapping.rs`), which de-duplicates each
semantic's values through a `BTreeSet` ordered by `paint_value_canonical_cmp` in that file before
the enumerator sees them.

The scheduler's separate `canonical_variant_chain_order`
(`crates/slicer-scheduler/src/region_split.rs`) returns the intended `(priority, name)` sequence,
but it has **no production caller in the current tree**. The RegionMapping driver passes
`aggregated_region_split` to `execute_region_mapping_inner`
(`crates/slicer-core/src/algos/region_mapping.rs`), which builds its axis order from the
aggregate's `BTreeMap` keys — that is, **name order** — and hands that to the enumerator. Actual
cross-product chain axes are therefore name-ordered even where that differs from the scheduler
helper's `(priority, name)` sequence. This is a source-observed discrepancy, not a policy
decision to prefer name order; ADR-0018's amendment records it, and auditing or changing the
driver needs coordinated fixture work outside this scheduler contract. Property `H-REGISTRY`
proves the aggregate, the helper sequence and the map's name-ordered keys; it does **not** prove
helper integration with RegionMapping.

**Cap and overflow:** `DEFAULT_REGION_MAP_CAP = 750_000` (raised from
`1_000` in Packet 93 to accommodate worst-realistic envelopes of 16
colors × 1000 layers × 16 regions × ~3 modifier subtypes). Overflow
surfaces `RegionMappingError::CapExceeded` naming
the top contributor's `(object_id, region_count, layer_count)` so callers can diagnose which object
exploded the cross-product.

**Cross-crate dependency:** `AggregatedRegionSplitEntry` is owned by `slicer-ir::slice_ir`; `slicer-core` imports it from `slicer-ir` and no longer has a normal `slicer-scheduler` dependency.

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
                config: interned ConfigId, // Packet 91: index into RegionMapIR.configs, not a cloned ResolvedConfig (see 02 IR 4)
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
// Illustrative — the real method is `ExecutionPlan::resolve_active_regions`
// in `crates/slicer-scheduler/src/execution_plan.rs`.
fn resolve_active_regions(
    plan: &ExecutionPlan,
    layer: &GlobalLayer,
    module: &CompiledModuleStatic,
) -> &[ActiveRegion] {
    plan.module_region_index
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

The resolved-config-view packet strengthens both legs: the frozen-at-load
view is bound by `bind_module_config_view` from the fully resolved config, so
every declared key with a registry default or an authored value is present and
the load-time view is registry-complete; and the per-region overlay rides on
the same resolved base, so a region view also contains every declared key —
authored or seeded default — with the region's overlay applied on top.

### Ordinary full-identity delivery (Normative — TASK-572 row 1)

`build_layer_selection_index(LayerSelectionIndexInput)` (`crates/slicer-wasm-host/src/selection.rs`)
builds the whole-print `LayerSelectionIndex` once, after late PrePass and global-layer promotion,
via the runtime late-bind seam `build_and_commit_layer_selection_index` (`crates/slicer-runtime/src/layer_executor.rs`).
The index is committed as one immutable
Blackboard product, with no partial publication. `selection_for(global_layer_index, module_id)`
is O(1); a missing index or missing bound pair is an internal-consistency fatal, never a fallback resolver.
The index contains selection metadata only: delivered geometry always comes from current call-local products associated by full `RegionKey`/`OriginId`.

During construction, model authority requires `entries.get(&key)` before `config_for(&key)`
(the Config Interner Contract in `docs/02_ir_schemas.md`). A missing exact key produces
`SelectionError::MissingModelConfig`, wrapped by `LayerSelectionBuildError` and mapped by the runtime seam
to `LayerStageError::FatalModule`, naming stage, module and exact key before any Layer invocation.
Default, sibling and first-match config authority are forbidden.

`variant_chain` is ordered and is never sorted or regenerated. `SelectedLayerTarget`'s
`LayerTargetSource::Model { key, members }` retains the base plus matching merged modifier
sources; config authority is the base's exact key. This section owns the painted-id encoding:
`RegionMapIR.entries` keys painted variants by BASE region id plus ordered chain, whereas
post-paint `SliceIR`/`PerimeterIR` ids are DERIVED by `paint_variant_region_id` (`crates/slicer-core/src/algos/paint_segmentation/mod.rs`)
as `base * PAINT_VARIANT_REGION_ID_STRIDE + h`.
Only inside config resolution, the builder derives a temporary map-space address by
`id / PAINT_VARIANT_REGION_ID_STRIDE` for non-modifier nonempty-chain ids;
modifier-namespace ids and empty-chain ids stay verbatim. This is exact config addressing, not fallback.
Ordinary `Layer::Infill` delivery and commit preserve source-space object/region IDs and the complete ordered chain,
exactly like perimeter and postprocess delivery and commit; stored source identities are never normalized.

Non-region contexts use explicit `LayerTargetSource` provenance, never a fabricated model key.
The selection describes EVERY role the module holds on the layer, each from its own
`LayerTargetSource`: model targets (the perimeter/fill products selected by full
`RegionKey`), `Layer::Support` carriers, the raft context (`LayerTargetSource::Raft`,
`GlobalLayer.is_raft`) and the anchored stage (`LayerTargetSource::AnchoredStage`).
Targets are ordered source outputs enumerated in deterministic source order — model
regions in committed slice order, carriers in immutable plan-entry order
(`LayerTargetSource::SupportCarrier.entry_index`) — never temporary config-map
addresses: each target's config is resolved once at build time through exact-key
identity addressing, and invoke-time delivery re-associates CURRENT call-local
geometry by full `RegionKey`/`OriginId` on top of that ordered identity.
Support carriers retain `SupportPlanEntry` provenance and use the resolved, declared-key-filtered
`invocation_config` from `bind_module_config_view` — the frozen invocation/non-region
config, never a per-region config-map read.
Carrier layer eligibility is `global_layer_index == layer.index`;
`anchor_layer_index` is provenance only.
Carrier exclusions are complete and exact (`eligible_carrier_entries`,
`crates/slicer-wasm-host/src/selection.rs`) — an entry failing any one of the
following is not selected and not delivered as a carrier:

- it belongs to a support family the module does not claim (the
  `support-family:` claim prefix);
- its `global_layer_index` differs from the physical layer;
- it carries a decline reason;
- every one of its roles is empty geometry;
- the committed slice already contains a region of the SAME full origin
  (object + source region + exact ordered chain) — presence exclusion is
  full-origin, so a slice row differing only by chain never excludes a
  carrier, and a present-but-EMPTY slice row excludes by identity presence
  alone, polygon content being irrelevant (covered by
  `model_identity_and_non_region_authority` in
  `crates/slicer-wasm-host/src/selection.rs`).

Eligible carriers are delivered even with `slice = None` (plan present, slice
product absent). Carrier wire handles (`support_carrier_regions`, same file)
are one `SlicedRegion` per full source origin, copied exactly from the plan
entry (chain-differing carriers stay distinct handles) and sorted
deterministically by `(object_id, region_id)`. Carriers hold NO model or
coloring authority: they receive the non-fill claim set, never resolve the
model config map, and are granted no authored coloring and no role-tool
authority (`authored_coloring = false`, `resolved_role_tool = None` on the
`SelectedLayerTarget`). Raft context derives from `GlobalLayer.is_raft` for
`Layer::Infill`/`Layer::Support`.
`AnchoredStage` records the physical layer index with invocation config as module execution context, not a synthetic anchored closure/entity.

Extruder/tool authority is host-only. The host resolves the tool authority of
each selected model target at build time from the target's resolved config
(`extruder` extension → `SelectedLayerTarget.resolved_role_tool`,
`crates/slicer-wasm-host/src/selection.rs`), and the marshal/commit boundary
applies it to committed output; carriers, raft and anchored targets carry
`None`, so a module can never mint, override or extend tool selection — it
only reads what the host delivers.

`OptionalProductPresence::{Absent, Present}` distinguishes absence from present-empty; `invoke`
is not `!targets.is_empty()`: under `paint_allows_layer`, a paint-transparent
non-support module on an empty present slice retains `invoke = true` with no targets. `Ok(None)` preserves no-output semantics.

Both native and WASM legs consume the same call-local `LayerStageInput.selection` (`crates/slicer-wasm-host/src/binding.rs`).
`WasmRuntimeDispatcher::run_stage` (`crates/slicer-wasm-host/src/dispatch.rs`) returns `LayerStageError::FatalModule`
for a scheduled module with `selection = None`, invoking and committing nothing.
With a selection and `invoke = false`, neither leg invokes. Native production uses
`build_native_layer_request_from_selection` and `commit_native_layer_response_with_authored`
(`crates/slicer-wasm-host/src/marshal/native.rs`); old direct builder/commit wrappers remain
for legacy direct callers only and are deny-by-default for coloring.

`AuthoredColoringContext` grants are full-origin `HashSet<OriginId>` values derived from selected model targets.
`allows` checks the complete origin and tool range; a sibling variant is never authorized by another variant's grant.
Enforcement remains at the marshal/commit boundary in `enforce_authored_coloring` (`crates/slicer-wasm-host/src/marshal/out.rs`),
silently stripping unauthorized or out-of-range authored tool indices to `None`.

This rule supersedes Packet 51 for module-backed Layer stages bound to the whole-print selection index:
config authority is build-time exact-key resolution carried by `LayerSelection`
(per-target `config` and `perimeter_configs`), not fresh per-invocation `RegionPlan.config`
reconstruction inside `dispatch_layer_call`. Both rules must be read together. The Packet 51
section is retained unchanged for stages outside this indexed contract: `is_indexed_module_stage` (`crates/slicer-runtime/src/layer_executor.rs`)
distinguishes the roster slice the index is built from; host-built-in/non-roster stages keep the pre-index path.
`bind_module_config_view`'s frozen view and explicit `invocation_config` remain authoritative only for invocation-wide/non-region reads.

### PrePass Config-View Plumbing (Normative — Packet 73)

Every module-implementable PrePass export (`mesh-analysis`, `layer-planning`,
`seam-planning`, `support-geometry`) receives a `config-view`
parameter providing read-only access to declared config keys, normalised
across stages by Packet 73 (the `support-geometry` runner was the final
holdout). Modules declaring no `[config.schema]` receive an empty
`ConfigView`. Config keys are looked up by string name; with the
resolved-config-view packet, resolution seeds every declared key that has a
registry default, so a declared key is absent from a view only when neither a
registry default nor an authored value exists (undeclared-absent /
declared-present): an undeclared key is never visible, and a declared key
carries its registry default or an authored value. Reads of a declared key
that is genuinely absent still return `None` (or `Err` from the
`require_*` accessors). The `support-geometry` runner specifically:

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
    // + requires_modules, layer_parallel_safe
    // + region_split_semantics (empty unless the manifest set paint_only = true; ADR-0071)
}
```

The authoritative definitions are `ExecutionPlan`, `CompiledStage`, and
`CompiledModuleStatic` in `crates/slicer-scheduler/src/execution_plan.rs`.
The scheduler is wasmtime-free (packet 85): no `WasmInstancePool` lives on
the plan — WASM instance pools are owned by the runtime
(`slicer-wasm-host`), keyed by module, sized by `layer_parallel_safe`
(N instances for parallel-safe modules, 1 for sequential).

### Anchored invocation closure

An anchored event is scheduled through `AnchoredInvocation`, constructed by
`ExecutionPlan::anchored_invocation`. Its stage closure is
**capability-derived**: the scheduler derives the required stages from the
entity's declared input and output capabilities rather than consulting a
hardcoded event-kind table or feature-owned stage list. The resulting
`CapabilityDerivedEventClosure` is carried with the invocation and checked
as part of the frozen plan.

The invocation also preserves the manifest's `layer_parallel_safe` hint.
Parallel-safe anchored invocations may run concurrently against immutable
prepass state; serial modules retain the single-instance execution path.
Either mode produces the same ordered event collections, while the closure
keeps capability requirements explicit in the scheduler plan.

### Runner-Trait Input Borrow Structs (Normative — Packet 83)

Runner trait signatures (`PrepassStageRunner::run_prepass`,
`LayerStageRunner::run_layer`,
`FinalizationStageRunner::run_finalization`,
`PostpassStageRunner::run_postpass`) accept IR-typed `*StageInput<'a>`
borrow structs rather than raw `&Blackboard` or `&LayerArena`. This
decouples the dispatcher (which lives in `slicer-wasm-host`) from
runtime-owned aggregates (which stay in `slicer-runtime`):

```rust
// Illustrative shape — the authoritative definitions are `LayerStageInput`,
// `PrepassStageInput`, `FinalizationStageInput`, and `PostpassStageInput` in
// `crates/slicer-wasm-host/src/binding.rs`. Every field is an `Option` borrow
// (or `Arc`) projected from the Blackboard / `LayerArena` before dispatch;
// representative fields of `LayerStageInput`:
pub struct LayerStageInput<'a> {
    pub mesh:                     Arc<MeshIR>,
    pub seam_plan:                Option<Arc<SeamPlanIR>>,
    pub support_plan:             Option<Arc<SupportPlanIR>>,
    pub region_map:               Option<Arc<RegionMapIR>>,
    pub slice:                    Option<&'a SliceIR>,
    pub perimeter:                Option<&'a PerimeterIR>,
    pub surface_classification:   Option<&'a SurfaceClassificationIR>,
    // … further per-stage optional borrows (infill, layer_collection,
    //     prepared regions); see `binding.rs`.
}
```

Dispatch is provenance-routed: `CompiledModuleLive.native_entry` decides
whether a stage uses the native call path or WASM instantiation. The marshalling
boundary is shared between transports; the native path re-enters at the
`*OutputCollected` accumulator layer, so the `out.rs` converters and
`origin.rs` `OriginBucket` re-attribution run unchanged. Only the input-view
leg differs. Module logic is single-threaded on both paths (ADR-0056 Decision
item 5).

Native calls do not lease WASM instances, but each dispatch still performs one
module invocation and preserves stage/module ordering. Host-layer fan-out is
the only permitted concurrency; skipping instance-pool bookkeeping does not
permit concurrent calls within a module.

For live bindings, construction rejects an Integrated-provenance module that
has no matching `NativeStageEntry`, naming both the module and stage. External
provenance remains WASM-backed. An Integrated module with a matching native
entry is never silently skipped or routed through an absent native function:
the existing phase-appropriate `MissingComponent`/fatal dispatch error is the
loud failure for a missing entry.

Native dispatch currently leaves all `last_*` capture channels empty,
including profiling, fuel, and batch-call captures. This is a known
packet-local limitation, not evidence that module execution failed.

Native commit is lossless for every declared supported stage output: it
preserves all output variants, commits explicit empty postprocess results,
retains region IDs and seam reasons, and does not fatal on outputless
`PrePass::PaintSegmentation` when the WASM leg is also outputless. For
`Layer::InfillPostProcess` specifically, an invocation that ran and re-emitted
nothing commits the empty replacement set on both legs (ADR-0028 §Amendment
2026-09-29); `Layer::Infill` treats empty output as no contribution.

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
`skirt-brim` + `wipe-tower`). Modules execute SEQUENTIALLY in stage-DAG topological order (manifest `requires` edges). Two modules MUST NOT hold the same claim (claim conflict → DAG validation failure). Example split:

| Module                        | Orders after (`[compatibility].requires`)              |
|-------------------------------|--------------------------------------------------------|
| `overhang-classifier-default` | — (no requirements; holds no claim)                    |
| `part-cooling`                | — (no requirements; holds no claim)                    |
| `skirt-brim`                  | — (no requirements; holds no claim)                    |
| `wipe-tower`                  | `skirt-brim`, `part-cooling`, `top-surface-ironing` (holds no claim) |

A finalization module is permitted to be unconditionally `layer_parallel_safe = false` (enforced by Phase 2 DAG construction); modules in the same stage execute in dependency order without any mutual-exclusion machinery. `wipe-tower`'s manifest declares `[compatibility].requires = ["skirt-brim", "part-cooling", "top-surface-ironing"]` to force itself last. Top-surface ironing is not a finalization module: it runs at `Layer::Infill`, gated on `top_solid_fill` and ordered after infill via compatibility `requires` (packet 38-rev1's finalization placement was superseded).

### Model Loading — 3MF Sidecar Parse Order (Normative — Packet 56)

Inside `load_3mf` the host opens the 3MF ZIP archive and calls
`parse_3mf_sidecar(&mut zip)` BEFORE the `ZipArchive` is dropped, then
parses the model XML with the sidecar threaded through:
`parse_3mf_model_xml(&xml_bytes, &sidecar, ...)` feeds `resolve_object`
(unused in Packet 56 — branched only in Packets 56b/56c). Missing
sidecar files return an empty map silently; malformed XML returns an
empty map plus a `log::warn!` on the `slicer_model_io::sidecar` target.
Either way `load_model` returns `Ok(MeshIR)` — sidecar failure is
non-fatal and falls back to treating all parts as `NormalPart`. See
`docs/02_ir_schemas.md` § Host-Local Sidecar Types for the exact
return types.

### Modifier-Part and Negative-Volume Routing (packets 56b / 56c)

Modifier parts (3MF `Metadata/model_settings.config`) are routed into `MeshIR.objects[].modifier_volumes` by the host loader (packet 56b). Each modifier volume carries its routing classification as the typed `ModifierVolume.kind` field (`ModifierKind`) across the IR seam (ADR-0070), not a `config_delta` string. Negative-volume and support-kind modifiers (`ModifierKind::NegativePart`, `ModifierKind::SupportEnforcer` / `ModifierKind::SupportBlocker`) are applied by the per-layer negative-part subtract host stage described in the next section (packet 56c): the host subtracts negative-volume geometry per layer and routes support-kind modifiers into the Support claim's per-region override stream.

#### Negative-Part Per-Layer Subtract (Normative — Packet 56c)

Negative-part subtract is a **per-layer host stage** inserted inside
`run_paint_annotation` (`crates/slicer-runtime/src/layer_executor.rs`), after `arena.take_slice()`
returns the layer's `SliceIR` and BEFORE the paint annotation loop
begins. This insertion point is binding:

- Earlier designs put the subtract in a prepass phase-0 built-in or in
  `crates/slicer-runtime/src/pipeline.rs`; both were infeasible because `Vec<SliceIR>` is
  produced per-layer during execution, not during prepass.
- The per-layer seam guarantees paint annotation and all downstream
  per-layer consumers (perimeters, infill, support) see post-subtract
  polygons.

Per-layer call order is locked:
`arena.take_slice()` → `apply_negative_part_subtract(...)` →
`run_paint_annotation` loop → downstream per-layer stages.

For each `ModifierVolume` whose kind is `ModifierKind::NegativePart`, the
stage projects the modifier mesh at `slice_ir.z` via
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
| `PrePass::MeshAnalysis`            | — (no prerequisites)                                                      |
| `PrePass::LayerPlanning`           | `SurfaceClassification`                                                   |
| `PrePass::Slice`                   | `SurfaceClassification`, `LayerPlan`, `RegionMap`                         |
| `PrePass::OverhangAnnotation`      | `SurfaceClassification`, `LayerPlan` — reads the committed `SliceIR` and writes `overhang_quartile_polygons` into `SurfaceClassificationIR`. Runs after `PrePass::Slice`. |
| `PrePass::ShellClassification`     | `SurfaceClassification`, `LayerPlan`, `RegionMap`, `SliceIR`              |
| `PrePass::SeamPlanning`            | `LayerPlan`, `SliceIR`, `RegionMap` — reads the committed `LayerPlan` plus per-region `SliceIR` geometry and annotations (projected via `SeamPlanningView`, packet 178); writes `SeamPlanIR`. Dispatch occurs only after these products are committed. |
| `PrePass::PaintSegmentation`       | — (empty; ordered by execution sequence after `ShellClassification`). Reads the committed `SliceIR`; produces split `SliceIR` via `replace_slice_ir`. |
| `PrePass::RegionMapping`           | `LayerPlan`                                                               |
| `PrePass::SupportAnalysis`         | `SliceIR`                                                                 |
| `PrePass::SupportGeometry`         | `SurfaceClassification`, `LayerPlan`, `RegionMap`, `SliceIR`, `SupportGeometry` (the last committed by the host built-in within this stage before the guest runs) |

A stage scheduled before its prerequisites are committed produces
`PrepassExecutionError::MissingRequiredPrepass { stage_id, slot }` and aborts
the prepass without invoking any module. This guard short-circuits before
dispatch so module-side error handling for "the IR I need wasn't committed"
is unnecessary.

Bridge classification runs inside `PrePass::Slice`:
`classify_unsupported_bridge_areas` applies canonical
`PrintObject::detect_surfaces_type`'s whole-set rule —
`opening_ex(diff(current_raw, safety_offset(previous_raw)), 0.1 x external_perimeter_width)`
— to each region. A missing previous layer clears the candidates; an existing
empty layer leaves the whole cross-section unsupported. `ShellClassification`
does not subtract a second lower layer. Cross-layer internal-bridge
qualification runs only after the `ShellClassification` shell state is
committed, and it is the single internal-bridge producer: it writes
`internal_bridge_areas`, one bridge angle per polygon in
`internal_bridge_angles_deg`, and the qualified polygons into `bridge_areas`
(plus the optional extra-bridge-layer duplicates at the parent angle + 90°).
The `claim:bridge-fill` holder emits them during `Layer::Infill`; the
`InfillPostProcess` commit arm constructs no internal-bridge paths.

`ShellClassification` consumes the classified bridge areas plus previous-layer
contours and writes `bridge_orientation_deg`. Empty classified areas leave the
existing orientation unchanged. External orientation is computed from the
classified geometry and raw lower contours by floating-edge normal-cost
minimization; empty candidates use the minor principal axis, and degenerate
geometry falls back to `0°`.



#### Precision-Key Touch Points (packet 60)

`PrePass::Slice` (host-built-in): reads `slice_closing_radius` from `ResolvedConfig`; this key is consumed by `slicer_core::triangle_mesh_slicer` to close open contours at the slice plane.

`PostPass::GCodeEmit` (host-built-in): reads seven precision keys from `ResolvedConfig` (see `docs/02_ir_schemas.md` "Polyline simplification and precision" subsection). Key routing:
- `gcode_resolution`, `infill_resolution`, `support_resolution`, `min_segment_length`, `gcode_xy_decimals` — consumed inside `DefaultGCodeEmitter` during G-code serialization.
- `perimeter_arc_tolerance` — read by perimeter modules at module-load time and threaded into every `slicer_core::polygon_ops::offset(...)` call.
- `slice_closing_radius` — consumed by `slicer_core::triangle_mesh_slicer` at the host-built-in `PrePass::Slice` stage (see above).

#### Layer::PaintRegionAnnotation Stage (packet 64)

`Layer::PaintRegionAnnotation` sits first in the per-layer stage order, ahead of `Layer::SlicePostProcess`. The host handler `run_paint_annotation` (`crates/slicer-runtime/src/layer_executor.rs`) is a **no-op stub** since packet 95: `PrePass::PaintSegmentation` writes `segment_annotations` (and per-variant geometry) directly into the committed `SliceIR` during prepass. The stage boundary is retained for plan wiring; any WASM module claiming `Layer::PaintRegionAnnotation` in its manifest runs instead of the host built-in, providing a full override contract. When no module claims the stage, the host built-in (no-op) handles it.

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
error (`SlicePostProcessPaintAnnotationError::DeterministicConflict`). This is a
correctness improvement over the pre-Packet-64 path where the same
   conflict failed per-layer at query time.

### Automatic preparation lifecycle (Normative — TASK-574 row 5)

The private pipeline core closes preparation after **all final committed
PrePass products** (slices, region map, support plan and perimeter products)
and global-layer promotion, before ending the PrePass bracket or admitting
any `LayerArena`, Layer stage or anchored consumer. The row-01 whole-print
selection index is built once per print from those final products. Eligible
owners prepare exactly once in stable bytewise identity order: module,
version, stage, artifact. An all-Ready guard precedes every dependent ordinary
or `Layer::AnchoredEvents` consumer. Only row-03's atomic transaction finish
and `current` establish Ready; successful empty output is Ready-empty.

Participation requires `invoke && !targets.is_empty()` on `selection_for`
entries. A declared owner with no selected targets emits `NoWork` without
creating a plan owner or calling preparation. At the ordinary boundary,
prepared calls resolve their selection against the **actual available slice
views**: identity-only empty regions are not prepared work. Such no-work calls
skip before readiness checks, reads or Consumption observations. Plain
`invoke=true` empty-target calls remain unchanged; legacy plain modules never
acquire prepared behavior.

Required preparation failure aborts through `PipelineError::Preparation`,
preserving the original typed `ModuleError` fields, including `fatal: false`.
This is a required-dependency abort, not a rewrite of the general ordinary
non-fatal policy or its documented host limitation. An original module error
or trap takes precedence over writer poison. Poisoned staging publishes
nothing, never Ready, and no selected consumer executes after that failure.

Cancellation checkpoints occur before preparation, between owners and before
consumers. An active call is allowed to return; unfinished staging is
discarded. The recorded host cancellation decision maps to the existing
Cancelled outcome, never by matching diagnostic text. The caller joins active
calls before disposing staging and owners, after guards quiesce; row-03
`Busy` is a producer return value, not something to wait on, spin on or force
past. Disposal leaves zero retained logical bytes. Normal Layer arenas still
run to completion or their explicit error, without a between-stage barrier.

Actual selected `Layer::AnchoredEvents` runs as an ordinary Layer-module call
through `PreparedLayerStageRunner`
(`crates/slicer-runtime/src/preparation_lifecycle.rs`); its module-owned plan
reads count as consumption. Synthetic `Anchored::Event` /
`host:anchored-events` commits are builtin host work with no module dispatch
and no plan reads. Their host-closure Finish observation denotes return or
unwind separately from real module calls. The print scope remains alive
through both: owners are disposed only after all real calls and the applicable
synthetic host closure finish.

### Preparation-only runtime capture (Normative — TASK-574 row 5)

`run_slice_preparation_capture` (`crates/slicer-runtime/src/run.rs`) shares
production's full validated startup, committed PrePass and preparation prefix.
It stops before any `LayerArena`, ordinary Layer-module call or synthetic
anchored-host closure; preparation calls themselves remain part of the prefix.
It snapshots metadata only: piece names and lengths, typed selected targets,
lifecycle records and `accounting_before_disposal` from row-03's actual
production gauges, including `returned_read_bytes` observed before disposal.

Capture `retained_bytes` is the checked post-cleanup retained total (zero).
`transferred_bytes` is the checked sum of production `returned_read_bytes`,
not stored lengths, metadata sizes or requested bytes; it is zero when no
consumer reads. Retention and transfer are logical byte counts, not allocator
or process peak-memory claims (see `docs/16_slicer_report.md`).

Capture never opens, reads, copies or decodes opaque pieces. Neither capture,
observer, CLI nor DAG exports raw payloads, and the host supplies no plan
decoder. Row 06 alone adds opt-in atomic typed diagnostic snapshots through
row-03's publication hook.

Lifecycle records carry module/version/artifact identity, lifecycle phase
separately from `consumer_stage`, piece names/lengths, retained/transferred
counts and typed `PreparationFailure` on failure. Whole-print preparation and
disposal have no global layer index; actual layer-scoped calls carry their
real index. `elapsed` is `None` unless existing instrumentation is enabled.
These typed observations add no `ProgressEvent` JSONL fields or schema/version
change. Static capability or DAG metadata never fabricates Start or Ready.

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

Anchored synthesized rows are an explicit exception to duplicate-index
validation: anchored rows may share the upper anchor's `global_layer_index`.
`validate_finalization_state` must distinguish those rows from invalid
duplicate ordinary layers.


Finalization ordering guarantees:
- Modules execute sequentially in stage order.
- Module B always sees the fully committed output of module A.
- If two modules insert at the same position, order is deterministic by module execution order.

Top-surface ironing runs at `Layer::Infill`, gated on `top_solid_fill` and ordered after infill via compatibility `requires` (packet 38-rev1's `PostPass::LayerFinalization` placement was superseded). The module emits `Ironing`-role entities; the role's `default_priority()` is 6000 (Ironing prints last on its layer).

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
// Pseudocode — illustrates the Error Handling Policy below; `LayerErrorAction`
// is not a source type. See the normative behavior list after this snippet.
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
- An absent compiled component is always fatal: load rejects it via `LiveModuleLoadError::Component`, and dispatch rejects it via the phase-appropriate fatal error in `dispatch.rs`. Placeholder (≤8-byte) stubs likewise fail live loading with a structured diagnostic naming the module (see `placeholder_wasm_is_skipped_with_structured_warning_diagnostic` in `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs`); a slice never proceeds without the module's component.

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

    // Owning variant (perf-vs-orca t90). Behaviour is identical; an
    // implementation may consume the vector instead of copying it, and the
    // default implementation forwards to `emit_gcode`. `DefaultGCodeEmitter`
    // overrides it so the postpass `Vec<LayerCollectionIR>` is the emitter's
    // working copy and is dropped exactly once.
    fn emit_gcode_owned(&self, layers: Vec<LayerCollectionIR>)
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
`crates/slicer-runtime/src/run.rs` / `crates/slicer-runtime/src/postpass.rs` and wraps `DefaultGCodeEmitter::emit_gcode_owned`,
converting `GCodeEmitError` → `PostpassError` at the boundary via a free
function (not a `From` impl — orphan rule prevents that). This
preserves ADR-0001's in-stage-commit pattern without introducing a
`slicer-gcode` → `slicer-runtime` circular dependency. The owning call is
taken only when no visual-debug capture sink is attached; a capturing caller
keeps the borrowed path and receives the collection in the sink.

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
  post-processors reject zero-height comments. <!-- VERIFY: the terminal-layer fallback (`height_N = height_{N-1}`) was not found in `crates/slicer-gcode/src/emit.rs`; confirm the fallback site or drop the sentence. -->

`ExtrusionRole` → `;TYPE:` label mapping (host-canonical, OrcaSlicer parity):

| `ExtrusionRole`        | `;TYPE:` label      |
|------------------------|---------------------|
| `OuterWall`            | `Outer wall`        |
| `InnerWall`            | `Inner wall`        |
| `ThinWall`             | `Inner wall`         |
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
| `GapFill`             | `Gap infill`        |
| `InternalSolidInfill` | `Internal solid infill` |
| `InternalBridgeInfill`| `Internal Bridge`   |
| `RaftInfill`          | `Support`           |
| `Custom(_)`            | `Custom`            |

`SupportBaseInterface` uses the same `Support interface` label as
`SupportInterface`; `;TYPE:` markers therefore prove interface-family
emission only, not the specific interface role. Base-interface retention must
be asserted at the IR role or represented by a distinct marker.

Modules that attempt to emit any of these strings via `Raw(text)` are accepted
(the escape hatch is intentional) but doing so duplicates the host-emitted
markers and is logged as a `MUDDIED_GCODE_PREAMBLE` warning. <!-- VERIFY: no `MUDDIED_GCODE_PREAMBLE` symbol exists in `slicer-gcode`; confirm the warning name or drop the clause. -->

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
    mesh_ir: Arc<MeshIR>,

    // Written by PrePass (each Option slot committed once), immutable during per-layer
    surface_classification: Option<Arc<SurfaceClassificationIR>>,
    layer_plan:             Option<Arc<LayerPlanIR>>,
    seam_plan:              Option<Arc<SeamPlanIR>>,
    support_plan:           Option<Arc<SupportPlanIR>>,
    region_map:             Option<Arc<RegionMapIR>>,
    slice_ir:               Option<Arc<Vec<SliceIR>>>,
    support_geometry:       Option<Arc<SupportGeometryIR>>,
    support_analysis:       Option<Arc<SupportAnalysisIR>>,

    // Written by per-layer (one slot per layer, written once, read after join)
    layer_outputs: Option<Vec<Option<LayerCollectionIR>>>,
}
```

The authoritative definition is `Blackboard` in
`crates/slicer-runtime/src/blackboard.rs`; per-layer slots are
`Option<Vec<Option<LayerCollectionIR>>>` (not a `SlotVec` type — that name does not
exist in the codebase).

---

## Full Lifecycle

```text
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
    │    ├─ PrePassSupportAnalysis       → SupportAnalysisIR                   → Blackboard
    │    ├─ PrePassSupportGeometry       → SupportGeometryIR+SupportPlanIR      → Blackboard  (guest optional)
  ├─ execute_per_layer()  [rayon::par_iter]
  │    └─ per layer (parallel):
  │         ├─ LayerPaintRegionAnnotation  (host-built-in; WASM override)
  │         ├─ LayerSlicePostProcess
  │         ├─ LayerPerimeters
  │         ├─ LayerPerimetersPostProcess
  │         ├─ LayerInfill
  │         ├─ LayerInfillPostProcess
  │         ├─ LayerSupport
  │         ├─ LayerSupportPostProcess
  │         ├─ LayerAnchoredEvents
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
       └─ write the .gcode file
```
