//! Independent contact model for `H-SUPPORT-CONTACT`.
//!
//! `detect_support_contacts` (`crates/slicer-core/src/algos/overhang_annotation.rs`)
//! on the **enforced-layer plain-difference branch** computes
//! `upper − lower − blockers` (no offset, no expand-back, no sharp tails, no
//! bridge margin), then drops contacts that erode away under a tenth of the
//! line width, then unions. This model builds that set on an even cell lattice
//! and classifies the result with `oracles::point_in_region`, so the oracle is
//! never the production Boolean.

use std::collections::BTreeSet;

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use slicer_core::algos::overhang_annotation::{detect_support_contacts, SupportContactParams};
use slicer_ir::{ExPolygon, Point2, Polygon};
use slicer_sdk::test_support::property_testing::oracles::{point_in_region, PointLocation};
use slicer_sdk::test_support::property_testing::{CaseGuard, PropertyRun};

/// One generated contact scene on a 2 x 1 cell lattice.
#[derive(Debug, Clone)]
pub struct ContactCase {
    /// Upper-layer cells (index 0 = left, 1 = right).
    pub upper: Vec<bool>,
    /// Lower-layer cells.
    pub lower: Vec<bool>,
    /// Blocker cells.
    pub blocker: Vec<bool>,
    /// External perimeter width in millimetres.
    pub width_mm: f32,
}

// Millimetre-scale cells: the tiny-spot filter erodes by a tenth of the line
// width, so a fixture in raw units would vanish. 1 mm = 10_000 canonical units.
const MM: i64 = 10_000;
const WIDTH: i64 = 8 * MM;
const HEIGHT: i64 = 8 * MM;
const CUT: i64 = 4 * MM;

fn cell_rect(index: usize) -> ExPolygon {
    let (x0, x1) = if index == 0 { (0, CUT) } else { (CUT, WIDTH) };
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: x0, y: 0 },
                Point2 { x: x1, y: 0 },
                Point2 { x: x1, y: HEIGHT },
                Point2 { x: x0, y: HEIGHT },
            ],
        },
        holes: Vec::new(),
    }
}

fn cell_midpoint(index: usize) -> Point2 {
    Point2 {
        x: if index == 0 { CUT / 2 } else { CUT + CUT / 2 },
        y: HEIGHT / 2,
    }
}

/// The plain-difference parameters: layer 0 of an object with one enforced
/// support layer, no expansion and no optional branches.
pub fn params_for_test(width_mm: f32) -> SupportContactParams {
    params(width_mm)
}

fn params(width_mm: f32) -> SupportContactParams {
    // exhaustive: the plain-difference fixture pins every parameter on purpose.
    SupportContactParams {
        threshold_angle_deg: 30.0,
        lower_layer_height_mm: 0.2,
        external_perimeter_width_mm: width_mm,
        threshold_overlap_mm: width_mm * 0.5,
        xy_expansion_mm: 0.0,
        bridge_no_support: false,
        bridge_polygons: Vec::new(),
        support_sharp_tails: false,
        enforce_support_layers: 1,
        layer_id: 0,
        raft_layers: 0,
    }
}

pub fn collect_flags(flags: &[bool]) -> Vec<ExPolygon> {
    flags
        .iter()
        .enumerate()
        .filter(|(_, flag)| **flag)
        .map(|(index, _)| cell_rect(index))
        .collect()
}

impl ContactCase {
    /// The expected contact cells: upper, not lower, not blocker.
    #[must_use]
    pub fn expected_cells(&self) -> BTreeSet<usize> {
        (0..self.upper.len())
            .filter(|index| self.upper[*index] && !self.lower[*index] && !self.blocker[*index])
            .collect()
    }
}

/// Verify the real detector against the model.
pub fn verify(case: &ContactCase) -> Result<(), String> {
    let contacts = detect_support_contacts(
        &collect_flags(&case.upper),
        &collect_flags(&case.lower),
        &collect_flags(&case.blocker),
        &params(case.width_mm),
    );
    verify_observation(case, &contacts)
}

