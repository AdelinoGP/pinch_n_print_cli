//! Independent analytic band model for `H-PAINT-OVERHANG`.
//!
//! `annotate_overhangs` (`crates/slicer-core/src/algos/overhang_annotation.rs`)
//! partitions `current \ previous` into four distance bands measured from the
//! previous layer's boundary, at `0.5 / 1.0 / 1.5 x line_width`, with band 4
//! holding the remainder (never a fixed `2.0 x` cutoff).
//!
//! The fixture is the packet's X-strip scene: two rectangles with the same Y
//! extent, the current one wider by `shift` along +X. Every point of the strip
//! that is not within `1.5 x line_width` of the rectangle's top or bottom corner
//! has its nearest previous-boundary point on the straight right edge, so the
//! band boundaries are exact X intervals. Cell midpoints are kept well clear of
//! every threshold and of the corners, so the offset's `0.01 mm` arc tolerance
//! cannot flip a classification.

use std::collections::BTreeMap;

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use slicer_core::algos::overhang_annotation::annotate_overhangs;
use slicer_ir::{ExPolygon, PaintValue, Point2, Polygon};
use slicer_sdk::test_support::property_testing::oracles::{point_in_region, PointLocation};
use slicer_sdk::test_support::property_testing::{CaseGuard, PropertyRun};

/// Fixed previous-layer rectangle: `[0, WIDTH] x [Y0, Y1]` millimetres.
const WIDTH_MM: f32 = 10.0;
/// Lower Y edge of both rectangles (whole millimetres).
const Y0_MM: f32 = 0.0;
/// Upper Y edge of both rectangles.
const Y1_MM: f32 = 10.0;
/// The row of cells the model classifies, far from the rounded corners.
const ROW_Y0_MM: i64 = 400;
const ROW_Y1_MM: i64 = 600;

/// One generated X-strip scene.
#[derive(Debug, Clone)]
pub struct OverhangCase {
    /// Line width in millimetres; drives the three interior thresholds.
    pub line_width_mm: f32,
    /// How far the current rectangle extends past the previous one, in
    /// millimetres. Always greater than `2 * line_width_mm` so all four bands
    /// are observable.
    pub shift_mm: f32,
    /// Interior X cut points of the classification grid, in canonical units,
    /// strictly between the previous right edge and the current right edge.
    pub cuts: Vec<i64>,
}

fn units(mm: f32) -> i64 {
    (mm * 10_000.0).round() as i64
}

fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(x0, y0),
                Point2::from_mm(x1, y0),
                Point2::from_mm(x1, y1),
                Point2::from_mm(x0, y1),
            ],
        },
        holes: Vec::new(),
    }
}

impl OverhangCase {
    /// The previous-layer footprint.
    #[must_use]
    pub fn previous(&self) -> Vec<ExPolygon> {
        vec![rect(0.0, Y0_MM, WIDTH_MM, Y1_MM)]
    }

    /// The current-layer footprint.
    #[must_use]
    pub fn current(&self) -> Vec<ExPolygon> {
        vec![rect(0.0, Y0_MM, WIDTH_MM + self.shift_mm, Y1_MM)]
    }

    /// Canonical-unit X of the previous right edge.
    #[must_use]
    pub fn strip_start(&self) -> i64 {
        units(WIDTH_MM)
    }

    /// Canonical-unit X of the current right edge.
    #[must_use]
    pub fn strip_end(&self) -> i64 {
        units(WIDTH_MM + self.shift_mm)
    }

    /// The three interior band boundaries, in canonical units.
    #[must_use]
    pub fn thresholds(&self) -> [i64; 3] {
        [
            units(WIDTH_MM + self.line_width_mm * 0.5),
            units(WIDTH_MM + self.line_width_mm * 1.0),
            units(WIDTH_MM + self.line_width_mm * 1.5),
        ]
    }

    /// The expected quartile of a cell given its X midpoint.
    #[must_use]
    pub fn expected_band(&self, x_mid: i64) -> u8 {
        let [first, second, third] = self.thresholds();
        if x_mid < first {
            1
        } else if x_mid < second {
            2
        } else if x_mid < third {
            3
        } else {
            4
        }
    }

    /// Cells of the classified row, in increasing X order.
    ///
    /// Boundaries are the strip edges plus every interior cut; each cell spans
    /// `[x0, x1] x [ROW_Y0, ROW_Y1]` in canonical units.
    #[must_use]
    pub fn cells(&self) -> Vec<(i64, i64)> {
        let mut boundaries = vec![self.strip_start(), self.strip_end()];
        boundaries.extend(
            self.cuts
                .iter()
                .copied()
                .filter(|cut| *cut > self.strip_start() && *cut < self.strip_end()),
        );
        boundaries.sort_unstable();
        boundaries.dedup();
        boundaries
            .windows(2)
            .map(|pair| (pair[0], pair[1]))
            .collect()
    }
}

