//! Deterministic AC-2 control for charter `H-DAG`: real intra-stage DAG
//! construction and real execution-plan assembly.
//!
//! Every expectation below is derived from the declaration tables
//! (`GRAPH_DECLARATIONS`, `PLAN_DECLARATIONS`) or from the real
//! `slicer_scheduler::topological_sort` output — never from
//! `ModuleNode::edges_to` or `ExecutionPlan` fields copied back. Nothing here
//! reads `PNP_PROPERTY_CONTEXT` or builds a `PropertyRun`.
//!
//! The graph half is the relation-table model the charter mandates:
//! `slicer_scheduler::build_intra_stage_dag` over real
//! `slicer_scheduler::LoadedModule` declarations must produce exactly the
//! same-stage write/read path intersection plus the reversed
//! `requires_modules` relation, with one `EdgeTo` per destination and exact
//! typed reason sets. A producer declared in a different `STAGE_ORDER` stage
//! contributes neither node nor edge.
//!
//! The plan half assembles a real `ExecutionPlan` from the valid sorted output
//! of `slicer_scheduler::topological_sort`, real
//! `slicer_scheduler::bind_module_config_view` views and real
//! `ExecutionModuleBinding`s, then asserts compiled sequences, claims, masks,
//! explicit requirements, paint-only semantics, stage bucketing and the host
//! `Layer::PaintRegionAnnotation` insertion. The full region-split registry
//! witness is owned by
//! `crates/slicer-scheduler/tests/common/property_scheduler_contracts/registry_controls.rs`;
//! here the aggregate is only the
//! single declaration this fixture binds.

#![allow(missing_docs)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{GlobalLayer, ModuleId, RegionKey, RegionPlan, ResolvedConfig, SemVer};
use slicer_scheduler::{
    bind_module_config_view, build_execution_plan, build_intra_stage_dag, topological_sort,
    AggregatedRegionSplitEntry, CompiledModuleStatic, CompiledStage, EdgeReason,
    ExecutionModuleBinding, ExecutionPlan, ExecutionPlanRequest, LoadDiagnostic, LoadedModule,
    LoadedModuleBuilder, ModuleNode, Producer, RegionSplitDeclaration, RegionSplitValueType,
    SortedStageModules, STAGE_ORDER,
};

/// Requested intra-stage DAG stage. Asserted against `STAGE_ORDER` below so the
/// "same stage" premise is real.
const DAG_STAGE: &str = "Layer::Perimeters";

/// A different real `STAGE_ORDER` stage, used to prove stage exclusion.
const OTHER_STAGE: &str = "Layer::Infill";

/// Stage the host always provides at plan-build time when no manifest does.
const PAINT_STAGE: &str = "Layer::PaintRegionAnnotation";

/// The single region-split semantic this plan fixture declares.
const SHARED_SEMANTIC: &str = "com.control.plan.shared";

/// Community priority for the single declared semantic.
const SHARED_PRIORITY: u32 = 1500;

/// One loaded-module declaration row. Both fixtures derive their expectations
/// from these rows; no row is ever reconstructed from production output.
#[derive(Debug, Clone, Copy)]
struct Declaration {
    id: &'static str,
    stage: &'static str,
    ir_reads: &'static [&'static str],
    ir_writes: &'static [&'static str],
    requires_modules: &'static [&'static str],
    claims: &'static [&'static str],
    declares_shared_semantic: bool,
    paint_only: bool,
}

/// Graph-construction declarations for `build_intra_stage_dag`.
///
/// `com.control.dag.mirror` both reads and writes
/// `PerimeterIR.regions.walls` (self write/read must not self-edge).
/// `com.control.dag.consumer` requires a module declared in `OTHER_STAGE`, and
/// `com.control.dag.outside` writes a path a requested-stage reader declares —
/// both must be suppressed by the stage filter.
const GRAPH_DECLARATIONS: &[Declaration] = &[
    Declaration {
        id: "com.control.dag.writer",
        stage: DAG_STAGE,
        ir_reads: &[],
        ir_writes: &["PerimeterIR.regions.walls"],
        requires_modules: &[],
        claims: &[],
        declares_shared_semantic: false,
        paint_only: false,
    },
    Declaration {
        id: "com.control.dag.mirror",
        stage: DAG_STAGE,
        ir_reads: &["PerimeterIR.regions.walls"],
        ir_writes: &["PerimeterIR.regions.walls", "PerimeterIR.regions.finish"],
        requires_modules: &["com.control.dag.writer"],
        claims: &[],
        declares_shared_semantic: false,
        paint_only: false,
    },
    Declaration {
        id: "com.control.dag.finisher",
        stage: DAG_STAGE,
        ir_reads: &["PerimeterIR.regions.walls", "PerimeterIR.regions.finish"],
        ir_writes: &[],
        requires_modules: &["com.control.dag.mirror"],
        claims: &[],
        declares_shared_semantic: false,
        paint_only: false,
    },
    Declaration {
        id: "com.control.dag.consumer",
        stage: DAG_STAGE,
        ir_reads: &["PerimeterIR.regions.walls"],
        ir_writes: &[],
        requires_modules: &["com.control.dag.outside"],
        claims: &[],
        declares_shared_semantic: false,
        paint_only: false,
    },
    Declaration {
        id: "com.control.dag.isolated",
        stage: DAG_STAGE,
        ir_reads: &[],
        ir_writes: &[],
        requires_modules: &[],
        claims: &[],
        declares_shared_semantic: false,
        paint_only: false,
    },
    Declaration {
        id: "com.control.dag.outside",
        stage: OTHER_STAGE,
        ir_reads: &[],
        ir_writes: &["PerimeterIR.regions.walls"],
        requires_modules: &[],
        claims: &[],
        declares_shared_semantic: false,
        paint_only: false,
    },
];

