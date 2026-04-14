//! TDD tests for output capture types (TASK-048).
//!
//! These tests verify that InfillOutputCapture, PerimeterOutputCapture, and
//! SupportOutputCapture mirror the SDK output builder shapes for test inspection.

use slicer_ir::{
    ExPolygon, ExtrusionPath3D, ExtrusionRole, LoopType, Point3, Point3WithWidth, Polygon,
    WallBoundaryType, WallLoop, WidthProfile,
};
use slicer_test::capture::{InfillOutputCapture, PerimeterOutputCapture, SupportOutputCapture};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn dummy_path(role: ExtrusionRole) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: vec![Point3WithWidth {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            width: 0.4,
            flow_factor: 1.0,
        }],
        role,
        speed_factor: 1.0,
    }
}

fn dummy_wall_loop(loop_type: LoopType) -> WallLoop {
    WallLoop {
        perimeter_index: 0,
        loop_type,
        path: dummy_path(ExtrusionRole::OuterWall),
        width_profile: WidthProfile { widths: vec![0.4] },
        feature_flags: vec![],
        boundary_type: WallBoundaryType::ExteriorSurface,
    }
}

fn dummy_expolygon() -> ExPolygon {
    ExPolygon {
        contour: Polygon { points: vec![] },
        holes: vec![],
    }
}

// ===========================================================================
// InfillOutputCapture
// ===========================================================================

#[test]
fn infill_capture_new_is_empty() {
    let cap = InfillOutputCapture::new();
    assert!(cap.sparse_paths().is_empty());
    assert!(cap.solid_paths().is_empty());
    assert!(cap.ironing_paths().is_empty());
}

#[test]
fn infill_capture_push_sparse_path() {
    let mut cap = InfillOutputCapture::new();
    cap.push_sparse_path(dummy_path(ExtrusionRole::SparseInfill));
    assert_eq!(cap.sparse_paths().len(), 1);
    assert!(cap.solid_paths().is_empty());
    assert!(cap.ironing_paths().is_empty());
}

#[test]
fn infill_capture_push_solid_path() {
    let mut cap = InfillOutputCapture::new();
    cap.push_solid_path(dummy_path(ExtrusionRole::TopSolidInfill));
    assert!(cap.sparse_paths().is_empty());
    assert_eq!(cap.solid_paths().len(), 1);
    assert!(cap.ironing_paths().is_empty());
}

#[test]
fn infill_capture_push_ironing_path() {
    let mut cap = InfillOutputCapture::new();
    cap.push_ironing_path(dummy_path(ExtrusionRole::Ironing));
    assert!(cap.sparse_paths().is_empty());
    assert!(cap.solid_paths().is_empty());
    assert_eq!(cap.ironing_paths().len(), 1);
}

#[test]
fn infill_capture_category_isolation() {
    let mut cap = InfillOutputCapture::new();
    cap.push_sparse_path(dummy_path(ExtrusionRole::SparseInfill));
    cap.push_solid_path(dummy_path(ExtrusionRole::TopSolidInfill));
    cap.push_ironing_path(dummy_path(ExtrusionRole::Ironing));
    assert_eq!(cap.sparse_paths().len(), 1);
    assert_eq!(cap.solid_paths().len(), 1);
    assert_eq!(cap.ironing_paths().len(), 1);
}

#[test]
fn infill_capture_debug_impl() {
    let cap = InfillOutputCapture::new();
    let dbg = format!("{cap:?}");
    assert!(dbg.contains("InfillOutputCapture"));
}

// ===========================================================================
// PerimeterOutputCapture
// ===========================================================================

#[test]
fn perimeter_capture_new_is_empty() {
    let cap = PerimeterOutputCapture::new();
    assert!(cap.wall_loops().is_empty());
    assert!(cap.infill_areas().is_empty());
    assert!(cap.seam_candidates().is_empty());
}

#[test]
fn perimeter_capture_push_wall_loop() {
    let mut cap = PerimeterOutputCapture::new();
    cap.push_wall_loop(dummy_wall_loop(LoopType::Outer));
    assert_eq!(cap.wall_loops().len(), 1);
    assert_eq!(cap.wall_loops()[0].loop_type, LoopType::Outer);
}