/// Verify an arbitrary contact observation against the model.
pub fn verify_observation(case: &ContactCase, contacts: &[ExPolygon]) -> Result<(), String> {
    // Contacts stay inside the upper extent.
    for polygon in contacts {
        for point in &polygon.contour.points {
            if point.x < 0 || point.x > WIDTH || point.y < 0 || point.y > HEIGHT {
                return Err(format!("contact leaves the upper extent at {point:?}"));
            }
        }
    }

    let expected = case.expected_cells();
    for index in 0..case.upper.len() {
        let point = cell_midpoint(index);
        let inside = match point_in_region(point, contacts) {
            Ok(PointLocation::Inside) => true,
            Ok(PointLocation::Outside) => false,
            Ok(PointLocation::Boundary) => {
                return Err(format!("cell midpoint {point:?} landed on a boundary"))
            }
            Err(error) => return Err(format!("point_in_region failed: {error:?}")),
        };
        let want = expected.contains(&index);
        if inside != want {
            return Err(format!(
                "cell {index} contact={inside}, expected {want} (upper={} lower={} blocker={}); contacts={contacts:?}",
                case.upper[index], case.lower[index], case.blocker[index]
            ));
        }
    }
    Ok(())
}

/// The tiny-spot filter and the supported-column modes.
pub fn edge_modes() -> Result<(), String> {
    // A 0.01 mm wide contact erodes away under 0.1 x 0.4 mm and is dropped.
    let sliver_upper = vec![ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: 0, y: 0 },
                Point2 { x: 100, y: 0 },
                Point2 { x: 100, y: HEIGHT },
                Point2 { x: 0, y: HEIGHT },
            ],
        },
        holes: Vec::new(),
    }];
    let contacts = detect_support_contacts(&sliver_upper, &[], &[], &params(0.4));
    if !contacts.is_empty() {
        return Err(String::from(
            "a sub-tenth-line-width contact must be dropped by the tiny-spot filter",
        ));
    }

    // An unchanged supported column has no contact.
    let column = vec![cell_rect(0)];
    let contacts = detect_support_contacts(&column, &column, &[], &params(0.4));
    if !contacts.is_empty() {
        return Err(String::from(
            "a fully supported column must produce no contact",
        ));
    }

    // A blocker-covered contact is removed.
    let contacts = detect_support_contacts(&column, &[], &column, &params(0.4));
    if !contacts.is_empty() {
        return Err(String::from("a blocker-covered contact must be removed"));
    }

    // An empty upper region is the no-work mode.
    if !detect_support_contacts(&[], &column, &[], &params(0.4)).is_empty() {
        return Err(String::from(
            "an empty upper region must produce no contact",
        ));
    }
    Ok(())
}

/// Strategy for one generated contact scene.
pub fn strategy() -> impl Strategy<Value = ContactCase> {
    (
        proptest::collection::vec(any::<bool>(), 2),
        proptest::collection::vec(any::<bool>(), 2),
        proptest::collection::vec(any::<bool>(), 2),
    )
        .prop_map(|(upper, lower, blocker)| ContactCase {
            upper,
            lower,
            blocker,
            width_mm: 0.4,
        })
}

/// Registry entry point for `H-SUPPORT-CONTACT`.
pub fn run_property() -> Result<(), String> {
    let run = PropertyRun::new("H-SUPPORT-CONTACT", "proptest_support_contact_rectangles")
        .map_err(|error| error.to_string())?;
    let _max_abs = run.bound("max_abs_units").map_err(|e| e.to_string())?;
    let _max_cells = run.bound("max_cells").map_err(|e| e.to_string())?;

    edge_modes()?;

    // Deterministic positive witness: upper only on the left cell.
    let witness = ContactCase {
        upper: vec![true, false],
        lower: vec![false, false],
        blocker: vec![false, false],
        width_mm: 0.4,
    };
    verify(&witness)?;

    let generated = strategy();
    run.check(&generated, |case: ContactCase, _guard: &mut CaseGuard| {
        verify(&case).map_err(TestCaseError::fail)
    })
    .map_err(|error| error.to_string())
}
