//! Independent analytic area model for `H-PAINT-OVERHANG`.
//!
//! `annotate_overhangs` (`crates/slicer-core/src/algos/overhang_annotation.rs`)
//! reports, per overhanging layer, the unsupported area `current \ previous`
//! and the previous layer's footprint. Ticket 142 removed the producer-side
//! four-band distance partition: the consumer derives the quartile from the
//! signed distance to the previous boundary and the producer-supplied band
//! width, so this module checks the annotation's area and predecessor identity
//! instead of `QuartileBand` polygons.
//!
//! The fixture is the packet's X-strip scene: two rectangles with the same Y
//! extent, the current one wider by `shift` along +X. The unsupported area is
//! therefore the exact X strip `[WIDTH, WIDTH + shift] x [Y0, Y1]`, and a cell
//! midpoint a quarter cell inside each edge is unambiguous.

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use slicer_core::algos::overhang_annotation::{annotate_overhangs, OverhangAnnotation};
use slicer_ir::{ExPolygon, Point2, Polygon};
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
    /// How far the current rectangle extends past the previous one, in
    /// millimetres. Always positive, so the unsupported strip is nonempty.
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

    /// A cell-row point well inside the previous footprint (not the strip).
    #[must_use]
    fn previous_interior(&self) -> Point2 {
        Point2 {
            x: self.strip_start() / 2,
            y: (ROW_Y0_MM + ROW_Y1_MM) / 2,
        }
    }
}

/// Check the real annotation against the analytic model.
pub fn verify(case: &OverhangCase) -> Result<(), String> {
    let footprints = vec![(0_u32, case.previous()), (1_u32, case.current())];
    let annotation = annotate_overhangs(&footprints);
    verify_annotation(case, &annotation)
}

/// Check an arbitrary annotation against the model.
///
/// The deterministic controls mutate a real observation and feed it back here.
pub fn verify_annotation(
    case: &OverhangCase,
    annotation: &OverhangAnnotation,
) -> Result<(), String> {
    let areas = annotation
        .overhang_areas
        .get(&1)
        .ok_or_else(|| String::from("layer 1 must carry its unsupported area"))?;
    let previous = annotation
        .prev_layer_boundaries
        .get(&1)
        .ok_or_else(|| String::from("layer 1 must carry its previous-boundary footprint"))?;
    if previous != &case.previous() {
        return Err(format!(
            "previous-boundary map holds {previous:?}, expected {:?}",
            case.previous()
        ));
    }
    if annotation.overhang_areas.len() != 1 || annotation.prev_layer_boundaries.len() != 1 {
        return Err(format!(
            "only layer 1 can overhang, but the maps carry {} area(s) and {} predecessor(s)",
            annotation.overhang_areas.len(),
            annotation.prev_layer_boundaries.len()
        ));
    }

    // Every area vertex must stay inside the current footprint's extent.
    for polygon in areas {
        for point in &polygon.contour.points {
            if point.x < 0
                || point.x > case.strip_end()
                || point.y < units(Y0_MM)
                || point.y > units(Y1_MM)
            {
                return Err(format!(
                    "the unsupported area leaves the current extent at {point:?}"
                ));
            }
        }
    }

    // Exact cell ownership: every cell midpoint is inside the reported
    // unsupported area, and a point well inside the previous footprint is not.
    let row_y = (ROW_Y0_MM + ROW_Y1_MM) / 2;
    for (x0, x1) in case.cells() {
        let point = Point2 {
            x: (x0 + x1) / 2,
            y: row_y,
        };
        match point_in_region(point, areas) {
            Ok(PointLocation::Inside) => {}
            Ok(PointLocation::Outside) => {
                return Err(format!(
                    "cell midpoint {point:?} is outside the reported unsupported area"
                ))
            }
            Ok(PointLocation::Boundary) => {
                return Err(format!("cell midpoint {point:?} landed on a boundary"))
            }
            Err(error) => return Err(format!("point_in_region failed: {error:?}")),
        }
    }
    match point_in_region(case.previous_interior(), areas) {
        Ok(PointLocation::Outside) => {}
        Ok(location) => {
            return Err(format!(
                "a point inside the previous footprint was classified {location:?}, \
                 so the reported area is not `current \\ previous`"
            ))
        }
        Err(error) => return Err(format!("point_in_region failed: {error:?}")),
    }
    Ok(())
}

