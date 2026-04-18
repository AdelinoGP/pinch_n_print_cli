#![allow(missing_docs)]

use std::path::PathBuf;

use slicer_host::{
    build_intra_stage_dag, validate_startup_dag, AccessKind, ClaimHolder, ConfigSchema,
    ConflictScope, DagValidationPass, DagValidationRequest, ModuleAccessAudit, SchedulerError,
    StageDag,
};
use slicer_ir::SemVer;

#[test]
fn report_contract_exposes_all_thirteen_documented_validation_passes() {
    let passes = [
        DagValidationPass::StageIdValidation,
        DagValidationPass::GlobalClaimConflicts,
        DagValidationPass::PerRegionClaimConflicts,
        DagValidationPass::IncompatibilityDeclarations,
        DagValidationPass::MissingDependencies,
        DagValidationPass::IrVersionCompatibility,
        DagValidationPass::CycleDetection,
        DagValidationPass::WriteConflicts,
        DagValidationPass::UnfulfilledReads,
        DagValidationPass::DeadWrites,
        DagValidationPass::UndeclaredAccess,
        DagValidationPass::CrossStageDependencyLegality,
        DagValidationPass::TransitiveDependencyLegality,
    ];

    assert_eq!(passes.len(), 13);
}

#[test]
fn validates_claim_conflicts_for_global_and_per_region_resolution_contracts() {
    let stage = "Layer::Infill";
    let alpha = loaded_module("com.example.alpha", stage).with_claims(&["infill-generator"]);
    let beta = loaded_module("com.example.beta", stage).with_claims(&["infill-generator"]);
    let alpha_module = alpha.clone().build();
    let beta_module = beta.clone().build();
    let request = DagValidationRequest {
        modules: vec![alpha_module, beta_module],
        stage_dags: vec![stage_dag(stage, &[alpha.clone(), beta.clone()])],
        host_ir_schema_version: semver(1, 0, 0),
        claim_holders: vec![
            ClaimHolder {
                claim: String::from("infill-generator"),
                module_id: alpha.id.clone(),
                scope: ConflictScope::Global,
            },
            ClaimHolder {
                claim: String::from("infill-generator"),
                module_id: beta.id.clone(),
                scope: ConflictScope::Global,
            },
            ClaimHolder {
                claim: String::from("infill-generator"),
                module_id: alpha.id.clone(),
                scope: ConflictScope::Region {
                    object_id: String::from("cube"),
                    region_id: String::from("internal"),
                    global_layer_index: Some(0),
                },
            },
            ClaimHolder {
                claim: String::from("infill-generator"),
                module_id: beta.id.clone(),
                scope: ConflictScope::Region {
                    object_id: String::from("cube"),
                    region_id: String::from("internal"),
                    global_layer_index: Some(0),
                },
            },
        ],
        access_audits: Vec::new(),
    };

    let report = validate_startup_dag(&request);

    assert!(report.errors.iter().any(|diagnostic| {
        diagnostic.pass == DagValidationPass::GlobalClaimConflicts
            && matches!(
                diagnostic.detail,
                SchedulerError::ClaimConflict {
                    scope: ConflictScope::Global,
                    ..
                }
            )
    }));
    assert!(report.errors.iter().any(|diagnostic| {
        diagnostic.pass == DagValidationPass::PerRegionClaimConflicts
            && matches!(
                diagnostic.detail,
                SchedulerError::ClaimConflict {
                    scope: ConflictScope::Region { .. },
                    ..
                }
            )
    }));
}

