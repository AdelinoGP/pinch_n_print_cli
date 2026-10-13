//! Fixture guards for the `mesh_ops` criterion bench (perf-vs-orca ticket 29).
//!
//! The bench's `repair` and `decimate` leaves must measure successful
//! operations. These tests assert the same property in the test suite, so a
//! fixture that silently becomes a no-op fails `cargo test -p slicer-helpers`
//! even when nobody runs the bench.
//!
//! Regression input this file catches: a swap of
//! `resources/regression_wedge.stl` for a clean mesh (all `repair` stats
//! zero), or of `resources/calicat.stl` for a mesh whose decimation stops
//! above the declared target under the default error budget — the two
//! pre-ticket defects recorded in
//! `docs/specs/perf-vs-orca/evidence/t15-criterion-refresh/FINDINGS.md`
//! (findings #2 and #3).

use std::path::PathBuf;

use slicer_helpers::decimate::{decimate, DecimateConfigBuilder};
use slicer_helpers::repair;
use slicer_ir::MeshIR;

/// Mirrors `DECIMATE_TARGET_RATIO` in `benches/mesh_ops.rs`.
const DECIMATE_TARGET_RATIO: f32 = 0.5;

fn repo_resources_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("resources")
}

fn load_fixture(path: PathBuf) -> MeshIR {
    slicer_model_io::load_model(&path)
        .unwrap_or_else(|e| panic!("bench fixture {} must load: {e}", path.display()))
}

#[test]
fn repair_bench_fixture_is_not_a_noop() {
    let path = repo_resources_dir().join("regression_wedge.stl");
    let mesh = load_fixture(path);
    let result = repair(mesh).expect("repair must succeed on the bench fixture");
    let touched = result.stats.degenerate_removed
        + result.stats.faces_reoriented
        + result.stats.open_edges_closed;
    assert!(
        touched > 0,
        "repair bench fixture must carry real damage; got \
         degenerate_removed={}, faces_reoriented={}, open_edges_closed={} — \
         a clean fixture makes the mesh_ops/repair/wedge leaf a no-op scan",
        result.stats.degenerate_removed,
        result.stats.faces_reoriented,
        result.stats.open_edges_closed
    );
}

#[test]
fn decimate_bench_fixture_reaches_declared_target() {
    let path = repo_resources_dir().join("calicat.stl");
    let mesh = load_fixture(path);
    let original_triangle_count: usize =
        mesh.objects.iter().map(|o| o.mesh.indices.len() / 3).sum();
    let declared_target = ((original_triangle_count as f32) * DECIMATE_TARGET_RATIO)
        .round()
        .max(1.0) as usize;
    let config = DecimateConfigBuilder::new()
        .target_ratio(DECIMATE_TARGET_RATIO)
        .build()
        .expect("fixture guard builder should validate");
    let result = decimate(mesh, config).expect("decimate must succeed on the bench fixture");
    assert!(
        result.final_triangle_count <= declared_target,
        "decimate bench fixture must reach its declared target; got {} -> {} \
         (target {}) — the mesh_ops/decimate/calicat_default leaf would \
         measure rejected work",
        result.original_triangle_count,
        result.final_triangle_count,
        declared_target
    );
}

/// Negative control: run both guard predicates against the pre-ticket
/// `cube.step` fixture through the pre-ticket load path (`import_step`, as
/// the old bench's `load_cube_mesh` did). Neither predicate holds — repair
/// touches nothing and decimate stops above the declared target — which is
/// exactly the defect
/// `docs/specs/perf-vs-orca/evidence/t15-criterion-refresh/FINDINGS.md`
/// recorded as findings #2 and #3. This proves the guard predicates are
/// falsifiable by the defect they exist to catch, rather than vacuously true.
///
/// If `cube.step` ever stops reproducing both no-op outcomes, this control
/// must be revisited deliberately: it pins the historical defect, not a
/// contract of `cube.step` itself.
#[test]
fn pre_ticket_cube_fixture_would_fail_both_guards() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/resources/cube.step");
    let imported = slicer_helpers::import_step(&path).expect("cube.step must import");
    let mesh = imported
        .meshes
        .into_iter()
        .next()
        .expect("cube.step must yield one mesh")
        .mesh;

    let repaired = repair(mesh.clone()).expect("repair must succeed on cube.step");
    let touched = repaired.stats.degenerate_removed
        + repaired.stats.faces_reoriented
        + repaired.stats.open_edges_closed;
    assert_eq!(
        touched,
        0,
        "negative control premise changed: cube.step now carries repairable \
         damage (degenerate_removed={}, faces_reoriented={}, open_edges_closed={})",
        repaired.stats.degenerate_removed,
        repaired.stats.faces_reoriented,
        repaired.stats.open_edges_closed
    );

    let original_triangle_count: usize =
        mesh.objects.iter().map(|o| o.mesh.indices.len() / 3).sum();
    let declared_target = ((original_triangle_count as f32) * DECIMATE_TARGET_RATIO)
        .round()
        .max(1.0) as usize;
    let config = DecimateConfigBuilder::new()
        .target_ratio(DECIMATE_TARGET_RATIO)
        .build()
        .expect("negative control builder should validate");
    let decimated = decimate(mesh, config).expect("decimate must succeed on cube.step");
    assert!(
        decimated.final_triangle_count > declared_target,
        "negative control premise changed: cube.step now reaches the declared \
         target ({} -> {}, target {})",
        decimated.original_triangle_count,
        decimated.final_triangle_count,
        declared_target
    );
}
