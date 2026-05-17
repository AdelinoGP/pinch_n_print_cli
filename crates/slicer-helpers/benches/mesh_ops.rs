//! Microbenchmarks for slicer-helpers mesh ops.
//!
//! Loads STEP fixtures from `tests/resources/` once outside `b.iter` and
//! benches `import_step`, `repair`, and `decimate` on the resulting mesh.
//! Repair and decimate consume `MeshIR` by value, so the iteration body
//! uses `b.iter_batched` to clone the prepared mesh per iteration.

#![allow(missing_docs)]

use std::path::PathBuf;

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};

use slicer_helpers::decimate::{decimate, DecimateConfig};
use slicer_helpers::import::step::import_step;
use slicer_helpers::repair::repair;
use slicer_ir::MeshIR;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/resources")
}

fn load_cube_mesh() -> MeshIR {
    let path = fixtures_dir().join("cube.step");
    let mut result = import_step(&path).expect("import cube.step");
    let named = result.meshes.swap_remove(0);
    named.mesh
}

fn bench_import_step(c: &mut Criterion) {
    let mut g = c.benchmark_group("mesh_ops/import_step");
    let cube_path = fixtures_dir().join("cube.step");
    let assembly_path = fixtures_dir().join("assembly.step");
    g.bench_function("cube", |b| {
        b.iter(|| {
            let r = import_step(black_box(&cube_path)).expect("cube import");
            black_box(r);
        })
    });
    g.bench_function("assembly", |b| {
        b.iter(|| {
            let r = import_step(black_box(&assembly_path)).expect("assembly import");
            black_box(r);
        })
    });
    g.finish();
}

fn bench_repair(c: &mut Criterion) {
    let mesh = load_cube_mesh();
    c.bench_function("mesh_ops/repair/cube", |b| {
        b.iter_batched(
            || mesh.clone(),
            |m| {
                let r = repair(m).expect("repair");
                black_box(r);
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_decimate(c: &mut Criterion) {
    let mesh = load_cube_mesh();
    let config = DecimateConfig::default();
    c.bench_function("mesh_ops/decimate/cube_default", |b| {
        b.iter_batched(
            || mesh.clone(),
            |m| {
                let r = decimate(m, config.clone()).expect("decimate");
                black_box(r);
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, bench_import_step, bench_repair, bench_decimate);
criterion_main!(benches);