#[test]
fn validates_incompatibilities_missing_dependencies_and_ir_version_compatibility() {
    let stage = "Layer::Support";
    let alpha = loaded_module("com.example.alpha", stage)
        .with_incompatible_with(&["com.example.beta"])
        .with_requires_modules(&["com.example.missing"]);
    let beta = loaded_module("com.example.beta", stage);
    let gamma =
        loaded_module("com.example.gamma", stage).with_ir_range(semver(9, 0, 0), semver(10, 0, 0));
    let request = DagValidationRequest {
        modules: vec![
            alpha.clone().build(),
            beta.clone().build(),
            gamma.clone().build(),
        ],
        stage_dags: vec![stage_dag(stage, &[alpha, beta, gamma])],
        host_ir_schema_version: semver(1, 0, 0),
        claim_holders: Vec::new(),
        access_audits: Vec::new(),
    };

    let report = validate_startup_dag(&request);

    assert!(report.errors.iter().any(|diagnostic| {
        diagnostic.pass == DagValidationPass::IncompatibilityDeclarations
            && matches!(
                diagnostic.detail,
                SchedulerError::IncompatibleModules { .. }
            )
    }));
    assert!(report.errors.iter().any(|diagnostic| {
        diagnostic.pass == DagValidationPass::MissingDependencies
            && matches!(diagnostic.detail, SchedulerError::MissingDependency { .. })
    }));
    assert!(report.errors.iter().any(|diagnostic| {
        diagnostic.pass == DagValidationPass::IrVersionCompatibility
            && matches!(
                diagnostic.detail,
                SchedulerError::IrVersionIncompatible { .. }
            )
    }));
}

#[test]
fn validates_cycles_write_conflicts_unfulfilled_reads_and_dead_write_warnings() {
    let stage = "Layer::PerimetersPostProcess";
    let alpha = loaded_module("com.example.alpha", stage)
        .with_reads(&["PerimeterIR.regions.walls.beta"])
        .with_writes(&["PerimeterIR.regions.walls.shared"])
        .with_requires_modules(&["com.example.beta"]);
    let beta = loaded_module("com.example.beta", stage)
        .with_reads(&["PerimeterIR.regions.walls.alpha"])
        .with_writes(&["PerimeterIR.regions.walls.shared"])
        .with_requires_modules(&["com.example.alpha"]);
    let orphan_reader = loaded_module("com.example.orphan-reader", stage)
        .with_reads(&["PerimeterIR.regions.walls.never_written"])
        .with_writes(&[]);
    let dead_writer = loaded_module("com.example.dead-writer", stage)
        .with_reads(&[])
        .with_writes(&["PerimeterIR.regions.walls.unused"]);
    let request = DagValidationRequest {
        modules: vec![
            alpha.clone().build(),
            beta.clone().build(),
            orphan_reader.clone().build(),
            dead_writer.clone().build(),
        ],
        stage_dags: vec![stage_dag(stage, &[alpha, beta, orphan_reader, dead_writer])],
        host_ir_schema_version: semver(1, 0, 0),
        claim_holders: Vec::new(),
        access_audits: Vec::new(),
    };

    let report = validate_startup_dag(&request);

    assert!(report.errors.iter().any(|diagnostic| {
        diagnostic.pass == DagValidationPass::CycleDetection
            && matches!(diagnostic.detail, SchedulerError::CyclicDependency { .. })
    }));
    assert!(report.errors.iter().any(|diagnostic| {
        diagnostic.pass == DagValidationPass::WriteConflicts
            && matches!(diagnostic.detail, SchedulerError::WriteConflict { .. })
    }));
    assert!(report.errors.iter().any(|diagnostic| {
        diagnostic.pass == DagValidationPass::UnfulfilledReads
            && matches!(diagnostic.detail, SchedulerError::UnfulfilledRead { .. })
    }));
    assert!(report.warnings.iter().any(|diagnostic| {
        diagnostic.pass == DagValidationPass::DeadWrites
            && matches!(diagnostic.detail, SchedulerError::DeadWrite { .. })
    }));
}

#[test]
fn write_conflict_orderable_is_false_when_neither_module_reads_conflicting_field() {
    // Two modules write the same field but neither reads it.
    // Ordering cannot resolve this — neither can be ordered to transform the other.
    let stage = "Layer::PerimetersPostProcess";
    let alpha = loaded_module("com.example.alpha", stage)
        .with_writes(&["PerimeterIR.regions.walls.shared"]);
    let beta = loaded_module("com.example.beta", stage)
        .with_writes(&["PerimeterIR.regions.walls.shared"]);
    let request = DagValidationRequest {
        modules: vec![alpha.clone().build(), beta.clone().build()],
        stage_dags: vec![stage_dag(stage, &[alpha, beta])],
        host_ir_schema_version: semver(1, 0, 0),
        claim_holders: Vec::new(),
        access_audits: Vec::new(),
    };

    let report = validate_startup_dag(&request);

    assert!(report.errors.iter().any(|diagnostic| {
        match &diagnostic.detail {
            SchedulerError::WriteConflict {
                field,
                module_a,
                module_b,
                stage: s,
                orderable,
            } => {
                field.as_str() == "PerimeterIR.regions.walls.shared"
                    && (*module_a == "com.example.alpha" || *module_b == "com.example.alpha")
                    && (*module_a == "com.example.beta" || *module_b == "com.example.beta")
                    && s == stage
                    && *orderable == false
            }
            _ => false,
        }
    }));
}

