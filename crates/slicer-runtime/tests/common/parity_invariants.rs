//! Structural parity comparator for native-vs-wasm dispatch contract tests.
//!
//! ADR-0042: parity gates are structural invariants over a tolerance, never
//! byte equality of floats. ADR-0056 Decision item 4: the native/wasm parity
//! gate is structural, not byte-equal. This module is self-tested by
//! `contract/parity_invariants_selftest_tdd.rs` before any pilot module is
//! compared, so it cannot certify a broken path vacuously.

use slicer_ir::{
    ExtrusionPath3D, LayerStageCommit, PerimeterIR, PerimeterRegion, Point3WithWidth,
    SupportPlanIR, WallLoop,
};
use slicer_runtime::PrepassStageOutput;

/// Tolerances for the structural parity gate. No float is ever compared with
/// `==`; every comparison is within one of these tolerances.
#[derive(Debug, Clone, Copy)]
pub struct ParityTolerance {
    /// Per-coordinate (and per-width) tolerance in millimeters.
    pub coord_mm: f32,
    /// Loop-closure tolerance in millimeters (first ≈ last point, XY).
    pub closure_mm: f32,
    /// Upper bound on any bead width as a multiple of the loop's optimal width.
    pub max_bead_width_factor: f32,
}

impl Default for ParityTolerance {
    fn default() -> Self {
        Self {
            coord_mm: 1e-3,
            closure_mm: 1e-3,
            max_bead_width_factor: 2.0,
        }
    }
}

// ── Layer family: LayerStageCommit::Perimeters ─────────────────────────────

/// Structural parity gate for the per-layer commit family. Both commits must
/// be `LayerStageCommit::Perimeters`; a variant mismatch is itself a parity
/// failure. Returns `Err` naming the first violated invariant.
pub fn assert_parity_structural(
    native: &LayerStageCommit,
    wasm: &LayerStageCommit,
    tol: ParityTolerance,
    optimal_width_mm: f32,
) -> Result<(), String> {
    match (native, wasm) {
        (LayerStageCommit::Perimeters(n), LayerStageCommit::Perimeters(w)) => {
            compare_perimeter_ir(n, w, &tol, optimal_width_mm)
        }
        _ => Err(format!(
            "variant mismatch: native={} wasm={} \
             (assert_parity_structural requires LayerStageCommit::Perimeters on both paths)",
            commit_variant_name(native),
            commit_variant_name(wasm)
        )),
    }
}

fn commit_variant_name(commit: &LayerStageCommit) -> &'static str {
    commit.stage_id().unwrap_or("SeedLayerCollection")
}

fn compare_perimeter_ir(
    native: &PerimeterIR,
    wasm: &PerimeterIR,
    tol: &ParityTolerance,
    optimal_width_mm: f32,
) -> Result<(), String> {
    if native.regions.len() != wasm.regions.len() {
        return Err(format!(
            "region count mismatch: native={} wasm={}",
            native.regions.len(),
            wasm.regions.len()
        ));
    }
    for n_region in &native.regions {
        let w_region = wasm
            .regions
            .iter()
            .find(|r| r.object_id == n_region.object_id && r.region_id == n_region.region_id)
            .ok_or_else(|| {
                format!(
                    "region key set mismatch: wasm missing region (object_id={}, region_id={})",
                    n_region.object_id, n_region.region_id
                )
            })?;
        compare_region(n_region, w_region, tol, optimal_width_mm)?;
    }
    // (h) symmetric coverage ratio, mirrored on the model's
    // `symmetric_coverage_ratio` (min extent / max extent).
    let n_cov = coverage_measure(native);
    let w_cov = coverage_measure(wasm);
    let (lo, hi) = (n_cov.min(w_cov), n_cov.max(w_cov));
    let ratio = if hi > 0.0 { lo / hi } else { 1.0 };
    if ratio < 1.0 - tol.coord_mm {
        return Err(format!(
            "coverage ratio mismatch: native_coverage={n_cov:.6} wasm_coverage={w_cov:.6} \
             symmetric ratio {ratio:.6} < {:.6}",
            1.0 - tol.coord_mm
        ));
    }
    Ok(())
}

