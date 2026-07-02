//! TDD (packet 107, AC-2): wiring `sliced_region_to_data`'s `overhang_areas`
//! populator to `SurfaceClassificationIR.overhang_quartile_polygons`.
//!
//! Prior to this packet the populator hardcoded `overhang_areas: Vec::new()`
//! (see `overhang_areas_empty_until_p106_tdd.rs`, whose default-constructed-
//! view assertion is unaffected by this test). This test proves the wiring
//! end-to-end: slice a real overhang-ramp mesh through
//! `execute_prepass_with_builtins` (host built-ins, including
//! `PrePass::OverhangAnnotation` from packet 106) to obtain a genuine
//! `SurfaceClassificationIR` with non-empty `overhang_quartile_polygons`,
//! then feed a `SlicedRegion` covering the ramp's footprint through
//! `sliced_region_to_data` and assert the resulting `SliceRegionData.
//! overhang_areas` (and `overhang_quartile_polygons`) are non-empty.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{
    BoundingBox3, ExPolygon, GlobalLayer, IndexedTriangleSet, LayerPlanIR, MeshIR, ObjectLayerRef,
    ObjectMesh, Point2, Point3, Polygon, PrepassRunnerError, SemVer, SlicedRegion, StageId,
    Transform3d,
};
use slicer_runtime::{
    build_wasm_instance_pool, execute_prepass_with_builtins, Blackboard, CompiledModule,
    CompiledModuleBuilder, CompiledModuleLive, CompiledStage, ExecutionPlan, LoadedModuleBuilder,
    PrepassStageInput, PrepassStageOutput, PrepassStageRunner, WasmArtifactMetadata,
};
use slicer_wasm_host::host::sliced_region_to_data;

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn identity_transform() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    }
}

/// Axis-aligned box triangle soup (12 triangles). Mirrors
/// `prepass_overhang_annotation_stage_order_tdd.rs::box_triangles`.
fn box_triangles(
    base_index: u32,
    (x0, y0, z0): (f32, f32, f32),
    (x1, y1, z1): (f32, f32, f32),
) -> (Vec<Point3>, Vec<u32>) {
    let vertices = vec![
        Point3 {
            x: x0,
            y: y0,
            z: z0,
        },
        Point3 {
            x: x1,
            y: y0,
            z: z0,
        },
        Point3 {
            x: x1,
            y: y1,
            z: z0,
        },
        Point3 {
            x: x0,
            y: y1,
            z: z0,
        },
        Point3 {
            x: x0,
            y: y0,
            z: z1,
        },
        Point3 {
            x: x1,
            y: y0,
            z: z1,
        },
        Point3 {
            x: x1,
            y: y1,
            z: z1,
        },
        Point3 {
            x: x0,
            y: y1,
            z: z1,
        },
    ];
    let b = base_index;
    #[rustfmt::skip]
    let indices = vec![
        b, b + 1, b + 2,   b, b + 2, b + 3,
        b + 4, b + 5, b + 6,   b + 4, b + 6, b + 7,
        b, b + 1, b + 5,   b, b + 5, b + 4,
        b + 1, b + 2, b + 6,   b + 1, b + 6, b + 5,
        b + 2, b + 3, b + 7,   b + 2, b + 7, b + 6,
        b + 3, b, b + 4,   b + 3, b + 4, b + 7,
    ];
    (vertices, indices)
}

/// Two 10x10x1mm boxes stacked in Z, the upper laterally offset by 5mm so its
/// footprint at z=1.5 is not fully supported by the lower box's footprint at
/// z=0.5 — produces a real overhang region. Identical geometry to
/// `prepass_overhang_annotation_stage_order_tdd.rs::overhang_ramp_mesh`.
fn overhang_ramp_mesh() -> MeshIR {
    let (mut vertices, mut indices) = box_triangles(0, (0.0, 0.0, 0.0), (10.0, 10.0, 1.0));
    let (v2, i2) = box_triangles(vertices.len() as u32, (5.0, 0.0, 1.0), (15.0, 10.0, 2.0));
    vertices.extend(v2);
    indices.extend(i2);

    MeshIR {
        objects: vec![ObjectMesh {
            id: String::from("ramp"),
            mesh: IndexedTriangleSet { vertices, indices },
            transform: identity_transform(),
            ..Default::default()
        }],
        build_volume: BoundingBox3 {
            min: Point3::default(),
            max: Point3 {
                x: 200.0,
                y: 200.0,
                z: 200.0,
            },
        },
        ..Default::default()
    }
}

