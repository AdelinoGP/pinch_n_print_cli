//! AC-N2 domain and cell-comparator controls (`H-RUNTIME-PARTITION`).
//!
//! Each control falsifies the shared comparator used by the modifier and fill
//! partition charters, using hand-built expected cell sets rather than the
//! comparator's own classification. A control that asked the comparator to
//! derive both sides would be self-referential and would stay green on a broken
//! comparator.

#![allow(missing_docs)]

use crate::cells::{
    assert_disjoint_union, checked_domain, checked_double, checked_max_abs_units,
    checked_max_cells, checked_narrow_units, compare_region_to_cells, holed_rect, rect_field,
    CellGrid, CellSet, Rect,
};

fn make_grid(xs: &[i64], ys: &[i64], max: usize) -> CellGrid {
    CellGrid::from_boundaries(xs.iter().copied(), ys.iter().copied(), max).expect("bounded grid")
}

fn rect(x0: i64, y0: i64, x1: i64, y1: i64) -> Rect {
    Rect::new(x0, y0, x1, y1).expect("ordered rectangle")
}

/// One named AC-N2 control: domain bounds fail closed and the comparator rejects
/// every known-wrong output the charters rely on it to catch.
#[test]
fn runtime_property_domain_and_oracle_controls() {
    domain_bounds_fail_closed();
    comparator_accepts_exact_witness();
    comparator_rejects_lost_sliver();
    comparator_rejects_invented_hole();
    comparator_rejects_extra_cut();
    comparator_rejects_non_orthogonal_edge();
    union_control_rejects_overlap_and_gap();
}

fn domain_bounds_fail_closed() {
    for raw in [0, 8, 15, (1 << 20) + 1, u64::MAX] {
        assert!(
            checked_max_abs_units(raw).is_err(),
            "max_abs_units {raw} must be rejected outside 16..=2^20"
        );
    }
    assert!(checked_max_abs_units(16).is_ok());
    assert!(checked_max_abs_units(1 << 20).is_ok());

    for raw in [0, 8, 15, 4097, u64::MAX] {
        assert!(
            checked_max_cells(raw).is_err(),
            "max_cells {raw} must be rejected outside 16..=4096"
        );
    }
    assert!(checked_max_cells(16).is_ok());
    assert!(checked_max_cells(4096).is_ok());

    assert!(
        checked_double(i64::MAX).is_err(),
        "an i64::MAX coordinate cannot be doubled; the domain must reject it"
    );
    assert!(
        checked_narrow_units(u64::MAX).is_err(),
        "a u64::MAX span must not wrap into i64"
    );
    assert!(
        checked_domain(16, 16, 17).is_err(),
        "a span wider than max_abs_units must be rejected"
    );
    assert!(
        checked_domain(16, 16, 0).is_err(),
        "a zero span cannot carry a witness"
    );
    assert!(checked_domain(16, 16, 16).is_ok());

    assert!(
        CellGrid::from_boundaries([0], [0, 1], 16).is_err(),
        "a single x boundary carries no cell"
    );
    assert!(
        CellGrid::from_boundaries([0, 1], [0, 1], 0).is_err(),
        "a zero cell budget must reject before allocating"
    );
    // 4097 boundaries on one axis exceed every legal budget.
    let wide: Vec<i64> = (0..=64).collect();
    assert!(
        CellGrid::from_boundaries(wide.iter().copied(), [0, 1], 16).is_err(),
        "an oversized grid product must report exhaustion, not pass"
    );
}

fn comparator_accepts_exact_witness() {
    let grid = make_grid(&[0, 8], &[0, 8], 16);
    let full = CellSet::full(grid.cell_count());
    let region = rect(0, 0, 8, 8).to_expolygon();
    compare_region_to_cells(&[region], &full, &grid, "full square").expect("exact witness passes");

    // 4x4 grid with the centre cell carrying a hole.
    let grid = make_grid(&[0, 2, 4, 8], &[0, 2, 4, 8], 16);
    let hole = Rect::new(2, 2, 4, 4).expect("hole");
    let holed = holed_rect(rect(0, 0, 8, 8), &[hole]).expect("valid holed rect");
    let mut expected = CellSet::full(grid.cell_count());
    // Column 1, row 1 is the hole (xs and ys share the same boundaries).
    let hole_index = 1 + (grid.xs().len() - 1);
    expected.cells_mut()[hole_index] = false;
    compare_region_to_cells(&[holed], &expected, &grid, "holed square")
        .expect("the independent cell set matches the holed witness");
}

