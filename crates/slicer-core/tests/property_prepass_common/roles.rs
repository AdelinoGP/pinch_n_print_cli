//! Independent role-partition model for `H-SUPPORT-ROLES`.
//!
//! `regularize_entry_roles` (`crates/slicer-core/src/support_regularize.rs`) is
//! modelled here in its **grid style** (`smooth_supports = false`), where the
//! regularizer is a plain union and the whole contract is exact set algebra:
//!
//! ```text
//! layer_area  = top ∪ bottom ∪ body
//! iface_top   = top ∩ layer_area                 (= top)
//! iface_bottom = (bottom ∩ layer_area) − iface_top
//! body_out    = layer_area − (iface_top ∪ iface_bottom)
//! ```
//!
//! Raft geometry is passed through untouched and is deliberately **not** part of
//! `layer_area`. The model uses an even-coordinate cell grid, so every cell
//! midpoint is an integer point and ownership can be classified with
//! `oracles::point_in_region` instead of an exact-area comparison.

use std::collections::BTreeSet;

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use slicer_core::support_regularize::regularize_entry_roles;
use slicer_ir::{ExPolygon, Point2, Polygon, SupportPlanRole, SupportPlanRoleRegion};
use slicer_sdk::test_support::property_testing::oracles::{point_in_region, PointLocation};
use slicer_sdk::test_support::property_testing::{CaseGuard, PropertyRun};

/// One cell label of the role fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellLabel {
    /// No role covers this cell.
    Absent,
    /// `TopInterface`.
    Top,
    /// `BottomInterface`.
    Bottom,
    /// `SupportBody` only.
    Body,
    /// `RaftRelated` (outside the support layer area).
    Raft,
}

/// One generated role scene.
#[derive(Debug, Clone)]
pub struct RoleCase {
    /// Even width in canonical units.
    pub width: i64,
    /// Even height in canonical units.
    pub height: i64,
    /// Interior X cuts (even).
    pub xs: Vec<i64>,
    /// Interior Y cuts (even).
    pub ys: Vec<i64>,
    /// Per-cell label, in `(row, column)` order.
    pub labels: Vec<CellLabel>,
    /// Input role order; a subset of the four role spellings, in draw order.
    pub role_order: Vec<u8>,
    /// `flow_spacing_mm`.
    pub flow_spacing_mm: f32,
    /// Top interface pitch.
    pub top_pitch_mm: f32,
    /// Bottom interface pitch.
    pub bottom_pitch_mm: f32,
}

/// Role spellings used by the fixture, in a fixed numeric encoding.
const TOP: u8 = 0;
const BOTTOM: u8 = 1;
const BODY: u8 = 2;
const RAFT: u8 = 3;

fn role_of(code: u8) -> SupportPlanRole {
    match code {
        TOP => SupportPlanRole::TopInterface,
        BOTTOM => SupportPlanRole::BottomInterface,
        BODY => SupportPlanRole::SupportBody,
        _ => SupportPlanRole::RaftRelated,
    }
}

fn cell_rect(cell: &Cell) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 {
                    x: cell.x0,
                    y: cell.y0,
                },
                Point2 {
                    x: cell.x1,
                    y: cell.y0,
                },
                Point2 {
                    x: cell.x1,
                    y: cell.y1,
                },
                Point2 {
                    x: cell.x0,
                    y: cell.y1,
                },
            ],
        },
        holes: Vec::new(),
    }
}

/// One open cell.
#[derive(Debug, Clone, Copy)]
pub struct Cell {
    /// Left edge.
    pub x0: i64,
    /// Right edge.
    pub x1: i64,
    /// Bottom edge.
    pub y0: i64,
    /// Top edge.
    pub y1: i64,
}

impl RoleCase {
    /// Cells in `(row, column)` order.
    #[must_use]
    pub fn cells(&self) -> Vec<Cell> {
        let mut xs = vec![0, self.width];
        xs.extend(self.xs.iter().copied());
        xs.sort_unstable();
        xs.dedup();
        let mut ys = vec![0, self.height];
        ys.extend(self.ys.iter().copied());
        ys.sort_unstable();
        ys.dedup();
        let mut cells = Vec::new();
        for row in 0..ys.len().saturating_sub(1) {
            for column in 0..xs.len().saturating_sub(1) {
                cells.push(Cell {
                    x0: xs[column],
                    x1: xs[column + 1],
                    y0: ys[row],
                    y1: ys[row + 1],
                });
            }
        }
        cells
    }

    /// The declared `roles` slice, in the drawn order.
    #[must_use]
    pub fn roles(&self) -> Vec<SupportPlanRoleRegion> {
        self.role_order
            .iter()
            .map(|code| SupportPlanRoleRegion {
                role: role_of(*code),
                regions: self
                    .labels
                    .iter()
                    .enumerate()
                    .filter(|(_, label)| cell_label_code(**label) == *code)
                    .map(|(index, _)| cell_rect(&self.cells()[index]))
                    .collect(),
            })
            .collect()
    }