fn compare_region(
    native: &PerimeterRegion,
    wasm: &PerimeterRegion,
    tol: &ParityTolerance,
    optimal_width_mm: f32,
) -> Result<(), String> {
    let region_label = format!(
        "region (object_id={}, region_id={})",
        native.object_id, native.region_id
    );
    // (a) loop count.
    if native.walls.len() != wasm.walls.len() {
        return Err(format!(
            "loop count mismatch in {region_label}: native={} wasm={}",
            native.walls.len(),
            wasm.walls.len()
        ));
    }
    for (idx, (n_wall, w_wall)) in native.walls.iter().zip(&wasm.walls).enumerate() {
        compare_wall(
            n_wall,
            w_wall,
            tol,
            optimal_width_mm,
            &format!("{region_label} wall {idx}"),
        )?;
    }
    Ok(())
}

fn compare_wall(
    native: &WallLoop,
    wasm: &WallLoop,
    tol: &ParityTolerance,
    optimal_width_mm: f32,
    label: &str,
) -> Result<(), String> {
    // (a) loop nesting depth sequence. The IR carries no explicit depth field;
    // `perimeter_index` (0 = outermost) is the per-loop nesting-depth proxy.
    if native.perimeter_index != wasm.perimeter_index {
        return Err(format!(
            "loop nesting depth mismatch at {label}: native perimeter_index={} wasm={}",
            native.perimeter_index, wasm.perimeter_index
        ));
    }
    let n_pts = &native.path.points;
    let w_pts = &wasm.path.points;
    // (b) per-loop point count.
    if n_pts.len() != w_pts.len() {
        return Err(format!(
            "point count mismatch at {label}: native={} wasm={}",
            n_pts.len(),
            w_pts.len()
        ));
    }
    // (c) closure within closure_mm on BOTH paths (first ≈ last point, XY).
    for (side, wall) in [("native", native), ("wasm", wasm)] {
        let pts = &wall.path.points;
        let (first, last) = match (pts.first(), pts.last()) {
            (Some(f), Some(l)) if pts.len() >= 2 => (f, l),
            _ => {
                return Err(format!(
                    "closure violation at {label} ({side}): loop has {} points (< 2)",
                    pts.len()
                ))
            }
        };
        let gap = ((last.x - first.x).powi(2) + (last.y - first.y).powi(2)).sqrt();
        if gap > tol.closure_mm {
            return Err(format!(
                "closure violation at {label} ({side}): first/last gap {gap:.6} mm \
                 > closure_mm {}",
                tol.closure_mm
            ));
        }
    }
    // Per-point (x, y, z, width) within coord_mm.
    for (i, (np, wp)) in n_pts.iter().zip(w_pts).enumerate() {
        let d = (np.x - wp.x)
            .abs()
            .max((np.y - wp.y).abs())
            .max((np.z - wp.z).abs())
            .max((np.width - wp.width).abs());
        if d > tol.coord_mm {
            return Err(format!(
                "point coordinates mismatch at {label} point {i}: max component delta {d:.6} mm \
                 > coord_mm {}",
                tol.coord_mm
            ));
        }
    }
    // (d)+(i) bead counts use the resolved module width supplied by the test.
    let n_beads = bead_counts(n_pts, optimal_width_mm);
    let w_beads = bead_counts(w_pts, optimal_width_mm);
    // (d) bead-count sequence per loop.
    if n_beads != w_beads {
        return Err(format!(
            "bead-count sequence mismatch at {label}: native={n_beads:?} wasm={w_beads:?}"
        ));
    }
    // (e) transitions-present: identical positions AND directions of
    // bead-count change between adjacent positions.
    let n_trans = transitions(&n_beads);
    let w_trans = transitions(&w_beads);
    if n_trans != w_trans {
        return Err(format!(
            "transitions-present mismatch at {label}: native={n_trans:?} wasm={w_trans:?}"
        ));
    }
    // (i) no bead wider than max_bead_width_factor × optimal_width on
    // EITHER path.
    let bound = tol.max_bead_width_factor * optimal_width_mm;
    for (side, pts) in [("native", n_pts), ("wasm", w_pts)] {
        for (i, p) in pts.iter().enumerate() {
            if p.width > bound {
                return Err(format!(
                    "max bead width violation at {label} ({side}) point {i}: width {:.6} \
                         > {} × optimal_width {:.6}",
                    p.width, tol.max_bead_width_factor, optimal_width_mm
                ));
            }
        }
    }
    // (f) ExtrusionRole sequence per loop.
    if native.path.role != wasm.path.role {
        return Err(format!(
            "ExtrusionRole sequence mismatch at {label}: native={:?} wasm={:?}",
            native.path.role, wasm.path.role
        ));
    }
    // (g) no self-intersection in any loop, on EITHER path.
    for (side, wall) in [("native", native), ("wasm", wasm)] {
        if let Some((i, j)) = first_self_intersection(&wall.path.points) {
            return Err(format!(
                "self-intersection at {label} ({side}): segments {i} and {j} cross"
            ));
        }
    }
    Ok(())
}