fn comparator_rejects_lost_sliver() {
    let grid = make_grid(&[0, 2, 4, 8], &[0, 8], 16);
    let expected = CellSet::full(grid.cell_count());
    // Two rects with a missing 2-unit column: a lost sliver between them.
    let left = rect(0, 0, 2, 8).to_expolygon();
    let right = rect(4, 0, 8, 8).to_expolygon();
    let error = compare_region_to_cells(&[left, right], &expected, &grid, "lost sliver")
        .expect_err("a missing sliver must fail the comparator");
    assert_eq!(error.kind, "cell_membership_mismatch", "{error}");
}

fn comparator_rejects_invented_hole() {
    let grid = make_grid(&[0, 2, 4, 8], &[0, 2, 4, 8], 16);
    let expected = CellSet::full(grid.cell_count());
    let hole = Rect::new(2, 2, 4, 4).expect("hole");
    let holed = holed_rect(rect(0, 0, 8, 8), &[hole]).expect("valid holed rect");
    let error = compare_region_to_cells(&[holed], &expected, &grid, "invented hole")
        .expect_err("an unexpected hole must fail the comparator");
    assert_eq!(error.kind, "cell_membership_mismatch", "{error}");
}

fn comparator_rejects_extra_cut() {
    let grid = make_grid(&[0, 4, 6, 8], &[0, 8], 16);
    let expected = CellSet::full(grid.cell_count());
    // A rect that stops short of the grid's right edge: an unrequested cut.
    let short = rect(0, 0, 6, 8).to_expolygon();
    let error = compare_region_to_cells(&[short], &expected, &grid, "extra cut")
        .expect_err("an extra cut must fail the comparator");
    assert_eq!(error.kind, "cell_membership_mismatch", "{error}");
}

fn comparator_rejects_non_orthogonal_edge() {
    let grid = make_grid(&[0, 8], &[0, 8], 16);
    let expected = CellSet::full(grid.cell_count());
    let triangle = slicer_ir::ExPolygon {
        contour: slicer_ir::Polygon {
            points: vec![
                slicer_ir::Point2 { x: 0, y: 0 },
                slicer_ir::Point2 { x: 8, y: 0 },
                slicer_ir::Point2 { x: 0, y: 8 },
            ],
        },
        holes: Vec::new(),
    };
    let error = compare_region_to_cells(&[triangle], &expected, &grid, "diagonal")
        .expect_err("a diagonal edge cannot be certified by an orthogonal cell grid");
    assert_eq!(error.kind, "non_orthogonal_edge", "{error}");
}

fn union_control_rejects_overlap_and_gap() {
    let grid = make_grid(&[0, 2, 4], &[0, 8], 16);
    let universe = grid.cell_count();
    let full = CellSet::full(universe);
    let mut left = CellSet::empty(universe);
    let mut right = CellSet::empty(universe);
    left.cells_mut()[0] = true;
    right.cells_mut()[1] = true;
    assert_disjoint_union(&[("left", &left), ("right", &right)], &full, universe)
        .expect("disjoint covering parts are valid");

    let mut overlapping = right.clone();
    overlapping.cells_mut()[0] = true;
    let error = assert_disjoint_union(&[("left", &left), ("right", &overlapping)], &full, universe)
        .expect_err("overlapping parts must be rejected");
    assert_eq!(error.kind, "roles_overlap", "{error}");

    let mut gapped = CellSet::empty(universe);
    gapped.cells_mut()[0] = true;
    let error = assert_disjoint_union(&[("only", &gapped)], &full, universe)
        .expect_err("a missing cell must be rejected as incomplete coverage");
    assert_eq!(error.kind, "roles_do_not_cover_domain", "{error}");
}

/// Field helper check: disjoint abutting rectangles are a legal concave union,
/// while overlapping rectangles are rejected before reaching the SUT.
#[test]
fn runtime_cell_field_rejects_overlap_but_allows_shared_edges() {
    let abutting = [rect(0, 0, 4, 8), rect(4, 0, 8, 8)];
    rect_field(&abutting).expect("shared edge is a legal concave field union");
    let overlapping = [rect(0, 0, 4, 8), rect(2, 0, 8, 8)];
    let error = rect_field(&overlapping).expect_err("overlapping field rects must be rejected");
    assert_eq!(error.kind, "overlapping_field_rects", "{error}");
}
