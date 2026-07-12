//! TDD test for packet 152, Step 3 (G3 part 1, AC-N2): `only_one_wall_top`
//! disabled keeps the full wall count on a topmost region.
//!
//! `only_one_wall_top` only collapses the wall stack to a single wall when it
//! is *enabled* AND the region is the topmost shell (`top_shell_index ==
//! Some(0)`). When the key is OFF, a topmost region must still emit the full
//! `max_bead_count`-derived wall count — the gate must not fire. This is the
//! AC-N2 negative case (key off -> full count). The AC-2 second-pass case
//! (`only_one_wall_top=true` collapses to one wall) is added in packet 152
//! Step 4, appended to this same file.
//!
//! Harness mirrors `alternate_extra_wall_tdd.rs`: drives
//! `ArachnePerimeters::run_perimeters` directly and pins the wall count via
//! `max_bead_count` on a square large enough that the cap always binds. Per
//! `alternate_extra_wall_tdd.rs`'s measured mapping, an even `max_bead_count`
//! emits exactly `max_bead_count / 2` walls.

use arachne_perimeters::ArachnePerimeters;
use slicer_ir::{mm_to_units, ConfigView, ExPolygon, WallLoop};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

const BEAD_WIDTH_MM: f32 = 1.0;
/// Large enough that the bead cap, not the polygon's geometry, is the
/// binding constraint on the emitted wall count.
const SQUARE_SIDE_MM: f32 = 20.0;
/// Even so the emitted wall count is exactly `max_bead_count / 2` == 2 walls
/// at baseline (see `alternate_extra_wall_tdd.rs` for the measured mapping).
const BASE_MAX_BEAD_COUNT: i64 = 4;
const BASE_WALL_COUNT: usize = 2;

fn make_config(only_one_wall_top: bool) -> ConfigView {
    ConfigViewBuilder::new()
        .float("optimal_width", mm_to_units(BEAD_WIDTH_MM) as f64)
        .float(
            "preferred_bead_width_outer",
            mm_to_units(BEAD_WIDTH_MM) as f64,
        )
        .int("max_bead_count", BASE_MAX_BEAD_COUNT)
        .bool("only_one_wall_top", only_one_wall_top)
        .build()
}

fn make_region(z: f32, top_shell_index: Option<u8>) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(z)
        .add_polygon(square_polygon(0.0, 0.0, SQUARE_SIDE_MM))
        .top_shell_index(top_shell_index)
        .build()
}

fn wall_loop_count(config: &ConfigView, top_shell_index: Option<u8>) -> usize {
    let module = ArachnePerimeters::on_print_start(config).unwrap();
    // A non-zero layer so is_initial_layer is false and the only_one_wall_top
    // topmost gate (region metadata) is the only thing that could collapse.
    let regions = vec![make_region(1.0, top_shell_index)];
    let paint = PaintRegionLayerView::new(5);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(5, &regions, &paint, &mut output, config)
        .unwrap();

    output.wall_loops().len()
}

/// AC-N2 (key off -> full count): with `only_one_wall_top=false`, a topmost
/// region (top_shell_index == Some(0)) must still emit the FULL baseline wall
/// count (BASE_WALL_COUNT == 2), not a single collapsed wall. The gate only
/// collapses when the key is ON.
#[test]
fn only_one_wall_top_disabled() {
    let config = make_config(false);

    let count = wall_loop_count(&config, Some(0));

    assert_eq!(
        count, BASE_WALL_COUNT,
        "only_one_wall_top=false must NOT collapse the wall stack; a topmost \
         region (top_shell_index == Some(0)) must emit the full baseline wall \
         count ({BASE_WALL_COUNT}); got {count}"
    );
}

