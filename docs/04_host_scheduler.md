# ModularSlicer — Host Scheduler

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

| Manifest key | Runtime field |
|---|---|
| `[ir-access].reads` | `ir_reads` |
| `[ir-access].writes` | `ir_writes` |
| `[claims].holds` | `claims` |
| `[claims].requires` | `requires_claims` |
| `[compatibility].incompatible-with` | `incompatible_with` |
| `[compatibility].requires` | `requires_modules` |
| `[config.overridable-per-region].keys` | `overridable_per_region` |
| `[config.overridable-per-layer].keys` | `overridable_per_layer` |

The manifest naming is canonical for author-facing docs and examples. Runtime field names are internal and must not appear in user-facing manifest examples.

Ingestion scans all module search paths and deserializes every `.toml`. TOML schema errors produce a structured `LoadError` with file path and field name. No module is silently skipped.

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
    StageId::PrePassMeshSegmentation,
    StageId::PrePassMeshAnalysis,
    StageId::PrePassLayerPlanning,
    StageId::PrePassPaintSegmentation,
    StageId::PrePassRegionMapping,   // host-built-in, not a module stage
    StageId::LayerSlice,             // host-built-in
    StageId::LayerSlicePostProcess,
    StageId::LayerPerimeters,
    StageId::LayerPerimetersPostProcess,
    StageId::LayerInfill,
    StageId::LayerInfillPostProcess,
    StageId::LayerSupport,
    StageId::LayerSupportPostProcess,
    StageId::LayerPathOptimization,
    // ── rayon join happens here ──────────────────────────────────────────
    // PostPass tier — all stages below are sequential, whole-print.
    // Full Vec<LayerCollectionIR> visible. Never parallelized.
    StageId::PostPassLayerFinalization,
    StageId::PostPassGCodeEmit,      // host-built-in
    StageId::PostPassGCodePostProcess,
    StageId::PostPassTextPostProcess,
];
```

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
2. **Global claim conflicts** — two enabled modules hold the same claim globally
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

| | Claim Conflict | Write Conflict |
|---|---|---|
| **What it detects** | Two modules both intend to be the primary generator of a feature | Two modules both write the same IR field with no ordering between them |
| **Granularity** | One claim name per feature category (coarse) | Per IR access path (fine) |
| **Caught at** | Startup, validation pass 1–2 | Startup, validation pass 6b |
| **Typical cause** | User enables two infill modules simultaneously | Developer adds a new PostProcess module that overwrites a field another module already modifies |
| **Resolution** | Region overrides; disable one module | Declare `incompatible-with`, OR have one module read the field it will overwrite (establishing ordering), OR use a claim |
| **Runtime fallback** | None — fatal startup error | None — fatal startup error |

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

`PrePass::RegionMapping` is host-built-in and precomputes per-region execution context so Tier 2 has no config or claim resolution overhead.

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

### PrePass Execution (sequential)

```rust
pub fn execute_prepass(
    plan: &ExecutionPlan,
    blackboard: &mut Blackboard,
) -> Result<(), SlicerError> {
    for stage in &plan.prepass_stages {
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
  ├─ load model → MeshIR
  ├─ execute_prepass()
    │    ├─ PrePassMeshSegmentation → MeshIR (normalized paint) → Blackboard
    │    ├─ PrePassMeshAnalysis     → SurfaceClassificationIR   → Blackboard
    │    ├─ PrePassLayerPlanning    → LayerPlanIR               → Blackboard
        │    ├─ PrePassPaintSegmentation→ PaintRegionIR             → Blackboard
    │    └─ PrePassRegionMapping    → RegionMapIR               → Blackboard
  ├─ execute_per_layer()  [rayon::par_iter]
  │    └─ per layer (parallel):
  │         ├─ LayerSlice              (host-built-in)
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