    /// The role order the output must follow: the input order, with
    /// `SupportBody` appended last when it was absent from the input and its
    /// geometry is nonempty.
    #[must_use]
    pub fn expected_role_order(&self) -> Vec<SupportPlanRole> {
        let mut order: Vec<SupportPlanRole> =
            self.role_order.iter().map(|code| role_of(*code)).collect();
        // In grid style the layer area is the union of the top, bottom and body
        // cells, and the interface cells are always inside it, so the body
        // remainder is exactly the explicit body cells.
        let has_body_geometry = self.labels.contains(&CellLabel::Body);
        if has_body_geometry && !self.role_order.contains(&BODY) {
            order.push(SupportPlanRole::SupportBody);
        }
        order
    }
}

fn cell_label_code(label: CellLabel) -> u8 {
    match label {
        CellLabel::Top => TOP,
        CellLabel::Bottom => BOTTOM,
        CellLabel::Body => BODY,
        CellLabel::Raft => RAFT,
        CellLabel::Absent => 255,
    }
}

/// The expected owner of a cell.
fn expected_owner(label: CellLabel) -> Option<SupportPlanRole> {
    match label {
        CellLabel::Absent => None,
        CellLabel::Top => Some(SupportPlanRole::TopInterface),
        CellLabel::Bottom => Some(SupportPlanRole::BottomInterface),
        CellLabel::Body => Some(SupportPlanRole::SupportBody),
        CellLabel::Raft => Some(SupportPlanRole::RaftRelated),
    }
}

/// Verify the real partition against the model.
pub fn verify(case: &RoleCase) -> Result<(), String> {
    let roles = case.roles();
    let result = regularize_entry_roles(
        &roles,
        case.flow_spacing_mm,
        case.top_pitch_mm,
        case.bottom_pitch_mm,
        /* smooth_supports */ false,
    );

    let has_top = case.labels.contains(&CellLabel::Top);
    let has_bottom = case.labels.contains(&CellLabel::Bottom);
    if !has_top && !has_bottom {
        return match result {
            None => Ok(()),
            Some(out) => Err(format!("no interface input must return None, got {out:?}")),
        };
    }
    let Some(out) = result else {
        return Err(String::from(
            "an interface-bearing entry must regularize, not return None",
        ));
    };

    let actual_order: Vec<SupportPlanRole> = out.iter().map(|(role, _)| *role).collect();
    let expected_order = case.expected_role_order();
    if actual_order != expected_order {
        return Err(format!(
            "role order {actual_order:?} != expected {expected_order:?}"
        ));
    }

    for (index, cell) in case.cells().iter().enumerate() {
        let point = Point2 {
            x: (cell.x0 + cell.x1) / 2,
            y: (cell.y0 + cell.y1) / 2,
        };
        let expected = expected_owner(case.labels[index]);
        let mut owners: Vec<SupportPlanRole> = Vec::new();
        for (role, polygons) in &out {
            match point_in_region(point, polygons) {
                Ok(PointLocation::Inside) => owners.push(*role),
                Ok(PointLocation::Outside) => {}
                Ok(PointLocation::Boundary) => {
                    return Err(format!("cell midpoint {point:?} landed on a boundary"))
                }
                Err(error) => return Err(format!("point_in_region failed: {error:?}")),
            }
        }
        match expected {
            None => {
                if !owners.is_empty() {
                    return Err(format!("absent cell {point:?} is owned by {owners:?}"));
                }
            }
            Some(role) => {
                if owners.len() != 1 || owners[0] != role {
                    return Err(format!(
                        "cell {point:?} is owned by {owners:?}, expected [{role:?}]"
                    ));
                }
            }
        }
    }

    // Every populated role carries a nonempty population.
    for (role, polygons) in &out {
        if polygons.is_empty() {
            return Err(format!("role {role:?} is present but empty"));
        }
    }
    Ok(())
}

