//! Deterministic controls for charter `H-DAG`.
//!
//! Hand-computed witnesses proving the exhaustive linear-extension model, the
//! nonempty-path blocked-remainder model and the checked raw-graph adapter
//! reject plausible bad sorters before any production call. Nothing here reads
//! `PNP_PROPERTY_CONTEXT` or builds a `PropertyRun`.
//!
//! `scheduler_raw_graph_and_plan_rejections` additionally observes the four
//! plan-binding errors at `slicer_scheduler::build_execution_plan`
//! (`crates/slicer-scheduler/src/execution_plan.rs`) and a cyclic production
//! stage graph at `slicer_scheduler::topological_sort`
//! (`crates/slicer-scheduler/src/topology.rs`) plus
//! `slicer_scheduler::validate_startup_dag`
//! (`crates/slicer-scheduler/src/validation.rs`). Those are the real
//! validating APIs for scheduling negatives: the scheduler has no raw-graph
//! production validator, so malformed raw models stay at the checked
//! test-adapter/oracle boundary.
//!
//! `models` re-mounts the target-private model file relative to this
//! directory (`tests/common/property_scheduler_contracts/`).

// The target-private model file is mounted once by this binary's root; this
// alias keeps the control body readable.
use crate::dag_models as models;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{
    ConfigValue, ConfigView, GlobalLayer, RegionKey, RegionPlan, ResolvedConfig, SemVer,
};
use slicer_scheduler::{
    bind_module_config_view, build_execution_plan, build_intra_stage_dag, topological_sort,
    validate_startup_dag, ClaimHolder, ConfigFieldEntry, ConfigSchema, ConflictScope,
    DagValidationPass, DagValidationRequest, DiagnosticLevel, EdgeReason, ExecutionModuleBinding,
    ExecutionPlanError, ExecutionPlanRequest, LoadedModule, LoadedModuleBuilder, Producer,
    SchedulerError, SortedStageModules, StageDag, FILL_CLAIM_IDS, STAGE_ORDER,
};
use slicer_sdk::test_support::property_testing::domains::SmallGraph;
use slicer_sdk::test_support::property_testing::oracles::{
    graph_cycle_remainder, graph_reachability,
};

fn ids(values: &[u16]) -> Vec<String> {
    values.iter().copied().map(models::module_id_for).collect()
}

#[test]
fn scheduler_dag_oracle_controls() {
    // ── (a) FIFO / insertion-order-ready instead of lexical minimum ────────
    // Node Vec order is [1, 2, 0] with 1 -> 2. Insertion-order-ready pops 1
    // first, then 2 (newly ready), then 0: [1, 2, 0]. The lexical-minimum-ready
    // definition must pop the independent 0 first: [0, 1, 2].
    let graph = SmallGraph {
        node_ids: vec![1, 2, 0],
        edges: vec![(1, 2)],
    };
    let model = models::minimum_linear_extension(&graph)
        .expect("linear-extension model accepts the fixture graph")
        .expect("the fixture graph is acyclic");
    let expected = ids(&[0, 1, 2]);
    assert_eq!(
        model, expected,
        "the exhaustive model must choose the lexically minimum linear extension [0, 1, 2]"
    );

    let insertion_ready = ids(&[1, 2, 0]);
    models::edge_precedence_holds(&graph, &insertion_ready).expect(
        "the FIFO mutation satisfies every edge precedence, so it is a plausible non-minimal sorter",
    );
    let rejection = models::check_success_oracle(&graph, &insertion_ready).expect_err(
        "FIFO/insertion-order-ready mutation must be rejected by the exact lexically-minimum assertion",
    );
    assert!(
        rejection.contains("exact order mismatch"),
        "the FIFO mutation must be caught by the exact-order assertion, got: {rejection}"
    );

    // ── (b) a result missing one node ──────────────────────────────────────
    let missing = expected[..2].to_vec();
    let rejection = models::check_success_oracle(&graph, &missing)
        .expect_err("missing-node mutation must be rejected by the bijection assertion");
    assert!(
        rejection.contains("omits"),
        "the missing-node mutation must be caught by the bijection omits assertion, got: {rejection}"
    );

    // ── (c) a result duplicating one node ──────────────────────────────────
    let duplicated = vec![
        expected[0].clone(),
        expected[1].clone(),
        expected[1].clone(),
    ];
    let rejection = models::check_success_oracle(&graph, &duplicated)
        .expect_err("duplicate-node mutation must be rejected by the bijection assertion");
    assert!(
        rejection.contains("repeats"),
        "the duplicate-node mutation must be caught by the bijection repeats assertion, got: {rejection}"
    );

    // ── (d) a cycle-only remainder that drops the acyclic tail ─────────────
    // Cycle 0 <-> 1 with acyclic tail edge 1 -> 2. The path model must return
    // the cycle nodes plus the tail; a simple-cycle route returns {0, 1} only.
    let cyclic = SmallGraph {
        node_ids: vec![0, 1, 2],
        edges: vec![(0, 1), (1, 0), (1, 2)],
    };
    let oracle_remainder =
        graph_cycle_remainder(&cyclic).expect("raw graph oracle accepts the cyclic fixture");
    assert!(
        oracle_remainder.contains(&2),
        "the nonempty-path model must include the acyclic tail node 2, got {oracle_remainder:?}"
    );
    let expected_remainder: BTreeSet<String> = ids(&[0, 1, 2]).into_iter().collect();
    let model_remainder =
        models::cycle_remainder_model(&cyclic).expect("cycle-remainder model accepts the fixture");
    assert_eq!(
        model_remainder, expected_remainder,
        "the model remainder must be the cycle plus the acyclic tail [0, 1, 2]"
    );

    let cycle_only = ids(&[0, 1]);
    let rejection = models::check_cycle_remainder(&cyclic, &cycle_only).expect_err(
        "cycle-only-tail-omission mutation must be rejected by the blocked-remainder model",
    );
    assert!(
        rejection.contains("omits blocked node"),
        "the cycle-only mutation must be caught by the omits-blocked-node assertion, got: {rejection}"
    );
    models::check_cycle_remainder(&cyclic, &ids(&[0, 1, 2]))
        .expect("the complete cycle-plus-tail remainder must pass the model");
}