/// AC-2 (packet 152 Step 4, G3 part 2): for a NON-topmost region whose top
/// surface is a SUB-AREA (`top_solid_fill` set), `only_one_wall_top=true` must
/// (a) emit a single wall (`inset_idx == 0`) whose centroid lies inside the top
/// sub-area, and (b) the non-top remainder's inner walls must have `inset_idx`
/// incremented by 1 relative to a naive single-pass run (`only_one_wall_top=
/// false`) on the same region.
///
/// PnP resolves the top-area source from `top_solid_fill` (not Orca's
/// `diff_ex(infill_contour, upper_slices_clipped)` — PnP has no `upper_slices`
/// access), recorded in `D-152-TOP-AREA-SOURCE`. The renumbering assertion (b)
/// is the primary lock; the single-top-wall assertion (a) is checked via
/// centroid containment in the top sub-area bounding box.
#[test]
fn only_one_wall_top_second_pass() {
    const MAX_BEAD: i64 = 6; // -> 3 walls in the naive single pass ({0,1,2})

    let naive_config = ConfigViewBuilder::new()
        .float("optimal_width", mm_to_units(BEAD_WIDTH_MM) as f64)
        .float(
            "preferred_bead_width_outer",
            mm_to_units(BEAD_WIDTH_MM) as f64,
        )
        .int("max_bead_count", MAX_BEAD)
        .bool("only_one_wall_top", false)
        .build();
    let second_config = ConfigViewBuilder::new()
        .float("optimal_width", mm_to_units(BEAD_WIDTH_MM) as f64)
        .float(
            "preferred_bead_width_outer",
            mm_to_units(BEAD_WIDTH_MM) as f64,
        )
        .int("max_bead_count", MAX_BEAD)
        .bool("only_one_wall_top", true)
        .build();

    // Top sub-area: a 4 mm square in the corner of the 20 mm region square.
    let top_fill: Vec<ExPolygon> = vec![square_polygon(2.0, 2.0, 4.0)];

    let naive_regions = vec![SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(1.0)
        .add_polygon(square_polygon(0.0, 0.0, SQUARE_SIDE_MM))
        .top_shell_index(Some(1)) // non-topmost
        .top_solid_fill(top_fill.clone())
        .build()];
    let second_regions = vec![SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(1.0)
        .add_polygon(square_polygon(0.0, 0.0, SQUARE_SIDE_MM))
        .top_shell_index(Some(1)) // non-topmost
        .top_solid_fill(top_fill.clone())
        .build()];

    let naive_walls = run_and_collect(&naive_config, &naive_regions);
    let second_walls = run_and_collect(&second_config, &second_regions);

    // (a) exactly one inset-0 wall lies entirely inside the top sub-area (the
    // single top wall). The not-top remainder's outer wall also has inset_idx
    // == 0 but spans the whole region, so it is excluded by the
    // all-points-inside check.
    let (minx, miny, maxx, maxy) = top_fill_bbox(&top_fill);
    let top_walls = second_walls
        .iter()
        .filter(|w| w.perimeter_index == 0 && wall_inside_top_fill(&w.path, minx, miny, maxx, maxy))
        .count();
    assert_eq!(
        top_walls, 1,
        "second pass must emit exactly one inset-0 wall inside the top sub-area; got {top_walls}"
    );

    // (b) renumbering lock: every inner wall (inset_idx > 0) of the second
    // pass must be a naive inner wall + 1. This is the G3 part-2 `inset_idx`
    // renumbering (`++el.inset_idx` on each inner perimeter, BEFORE merge).
    // Without the renumbering, the second pass would re-emit inner walls at
    // the naive pidx values (e.g. 1-1 == 0, which is never a naive inner
    // wall), so this assertion fails closed if the renumber is dropped.
    let mut naive_inner: Vec<i64> = naive_walls
        .iter()
        .filter(|w| w.perimeter_index > 0)
        .map(|w| w.perimeter_index as i64)
        .collect();
    naive_inner.sort_unstable();
    let mut second_inner: Vec<i64> = second_walls
        .iter()
        .filter(|w| w.perimeter_index > 0)
        .map(|w| w.perimeter_index as i64)
        .collect();
    second_inner.sort_unstable();
    assert!(
        !second_inner.is_empty(),
        "second pass must emit inner (non-top) walls to exercise the renumber"
    );
    for &x in &second_inner {
        assert!(
            naive_inner.contains(&(x - 1)),
            "second-pass inner wall inset_idx {x} is not a naive inner wall + 1; \
             second_inner={second_inner:?}, naive_inner={naive_inner:?}"
        );
    }
}

fn run_and_collect(config: &ConfigView, regions: &[SliceRegionView]) -> Vec<WallLoop> {
    let module = ArachnePerimeters::on_print_start(config).unwrap();
    let paint = PaintRegionLayerView::new(5);
    let mut output = PerimeterOutputBuilder::new();
    module
        .run_perimeters(5, regions, &paint, &mut output, config)
        .unwrap();
    output.wall_loops().to_vec()
}

/// Bounding box (mm) of the top sub-area, expanded by a 1 mm margin so the
/// single top wall (offset outward by ~half a bead width) is still contained.
fn top_fill_bbox(top_fill: &[ExPolygon]) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for ep in top_fill {
        for p in &ep.contour.points {
            let x = p.x as f64;
            let y = p.y as f64;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }
    (min_x - 1.0, min_y - 1.0, max_x + 1.0, max_y + 1.0)
}

/// True if every point of the wall path (mm-space x/y) lies inside the given
/// top sub-area bounding box — i.e. the wall is the single top-area wall, not
/// the not-top remainder's outer wall (which spans the whole region).
fn wall_inside_top_fill(
    path: &slicer_ir::ExtrusionPath3D,
    minx: f64,
    miny: f64,
    maxx: f64,
    maxy: f64,
) -> bool {
    if path.points.is_empty() {
        return false;
    }
    for p in &path.points {
        let x = p.x as f64;
        let y = p.y as f64;
        if x < minx || x > maxx || y < miny || y > maxy {
            return false;
        }
    }
    true
}