/// Plan-assembly declarations, one non-empty stage per bucket plus two
/// declaring modules for the single semantic.
///
/// `Layer::Perimeters` forms the chain
/// `perimeters -> declaring -> paint`:
/// `perimeters` writes `PerimeterIR.regions.walls` which `declaring` reads and
/// which `declaring` also names in `requires_modules` (dual reason);
/// `declaring` writes `PerimeterIR.regions.finish` which `paint` reads and
/// which `paint` also requires (dual reason).
const PLAN_DECLARATIONS: &[Declaration] = &[
    Declaration {
        id: "com.control.plan.layer-planner",
        stage: "PrePass::LayerPlanning",
        ir_reads: &["MeshIR.objects"],
        ir_writes: &["LayerPlanIR.layers"],
        requires_modules: &[],
        claims: &["layer-planner"],
        declares_shared_semantic: false,
        paint_only: false,
    },
    Declaration {
        id: "com.control.plan.perimeters",
        stage: "Layer::Perimeters",
        ir_reads: &["LayerPlanIR.layers", "PerimeterIR.regions.walls"],
        ir_writes: &["PerimeterIR.regions.walls"],
        requires_modules: &[],
        claims: &["perimeter-generator"],
        declares_shared_semantic: false,
        paint_only: false,
    },
    Declaration {
        id: "com.control.plan.declaring",
        stage: "Layer::Perimeters",
        ir_reads: &["PerimeterIR.regions.walls"],
        ir_writes: &["PerimeterIR.regions.finish"],
        requires_modules: &["com.control.plan.perimeters"],
        claims: &["com.control.claim.finish"],
        declares_shared_semantic: true,
        paint_only: false,
    },
    Declaration {
        id: "com.control.plan.paint",
        stage: "Layer::Perimeters",
        ir_reads: &["PerimeterIR.regions.finish"],
        ir_writes: &["PaintIR.region_annotations"],
        requires_modules: &["com.control.plan.declaring"],
        claims: &["com.control.claim.paint"],
        declares_shared_semantic: true,
        paint_only: true,
    },
    Declaration {
        id: "com.control.plan.infill",
        stage: "Layer::Infill",
        ir_reads: &["PerimeterIR.regions.walls"],
        ir_writes: &["InfillIR.regions.sparse"],
        requires_modules: &[],
        claims: &["com.control.claim.infill"],
        declares_shared_semantic: false,
        paint_only: false,
    },
    Declaration {
        id: "com.control.plan.finalizer",
        stage: "PostPass::LayerFinalization",
        ir_reads: &["InfillIR.regions.sparse"],
        ir_writes: &["LayerCollectionIR.finalized"],
        requires_modules: &[],
        claims: &[],
        declares_shared_semantic: false,
        paint_only: false,
    },
    Declaration {
        id: "com.control.plan.gcode",
        stage: "PostPass::GCodeEmit",
        ir_reads: &["LayerCollectionIR.finalized"],
        ir_writes: &["GCodeIR.text"],
        requires_modules: &[],
        claims: &[],
        declares_shared_semantic: false,
        paint_only: false,
    },
];

/// Manifest-provided paint stage used only by the not-duplicated control.
const PAINT_ANNOTATOR: Declaration = Declaration {
    id: "com.control.plan.paint-annotator",
    stage: PAINT_STAGE,
    ir_reads: &["SliceIR.regions"],
    ir_writes: &["SliceIR.regions.segment_annotations"],
    requires_modules: &[],
    claims: &[],
    declares_shared_semantic: false,
    paint_only: false,
};

/// Already-sorted stages with no modules. They must not become compiled
/// stages (one per bucket family).
const EMPTY_PLAN_STAGES: &[&str] = &[
    "PrePass::RegionMapping",
    "Layer::PerimetersPostProcess",
    "PostPass::GCodePostProcess",
];

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| String::from(*value)).collect()
}