fn bead_counts(points: &[Point3WithWidth], optimal: f32) -> Vec<i64> {
    points
        .iter()
        .map(|p| (p.width / optimal).round() as i64)
        .collect()
}

/// Positions (point index where the new count begins) and directions (+1/-1)
/// of bead-count changes between adjacent positions.
fn transitions(beads: &[i64]) -> Vec<(usize, i64)> {
    beads
        .windows(2)
        .enumerate()
        .filter(|(_, w)| w[0] != w[1])
        .map(|(i, w)| (i + 1, (w[1] - w[0]).signum()))
        .collect()
}

/// O(n²) XY segment-intersection scan over the closed loop, skipping adjacent
/// segment pairs (which share an endpoint). Returns the first crossing pair.
fn first_self_intersection(points: &[Point3WithWidth]) -> Option<(usize, usize)> {
    let n = points.len();
    if n < 4 {
        return None;
    }
    let seg_count = n - 1; // closed loop: first == last, so segments are 0..n-1
    for i in 0..seg_count {
        for j in (i + 1)..seg_count {
            // Skip pairs sharing an endpoint (adjacent, incl. the closing pair).
            if j == i + 1 || (i == 0 && j == seg_count - 1) {
                continue;
            }
            if segments_cross(&points[i], &points[i + 1], &points[j], &points[j + 1]) {
                return Some((i, j));
            }
        }
    }
    None
}

fn segments_cross(
    a: &Point3WithWidth,
    b: &Point3WithWidth,
    c: &Point3WithWidth,
    d: &Point3WithWidth,
) -> bool {
    let orient = |p: &Point3WithWidth, q: &Point3WithWidth, r: &Point3WithWidth| {
        (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x)
    };
    let d1 = orient(c, d, a);
    let d2 = orient(c, d, b);
    let d3 = orient(a, b, c);
    let d4 = orient(a, b, d);
    ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
}

/// Coverage proxy per path: Σ over loops of segment XY length × mean endpoint
/// width. Compared symmetrically (min/max), mirroring the model's
/// `symmetric_coverage_ratio` shape.
fn coverage_measure(ir: &PerimeterIR) -> f32 {
    let mut total = 0.0;
    for region in &ir.regions {
        for wall in &region.walls {
            for pair in wall.path.points.windows(2) {
                let (a, b) = (&pair[0], &pair[1]);
                let len = ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
                total += len * (a.width + b.width) * 0.5;
            }
        }
    }
    total
}

// ── Prepass family: PrepassStageOutput::SupportPlan ────────────────────────

/// Structural parity gate for the prepass output family. Both outputs must be
/// `PrepassStageOutput::SupportPlan`; a variant mismatch is itself a parity
/// failure. Returns `Err` naming the first violated invariant.
pub fn assert_prepass_parity_structural(
    native: &PrepassStageOutput,
    wasm: &PrepassStageOutput,
    tol: ParityTolerance,
) -> Result<(), String> {
    match (native, wasm) {
        (PrepassStageOutput::SupportPlan(n), PrepassStageOutput::SupportPlan(w)) => {
            compare_support_plan_ir(n, w, &tol)
        }
        _ => Err(format!(
            "variant mismatch: native={} wasm={} \
             (assert_prepass_parity_structural requires PrepassStageOutput::SupportPlan on both paths)",
            prepass_variant_name(native),
            prepass_variant_name(wasm)
        )),
    }
}