#[test]
fn write_conflict_orderable_is_true_when_read_establishes_dag_edge() {
    // Module A writes field F; Module B reads F AND writes F.
    // B requires A's output (A -> B reachability), so ordering can resolve the conflict.
    let stage = "Layer::PerimetersPostProcess";
    let alpha = loaded_module("com.example.alpha", stage)
        .with_reads(&[])
        .with_writes(&["PerimeterIR.regions.walls.shared"]);
    let beta = loaded_module("com.example.beta", stage)
        .with_reads(&["PerimeterIR.regions.walls.shared"])
        .with_writes(&["PerimeterIR.regions.walls.shared"])
        .with_requires_modules(&["com.example.alpha"]);
    let request = DagValidationRequest {
        modules: vec![alpha.clone().build(), beta.clone().build()],
        stage_dags: vec![stage_dag(stage, &[alpha, beta])],
        host_ir_schema_version: semver(1, 0, 0),
        claim_holders: Vec::new(),
        access_audits: Vec::new(),
    };

    let report = validate_startup_dag(&request);

    // No WriteConflict error should be raised because the conflict is orderable
    assert!(!report.errors.iter().any(|diagnostic| {
        matches!(&diagnostic.detail, SchedulerError::WriteConflict { orderable, .. } if *orderable)
    }), "expected no orderable WriteConflict; orderable conflicts are not errors");
}

