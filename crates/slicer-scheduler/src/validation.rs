//! Startup DAG validation contracts for the host scheduler.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use slicer_ir::{ModuleId, SemVer, StageId};

use crate::dag::ModuleNode;
use crate::manifest::{DiagnosticLevel, LoadedModule};

/// The four fill-role claim IDs registered by packet 37.
pub const FILL_CLAIM_IDS: &[&str] = &[
    "claim:top-fill",
    "claim:bottom-fill",
    "claim:bridge-fill",
    "claim:sparse-fill",
];

/// Per-claim configured holders — one module ID per fill-role claim.
///
/// Built from `ResolvedConfig.{top,bottom,bridge,sparse}_fill_holder` and
/// passed to `resolve_held_claims` to filter a module's declared `[claims].holds`
/// down to the set actually in effect for the current call.
#[derive(Debug, Clone)]
pub struct FillHolders<'a> {
    /// Module ID configured to hold `claim:top-fill`.
    pub top: &'a str,
    /// Module ID configured to hold `claim:bottom-fill`.
    pub bottom: &'a str,
    /// Module ID configured to hold `claim:bridge-fill`.
    pub bridge: &'a str,
    /// Module ID configured to hold `claim:sparse-fill`.
    pub sparse: &'a str,
}

impl<'a> FillHolders<'a> {
    /// Returns the configured module ID for the given fill claim, or `None` if
    /// the claim is not one of the four fill-role claims.
    pub fn holder_for(&self, claim: &str) -> Option<&str> {
        match claim {
            "claim:top-fill" => Some(self.top),
            "claim:bottom-fill" => Some(self.bottom),
            "claim:bridge-fill" => Some(self.bridge),
            "claim:sparse-fill" => Some(self.sparse),
            _ => None,
        }
    }
}

/// Returns true when `holder` (a config value, possibly a short name) refers
/// to the module identified by `module_id` (always a full ID with namespace).
///
/// Accepts both formats so user configs can use either form:
/// - Full module ID: `top_fill_holder = "com.core.rectilinear-infill"`
/// - Short name:     `top_fill_holder = "rectilinear-infill"`
///
/// Matching rule: exact match, OR strip the canonical built-in namespace
/// `com.core.` from `module_id` and compare against `holder`. The `com.core.`
/// prefix is reserved for built-in modules; community modules use their own
/// namespaces (`com.acme.foo`) and must be referenced by full ID in config
/// because no other short form is unambiguous. See
/// `docs/03_wit_and_manifest.md §"Holder identifier matching"`.
#[must_use]
pub fn module_id_matches_holder(module_id: &str, holder: &str) -> bool {
    if module_id == holder {
        return true;
    }
    module_id
        .strip_prefix("com.core.")
        .is_some_and(|short| short == holder)
}

/// Resolve the set of fill-role claims a module effectively holds for the
/// current call.
///
/// Inputs:
/// - `module_id` — short module name (e.g. `"rectilinear-infill"`). Matched
///   case-sensitively against `holders.holder_for(claim)`.
/// - `declared` — raw `[claims].holds` array from the module manifest. Entries
///   that are not in `FILL_CLAIM_IDS` (e.g. `"infill-generator"`) are ignored
///   for fill-role purposes; only the four fill claims gate emission.
/// - `holders` — global (or per-region) configured holder per claim.
///
/// Output: the subset of declared fill-role claims that this module actually
/// holds in this scope. An empty slice means "this module holds nothing for
/// fill roles" — the SDK's `should_emit` convention then suppresses emission.
///
/// Note: the empty-set "holds all" convention applied at the SDK boundary is
/// driven by whether `held_claims` was *populated at all*; this resolver is the
/// host-side population step, so an empty result here means a real "holds none".
pub fn resolve_held_claims(
    module_id: &str,
    declared: &[String],
    holders: &FillHolders<'_>,
) -> Vec<String> {
    declared
        .iter()
        .filter(|claim| FILL_CLAIM_IDS.contains(&claim.as_str()))
        .filter(|claim| {
            holders
                .holder_for(claim)
                .is_some_and(|h| module_id_matches_holder(module_id, h))
        })
        .cloned()
        .collect()
}

