//! Programmatic fixture builders for the Arachne parity audit
//! (`crates/slicer-runtime/tests/arachne_parity.rs`).
//!
//! No external STL files are required: every fixture is constructed
//! in-process with `Point2::from_mm` / `mm_to_units` so the 1 unit = 100 nm
//! convention (`docs/08_coordinate_system.md`) is honoured verbatim.
//!
//! All builders are pure and deterministic; they exist solely so the parity
//! tests can construct minimal `ExPolygon` inputs that trigger specific
//! Arachne features (thin walls, overhangs, multi-wall regions, top surfaces).

#![allow(dead_code)]

use slicer_ir::{ExPolygon, Point2, Polygon, UNITS_PER_MM};

/// `Point2::from_mm` convenience wrapper kept here so tests read fluently.
pub fn p_mm(x: f32, y: f32) -> Point2 {
    Point2::from_mm(x, y)
}

/// Square `side_mm` × `side_mm` centred at the origin. The canonical
/// multi-wall Arachne input: deep enough that 2+ beads fit.
pub fn square_mm(side_mm: f32) -> ExPolygon {
    let half = side_mm / 2.0;
    ExPolygon {
        contour: Polygon {
            points: vec![
                p_mm(-half, -half),
                p_mm(half, -half),
                p_mm(half, half),
                p_mm(-half, half),
            ],
        },
        holes: Vec::new(),
    }
}

/// A thin rectangular strip of total thickness `thickness_mm` and length
/// `length_mm`. Used to exercise WideningBeadingStrategy / thin-wall
/// collapse: when `thickness_mm` < `min_feature_size` the feature should
/// be dropped; when `min_feature_size <= thickness < optimal_width` it
/// collapses to a single bead.
pub fn thin_strip_mm(thickness_mm: f32, length_mm: f32) -> ExPolygon {
    let half_t = thickness_mm / 2.0;
    let half_l = length_mm / 2.0;
    ExPolygon {
        contour: Polygon {
            points: vec![
                p_mm(-half_l, -half_t),
                p_mm(half_l, -half_t),
                p_mm(half_l, half_t),
                p_mm(-half_l, half_t),
            ],
        },
        holes: Vec::new(),
    }
}

/// A wide annulus (outer square minus a concentric square hole) producing a
/// region whose local thickness varies, exercising variable-width
/// distribution and transition bands.
pub fn annulus_mm(outer_mm: f32, hole_mm: f32) -> ExPolygon {
    let oh = outer_mm / 2.0;
    let hh = hole_mm / 2.0;
    ExPolygon {
        contour: Polygon {
            points: vec![p_mm(-oh, -oh), p_mm(oh, -oh), p_mm(oh, oh), p_mm(-oh, oh)],
        },
        holes: vec![Polygon {
            points: vec![p_mm(-hh, -hh), p_mm(-hh, hh), p_mm(hh, hh), p_mm(hh, -hh)],
        }],
    }
}

/// Convenience: read a manifest TOML file shipped with the module under test
/// (relative to the workspace root) and return its parsed `toml::Value`.
pub fn read_module_manifest(relative_path: &str) -> toml::Value {
    let manifest_text = std::fs::read_to_string(relative_path)
        .unwrap_or_else(|e| panic!("manifest {relative_path} unreadable: {e}"));
    toml::from_str(&manifest_text)
        .unwrap_or_else(|e| panic!("manifest {relative_path} unparseable: {e}"))
}

/// Re-export `UNITS_PER_MM` so tests need not import `slicer_ir` separately.
pub const UNITS_PER_MM_F64: f64 = UNITS_PER_MM;
