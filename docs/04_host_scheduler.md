# ModularSlicer — Host Scheduler

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
> - `crates/slicer-wasm-host/src/traits.rs` — runner traits (`PrepassRunner`,
>   `LayerRunner`, `FinalizationRunner`, `PostpassRunner`) and their
>   `*StageInput<'a>` borrow-struct inputs.
> - `crates/slicer-wasm-host/src/host.rs` — four co-located `bindgen!`
>   invocations (per-world WIT remap onto the canonical layer world per ADR-0002).
>
> Scheduler-no-wasmtime invariant (Packet 85): `slicer-scheduler` declares
> no dep on `slicer-wasm-host`, `slicer-runtime`, or `wasmtime`. Verify
> with `cargo tree -p slicer-scheduler --edges normal | grep wasmtime`
> (must be empty). This is what enables the ~5500 LOC of planning logic
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
    pub ir_reads:              Vec<IrAccessPath>, // from manifest [ir-access].reads
    pub ir_writes:             Vec<IrAccessPath>, // from manifest [ir-access].writes
    pub claims:                Vec<ClaimId>,
    pub requires_claims:       Vec<ClaimId>,      // from manifest [claims].requires
    pub incompatible_with:     Vec<ModuleGlob>,   // from manifest [compatibility].incompatible-with
    pub requires_modules:      Vec<ModuleId>,     // from manifest [compatibility].requires
    pub config_schema:         ModuleConfigSchema,
    pub overridable_per_region:Vec<ConfigKey>,    // from manifest [config.overridable-per-region].keys
    pub overridable_per_layer: Vec<ConfigKey>,    // from manifest [config.overridable-per-layer].keys
    pub layer_parallel_safe:   bool,
    pub wasm_path:             PathBuf,
    pub instance:              Option<WasmInstance>,  // populated in Phase 4
}
```

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
   (`LoadErrorKind::CoreSemanticPriorityMismatch`).

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
fn validate_stage_id(module: &LoadedModule) -> Result<(), SchedulerError> {
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

```rust
pub static STAGE_ORDER: &[StageId] = &[
    StageId::PrePassMeshAnalysis,
    StageId::PrePassLayerPlanning,
    StageId::PrePassOverhangAnnotation,  // introduced P106; populates SurfaceClassificationIR.overhang_quartile_polygons
    StageId::PrePassSeamPlanning,        // optional; runs when a seam-planner module is loaded
    StageId::PrePassSupportGeometry,     // optional; runs when a support-planner module is loaded
    StageId::PrePassPaintSegmentation,
    StageId::PrePassRegionMapping,   // host-built-in, not a module stage
    StageId::LayerSlice,             // host-built-in
    StageId::LayerPaintRegionAnnotation, // host-built-in; WASM override contract — any module claiming this stage runs instead of the host
    StageId::LayerSlicePostProcess,
    StageId::LayerPerimeters,
    StageId::LayerPerimetersPostProcess,
    StageId::LayerInfill,
    StageId::LayerInfillPostProcess,
    StageId::LayerSupport,
    StageId::LayerSupportPostProcess,
    StageId::LayerPathOptimization,
    // PathOptimization note (packet 33): nearest-neighbour entity ordering is
    // owned entirely by `path-optimization-default`. The host carries no
    // entity-ordering fallback. When no module claims `path-optimization` on a
    // layer, `LayerCollectionIR.ordered_entities` retains the order produced by
    // upstream per-layer stages (no reorder). Packet 18 is marked superseded.
    // ── rayon join happens here ──────────────────────────────────────────
    // PostPass tier — all stages below are sequential, whole-print.
    // Full Vec<LayerCollectionIR> visible. Never parallelized.
    StageId::PostPassLayerFinalization,
    StageId::PostPassGCodeEmit,      // host-built-in
    StageId::PostPassGCodePostProcess,
    StageId::PostPassTextPostProcess,
];
```

`PrePass::OverhangAnnotation` — populates `SurfaceClassificationIR.overhang_quartile_polygons` via mesh cross-section analysis. Reads `MeshIR` + per-layer slices from `PrePass::LayerPlanning`. Owned by `core-modules/overhang-annotator-default` (introduced in P106).

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
from `layer_executor.rs:362`). Filter cost is `O(|regions| × |S|)` per
dispatch decision; the `region_split_semantics` HashSet keeps the
inner check at O(1).

### Host-Filtered Dispatch (Normative — Packet 92)

In addition to the per-layer region-split filter above, the host applies a
**paint-transparent dispatch gate** before invoking any module. This is the
**host-filtered dispatch** contract:

- A **paint-transparent** caller (one that does not declare any paint-mutating
  or geometry-mutating stage claims) is allowed to invoke only stages that are
  read-only with respect to paint and geometry IR. The predicate
  `module_invocation_allowed_on_layer(...)` implements this gate.
- A module that declares `[[region_split]]` semantics is considered
  **paint-mutating**; paint-transparent callers cannot invoke it.
- A module that claims any geometry-mutating stage (`MeshAnalysis`,
  `LayerPlanning`, `Slice`, `ShellClassification`, `SupportGeometry`, etc.)
  is similarly blocked for paint-transparent callers.
- **Non-paint-transparent** callers (modules that declare at least one
  paint-mutating or geometry-mutating claim) are **unrestricted** — the host
  does not filter their invocations beyond the per-layer region-split check
  above.

This two-tier filter ensures that paint-transparent modules cannot silently
corrupt shared paint/geometry state: the host gate is a hard precondition,
not a module advisory.

### Universal Empty-Polygon Dispatch Guard

For **all** PrePass stages and all GCode-emitting stages, the host applies a
universal guard before dispatching a module: if the module's input polygons
are empty (zero contours, zero area), dispatch is skipped entirely. This
applies regardless of the caller's paint-transparency status and regardless of
the stage's position in the pipeline. The rationale: dispatching a module
with empty inputs is always a no-op (there is no geometry to process, slice,
or annotate), so skipping it avoids wasted WASM-call overhead and keeps the
instrumentation log free of phantom empty-stage entries.

---

## Phase 3 — DAG Validation

All validation errors are structured and collected before any are surfaced to the user.

```rust
pub enum SchedulerError {
    UnknownStage {
        module: ModuleId,
        declared_stage: StageId,
    },
    ClaimConflict {
        claim: ClaimId, module_a: ModuleId, module_b: ModuleId, scope: ConflictScope,
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
        module: ModuleId, field: IrAccessPath, suggestion: Option<String>,
    },
    IrVersionIncompatible {
        module: ModuleId, ir_type: IrType, required: SemVer, available: SemVer,
    },
    StageMismatch {
        module: ModuleId, declared_stage: StageId, exported_fn: String,
    },
    /// Two modules in the same stage both write the same IR field with no
    /// read-after-write dependency between them. The result would depend on
    /// DAG traversal order, which is an implementation detail.
    ///
    /// Resolution options for module authors:
    ///   A) Declare one module incompatible with the other in its manifest.
    ///   B) Have module B declare it reads the field that module A writes,
    ///      establishing an explicit ordering (B transforms A's output).
    ///   C) Use a claim so only one can be active per region at a time.
    WriteConflict {
        field:    IrAccessPath,
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
        module: ModuleId, field: IrAccessPath,
    },
}
```

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
fn check_write_conflicts(
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

During region mapping, modifier volume `config_delta.fields` from every `modifier_volume` attached to a region's parent `ObjectMesh` are stamped into `RegionPlan.config.extensions` via `overlay_resolved` (priority-ascending, last-writer-wins), with `support_enforcer` and `support_blocker` subtypes filtered out for OrcaSlicer parity (`PrintApply.cpp:590-594`). Scope is global per object — the only `ModifierScope` variant in use is `AllFeatures`; bbox / polygon-level overlap is a future refinement when partial-volume scopes are introduced.

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
   `RegionMappingError::DeterministicConflict` (Packet 93 guard, the
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

- Now honours `support_enabled` (false → planner is invoked but emits
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
- Default cap: `1_000` entries.
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
    pub postpass_stages:  Vec<CompiledStage>,
    pub global_layers:    Arc<Vec<GlobalLayer>>,
    pub region_plans:     Arc<HashMap<RegionKey, RegionPlan>>,
}

pub struct CompiledStage {
    pub stage_id: StageId,
    pub modules:  Vec<CompiledModule>,  // topologically sorted, iterate directly
}

pub struct CompiledModule {
    pub module_id:     ModuleId,
    pub instance_pool: Arc<WasmInstancePool>,
    pub ir_read_mask:  IrAccessMask,
    pub ir_write_mask: IrAccessMask,
    pub config_view:   Arc<ConfigView>,
}

// parallel-safe: N instances (N = rayon thread count)
// sequential:    1 instance  (serializes access)
pub struct WasmInstancePool {
    pub instances: Vec<Mutex<WasmInstance>>,
}
```