#[test]
fn scheduler_dag_raw_model_rejections_before_sorter() {
    // Duplicate node entry: the checked adapter must reject before any call to
    // `slicer_scheduler::topological_sort`.
    let duplicate = SmallGraph {
        node_ids: vec![0, 1, 1],
        edges: Vec::new(),
    };
    let rejection = models::sort_checked(&duplicate)
        .expect_err("the checked adapter must reject a duplicate-node raw model");
    assert!(
        rejection.contains("raw graph rejected before sorting"),
        "the duplicate-node raw model must be rejected by the adapter, not the sorter, got: {rejection}"
    );
    assert!(
        rejection.contains("duplicate node"),
        "the duplicate-node rejection must name the fault, got: {rejection}"
    );

    // Unknown endpoint: same pre-sorter rejection requirement.
    let unknown_endpoint = SmallGraph {
        node_ids: vec![0, 1],
        edges: vec![(1, 7)],
    };
    let rejection = models::sort_checked(&unknown_endpoint)
        .expect_err("the checked adapter must reject an unknown-endpoint raw model");
    assert!(
        rejection.contains("raw graph rejected before sorting"),
        "the unknown-endpoint raw model must be rejected by the adapter, not the sorter, got: {rejection}"
    );
    assert!(
        rejection.contains("unknown endpoint"),
        "the unknown-endpoint rejection must name the fault, got: {rejection}"
    );

    // The shared raw-graph oracles reject the same two inputs.
    for (label, graph) in [
        ("duplicate-node", &duplicate),
        ("unknown-endpoint", &unknown_endpoint),
    ] {
        let reachability = graph_reachability(graph)
            .expect_err("graph_reachability must reject a raw malformed graph");
        assert!(
            reachability
                .to_string()
                .contains("graph has duplicate nodes or unknown endpoints"),
            "{label} reachability rejection must name duplicate/unknown endpoints, got: {reachability}"
        );

        let remainder = graph_cycle_remainder(graph)
            .expect_err("graph_cycle_remainder must reject a raw malformed graph");
        assert!(
            remainder
                .to_string()
                .contains("graph has duplicate nodes or unknown endpoints"),
            "{label} cycle-remainder rejection must name duplicate/unknown endpoints, got: {remainder}"
        );
    }

    // A valid raw model passes the same adapter and sorts, proving only rejected
    // models skip the sorter.
    let valid = SmallGraph {
        node_ids: vec![1, 0],
        edges: vec![(0, 1)],
    };
    let sorted = models::sort_checked(&valid)
        .expect("a valid raw model must pass the checked adapter and the public sorter");
    assert_eq!(
        sorted,
        ids(&[0, 1]),
        "the valid raw model must sort to [0, 1]"
    );
}