/// Structured scheduler error surfaced by DAG validation and DAG construction APIs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulerError {
    /// Placeholder variant used until validation logic is implemented.
    NotImplemented,
    /// A manifest declared an unknown scheduler stage.
    UnknownStage {
        /// Module that declared the invalid stage.
        module: ModuleId,
        /// Stage string read from the manifest.
        declared_stage: StageId,
    },
    /// Two modules claim the same capability in the same effective scope.
    ClaimConflict {
        /// Claim identifier in conflict.
        claim: String,
        /// First conflicting module.
        module_a: ModuleId,
        /// Second conflicting module.
        module_b: ModuleId,
        /// Scope in which the conflict was observed.
        scope: ConflictScope,
    },
    /// Two modules were declared incompatible.
    IncompatibleModules {
        /// Module that declared the incompatibility.
        declared_by: ModuleId,
        /// Module matched by the incompatibility rule.
        conflicting: ModuleId,
        /// Human-readable explanation for the incompatibility.
        reason: String,
    },
    /// A required module or required capability could not be satisfied.
    MissingDependency {
        /// Module with the missing dependency.
        module: ModuleId,
        /// Missing module id or capability identifier.
        requires: ModuleId,
    },
    /// One stage DAG contains a cycle.
    CyclicDependency {
        /// Module ids that remain in the cycle.
        cycle: Vec<ModuleId>,
    },
    /// A module reads an IR field with no available upstream producer.
    UnfulfilledRead {
        /// Module performing the read.
        module: ModuleId,
        /// Missing IR field path.
        field: String,
        /// Optional remediation hint.
        suggestion: Option<String>,
    },
    /// A module requires an incompatible IR schema range.
    IrVersionIncompatible {
        /// Module with the incompatible requirement.
        module: ModuleId,
        /// IR contract or type name being checked.
        ir_type: String,
        /// Minimum required version.
        required: SemVer,
        /// Host-provided version.
        available: SemVer,
    },
    /// A module declares `min-host-version` newer than the running host.
    HostVersionIncompatible {
        /// Module with the incompatible requirement.
        module: ModuleId,
        /// Minimum host version the module requires.
        required: SemVer,
        /// Host version actually running.
        available: SemVer,
    },
    /// The exported runtime entrypoint does not match the declared stage.
    StageMismatch {
        /// Module with the mismatch.
        module: ModuleId,
        /// Stage declared in the manifest.
        declared_stage: StageId,
        /// Exported function that disagreed.
        exported_fn: String,
    },
    /// Two modules in the same stage both write the same field without ordering.
    WriteConflict {
        /// IR field path written by both modules.
        field: String,
        /// First conflicting module.
        module_a: ModuleId,
        /// Second conflicting module.
        module_b: ModuleId,
        /// Stage containing the conflict.
        stage: StageId,
        /// Whether a semantic read-after-write chain could order the pair.
        orderable: bool,
    },
    /// A module wrote an IR field that no downstream module consumes.
    DeadWrite {
        /// Module that performed the write.
        module: ModuleId,
        /// Dead IR field path.
        field: String,
    },
    /// Runtime access exceeded the module's declared manifest access mask.
    UndeclaredAccess {
        /// Module performing the undeclared access.
        module: ModuleId,
        /// Whether the access was a read or write.
        access: AccessKind,
        /// Access path that was not declared.
        path: String,
    },
    /// A module directly requires a module from a later scheduler stage.
    CrossStageDependency {
        /// Requesting module.
        module: ModuleId,
        /// Required module.
        requires: ModuleId,
        /// Requesting module stage.
        module_stage: StageId,
        /// Required module stage.
        required_stage: StageId,
    },
    /// A module transitively depends on a later scheduler stage.
    TransitiveStageDependency {
        /// Requesting module.
        module: ModuleId,
        /// Discovered dependency chain.
        path: Vec<ModuleId>,
        /// Requesting module stage.
        module_stage: StageId,
        /// Later stage found in the transitive closure.
        later_stage: StageId,
    },
}

/// Scope used when reporting claim conflicts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictScope {
    /// Conflict applies globally to the entire print.
    Global,
    /// Conflict applies after region-level filtering.
    Region {
        /// Object identifier participating in the conflict.
        object_id: String,
        /// Region identifier participating in the conflict.
        region_id: String,
        /// Optional global layer index used to pin the conflict.
        global_layer_index: Option<u32>,
    },
}

/// Runtime access classification used by undeclared-access diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessKind {
    /// Runtime IR read.
    Read,
    /// Runtime IR write.
    Write,
}

/// One stage-local DAG supplied to the startup validator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageDag {
    /// Stage represented by these nodes.
    pub stage: StageId,
    /// Nodes for that stage.
    pub nodes: Vec<ModuleNode>,
}