### Runner-Trait Input Borrow Structs (Normative — Packet 83)

Runner trait signatures (`PrepassRunner::run_prepass`,
`LayerRunner::run_layer`, `FinalizationRunner::run_finalization`,
`PostpassRunner::run_postpass`) accept IR-typed `*StageInput<'a>`
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
`layer_executor.rs::run_paint_annotation`, after `arena.take_slice()`
returns the layer's `SliceIR` and BEFORE the paint annotation loop
begins. This insertion point is binding (see proposed ADR-0012):

- Earlier designs put the subtract in a prepass phase-0 built-in or in
  `pipeline.rs`; both were infeasible because `Vec<SliceIR>` is
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
| `PrePass::OverhangAnnotation`      | `MeshIR`, `LayerPlanIR`; writes `overhang_quartile_polygons` into `SurfaceClassificationIR` |
| `PrePass::SeamPlanning`            | `LayerPlan`                                                               |
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

`Layer::PaintRegionAnnotation` sits between `Layer::Slice` and `Layer::SlicePostProcess` in the per-layer stage order. The host handler `execute_slice_postprocess_paint_annotation()` annotates slice-region entities with paint data from `PaintRegionIR`. Any WASM module claiming `Layer::PaintRegionAnnotation` in its manifest runs instead of the host built-in, providing a full override contract. When no module claims the stage, the host built-in handles it.