#[test]
fn perimeter_capture_set_infill_areas() {
    let mut cap = PerimeterOutputCapture::new();
    cap.set_infill_areas(vec![dummy_expolygon(), dummy_expolygon()]);
    assert_eq!(cap.infill_areas().len(), 2);
}

#[test]
fn perimeter_capture_set_infill_areas_replaces() {
    let mut cap = PerimeterOutputCapture::new();
    cap.set_infill_areas(vec![dummy_expolygon()]);
    cap.set_infill_areas(vec![dummy_expolygon(), dummy_expolygon()]);
    assert_eq!(cap.infill_areas().len(), 2);
}

#[test]
fn perimeter_capture_push_seam_candidate() {
    let mut cap = PerimeterOutputCapture::new();
    let pt = Point3 {
        x: 1.0,
        y: 2.0,
        z: 3.0,
    };
    cap.push_seam_candidate(pt, 0.75);
    assert_eq!(cap.seam_candidates().len(), 1);
    assert_eq!(cap.seam_candidates()[0].0, pt);
    assert!((cap.seam_candidates()[0].1 - 0.75).abs() < f32::EPSILON);
}

#[test]
fn perimeter_capture_debug_impl() {
    let cap = PerimeterOutputCapture::new();
    let dbg = format!("{cap:?}");
    assert!(dbg.contains("PerimeterOutputCapture"));
}

// ===========================================================================
// SupportOutputCapture
// ===========================================================================

#[test]
fn support_capture_new_is_empty() {
    let cap = SupportOutputCapture::new();
    assert!(cap.support_paths().is_empty());
    assert!(cap.interface_paths().is_empty());
    assert!(cap.raft_paths().is_empty());
}

#[test]
fn support_capture_push_support_path() {
    let mut cap = SupportOutputCapture::new();
    cap.push_support_path(dummy_path(ExtrusionRole::SupportMaterial));
    assert_eq!(cap.support_paths().len(), 1);
    assert!(cap.interface_paths().is_empty());
    assert!(cap.raft_paths().is_empty());
}

#[test]
fn support_capture_push_interface_path() {
    let mut cap = SupportOutputCapture::new();
    cap.push_interface_path(dummy_path(ExtrusionRole::SupportInterface), true);
    cap.push_interface_path(dummy_path(ExtrusionRole::SupportInterface), false);
    assert_eq!(cap.interface_paths().len(), 2);
    assert!(cap.interface_paths()[0].1); // is_top_interface = true
    assert!(!cap.interface_paths()[1].1); // is_top_interface = false
}

#[test]
fn support_capture_push_raft_path() {
    let mut cap = SupportOutputCapture::new();
    cap.push_raft_path(dummy_path(ExtrusionRole::SupportMaterial));
    assert!(cap.support_paths().is_empty());
    assert!(cap.interface_paths().is_empty());
    assert_eq!(cap.raft_paths().len(), 1);
}

#[test]
fn support_capture_category_isolation() {
    let mut cap = SupportOutputCapture::new();
    cap.push_support_path(dummy_path(ExtrusionRole::SupportMaterial));
    cap.push_interface_path(dummy_path(ExtrusionRole::SupportInterface), true);
    cap.push_raft_path(dummy_path(ExtrusionRole::SupportMaterial));
    assert_eq!(cap.support_paths().len(), 1);
    assert_eq!(cap.interface_paths().len(), 1);
    assert_eq!(cap.raft_paths().len(), 1);
}

#[test]
fn support_capture_debug_impl() {
    let cap = SupportOutputCapture::new();
    let dbg = format!("{cap:?}");
    assert!(dbg.contains("SupportOutputCapture"));
}

// ===========================================================================
// Default trait
// ===========================================================================

#[test]
fn all_captures_implement_default() {
    let _i: InfillOutputCapture = Default::default();
    let _p: PerimeterOutputCapture = Default::default();
    let _s: SupportOutputCapture = Default::default();
}