fn two_global_layers() -> Vec<GlobalLayer> {
    vec![
        GlobalLayer {
            index: 0,
            z: 0.5,
            active_regions: vec![],
            has_nonplanar: false,
            is_sync_layer: false,
        },
        GlobalLayer {
            index: 1,
            z: 1.5,
            active_regions: vec![],
            has_nonplanar: false,
            is_sync_layer: false,
        },
    ]
}

fn compiled_stub_module(stage_id: &str, module_id: &str) -> CompiledModule {
    let loaded = LoadedModuleBuilder::new(
        module_id,
        semver(0, 1, 0),
        stage_id,
        "slicer:world-prepass@1.0.0",
        PathBuf::from(format!("fixtures/{module_id}.wasm")),
    )
    .claims(vec!["layer-planner".to_string()])
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(2, 0, 0))
    .build();
    let _pool = Arc::new(
        build_wasm_instance_pool(
            loaded.id(),
            loaded.stage(),
            loaded.layer_parallel_safe(),
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("fixture pool must build"),
    );
    CompiledModuleBuilder::new(loaded.id().to_string()).build()
}

/// Stub runner that returns a fixed `LayerPlanIR` for `PrePass::LayerPlanning`.
/// Mirrors `prepass_overhang_annotation_stage_order_tdd.rs::LayerPlanningStubRunner`.
struct LayerPlanningStubRunner {
    mesh: MeshIR,
    global_layers: Vec<GlobalLayer>,
}

impl PrepassStageRunner for LayerPlanningStubRunner {
    fn run_stage(
        &self,
        stage_id: &StageId,
        _module: &CompiledModuleLive<'_>,
        _input: PrepassStageInput<'_>,
    ) -> Result<PrepassStageOutput, PrepassRunnerError> {
        assert_eq!(stage_id, "PrePass::LayerPlanning");
        let mut object_participation = HashMap::new();
        for obj in &self.mesh.objects {
            object_participation.insert(
                obj.id.clone(),
                vec![
                    ObjectLayerRef {
                        local_layer_index: 0,
                        global_layer_index: 0,
                        effective_layer_height: 1.0,
                    },
                    ObjectLayerRef {
                        local_layer_index: 1,
                        global_layer_index: 1,
                        effective_layer_height: 1.0,
                    },
                ],
            );
        }
        Ok(PrepassStageOutput::LayerPlan(Arc::new(LayerPlanIR {
            global_layers: self.global_layers.clone(),
            object_participation,
            ..Default::default()
        })))
    }
}

/// Axis-aligned square `ExPolygon` at the given mm corners, generously sized
/// to bbox-overlap whichever layer's overhang footprint the annotation
/// pipeline produces (this test does not depend on the exact overhang
/// geometry, only that the wiring surfaces *some* non-empty result).
fn covering_square(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(min_x, min_y),
                Point2::from_mm(max_x, min_y),
                Point2::from_mm(max_x, max_y),
                Point2::from_mm(min_x, max_y),
            ],
        },
        holes: Vec::new(),
    }
}

/// Bounding box (min_x, min_y, max_x, max_y) in scaled integer units (1 unit
/// = 100 nm) over every contour/hole vertex of the given `slicer_ir::ExPolygon`
/// slice.
fn ir_expolygons_bbox(polys: &[ExPolygon]) -> Option<(i64, i64, i64, i64)> {
    let mut min_x = i64::MAX;
    let mut min_y = i64::MAX;
    let mut max_x = i64::MIN;
    let mut max_y = i64::MIN;
    let mut any = false;
    for poly in polys {
        for p in poly
            .contour
            .points
            .iter()
            .chain(poly.holes.iter().flat_map(|h| h.points.iter()))
        {
            any = true;
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        }
    }
    any.then_some((min_x, min_y, max_x, max_y))
}

/// Same as [`ir_expolygons_bbox`] but over the WIT-side `ExPolygon` type
/// returned in `SliceRegionData` (identical field shape, distinct Rust type
/// generated by `bindgen!`).
fn wit_expolygons_bbox(
    polys: &[slicer_wasm_host::host::layer::slicer::types::geometry::ExPolygon],
) -> Option<(i64, i64, i64, i64)> {
    let mut min_x = i64::MAX;
    let mut min_y = i64::MAX;
    let mut max_x = i64::MIN;
    let mut max_y = i64::MIN;
    let mut any = false;
    for poly in polys {
        for p in poly
            .contour
            .points
            .iter()
            .chain(poly.holes.iter().flat_map(|h| h.points.iter()))
        {
            any = true;
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        }
    }
    any.then_some((min_x, min_y, max_x, max_y))
}