/// One effective claim holder observation used by validation passes 2 and 3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimHolder {
    /// Claim identifier being resolved.
    pub claim: String,
    /// Module selected as a holder in the given scope.
    pub module_id: ModuleId,
    /// Scope in which this holder is active.
    pub scope: ConflictScope,
}

/// Static or recorded runtime access summary for validation pass 11.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleAccessAudit {
    /// Module being audited.
    pub module_id: ModuleId,
    /// Runtime read paths requested by the module.
    pub runtime_reads: Vec<String>,
    /// Runtime write paths committed by the module.
    pub runtime_writes: Vec<String>,
    /// Typed diagnostics emitted by the module during prepass execution.
    /// FIFO order, preserved from guest emission. Not compared by scheduler
    /// validation — only runtime_reads and runtime_writes participate.
    pub diagnostics: Vec<slicer_ir::Diagnostic>,
}

/// Startup validation input spanning all 14 documented passes.
#[derive(Debug, Clone, PartialEq)]
pub struct DagValidationRequest {
    /// Loaded modules to validate.
    pub modules: Vec<LoadedModule>,
    /// Per-stage DAGs produced during phase 2.
    pub stage_dags: Vec<StageDag>,
    /// Host IR schema version available to loaded modules.
    pub host_ir_schema_version: SemVer,
    /// Host semver of the running host, checked against each module's
    /// declared `min-host-version` by pass 14.
    pub host_version: SemVer,
    /// Effective claim holder snapshots for global and region scopes.
    pub claim_holders: Vec<ClaimHolder>,
    /// Optional runtime/static access audits used by pass 11.
    pub access_audits: Vec<ModuleAccessAudit>,
}

/// One of the 14 startup DAG validation passes from the scheduler contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DagValidationPass {
    /// Pass 1.
    StageIdValidation,
    /// Pass 2.
    GlobalClaimConflicts,
    /// Pass 3.
    PerRegionClaimConflicts,
    /// Pass 4.
    IncompatibilityDeclarations,
    /// Pass 5.
    MissingDependencies,
    /// Pass 6.
    IrVersionCompatibility,
    /// Pass 7.
    CycleDetection,
    /// Pass 8.
    WriteConflicts,
    /// Pass 9.
    UnfulfilledReads,
    /// Pass 10.
    DeadWrites,
    /// Pass 11.
    UndeclaredAccess,
    /// Pass 12.
    CrossStageDependencyLegality,
    /// Pass 13.
    TransitiveDependencyLegality,
    /// Pass 14.
    HostVersionCompatibility,
}

/// One structured startup validation diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagValidationDiagnostic {
    /// Validation pass that emitted the diagnostic.
    pub pass: DagValidationPass,
    /// Whether the diagnostic blocks execution.
    pub level: DiagnosticLevel,
    /// Structured scheduler error or warning payload.
    pub detail: SchedulerError,
}

/// Aggregated DAG validation output collected before surfacing diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DagValidationReport {
    /// Fatal diagnostics that block execution.
    pub errors: Vec<DagValidationDiagnostic>,
    /// Warning diagnostics that do not block execution.
    pub warnings: Vec<DagValidationDiagnostic>,
}

impl DagValidationReport {
    /// Appends one fatal diagnostic to the report.
    pub fn push_error(&mut self, pass: DagValidationPass, detail: SchedulerError) {
        self.errors.push(DagValidationDiagnostic {
            pass,
            level: DiagnosticLevel::Error,
            detail,
        });
    }

    /// Appends one warning diagnostic to the report.
    pub fn push_warning(&mut self, pass: DagValidationPass, detail: SchedulerError) {
        self.warnings.push(DagValidationDiagnostic {
            pass,
            level: DiagnosticLevel::Warning,
            detail,
        });
    }