/// Build the real `LoadedModule` a declaration row describes.
fn build_module(row: &Declaration) -> LoadedModule {
    let mut builder = LoadedModuleBuilder::new(
        row.id,
        semver(1, 0, 0),
        row.stage,
        slicer_schema::TIER_LAYER,
        PathBuf::from(format!("fixtures/{}.wasm", row.id)),
    )
    .ir_reads(strings(row.ir_reads))
    .ir_writes(strings(row.ir_writes))
    .requires_modules(strings(row.requires_modules))
    .claims(strings(row.claims))
    .paint_only(row.paint_only)
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(2, 0, 0))
    .layer_parallel_safe(true);

    if row.declares_shared_semantic {
        builder = builder.region_splits(vec![RegionSplitDeclaration {
            semantic: SHARED_SEMANTIC.to_string(),
            priority: SHARED_PRIORITY,
            value_type: RegionSplitValueType::CustomString,
        }]);
    }

    builder.build()
}

fn stage_declarations<'a>(rows: &'a [Declaration], stage: &str) -> Vec<&'a Declaration> {
    rows.iter().filter(|row| row.stage == stage).collect()
}

fn stage_position(stage: &str) -> usize {
    STAGE_ORDER
        .iter()
        .position(|candidate| *candidate == stage)
        .unwrap_or_else(|| panic!("stage '{stage}' must be a canonical STAGE_ORDER stage"))
}

/// Independent relation table for one stage.
///
/// The same-stage write/read intersection produces one
/// `EdgeReason::IrWriteRead { writer_path }` per shared path, and each
/// `requires_modules` entry that names another module in the *same* stage is
/// reversed into `EdgeReason::ExplicitRequires` from required to consumer.
/// Cross-stage requirements are deliberately absent: that is the expectation
/// the stage-exclusion assertions check.
fn relation_table(
    rows: &[Declaration],
    stage: &str,
) -> BTreeMap<(ModuleId, ModuleId), Vec<EdgeReason>> {
    let in_stage = stage_declarations(rows, stage);
    let mut table: BTreeMap<(ModuleId, ModuleId), Vec<EdgeReason>> = BTreeMap::new();

    for writer in &in_stage {
        for reader in &in_stage {
            if writer.id == reader.id {
                continue;
            }
            for written_path in writer.ir_writes {
                if reader.ir_reads.contains(written_path) {
                    table
                        .entry((writer.id.to_string(), reader.id.to_string()))
                        .or_default()
                        .push(EdgeReason::IrWriteRead {
                            writer_path: (*written_path).to_string(),
                        });
                }
            }
        }
    }

    for consumer in &in_stage {
        for required in consumer.requires_modules {
            if in_stage.iter().any(|row| row.id == *required) {
                table
                    .entry(((*required).to_string(), consumer.id.to_string()))
                    .or_default()
                    .push(EdgeReason::ExplicitRequires);
            }
        }
    }

    // The builder drops repeated identical reasons; the model must dedupe too.
    for reasons in table.values_mut() {
        let mut deduped: Vec<EdgeReason> = Vec::new();
        for reason in reasons.drain(..) {
            if !deduped.contains(&reason) {
                deduped.push(reason);
            }
        }
        *reasons = deduped;
    }

    table
}

/// Stable ordering key for an `EdgeReason` (the enum has no `Ord`).
fn reason_key(reason: &EdgeReason) -> String {
    match reason {
        EdgeReason::IrWriteRead { writer_path } => format!("ir-write-read:{writer_path}"),
        EdgeReason::ExplicitRequires => String::from("explicit-requires"),
    }
}

fn sorted_reasons(reasons: &[EdgeReason]) -> Vec<EdgeReason> {
    let mut sorted = reasons.to_vec();
    sorted.sort_by_key(|reason| reason_key(reason));
    sorted
}

fn edge_reasons<'a>(nodes: &'a [ModuleNode], from: &str, to: &str) -> &'a [EdgeReason] {
    let node = nodes
        .iter()
        .find(|node| node.module_id == from)
        .unwrap_or_else(|| panic!("node '{from}' must exist"));
    let edge = node
        .edges_to
        .iter()
        .find(|edge| edge.to == to)
        .unwrap_or_else(|| panic!("edge '{from}' -> '{to}' must exist"));
    &edge.reasons
}

/// One real plan plus the sorted input and bindings it was built from.
struct PlanFixture {
    plan: ExecutionPlan,
    sorted_by_stage: BTreeMap<String, Vec<ModuleId>>,
    ordered_stages: Vec<String>,
    bindings: Vec<ExecutionModuleBinding>,
}

/// Non-empty stages per plan bucket, derived from the declaration table and the
/// canonical `STAGE_ORDER` — not from `ExecutionPlan`.
struct ExpectedBuckets {
    prepass: Vec<String>,
    layer: Vec<String>,
    layer_finalization: Option<String>,
    postpass: Vec<String>,
}

