#![allow(missing_docs)]

//! TDD tests for TASK-105: host-built-in `PrePass::MeshAnalysis`.
//!
//! Proves that:
//! - the built-in runs on the real prepass path via
//!   `execute_prepass_with_builtins` and commits `SurfaceClassificationIR`,
//! - expected analysis outputs are produced (BottomSurface / TopSurface /
//!   Overhang / Normal classifications and an overhang region),
//! - invalid inputs (bad index buffer, vertex-index out of range) fail
//!   with a structured `PrepassExecutionError::MeshAnalysis` diagnostic,
//! - repeated invocations on the same mesh are byte-identical.
//!
//! Reference: docs/01_system_architecture.md Â§"PrePass::MeshAnalysis",
//! docs/02_ir_schemas.md Â§"IR 2 â€" SurfaceClassificationIR",
//! docs/04_host_scheduler.md Â§"Full Lifecycle" (prepass).

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use slicer_ir::{
    BoundingBox3, FacetClass, IndexedTriangleSet, MeshIR, ObjectConfig, ObjectMesh, Point3, SemVer,
    Transform3d,
};
use slicer_runtime::{
    execute_mesh_analysis, execute_prepass_with_builtins, Blackboard, CompiledModuleLive,
    ExecutionPlan, MeshAnalysisError, PrepassExecutionError, PrepassRunnerError, PrepassStageInput,
    PrepassStageOutput, PrepassStageRunner,
};

// A runner that must never be called â€" the built-in runs before any user
// prepass module, and our plans here contain no user modules.
struct UnreachableRunner;
impl PrepassStageRunner for UnreachableRunner {
    fn run_stage(
        &self,
        stage_id: &slicer_ir::StageId,
        module: &CompiledModuleLive<'_>,
        _input: PrepassStageInput<'_>,
    ) -> Result<PrepassStageOutput, PrepassRunnerError> {
        panic!(
            "prepass runner should not be invoked for this test (stage={stage_id}, module={})",
            module.module_id
        );
    }
}

// ----------------------------------------------------------------------
// Test 1 â€" built-in runs on the real prepass path
// ----------------------------------------------------------------------

#[test]
fn mesh_analysis_builtin_runs_on_real_prepass_path_and_commits_surface_classification() {
    let mesh = Arc::new(cube_like_mesh());
    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 0);
    let plan = empty_plan();

    assert!(
        blackboard.surface_classification().is_none(),
        "precondition: no surface classification yet"
    );

    execute_prepass_with_builtins(
        &plan,
        &mut blackboard,
        &UnreachableRunner,
        &Default::default(),
    )
    .expect("prepass with builtins should succeed");

    let sc = blackboard
        .surface_classification()
        .expect("built-in must commit SurfaceClassificationIR");
    assert_eq!(sc.per_object.len(), 1);
    assert!(sc.per_object.contains_key("cube"));
}

// ----------------------------------------------------------------------
// Test 2 â€" expected analysis outputs for known facet normals
// ----------------------------------------------------------------------

#[test]
fn mesh_analysis_classifies_known_facets_and_emits_overhang_region() {
    // Three triangles with well-known normals:
    //   t0: up-facing  (normal +Z)    â†’ TopSurface
    //   t1: down-facing (normal -Z)   â†’ BottomSurface
    //   t2: tilted down at ~10Â° from horizontal (strong overhang) â†’ Overhang
    //   t3: vertical side wall (normal +X) â†’ Normal
    let mesh = MeshIR {
        schema_version: sv(1, 0, 0),
        objects: vec![ObjectMesh {
            id: "probe".to_string(),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    // t0 (top, CCW viewed from +Z) â€" normal = +Z
                    p3(0.0, 0.0, 1.0),
                    p3(1.0, 0.0, 1.0),
                    p3(0.0, 1.0, 1.0),
                    // t1 (bottom, CCW viewed from -Z) â€" normal = -Z
                    p3(0.0, 0.0, 0.0),
                    p3(0.0, 1.0, 0.0),
                    p3(1.0, 0.0, 0.0),
                    // t2 (clearly-down-facing overhang, normal â‰ˆ (-0.6, 0, -0.8)):
                    //   v0=(0,0,0), v1=(0,1,0), v2=(1,0,-0.75)
                    //   u=(0,1,0), v=(1,0,-0.75), n=uÃ—v=(-0.75,0,-1), |n|=1.25
                    //   â†’ normalized nz = -0.8 â†’ overhang at ~36.87Â° from straight down.
                    p3(0.0, 0.0, 0.0),
                    p3(0.0, 1.0, 0.0),
                    p3(1.0, 0.0, -0.75),
                    // t3 (side wall, CCW viewed from +X) â€" normal = +X
                    p3(0.0, 0.0, 0.0),
                    p3(0.0, 0.0, 1.0),
                    p3(0.0, 1.0, 0.0),
                ],
                indices: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            },
            transform: identity_transform(),
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: vec![],
            paint_data: None,
            world_z_extent: None,
        }],
        build_volume: build_volume(),
    };

    let ir = execute_mesh_analysis(&mesh).expect("analysis should succeed");
    let obj = ir.per_object.get("probe").expect("probe object present");

    assert_eq!(obj.facet_classes.len(), 4, "one class per triangle");
    assert!(
        matches!(obj.facet_classes[0], FacetClass::TopSurface),
        "t0 should be TopSurface, got {:?}",
        obj.facet_classes[0]
    );
    assert!(
        matches!(obj.facet_classes[1], FacetClass::BottomSurface),
        "t1 should be BottomSurface, got {:?}",
        obj.facet_classes[1]
    );
    assert!(
        matches!(obj.facet_classes[2], FacetClass::Overhang { .. }),
        "t2 should be Overhang, got {:?}",
        obj.facet_classes[2]
    );
    assert!(
        matches!(obj.facet_classes[3], FacetClass::Normal),
        "t3 (side wall) should be Normal, got {:?}",
        obj.facet_classes[3]
    );

    assert_eq!(obj.surface_groups.len(), 1, "one group per object baseline");
    let g = &obj.surface_groups[0];
    assert_eq!(g.facet_indices.len(), 4);
    assert!(g.printable);
    assert!(g.area_mm2 > 0.0);

    assert_eq!(obj.overhang_regions.len(), 1);
    let oh = &obj.overhang_regions[0];
    assert_eq!(oh.facet_indices, vec![2]);
    assert!(oh.needs_support);
    assert!(oh.max_angle_deg >= 0.0);
}

