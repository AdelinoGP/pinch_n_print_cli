//! TDD: host validation and application of `set-entity-order` proposals.
//!
//! Covers TASK-152g (packet 32 — `layer-collection-builder` WIT surface).
//! All tests exercise `apply_entity_order_proposal` directly; the live
//! dispatch path is covered by `path_ordering_tdd` (host fallback) and
//! by packet 33's module-side tests once `path-optimization-default`
//! migrates to call `set_entity_order`.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use slicer_host::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_host::{
    apply_entity_order_proposal, execute_per_layer, project_ordered_entities, Blackboard,
    CompiledModule, CompiledStage, ExecutionPlan, IrAccessMask, LayerArena, LoadedModule,
    WasmEngine, WasmRuntimeDispatcher, HOST_GET_ORDERED_ENTITIES_TOTAL_CALLS,
};
use slicer_ir::{
    BoundingBox3, ExPolygon, ExtrusionPath3D, ExtrusionRole, GlobalLayer, LayerCollectionIR,
    LoopType, MeshIR, PerimeterIR, PerimeterRegion, Point2, Point3, Point3WithWidth, Polygon,
    PrintEntity, RegionKey, SemVer, WallBoundaryType, WallFeatureFlags, WallLoop, WidthProfile,
};

// ── Fixtures ─────────────────────────────────────────────────────────────────

fn semver() -> SemVer {
    SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    }
}

fn pt(x: f32, y: f32, z: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z,
        width: 0.4,
        flow_factor: 1.0,
    }
}

fn entity_with_points(points: Vec<Point3WithWidth>, original_idx: u32) -> PrintEntity {
    let role = ExtrusionRole::SparseInfill;
    PrintEntity {
        entity_id: (original_idx as u64) + 1,
        path: ExtrusionPath3D {
            points,
            role: role.clone(),
            speed_factor: 1.0,
        },
        role,
        region_key: RegionKey {
            global_layer_index: 0,
            object_id: "obj".to_string(),
            region_id: 0,
        },
        topo_order: original_idx,
    }
}

fn three_entity_arena() -> LayerArena {
    // Raw start-x: [30.0, 0.0, 10.0] — the same fixture shape as the
    // packet.spec.md acceptance criteria.
    let entities = vec![
        entity_with_points(vec![pt(30.0, 0.0, 0.2)], 0),
        entity_with_points(vec![pt(0.0, 0.0, 0.2)], 1),
        entity_with_points(vec![pt(10.0, 0.0, 0.2)], 2),
    ];
    let mut arena = LayerArena::new();
    arena.set_layer_collection(LayerCollectionIR {
        schema_version: semver(),
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: entities,
        tool_changes: Vec::new(),
        z_hops: Vec::new(),
        annotations: Vec::new(),
        retracts: Vec::new(),
        travel_moves: Vec::new(),
    });
    arena
}

fn single_entity_arena_with_path(start: Point3WithWidth, end: Point3WithWidth) -> LayerArena {
    let entity = entity_with_points(vec![start, end], 0);
    let mut arena = LayerArena::new();
    arena.set_layer_collection(LayerCollectionIR {
        schema_version: semver(),
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: vec![entity],
        tool_changes: Vec::new(),
        z_hops: Vec::new(),
        annotations: Vec::new(),
        retracts: Vec::new(),
        travel_moves: Vec::new(),
    });
    arena
}

fn ordered_start_xs(arena: &LayerArena) -> Vec<f32> {
    arena
        .layer_collection()
        .expect("layer_collection must be staged")
        .ordered_entities
        .iter()
        .map(|e| e.path.points[0].x)
        .collect()
}

fn ordered_topo_orders(arena: &LayerArena) -> Vec<u32> {
    arena
        .layer_collection()
        .expect("layer_collection must be staged")
        .ordered_entities
        .iter()
        .map(|e| e.topo_order)
        .collect()
}

// ── Positive path: permutation applied ───────────────────────────────────────