#[test]
fn overhang_areas_non_empty_on_layer_with_overhang_facets() {
    let mesh = overhang_ramp_mesh();
    let mesh_arc = Arc::new(mesh.clone());
    let mut blackboard = Blackboard::new(Arc::clone(&mesh_arc), 0);

    let plan = ExecutionPlan {
        prepass_stages: vec![CompiledStage {
            stage_id: String::from("PrePass::LayerPlanning"),
            modules: vec![compiled_stub_module(
                "PrePass::LayerPlanning",
                "com.test.layer-planning-stub",
            )],
        }],
        per_layer_stages: vec![],
        layer_finalization_stage: None,
        postpass_stages: vec![],
        global_layers: Arc::new(vec![]),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
        aggregated_region_split: Default::default(),
    };

    let runner = LayerPlanningStubRunner {
        mesh,
        global_layers: two_global_layers(),
    };
    let wasm_handles = HashMap::new();

    // Downstream builtins may error on this intentionally-tiny fixture; only
    // the state committed up through `PrePass::OverhangAnnotation` matters
    // here (mirrors `overhang_annotation_runs_after_mesh_analysis_and_layer_planning`).
    let _ = execute_prepass_with_builtins(&plan, &mut blackboard, &runner, &wasm_handles);

    let surface_classification = blackboard
        .surface_classification()
        .expect("SurfaceClassificationIR must be committed by PrePass::MeshAnalysis");
    assert!(
        !surface_classification.overhang_quartile_polygons.is_empty(),
        "fixture setup invariant: expected at least one global layer index with \
         overhang quartile bands, got: {:?}",
        surface_classification.overhang_quartile_polygons
    );

    // Pick any layer index that actually has overhang quartile bands, and
    // build a region for the "ramp" object whose polygon generously covers
    // both boxes' XY footprint so the cheap AABB prefilter overlaps.
    let (&overhang_layer_index, _) = surface_classification
        .overhang_quartile_polygons
        .iter()
        .next()
        .expect("checked non-empty above");
    let layer_z = two_global_layers()
        .into_iter()
        .find(|l| l.index == overhang_layer_index)
        .map(|l| l.z)
        .unwrap_or(0.0);

    let region = SlicedRegion {
        object_id: String::from("ramp"),
        region_id: 1,
        polygons: vec![covering_square(-5.0, -5.0, 20.0, 15.0)],
        ..Default::default()
    };

    let data = sliced_region_to_data(
        &region,
        layer_z,
        vec![],
        Some(surface_classification.as_ref()),
        overhang_layer_index,
    );

    assert!(
        !data.overhang_areas.is_empty(),
        "overhang_areas() must be non-empty for a region covering the overhang \
         footprint at layer {overhang_layer_index} (packet 107 wiring)"
    );
    assert!(
        !data.overhang_quartile_polygons.is_empty(),
        "overhang_quartile_polygons() must be non-empty for a region covering the \
         overhang footprint at layer {overhang_layer_index} (packet 107 wiring)"
    );

    // AC-1/AC-2 (packet 107 fix): `overhang_areas` and each quartile band's
    // polygons must be exactly clipped to the region's own polygon area, not
    // merely AABB-adjacent to it. Assert every returned vertex lies within
    // the region's own bounding box (a necessary condition of exact clipping
    // — the pre-fix AABB-only prefilter could leak vertices from a band that
    // extends past the region but whose AABB happens to overlap).
    let region_bbox =
        ir_expolygons_bbox(&region.polygons).expect("test region has non-empty polygons");
    let (r_min_x, r_min_y, r_max_x, r_max_y) = region_bbox;

    let overhang_bbox =
        wit_expolygons_bbox(&data.overhang_areas).expect("checked overhang_areas non-empty above");
    assert!(
        overhang_bbox.0 >= r_min_x
            && overhang_bbox.1 >= r_min_y
            && overhang_bbox.2 <= r_max_x
            && overhang_bbox.3 <= r_max_y,
        "overhang_areas bbox {overhang_bbox:?} must be contained within the region's own \
         polygon bbox {region_bbox:?} — a wider bbox proves band polygons leaked in from \
         outside the region's actual footprint (AC-1)"
    );

    for band in &data.overhang_quartile_polygons {
        let band_bbox = wit_expolygons_bbox(&band.polygons)
            .expect("dropped-if-empty invariant: only non-empty clipped bands are emitted");
        assert!(
            band_bbox.0 >= r_min_x
                && band_bbox.1 >= r_min_y
                && band_bbox.2 <= r_max_x
                && band_bbox.3 <= r_max_y,
            "quartile band (q={}) bbox {band_bbox:?} must be contained within the region's \
             own polygon bbox {region_bbox:?} (AC-1)",
            band.quartile
        );
    }
}