fn expected_buckets(fixture: &PlanFixture) -> ExpectedBuckets {
    let mut buckets = ExpectedBuckets {
        prepass: Vec::new(),
        layer: Vec::new(),
        layer_finalization: None,
        postpass: Vec::new(),
    };

    for stage in &fixture.ordered_stages {
        let modules = fixture
            .sorted_by_stage
            .get(stage)
            .unwrap_or_else(|| panic!("every ordered stage has a sorted entry: {stage}"));
        if modules.is_empty() {
            continue;
        }
        if stage.starts_with("PrePass::") {
            buckets.prepass.push(stage.clone());
        } else if stage.starts_with("Layer::") {
            buckets.layer.push(stage.clone());
        } else if stage == "PostPass::LayerFinalization" {
            buckets.layer_finalization = Some(stage.clone());
        } else if stage.starts_with("PostPass::") {
            buckets.postpass.push(stage.clone());
        }
    }

    buckets
}

/// Build a real plan from declaration rows.
///
/// Stages are ordered by canonical `STAGE_ORDER`; every stage's module sequence
/// is the real `topological_sort` output for a real
/// `build_intra_stage_dag` graph over the plan's producers. Bindings use the
/// production `bind_module_config_view` helper with `ResolvedConfig::default()`.
fn build_plan_fixture(rows: &[Declaration]) -> (PlanFixture, Vec<LoadDiagnostic>) {
    let modules: Vec<LoadedModule> = rows.iter().map(build_module).collect();
    let producers: Vec<&dyn Producer> = modules
        .iter()
        .map(|module| module as &dyn Producer)
        .collect();

    let mut stage_ids: BTreeSet<String> = rows.iter().map(|row| row.stage.to_string()).collect();
    stage_ids.extend(EMPTY_PLAN_STAGES.iter().map(|stage| (*stage).to_string()));
    let mut ordered_stages: Vec<String> = stage_ids.into_iter().collect();
    ordered_stages.sort_by_key(|stage| stage_position(stage));

    let mut sorted_by_stage: BTreeMap<String, Vec<ModuleId>> = BTreeMap::new();
    let mut sorted_stages: Vec<SortedStageModules> = Vec::new();
    for stage in &ordered_stages {
        let nodes = build_intra_stage_dag(stage.clone(), &producers).unwrap_or_else(|error| {
            panic!("build_intra_stage_dag rejected plan fixture stage '{stage}': {error:?}")
        });
        let sorted = topological_sort(&nodes).unwrap_or_else(|blocked| {
            panic!("plan fixture stage '{stage}' must be acyclic, blocked remainder {blocked:?}")
        });
        sorted_by_stage.insert(stage.clone(), sorted.clone());
        // exhaustive: SortedStageModules has no Default; both fields are the sorted plan input
        sorted_stages.push(SortedStageModules {
            stage_id: stage.clone(),
            module_ids: sorted,
        });
    }

    let bindings: Vec<ExecutionModuleBinding> = modules
        .iter()
        .map(|module| {
            // exhaustive: ExecutionModuleBinding has no Default; the module plus its frozen view is the binding pair
            ExecutionModuleBinding {
                module: module.clone(),
                config_view: bind_module_config_view(module, &ResolvedConfig::default()),
            }
        })
        .collect();

    // exhaustive: ExecutionPlanRequest has no Default; every field is part of this plan stimulus
    let request = ExecutionPlanRequest {
        sorted_stages,
        module_bindings: bindings.clone(),
        global_layers: Arc::new(Vec::<GlobalLayer>::new()),
        region_plans: Arc::new(HashMap::<RegionKey, RegionPlan>::new()),
    };

    let mut diagnostics = Vec::new();
    let plan = build_execution_plan(&request, &mut diagnostics)
        .expect("valid sorted stages and bound modules must compile into an execution plan");

    (
        PlanFixture {
            plan,
            sorted_by_stage,
            ordered_stages,
            bindings,
        },
        diagnostics,
    )
}

fn compiled_stage_ids(stages: &[CompiledStage]) -> Vec<String> {
    stages.iter().map(|stage| stage.stage_id.clone()).collect()
}

fn compiled_stage<'a>(plan: &'a ExecutionPlan, stage_id: &str) -> &'a CompiledStage {
    let mut stages: Vec<&CompiledStage> = plan
        .prepass_stages
        .iter()
        .chain(plan.per_layer_stages.iter())
        .chain(plan.postpass_stages.iter())
        .collect();
    if let Some(stage) = &plan.layer_finalization_stage {
        stages.push(stage);
    }
    stages
        .into_iter()
        .find(|stage| stage.stage_id == stage_id)
        .unwrap_or_else(|| panic!("plan must contain compiled stage '{stage_id}'"))
}

fn compiled_module<'a>(
    plan: &'a ExecutionPlan,
    stage_id: &str,
    module_id: &str,
) -> &'a CompiledModuleStatic {
    let stage = compiled_stage(plan, stage_id);
    stage
        .modules
        .iter()
        .find(|module| module.module_id().as_str() == module_id)
        .unwrap_or_else(|| panic!("stage '{stage_id}' must contain module '{module_id}'"))
}