// ----------------------------------------------------------------------
// Test 3 â€" invalid indices fail cleanly with structured diagnostics
// ----------------------------------------------------------------------

#[test]
fn mesh_analysis_rejects_index_buffer_not_multiple_of_three() {
    let mut mesh = triangle_mesh("bad");
    mesh.objects[0].mesh.indices.push(0); // 4 indices â€" not a triangle list

    let err = execute_mesh_analysis(&mesh).expect_err("must fail");
    assert!(matches!(
        err,
        MeshAnalysisError::IndicesNotMultipleOfThree { ref object_id, count: 4 } if object_id == "bad"
    ));
}

#[test]
fn mesh_analysis_rejects_out_of_range_vertex_index() {
    let mut mesh = triangle_mesh("oor");
    mesh.objects[0].mesh.indices[2] = 99;

    let err = execute_mesh_analysis(&mesh).expect_err("must fail");
    assert!(matches!(
        err,
        MeshAnalysisError::InvalidVertexIndex { ref object_id, index: 99, vertex_count: 3 } if object_id == "oor"
    ));
}

#[test]
fn mesh_analysis_builtin_surfaces_invalid_mesh_as_prepass_error() {
    let mut mesh = triangle_mesh("oor");
    mesh.objects[0].mesh.indices[2] = 77;
    let mut blackboard = Blackboard::new(Arc::new(mesh), 0);
    let plan = empty_plan();

    let err = execute_prepass_with_builtins(
        &plan,
        &mut blackboard,
        &UnreachableRunner,
        &Default::default(),
    )
    .expect_err("must surface mesh analysis failure");

    match err {
        PrepassExecutionError::MeshAnalysis {
            source: MeshAnalysisError::InvalidVertexIndex { index: 77, .. },
        } => {}
        other => panic!("expected MeshAnalysis/InvalidVertexIndex, got {other:?}"),
    }
}

// ----------------------------------------------------------------------
// Test 4 â€" determinism
// ----------------------------------------------------------------------

#[test]
fn mesh_analysis_is_deterministic_for_same_input() {
    let mesh = cube_like_mesh();

    let a = execute_mesh_analysis(&mesh).unwrap();
    let b = execute_mesh_analysis(&mesh).unwrap();
    let c = execute_mesh_analysis(&mesh).unwrap();

    assert_eq!(a, b, "run 1 vs 2 must match");
    assert_eq!(b, c, "run 2 vs 3 must match");
}

// ----------------------------------------------------------------------
// Fixtures
// ----------------------------------------------------------------------

fn empty_plan() -> ExecutionPlan {
    ExecutionPlan {
        prepass_stages: vec![],
        per_layer_stages: vec![],
        layer_finalization_stage: None,
        postpass_stages: vec![],
        global_layers: Arc::new(vec![]),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
        aggregated_region_split: BTreeMap::new(),
    }
}

fn sv(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn p3(x: f32, y: f32, z: f32) -> Point3 {
    Point3 { x, y, z }
}

fn identity_transform() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    }
}

fn build_volume() -> BoundingBox3 {
    BoundingBox3 {
        min: Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        max: Point3 {
            x: 200.0,
            y: 200.0,
            z: 200.0,
        },
    }
}

/// A tiny mesh with a single up-facing triangle â€" enough to drive the
/// built-in without inventing a full solid.
fn triangle_mesh(id: &str) -> MeshIR {
    MeshIR {
        schema_version: sv(1, 0, 0),
        objects: vec![ObjectMesh {
            id: id.to_string(),
            mesh: IndexedTriangleSet {
                vertices: vec![p3(0.0, 0.0, 0.0), p3(1.0, 0.0, 0.0), p3(0.0, 1.0, 0.0)],
                indices: vec![0, 1, 2],
            },
            transform: identity_transform(),
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: vec![],
            paint_data: None,
            world_z_extent: None,
        }],
        build_volume: build_volume(),
    }
}

/// Minimal cube-shell fixture: two opposed triangles (top and bottom).
/// Enough to exercise classification + commit without modeling a full
/// manifold solid.
fn cube_like_mesh() -> MeshIR {
    MeshIR {
        schema_version: sv(1, 0, 0),
        objects: vec![ObjectMesh {
            id: "cube".to_string(),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    // top (normal +Z)
                    p3(0.0, 0.0, 1.0),
                    p3(1.0, 0.0, 1.0),
                    p3(0.0, 1.0, 1.0),
                    // bottom (normal -Z)
                    p3(0.0, 0.0, 0.0),
                    p3(0.0, 1.0, 0.0),
                    p3(1.0, 0.0, 0.0),
                ],
                indices: vec![0, 1, 2, 3, 4, 5],
            },
            transform: identity_transform(),
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: vec![],
            paint_data: None,
            world_z_extent: None,
        }],
        build_volume: build_volume(),
    }
}