fn prepass_variant_name(output: &PrepassStageOutput) -> &'static str {
    match output {
        PrepassStageOutput::None => "None",
        PrepassStageOutput::SurfaceClassification(_) => "SurfaceClassification",
        PrepassStageOutput::LayerPlan(_) => "LayerPlan",
        PrepassStageOutput::SeamPlan(_) => "SeamPlan",
        PrepassStageOutput::SupportPlan(_) => "SupportPlan",
        PrepassStageOutput::RegionMap(_) => "RegionMap",
        PrepassStageOutput::SupportGeometry(_) => "SupportGeometry",
        PrepassStageOutput::MeshAnalysisAuxiliary(_) => "MeshAnalysisAuxiliary",
    }
}

fn compare_support_plan_ir(
    native: &SupportPlanIR,
    wasm: &SupportPlanIR,
    tol: &ParityTolerance,
) -> Result<(), String> {
    if native.entries.len() != wasm.entries.len() {
        return Err(format!(
            "entries count mismatch: native={} wasm={}",
            native.entries.len(),
            wasm.entries.len()
        ));
    }
    // Sort by the FULL triple so duplicate keys remain distinct and are
    // compared pairwise rather than collapsed by a map.
    let mut native_entries: Vec<_> = native.entries.iter().collect();
    let mut wasm_entries: Vec<_> = wasm.entries.iter().collect();
    let key =
        |e: &slicer_ir::SupportPlanEntry| (e.global_layer_index, e.object_id.clone(), e.region_id);
    native_entries.sort_by_key(|entry| key(entry));
    wasm_entries.sort_by_key(|entry| key(entry));
    for (n_entry, w_entry) in native_entries.iter().zip(&wasm_entries) {
        if key(n_entry) != key(w_entry) {
            return Err(format!(
                "entry key set mismatch: native=({},{},{}) wasm=({},{},{})",
                n_entry.global_layer_index,
                n_entry.object_id,
                n_entry.region_id,
                w_entry.global_layer_index,
                w_entry.object_id,
                w_entry.region_id
            ));
        }
        let label = format!(
            "entry (global_layer_index={}, object_id={}, region_id={})",
            n_entry.global_layer_index, n_entry.object_id, n_entry.region_id
        );
        if n_entry.branch_segments.len() != w_entry.branch_segments.len() {
            return Err(format!(
                "branch_segments count mismatch at {label}: native={} wasm={}",
                n_entry.branch_segments.len(),
                w_entry.branch_segments.len()
            ));
        }
        for (seg_idx, (n_seg, w_seg)) in n_entry
            .branch_segments
            .iter()
            .zip(&w_entry.branch_segments)
            .enumerate()
        {
            compare_segment(n_seg, w_seg, tol, &format!("{label} segment {seg_idx}"))?;
        }
    }
    Ok(())
}

fn compare_segment(
    native: &ExtrusionPath3D,
    wasm: &ExtrusionPath3D,
    tol: &ParityTolerance,
    label: &str,
) -> Result<(), String> {
    if native.points.len() != wasm.points.len() {
        return Err(format!(
            "points count mismatch at {label}: native={} wasm={}",
            native.points.len(),
            wasm.points.len()
        ));
    }
    if native.role != wasm.role {
        return Err(format!(
            "segment role mismatch at {label}: native={:?} wasm={:?}",
            native.role, wasm.role
        ));
    }
    for (i, (np, wp)) in native.points.iter().zip(&wasm.points).enumerate() {
        let d = (np.x - wp.x)
            .abs()
            .max((np.y - wp.y).abs())
            .max((np.z - wp.z).abs())
            .max((np.width - wp.width).abs());
        if d > tol.coord_mm {
            return Err(format!(
                "point (x, y, z, width) mismatch at {label} point {i}: max component delta \
                 {d:.6} mm > coord_mm {}",
                tol.coord_mm
            ));
        }
    }
    Ok(())
}