fn assert_module_sequence(plan: &ExecutionPlan, stage_id: &str, sorted: &[ModuleId]) {
    let stage = compiled_stage(plan, stage_id);
    let actual: Vec<ModuleId> = stage
        .modules
        .iter()
        .map(|module| module.module_id().clone())
        .collect();
    assert_eq!(
        actual, sorted,
        "compiled module sequence for '{stage_id}' must equal the sorted input"
    );
}

#[test]
fn scheduler_graph_construction_and_plan_observations() {
    // ── Fixture premises ────────────────────────────────────────────────
    assert_eq!(DAG_STAGE, "Layer::Perimeters");
    assert_ne!(DAG_STAGE, OTHER_STAGE);
    for stage in [DAG_STAGE, OTHER_STAGE] {
        assert_eq!(
            STAGE_ORDER
                .iter()
                .filter(|candidate| **candidate == stage)
                .count(),
            1,
            "'{stage}' must be a real canonical STAGE_ORDER stage so stage exclusion is meaningful"
        );
    }
    let outside_row = GRAPH_DECLARATIONS
        .iter()
        .find(|row| row.stage != DAG_STAGE)
        .expect("the graph table declares exactly one producer outside the requested stage");
    assert_eq!(outside_row.stage, OTHER_STAGE);

    // ── Real intra-stage DAG construction over bounded declarations ─────
    let graph_modules: Vec<LoadedModule> = GRAPH_DECLARATIONS.iter().map(build_module).collect();
    let graph_producers: Vec<&dyn Producer> = graph_modules
        .iter()
        .map(|module| module as &dyn Producer)
        .collect();
    let nodes = build_intra_stage_dag(DAG_STAGE.to_string(), &graph_producers)
        .expect("build_intra_stage_dag must accept the bounded LoadedModule declarations");

    let graph_rows = stage_declarations(GRAPH_DECLARATIONS, DAG_STAGE);
    let mut expected_node_ids: Vec<ModuleId> =
        graph_rows.iter().map(|row| row.id.to_string()).collect();
    expected_node_ids.sort();
    let actual_node_ids: Vec<ModuleId> = nodes.iter().map(|node| node.module_id.clone()).collect();
    assert_eq!(
        actual_node_ids, expected_node_ids,
        "the node set must be exactly the requested stage's declaration rows"
    );
    assert!(
        !actual_node_ids
            .iter()
            .any(|module_id| module_id == outside_row.id),
        "a producer declared in {} must contribute no node to {}",
        outside_row.stage,
        DAG_STAGE
    );

    let node_by_id: BTreeMap<&str, &ModuleNode> = nodes
        .iter()
        .map(|node| (node.module_id.as_str(), node))
        .collect();
    for row in &graph_rows {
        let node = node_by_id
            .get(row.id)
            .unwrap_or_else(|| panic!("requested-stage node '{}' must exist", row.id));
        assert_eq!(
            node.ir_reads,
            strings(row.ir_reads),
            "node '{}' must carry exactly its declared reads",
            row.id
        );
        assert_eq!(
            node.ir_writes,
            strings(row.ir_writes),
            "node '{}' must carry exactly its declared writes",
            row.id
        );
    }

    // ── Relation-table model: destination uniqueness, exact reason sets ─
    let table = relation_table(GRAPH_DECLARATIONS, DAG_STAGE);
    for node in &nodes {
        let destination_ids: Vec<ModuleId> =
            node.edges_to.iter().map(|edge| edge.to.clone()).collect();
        let destination_set: BTreeSet<ModuleId> = destination_ids.iter().cloned().collect();
        assert_eq!(
            destination_ids.len(),
            destination_set.len(),
            "each node must have at most one EdgeTo per destination; '{}' repeats one",
            node.module_id
        );

        let expected_destinations: BTreeSet<ModuleId> = table
            .keys()
            .filter(|(from, _)| from == &node.module_id)
            .map(|(_, to)| to.clone())
            .collect();
        assert_eq!(
            destination_set, expected_destinations,
            "destination set for '{}' must match the relation table",
            node.module_id
        );

        for edge in &node.edges_to {
            let key = (node.module_id.clone(), edge.to.clone());
            let expected = table.get(&key).unwrap_or_else(|| {
                panic!("builder emitted edge {key:?} outside the relation table")
            });
            assert_eq!(
                sorted_reasons(&edge.reasons),
                sorted_reasons(expected),
                "reason set for edge {key:?} must be exactly the relation-table reasons"
            );
        }
    }
    for (from, to) in table.keys() {
        let node = node_by_id
            .get(from.as_str())
            .unwrap_or_else(|| panic!("relation-table source '{from}' must be a built node"));
        assert!(
            node.edges_to.iter().any(|edge| &edge.to == to),
            "relation-table edge '{from}' -> '{to}' is missing from the built graph"
        );
    }

    // A module that both writes and reads the same path must not self-edge.
    let mirror = node_by_id
        .get("com.control.dag.mirror")
        .expect("the mirror declaration must be a node");
    assert!(mirror
        .ir_reads
        .iter()
        .any(|path| path == "PerimeterIR.regions.walls"));
    assert!(mirror
        .ir_writes
        .iter()
        .any(|path| path == "PerimeterIR.regions.walls"));
    assert!(
        mirror
            .edges_to
            .iter()
            .all(|edge| edge.to != mirror.module_id),
        "a module that writes and reads the same path must not emit a self-edge"
    );

    // requires_modules coinciding with an IR write/read edge: both reasons on
    // one EdgeTo (writer -> mirror), and two IrWriteRead reasons plus the
    // explicit reason on mirror -> finisher.
    assert_eq!(
        sorted_reasons(edge_reasons(
            &nodes,
            "com.control.dag.writer",
            "com.control.dag.mirror"
        )),
        sorted_reasons(&[
            EdgeReason::IrWriteRead {
                writer_path: String::from("PerimeterIR.regions.walls"),
            },
            EdgeReason::ExplicitRequires,
        ]),
        "writer -> mirror must carry both the write/read and explicit-requires reasons"
    );
    assert_eq!(
        sorted_reasons(edge_reasons(
            &nodes,
            "com.control.dag.mirror",
            "com.control.dag.finisher"
        )),
        sorted_reasons(&[
            EdgeReason::IrWriteRead {
                writer_path: String::from("PerimeterIR.regions.walls"),
            },
            EdgeReason::IrWriteRead {
                writer_path: String::from("PerimeterIR.regions.finish"),
            },
            EdgeReason::ExplicitRequires,
        ]),
        "mirror -> finisher must carry both shared writer paths plus the explicit requires reason"
    );

    // Stage exclusion is non-vacuous: the outside producer writes a path a
    // requested-stage reader declares, and the consumer requires it, yet no
    // node or edge may mention it.
    assert!(
        graph_rows.iter().any(|row| row
            .ir_reads
            .iter()
            .any(|path| outside_row.ir_writes.contains(path))),
        "the out-of-stage producer must write a path a requested-stage reader declares"
    );
    let consumer_row = graph_rows
        .iter()
        .find(|row| row.id == "com.control.dag.consumer")
        .expect("the consumer declaration must be in the requested stage");
    assert_eq!(consumer_row.requires_modules, &[outside_row.id]);
    assert!(
        node_by_id["com.control.dag.consumer"].edges_to.is_empty(),
        "a requires_modules entry naming another STAGE_ORDER stage must produce no intra-stage edge"
    );
    assert!(
        nodes
            .iter()
            .flat_map(|node| node.edges_to.iter())
            .all(|edge| edge.to != outside_row.id),
        "no edge may target the out-of-stage producer"
    );
    assert!(
        node_by_id["com.control.dag.isolated"].edges_to.is_empty(),
        "a module with no declarations must remain an isolated node"
    );

    // ── Real plan assembly from the valid sorted output ─────────────────
    let (fixture, diagnostics) = build_plan_fixture(PLAN_DECLARATIONS);
    assert!(
        diagnostics.is_empty(),
        "compatible declarations must not warn: {diagnostics:?}"
    );

    // The chain order is hand-computed from the declaration table: the only
    // in-degree-zero node is `perimeters`; `declaring` requires it and reads
    // its write; `paint` requires `declaring` and reads its write.
    let expected_perimeter_order: Vec<ModuleId> = vec![
        String::from("com.control.plan.perimeters"),
        String::from("com.control.plan.declaring"),
        String::from("com.control.plan.paint"),
    ];
    assert_eq!(
        fixture.sorted_by_stage["Layer::Perimeters"], expected_perimeter_order,
        "the Layer::Perimeters chain must sort to its unique hand-computed order"
    );
    let plan_table = relation_table(PLAN_DECLARATIONS, "Layer::Perimeters");
    let plan_positions: BTreeMap<ModuleId, usize> = expected_perimeter_order
        .iter()
        .cloned()
        .enumerate()
        .map(|(position, module_id)| (module_id, position))
        .collect();
    for (from, to) in plan_table.keys() {
        assert!(
            plan_positions[from] < plan_positions[to],
            "hand-computed order must place relation-table edge '{from}' -> '{to}' upstream first"
        );
    }

    // ── Stage bucketing (derived from the table, not the plan) ──────────
    let buckets = expected_buckets(&fixture);
    assert_eq!(
        compiled_stage_ids(&fixture.plan.prepass_stages),
        buckets.prepass,
        "PrePass:: stages must land in prepass_stages"
    );
    assert_eq!(
        fixture
            .plan
            .layer_finalization_stage
            .as_ref()
            .map(|stage| stage.stage_id.clone()),
        buckets.layer_finalization,
        "PostPass::LayerFinalization must be isolated into layer_finalization_stage"
    );
    assert_eq!(
        compiled_stage_ids(&fixture.plan.postpass_stages),
        buckets.postpass,
        "other PostPass:: stages must land in postpass_stages"
    );

    // ── Host paint-stage insertion before the first later Layer stage ───
    assert!(
        !PLAN_DECLARATIONS.iter().any(|row| row.stage == PAINT_STAGE),
        "the base fixture must declare no manifest paint stage"
    );
    let mut expected_layer_stages = buckets.layer.clone();
    expected_layer_stages.insert(0, PAINT_STAGE.to_string());
    assert_eq!(
        compiled_stage_ids(&fixture.plan.per_layer_stages),
        expected_layer_stages,
        "Layer:: stages must land in per_layer_stages with the host paint stage inserted first"
    );
    assert_eq!(
        fixture.plan.per_layer_stages[0].stage_id, PAINT_STAGE,
        "the host-inserted paint stage must precede the first later Layer stage"
    );
    assert!(
        fixture.plan.per_layer_stages[0].modules.is_empty(),
        "the host-inserted paint stage must carry zero modules"
    );
    assert_eq!(
        fixture
            .plan
            .per_layer_stages
            .iter()
            .filter(|stage| stage.stage_id == PAINT_STAGE)
            .count(),
        1,
        "the host paint stage must be inserted exactly once"
    );
    for (offset, stage) in buckets.layer.iter().enumerate() {
        assert_eq!(
            fixture.plan.per_layer_stages[offset + 1].stage_id,
            *stage,
            "declared layer stage '{stage}' must follow the inserted paint stage"
        );
    }

    // ── Empty sorted stages produce no compiled stage ───────────────────
    let mut all_compiled: BTreeSet<String> = fixture
        .plan
        .prepass_stages
        .iter()
        .map(|stage| stage.stage_id.clone())
        .collect();
    all_compiled.extend(
        fixture
            .plan
            .per_layer_stages
            .iter()
            .map(|stage| stage.stage_id.clone()),
    );
    all_compiled.extend(
        fixture
            .plan
            .postpass_stages
            .iter()
            .map(|stage| stage.stage_id.clone()),
    );
    if let Some(stage) = &fixture.plan.layer_finalization_stage {
        all_compiled.insert(stage.stage_id.clone());
    }
    for stage in EMPTY_PLAN_STAGES {
        assert!(
            fixture.sorted_by_stage[*stage].is_empty(),
            "fixture stage '{stage}' must be supplied with no module ids"
        );
        assert!(
            !all_compiled.contains(*stage),
            "empty sorted stage '{stage}' must produce no CompiledStage"
        );
    }

    // ── Compiled sequence per stage equals the sorted input ─────────────
    for stage in &buckets.prepass {
        assert_module_sequence(&fixture.plan, stage, &fixture.sorted_by_stage[stage]);
    }
    for stage in &buckets.layer {
        assert_module_sequence(&fixture.plan, stage, &fixture.sorted_by_stage[stage]);
    }
    if let Some(stage) = &buckets.layer_finalization {
        assert_module_sequence(&fixture.plan, stage, &fixture.sorted_by_stage[stage]);
    }
    for stage in &buckets.postpass {
        assert_module_sequence(&fixture.plan, stage, &fixture.sorted_by_stage[stage]);
    }

    // ── Binding propagation: claims, masks, requirements, semantics ─────
    for row in PLAN_DECLARATIONS {
        let compiled = compiled_module(&fixture.plan, row.stage, row.id);
        assert_eq!(
            compiled.module_id().as_str(),
            row.id,
            "compiled module identity must be the declared id"
        );
        assert_eq!(
            compiled.claims().to_vec(),
            strings(row.claims),
            "claims for '{}' must propagate from the declaration",
            row.id
        );
        assert_eq!(
            compiled.ir_read_mask().paths,
            strings(row.ir_reads),
            "ir_read_mask().paths for '{}' must propagate from the declaration",
            row.id
        );
        assert_eq!(
            compiled.ir_write_mask().paths,
            strings(row.ir_writes),
            "ir_write_mask().paths for '{}' must propagate from the declaration",
            row.id
        );
        assert_eq!(
            compiled.requires_modules().to_vec(),
            strings(row.requires_modules),
            "requires_modules for '{}' must propagate from the declaration",
            row.id
        );
        let expected_semantics: HashSet<String> = if row.paint_only {
            [SHARED_SEMANTIC.to_string()].into_iter().collect()
        } else {
            HashSet::new()
        };
        assert_eq!(
            compiled.region_split_semantics(),
            &expected_semantics,
            "region_split_semantics for '{}' must be the paint-only declaration set (empty for a declaring-but-transparent module)",
            row.id
        );
        let binding = fixture
            .bindings
            .iter()
            .find(|binding| binding.module.id() == row.id)
            .unwrap_or_else(|| panic!("every declaration row must be bound: {}", row.id));
        assert!(
            Arc::ptr_eq(compiled.config_view(), &binding.config_view),
            "compiled config view for '{}' must be the binding's frozen view",
            row.id
        );
    }

    // Both directions of the paint-only split, named explicitly.
    let paint_row = PLAN_DECLARATIONS
        .iter()
        .find(|row| row.id == "com.control.plan.paint")
        .expect("the paint-only declaration must exist");
    assert!(paint_row.paint_only && paint_row.declares_shared_semantic);
    let declaring_row = PLAN_DECLARATIONS
        .iter()
        .find(|row| row.id == "com.control.plan.declaring")
        .expect("the declaring-but-transparent declaration must exist");
    assert!(declaring_row.declares_shared_semantic && !declaring_row.paint_only);
    assert_eq!(
        compiled_module(&fixture.plan, paint_row.stage, paint_row.id).region_split_semantics(),
        &[SHARED_SEMANTIC.to_string()]
            .into_iter()
            .collect::<HashSet<_>>(),
        "paint_only = true must compile exactly the declared semantic"
    );
    assert!(
        compiled_module(&fixture.plan, declaring_row.stage, declaring_row.id)
            .region_split_semantics()
            .is_empty(),
        "declaration alone must not compile a per-layer dispatch set"
    );

    // ── Plan aggregate derived from the bound declarations ──────────────
    let mut expected_declaring_modules: Vec<ModuleId> = PLAN_DECLARATIONS
        .iter()
        .filter(|row| row.declares_shared_semantic)
        .map(|row| row.id.to_string())
        .collect();
    expected_declaring_modules.sort();
    assert_eq!(
        expected_declaring_modules.len(),
        2,
        "the fixture binds exactly two declarers of its one semantic"
    );
    assert_eq!(
        fixture
            .plan
            .aggregated_region_split
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec![SHARED_SEMANTIC],
        "the plan aggregate must hold exactly the bound semantic; no core semantics are seeded"
    );
    let entry = fixture
        .plan
        .aggregated_region_split
        .get(SHARED_SEMANTIC)
        .expect("the bound declaration must be aggregated");
    assert_eq!(
        entry,
        &AggregatedRegionSplitEntry {
            priority: SHARED_PRIORITY,
            value_type: RegionSplitValueType::CustomString,
            declaring_modules: expected_declaring_modules,
        },
        "the plan aggregate entry must be the declaration-table entry with sorted declaring modules"
    );

    // ── A manifest-provided paint stage is not duplicated ───────────────
    let mut provided_rows: Vec<Declaration> = PLAN_DECLARATIONS.to_vec();
    provided_rows.push(PAINT_ANNOTATOR);
    assert_eq!(
        provided_rows
            .iter()
            .filter(|row| row.stage == PAINT_STAGE)
            .count(),
        1,
        "the provided-stage fixture declares exactly one paint module"
    );
    let (provided, provided_diagnostics) = build_plan_fixture(&provided_rows);
    assert!(
        provided_diagnostics.is_empty(),
        "a manifest paint module must not add declarations or warnings: {provided_diagnostics:?}"
    );
    let provided_buckets = expected_buckets(&provided);
    assert_eq!(
        compiled_stage_ids(&provided.plan.per_layer_stages),
        provided_buckets.layer,
        "a manifest-provided paint stage must not be duplicated by the host"
    );
    assert_eq!(
        provided_buckets.layer.first().map(String::as_str),
        Some(PAINT_STAGE),
        "STAGE_ORDER places the manifest paint stage first among declared layer stages"
    );
    assert_eq!(
        provided
            .plan
            .per_layer_stages
            .iter()
            .filter(|stage| stage.stage_id == PAINT_STAGE)
            .count(),
        1,
        "the manifest paint stage must appear exactly once"
    );
    let provided_paint = compiled_stage(&provided.plan, PAINT_STAGE);
    assert_eq!(
        provided_paint
            .modules
            .iter()
            .map(|module| module.module_id().clone())
            .collect::<Vec<_>>(),
        vec![PAINT_ANNOTATOR.id.to_string()],
        "the provided paint stage must contain exactly its manifest module, not an empty host stage"
    );
    assert!(
        provided
            .plan
            .per_layer_stages
            .iter()
            .all(|stage| !stage.modules.is_empty()),
        "no zero-module paint stage may remain when the manifest provides one"
    );
    assert_module_sequence(
        &provided.plan,
        PAINT_STAGE,
        &provided.sorted_by_stage[PAINT_STAGE],
    );
    let annotator = compiled_module(&provided.plan, PAINT_ANNOTATOR.stage, PAINT_ANNOTATOR.id);
    assert_eq!(
        annotator.ir_read_mask().paths,
        strings(PAINT_ANNOTATOR.ir_reads)
    );
    assert_eq!(
        annotator.ir_write_mask().paths,
        strings(PAINT_ANNOTATOR.ir_writes)
    );
    assert!(annotator.claims().is_empty());
    assert!(annotator.requires_modules().is_empty());
    assert!(annotator.region_split_semantics().is_empty());
}