#[test]
fn valid_permutation_is_applied_to_ordered_entities() {
    let mut arena = three_entity_arena();

    // Raw x is [30.0, 0.0, 10.0]. Proposal [(2,false),(0,false),(1,false)]
    // moves slot-2 (x=10) → first, slot-0 (x=30) → second, slot-1 (x=0) → third.
    let proposal: Vec<(u32, bool)> = vec![(2, false), (0, false), (1, false)];

    apply_entity_order_proposal(&mut arena, &proposal)
        .expect("valid proposal must apply without error");

    assert_eq!(
        ordered_start_xs(&arena),
        vec![10.0, 30.0, 0.0],
        "permutation must yield x=[10.0, 30.0, 0.0]"
    );
    assert_eq!(
        ordered_topo_orders(&arena),
        vec![0, 1, 2],
        "topo_order must be reassigned to post-permutation 0-based slot index"
    );
}

// ── Reversal flag: per-entity Vec::reverse() on path.points ──────────────────

#[test]
fn reversal_flag_reverses_path_points_in_place() {
    let mut arena = single_entity_arena_with_path(pt(0.0, 0.0, 0.2), pt(5.0, 0.0, 0.2));

    let proposal: Vec<(u32, bool)> = vec![(0, true)];

    apply_entity_order_proposal(&mut arena, &proposal)
        .expect("single-entity reversal proposal must apply");

    let lc = arena.layer_collection().expect("layer_collection staged");
    let entity = &lc.ordered_entities[0];
    assert_eq!(
        entity.path.points.first().expect("nonempty points").x,
        5.0,
        "after reversal first point x must be 5.0"
    );
    assert_eq!(
        entity.path.points.last().expect("nonempty points").x,
        0.0,
        "after reversal last point x must be 0.0"
    );
}

// ── Negative: duplicate index in proposal ────────────────────────────────────

#[test]
fn duplicate_index_is_rejected_with_fatal_diagnostic() {
    let mut arena = three_entity_arena();

    let proposal: Vec<(u32, bool)> = vec![(0, false), (0, false), (1, false)];

    let err = apply_entity_order_proposal(&mut arena, &proposal)
        .expect_err("duplicate index must produce an Err");

    assert!(
        err.contains("set-entity-order: duplicate index 0"),
        "diagnostic must mention the duplicate index 0; got: {err}"
    );
}

// ── Negative: out-of-range index ─────────────────────────────────────────────

#[test]
fn out_of_range_index_is_rejected_with_fatal_diagnostic() {
    let mut arena = three_entity_arena();

    let proposal: Vec<(u32, bool)> = vec![(99, false), (0, false), (1, false)];

    let err = apply_entity_order_proposal(&mut arena, &proposal)
        .expect_err("out-of-range index must produce an Err");

    assert!(
        err.contains("set-entity-order: index 99 out of range [0, 3)"),
        "diagnostic must name the offending index and the valid range; got: {err}"
    );
}

// ── Negative: wrong-length proposal ──────────────────────────────────────────

#[test]
fn wrong_length_proposal_is_rejected_with_fatal_diagnostic() {
    let mut arena = three_entity_arena();

    let proposal: Vec<(u32, bool)> = vec![(0, false), (1, false)];

    let err = apply_entity_order_proposal(&mut arena, &proposal)
        .expect_err("wrong-length proposal must produce an Err");

    assert!(
        err.contains("set-entity-order: expected 3 indices, got 2"),
        "diagnostic must name expected and received counts; got: {err}"
    );
}

// ── Negative: arena has no LayerCollectionIR staged ─────────────────────────

#[test]
fn missing_layer_collection_is_rejected() {
    let mut arena = LayerArena::new();
    let proposal: Vec<(u32, bool)> = vec![(0, false)];

    let err = apply_entity_order_proposal(&mut arena, &proposal)
        .expect_err("absent layer_collection must produce an Err");

    assert!(
        err.contains("set-entity-order: no LayerCollectionIR staged on arena"),
        "diagnostic must explain the missing staged collection; got: {err}"
    );
}

// ── Read side: project_ordered_entities snapshot projection ──────────────────