    /// Returns true when no fatal diagnostics were recorded.
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Runs all 14 startup DAG validation passes and aggregates diagnostics.
pub fn validate_startup_dag(request: &DagValidationRequest) -> DagValidationReport {
    let mut report = DagValidationReport::default();
    let modules_by_id: BTreeMap<_, _> = request
        .modules
        .iter()
        .map(|module| (module.id.clone(), module))
        .collect();
    let stage_order = stage_order_index();

    validate_stage_ids(request, &stage_order, &mut report);
    validate_claim_conflicts(
        request,
        DagValidationPass::GlobalClaimConflicts,
        true,
        &mut report,
    );
    validate_claim_conflicts(
        request,
        DagValidationPass::PerRegionClaimConflicts,
        false,
        &mut report,
    );
    validate_incompatibilities(&modules_by_id, &mut report);
    validate_missing_dependencies(request, &modules_by_id, &mut report);
    validate_ir_versions(request, &mut report);
    validate_host_version(request, &mut report);
    validate_cycles(request, &mut report);
    validate_write_conflicts(request, &mut report);
    validate_unfulfilled_reads(request, &stage_order, &mut report);
    validate_dead_writes(request, &stage_order, &mut report);
    validate_undeclared_access(request, &modules_by_id, &mut report);
    validate_cross_stage_dependencies(&request.modules, &modules_by_id, &stage_order, &mut report);
    validate_transitive_dependencies(&request.modules, &modules_by_id, &stage_order, &mut report);

    report
}

fn validate_stage_ids(
    request: &DagValidationRequest,
    stage_order: &BTreeMap<&'static str, usize>,
    report: &mut DagValidationReport,
) {
    for module in &request.modules {
        if !stage_order.contains_key(module.stage.as_str()) {
            report.push_error(
                DagValidationPass::StageIdValidation,
                SchedulerError::UnknownStage {
                    module: module.id.clone(),
                    declared_stage: module.stage.clone(),
                },
            );
        }
    }

    for stage_dag in &request.stage_dags {
        if !stage_order.contains_key(stage_dag.stage.as_str()) {
            let module = stage_dag
                .nodes
                .first()
                .map(|node| node.module_id.clone())
                .unwrap_or_else(|| String::from("<stage-dag>"));
            report.push_error(
                DagValidationPass::StageIdValidation,
                SchedulerError::UnknownStage {
                    module,
                    declared_stage: stage_dag.stage.clone(),
                },
            );
        }
    }
}

fn validate_claim_conflicts(
    request: &DagValidationRequest,
    pass: DagValidationPass,
    global_only: bool,
    report: &mut DagValidationReport,
) {
    let non_transitionable_claims: BTreeSet<&'static str> = [
        "perimeter-generator",
        "seam-placer",
        "layer-planner",
        "mesh-analyzer",
    ]
    .into_iter()
    .collect();

    let mut holders_by_claim: BTreeMap<(String, String), Vec<ModuleId>> = BTreeMap::new();

    for holder in &request.claim_holders {
        let is_global = matches!(holder.scope, ConflictScope::Global);
        if is_global != global_only {
            continue;
        }

        let use_scope_key = if global_only {
            scope_key(&holder.scope)
        } else if non_transitionable_claims.contains(holder.claim.as_str()) {
            scope_key_without_layer(&holder.scope)
        } else {
            scope_key(&holder.scope)
        };

        holders_by_claim
            .entry((holder.claim.clone(), use_scope_key))
            .or_default()
            .push(holder.module_id.clone());
    }

    // ── Packet 37 fill-role claim coverage check ────────────────────────
    // Every fill-role claim must have at least one holder or the print is
    // impossible to produce — emit MissingDependency for each uncovered ID.
    if global_only {
        for fill_claim in FILL_CLAIM_IDS {
            let has_holder = holders_by_claim
                .keys()
                .any(|(claim, _)| claim == *fill_claim);
            if !has_holder {
                report.push_error(
                    DagValidationPass::GlobalClaimConflicts,
                    SchedulerError::MissingDependency {
                        module: format!("fill-role-claim:{}", fill_claim),
                        requires: format!("no module holds {}", fill_claim),
                    },
                );
            }
        }
    }

    for ((claim, scope_key_value), modules) in holders_by_claim {
        let mut sorted_modules = modules;
        sorted_modules.sort();
        sorted_modules.dedup();
        if sorted_modules.len() < 2 {
            continue;
        }

        // Fill-role claims are per-region-configurable and intentionally
        // allow multiple global holders: at dispatch time the per-region
        // `ResolvedConfig.{top,bottom,bridge,sparse}_fill_holder` picks the
        // active module. The global conflict pass therefore exempts them
        // (docs/04 §"Validation Passes" — "A single module may hold multiple
        // fill-role claims … per-region overrides may transfer a fill-role
        // claim to a different module"). The per-region pass still flags
        // genuine collisions at the (layer, object, region) level.
        if global_only && FILL_CLAIM_IDS.contains(&claim.as_str()) {
            continue;
        }

        let scope = if global_only {
            ConflictScope::Global
        } else {
            region_scope_from_key(&scope_key_value)
        };

        for i in 0..sorted_modules.len() {
            for j in (i + 1)..sorted_modules.len() {
                report.push_error(
                    pass,
                    SchedulerError::ClaimConflict {
                        claim: claim.clone(),
                        module_a: sorted_modules[i].clone(),
                        module_b: sorted_modules[j].clone(),
                        scope: scope.clone(),
                    },
                );
            }
        }
    }
}