/// Check the real classifier against the analytic model.
pub fn verify(case: &OverhangCase) -> Result<(), String> {
    let footprints = vec![(0_u32, case.previous()), (1_u32, case.current())];
    let (bands_by_layer, previous_by_layer) = annotate_overhangs(&footprints, case.line_width_mm);
    verify_observation(case, &bands_by_layer, &previous_by_layer)
}

/// Check an arbitrary band/previous observation against the model.
///
/// The deterministic controls mutate a real observation and feed it back here.
pub fn verify_observation(
    case: &OverhangCase,
    bands_by_layer: &std::collections::HashMap<u32, Vec<slicer_ir::QuartileBand>>,
    previous_by_layer: &std::collections::HashMap<u32, Vec<ExPolygon>>,
) -> Result<(), String> {
    let bands = bands_by_layer
        .get(&1)
        .ok_or_else(|| String::from("layer 1 must carry an overhang band partition"))?;
    let previous = previous_by_layer
        .get(&1)
        .ok_or_else(|| String::from("layer 1 must carry its previous-boundary footprint"))?;
    if previous != &case.previous() {
        return Err(format!(
            "previous-boundary map holds {previous:?}, expected {:?}",
            case.previous()
        ));
    }

    // Every band's polygons must stay inside the current footprint's extent.
    for band in bands {
        for polygon in &band.polygons {
            for point in &polygon.contour.points {
                if point.x < 0
                    || point.x > case.strip_end()
                    || point.y < units(Y0_MM)
                    || point.y > units(Y1_MM)
                {
                    return Err(format!(
                        "band {} leaves the current extent at {point:?}",
                        band.quartile
                    ));
                }
            }
        }
    }

    // The quartile set must be exactly the observable band set of this scene.
    let mut expected_bands: BTreeMap<i64, u8> = BTreeMap::new();
    for (x0, x1) in case.cells() {
        let mid = (x0 + x1) / 2;
        expected_bands.insert(mid, case.expected_band(mid));
    }
    let expected_quartiles: std::collections::BTreeSet<u8> =
        expected_bands.values().copied().collect();
    let actual_quartiles: std::collections::BTreeSet<u8> =
        bands.iter().map(|band| band.quartile).collect();
    if actual_quartiles != expected_quartiles {
        return Err(format!(
            "quartile set {actual_quartiles:?} != expected {expected_quartiles:?}"
        ));
    }

    // Exact cell ownership: each cell midpoint is inside exactly the band the
    // analytic X-interval model predicts.
    for (mid, expected) in &expected_bands {
        let point = Point2 {
            x: *mid,
            y: (ROW_Y0_MM + ROW_Y1_MM) / 2,
        };
        let mut owners = Vec::new();
        for band in bands {
            match point_in_region(point, &band.polygons) {
                Ok(PointLocation::Inside) => owners.push(band.quartile),
                Ok(PointLocation::Outside) => {}
                Ok(PointLocation::Boundary) => {
                    return Err(format!("cell midpoint {point:?} landed on a boundary"))
                }
                Err(error) => return Err(format!("point_in_region failed: {error:?}")),
            }
        }
        if owners.len() != 1 {
            return Err(format!(
                "cell midpoint {point:?} is owned by {} bands: {owners:?}",
                owners.len()
            ));
        }
        if owners[0] != *expected {
            return Err(format!(
                "cell midpoint {point:?} is in band {} not {expected}",
                owners[0]
            ));
        }
    }
    Ok(())
}

/// The no-overhang modes: identical footprints, a single layer and an empty
/// current slice must all leave both maps without a key for layer 1.
pub fn no_overhang_modes() -> Result<(), String> {
    let previous = vec![(0_u32, vec![rect(0.0, Y0_MM, WIDTH_MM, Y1_MM)])];

    // Identical consecutive footprints.
    let identical = vec![
        (0_u32, vec![rect(0.0, Y0_MM, WIDTH_MM, Y1_MM)]),
        (1_u32, vec![rect(0.0, Y0_MM, WIDTH_MM, Y1_MM)]),
    ];
    let (bands, prev) = annotate_overhangs(&identical, 0.4);
    if bands.contains_key(&1) || prev.contains_key(&1) {
        return Err(String::from(
            "identical footprints must not be reported as overhanging",
        ));
    }

    // Absent predecessor.
    let single = vec![(0_u32, vec![rect(0.0, Y0_MM, WIDTH_MM, Y1_MM)])];
    let (bands, prev) = annotate_overhangs(&single, 0.4);
    if !bands.is_empty() || !prev.is_empty() {
        return Err(String::from(
            "a single layer has no predecessor and must not overhang",
        ));
    }

    // Empty current slice after a nonempty predecessor.
    let empty_current = vec![
        (0_u32, vec![rect(0.0, Y0_MM, WIDTH_MM, Y1_MM)]),
        (1_u32, Vec::new()),
    ];
    let (bands, prev) = annotate_overhangs(&empty_current, 0.4);
    if bands.contains_key(&1) || prev.contains_key(&1) {
        return Err(String::from("an empty current slice must not overhang"));
    }

    // Sanity: `previous` is still the fixture this module declares.
    if previous.len() != 1 {
        return Err(String::from("fixture error"));
    }
    Ok(())
}