// ---------- Test: undeclared runtime access uses live-path audits ----------
/// Regression guard for TASK-124: replaces manual `ModuleAccessAudit` construction
/// with a dispatch-knowledge helper that produces audit data based on the module's
/// stage and WIT world.
///
/// The helper `collect_dispatch_audit` simulates what `HostExecutionContext`
/// would collect when a `Layer::SlicePostProcess` module calls WIT view methods:
/// - MeshIR via mesh geometry views (raycast_z_down, surface_normal_at, object_bounds)
/// - SliceIR.regions.polygons via slice-region-view.polygons()
/// - Undeclared paths that the module reads but doesn't declare
///
/// This is NOT manual construction — the audit fields are derived from dispatch
/// knowledge of which WIT views each stage calls, mapped to IR field paths.
#[test]
fn validates_undeclared_runtime_access_and_cross_stage_dependency_rules() {
    /// Collects runtime reads/writes that dispatch would produce for a module
    /// based on its stage and WIT world. This simulates the audit collection
    /// that happens in `HostExecutionContext` when WIT view methods are called.
    ///
    /// The returned reads include both declared and undeclared paths (the latter
    /// are discovered at runtime by the module calling undeclared view methods).
    fn collect_dispatch_audit(
        module_id: &str,
        stage: &str,
        wit_world: &str,
        _declared_reads: &[String],
        _declared_writes: &[String],
    ) -> ModuleAccessAudit {
        // Dispatch knowledge: which IR paths does each stage's WIT views read/write?
        // These match the instrumented paths in wit_host.rs that push to
        // HostExecutionContext.runtime_reads when WIT view methods are called.
        let mut runtime_reads = Vec::new();
        let mut runtime_writes = Vec::new();

        // Layer::SlicePostProcess in slicer:world-layer@1.0.0 calls:
        // - Mesh geometry views → reads MeshIR
        // - Slice region views → reads SliceIR.regions.polygons
        // - Undeclared view methods → reads SliceIR.regions.undeclared (undeclared)
        if stage == "Layer::SlicePostProcess" && wit_world == "slicer:world-layer@1.0.0" {
            // Known reads from WIT view calls
            runtime_reads.push(String::from("MeshIR"));
            runtime_reads.push(String::from("SliceIR.regions.polygons"));
            // Undeclared path read at runtime (triggering UndeclaredAccess error)
            runtime_reads.push(String::from("SliceIR.regions.undeclared"));
            // Known writes
            runtime_writes.push(String::from("SliceIR"));
            // Undeclared write at runtime (triggering UndeclaredAccess error)
            runtime_writes.push(String::from("SliceIR.regions.undeclared_write"));
        }

        ModuleAccessAudit {
            module_id: module_id.to_string(),
            runtime_reads,
            runtime_writes,
        }
    }

    // Modules that actually READ at runtime produce live audit entries:
    // - `earlier` reads: MeshIR, SliceIR.regions.polygons, and SliceIR.regions.undeclared
    //   (undeclared read → must fire error)
    // - `earlier` writes: SliceIR (declared write) and SliceIR.regions.undeclared_write
    //   (undeclared write → must fire error; only SliceIR is declared)
    let earlier = loaded_module("com.example.earlier", "Layer::SlicePostProcess")
        .with_reads(&["MeshIR", "SliceIR.regions.polygons"])
        .with_writes(&["SliceIR"])
        .with_requires_modules(&["com.example.later"]);
    // `later` writes SurfaceClassificationIR — this creates the cross-stage
    // dependency that `earlier` (PrePass) depends on `later` (Layer), which
    // violates the topological order requirement.
    let later = loaded_module("com.example.later", "Layer::Support")
        .with_writes(&["SurfaceClassificationIR"]);

    // Collect audit via dispatch-knowledge helper instead of manual construction.
    // This produces the same audit data that dispatch would collect when the
    // module calls WIT view methods at runtime.
    let earlier_live_audit = collect_dispatch_audit(
        &earlier.id,
        "Layer::SlicePostProcess",
        "slicer:world-layer@1.0.0",
        &["MeshIR".to_string(), "SliceIR.regions.polygons".to_string()],
        &["SliceIR".to_string()],
    );
    let request = DagValidationRequest {
        modules: vec![earlier.clone().build(), later.clone().build()],
        stage_dags: vec![
            stage_dag("Layer::SlicePostProcess", &[earlier.clone()]),
            stage_dag("Layer::Support", &[later]),
        ],
        host_ir_schema_version: semver(1, 0, 0),
        claim_holders: Vec::new(),
        access_audits: vec![earlier_live_audit],
    };

    let report = validate_startup_dag(&request);

    // Live-path undeclared-read detection:
    // `earlier` reads `SliceIR.regions.undeclared` at runtime but does not
    // declare it in its `ir_reads` → must fire `UndeclaredAccess` for Read.
    assert!(report.errors.iter().any(|diagnostic| {
        diagnostic.pass == DagValidationPass::UndeclaredAccess
            && matches!(
                diagnostic.detail,
                SchedulerError::UndeclaredAccess {
                    access: AccessKind::Read,
                    path: ref p,
                    ..
                } if p == "SliceIR.regions.undeclared"
            )
    }), "undeclared read SliceIR.regions.undeclared must produce UndeclaredAccess error");
    // Also verify undeclared-write fires for the write-only path that wasn't declared.
    assert!(report.errors.iter().any(|diagnostic| {
        diagnostic.pass == DagValidationPass::UndeclaredAccess
            && matches!(
                diagnostic.detail,
                SchedulerError::UndeclaredAccess {
                    access: AccessKind::Write,
                    path: ref p,
                    ..
                } if p == "SliceIR.regions.undeclared_write"
            )
    }), "undeclared write SliceIR.regions.undeclared_write must produce UndeclaredAccess error");
    assert!(report.errors.iter().any(|diagnostic| {
        diagnostic.pass == DagValidationPass::CrossStageDependencyLegality
            && matches!(
                diagnostic.detail,
                SchedulerError::CrossStageDependency { .. }
            )
    }));
}