fn validate_incompatibilities(
    modules_by_id: &BTreeMap<ModuleId, &LoadedModule>,
    report: &mut DagValidationReport,
) {
    for module in modules_by_id.values() {
        for conflicting in &module.incompatible_with {
            if modules_by_id.contains_key(conflicting) {
                report.push_error(
                    DagValidationPass::IncompatibilityDeclarations,
                    SchedulerError::IncompatibleModules {
                        declared_by: module.id.clone(),
                        conflicting: conflicting.clone(),
                        reason: format!(
                            "module '{}' declares '{}' as incompatible",
                            module.id, conflicting
                        ),
                    },
                );
            }
        }
    }
}

fn validate_missing_dependencies(
    request: &DagValidationRequest,
    modules_by_id: &BTreeMap<ModuleId, &LoadedModule>,
    report: &mut DagValidationReport,
) {
    let available_claims: BTreeSet<String> = request
        .claim_holders
        .iter()
        .map(|holder| holder.claim.clone())
        .collect();

    for module in &request.modules {
        for required_module in &module.requires_modules {
            if !modules_by_id.contains_key(required_module) {
                report.push_error(
                    DagValidationPass::MissingDependencies,
                    SchedulerError::MissingDependency {
                        module: module.id.clone(),
                        requires: required_module.clone(),
                    },
                );
            }
        }

        for required_claim in &module.requires_claims {
            if !available_claims.contains(required_claim) {
                report.push_error(
                    DagValidationPass::MissingDependencies,
                    SchedulerError::MissingDependency {
                        module: module.id.clone(),
                        requires: required_claim.clone(),
                    },
                );
            }
        }
    }
}

fn validate_ir_versions(request: &DagValidationRequest, report: &mut DagValidationReport) {
    for module in &request.modules {
        if semver_lt(request.host_ir_schema_version, module.min_ir_schema)
            || !semver_lt(request.host_ir_schema_version, module.max_ir_schema)
        {
            report.push_error(
                DagValidationPass::IrVersionCompatibility,
                SchedulerError::IrVersionIncompatible {
                    module: module.id.clone(),
                    ir_type: String::from("host-ir-schema"),
                    required: module.min_ir_schema,
                    available: request.host_ir_schema_version,
                },
            );
        }
    }
}

fn validate_host_version(request: &DagValidationRequest, report: &mut DagValidationReport) {
    for module in &request.modules {
        if semver_lt(request.host_version, module.min_host_version) {
            report.push_error(
                DagValidationPass::HostVersionCompatibility,
                SchedulerError::HostVersionIncompatible {
                    module: module.id.clone(),
                    required: module.min_host_version,
                    available: request.host_version,
                },
            );
        }
    }
}

fn validate_cycles(request: &DagValidationRequest, report: &mut DagValidationReport) {
    for stage_dag in &request.stage_dags {
        if let Err(cycle) = topological_sort(&stage_dag.nodes) {
            report.push_error(
                DagValidationPass::CycleDetection,
                SchedulerError::CyclicDependency { cycle },
            );
        }
    }
}

fn validate_write_conflicts(request: &DagValidationRequest, report: &mut DagValidationReport) {
    for stage_dag in &request.stage_dags {
        let reachability = compute_reachability(&stage_dag.nodes);
        for i in 0..stage_dag.nodes.len() {
            for j in (i + 1)..stage_dag.nodes.len() {
                let left = &stage_dag.nodes[i];
                let right = &stage_dag.nodes[j];
                let shared_fields = shared_paths(&left.ir_writes, &right.ir_writes);
                if shared_fields.is_empty() {
                    continue;
                }

                for field in shared_fields {
                    let left_transforms_right = right.ir_reads.contains(&field)
                        && can_reach(&reachability, &left.module_id, &right.module_id);
                    let right_transforms_left = left.ir_reads.contains(&field)
                        && can_reach(&reachability, &right.module_id, &left.module_id);
                    let orderable = left_transforms_right || right_transforms_left;
                    if orderable {
                        continue;
                    }

                    // Neither module's read establishes an ordering over this field.
                    // Ordering cannot resolve this write-write conflict.
                    report.push_error(
                        DagValidationPass::WriteConflicts,
                        SchedulerError::WriteConflict {
                            field,
                            module_a: left.module_id.clone(),
                            module_b: right.module_id.clone(),
                            stage: stage_dag.stage.clone(),
                            orderable,
                        },
                    );
                }
            }
        }
    }
}

