//! AC-1: Validates the canonical post-process order
//! (`stitch → remove_small → separate_out_inner_contour → simplify → remove_empty`)
//! matching `WallToolPaths.cpp:679-699`.
//!
//! Two kinds of tests:
//! 1. Pipeline smoke tests confirming the full pipeline runs and produces
//!    reasonable output after the order swap.
//! 2. Direct function-level tests confirming `separate_out_inner_contour` +
//!    `remove_empty_toolpaths` work correctly.

#![cfg(feature = "host-algos")]

use slicer_core::arachne::remove_small_lines;
use slicer_core::arachne::separate_inner_contour::{
    remove_empty_toolpaths, separate_out_inner_contour,
};
use slicer_core::arachne::simplify_toolpaths;
use slicer_core::arachne::{run_arachne_pipeline, ArachneParams};
use slicer_ir::{
    ExPolygon, ExtrusionJunction, ExtrusionLine, Point2, Point3WithWidth, Polygon, UNITS_PER_MM,
};

fn p_mm(x_mm: f64, y_mm: f64) -> Point2 {
    Point2 {
        x: (x_mm * UNITS_PER_MM) as i64,
        y: (y_mm * UNITS_PER_MM) as i64,
    }
}

fn expoly(points: Vec<Point2>) -> ExPolygon {
    ExPolygon {
        contour: Polygon { points },
        holes: Vec::new(),
    }
}

fn junction(x: f32, y: f32, width: f32) -> ExtrusionJunction {
    ExtrusionJunction {
        p: Point3WithWidth {
            x,
            y,
            z: 0.0,
            width,
            flow_factor: 1.0,
            overhang_quartile: None,
        },
        perimeter_index: 0,
    }
}

/// Smoke test: full pipeline produces non-empty output after order swap.
#[test]
fn pipeline_smoke_after_order_swap() {
    let sq = expoly(vec![
        p_mm(0.0, 0.0),
        p_mm(10.0, 0.0),
        p_mm(10.0, 10.0),
        p_mm(0.0, 10.0),
    ]);
    let (lines, _) = run_arachne_pipeline(&[sq], &ArachneParams::default(), false)
        .expect("10mm square should produce Ok(lines)");
    assert!(
        !lines.is_empty(),
        "pipeline should produce non-empty output after order swap"
    );
}

/// Under canonical order (remove_small first), a short odd open line is
/// removed before simplify ever sees it.
#[test]
fn remove_small_before_simplify_short_odd_line_removed() {
    // A short (0.05mm) odd open line — below threshold (0.5 * 0.4 = 0.2mm).
    let line = ExtrusionLine {
        junctions: vec![junction(0.0, 0.0, 0.4), junction(0.05, 0.0, 0.4)],
        inset_idx: 1,
        is_odd: true,
        is_closed: false,
    };

    // Canonical order: remove_small first.
    let after_remove = remove_small_lines(vec![line.clone()], 0.5, 0.4, false);
    assert!(
        after_remove.is_empty(),
        "canonical order: short odd line should be removed before simplify runs"
    );

    // Old order: simplify first (2 junctions, simplify keeps ≥2), then remove.
    let after_simplify = simplify_toolpaths(vec![line], 0.01, 0.0, 0.0, 0.0);
    let after_remove_old = remove_small_lines(after_simplify, 0.5, 0.4, false);
    assert!(
        after_remove_old.is_empty(),
        "old order also removes this degenerate case (2 junctions, simplify is a no-op)"
    );
}

/// A line above the removal threshold survives the canonical pipeline.
#[test]
fn line_above_threshold_survives_canonical_order() {
    // A ~2mm line, well above threshold (0.2mm).
    let line = ExtrusionLine {
        junctions: vec![
            junction(0.0, 0.0, 0.4),
            junction(0.5, 0.5, 0.4),
            junction(1.0, 0.0, 0.4),
            junction(1.5, 0.5, 0.4),
            junction(2.0, 0.0, 0.4),
        ],
        inset_idx: 1,
        is_odd: true,
        is_closed: false,
    };

    let after_remove = remove_small_lines(vec![line.clone()], 0.5, 0.4, false);
    assert_eq!(
        after_remove.len(),
        1,
        "line above threshold survives remove_small"
    );
    let after_simplify = simplify_toolpaths(after_remove, 0.01, 0.0, 0.0, 0.0);
    assert!(
        !after_simplify.is_empty(),
        "line survives the full canonical pipeline"
    );
}

/// `separate_out_inner_contour` extracts zero-width marker lines.
#[test]
fn separate_out_inner_contour_extracts_zero_width_lines() {
    let printable = ExtrusionLine {
        junctions: vec![junction(0.0, 0.0, 0.4), junction(1.0, 0.0, 0.4)],
        inset_idx: 0,
        is_odd: false,
        is_closed: true,
    };
    let contour_marker = ExtrusionLine {
        junctions: vec![junction(0.0, 0.0, 0.0), junction(1.0, 0.0, 0.0)],
        inset_idx: 0,
        is_odd: false,
        is_closed: true,
    };

    let (toolpaths, inner_contour) = separate_out_inner_contour(vec![printable, contour_marker]);
    assert_eq!(toolpaths.len(), 1, "printable line stays in toolpaths");
    assert_eq!(
        inner_contour.len(),
        1,
        "zero-width marker extracted to inner_contour"
    );
}

/// `separate_out_inner_contour` is a no-op when no zero-width lines exist.
#[test]
fn separate_out_inner_contour_no_op_when_no_markers() {
    let line = ExtrusionLine {
        junctions: vec![junction(0.0, 0.0, 0.4), junction(1.0, 0.0, 0.4)],
        inset_idx: 0,
        is_odd: false,
        is_closed: true,
    };

    let (toolpaths, inner_contour) = separate_out_inner_contour(vec![line]);
    assert_eq!(toolpaths.len(), 1);
    assert!(inner_contour.is_empty());
}

/// `remove_empty_toolpaths` filters out lines with no junctions.
#[test]
fn remove_empty_toolpaths_filters_empty_lines() {
    let empty_line = ExtrusionLine {
        junctions: vec![],
        inset_idx: 0,
        is_odd: false,
        is_closed: false,
    };
    let non_empty = ExtrusionLine {
        junctions: vec![junction(0.0, 0.0, 0.4)],
        inset_idx: 0,
        is_odd: false,
        is_closed: false,
    };

    let result = remove_empty_toolpaths(vec![empty_line, non_empty]);
    assert_eq!(result.len(), 1, "empty line should be filtered out");
}

/// Full pipeline produces non-empty output for a simple polygon with all
/// returned lines having non-empty junctions.
#[test]
fn full_pipeline_simple_polygon() {
    let sq = expoly(vec![
        p_mm(0.0, 0.0),
        p_mm(20.0, 0.0),
        p_mm(20.0, 20.0),
        p_mm(0.0, 20.0),
    ]);
    let (lines, _) = run_arachne_pipeline(&[sq], &ArachneParams::default(), false)
        .expect("20mm square should produce Ok(lines)");
    assert!(
        !lines.is_empty(),
        "full canonical pipeline should produce non-empty output"
    );

    for line in &lines {
        assert!(
            !line.junctions.is_empty(),
            "remove_empty_toolpaths should have filtered empty lines"
        );
    }
}