fn entity_with_points_and_role(
    points: Vec<Point3WithWidth>,
    original_idx: u32,
    role: ExtrusionRole,
) -> PrintEntity {
    PrintEntity {
        entity_id: (original_idx as u64) + 1,
        path: ExtrusionPath3D {
            points,
            role: role.clone(),
            speed_factor: 1.0,
        },
        role,
        region_key: RegionKey {
            global_layer_index: 0,
            object_id: "obj".to_string(),
            region_id: 0,
        },
        topo_order: original_idx,
    }
}

#[test]
fn get_ordered_entities_projects_staged_entities_in_index_order() {
    // Three-entity fixture with (start-x, role) tuples:
    //   [(30.0, SparseInfill), (0.0, BridgeInfill), (10.0, SparseInfill)].
    // All entities share object_id = "obj".
    let entities = vec![
        entity_with_points_and_role(vec![pt(30.0, 0.0, 0.2)], 0, ExtrusionRole::SparseInfill),
        entity_with_points_and_role(vec![pt(0.0, 0.0, 0.2)], 1, ExtrusionRole::BridgeInfill),
        entity_with_points_and_role(vec![pt(10.0, 0.0, 0.2)], 2, ExtrusionRole::SparseInfill),
    ];
    let mut arena = LayerArena::new();
    arena.set_layer_collection(LayerCollectionIR {
        schema_version: semver(),
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: entities,
        tool_changes: Vec::new(),
        z_hops: Vec::new(),
        annotations: Vec::new(),
        retracts: Vec::new(),
        travel_moves: Vec::new(),
    });

    let views = project_ordered_entities(&arena);

    assert_eq!(
        views.len(),
        3,
        "projection must include all 3 staged entities"
    );

    assert_eq!(
        views[0].original_index, 0,
        "views[0].original_index must be 0"
    );
    assert_eq!(
        views[1].original_index, 1,
        "views[1].original_index must be 1"
    );
    assert_eq!(
        views[2].original_index, 2,
        "views[2].original_index must be 2"
    );

    let xs: Vec<f32> = views.iter().map(|v| v.start_point.x).collect();
    assert_eq!(
        xs,
        vec![30.0, 0.0, 10.0],
        "projection must preserve ordered_entities start-x order"
    );

    let roles: Vec<ExtrusionRole> = views.iter().map(|v| v.role.clone()).collect();
    assert_eq!(
        roles,
        vec![
            ExtrusionRole::SparseInfill,
            ExtrusionRole::BridgeInfill,
            ExtrusionRole::SparseInfill,
        ],
        "projection must preserve ordered_entities role order"
    );

    for (i, v) in views.iter().enumerate() {
        assert_eq!(
            v.region_key.object_id, "obj",
            "views[{i}].region_key.object_id must be carried through verbatim"
        );
    }
}

#[test]
fn get_ordered_entities_carries_endpoints_and_point_count() {
    let arena = single_entity_arena_with_path(pt(0.0, 0.0, 0.2), pt(5.0, 0.0, 0.2));

    let views = project_ordered_entities(&arena);

    assert_eq!(
        views.len(),
        1,
        "single-entity fixture must project to 1 view"
    );
    let v = &views[0];
    assert_eq!(
        v.start_point.x, 0.0,
        "start_point.x must mirror path.points.first().x"
    );
    assert_eq!(
        v.end_point.x, 5.0,
        "end_point.x must mirror path.points.last().x"
    );
    assert_eq!(v.point_count, 2, "point_count must equal path.points.len()");
}

#[test]
fn get_ordered_entities_returns_empty_when_no_layer_collection_is_staged() {
    let arena = LayerArena::new();

    let views = project_ordered_entities(&arena);

    assert!(
        views.is_empty(),
        "projection on an arena with no LayerCollectionIR staged must return an empty Vec (read accessor is total)"
    );
}

// ── Atomicity: malformed proposal leaves ordered_entities unchanged ──────────