fn validate_unfulfilled_reads(
    request: &DagValidationRequest,
    stage_order: &BTreeMap<&'static str, usize>,
    report: &mut DagValidationReport,
) {
    let all_writers = writers_by_field(&request.stage_dags, stage_order);
    let modules_by_id: BTreeMap<_, _> = request
        .modules
        .iter()
        .map(|module| (module.id.clone(), module))
        .collect();

    for stage_dag in &request.stage_dags {
        let reachability = compute_reachability(&stage_dag.nodes);
        let current_stage_index = stage_order.get(stage_dag.stage.as_str()).copied();
        for node in &stage_dag.nodes {
            let Some(module) = modules_by_id.get(&node.module_id) else {
                continue;
            };

            for field in &module.ir_reads {
                let satisfied = all_writers.get(field).is_some_and(|writers| {
                    writers.iter().any(|(writer_stage, writer_id)| {
                        if writer_id == &module.id {
                            return false;
                        }

                        match (
                            current_stage_index,
                            stage_order.get(writer_stage.as_str()).copied(),
                        ) {
                            (Some(reader_index), Some(writer_index))
                                if writer_index < reader_index =>
                            {
                                true
                            }
                            (Some(_), Some(_)) if writer_stage == &module.stage => {
                                can_reach(&reachability, writer_id, &module.id)
                            }
                            _ => false,
                        }
                    })
                });

                if !satisfied {
                    report.push_error(
                        DagValidationPass::UnfulfilledReads,
                        SchedulerError::UnfulfilledRead {
                            module: module.id.clone(),
                            field: field.clone(),
                            suggestion: None,
                        },
                    );
                }
            }
        }
    }
}

fn validate_dead_writes(
    request: &DagValidationRequest,
    stage_order: &BTreeMap<&'static str, usize>,
    report: &mut DagValidationReport,
) {
    let all_readers = readers_by_field(&request.stage_dags, stage_order);

    for stage_dag in &request.stage_dags {
        let reachability = compute_reachability(&stage_dag.nodes);
        let current_stage_index = stage_order.get(stage_dag.stage.as_str()).copied();

        for node in &stage_dag.nodes {
            for field in &node.ir_writes {
                let consumed = all_readers.get(field).is_some_and(|readers| {
                    readers.iter().any(|(reader_stage, reader_id)| {
                        match (
                            current_stage_index,
                            stage_order.get(reader_stage.as_str()).copied(),
                        ) {
                            (Some(writer_index), Some(reader_index))
                                if writer_index < reader_index =>
                            {
                                true
                            }
                            (Some(_), Some(_)) if reader_stage == &stage_dag.stage => {
                                can_reach(&reachability, &node.module_id, reader_id)
                            }
                            _ => false,
                        }
                    })
                });

                if !consumed {
                    report.push_warning(
                        DagValidationPass::DeadWrites,
                        SchedulerError::DeadWrite {
                            module: node.module_id.clone(),
                            field: field.clone(),
                        },
                    );
                }
            }
        }
    }
}

fn validate_undeclared_access(
    request: &DagValidationRequest,
    modules_by_id: &BTreeMap<ModuleId, &LoadedModule>,
    report: &mut DagValidationReport,
) {
    for audit in &request.access_audits {
        let Some(module) = modules_by_id.get(&audit.module_id) else {
            continue;
        };

        for path in &audit.runtime_reads {
            if !module.ir_reads.contains(path) {
                report.push_error(
                    DagValidationPass::UndeclaredAccess,
                    SchedulerError::UndeclaredAccess {
                        module: module.id.clone(),
                        access: AccessKind::Read,
                        path: path.clone(),
                    },
                );
            }
        }

        for path in &audit.runtime_writes {
            if !module.ir_writes.contains(path) {
                report.push_error(
                    DagValidationPass::UndeclaredAccess,
                    SchedulerError::UndeclaredAccess {
                        module: module.id.clone(),
                        access: AccessKind::Write,
                        path: path.clone(),
                    },
                );
            }
        }
    }
}

