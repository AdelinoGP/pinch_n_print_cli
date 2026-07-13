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

use slicer_core::beading::factory::{BeadingFactoryParams, BeadingStrategyFactory};
use slicer_core::beading::BeadingStrategy;
use slicer_ir::{ExPolygon, ExtrusionJunction, Point2, Point3WithWidth, Polygon, UNITS_PER_MM};

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

// ===========================================================================
// Round-2 audit fixtures (gaps NOT covered by `arachne_parity_gaps.rs`).
//
// These extend the existing `ExPolygon`-builder convention that feeds the same
// `run_arachne_pipeline` / `ArachnePerimeters::run_perimeters` harness the
// sibling builders (`square_mm`, `thin_strip_mm`, `annulus_mm`) already serve.
// For G15 the fixture hands back a fully-built `Box<dyn BeadingStrategy>` so
// the red test can call the (currently unexposed) trait method directly — a
// TDD-red that fails to compile until the gap is closed.
// ===========================================================================

/// Two concentric square islands (outer 20 mm, inner 10 mm, same centre)
/// representing separate `ExPolygon` regions. Used to probe OrcaSlicer's
/// "odd-after-enclosing" region ordering (`WallToolPaths::getRegionOrder`,
/// `WallToolPaths.cpp:809`): the emitted wall regions must be ordered so that
/// an inner (odd) region follows the enclosing even region. The PnP pipeline
/// flattens the per-inset buckets in source order and performs no such
/// reordering (G12).
pub fn ex_polygons_concentric_islands_mm() -> Vec<ExPolygon> {
    vec![square_mm(20.0), square_mm(10.0)]
}

/// Builds the canonical Arachne beading-strategy stack via
/// `BeadingStrategyFactory::create_stack` with `print_thin_walls = true`
/// (so the `Widening` decorator is present, like `detect_thin_wall` on) and a
/// realistic `max_bead_count`. Returned as `Box<dyn BeadingStrategy>` so a red
/// test can call the (currently unexposed) `get_split_middle_threshold`
/// method (G15) — the call will not compile until the trait gains the method.
pub fn beading_stack_for_split_middle() -> Box<dyn BeadingStrategy> {
    let params = BeadingFactoryParams {
        print_thin_walls: true,
        ..BeadingFactoryParams::default()
    };
    BeadingStrategyFactory::create_stack(&params)
}

/// A polyline shaped as a thin "Z": four junctions where the two middle
/// segments are short and (almost) colinear with the long surrounding chord,
/// but the proposed intersection point of the surrounding chord lies farther
/// from the middle junction than `smallest_line_segment_squared` permits.
/// OrcaSlicer's `ExtrusionLine::simplify` (`Arachne/utils/ExtrusionLine.cpp:
/// 163-175`) uses a `dist_greater` predicate to reject removal in this
/// shape; the Rust impl (`crates/slicer-core/src/arachne/simplify.rs`)
/// drops the middle junction because it only checks `seg_len²` and
/// `height_2` (G20). Returned as a `Vec<ExtrusionJunction>` so the red
/// test can feed it into `simplify_extrusion_line` directly.
pub fn simplify_input_intersection_distance_gate() -> Vec<ExtrusionJunction> {
    fn j(x: f32, y: f32) -> ExtrusionJunction {
        ExtrusionJunction {
            p: Point3WithWidth {
                x,
                y,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
                overhang_quartile: None,
            },
            perimeter_index: 0,
        }
    }
    // Coordinates in millimetres. The Z chord (0,0)-(10,0) is the long
    // surrounding line; the middle junctions at (5, 0.05) and (5.01, 0.04)
    // are very close to it but colinearly extended, their intersection with
    // the chord sits far from (5, 0). The Rust impl's tier-3 will drop the
    // (5, 0.05) and (5.01, 0.04) junctions because each short segment
    // satisfies the per-step length and height tests; OrcaSlicer's
    // `dist_greater` gate (lines 163-175) keeps them because the
    // intersection of the long chord with each short segment lies more
    // than `smallest_line_segment_squared` from the surviving neighbor.
    vec![j(0.0, 0.0), j(5.0, 0.05), j(5.01, 0.04), j(10.0, 0.0)]
}
