#![cfg(feature = "host-algos")]
#![allow(missing_docs)]

// Metric helpers: per_vertex_to_reference, coverage_ref_to_polylines,
// max_width_error, PER_VERTEX_PASS_MM, COVERAGE_PASS_MM, WIDTH_PASS_MM
include!("fixtures/medial_axis_golden/metric.rs.txt");

use slicer_core::medial_axis::medial_axis;
use slicer_ir::{ExPolygon, Point2, Polygon};
use std::path::Path;

// ---------------------------------------------------------------------------
// Fixture schema
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct RefPoint {
    x: f64,
    y: f64,
    width: f64,
}

#[derive(serde::Deserialize)]
struct Fixture {
    name: String,
    contour_mm: Vec<[f64; 2]>,
    holes_mm: Vec<Vec<[f64; 2]>>,
    min_width: f32,
    max_width: f32,
    reference_axis: Vec<RefPoint>,
}

// ---------------------------------------------------------------------------
// Helper: load fixture JSON from tests/fixtures/medial_axis_golden/<name>.json
// ---------------------------------------------------------------------------

fn load_fixture(name: &str) -> Fixture {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir)
        .join("tests/fixtures/medial_axis_golden")
        .join(format!("{}.json", name));
    let data = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read fixture {}: {}", path.display(), e));
    serde_json::from_str(&data).unwrap_or_else(|e| panic!("cannot parse fixture {}: {}", name, e))
}

// ---------------------------------------------------------------------------
// Helper: build ExPolygon from fixture
// ---------------------------------------------------------------------------

fn fixture_to_expoly(f: &Fixture) -> ExPolygon {
    let contour_pts: Vec<Point2> = f
        .contour_mm
        .iter()
        .map(|&[x, y]| Point2::from_mm(x as f32, y as f32))
        .collect();
    let holes: Vec<Polygon> = f
        .holes_mm
        .iter()
        .map(|hole| Polygon {
            points: hole
                .iter()
                .map(|&[x, y]| Point2::from_mm(x as f32, y as f32))
                .collect(),
        })
        .collect();
    ExPolygon {
        contour: Polygon {
            points: contour_pts,
        },
        holes,
    }
}

// ---------------------------------------------------------------------------
// Core test logic (shared by all fixture tests)
// ---------------------------------------------------------------------------

fn run_golden_test(fixture_name: &str) {
    let fixture = load_fixture(fixture_name);

    // Reference fixture.name in the assert messages so dead_code lint is
    // satisfied — the JSON name may differ from the file stem (e.g. a fixture
    // JSON can say "wedge_25deg" while the file was previously "wedge_30deg").
    let label = format!("{} (file={})", fixture.name, fixture_name);

    let expoly = fixture_to_expoly(&fixture);

    let polylines = medial_axis(&expoly, fixture.min_width, fixture.max_width)
        .unwrap_or_else(|e| panic!("[{}] medial_axis returned Err({:?})", label, e));

    // Per-polyline (x, y) lists for the coverage metric.
    let polyline_xy: Vec<Vec<(f64, f64)>> = polylines
        .iter()
        .map(|pl| pl.points.iter().map(|p| (p.x as f64, p.y as f64)).collect())
        .collect();

    // Flat (x, y) for the per-vertex metric.
    let out_xy: Vec<(f64, f64)> = polyline_xy.iter().flat_map(|v| v.iter().copied()).collect();

    // Flat (x, y, width) for the width-error metric.
    let out_xyw: Vec<(f64, f64, f64)> = polylines
        .iter()
        .flat_map(|pl| {
            pl.points
                .iter()
                .map(|p| (p.x as f64, p.y as f64, p.width as f64))
        })
        .collect();

    // Reference point lists.
    let ref_xy: Vec<(f64, f64)> = fixture.reference_axis.iter().map(|r| (r.x, r.y)).collect();
    let ref_xyw: Vec<(f64, f64, f64)> = fixture
        .reference_axis
        .iter()
        .map(|r| (r.x, r.y, r.width))
        .collect();

    // Compute all three metrics.
    let pv = per_vertex_to_reference(&out_xy, &ref_xy);
    let cov = coverage_ref_to_polylines(&ref_xy, &polyline_xy);
    let width_err = max_width_error(&out_xyw, &ref_xyw);

    println!(
        "[{}] per_vertex_mm={:.6}  coverage_mm={:.6}  width_err_mm={:.6}  \
         (thresholds: PV≤{:.3}, COV≤{:.3}, W≤{:.3})",
        label, pv, cov, width_err, PER_VERTEX_PASS_MM, COVERAGE_PASS_MM, WIDTH_PASS_MM
    );

    assert!(
        pv <= PER_VERTEX_PASS_MM,
        "[{}] FAIL: per_vertex_to_reference={:.6} mm exceeds threshold {:.3} mm",
        label,
        pv,
        PER_VERTEX_PASS_MM
    );

    assert!(
        cov <= COVERAGE_PASS_MM,
        "[{}] FAIL: coverage_ref_to_polylines={:.6} mm exceeds threshold {:.3} mm",
        label,
        cov,
        COVERAGE_PASS_MM
    );

    assert!(
        width_err <= WIDTH_PASS_MM,
        "[{}] FAIL: max_width_error={:.6} mm exceeds threshold {:.3} mm",
        label,
        width_err,
        WIDTH_PASS_MM
    );
}

// ---------------------------------------------------------------------------
// One test per fixture
// ---------------------------------------------------------------------------

#[test]
fn golden_rectangle() {
    run_golden_test("rectangle");
}

#[test]
fn golden_wedge_25deg() {
    run_golden_test("wedge_25deg");
}

#[test]
fn golden_asymmetric_taper() {
    run_golden_test("asymmetric_taper");
}

#[test]
fn golden_curved_boundary() {
    run_golden_test("curved_boundary");
}

#[test]
fn golden_nested_hole() {
    run_golden_test("nested_hole");
}

/// Proves that Douglas-Peucker decimation makes a densely-tessellated version of the
/// curved_boundary shape produce an axis matching the coarse fixture within tolerance.
/// The dense fixture has ≥50 segments with near-collinear (<0.0115 mm) perturbations;
/// DP collapses them back to the coarse hexagon vertices before the VD is built.
#[test]
fn golden_curved_boundary_dense() {
    run_golden_test("curved_boundary_dense");
}