fn validate_cross_stage_dependencies(
    modules: &[LoadedModule],
    modules_by_id: &BTreeMap<ModuleId, &LoadedModule>,
    stage_order: &BTreeMap<&'static str, usize>,
    report: &mut DagValidationReport,
) {
    for module in modules {
        let Some(module_index) = stage_order.get(module.stage.as_str()).copied() else {
            continue;
        };

        for required_module in &module.requires_modules {
            let Some(required) = modules_by_id.get(required_module) else {
                continue;
            };
            let Some(required_index) = stage_order.get(required.stage.as_str()).copied() else {
                continue;
            };

            if required_index > module_index {
                report.push_error(
                    DagValidationPass::CrossStageDependencyLegality,
                    SchedulerError::CrossStageDependency {
                        module: module.id.clone(),
                        requires: required.id.clone(),
                        module_stage: module.stage.clone(),
                        required_stage: required.stage.clone(),
                    },
                );
            }
        }
    }
}

fn validate_transitive_dependencies(
    modules: &[LoadedModule],
    modules_by_id: &BTreeMap<ModuleId, &LoadedModule>,
    stage_order: &BTreeMap<&'static str, usize>,
    report: &mut DagValidationReport,
) {
    for module in modules {
        let Some(module_index) = stage_order.get(module.stage.as_str()).copied() else {
            continue;
        };

        let mut queue: VecDeque<Vec<ModuleId>> = module
            .requires_modules
            .iter()
            .cloned()
            .map(|required| vec![module.id.clone(), required])
            .collect();
        let mut visited_edges = BTreeSet::new();

        while let Some(path) = queue.pop_front() {
            let Some(current_id) = path.last() else {
                continue;
            };
            if !visited_edges.insert(path.clone()) {
                continue;
            }

            let Some(current) = modules_by_id.get(current_id) else {
                continue;
            };
            let Some(current_index) = stage_order.get(current.stage.as_str()).copied() else {
                continue;
            };

            if current_index > module_index {
                report.push_error(
                    DagValidationPass::TransitiveDependencyLegality,
                    SchedulerError::TransitiveStageDependency {
                        module: module.id.clone(),
                        path: path.clone(),
                        module_stage: module.stage.clone(),
                        later_stage: current.stage.clone(),
                    },
                );
                break;
            }

            let mut next_ids = current.requires_modules.clone();
            next_ids.sort();
            for next_id in next_ids {
                if path.contains(&next_id) {
                    continue;
                }
                let mut next_path = path.clone();
                next_path.push(next_id);
                queue.push_back(next_path);
            }
        }
    }
}

fn stage_order_index() -> BTreeMap<&'static str, usize> {
    crate::stage_order::stage_order_index()
}

fn semver_lt(left: SemVer, right: SemVer) -> bool {
    (left.major, left.minor, left.patch) < (right.major, right.minor, right.patch)
}

fn scope_key(scope: &ConflictScope) -> String {
    match scope {
        ConflictScope::Global => String::from("global"),
        ConflictScope::Region {
            object_id,
            region_id,
            global_layer_index,
        } => format!(
            "region:{object_id}:{region_id}:{}",
            global_layer_index.map_or_else(|| String::from("none"), |value| value.to_string())
        ),
    }
}

fn scope_key_without_layer(scope: &ConflictScope) -> String {
    match scope {
        ConflictScope::Global => String::from("global"),
        ConflictScope::Region {
            object_id,
            region_id,
            global_layer_index: _,
        } => format!("region:{object_id}:{region_id}:_"),
    }
}

fn region_scope_from_key(key: &str) -> ConflictScope {
    let mut parts = key.splitn(4, ':');
    let _ = parts.next();
    let object_id = parts.next().unwrap_or_default().to_string();
    let region_id = parts.next().unwrap_or_default().to_string();
    let global_layer_index = parts.next().and_then(|value| {
        if value == "none" {
            None
        } else {
            value.parse::<u32>().ok()
        }
    });

    ConflictScope::Region {
        object_id,
        region_id,
        global_layer_index,
    }
}