// ── AC-N1 fixture constants ─────────────────────────────────────────────

/// Canonical `slicer_scheduler::STAGE_ORDER` stage the placement negatives
/// supply as the already-sorted stage.
const PLAN_STAGE: &str = "Layer::Infill";

/// A different canonical `STAGE_ORDER` stage, used to prove that a module
/// declared in one stage and placed in another is rejected.
const DECLARED_STAGE: &str = "Layer::Perimeters";

/// Module id no binding supplies.
const UNBOUND_MODULE: &str = "com.pbt.binding.unbound";

/// Module id two bindings supply.
const DUPLICATE_MODULE: &str = "com.pbt.binding.duplicate";

/// Module declared in [`DECLARED_STAGE`] but supplied in [`PLAN_STAGE`].
const MISPLACED_MODULE: &str = "com.pbt.binding.misplaced";

/// Module whose `[config.schema]` declares only [`DECLARED_CONFIG_KEY`].
const CONFIG_MODULE: &str = "com.pbt.binding.config";

/// The single declared config key [`CONFIG_MODULE`] exposes.
const DECLARED_CONFIG_KEY: &str = "layer_height";

/// The undeclared config key a bypassing `ConfigView::from_map` leaks.
const LEAKED_CONFIG_KEY: &str = "pbt_leaked_knob";

/// Canonical `STAGE_ORDER` stage of the cyclic production fixture.
const CYCLE_STAGE: &str = "Layer::Perimeters";

/// Cycle member that requires [`CYCLE_B`].
const CYCLE_A: &str = "com.pbt.cycle.a";

/// Cycle member that requires [`CYCLE_A`].
const CYCLE_B: &str = "com.pbt.cycle.b";

/// Downstream acyclic tail that requires [`CYCLE_A`].
const CYCLE_TAIL: &str = "com.pbt.cycle.tail";

/// Independent sortable module that must stay outside the blocked set.
const CYCLE_FREE: &str = "com.pbt.cycle.free";

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

/// Real `LoadedModuleBuilder` base for the negative fixtures.
///
/// The version window accepts the validator request's host versions
/// (`host_version` 0.1.0, `host_ir_schema_version` 1.0.0), so a cyclic
/// request cannot be turned green by an unrelated version diagnostic.
fn module_builder(id: &str, stage: &str, requires_modules: &[&str]) -> LoadedModuleBuilder {
    LoadedModuleBuilder::new(
        id,
        semver(1, 0, 0),
        stage,
        slicer_schema::TIER_LAYER,
        PathBuf::from(format!("fixtures/{id}.wasm")),
    )
    .requires_modules(
        requires_modules
            .iter()
            .map(|required| (*required).to_string())
            .collect(),
    )
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(2, 0, 0))
    .layer_parallel_safe(true)
}

fn fixture_module(id: &str, stage: &str, requires_modules: &[&str]) -> LoadedModule {
    module_builder(id, stage, requires_modules).build()
}

/// Module declaring exactly one `[config.schema]` key.
fn config_fixture_module(id: &str) -> LoadedModule {
    let entries = BTreeMap::from([(
        DECLARED_CONFIG_KEY.to_string(),
        ConfigFieldEntry {
            field_type: String::from("float"),
            ..Default::default()
        },
    )]);
    module_builder(id, PLAN_STAGE, &[])
        .config_schema(ConfigSchema { entries })
        .build()
}

fn binding(module: LoadedModule, config_view: Arc<ConfigView>) -> ExecutionModuleBinding {
    // exhaustive: ExecutionModuleBinding has no Default; the module plus its frozen view is the binding pair
    ExecutionModuleBinding {
        module,
        config_view,
    }
}

/// Plan request with exactly one already-sorted stage.
fn plan_request(
    stage_id: &str,
    module_ids: &[&str],
    module_bindings: Vec<ExecutionModuleBinding>,
) -> ExecutionPlanRequest {
    // exhaustive: SortedStageModules has no Default; both fields are the sorted plan input
    let sorted_stage = SortedStageModules {
        stage_id: stage_id.to_string(),
        module_ids: module_ids.iter().map(|id| (*id).to_string()).collect(),
    };
    // exhaustive: ExecutionPlanRequest has no Default; every field is part of this rejection stimulus
    ExecutionPlanRequest {
        sorted_stages: vec![sorted_stage],
        module_bindings,
        global_layers: Arc::new(Vec::<GlobalLayer>::new()),
        region_plans: Arc::new(HashMap::<RegionKey, RegionPlan>::new()),
    }
}