/// The no-overhang modes: identical footprints, a single layer and an empty
/// current slice must all leave both maps without a key for layer 1. A nonempty
/// current after an explicitly empty predecessor is the separate positive mode.
pub fn no_overhang_modes() -> Result<(), String> {
    // Identical consecutive footprints.
    let identical = vec![
        (0_u32, vec![rect(0.0, Y0_MM, WIDTH_MM, Y1_MM)]),
        (1_u32, vec![rect(0.0, Y0_MM, WIDTH_MM, Y1_MM)]),
    ];
    let annotation = annotate_overhangs(&identical);
    if annotation.overhang_areas.contains_key(&1)
        || annotation.prev_layer_boundaries.contains_key(&1)
    {
        return Err(String::from(
            "identical footprints must not be reported as overhanging",
        ));
    }

    // Absent predecessor.
    let single = vec![(0_u32, vec![rect(0.0, Y0_MM, WIDTH_MM, Y1_MM)])];
    let annotation = annotate_overhangs(&single);
    if !annotation.overhang_areas.is_empty() || !annotation.prev_layer_boundaries.is_empty() {
        return Err(String::from(
            "a single layer has no predecessor and must not overhang",
        ));
    }

    // Empty current slice after a nonempty predecessor.
    let empty_current = vec![
        (0_u32, vec![rect(0.0, Y0_MM, WIDTH_MM, Y1_MM)]),
        (1_u32, Vec::new()),
    ];
    let annotation = annotate_overhangs(&empty_current);
    if annotation.overhang_areas.contains_key(&1)
        || annotation.prev_layer_boundaries.contains_key(&1)
    {
        return Err(String::from("an empty current slice must not overhang"));
    }

    // Nonempty current after an explicitly empty predecessor: the whole
    // current footprint is unsupported, and the predecessor is present but
    // empty.
    let empty_previous = vec![
        (0_u32, Vec::new()),
        (1_u32, vec![rect(0.0, Y0_MM, WIDTH_MM, Y1_MM)]),
    ];
    let annotation = annotate_overhangs(&empty_previous);
    let areas = annotation
        .overhang_areas
        .get(&1)
        .ok_or_else(|| String::from("an empty predecessor must report the current area"))?;
    let previous = annotation
        .prev_layer_boundaries
        .get(&1)
        .ok_or_else(|| String::from("an empty predecessor entry must still be present"))?;
    if !previous.is_empty() {
        return Err(String::from(
            "an explicitly empty predecessor must be reported as an empty footprint",
        ));
    }
    match point_in_region(
        Point2 {
            x: units(WIDTH_MM) / 2,
            y: (ROW_Y0_MM + ROW_Y1_MM) / 2,
        },
        areas,
    ) {
        Ok(PointLocation::Inside) => {}
        Ok(location) => {
            return Err(format!(
                "the whole current footprint must be unsupported after an empty \
                 predecessor, but its interior point was {location:?}"
            ))
        }
        Err(error) => return Err(format!("point_in_region failed: {error:?}")),
    }
    Ok(())
}

/// Strategy for one generated scene: a positive shift capped by the registry
/// bound, with interior cuts on a quarter-point grid so every cell midpoint is
/// strictly inside the unsupported strip.
pub fn strategy(max_strip_units: i64) -> impl Strategy<Value = OverhangCase> {
    let minimum = 4_000_i64;
    let maximum = max_strip_units.max(minimum + 1);
    (minimum..=maximum).prop_map(move |shift_units| {
        let case = OverhangCase {
            shift_mm: shift_units as f32 / 10_000.0,
            cuts: Vec::new(),
        };
        let start = case.strip_start();
        let end = case.strip_end();
        let mut cuts = vec![start, end, (start + end) / 2];
        let quarter = (end - start) / 4;
        if quarter > 0 {
            cuts.push(start + quarter);
            cuts.push(end - quarter);
        }
        OverhangCase {
            shift_mm: case.shift_mm,
            cuts,
        }
    })
}

/// Registry entry point for `H-PAINT-OVERHANG`.
pub fn run_property() -> Result<(), String> {
    let run = PropertyRun::new("H-PAINT-OVERHANG", "proptest_overhang_adjacent_layer_areas")
        .map_err(|error| error.to_string())?;
    // The registry still caps the strip with `max_band_width_units`; since
    // ticket 142 the annotation no longer produces band polygons, so the bound
    // now caps the unsupported strip's width.
    let max_strip_units = i64::try_from(
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
    if max_cells < 2 {
        return Err(String::from(
            "max_cells must admit at least one positive and one negative cell",
        ));
    }

    no_overhang_modes()?;

    // The designated positive witness: a 1.2 mm unsupported strip with a
    // quarter-point grid.
    let witness = OverhangCase {
        shift_mm: 1.2,
        cuts: {
            let case = OverhangCase {
                shift_mm: 1.2,
                cuts: Vec::new(),
            };
            let start = case.strip_start();
            let end = case.strip_end();
            let mut cuts = vec![start, end, (start + end) / 2];
            let quarter = (end - start) / 4;
            if quarter > 0 {
                cuts.push(start + quarter);
                cuts.push(end - quarter);
            }
            cuts
        },
    };
    verify(&witness)?;

    let generated = strategy(max_strip_units);
    run.check(&generated, |case: OverhangCase, _guard: &mut CaseGuard| {
        verify(&case).map_err(TestCaseError::fail)
    })
    .map_err(|error| error.to_string())
}

/// Observe the real annotation, for the deterministic controls.
pub fn observe(case: &OverhangCase) -> OverhangAnnotation {
    let footprints = vec![(0_u32, case.previous()), (1_u32, case.current())];
    annotate_overhangs(&footprints)
}
