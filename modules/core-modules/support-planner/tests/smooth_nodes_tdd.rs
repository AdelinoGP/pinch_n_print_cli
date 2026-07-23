//! RED-phase TDD tests for `support_planner::smooth_branches` — the Rust port
//! of Orca's `TreeSupport::smooth_nodes` (TASK-286, packet 121).
//!
//! These tests are authored BEFORE `smooth_branches` exists, so the file is
//! expected to FAIL TO COMPILE (unresolved import `smooth_branches`). That
//! compile error is the canonical RED state.
//!
//! Import convention: the planner's `lib.rs` does NOT re-export the IR data
//! types, so they are pulled directly from `slicer_ir` (a normal dependency of
//! the `support-planner` crate). The function under test lives in the planner
//! crate root: `support_planner::smooth_branches`.
//!
//! Column shape (chosen for simplicity, per packet guidance):
//!   FIVE `SupportPlanEntry` rows, all sharing `object_id`/`region_id`, each
//!   holding ONE `ExtrusionPath3D` that holds ONE `Point3WithWidth`. The
//!   smoother chains ACROSS entries within a `(object_id, region_id)` group,
//!   sorted by `global_layer_index` DESCENDING (tip = highest layer index at
//!   the top of the descending chain). We assign layer indices so that the
//!   descending-sorted chain reproduces the coordinate list in listed order.
//!   `smooth_branches` mutates `x`/`y` (and `width`) in place; z is preserved.

use slicer_ir::Point3WithWidth;
use slicer_sdk::prepass_types::SupportPlanEntry;

/// The canonical 5-point branch column used by the curvature/endpoint tests.
/// Tip (z=5.0) and root (z=1.0) both sit on the axis (x/y unchanged relative
/// to their neighbours only through smoothing); the interior zig-zags in x/y
/// so smoothing has curvature to remove.
const COLUMN: [(f32, f32, f32); 5] = [
    (0.0, 0.0, 5.0),
    (1.0, 0.0, 4.0),
    (1.0, 1.0, 3.0),
    (2.0, 1.0, 2.0),
    (2.0, 0.0, 1.0),
];

/// Build a single `Point3WithWidth` at the given coordinates with benign
/// defaults for the ancillary fields.
fn pt(x: f32, y: f32, z: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
        dist_to_top_mm: 0.0,
    }
}

/// Wrap a single point into a `SupportPlanEntry` at the given global layer.
fn entry(global_layer_index: i32, p: Point3WithWidth) -> SupportPlanEntry {
    SupportPlanEntry {
        global_layer_index,
        object_id: "obj".to_string(),
        region_id: "0".to_string(),
        branch_segments: vec![vec![p]],
    }
}

/// Build a column of entries from a coordinate list. The FIRST coordinate maps
/// to the HIGHEST `global_layer_index`, so a descending-by-layer sort of the
/// resulting entries reproduces `coords` in the listed order.
fn build_column(coords: &[(f32, f32, f32)]) -> Vec<SupportPlanEntry> {
    let n = coords.len();
    coords
        .iter()
        .enumerate()
        .map(|(i, &(x, y, z))| entry((n - 1 - i) as i32, pt(x, y, z)))
        .collect()
}

/// Read the chained column back out in smoother-chain order: group members are
/// ordered by `global_layer_index` DESCENDING, one point per entry.
fn read_column(entries: &[SupportPlanEntry]) -> Vec<Point3WithWidth> {
    let mut refs: Vec<&SupportPlanEntry> = entries.iter().collect();
    refs.sort_by_key(|r| std::cmp::Reverse(r.global_layer_index));
    refs.iter().map(|e| e.branch_segments[0][0]).collect()
}

/// Maximum turn-angle (in degrees) between consecutive segment vectors
/// `(p[i+1] - p[i])` and `(p[i+2] - p[i+1])`. Returns 0.0 for chains with
/// fewer than 3 points.
fn max_turn_angle(points: &[Point3WithWidth]) -> f32 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut max_deg = 0.0f32;
    for i in 0..points.len() - 2 {
        let v1 = (
            points[i + 1].x - points[i].x,
            points[i + 1].y - points[i].y,
            points[i + 1].z - points[i].z,
        );
        let v2 = (
            points[i + 2].x - points[i + 1].x,
            points[i + 2].y - points[i + 1].y,
            points[i + 2].z - points[i + 1].z,
        );
        let dot = v1.0 * v2.0 + v1.1 * v2.1 + v1.2 * v2.2;
        let n1 = (v1.0 * v1.0 + v1.1 * v1.1 + v1.2 * v1.2).sqrt();
        let n2 = (v2.0 * v2.0 + v2.1 * v2.1 + v2.2 * v2.2).sqrt();
        if n1 == 0.0 || n2 == 0.0 {
            continue;
        }
        let cos = (dot / (n1 * n2)).clamp(-1.0, 1.0);
        let deg = cos.acos().to_degrees();
        if deg > max_deg {
            max_deg = deg;
        }
    }
    max_deg
}

/// AC-2: smoothing_reduces_curvature.
/// A 5-point zig-zag column, after 100 smoothing iterations, must have a lower
/// maximum consecutive-segment turn-angle across its (middle-three) interior
/// vertices than the input.
#[test]
fn smoothing_reduces_curvature() {
    let before_pts = read_column(&build_column(&COLUMN));
    let before = max_turn_angle(&before_pts);

    let mut entries = build_column(&COLUMN);
    support_planner::smooth_branches(&mut entries, 100);

    let after = max_turn_angle(&read_column(&entries));
    assert!(
        after < before,
        "smoothing must reduce curvature: after={after} !< before={before}"
    );
}

/// AC-3: endpoints_held_fixed.
/// The tip (first) and root (last) points of the chain are unchanged
/// (bit-exact) after smoothing.
#[test]
fn endpoints_held_fixed() {
    let before = read_column(&build_column(&COLUMN));

    let mut entries = build_column(&COLUMN);
    support_planner::smooth_branches(&mut entries, 100);
    let after = read_column(&entries);

    assert_eq!(before[0], after[0], "tip endpoint must be held fixed");
    assert_eq!(
        before[before.len() - 1],
        after[after.len() - 1],
        "root endpoint must be held fixed"
    );
}

/// AC-N1: columns_below_three_points_unchanged.
/// A 2-point column (root + tip only) is a no-op for the three-point
/// Laplacian; it must be returned unchanged.
#[test]
fn columns_below_three_points_unchanged() {
    let coords = [(0.0, 0.0, 2.0), (1.0, 1.0, 1.0)];
    let before = read_column(&build_column(&coords));

    let mut entries = build_column(&coords);
    support_planner::smooth_branches(&mut entries, 100);
    let after = read_column(&entries);

    assert_eq!(before, after, "chains with < 3 points must be unchanged");
}

/// AC-N2: empty_entries_no_panic.
/// An empty entry list must be handled without panicking. (May pass
/// coincidentally — an empty input is naturally a no-op.)
#[test]
fn empty_entries_no_panic() {
    let mut entries: Vec<SupportPlanEntry> = vec![];
    support_planner::smooth_branches(&mut entries, 100);
    assert!(entries.is_empty());
}