#[test]
fn malformed_proposal_leaves_ordered_entities_unchanged() {
    // Snapshot the pre-call state.
    let mut arena = three_entity_arena();
    let before_xs = ordered_start_xs(&arena);
    let before_topo = ordered_topo_orders(&arena);

    // Try each malformed proposal flavor and confirm the arena is untouched.
    for proposal in [
        vec![(0u32, false), (0, false), (1, false)],  // duplicate
        vec![(99u32, false), (0, false), (1, false)], // out of range
        vec![(0u32, false), (1, false)],              // wrong length
    ] {
        let err = apply_entity_order_proposal(&mut arena, &proposal)
            .expect_err("malformed proposal must produce an Err");
        assert!(
            err.starts_with("set-entity-order: "),
            "diagnostic must be prefixed with 'set-entity-order: '; got: {err}"
        );
        assert_eq!(
            ordered_start_xs(&arena),
            before_xs,
            "ordered_entities x sequence must be unchanged after rejected proposal"
        );
        assert_eq!(
            ordered_topo_orders(&arena),
            before_topo,
            "ordered_entities topo_order sequence must be unchanged after rejected proposal"
        );
    }
}

// ── Macro-call-once contract: multi-read guest exercises 5 SDK reads ─────────

const MULTI_READ_GUEST_COMPONENT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test-guests/path-optimization-multi-read.component.wasm"
);

fn semver_v(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn empty_mesh_ir() -> Arc<MeshIR> {
    Arc::new(MeshIR {
        schema_version: semver_v(1, 0, 0),
        objects: Vec::new(),
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
        },
    })
}

fn make_loaded_module(id: &str, stage: &str) -> LoadedModule {
    LoadedModule {
        id: id.to_string(),
        version: semver_v(1, 0, 0),
        stage: stage.to_string(),
        wit_world: "slicer:world-layer@1.0.0".to_string(),
        ir_reads: Vec::new(),
        ir_writes: Vec::new(),
        claims: Vec::new(),
        requires_claims: Vec::new(),
        incompatible_with: Vec::new(),
        requires_modules: Vec::new(),
        min_host_version: semver_v(0, 1, 0),
        min_ir_schema: semver_v(1, 0, 0),
        max_ir_schema: semver_v(2, 0, 0),
        config_schema: Default::default(),
        overridable_per_region: Vec::new(),
        overridable_per_layer: Vec::new(),
        layer_parallel_safe: false,
        wasm_path: PathBuf::from("/dev/null"),
        placeholder_wasm: false,
    }
}

fn make_module(
    id: &str,
    stage: &str,
    component: Arc<slicer_host::WasmComponent>,
) -> CompiledModule {
    let loaded = make_loaded_module(id, stage);
    let pool = Arc::new(
        build_wasm_instance_pool(
            &loaded,
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("build instance pool"),
    );
    CompiledModule {
        module_id: id.to_string(),
        instance_pool: pool,
        ir_read_mask: IrAccessMask { paths: Vec::new() },
        ir_write_mask: IrAccessMask { paths: Vec::new() },
        config_view: Arc::new(slicer_ir::ConfigView::new()),
        claims: Vec::new(),
        wasm_component: Some(component),
    }
}

fn make_wall_loop_at(perimeter_index: u32, x: f32) -> WallLoop {
    let points = vec![
        Point3WithWidth {
            x,
            y: 0.0,
            z: 0.2,
            width: 0.4,
            flow_factor: 1.0,
        },
        Point3WithWidth {
            x: x + 1.0,
            y: 0.0,
            z: 0.2,
            width: 0.4,
            flow_factor: 1.0,
        },
    ];
    WallLoop {
        perimeter_index,
        loop_type: LoopType::Outer,
        path: ExtrusionPath3D {
            points: points.clone(),
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        width_profile: WidthProfile {
            widths: points.iter().map(|p| p.width).collect(),
        },
        feature_flags: points
            .iter()
            .map(|_| WallFeatureFlags {
                tool_index: None,
                fuzzy_skin: false,
                is_bridge: false,
                is_thin_wall: false,
                skip_ironing: false,
                custom: HashMap::new(),
            })
            .collect(),
        boundary_type: WallBoundaryType::Interior,
    }
}

fn make_three_region_perimeter(layer_index: u32) -> PerimeterIR {
    let regions = (0..3u32)
        .map(|i| PerimeterRegion {
            object_id: format!("obj{}", i),
            region_id: i as u64,
            walls: vec![make_wall_loop_at(i, i as f32 * 10.0)],
            infill_areas: vec![ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2 { x: 0, y: 0 },
                        Point2 { x: 1, y: 0 },
                        Point2 { x: 1, y: 1 },
                    ],
                },
                holes: Vec::new(),
            }],
            seam_candidates: Vec::new(),
            resolved_seam: None,
        })
        .collect();
    PerimeterIR {
        schema_version: semver_v(1, 0, 0),
        global_layer_index: layer_index,
        regions,
    }
}