fn topological_sort(nodes: &[ModuleNode]) -> Result<Vec<ModuleId>, Vec<ModuleId>> {
    let mut in_degree: BTreeMap<ModuleId, usize> = nodes
        .iter()
        .map(|node| (node.module_id.clone(), 0usize))
        .collect();

    for node in nodes {
        for edge in &node.edges_to {
            if let Some(degree) = in_degree.get_mut(&edge.to) {
                *degree += 1;
            }
        }
    }

    let mut queue: VecDeque<ModuleId> = in_degree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(module_id, _)| module_id.clone())
        .collect();
    let mut sorted = Vec::with_capacity(nodes.len());

    while let Some(module_id) = queue.pop_front() {
        sorted.push(module_id.clone());
        if let Some(node) = nodes.iter().find(|node| node.module_id == module_id) {
            for edge in &node.edges_to {
                if let Some(degree) = in_degree.get_mut(&edge.to) {
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(edge.to.clone());
                    }
                }
            }
        }
    }

    if sorted.len() == nodes.len() {
        Ok(sorted)
    } else {
        let visited: BTreeSet<_> = sorted.into_iter().collect();
        Err(nodes
            .iter()
            .map(|node| node.module_id.clone())
            .filter(|module_id| !visited.contains(module_id))
            .collect())
    }
}

fn compute_reachability(nodes: &[ModuleNode]) -> BTreeMap<ModuleId, BTreeSet<ModuleId>> {
    let adjacency: BTreeMap<ModuleId, Vec<ModuleId>> = nodes
        .iter()
        .map(|node| {
            (
                node.module_id.clone(),
                node.edges_to.iter().map(|e| e.to.clone()).collect(),
            )
        })
        .collect();
    let mut reachability = BTreeMap::new();

    for node in nodes {
        let mut visited = BTreeSet::new();
        let mut queue: VecDeque<ModuleId> = adjacency
            .get(&node.module_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();

        while let Some(current) = queue.pop_front() {
            if !visited.insert(current.clone()) {
                continue;
            }
            if let Some(next_nodes) = adjacency.get(&current) {
                for next in next_nodes {
                    queue.push_back(next.clone());
                }
            }
        }

        reachability.insert(node.module_id.clone(), visited);
    }

    reachability
}

fn can_reach(
    reachability: &BTreeMap<ModuleId, BTreeSet<ModuleId>>,
    from: &ModuleId,
    to: &ModuleId,
) -> bool {
    reachability
        .get(from)
        .is_some_and(|reachable| reachable.contains(to))
}

fn shared_paths(left: &[String], right: &[String]) -> Vec<String> {
    let right_set: BTreeSet<_> = right.iter().cloned().collect();
    left.iter()
        .filter(|path| right_set.contains(*path))
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn writers_by_field(
    stage_dags: &[StageDag],
    stage_order: &BTreeMap<&'static str, usize>,
) -> BTreeMap<String, Vec<(StageId, ModuleId)>> {
    let mut writers = BTreeMap::new();

    for stage_dag in sorted_stage_dags(stage_dags, stage_order) {
        for node in &stage_dag.nodes {
            for field in &node.ir_writes {
                writers
                    .entry(field.clone())
                    .or_insert_with(Vec::new)
                    .push((stage_dag.stage.clone(), node.module_id.clone()));
            }
        }
    }

    writers
}

fn readers_by_field(
    stage_dags: &[StageDag],
    stage_order: &BTreeMap<&'static str, usize>,
) -> BTreeMap<String, Vec<(StageId, ModuleId)>> {
    let mut readers = BTreeMap::new();

    for stage_dag in sorted_stage_dags(stage_dags, stage_order) {
        for node in &stage_dag.nodes {
            for field in &node.ir_reads {
                readers
                    .entry(field.clone())
                    .or_insert_with(Vec::new)
                    .push((stage_dag.stage.clone(), node.module_id.clone()));
            }
        }
    }

    readers
}

fn sorted_stage_dags<'a>(
    stage_dags: &'a [StageDag],
    stage_order: &BTreeMap<&'static str, usize>,
) -> Vec<&'a StageDag> {
    let mut sorted: Vec<_> = stage_dags.iter().collect();
    sorted.sort_by_key(|stage_dag| {
        (
            stage_order
                .get(stage_dag.stage.as_str())
                .copied()
                .unwrap_or(usize::MAX),
            stage_dag.stage.clone(),
        )
    });
    sorted
}

#[cfg(test)]
mod tests {
    use super::{DagValidationPass, DagValidationReport, SchedulerError};

    #[test]
    fn report_groups_errors_and_warnings_by_severity() {
        let mut report = DagValidationReport::default();
        report.push_error(
            DagValidationPass::CycleDetection,
            SchedulerError::NotImplemented,
        );
        report.push_warning(
            DagValidationPass::DeadWrites,
            SchedulerError::NotImplemented,
        );

        assert_eq!(report.errors.len(), 1);
        assert_eq!(report.warnings.len(), 1);
        assert!(!report.is_valid());
    }
}