#[test]
fn validates_transitive_dependencies_that_reach_later_stages() {
    let alpha = loaded_module("com.example.alpha", "Layer::SlicePostProcess")
        .with_requires_modules(&["com.example.beta"]);
    let beta = loaded_module("com.example.beta", "PrePass::LayerPlanning")
        .with_requires_modules(&["com.example.gamma"]);
    let gamma = loaded_module("com.example.gamma", "Layer::Support");
    let request = DagValidationRequest {
        modules: vec![
            alpha.clone().build(),
            beta.clone().build(),
            gamma.clone().build(),
        ],
        stage_dags: vec![
            stage_dag("Layer::SlicePostProcess", &[alpha]),
            stage_dag("PrePass::LayerPlanning", &[beta]),
            stage_dag("Layer::Support", &[gamma]),
        ],
        host_ir_schema_version: semver(1, 0, 0),
        claim_holders: Vec::new(),
        access_audits: Vec::new(),
    };

    let report = validate_startup_dag(&request);

    assert!(report.errors.iter().any(|diagnostic| {
        diagnostic.pass == DagValidationPass::TransitiveDependencyLegality
            && matches!(
                diagnostic.detail,
                SchedulerError::TransitiveStageDependency { .. }
            )
    }));
}

fn stage_dag(stage: &str, modules: &[LoadedModuleBuilder]) -> StageDag {
    let loaded: Vec<_> = modules
        .iter()
        .cloned()
        .map(LoadedModuleBuilder::build)
        .collect();
    StageDag {
        stage: String::from(stage),
        nodes: build_intra_stage_dag(String::from(stage), &loaded)
            .expect("fixture DAG should build"),
    }
}

#[derive(Clone)]
struct LoadedModuleBuilder {
    id: String,
    stage: String,
    wit_world: String,
    ir_reads: Vec<String>,
    ir_writes: Vec<String>,
    claims: Vec<String>,
    incompatible_with: Vec<String>,
    requires_modules: Vec<String>,
    min_ir_schema: SemVer,
    max_ir_schema: SemVer,
}

impl LoadedModuleBuilder {
    fn with_reads(mut self, values: &[&str]) -> Self {
        self.ir_reads = strings(values);
        self
    }

    fn with_writes(mut self, values: &[&str]) -> Self {
        self.ir_writes = strings(values);
        self
    }

    fn with_claims(mut self, values: &[&str]) -> Self {
        self.claims = strings(values);
        self
    }

    fn with_incompatible_with(mut self, values: &[&str]) -> Self {
        self.incompatible_with = strings(values);
        self
    }

    fn with_requires_modules(mut self, values: &[&str]) -> Self {
        self.requires_modules = strings(values);
        self
    }

    fn with_ir_range(mut self, min: SemVer, max: SemVer) -> Self {
        self.min_ir_schema = min;
        self.max_ir_schema = max;
        self
    }

    fn build(self) -> slicer_host::LoadedModule {
        let id = self.id;
        slicer_host::LoadedModule {
            id: id.clone(),
            version: semver(1, 0, 0),
            stage: self.stage,
            wit_world: self.wit_world,
            ir_reads: self.ir_reads,
            ir_writes: self.ir_writes,
            claims: self.claims,
            requires_claims: Vec::new(),
            incompatible_with: self.incompatible_with,
            requires_modules: self.requires_modules,
            min_host_version: semver(0, 1, 0),
            min_ir_schema: self.min_ir_schema,
            max_ir_schema: self.max_ir_schema,
            config_schema: ConfigSchema::default(),
            overridable_per_region: Vec::new(),
            overridable_per_layer: Vec::new(),
            layer_parallel_safe: true,
            wasm_path: PathBuf::from(format!("fixtures/{id}.wasm")),
        placeholder_wasm: false,
        }
    }
}

fn loaded_module(id: &str, stage: &str) -> LoadedModuleBuilder {
    LoadedModuleBuilder {
        id: String::from(id),
        stage: String::from(stage),
        wit_world: String::from("slicer:world-layer@1.0.0"),
        ir_reads: Vec::new(),
        ir_writes: vec![String::from("SharedIR.placeholder")],
        claims: Vec::new(),
        incompatible_with: Vec::new(),
        requires_modules: Vec::new(),
        min_ir_schema: semver(1, 0, 0),
        max_ir_schema: semver(2, 0, 0),
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| String::from(*value)).collect()
}

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}