struct PerimeterSeedingRunner<'a> {
    inner: &'a WasmRuntimeDispatcher,
    perimeter: std::sync::Mutex<Option<PerimeterIR>>,
}

impl<'a> slicer_host::LayerStageRunner for PerimeterSeedingRunner<'a> {
    fn run_stage(
        &self,
        stage_id: &slicer_ir::StageId,
        layer: &GlobalLayer,
        module: &CompiledModule,
        blackboard: &Blackboard,
        arena: &mut LayerArena,
    ) -> Result<
        (slicer_host::LayerStageOutput, Vec<String>, Vec<String>),
        slicer_host::LayerStageError,
    > {
        if stage_id == "Layer::Perimeters" && arena.perimeter().is_none() {
            if let Some(perimeter) = self.perimeter.lock().expect("lock seed perimeter").take() {
                arena
                    .set_perimeter(perimeter)
                    .expect("seed perimeter into arena");
                return Ok((
                    slicer_host::LayerStageOutput::Success,
                    Vec::new(),
                    Vec::new(),
                ));
            }
        }
        slicer_host::LayerStageRunner::run_stage(
            self.inner, stage_id, layer, module, blackboard, arena,
        )
    }
}

#[test]
fn macro_drain_invokes_host_get_ordered_entities_exactly_once() {
    let component_path = PathBuf::from(MULTI_READ_GUEST_COMPONENT);
    if !component_path.exists() {
        eprintln!(
            "skipping macro_drain_invokes_host_get_ordered_entities_exactly_once: \
             multi-read guest component missing at {}",
            component_path.display()
        );
        return;
    }

    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let bytes = std::fs::read(&component_path).expect("read multi-read guest component");
    let component = Arc::new(
        engine
            .compile_component(&bytes)
            .expect("compile multi-read guest component"),
    );

    // Reset the cross-call observation counter before exercising the layer.
    HOST_GET_ORDERED_ENTITIES_TOTAL_CALLS.store(0, Ordering::SeqCst);

    let seeded_perimeter = make_three_region_perimeter(0);

    let plan = ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: vec![
            CompiledStage {
                stage_id: "Layer::Perimeters".to_string(),
                modules: vec![make_module(
                    "com.test.multi-read-seed",
                    "Layer::Perimeters",
                    Arc::clone(&component),
                )],
            },
            CompiledStage {
                stage_id: "Layer::PathOptimization".to_string(),
                modules: vec![make_module(
                    "com.test.multi-read-pathopt",
                    "Layer::PathOptimization",
                    component,
                )],
            },
        ],
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(vec![GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: Vec::new(),
            has_nonplanar: false,
            is_sync_layer: false,
        }]),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
    };
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let runner = PerimeterSeedingRunner {
        inner: &dispatcher,
        perimeter: std::sync::Mutex::new(Some(seeded_perimeter)),
    };

    let layers = execute_per_layer(&plan, &blackboard, &runner).expect("execute per-layer plan");
    assert_eq!(
        layers.len(),
        1,
        "expected exactly one finalized layer from execute_per_layer"
    );
    assert_eq!(
        layers[0].ordered_entities.len(),
        3,
        "host fallback must produce 3 ordered entities (one per seeded region)"
    );

    let total_calls = HOST_GET_ORDERED_ENTITIES_TOTAL_CALLS.load(Ordering::SeqCst);
    assert_eq!(
        total_calls, 1,
        "macro must call host get-ordered-entities exactly once per run-path-optimization \
         dispatch even though the guest body called the SDK accessor 5 times; got {}",
        total_calls
    );
}
