#![allow(missing_docs)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use slicer_host::{build_intra_stage_dag, LoadedModule, LoadedModuleBuilder, ModuleNode};
use slicer_ir::SemVer;

#[test]
fn requested_stage_filters_the_node_set() {
    let stage = String::from("Layer::Perimeters");
    let nodes = build_intra_stage_dag(
        stage.clone(),
        &[
            loaded_module(
                "com.example.alpha",
                &stage,
                &["SliceIR.regions"],
                &["PerimeterIR.regions"],
                &[],
            ),
            loaded_module(
                "com.example.other-stage",
                "Layer::Support",
                &["SliceIR.regions"],
                &["SupportIR.regions"],
                &[],
            ),
            loaded_module(
                "com.example.beta",
                &stage,
                &["PerimeterIR.regions"],
                &["PerimeterIR.regions.walls"],
                &[],
            ),
        ],
    )
    .expect("DAG construction should succeed for same-stage node selection");

    let by_id = nodes_by_id(&nodes);
    assert_eq!(
        by_id.len(),
        2,
        "only modules in the requested stage become nodes"
    );
    assert!(by_id.contains_key("com.example.alpha"));
    assert!(by_id.contains_key("com.example.beta"));
    assert!(!by_id.contains_key("com.example.other-stage"));
}

#[test]
fn read_after_write_access_derives_an_edge_from_writer_to_reader() {
    let stage = String::from("Layer::PerimetersPostProcess");
    let nodes = build_intra_stage_dag(
        stage.clone(),
        &[
            loaded_module(
                "com.example.writer",
                &stage,
                &[],
                &["PerimeterIR.regions.walls.feature_flags"],
                &[],
            ),
            loaded_module(
                "com.example.reader",
                &stage,
                &["PerimeterIR.regions.walls.feature_flags"],
                &["PerimeterIR.regions.walls"],
                &[],
            ),
        ],
    )
    .expect("DAG construction should succeed for auto-derived edges");

    let by_id = nodes_by_id(&nodes);
    let writer_edges: Vec<&str> = by_id["com.example.writer"]
        .edges_to
        .iter()
        .map(|e| e.to.as_str())
        .collect();
    assert_eq!(writer_edges, vec!["com.example.reader"]);
    assert!(by_id["com.example.reader"].edges_to.is_empty());
}

#[test]
fn same_stage_requires_modules_derives_an_explicit_dependency_edge() {
    let stage = String::from("Layer::Infill");
    let nodes = build_intra_stage_dag(
        stage.clone(),
        &[
            loaded_module(
                "com.example.base-infill",
                &stage,
                &["SliceIR.regions.infill_areas"],
                &["InfillIR.regions.sparse_infill"],
                &[],
            ),
            loaded_module(
                "com.example.decorator",
                &stage,
                &["InfillIR.regions.sparse_infill"],
                &["InfillIR.regions.sparse_infill"],
                &["com.example.base-infill"],
            ),
        ],
    )
    .expect("DAG construction should succeed for explicit requires edges");

    let by_id = nodes_by_id(&nodes);
    assert!(
        by_id["com.example.base-infill"]
            .edges_to
            .iter()
            .any(|e| e.to == "com.example.decorator"),
        "same-stage requires_modules should add an A -> B edge"
    );
}

#[test]
fn cross_stage_requires_modules_do_not_create_intra_stage_edges() {
    let stage = String::from("Layer::Support");
    let nodes = build_intra_stage_dag(
        stage.clone(),
        &[
            loaded_module(
                "com.example.support-base",
                &stage,
                &["SliceIR.regions"],
                &["SupportIR.regions"],
                &["com.example.prepass-planner"],
            ),
            loaded_module(
                "com.example.support-isolated",
                &stage,
                &[],
                &["SupportIR.regions.interface"],
                &[],
            ),
            loaded_module(
                "com.example.prepass-planner",
                "PrePass::LayerPlanning",
                &["MeshIR"],
                &["LayerPlanIR"],
                &[],
            ),
        ],
    )
    .expect("DAG construction should ignore cross-stage explicit requires edges");

    let by_id = nodes_by_id(&nodes);
    assert!(by_id["com.example.support-base"].edges_to.is_empty());
    assert!(by_id["com.example.support-isolated"].edges_to.is_empty());
}

#[test]
fn modules_without_dependencies_remain_as_isolated_nodes() {
    let stage = String::from("Layer::SlicePostProcess");
    let nodes = build_intra_stage_dag(
        stage.clone(),
        &[
            loaded_module(
                "com.example.annotator",
                &stage,
                &["SliceIR.regions"],
                &["SliceIR.regions.boundary_paint"],
                &[],
            ),
            loaded_module("com.example.noop", &stage, &[], &[], &[]),
        ],
    )
    .expect("DAG construction should keep isolated modules in the graph");

    let by_id = nodes_by_id(&nodes);
    assert!(by_id.contains_key("com.example.annotator"));
    assert!(by_id.contains_key("com.example.noop"));
    assert!(by_id["com.example.noop"].edges_to.is_empty());
}

#[test]
fn node_identity_is_loaded_module_id_even_when_other_fields_match() {
    let stage = String::from("Layer::Perimeters");
    let nodes = build_intra_stage_dag(
        stage.clone(),
        &[
            loaded_module(
                "com.example.variant-a",
                &stage,
                &["SliceIR.regions"],
                &["PerimeterIR.regions.walls"],
                &[],
            ),
            loaded_module(
                "com.example.variant-b",
                &stage,
                &["SliceIR.regions"],
                &["PerimeterIR.regions.walls"],
                &[],
            ),
        ],
    )
    .expect("DAG construction should preserve deterministic node identity from LoadedModule.id");

    let by_id = nodes_by_id(&nodes);
    assert_eq!(by_id.len(), 2);
    assert!(by_id.contains_key("com.example.variant-a"));
    assert!(by_id.contains_key("com.example.variant-b"));
}

fn nodes_by_id(nodes: &[ModuleNode]) -> BTreeMap<&str, &ModuleNode> {
    nodes
        .iter()
        .map(|node| (node.module_id.as_str(), node))
        .collect()
}

fn loaded_module(
    id: &str,
    stage: &str,
    ir_reads: &[&str],
    ir_writes: &[&str],
    requires_modules: &[&str],
) -> LoadedModule {
    LoadedModuleBuilder::new(
        id,
        semver(1, 0, 0),
        stage,
        "slicer:world-layer@1.0.0",
        PathBuf::from(format!("fixtures/{id}.wasm")),
    )
    .ir_reads(strings(ir_reads))
    .ir_writes(strings(ir_writes))
    .requires_modules(strings(requires_modules))
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(2, 0, 0))
    .layer_parallel_safe(true)
    .build()
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