The annotation loop processes contour points in **parallel chunks of
32** (`par_chunks(32)`, rayon). Results are byte-identical to serial
execution — per-point paint queries are order-independent, so the
chunked schedule is purely a wall-clock optimisation. Thread-local
warnings and `DeterministicConflict` detection flags are merged at
the end of the layer; cross-thread state contention is zero. Observed
multi-thread utilisation is exposed via report wall-clock timing
(non-gating).

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
/// `Arc<SlotVec<LayerCollectionIR>>` in the Blackboard has been drained
/// into this Vec. The Blackboard no longer holds any reference to these
/// values — there is no concurrent access, and no RwLock is needed.
///
/// After this function returns, the Vec is passed as `&[LayerCollectionIR]`
/// to `execute_postpass`. It is never re-entered into the Blackboard.
fn execute_layer_finalization(
    plan:       &ExecutionPlan,
    layer_irs:  &mut Vec<LayerCollectionIR>,  // exclusively owned, single-threaded
    blackboard: &Blackboard,                   // read-only; mesh, layer plan, etc.

) -> Result<(), SlicerError> {
    // Always sequential — pool size 1 for all finalization modules.
    for module in &plan.finalization_stage.modules {
        let instance = module.instance_pool.acquire();

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
    // Per-layer bump allocator — freed entirely when this function returns.
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

See progress event schema: `./docs/09_progress_events.md`.

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
`run.rs` / `postpass.rs` and wraps `DefaultGCodeEmitter::emit_gcode`,
converting `GCodeEmitError` → `PostpassError` at the boundary via a free
function (not a `From` impl — orphan rule prevents that). This
preserves ADR-0001's in-stage-commit pattern without introducing a
`slicer-gcode` → `slicer-runtime` circular dependency.

#### Overhang Classifier Prepass (Normative — Packet 57)

`DefaultGCodeEmitter::emit_gcode` runs an **embedded prepass** that
invokes `slicer_core::algos::overhang_classifier::classify_layers`
once per print (after cloning the layer set, before per-layer
emission). The classifier walks the layer set and stamps
`Point3WithWidth.overhang_quartile` (`1..=4`) on every wall-family
extrusion point against the previous layer's support polygons. The
emission path then uses `resolve_feedrate(role, speed_factor)` to
dispatch the matching `overhang_*_4_speed` config key per wall point.

Why inside `emit_gcode`: this single call site covers both pipeline
arms (`pnp_cli slice` and the WASM dispatch path) without separate
plumbing in `pipeline.rs`. The classifier short-circuits when all four
`overhang_*_4_speed` keys are zero (legacy-equivalent mode produces
byte-identical output to pre-Packet-57).

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
| `Skirt`                | `Skirt/Brim`        |
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

Host-side tool-grouping in `layer_executor.rs` is intentionally absent;
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
    pub layer_outputs: Arc<SlotVec<LayerCollectionIR>>,
}
```

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
  ├─ call on-print-start on all modules
  └─ freeze ExecutionPlan

slice command
  ├─ load model → MeshIR (paint normalized at load via split_triangle_strokes)
  ├─ execute_prepass()
    │    ├─ PrePassMeshAnalysis          → SurfaceClassificationIR   → Blackboard
    │    ├─ PrePassLayerPlanning         → LayerPlanIR               → Blackboard
    │    ├─ PrePassOverhangAnnotation    → SurfaceClassificationIR (overhang_quartile_polygons) → Blackboard  (P106)
    │    ├─ PrePassSeamPlanning          → SeamPlanIR                → Blackboard  (optional)
    │    ├─ PrePassSupportGeometry  → SupportGeometryIR+SupportPlanIR → Blackboard  (optional)
        │    ├─ PrePassPaintSegmentation→ PaintRegionIR             → Blackboard
    │    └─ PrePassRegionMapping    → RegionMapIR               → Blackboard
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
  │              └─ writes complete LayerCollectionIR into Blackboard SlotVec[layer_idx]
  │                 (written once per slot; no mutex required)
  ├─ rayon join
  │    └─ drain Blackboard SlotVec → plain Vec<LayerCollectionIR>
  │       (Blackboard no longer holds these values after this point)
  ├─ execute_layer_finalization()    [single-threaded, owns Vec<LayerCollectionIR>]
  │    └─ PostPassLayerFinalization modules may append or insert synthetic layers
  └─ execute_postpass()
       ├─ PostPassGCodeEmit         (host-built-in serializer)
       ├─ PostPassGCodePostProcess  (optional modules)
       └─ PostPassTextPostProcess   (optional, last resort)
       └─ write .gcode / .bgcode file
```