/// The four configured fill holders, so a cycle-only request cannot be
/// rejected for missing fill coverage instead of for its cycle.
fn fill_claim_holders() -> Vec<ClaimHolder> {
    FILL_CLAIM_IDS
        .iter()
        .map(|claim| ClaimHolder {
            claim: (*claim).to_string(),
            module_id: format!(
                "com.pbt.fill-holder.{}",
                claim.trim_start_matches("claim:").replace('-', "_")
            ),
            scope: ConflictScope::Global,
        })
        .collect()
}

/// AC-N1: malformed raw models are rejected before any validated sorter, and
/// the real plan/cycle validating APIs reject invalid scheduling inputs with
/// their offending values named.
#[test]
fn scheduler_raw_graph_and_plan_rejections() {
    // ── Raw duplicate-node model: adapter and both oracles reject ───────
    let duplicate = SmallGraph {
        node_ids: vec![0, 1, 1],
        edges: Vec::new(),
    };
    let duplicate_adapter = models::checked_nodes(&duplicate).expect_err(
        "the checked adapter must reject a duplicate-node raw model before producing sorter nodes",
    );
    assert_eq!(
        duplicate_adapter, "duplicate node id 1",
        "the adapter rejection must name the duplicated node value 1"
    );
    let duplicate_sort = models::sort_checked(&duplicate)
        .expect_err("a duplicate-node raw model must produce no public sorter call");
    assert_eq!(
        duplicate_sort, "raw graph rejected before sorting: duplicate node id 1",
        "the duplicate-node rejection must be tagged as pre-sorter and name value 1; \
         a sorter-relayed 'public sorter reported a blocked remainder' result is forbidden \
         because no topological_sort may run on this model"
    );
    let duplicate_reachability = graph_reachability(&duplicate).expect_err(
        "graph_reachability must reject the duplicate-node model (node_ids [0, 1, 1], duplicated value 1)",
    );
    assert_eq!(
        duplicate_reachability.to_string(),
        "invalid domain input: graph has duplicate nodes or unknown endpoints",
        "the reachability oracle must reject the duplicate-node witness for its structure"
    );
    let duplicate_remainder = graph_cycle_remainder(&duplicate).expect_err(
        "graph_cycle_remainder must reject the duplicate-node model (node_ids [0, 1, 1], duplicated value 1)",
    );
    assert_eq!(
        duplicate_remainder.to_string(),
        "invalid domain input: graph has duplicate nodes or unknown endpoints",
        "the cycle-remainder oracle must reject the duplicate-node witness for its structure"
    );

    // ── Raw unknown-endpoint model: adapter and both oracles reject ─────
    let unknown_endpoint = SmallGraph {
        node_ids: vec![0, 1],
        edges: vec![(1, 7)],
    };
    let unknown_adapter = models::checked_nodes(&unknown_endpoint).expect_err(
        "the checked adapter must reject an unknown-endpoint raw model before producing sorter nodes",
    );
    assert_eq!(
        unknown_adapter, "unknown endpoint 7",
        "the adapter rejection must name the unknown endpoint value 7"
    );
    let unknown_sort = models::sort_checked(&unknown_endpoint)
        .expect_err("an unknown-endpoint raw model must produce no public sorter call");
    assert_eq!(
        unknown_sort, "raw graph rejected before sorting: unknown endpoint 7",
        "the unknown-endpoint rejection must be tagged as pre-sorter and name value 7; \
         no partial schedule from a validated sorter is permitted"
    );
    let unknown_reachability = graph_reachability(&unknown_endpoint).expect_err(
        "graph_reachability must reject the unknown-endpoint model (node_ids [0, 1], edge 1 -> 7)",
    );
    assert_eq!(
        unknown_reachability.to_string(),
        "invalid domain input: graph has duplicate nodes or unknown endpoints",
        "the reachability oracle must reject the unknown-endpoint witness for its structure"
    );
    let unknown_remainder = graph_cycle_remainder(&unknown_endpoint).expect_err(
        "graph_cycle_remainder must reject the unknown-endpoint model (node_ids [0, 1], edge 1 -> 7)",
    );
    assert_eq!(
        unknown_remainder.to_string(),
        "invalid domain input: graph has duplicate nodes or unknown endpoints",
        "the cycle-remainder oracle must reject the unknown-endpoint witness for its structure"
    );

    // Non-vacuity: a valid raw model passes the same adapter and does reach
    // the public sorter, so the two rejections above are not a dead path.
    let valid = SmallGraph {
        node_ids: vec![1, 0],
        edges: vec![(0, 1)],
    };
    let valid_nodes = models::checked_nodes(&valid)
        .expect("a structurally valid raw model must pass the checked adapter");
    assert_eq!(
        valid_nodes.len(),
        2,
        "the valid control must produce one sorter node per raw node"
    );
    let valid_sorted = models::sort_checked(&valid)
        .expect("the valid control must reach and pass the public sorter");
    assert_eq!(
        valid_sorted,
        ids(&[0, 1]),
        "the valid control must sort to the lexically ordered [0, 1]"
    );
    // The same two oracles accept the valid control, so their four rejections
    // above are structural rather than an always-error stub.
    let valid_paths = graph_reachability(&valid)
        .expect("graph_reachability must accept the structurally valid control");
    assert!(
        valid_paths.contains(&(0, 1)),
        "the reachability oracle must report the valid control's 0 -> 1 path, got {valid_paths:?}"
    );
    let valid_remainder = graph_cycle_remainder(&valid)
        .expect("graph_cycle_remainder must accept the structurally valid control");
    assert!(
        valid_remainder.is_empty(),
        "the valid control is acyclic, so its blocked remainder must be empty, got {valid_remainder:?}"
    );

    // ── Premise: every stage below is a canonical STAGE_ORDER stage ─────
    assert!(
        [PLAN_STAGE, DECLARED_STAGE, CYCLE_STAGE]
            .iter()
            .all(|stage| STAGE_ORDER.contains(stage)),
        "the placement and cycle stages must be real STAGE_ORDER stages so the negatives \
         are about bindings/graphs, not unknown stage ids"
    );

    // ── MissingModuleBinding: sorted stage names an unbound module ──────
    let missing_binding = build_execution_plan(
        &plan_request(PLAN_STAGE, &[UNBOUND_MODULE], Vec::new()),
        &mut Vec::new(),
    )
    .expect_err("a sorted stage naming a module with no binding must be rejected");
    assert_eq!(
        missing_binding,
        ExecutionPlanError::MissingModuleBinding {
            stage_id: PLAN_STAGE.to_string(),
            module_id: UNBOUND_MODULE.to_string(),
        },
        "MissingModuleBinding must name both the referencing stage and the unbound module id"
    );

    // ── DuplicateModuleBinding: two bindings for one module id ──────────
    let duplicate_module = fixture_module(DUPLICATE_MODULE, PLAN_STAGE, &[]);
    let duplicate_bindings = vec![
        binding(duplicate_module.clone(), Arc::new(ConfigView::new())),
        binding(duplicate_module, Arc::new(ConfigView::new())),
    ];
    let duplicate_binding = build_execution_plan(
        &plan_request(PLAN_STAGE, &[DUPLICATE_MODULE], duplicate_bindings),
        &mut Vec::new(),
    )
    .expect_err("two runtime bindings for one module id must be rejected");
    assert_eq!(
        duplicate_binding,
        ExecutionPlanError::DuplicateModuleBinding {
            module_id: DUPLICATE_MODULE.to_string(),
        },
        "DuplicateModuleBinding must name the duplicated module id"
    );

    // ── StageMismatch: declared in one stage, placed in another ─────────
    let misplaced = fixture_module(MISPLACED_MODULE, DECLARED_STAGE, &[]);
    let misplaced_view = bind_module_config_view(&misplaced, &ResolvedConfig::default());
    let stage_mismatch = build_execution_plan(
        &plan_request(
            PLAN_STAGE,
            &[MISPLACED_MODULE],
            vec![binding(misplaced, misplaced_view)],
        ),
        &mut Vec::new(),
    )
    .expect_err(
        "a module declared in one STAGE_ORDER stage but placed in another must be rejected",
    );
    assert_eq!(
        stage_mismatch,
        ExecutionPlanError::StageMismatch {
            module_id: MISPLACED_MODULE.to_string(),
            expected_stage: PLAN_STAGE.to_string(),
            actual_stage: DECLARED_STAGE.to_string(),
        },
        "StageMismatch must name the module, the sorted stage it was placed in and the stage it declares"
    );

    // ── UndeclaredConfigKey: from_map view leaks a non-schema key ───────
    let config_module = config_fixture_module(CONFIG_MODULE);
    let mut resolved = ResolvedConfig::default();
    resolved
        .extensions
        .insert(DECLARED_CONFIG_KEY.to_string(), ConfigValue::Float(0.2));
    resolved
        .extensions
        .insert(LEAKED_CONFIG_KEY.to_string(), ConfigValue::Bool(true));
    let helper_view = bind_module_config_view(&config_module, &resolved);
    assert_eq!(
        helper_view.keys(),
        vec![DECLARED_CONFIG_KEY.to_string()],
        "the real bind_module_config_view helper must expose exactly the declared key and \
         drop the undeclared extension key"
    );

    let leaky_view = ConfigView::from_map(HashMap::from([
        (DECLARED_CONFIG_KEY.to_string(), ConfigValue::Float(0.2)),
        (LEAKED_CONFIG_KEY.to_string(), ConfigValue::Bool(true)),
    ]));
    let undeclared = build_execution_plan(
        &plan_request(
            PLAN_STAGE,
            &[CONFIG_MODULE],
            vec![binding(config_module.clone(), Arc::new(leaky_view))],
        ),
        &mut Vec::new(),
    )
    .expect_err(
        "a binding whose ConfigView was built with ConfigView::from_map and exposes a key \
         outside the module's [config.schema] must be rejected",
    );
    assert_eq!(
        undeclared,
        ExecutionPlanError::UndeclaredConfigKey {
            module_id: CONFIG_MODULE.to_string(),
            key: LEAKED_CONFIG_KEY.to_string(),
        },
        "UndeclaredConfigKey must name the module and the offending undeclared key"
    );

    // The real helper-produced view of the SAME module must not error, and
    // the compiled module must expose only the declared key.
    let plan = build_execution_plan(
        &plan_request(
            PLAN_STAGE,
            &[CONFIG_MODULE],
            vec![binding(config_module, Arc::clone(&helper_view))],
        ),
        &mut Vec::new(),
    )
    .expect("the helper-produced declared-only view of the same module must assemble a plan");
    let plan_stage = plan
        .per_layer_stages
        .iter()
        .find(|stage| stage.stage_id == PLAN_STAGE)
        .expect("the Layer::Infill stage must compile into per_layer_stages");
    assert_eq!(
        plan_stage
            .modules
            .iter()
            .map(|module| module.module_id().clone())
            .collect::<Vec<_>>(),
        vec![CONFIG_MODULE.to_string()],
        "the compiled stage must hold exactly the bound config module"
    );
    let compiled = &plan_stage.modules[0];
    assert_eq!(
        compiled.config_view().keys(),
        vec![DECLARED_CONFIG_KEY.to_string()],
        "the compiled view bound by bind_module_config_view must expose exactly the declared key"
    );
    assert!(
        !compiled.config_view().contains_key(LEAKED_CONFIG_KEY),
        "the compiled view must not expose the undeclared key that from_map leaked"
    );

    // ── Cyclic production input: builder-derived cycle plus acyclic tail ─
    // `requires_modules` is reversed into producer-before-consumer edges:
    // CYCLE_A requires CYCLE_B -> B -> A; CYCLE_B requires CYCLE_A -> A -> B
    // (the cycle); CYCLE_TAIL requires CYCLE_A -> A -> TAIL (the blocked tail).
    // CYCLE_FREE declares nothing and must stay sortable.
    let cycle_modules = vec![
        fixture_module(CYCLE_A, CYCLE_STAGE, &[CYCLE_B]),
        fixture_module(CYCLE_B, CYCLE_STAGE, &[CYCLE_A]),
        fixture_module(CYCLE_TAIL, CYCLE_STAGE, &[CYCLE_A]),
        fixture_module(CYCLE_FREE, CYCLE_STAGE, &[]),
    ];
    let producers: Vec<&dyn Producer> = cycle_modules
        .iter()
        .map(|module| module as &dyn Producer)
        .collect();
    let nodes = build_intra_stage_dag(CYCLE_STAGE.to_string(), &producers)
        .expect("the real intra-stage builder must accept the cyclic declaration set");
    assert_eq!(
        nodes.len(),
        4,
        "the real intra-stage builder must produce one node per cyclic-stage declaration"
    );
    let node_a = nodes
        .iter()
        .find(|node| node.module_id == CYCLE_A)
        .expect("the builder must produce the CYCLE_A node");
    assert!(
        node_a
            .edges_to
            .iter()
            .any(|edge| edge.to == CYCLE_B && edge.reasons == vec![EdgeReason::ExplicitRequires]),
        "the builder must emit the A -> B edge with exactly the explicit-requires reason"
    );
    assert!(
        node_a
            .edges_to
            .iter()
            .any(|edge| edge.to == CYCLE_TAIL && edge.reasons == vec![EdgeReason::ExplicitRequires]),
        "the builder must emit the A -> TAIL edge that makes the tail blocked"
    );
    let node_b = nodes
        .iter()
        .find(|node| node.module_id == CYCLE_B)
        .expect("the builder must produce the CYCLE_B node");
    assert!(
        node_b
            .edges_to
            .iter()
            .any(|edge| edge.to == CYCLE_A && edge.reasons == vec![EdgeReason::ExplicitRequires]),
        "the builder must emit the B -> A edge that closes the cycle"
    );

    // The cycle nodes plus the downstream acyclic tail; the independent
    // CYCLE_FREE node is deliberately absent.
    let expected_blocked: BTreeSet<String> = [CYCLE_A, CYCLE_B, CYCLE_TAIL]
        .into_iter()
        .map(str::to_string)
        .collect();

    let public_blocked = match topological_sort(&nodes) {
        Ok(order) => panic!(
            "the public sorter must not return a partial successful schedule for the cyclic \
             stage graph, got {order:?}"
        ),
        Err(blocked) => blocked,
    };
    let public_blocked_set: BTreeSet<String> = public_blocked.iter().cloned().collect();
    assert_eq!(
        public_blocked_set, expected_blocked,
        "the public sorter's blocked remainder must be the cycle nodes plus the downstream \
         acyclic tail, excluding the independent sortable node"
    );
    assert_eq!(
        public_blocked,
        vec![
            CYCLE_A.to_string(),
            CYCLE_B.to_string(),
            CYCLE_TAIL.to_string(),
        ],
        "the public error Vec must list the blocked remainder in lexical order"
    );

    // exhaustive: DagValidationRequest has no Default and every field is part of this validator stimulus
    let request = DagValidationRequest {
        modules: cycle_modules,
        stage_dags: vec![StageDag {
            stage: CYCLE_STAGE.to_string(),
            nodes: nodes.clone(),
        }],
        host_ir_schema_version: semver(1, 0, 0),
        host_version: semver(0, 1, 0),
        claim_holders: fill_claim_holders(),
        access_audits: Vec::new(),
    };
    let report = validate_startup_dag(&request);
    assert!(
        report.warnings.is_empty(),
        "the cycle-only request must not emit warnings: {:?}",
        report.warnings
    );
    assert_eq!(
        report.errors.len(),
        1,
        "the cycle-only request must produce exactly one fatal diagnostic, got {:?}",
        report.errors
    );
    let cycle_diagnostic = &report.errors[0];
    assert_eq!(
        cycle_diagnostic.pass,
        DagValidationPass::CycleDetection,
        "the sole fatal diagnostic must come from the CycleDetection pass"
    );
    assert_eq!(
        cycle_diagnostic.level,
        DiagnosticLevel::Error,
        "the cycle diagnostic must be fatal, not a warning"
    );
    let validator_cycle = match &cycle_diagnostic.detail {
        SchedulerError::CyclicDependency { cycle } => cycle,
        other => panic!("the sole fatal diagnostic must be CyclicDependency, got {other:?}"),
    };
    let validator_blocked_set: BTreeSet<String> = validator_cycle.iter().cloned().collect();
    assert_eq!(
        validator_blocked_set, expected_blocked,
        "the validator CycleDetection cycle must be the same cycle-plus-tail blocked set as the \
         path model, not a simple-cycle route that drops the tail"
    );
    assert_eq!(
        validator_blocked_set, public_blocked_set,
        "both public scheduling surfaces must report the same blocked set for this input"
    );
    assert!(
        !report.is_valid(),
        "a cyclic production input must never validate as a successful schedule"
    );
}