/// Strategy for one generated scene: a positive line width and a shift that
/// always exceeds `2 x line_width`, so all four bands are observable.
pub fn strategy(max_band_width_units: i64) -> impl Strategy<Value = OverhangCase> {
    let width_units = [(0.4_f32, 4_000_i64), (0.5, 5_000), (0.6, 6_000)];
    (0_usize..3, 3u8..=9).prop_map(move |(width_index, extra_tenths)| {
        let (line_width_mm, line_width_units) = width_units[width_index];
        // `shift > 2 * line_width`, capped so the strip stays inside the
        // rectangle's own X extent budget.
        let minimum = line_width_units * 2 + line_width_units / 2;
        let extra = i64::from(extra_tenths) * (line_width_units / 10).max(1);
        let shift_units = (minimum + extra).min(max_band_width_units.max(minimum + 1));
        let shift_mm = shift_units as f32 / 10_000.0;
        let case = OverhangCase {
            line_width_mm,
            shift_mm,
            cuts: Vec::new(),
        };
        // Two interior cuts per band keep the grid populated without placing a
        // boundary near a threshold.
        let [first, second, third] = case.thresholds();
        let start = case.strip_start();
        let end = case.strip_end();
        let mut cuts = Vec::new();
        for boundary in [start, first, second, third, end] {
            cuts.push(boundary);
        }
        // Quarter points of each band, so a band with width >= 4 units has at
        // least one cell strictly inside it.
        for (left, right) in [
            (start, first),
            (first, second),
            (second, third),
            (third, end),
        ] {
            let quarter = (right - left) / 4;
            if quarter > 0 {
                cuts.push(left + quarter);
                cuts.push(right - quarter);
            }
        }
        OverhangCase {
            line_width_mm,
            shift_mm,
            cuts,
        }
    })
}

/// Registry entry point for `H-PAINT-OVERHANG`.
pub fn run_property() -> Result<(), String> {
    let run = PropertyRun::new("H-PAINT-OVERHANG", "proptest_overhang_adjacent_layer_bands")
        .map_err(|error| error.to_string())?;
    let max_band_width_units = i64::try_from(
        run.bound("max_band_width_units")
            .map_err(|e| e.to_string())?,
    )
    .map_err(|_| String::from("max_band_width_units does not fit an i64"))?;
    let max_layers = run.bound("max_layers").map_err(|e| e.to_string())?;
    let max_abs_units = run.bound("max_abs_units").map_err(|e| e.to_string())?;
    let max_cells = run.bound("max_cells").map_err(|e| e.to_string())?;
    if max_layers < 2 {
        return Err(String::from("max_layers must admit a predecessor layer"));
    }
    if max_abs_units < u64::try_from(units(WIDTH_MM)).unwrap_or(u64::MAX) {
        return Err(String::from("max_abs_units is below the fixture width"));
    }
    if max_cells < 4 {
        return Err(String::from(
            "max_cells must admit at least one cell per band",
        ));
    }

    no_overhang_modes()?;

    // The designated positive witness: shift > 2 x line_width, so all four
    // bands, including the uncapped remainder, are observable.
    let witness = OverhangCase {
        line_width_mm: 0.4,
        shift_mm: 1.2,
        cuts: {
            let case = OverhangCase {
                line_width_mm: 0.4,
                shift_mm: 1.2,
                cuts: Vec::new(),
            };
            let [first, second, third] = case.thresholds();
            let start = case.strip_start();
            let end = case.strip_end();
            let mut cuts = vec![start, first, second, third, end];
            for (left, right) in [
                (start, first),
                (first, second),
                (second, third),
                (third, end),
            ] {
                let quarter = (right - left) / 4;
                if quarter > 0 {
                    cuts.push(left + quarter);
                    cuts.push(right - quarter);
                }
            }
            cuts
        },
    };
    verify(&witness)?;

    let generated = strategy(max_band_width_units);
    run.check(&generated, |case: OverhangCase, _guard: &mut CaseGuard| {
        verify(&case).map_err(TestCaseError::fail)
    })
    .map_err(|error| error.to_string())
}

/// Convenience alias used by the deterministic controls.
pub fn observe(
    case: &OverhangCase,
) -> (
    std::collections::HashMap<u32, Vec<slicer_ir::QuartileBand>>,
    std::collections::HashMap<u32, Vec<ExPolygon>>,
) {
    let footprints = vec![(0_u32, case.previous()), (1_u32, case.current())];
    annotate_overhangs(&footprints, case.line_width_mm)
}

/// Paint-value helper kept so the module's imports stay honest if the fixture
/// ever gains a painted axis.
#[allow(dead_code)]
fn _unused_paint_value() -> PaintValue {
    PaintValue::Flag(true)
}
