//! Microbench for `commit_shell_classification_builtin` against a realistic
//! multi-region multi-layer fixture.
//!
//! The fixture builds N "objects", each with one region, each with L layers
//! of a 20×20 mm square. This stresses the per-(object, region) parallel
//! loop introduced in the slicing-promotion refactor.
//!
//! Run with default thread count to measure parallel throughput; set
//! `RAYON_NUM_THREADS=1` in the environment to measure single-threaded
//! wall-clock for direct comparison.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

use slicer_host::{commit_shell_classification_builtin, Blackboard};
use slicer_ir::{
    ActiveRegion, BoundingBox3, ExPolygon, GlobalLayer, IndexedTriangleSet, LayerPlanIR, MeshIR,
    ObjectMesh, Point2, Point3, Polygon, RegionKey, RegionMapIR, RegionPlan, ResolvedConfig,
    SliceIR, SlicedRegion, Transform3d, CURRENT_SLICE_IR_SCHEMA_VERSION,
};

fn identity() -> Transform3d {
    let mut m = [0.0_f64; 16];
    m[0] = 1.0;
    m[5] = 1.0;
    m[10] = 1.0;
    m[15] = 1.0;
    Transform3d { matrix: m }
}

fn square_at(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(min_x, min_y),
                Point2::from_mm(max_x, min_y),
                Point2::from_mm(max_x, max_y),
                Point2::from_mm(min_x, max_y),
            ],
        },
        holes: vec![],
    }
}

/// Build a fixture: N objects, each with one region, each spanning L layers
/// of a 20×20 mm square (offset in X per object so they're spatially
/// disjoint).
fn build_fixture(n_objects: usize, n_layers: usize) -> Blackboard {
    let mesh = MeshIR {
        objects: (0..n_objects)
            .map(|i| ObjectMesh {
                id: format!("obj-{i}"),
                mesh: IndexedTriangleSet {
                    vertices: vec![],
                    indices: vec![],
                },
                transform: identity(),
                ..Default::default()
            })
            .collect(),
        build_volume: BoundingBox3 {
            min: Point3 {
                x: -100.0,
                y: -100.0,
                z: 0.0,
            },
            max: Point3 {
                x: 1000.0,
                y: 100.0,
                z: 100.0,
            },
        },
        ..Default::default()
    };
    let mut bb = Blackboard::new(Arc::new(mesh), n_layers);

    let mut global_layers = Vec::with_capacity(n_layers);
    for layer_idx in 0..n_layers {
        let z = 0.2 * (layer_idx + 1) as f32;
        let active_regions: Vec<ActiveRegion> = (0..n_objects)
            .map(|i| ActiveRegion {
                object_id: format!("obj-{i}"),
                region_id: 0,
                resolved_config: ResolvedConfig::default(),
                effective_layer_height: 0.2,
                nonplanar_shell: None,
                is_catchup_layer: false,
                catchup_z_bottom: 0.0,
                tool_index: 0,
            })
            .collect();
        global_layers.push(GlobalLayer {
            index: layer_idx as u32,
            z,
            active_regions,
            has_nonplanar: false,
            is_sync_layer: false,
        });
    }

    let plan = LayerPlanIR {
        global_layers: global_layers.clone(),
        ..Default::default()
    };
    bb.commit_layer_plan(Arc::new(plan)).unwrap();

    let mut entries = HashMap::new();
    for layer in &global_layers {
        for active in &layer.active_regions {
            let mut config = active.resolved_config.clone();
            config.top_shell_layers = 2;
            config.bottom_shell_layers = 2;
            entries.insert(
                RegionKey {
                    global_layer_index: layer.index,
                    object_id: active.object_id.clone(),
                    region_id: active.region_id,
                },
                RegionPlan {
                    config,
                    ..Default::default()
                },
            );
        }
    }
    bb.commit_region_map(Arc::new(RegionMapIR {
        entries,
        ..Default::default()
    }))
    .unwrap();

    // Build a SliceIR per global layer. Each object's region carries the
    // same 20×20 square at that layer's index (shifted in X per object).
    let mut slice_vec = Vec::with_capacity(n_layers);
    for (layer_idx, layer) in global_layers.iter().enumerate() {
        let regions: Vec<SlicedRegion> = layer
            .active_regions
            .iter()
            .enumerate()
            .map(|(obj_i, active)| {
                let x_offset = 30.0 * obj_i as f32;
                let polys = vec![square_at(x_offset - 10.0, -10.0, x_offset + 10.0, 10.0)];
                SlicedRegion {
                    object_id: active.object_id.clone(),
                    region_id: active.region_id,
                    polygons: polys.clone(),
                    infill_areas: polys,
                    ..Default::default()
                }
            })
            .collect();
        slice_vec.push(SliceIR {
            schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
            global_layer_index: layer_idx as u32,
            z: layer.z,
            regions,
        });
    }
    bb.commit_slice_ir(Arc::new(slice_vec)).unwrap();

    bb
}

fn bench_shell_classification(c: &mut Criterion) {
    let mut group = c.benchmark_group("shell_classification");

    // Scenarios: (n_objects, n_layers). The first is single-object (no
    // parallel benefit possible). The rest stress per-region parallelism.
    let scenarios = [(1usize, 50usize), (4, 50), (16, 50), (16, 200)];

    for &(n_objects, n_layers) in &scenarios {
        let id = format!("objs={n_objects}_layers={n_layers}");
        group.bench_with_input(BenchmarkId::from_parameter(&id), &(), |b, _| {
            b.iter_batched(
                || build_fixture(n_objects, n_layers),
                |mut bb| {
                    commit_shell_classification_builtin(black_box(&mut bb))
                        .expect("classification");
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

criterion_group!(benches, bench_shell_classification);
criterion_main!(benches);
