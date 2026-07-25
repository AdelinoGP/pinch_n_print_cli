//! Microbench for `commit_shell_classification_builtin`.
//!
//! # What this measures, and what the previous version did not
//!
//! The stage's parallel axis is the **timeline** — the run of layers belonging
//! to one `(object_id, region_id)` pair — not the set of regions. On an
//! ordinary single-material, single-object print there is exactly **one**
//! timeline, because `layer-planner-default`'s `run_layer_planning` emits
//! `region_id: "0"` at every emission site. A measured 0.1 mm benchy reports
//! `timelines=1 lengths=[480]`.
//!
//! The previous fixture built N objects each carrying one region, so it swept
//! the *object* count (1, 4, 16) and never varied timeline length beyond 200.
//! Worse, every layer of every object carried the **same** 4-point square, so
//! `difference(layer, neighbour)` was empty on all but the outermost layer;
//! `apply_opening` then hit its `r <= 0.0 || polys.is_empty()` short-circuit and
//! the `offset` calls — the dominant real cost — never executed. It also pinned
//! the shell counts to 2, leaving Pass 2 a single step per seed. The result was
//! a benchmark of near-zero work on 4-vertex polygons, whose "per-region work
//! runs in microseconds" conclusion did not transfer to real geometry.
//!
//! This fixture instead varies the cross-section per layer so the diffs are
//! non-empty and sliver-rich, uses the default shell count of 3, and sweeps
//! timeline length. `n_objects` is retained only to confirm that the object
//! axis is not where the time goes.
//!
//! Run with the default thread count to measure parallel throughput; set
//! `RAYON_NUM_THREADS=1` to measure single-threaded wall-clock for comparison.

#![allow(missing_docs)]

use std::sync::Arc;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

use slicer_ir::{
    ActiveRegion, BoundingBox3, ExPolygon, GlobalLayer, IndexedTriangleSet, LayerPlanIR, MeshIR,
    ObjectMesh, Point2, Point3, Polygon, RegionKey, RegionMapIR, RegionPlan, ResolvedConfig,
    SliceIR, SlicedRegion, Transform3d, CURRENT_SLICE_IR_SCHEMA_VERSION,
};
use slicer_runtime::{commit_shell_classification_builtin, Blackboard};

fn identity() -> Transform3d {
    let mut m = [0.0_f64; 16];
    m[0] = 1.0;
    m[5] = 1.0;
    m[10] = 1.0;
    m[15] = 1.0;
    Transform3d { matrix: m }
}

/// An approximated circle. Vertex count matters: the real cost driver is
/// `apply_opening`'s round-join `offset` over the many-vertex sliver rings that
/// coincident-edge subtraction produces, which a 4-point square never exercises.
fn disc(cx: f32, cy: f32, r: f32, segments: usize) -> Polygon {
    Polygon {
        points: (0..segments)
            .map(|i| {
                let a = std::f32::consts::TAU * i as f32 / segments as f32;
                Point2::from_mm(cx + r * a.cos(), cy + r * a.sin())
            })
            .collect(),
    }
}

/// One layer's cross-section for object `obj_i` at layer `layer_idx`.
///
/// The radius breathes with layer index and the hole drifts laterally, so
/// consecutive layers differ everywhere along their boundary. That is what
/// makes `difference(current, neighbour)` a thin high-vertex ring rather than
/// the empty set, and what forces `apply_opening` to actually run its two
/// offsets — reproducing the shape of the work a real model imposes.
fn cross_section(obj_i: usize, layer_idx: usize) -> ExPolygon {
    let x_offset = 30.0 * obj_i as f32;
    let phase = layer_idx as f32 * 0.11;
    let outer_r = 9.0 + 0.9 * phase.sin();
    let hole_dx = 2.2 * (phase * 0.7).cos();
    ExPolygon {
        contour: disc(x_offset, 0.0, outer_r, 64),
        holes: vec![disc(x_offset + hole_dx, 0.0, 3.0, 32)],
    }
}

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

    let mut region_map = RegionMapIR::default();
    for layer in &global_layers {
        for active in &layer.active_regions {
            // Defaults: 3 top / 3 bottom shells, so Pass 2 walks two steps per
            // seed rather than the single step a count of 2 allowed. Leaving
            // `line_width` at its default also gives `resolve_opening_radius` a
            // non-zero radius, so `apply_opening` does not short-circuit.
            let config = active.resolved_config.clone();
            let config_id = region_map.intern_config(config);
            region_map.entries.insert(
                RegionKey {
                    global_layer_index: layer.index,
                    object_id: active.object_id.clone(),
                    region_id: active.region_id,
                    variant_chain: Vec::new(),
                },
                RegionPlan {
                    config: config_id,
                    ..Default::default()
                },
            );
        }
    }
    bb.commit_region_map(Arc::new(region_map)).unwrap();

    let mut slice_vec = Vec::with_capacity(n_layers);
    for (layer_idx, layer) in global_layers.iter().enumerate() {
        let regions: Vec<SlicedRegion> = layer
            .active_regions
            .iter()
            .enumerate()
            .map(|(obj_i, active)| {
                let polys = vec![cross_section(obj_i, layer_idx)];
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
    // Fewer, longer samples: one iteration is now milliseconds of real polygon
    // work rather than microseconds, and building a 480-layer fixture per batch
    // is itself costly.
    group.sample_size(10);

    // (n_objects, n_layers). The single-object sweep is the shape that matters
    // — it is what a real print produces, and it isolates the timeline axis.
    // The 16-object rows are retained only to show the object axis is not the
    // cost driver.
    let scenarios = [
        (1usize, 120usize),
        (1, 240),
        (1, 480),
        (16, 120),
        (16, 480),
    ];

    for &(n_objects, n_layers) in &scenarios {
        let id = format!("objs={n_objects}_layers={n_layers}");
        group.bench_with_input(BenchmarkId::from_parameter(&id), &(), |b, _| {
            b.iter_batched(
                || build_fixture(n_objects, n_layers),
                |mut bb| {
                    commit_shell_classification_builtin(black_box(&mut bb))
                        .expect("classification");
                },
                criterion::BatchSize::PerIteration,
            );
        });
    }

    group.finish();
}

criterion_group!(benches, bench_shell_classification);
criterion_main!(benches);