/// The documented `None` modes and the top/base aliasing.
pub fn none_and_alias_modes() -> Result<(), String> {
    let square = |offset: i64| ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: offset, y: 0 },
                Point2 {
                    x: offset + 20_000,
                    y: 0,
                },
                Point2 {
                    x: offset + 20_000,
                    y: 20_000,
                },
                Point2 {
                    x: offset,
                    y: 20_000,
                },
            ],
        },
        holes: Vec::new(),
    };

    // Nonpositive flow spacing.
    let roles = vec![SupportPlanRoleRegion {
        role: SupportPlanRole::TopInterface,
        regions: vec![square(0)],
    }];
    if regularize_entry_roles(&roles, 0.0, 0.4, 0.4, false).is_some() {
        return Err(String::from(
            "a nonpositive flow spacing must return None (unchanged)",
        ));
    }

    // Body-only: no interface, so no regularization.
    let body_only = vec![SupportPlanRoleRegion {
        role: SupportPlanRole::SupportBody,
        regions: vec![square(0)],
    }];
    if regularize_entry_roles(&body_only, 0.4, 0.4, 0.4, false).is_some() {
        return Err(String::from("a body-only entry must return None"));
    }

    // Top and base interfaces alias onto the same output role.
    let aliased = vec![
        SupportPlanRoleRegion {
            role: SupportPlanRole::TopInterface,
            regions: vec![square(0)],
        },
        SupportPlanRoleRegion {
            role: SupportPlanRole::BaseInterface,
            regions: vec![square(20_000)],
        },
    ];
    let out = regularize_entry_roles(&aliased, 0.4, 0.4, 0.4, false)
        .ok_or_else(|| String::from("an aliased interface entry must regularize"))?;
    let top_roles: Vec<&SupportPlanRole> = out
        .iter()
        .filter(|(role, _)| {
            matches!(
                role,
                SupportPlanRole::TopInterface | SupportPlanRole::BaseInterface
            )
        })
        .map(|(role, _)| role)
        .collect();
    if top_roles.len() != 2 {
        return Err(format!(
            "both aliased interface roles must be emitted, got {top_roles:?}"
        ));
    }
    Ok(())
}

/// Strategy for one generated role scene.
pub fn strategy(max_cells: usize) -> impl Strategy<Value = RoleCase> {
    // One row of cells on an even lattice: the grid is derived from the label
    // count, so every label has a cell and no draw can index out of bounds.
    let cell_count = 1..=max_cells.clamp(1, 8);
    let order = proptest::sample::subsequence(vec![TOP, BOTTOM, BODY, RAFT], 0..=4);
    (cell_count, order)
        .prop_flat_map(move |(cell_count, role_order)| {
            let labels = proptest::collection::vec(0u8..5, cell_count..=cell_count);
            (labels, Just(role_order))
        })
        .prop_map(|(raw, role_order)| {
            let labels: Vec<CellLabel> = raw
                .into_iter()
                .map(|code| match code {
                    0 => CellLabel::Absent,
                    1 => CellLabel::Top,
                    2 => CellLabel::Bottom,
                    3 => CellLabel::Body,
                    _ => CellLabel::Raft,
                })
                .collect();
            // The input carries exactly the roles some cell declares, so the
            // fixture and the model always agree about which interfaces exist. The
            // drawn subsequence supplies a rotation of that roster, which varies
            // the output order without ever dropping a role.
            let present: BTreeSet<u8> =
                labels.iter().map(|label| cell_label_code(*label)).collect();
            let rotation = role_order.len();
            let mut role_order: Vec<u8> = [TOP, BOTTOM, BODY, RAFT]
                .into_iter()
                .filter(|code| present.contains(code))
                .collect();
            if !role_order.is_empty() {
                let shift = rotation % role_order.len();
                role_order.rotate_left(shift);
            }
            let cells = labels.len();
            let width = 2 * (cells as i64 + 1);
            let xs: Vec<i64> = (1..cells).map(|i| (i as i64) * 2).collect();
            RoleCase {
                width,
                height: 8,
                xs,
                ys: vec![],
                labels,
                role_order,
                flow_spacing_mm: 0.4,
                top_pitch_mm: 0.8,
                bottom_pitch_mm: 0.8,
            }
        })
}

/// Registry entry point for `H-SUPPORT-ROLES`.
pub fn run_property() -> Result<(), String> {
    let run = PropertyRun::new("H-SUPPORT-ROLES", "proptest_support_role_partition")
        .map_err(|error| error.to_string())?;
    let max_cells = usize::try_from(run.bound("max_cells").map_err(|e| e.to_string())?)
        .map_err(|_| String::from("max_cells does not fit a usize"))?;
    let _max_abs = run.bound("max_abs_units").map_err(|e| e.to_string())?;

    none_and_alias_modes()?;

    // A guaranteed interface-bearing witness: top over half the grid.
    let witness = RoleCase {
        width: 8,
        height: 8,
        xs: vec![4],
        ys: vec![],
        labels: vec![CellLabel::Top, CellLabel::Body],
        role_order: vec![BODY, TOP],
        flow_spacing_mm: 0.4,
        top_pitch_mm: 0.8,
        bottom_pitch_mm: 0.8,
    };
    verify(&witness)?;

    let generated = strategy(max_cells);
    run.check(&generated, |case: RoleCase, _guard: &mut CaseGuard| {
        verify(&case).map_err(TestCaseError::fail)
    })
    .map_err(|error| error.to_string())
}

/// The set of labels used by a case, for the controls' assertions.
#[must_use]
pub fn label_set(case: &RoleCase) -> BTreeSet<u8> {
    case.labels
        .iter()
        .map(|label| cell_label_code(*label))
        .collect()
}
