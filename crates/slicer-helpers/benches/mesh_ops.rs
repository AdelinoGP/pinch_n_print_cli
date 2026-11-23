//! Microbenchmarks for slicer-helpers mesh ops.
//!
//! Loads fixtures through the production model loader once outside `b.iter`
//! and benches `import_step`, `repair`, and `decimate` on the resulting mesh.
//! Repair and decimate consume `MeshIR` by value, so the iteration body
//! uses `b.iter_batched` to clone the prepared mesh per iteration.
//!
//! Fixture strategy (perf-vs-orca ticket 29): the `repair` leaf runs on
//! `resources/regression_wedge.stl` (real damage: flipped faces and open
//! edges) and the `decimate` leaf on `resources/calicat.stl` (dense enough
//! that the default error budget reaches the 0.5 target). Both replace the
//! pre-ticket `cube.step` mesh, which produced a no-op repair scan and a
//! decimate run the default budget rejected — see
//! `docs/specs/perf-vs-orca/evidence/t15-criterion-refresh/FINDINGS.md`
//! (findings #2 and #3). **Pre-ticket `mesh_ops/repair/*` and
//! `mesh_ops/decimate/*` baseline numbers are not comparable to these
//! leaves; the fixture changed.** Each leaf carries a loud fixture guard
//! that fails before timing if the fixture stops exercising a successful
//! operation; `tests/mesh_ops_bench_fixtures_tdd.rs` repeats the checks in
//! the test suite.

#![allow(missing_docs)]

use std::path::PathBuf;

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};

use slicer_helpers::decimate::{decimate, DecimateConfigBuilder};
use slicer_helpers::import::step::import_step;
use slicer_helpers::repair::repair;
use slicer_ir::MeshIR;

/// Fraction of triangles the decimate bench retains. Mirrored by
/// `tests/mesh_ops_bench_fixtures_tdd.rs`.
const DECIMATE_TARGET_RATIO: f32 = 0.5;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/resources")
}

/// Repo-root `resources/` directory (shared fixtures such as
/// `regression_wedge.stl` and `calicat.stl`).
fn repo_resources_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("resources")
}

fn load_fixture_mesh(path: PathBuf) -> MeshIR {
    slicer_model_io::load_model(&path)
        .unwrap_or_else(|e| panic!("failed to load bench fixture {}: {e}", path.display()))
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
    let mesh = load_fixture_mesh(repo_resources_dir().join("regression_wedge.stl"));

    // Fixture guard: a clean mesh would make this leaf time a no-op scan (the
    // pre-ticket defect). Fail before timing instead of reporting it.
    let probe = repair(mesh.clone()).expect("repair fixture probe");
    let touched = probe.stats.degenerate_removed
        + probe.stats.faces_reoriented
        + probe.stats.open_edges_closed;
    assert!(
        touched > 0,
        "repair bench fixture (resources/regression_wedge.stl) is clean \
         (degenerate_removed={}, faces_reoriented={}, open_edges_closed={}); \
         the leaf would measure a no-op scan",
        probe.stats.degenerate_removed,
        probe.stats.faces_reoriented,
        probe.stats.open_edges_closed
    );

    c.bench_function("mesh_ops/repair/wedge", |b| {
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
    let mesh = load_fixture_mesh(repo_resources_dir().join("calicat.stl"));
    let config = DecimateConfigBuilder::new()
        .target_ratio(DECIMATE_TARGET_RATIO)
        .build()
        .expect("bench builder should validate");

    // Fixture guard: the pre-ticket cube fixture could never reach its target
    // under the default error budget, so the leaf measured rejected work.
    // Fail before timing if the fixture stops reaching the declared target.
    let original_triangle_count: usize =
        mesh.objects.iter().map(|o| o.mesh.indices.len() / 3).sum();
    let declared_target = ((original_triangle_count as f32) * DECIMATE_TARGET_RATIO)
        .round()
        .max(1.0) as usize;
    let probe = decimate(mesh.clone(), config.clone()).expect("decimate fixture probe");
    assert!(
        probe.final_triangle_count <= declared_target,
        "decimate bench fixture (resources/calicat.stl) does not reach its \
         declared target ({} -> {}, target {}); the leaf would measure \
         rejected work",
        probe.original_triangle_count,
        probe.final_triangle_count,
        declared_target
    );

    c.bench_function("mesh_ops/decimate/calicat_default", |b| {
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
