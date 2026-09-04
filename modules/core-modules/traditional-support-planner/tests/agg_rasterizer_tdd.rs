//! TDD tests for the AGG-equivalent support grid rasterizer (packet 241, step 3).
//!
//! These tests pin the canonical `SupportGridPattern` constructor arithmetic and
//! the `rasterize_polygons` / `dilate_trimming_region` behaviour ported from
//! OrcaSlicer's `src/libslic3r/Support/SupportMaterial.cpp`.

use std::collections::HashMap;

use slicer_ir::{
    mm_to_units, ConfigKey, ConfigValue, ConfigView, ExPolygon, Point2, Polygon,
    SupportPlanDeclineReason,
};
use slicer_sdk::prelude::{
    host, ClipOperation, DiagnosticSeverity, LayerPlanView, LayerPlanViewEntry, MeshObjectView,
    OffsetJoinType, RegionSegmentationView, SupportAnalysisCandidate,
    SupportAnalysisGeometryEntry, SupportAnalysisView, SupportFamilyAssignment,
    SupportGeometryOutput, SupportGeometryView,
};
use slicer_sdk::traits::PrepassModule;
use traditional_support_planner::agg_raster::{
    dilate_trimming_region, fill_grid_holes, in_cell_offset_is_valid, max_in_cell_offset,
    rasterize_polygons, seed_fill_block, GridParams, SupportGrid,
};
use traditional_support_planner::{RasterizerMode, SupportPlanner};

/// Axis-aligned rectangle in mm.
fn rect_mm(x0: f32, y0: f32, x1: f32, y1: f32) -> Polygon {
    Polygon {
        points: vec![
            Point2::from_mm(x0, y0),
            Point2::from_mm(x1, y0),
            Point2::from_mm(x1, y1),
            Point2::from_mm(x0, y1),
        ],
    }
}

/// Solid square island, `size` mm on a side, anchored at the origin.
fn square_island(size: f32) -> ExPolygon {
    ExPolygon {
        contour: rect_mm(0.0, 0.0, size, size),
        holes: vec![],
    }
}

#[test]
fn grid_construction_matches_canonical_formulas() {
    let support = vec![square_island(10.0)];
    let spacing_mm = 2.0_f32;
    let width_mm = 0.4_f32;
    let params = GridParams::from_polygons(&support, spacing_mm, width_mm);

    let width_units = mm_to_units(width_mm);
    assert_eq!(width_units, 4000, "0.4 mm is 4000 PnP units (1 unit = 100 nm)");

    // oversampling = clamp(mm_to_units(spacing) / (width_units + 1), 1, 8)
    //              = clamp(20000 / 4001, 1, 8) = 4
    assert_eq!(params.oversampling, 4);
    assert!(
        (1..=8).contains(&params.oversampling),
        "canonical clamps oversampling into 1..=8"
    );

    // pixel_size = max(width_units + 1, mm_to_units(spacing / oversampling))
    //            = max(4001, mm_to_units(0.5)) = max(4001, 5000) = 5000
    let expected_pixel = std::cmp::max(
        width_units + 1,
        mm_to_units(spacing_mm / params.oversampling as f32),
    );
    assert_eq!(expected_pixel, 5000);
    assert_eq!(params.pixel_size, expected_pixel);

    // bbox: extents (0,0)-(100000,100000), offset(1), align_to_grid(20000)
    // pulls the min down to -20000, then a full-pixel margin offset(5000).
    assert_eq!(params.origin, Point2 { x: -25000, y: -25000 });

    // grid_size_raw = ceil(span / pixel_size); span = 105001 - (-25000) = 130001
    assert_eq!(params.grid_size_raw, (27, 27));

    // blocks = (raw + oversampling - 1 - 2) / oversampling = (27 + 1) / 4 = 7
    assert_eq!(params.blocks, (7, 7));

    // grid_size = blocks * oversampling + (2, 2)
    assert_eq!(params.grid_size, (30, 30));
    assert_eq!(
        params.grid_size,
        (
            params.blocks.0 * params.oversampling + 2,
            params.blocks.1 * params.oversampling + 2
        )
    );
    assert!(
        params.grid_size.0 >= params.grid_size_raw.0
            && params.grid_size.1 >= params.grid_size_raw.1,
        "canonical asserts grid_size >= grid_size_raw componentwise"
    );

    // The full-pixel margin guarantees an empty boundary ring.
    let grid = rasterize_polygons(&support, &params);
    assert_eq!(grid.len(), params.grid_size.0 * params.grid_size.1);
    let (nx, ny) = params.grid_size;
    for c in 0..nx {
        assert_eq!(grid[c], 0, "top row cell {c} must be unset");
        assert_eq!(grid[(ny - 1) * nx + c], 0, "bottom row cell {c} must be unset");
    }
    for r in 0..ny {
        assert_eq!(grid[r * nx], 0, "left column cell {r} must be unset");
        assert_eq!(grid[r * nx + nx - 1], 0, "right column cell {r} must be unset");
    }
    // Sanity: the island itself is actually rasterized.
    assert!(grid.iter().any(|&v| v != 0), "island must mark cells");
}

#[test]
fn rasterize_polygons_marks_any_covered_cell() {
    let support = vec![square_island(10.0)];
    let params = GridParams::from_polygons(&support, 2.0, 0.4);
    let (nx, _ny) = params.grid_size;
    let at = |g: &[u8], c: usize, r: usize| g[r * nx + c];

    // Grid space: origin -25000, pixel 5000 => the island spans cells 5..=24.
    let full = rasterize_polygons(&support, &params);
    assert_eq!(at(&full, 5, 5), 1, "first covered cell is set");
    assert_eq!(at(&full, 24, 24), 1, "last covered cell is set");
    assert_eq!(at(&full, 4, 4), 0, "cell fully outside is unset");
    assert_eq!(at(&full, 27, 12), 0, "cell fully outside is unset");

    // A polygon covering only part of a cell still sets it: shift the island by
    // a quarter pixel (0.0125 mm) so cell 4 is only fractionally covered.
    let sliver = vec![ExPolygon {
        contour: rect_mm(-0.0125, 0.0, 10.0, 10.0),
        holes: vec![],
    }];
    let sliver_grid = rasterize_polygons(&sliver, &params);
    assert_eq!(
        at(&sliver_grid, 4, 10),
        1,
        "any partial coverage sets the cell (canonical has no coverage threshold)"
    );

    // Nonzero-winding witness: a hole must come out unset, not filled.
    let holed = vec![ExPolygon {
        contour: rect_mm(0.0, 0.0, 10.0, 10.0),
        holes: vec![rect_mm(3.0, 7.0, 7.0, 3.0)],
    }];
    let holed_grid = rasterize_polygons(&holed, &params);
    // Hole spans mm 3..7 => cells 11..=18; cell (14, 14) is strictly interior.
    assert_eq!(at(&holed_grid, 14, 14), 0, "hole interior must be unset");
    assert_eq!(at(&holed_grid, 6, 14), 1, "solid ring around the hole is set");
}

#[test]
fn dilate_trimming_region_erodes_to_all_set_interior() {
    // 8x8 grid with a solid 4x4 block at columns/rows 2..=5.
    let params = GridParams::for_test(1000, Point2 { x: 0, y: 0 }, (8, 8), (0, 0), 1);
    let nx = 8usize;
    let mut input = vec![0u8; 64];
    for r in 2..=5 {
        for c in 2..=5 {
            input[r * nx + c] = 1;
        }
    }
    let out = dilate_trimming_region(&input, &params);
    assert_eq!(out.len(), 64);

    // The 4x4 block erodes by exactly one cell on every side -> 2x2 at 3..=4.
    let mut expected = vec![0u8; 64];
    for r in 3..=4 {
        for c in 3..=4 {
            expected[r * nx + c] = 1;
        }
    }
    assert_eq!(out, expected, "solid block erodes by one cell per side");

    // A cell with any unset neighbour is cleared.
    assert_eq!(out[3 * nx + 2], 0, "edge cell of the block has unset neighbours");

    // The outer ring is never written and stays unset, even when input is solid.
    let solid = vec![1u8; 64];
    let solid_out = dilate_trimming_region(&solid, &params);
    for c in 0..nx {
        assert_eq!(solid_out[c], 0, "outer ring row 0 stays unset");
        assert_eq!(solid_out[7 * nx + c], 0, "outer ring row 7 stays unset");
    }
    for r in 0..8 {
        assert_eq!(solid_out[r * nx], 0, "outer ring column 0 stays unset");
        assert_eq!(solid_out[r * nx + 7], 0, "outer ring column 7 stays unset");
    }
    // Everything interior of a solid input survives.
    for r in 1..7 {
        for c in 1..7 {
            assert_eq!(solid_out[r * nx + c], 1, "interior of a solid input survives");
        }
    }
}

// ---------------------------------------------------------------------------
// Step 4: seed fill, contour extraction, island filter.
// ---------------------------------------------------------------------------

/// Grid dimensions used by the seed-fill locality tests: `oversampling = 4`
/// with `blocks = (2, 1)` gives two macro blocks side by side, block 0 owning
/// columns 1..=4 and block 1 owning columns 5..=8, plus the one-cell boundary
/// ring canonical relies on.
const SEED_GRID: (usize, usize) = (10, 6);

fn seed_params() -> GridParams {
    GridParams::for_test(1000, Point2 { x: 0, y: 0 }, SEED_GRID, (2, 1), 4)
}

fn set_cell(grid: &mut [u8], c: usize, r: usize) {
    grid[r * SEED_GRID.0 + c] = 1;
}

fn cell(grid: &[u8], c: usize, r: usize) -> u8 {
    grid[r * SEED_GRID.0 + c]
}

#[test]
fn seed_fill_block_closes_gaps_within_a_block_but_not_across_blocks() {
    let params = seed_params();
    let mask = vec![0u8; params.cell_count()];
    let mut grid = vec![0u8; params.cell_count()];
    // A single seed in the lower-left cell of block 0.
    set_cell(&mut grid, 1, 1);

    seed_fill_block(
        &mut grid,
        params.grid_size,
        &mask,
        params.blocks,
        params.oversampling,
    );

    // Block 0 (columns 1..=4, rows 1..=4) is flooded from the single seed:
    // the two canonical passes propagate along every row and both column
    // directions inside the block.
    for r in 1..=4 {
        for c in 1..=4 {
            assert_eq!(cell(&grid, c, r), 1, "block 0 cell ({c},{r}) must be filled");
        }
    }
    // Block 1 (columns 5..=8) never receives anything: the horizontal steps
    // stop one cell short of each block edge, so propagation cannot cross a
    // macro-block boundary.
    for r in 0..SEED_GRID.1 {
        for c in 5..=8 {
            assert_eq!(cell(&grid, c, r), 0, "block 1 cell ({c},{r}) must stay empty");
        }
    }
    // The one-cell boundary ring is never written.
    for c in 0..SEED_GRID.0 {
        assert_eq!(cell(&grid, c, 0), 0, "boundary row 0 stays unset");
        assert_eq!(cell(&grid, c, 5), 0, "boundary row 5 stays unset");
    }
    for r in 0..SEED_GRID.1 {
        assert_eq!(cell(&grid, 0, r), 0, "boundary column 0 stays unset");
        assert_eq!(cell(&grid, 9, r), 0, "boundary column 9 stays unset");
    }

    // A seed in block 1 fills block 1 and leaves block 0 empty: locality is
    // symmetric, not an artifact of which block was seeded.
    let mut grid2 = vec![0u8; params.cell_count()];
    set_cell(&mut grid2, 5, 1);
    seed_fill_block(
        &mut grid2,
        params.grid_size,
        &mask,
        params.blocks,
        params.oversampling,
    );
    for r in 1..=4 {
        for c in 5..=8 {
            assert_eq!(cell(&grid2, c, r), 1, "block 1 cell ({c},{r}) must be filled");
        }
        for c in 1..=4 {
            assert_eq!(cell(&grid2, c, r), 0, "block 0 cell ({c},{r}) must stay empty");
        }
    }
}

#[test]
fn seed_fill_block_respects_the_dilated_mask_at_both_endpoints() {
    let params = seed_params();

    // (a) Masking the SOURCE cell: the seed itself is masked, so no step can
    // ever read it (`mask[addr2] == 0` fails) and nothing propagates.
    let mut grid = vec![0u8; params.cell_count()];
    set_cell(&mut grid, 1, 1);
    let mut mask = vec![0u8; params.cell_count()];
    set_cell(&mut mask, 1, 1);
    seed_fill_block(
        &mut grid,
        params.grid_size,
        &mask,
        params.blocks,
        params.oversampling,
    );
    assert_eq!(cell(&grid, 1, 1), 1, "the seed itself is untouched");
    assert_eq!(
        grid.iter().filter(|&&v| v != 0).count(),
        1,
        "a masked source propagates nowhere"
    );

    // (b) Masking the DESTINATION cell: the rest of the block still fills, but
    // the masked cell stays unset from every direction (`mask[addr] == 0`).
    let mut grid = vec![0u8; params.cell_count()];
    set_cell(&mut grid, 1, 1);
    let mut mask = vec![0u8; params.cell_count()];
    set_cell(&mut mask, 2, 1);
    seed_fill_block(
        &mut grid,
        params.grid_size,
        &mask,
        params.blocks,
        params.oversampling,
    );
    assert_eq!(
        cell(&grid, 2, 1),
        0,
        "the masked destination is never written, from any direction"
    );
    assert_eq!(
        cell(&grid, 1, 2),
        1,
        "propagation still leaves the seed vertically"
    );
    assert_eq!(
        cell(&grid, 3, 1),
        1,
        "cells beyond the mask are reached around it"
    );
}

#[test]
fn fill_holes_bridges_single_cell_holes_in_one_pass() {
    // 8x8 grid, solid 6x6 block at rows/columns 1..=6, with an L-shaped hole
    // at (3,3), (4,3), (4,4).
    let w = 8usize;
    let mut grid = vec![0u8; w * w];
    for r in 1..=6 {
        for c in 1..=6 {
            grid[r * w + c] = 1;
        }
    }
    for &(c, r) in &[(3usize, 3usize), (4, 3), (4, 4)] {
        grid[r * w + c] = 0;
    }

    let out = fill_grid_holes(&grid, (w, w));

    // (3,3) is bridged vertically: (3,2) and (3,4) are both set.
    assert_eq!(out[3 * w + 3], 1, "single-cell hole (3,3) is bridged");
    // (4,4) is bridged horizontally: (3,4) and (5,4) are both set.
    assert_eq!(out[4 * w + 4], 1, "single-cell hole (4,4) is bridged");
    // (4,3) is NOT bridged: in the UNMODIFIED grid its left neighbour (3,3)
    // and its lower neighbour (4,4) are both holes. A second pass -- or an
    // in-place write that let (3,3) become visible within the same pass --
    // would fill it. Canonical does exactly one pass over an unmodified read.
    assert_eq!(
        out[3 * w + 4],
        0,
        "fill_holes is a single non-iterative pass over the unmodified grid"
    );

    // The boundary ring is never written.
    for c in 0..w {
        assert_eq!(out[c], 0, "boundary row 0 stays unset");
        assert_eq!(out[7 * w + c], 0, "boundary row 7 stays unset");
    }
}

/// Contour-extraction grid: 16x8 cells of 1000 units at the origin.
fn contour_params() -> GridParams {
    GridParams::for_test(1000, Point2 { x: 0, y: 0 }, (16, 8), (0, 0), 1)
}

fn mark(grid: &mut [u8], w: usize, c: usize, r: usize) {
    grid[r * w + c] = 1;
}

/// Crossing-parity containment across a contour and all its holes.
fn contains(island: &ExPolygon, p: Point2) -> bool {
    let mut inside = false;
    for ring in std::iter::once(&island.contour).chain(island.holes.iter()) {
        let pts = &ring.points;
        let n = pts.len();
        let mut j = n - 1;
        for i in 0..n {
            let (a, b) = (pts[j], pts[i]);
            if (a.y > p.y) != (b.y > p.y) {
                let cross = (b.x - a.x) as i128 * (p.y - a.y) as i128
                    - (b.y - a.y) as i128 * (p.x - a.x) as i128;
                if (cross > 0) == (b.y > a.y) {
                    inside = !inside;
                }
            }
            j = i;
        }
    }
    inside
}

#[test]
fn contour_extraction_filters_islands_by_samples() {
    let params = contour_params();
    let w = params.grid_size.0;
    let mut grid = vec![0u8; params.cell_count()];
    // Island A: a solid 3x3 block at columns 2..=4, rows 2..=4. No sample.
    for r in 2..=4 {
        for c in 2..=4 {
            mark(&mut grid, w, c, r);
        }
    }
    // Island B: a one-cell-wide, three-cell-tall column at c = 10, rows 2..=4.
    // This is the rasterized image of a support column that narrows to a
    // sub-cell sliver (upstream a95607d7bf): it must NOT be dropped while it
    // still carries a sample.
    for r in 2..=4 {
        mark(&mut grid, w, 10, r);
    }

    let sample = Point2 { x: 10_500, y: 3_500 }; // centre of cell (10, 3)
    let sg = SupportGrid::for_test(params, grid, Vec::new());
    let islands = sg.extract_support(0, false, &[sample]);

    assert_eq!(islands.len(), 1, "only the sampled island survives");
    let survivor = &islands[0];
    assert!(
        contains(survivor, sample),
        "the survivor is the island that carries the sample"
    );
    assert!(
        !contains(survivor, Point2 { x: 3_500, y: 3_500 }),
        "island A (unsampled) is not the survivor"
    );

    // Real geometry: the sliver comes back as its exact 4-corner rectangle,
    // x in [10000, 11000], y in [2000, 5000].
    assert!(survivor.holes.is_empty(), "the sliver has no holes");
    let mut pts = survivor.contour.points.clone();
    pts.sort_by_key(|p| (p.x, p.y));
    assert_eq!(
        pts,
        vec![
            Point2 { x: 10_000, y: 2_000 },
            Point2 { x: 10_000, y: 5_000 },
            Point2 { x: 11_000, y: 2_000 },
            Point2 { x: 11_000, y: 5_000 },
        ],
        "the sub-cell sliver keeps its exact rasterized rectangle"
    );

    // Sampling the OTHER island flips which one survives, and the 3x3 block
    // comes back at its own exact extent.
    let params = contour_params();
    let w = params.grid_size.0;
    let mut grid = vec![0u8; params.cell_count()];
    for r in 2..=4 {
        for c in 2..=4 {
            mark(&mut grid, w, c, r);
        }
        mark(&mut grid, w, 10, r);
    }
    let sg = SupportGrid::for_test(params, grid, Vec::new());
    let islands = sg.extract_support(0, false, &[Point2 { x: 3_500, y: 3_500 }]);
    assert_eq!(islands.len(), 1, "only the sampled island survives");
    let mut pts = islands[0].contour.points.clone();
    pts.sort_by_key(|p| (p.x, p.y));
    assert_eq!(
        pts,
        vec![
            Point2 { x: 2_000, y: 2_000 },
            Point2 { x: 2_000, y: 5_000 },
            Point2 { x: 5_000, y: 2_000 },
            Point2 { x: 5_000, y: 5_000 },
        ],
        "the 3x3 block survives at its exact extent"
    );

    // No samples at all: every island is dropped.
    let params = contour_params();
    let w = params.grid_size.0;
    let mut grid = vec![0u8; params.cell_count()];
    for r in 2..=4 {
        mark(&mut grid, w, 10, r);
    }
    let sg = SupportGrid::for_test(params, grid, Vec::new());
    assert!(
        sg.extract_support(0, false, &[]).is_empty(),
        "an island with no sample inside it is dropped"
    );
}

#[test]
fn expansion_is_restricted_inside_the_macro_cell() {
    let params = contour_params();
    let pixel_size = params.pixel_size;
    let w = params.grid_size.0;
    // Two single-cell islands separated by exactly one empty cell at c = 4.
    let mut grid = vec![0u8; params.cell_count()];
    mark(&mut grid, w, 3, 3);
    mark(&mut grid, w, 5, 3);
    let samples = vec![
        Point2 { x: 3_500, y: 3_500 },
        Point2 { x: 5_500, y: 3_500 },
    ];

    // The in-cell bound is `abs(2 * offset) < pixel_size - 1` (canonical's
    // `-10` orca nm = 0.1 PnP units, rounded up to 1 unit, the strict side).
    let offset = max_in_cell_offset(pixel_size);
    assert_eq!(offset, 499, "largest legal offset for a 1000-unit pixel");
    assert!(in_cell_offset_is_valid(offset, pixel_size));
    assert!(2 * offset < pixel_size - 1, "the in-cell bound holds");
    assert!(
        !in_cell_offset_is_valid(offset + 1, pixel_size),
        "one unit more violates the in-cell bound"
    );
    assert!(
        !in_cell_offset_is_valid(600, pixel_size),
        "an unbounded global offset amount is rejected by the in-cell bound"
    );

    let sg = SupportGrid::for_test(params, grid, Vec::new());
    let expanded = sg.extract_support(offset, false, &samples);

    // The wall between the two islands survives: the expansion cannot leak
    // across the empty cell (upstream fb7b995050).
    assert_eq!(expanded.len(), 2, "the two islands stay separate");
    let mut spans: Vec<(i64, i64)> = expanded
        .iter()
        .map(|e| {
            let xs: Vec<i64> = e.contour.points.iter().map(|p| p.x).collect();
            (*xs.iter().min().unwrap(), *xs.iter().max().unwrap())
        })
        .collect();
    spans.sort();
    assert_eq!(spans[0], (3_000 - offset, 4_000 + offset));
    assert_eq!(spans[1], (5_000 - offset, 6_000 + offset));
    let gap = spans[1].0 - spans[0].1;
    assert_eq!(gap, pixel_size - 2 * offset, "the wall gap is 1000 - 998");
    assert!(gap > 0, "expansion never bridges the empty cell");

    // Every emitted point stays inside its originating macro cell: each corner
    // is displaced by exactly `offset` on each axis from its grid corner, and
    // `2 * offset < pixel_size` means the displaced corner never reaches the
    // far side of the neighbouring cell.
    for island in &expanded {
        assert_eq!(island.contour.points.len(), 4, "single-cell island: 4 corners");
        for p in &island.contour.points {
            let dx = (p.x - round_to(p.x, pixel_size)).abs();
            let dy = (p.y - round_to(p.y, pixel_size)).abs();
            assert_eq!(dx, offset, "x displacement is exactly the offset");
            assert_eq!(dy, offset, "y displacement is exactly the offset");
            assert!(2 * dx < pixel_size, "displacement stays inside the macro cell");
            assert!(2 * dy < pixel_size, "displacement stays inside the macro cell");
        }
    }

    // A GLOBAL polygon offset is unbounded and produces a different result:
    // offsetting the un-offset extraction by 600 units (0.06 mm) merges the
    // two islands across the wall, which is exactly the leak the in-cell
    // restriction prevents.
    let params = contour_params();
    let w = params.grid_size.0;
    let mut grid = vec![0u8; params.cell_count()];
    mark(&mut grid, w, 3, 3);
    mark(&mut grid, w, 5, 3);
    let sg = SupportGrid::for_test(params, grid, Vec::new());
    let plain = sg.extract_support(0, false, &samples);
    assert_eq!(plain.len(), 2);
    let globally_offset = host::offset_polygons(&plain, 0.06, OffsetJoinType::Miter, 0.0);
    assert_eq!(
        globally_offset.len(),
        1,
        "a global offset of the same order merges the islands"
    );
    assert_ne!(
        globally_offset.len(),
        expanded.len(),
        "grid-restricted expansion is not a global polygon offset"
    );
}

/// Nearest multiple of `step` to `v` (used to recover the grid corner a
/// displaced contour point came from).
fn round_to(v: i64, step: i64) -> i64 {
    let q = (v as f64) / (step as f64);
    (q.round() as i64) * step
}

// ---------------------------------------------------------------------------
// Step 5: `support_area_rasterizer` knob declaration, parse, and rejection.
// ---------------------------------------------------------------------------

/// A `ConfigView` carrying exactly one `support_area_rasterizer` value.
fn rasterizer_config(value: &str) -> ConfigView {
    let mut values: HashMap<ConfigKey, ConfigValue> = HashMap::new();
    values.insert(
        "support_area_rasterizer".into(),
        ConfigValue::String(value.into()),
    );
    ConfigView::from_map(values)
}

#[test]
fn invalid_rasterizer_value_is_rejected_not_defaulted() {
    // An out-of-vocabulary value must be a hard error, never silently
    // defaulted to either mode. This drives `from_config` directly: a full slice
    // would be intercepted by the host's `ConfigBoundsIndex`, which rejects
    // out-of-vocabulary enum values before the guest ever sees them, so it
    // would prove the host check rather than this one.
    let err = SupportPlanner::from_config(&rasterizer_config("marching_squares"))
        .err()
        .expect("an unknown support_area_rasterizer value must be rejected");
    assert!(err.fatal, "an unknown rasterizer value is fatal");
    assert!(
        err.message.contains("support_area_rasterizer"),
        "the message names the offending key: {}",
        err.message
    );
    assert!(
        err.message.contains("agg"),
        "the message names the `agg` value: {}",
        err.message
    );
    assert!(
        err.message.contains("legacy_semantic"),
        "the message names the `legacy_semantic` value: {}",
        err.message
    );

    // Both legal values parse to their own mode.
    let agg = SupportPlanner::from_config(&rasterizer_config("agg"))
        .expect("`agg` is a legal value");
    assert_eq!(agg.support_area_rasterizer, RasterizerMode::Agg);
    let legacy = SupportPlanner::from_config(&rasterizer_config("legacy_semantic"))
        .expect("`legacy_semantic` is a legal value");
    assert_eq!(
        legacy.support_area_rasterizer,
        RasterizerMode::LegacySemantic
    );

    // Absent key defaults to `legacy_semantic`: `agg` ships OPT-IN because the
    // faithful port block-snaps the carry (DEV-166; canonical `seed_fill_block`
    // in `SupportMaterial.cpp`).
    let absent = SupportPlanner::from_config(&ConfigView::new())
        .expect("the key is optional");
    assert_eq!(
        absent.support_area_rasterizer,
        RasterizerMode::LegacySemantic,
        "the default is `legacy_semantic`; `agg` is opt-in (DEV-166)"
    );
}


// ---------------------------------------------------------------------------
// Step 6: propagation routing -- agg by default, legacy selectable.
// ---------------------------------------------------------------------------

/// Six 0.2 mm layers, enough for a contact at layer 5 to descend to the plate.
fn six_layers() -> LayerPlanView {
    LayerPlanView {
        layers: (0..6)
            .map(|i| LayerPlanViewEntry {
                global_layer_index: i,
                z: 0.2 * (i + 1) as f32,
                effective_layer_height: 0.2,
            })
            .collect(),
    }
}

fn mesh_object(id: &str) -> MeshObjectView {
    MeshObjectView {
        object_id: id.to_string(),
        ..Default::default()
    }
}

/// A single traditional candidate plus per-layer model occupancy.
///
/// The occupancy is applied to every layer, which is what keeps the trimming
/// mask non-empty and therefore exercises the grid trimming polygons rather
/// than the degenerate no-trim short circuit.
fn analysis_fixture(geometry: Vec<ExPolygon>, occupancy: Vec<ExPolygon>) -> SupportAnalysisView {
    SupportAnalysisView {
        candidates: vec![SupportAnalysisCandidate {
            id: 7,
            geometry,
            object_id: "obj-1".into(),
            region_id: "region-0".into(),
            global_layer_index: 5,
            ..Default::default()
        }],
        model_occupancy: (0..6)
            .map(|layer| SupportAnalysisGeometryEntry {
                global_support_layer_index: layer,
                object_id: "obj-1".into(),
                region_id: "region-0".into(),
                polygons: occupancy.clone(),
            })
            .collect(),
        family_assignments: vec![SupportFamilyAssignment {
            object_id: "obj-1".into(),
            region_id: "region-0".into(),
            family_id: "traditional".into(),
        }],
        ..Default::default()
    }
}

/// Runs the planner over the fixture with the given `support_area_rasterizer`
/// value, or with the key entirely absent when `mode` is `None`.
fn plan_with(mode: Option<&str>, analysis: &SupportAnalysisView) -> SupportGeometryOutput {
    let config = match mode {
        Some(value) => rasterizer_config(value),
        None => ConfigView::new(),
    };
    let planner = SupportPlanner::from_config(&config).expect("planner config is valid");
    let mut output = SupportGeometryOutput::new();
    planner
        .run_support_geometry_with_analysis(
            &[mesh_object("obj-1")],
            &six_layers(),
            &RegionSegmentationView::default(),
            analysis,
            &SupportGeometryView::default(),
            &mut output,
            &config,
        )
        .expect("planning succeeds");
    output
}

/// Twice the absolute area of a polygon ring.
fn ring_area2(points: &[Point2]) -> i128 {
    let n = points.len();
    let mut acc: i128 = 0;
    for i in 0..n {
        let a = points[i];
        let b = points[(i + 1) % n];
        acc += a.x as i128 * b.y as i128 - b.x as i128 * a.y as i128;
    }
    acc.abs()
}

/// Total planned area per emitted layer, summed over every role region.
fn area_by_layer(output: &SupportGeometryOutput) -> Vec<(i32, i128)> {
    let mut rows: Vec<(i32, i128)> = output
        .entries()
        .iter()
        .map(|entry| {
            let area: i128 = entry
                .roles
                .iter()
                .flat_map(|role| role.regions.iter())
                .map(|ex| {
                    ring_area2(&ex.contour.points)
                        - ex.holes.iter().map(|h| ring_area2(&h.points)).sum::<i128>()
                })
                .sum();
            (entry.global_layer_index, area)
        })
        .collect();
    rows.sort();
    rows
}

#[test]
fn default_config_routes_propagation_through_legacy_semantic() {
    // A 6 mm square contact with a 3x2 mm model bar intruding from the left, so
    // the carry is genuinely trimmed on every descended layer.
    let contact = vec![ExPolygon {
        contour: rect_mm(2.0, 2.0, 8.0, 8.0),
        holes: vec![],
    }];
    let occupancy = vec![ExPolygon {
        contour: rect_mm(0.0, 3.0, 3.0, 5.0),
        holes: vec![],
    }];
    let analysis = analysis_fixture(contact, occupancy.clone());

    let default_run = plan_with(None, &analysis);
    let legacy_run = plan_with(Some("legacy_semantic"), &analysis);
    let explicit_agg_run = plan_with(Some("agg"), &analysis);

    let default_areas = area_by_layer(&default_run);
    let legacy_areas = area_by_layer(&legacy_run);
    let agg_areas = area_by_layer(&explicit_agg_run);

    // Termination bookkeeping is unchanged: same layers, no declines, no
    // `code: 1203` warning on either path.
    let default_layers: Vec<i32> = default_areas.iter().map(|(l, _)| *l).collect();
    let legacy_layers: Vec<i32> = legacy_areas.iter().map(|(l, _)| *l).collect();
    assert!(
        default_layers.len() >= 2,
        "the fixture must descend at least two layers, got {default_layers:?}"
    );
    assert_eq!(
        default_layers, legacy_layers,
        "the rasterizer must not change which layers terminate or emit"
    );
    for (run, label) in [
        (&default_run, "default"),
        (&legacy_run, "legacy"),
        (&explicit_agg_run, "agg"),
    ] {
        assert!(
            run.entries()
                .iter()
                .all(|entry| entry.decline_reason.is_none()),
            "{label} run must not decline this candidate"
        );
        assert!(
            run.diagnostics().iter().all(|d| d.code != 1203),
            "{label} run must not emit the NoRoute diagnostic"
        );
    }

    // The default IS `legacy_semantic`, area for area with the explicit
    // selection. `agg` is opt-in (DEV-166).
    assert_eq!(
        default_areas, legacy_areas,
        "an absent support_area_rasterizer key must behave exactly like `legacy_semantic`"
    );

    // Real geometry differs: canonical grid-snaps the printed area and expands
    // it by `expansion_to_slice`, so every emitted layer covers strictly more
    // than the unexpanded legacy carry.
    assert_ne!(
        default_areas, agg_areas,
        "the opt-in `agg` path must not reproduce the legacy propagate-without-growth areas"
    );
    // `zip` pairs positionally and stops at the shorter side, so a differing
    // layer set would silently compare layer N of one run against layer M of
    // the other and drop the tail. Pin the sets equal first.
    let agg_layers: Vec<i32> = agg_areas.iter().map(|(l, _)| *l).collect();
    assert_eq!(
        agg_layers, legacy_layers,
        "agg and legacy must emit the same layer set before their areas are compared pairwise"
    );
    for ((layer, agg_area), (_, legacy_area)) in agg_areas.iter().zip(legacy_areas.iter()) {
        assert!(
            *agg_area > *legacy_area,
            "layer {layer}: the grid-snapped printed area {agg_area} must exceed the \
             legacy unexpanded area {legacy_area}"
        );
    }

    // And the `agg` geometry is not a degenerate blob: it stays within one
    // grid cell plus the expansion of the contact square, and every ring is a
    // real polygon.
    for entry in explicit_agg_run.entries() {
        for region in entry.roles.iter().flat_map(|role| role.regions.iter()) {
            assert!(
                region.contour.points.len() >= 3,
                "every emitted region is a real polygon"
            );
            // The extraction is grid-snapped and the canonical block seed fill
            // floods each macro block that contains any set cell, so the
            // printed area is bounded by the grid extent -- the contact bbox
            // aligned to the base-pattern spacing plus a one-pixel margin --
            // NOT by the contact square itself. At spacing 2.5 mm / width
            // 0.4 mm that is pixel_size 4167 units, oversampling 6, so one
            // block is 25002 units (2.5002 mm) wide - the same figures
            // `agg_propagated_carry_grows_by_at_most_one_macro_block_extent`
            // pins against `GridParams::from_polygons` below.
            for p in &region.contour.points {
                assert!(
                    p.x >= mm_to_units(-0.5)
                        && p.x <= mm_to_units(10.5)
                        && p.y >= mm_to_units(-0.5)
                        && p.y <= mm_to_units(10.5),
                    "the printed area must stay inside the grid extent, got {p:?}"
                );
            }
            // The trimming difference is not lost in the rasterizer: no part of
            // the printed area may land inside the inflated model occupancy.
            let clearance = host::offset_polygons(
                &occupancy,
                0.35,
                OffsetJoinType::Miter,
                0.0,
            );
            let overlap = host::clip_polygons(
                std::slice::from_ref(region),
                &clearance,
                ClipOperation::Intersection,
            );
            assert!(
                overlap.is_empty(),
                "layer {}: the printed area must keep the gap_xy clearance",
                entry.global_layer_index
            );
            // A plain rectangle would mean the obstacle was rasterized away;
            // the bar bites a notch out of every emitted layer.
            assert!(
                region.contour.points.len() > 4,
                "layer {}: the obstacle must still notch the printed contour, got {} points",
                entry.global_layer_index,
                region.contour.points.len()
            );
        }
    }
}

#[test]
fn agg_path_still_declines_no_route_when_occupancy_closes_every_layer() {
    // The model swallows the whole contact area, so the carry empties on the
    // first descended layer. The agg path must decline exactly as the legacy
    // path does -- never truncate the column silently.
    let contact = vec![ExPolygon {
        contour: rect_mm(2.0, 2.0, 8.0, 8.0),
        holes: vec![],
    }];
    let occupancy = vec![ExPolygon {
        contour: rect_mm(-1.0, -1.0, 11.0, 11.0),
        holes: vec![],
    }];
    let analysis = analysis_fixture(contact, occupancy);

    for mode in [None, Some("agg"), Some("legacy_semantic")] {
        let run = plan_with(mode, &analysis);
        let label = mode.unwrap_or("<absent>");
        assert!(
            run.diagnostics()
                .iter()
                .any(|d| d.code == 1203 && d.severity == DiagnosticSeverity::Warn),
            "{label}: the blocked column must warn with code 1203"
        );
        assert_eq!(
            run.entries()
                .iter()
                .filter(|entry| entry.decline_reason == Some(SupportPlanDeclineReason::NoRoute))
                .count(),
            1,
            "{label}: exactly one NoRoute decline must be recorded"
        );
    }
}

// ---------------------------------------------------------------------------
// Step 13: the `agg` divergence, pinned by the suite.
//
// DEV-166. A faithful port of canonical `seed_fill_block`
// (`SupportMaterial.cpp`) deliberately "stretches supports into a grid" so the
// zig-zag infill can snake along grid lines: it floods every macro block that
// contains any set cell. That is genuine canonical behaviour, not a porting
// bug -- but PnP's demand model cannot absorb it, so `agg` ships OPT-IN and
// `legacy_semantic` is the default. The two tests below assert what `agg`
// ACTUALLY does, so the divergence is pinned by the suite rather than living
// only in prose.
// ---------------------------------------------------------------------------

/// Like `analysis_fixture`, but the occupancy is present on exactly one layer.
///
/// A single blocking layer is the shape that separates the two modes: the
/// legacy carry is the trimmed contact area, so the blocker empties it and the
/// column declines; the `agg` carry has already been block-snapped outward past
/// the blocker, so a route survives.
fn analysis_fixture_single_blocker(
    geometry: Vec<ExPolygon>,
    occupancy: Vec<ExPolygon>,
    blocking_layer: u32,
) -> SupportAnalysisView {
    SupportAnalysisView {
        candidates: vec![SupportAnalysisCandidate {
            id: 7,
            geometry,
            object_id: "obj-1".into(),
            region_id: "region-0".into(),
            global_layer_index: 5,
            ..Default::default()
        }],
        model_occupancy: vec![SupportAnalysisGeometryEntry {
            global_support_layer_index: blocking_layer,
            object_id: "obj-1".into(),
            region_id: "region-0".into(),
            polygons: occupancy,
        }],
        family_assignments: vec![SupportFamilyAssignment {
            object_id: "obj-1".into(),
            region_id: "region-0".into(),
            family_id: "traditional".into(),
        }],
        ..Default::default()
    }
}

/// DEV-166 divergence: under `agg`, a blocker that closes the whole demanded
/// column does NOT produce a structured `NoRoute` decline.
///
/// Cause: canonical `seed_fill_block` (`SupportMaterial.cpp`) block-snaps the
/// propagated carry outward by up to one macro-block extent, so by the time the
/// carry reaches the blocking layer it already covers ground outside the
/// contact that the blocker does not cover. The per-layer difference therefore
/// stays non-empty, `blocked_at` is never set, and neither
/// `SupportPlanDeclineReason::NoRoute` nor diagnostic `code: 1203` fires.
/// `legacy_semantic` (the default) declines correctly on the same fixture; this
/// test asserts BOTH halves so the divergence cannot silently close or widen.
#[test]
fn agg_does_not_decline_no_route_where_legacy_semantic_does() {
    // A 6 mm contact square with the model covering the whole contact area on
    // exactly one descended layer.
    let contact = vec![ExPolygon {
        contour: rect_mm(2.0, 2.0, 8.0, 8.0),
        holes: vec![],
    }];
    let blocker = vec![ExPolygon {
        contour: rect_mm(2.0, 2.0, 8.0, 8.0),
        holes: vec![],
    }];
    let analysis = analysis_fixture_single_blocker(contact, blocker, 1);

    // Reference half: `legacy_semantic` (the default) declines the column.
    let legacy = plan_with(Some("legacy_semantic"), &analysis);
    assert_eq!(
        legacy
            .entries()
            .iter()
            .filter(|entry| entry.decline_reason == Some(SupportPlanDeclineReason::NoRoute))
            .count(),
        1,
        "legacy_semantic must record exactly one NoRoute decline on this fixture"
    );
    assert!(
        legacy
            .diagnostics()
            .iter()
            .any(|d| d.code == 1203 && d.severity == DiagnosticSeverity::Warn),
        "legacy_semantic must warn with code 1203 on this fixture"
    );

    // Divergent half: `agg` records NO decline and NO 1203 diagnostic, and
    // instead keeps emitting real geometry through the blocked layer.
    let agg = plan_with(Some("agg"), &analysis);
    assert!(
        agg.entries()
            .iter()
            .all(|entry| entry.decline_reason.is_none()),
        "DEV-166: agg records no structured decline at all, got {:?}",
        agg.entries()
            .iter()
            .map(|e| (e.global_layer_index, e.decline_reason))
            .collect::<Vec<_>>()
    );
    assert!(
        agg.diagnostics().iter().all(|d| d.code != 1203),
        "DEV-166: agg never emits the NoRoute diagnostic on this fixture, got {:?}",
        agg.diagnostics()
    );

    // The precise observed outcome, MEASURED in this session: instead of one
    // declined entry, agg emits four accepted entries -- layers 3, 2, 1, 0 --
    // each carrying exactly one role region. Layer 1 is the blocked layer, and
    // agg prints support on it and on layer 0 beneath it.
    let agg_layers: Vec<i32> = agg
        .entries()
        .iter()
        .map(|entry| entry.global_layer_index)
        .collect();
    assert_eq!(
        agg_layers,
        vec![3, 2, 1, 0],
        "DEV-166: agg descends past the blocker to the plate"
    );
    for entry in agg.entries() {
        assert_eq!(
            entry.roles.len(),
            1,
            "layer {}: agg emits exactly one role region set",
            entry.global_layer_index
        );
        assert!(
            entry
                .roles
                .iter()
                .any(|role| role
                    .regions
                    .iter()
                    .any(|region| region.contour.points.len() >= 3)),
            "layer {}: agg emits real printed geometry where the route is closed",
            entry.global_layer_index
        );
    }
}

/// Axis-aligned bounding box of a set of expolygons: `(min_x, min_y, max_x, max_y)`.
fn bbox(polys: &[ExPolygon]) -> (i64, i64, i64, i64) {
    let mut b = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
    for poly in polys {
        for pt in &poly.contour.points {
            b.0 = b.0.min(pt.x);
            b.1 = b.1.min(pt.y);
            b.2 = b.2.max(pt.x);
            b.3 = b.3.max(pt.y);
        }
    }
    b
}

/// DEV-166 divergence: under `agg`, the propagated carry grows beyond its
/// pre-grid extent, and the growth is bounded by one macro-block extent.
///
/// Cause: canonical `seed_fill_block` (`SupportMaterial.cpp`) floods every
/// macro block that contains any set cell, so the carry snaps out to whole
/// macro-block boundaries. A macro block is `oversampling * pixel_size` units
/// wide, both derived by `GridParams::from_polygons` from the base-pattern
/// spacing and the line width -- the bound below is DERIVED from those grid
/// params in-test, never hardcoded.
#[test]
fn agg_propagated_carry_grows_by_at_most_one_macro_block_extent() {
    let carry = vec![ExPolygon {
        contour: rect_mm(2.0, 2.0, 8.0, 8.0),
        holes: vec![],
    }];
    let spacing_mm = 2.5_f32;
    let width_mm = 0.4_f32;

    // The same construction the planner's `agg` arm performs, with an empty
    // trimming region so the growth measured here is purely the block snap.
    let grid = SupportGrid::new(&carry, &[], spacing_mm, width_mm);
    let params = grid.params();
    let macro_block = params.oversampling as i64 * params.pixel_size;

    // The planner clamps `OFFSET_TO_PROPAGATE` (-1) into the in-cell bound.
    let offset_to_propagate = (-1i64).clamp(-max_in_cell_offset(params.pixel_size), 0);
    // `filter_islands_by_samples` only DROPS islands, it never grows one (see
    // `contour_extraction_filters_islands_by_samples`), so the unfiltered
    // extraction is an upper bound on the propagated carry's extent.
    let propagated = grid.extract_islands(offset_to_propagate, true);
    assert!(!propagated.is_empty(), "the carry survives propagation");

    // MEASURED in this session at spacing 2.5 mm / width 0.4 mm:
    // `pixel_size` 4167, `oversampling` 6, so one macro block is 25002 units
    // (2.5002 mm). Pinned so a change in the grid arithmetic cannot silently
    // move the bound this test enforces.
    assert_eq!(params.pixel_size, 4167, "measured pixel_size at 2.5 mm / 0.4 mm");
    assert_eq!(params.oversampling, 6, "measured oversampling at 2.5 mm / 0.4 mm");
    assert_eq!(macro_block, 25_002, "one macro block is oversampling * pixel_size");

    let before = bbox(&carry);
    let after = bbox(&propagated);

    // MEASURED in this session: the 6 mm contact square, bbox
    // (20000, 20000)-(80000, 80000), comes back as (1, 1)-(100007, 100007) --
    // it grew by 19999 units on the low sides and 20007 on the high sides,
    // every one of them inside the 25002-unit macro-block bound.
    assert_eq!(before, (20_000, 20_000, 80_000, 80_000), "pre-grid carry extent");
    assert_eq!(
        after,
        (1, 1, 100_007, 100_007),
        "measured block-snapped carry extent (DEV-166)"
    );

    // The carry really does grow: this is the divergence, not a no-op.
    assert!(
        after.0 < before.0 && after.1 < before.1 && after.2 > before.2 && after.3 > before.3,
        "DEV-166: the block snap grows the carry on every side, before={before:?} after={after:?}"
    );
    // ...and the growth is bounded by one macro-block extent on every side.
    for (grew, side) in [
        (before.0 - after.0, "min_x"),
        (before.1 - after.1, "min_y"),
        (after.2 - before.2, "max_x"),
        (after.3 - before.3, "max_y"),
    ] {
        assert!(
            grew > 0 && grew <= macro_block,
            "DEV-166: {side} grew by {grew}, which must lie in (0, {macro_block}]"
        );
    }
}

// ---------------------------------------------------------------------------
// Support-region identity: one entry per (layer, object, region).
// ---------------------------------------------------------------------------

/// Two traditional candidates on ONE object/region, far enough apart that
/// neither column's macro-block halo can reach the other.
fn analysis_fixture_two_candidates(
    first: Vec<ExPolygon>,
    second: Vec<ExPolygon>,
    occupancy: Vec<ExPolygon>,
) -> SupportAnalysisView {
    SupportAnalysisView {
        candidates: vec![
            SupportAnalysisCandidate {
                id: 7,
                geometry: first,
                object_id: "obj-1".into(),
                region_id: "region-0".into(),
                global_layer_index: 5,
                ..Default::default()
            },
            SupportAnalysisCandidate {
                id: 8,
                geometry: second,
                object_id: "obj-1".into(),
                region_id: "region-0".into(),
                global_layer_index: 5,
                ..Default::default()
            },
        ],
        model_occupancy: (0..6)
            .map(|layer| SupportAnalysisGeometryEntry {
                global_support_layer_index: layer,
                object_id: "obj-1".into(),
                region_id: "region-0".into(),
                polygons: occupancy.clone(),
            })
            .collect(),
        family_assignments: vec![SupportFamilyAssignment {
            object_id: "obj-1".into(),
            region_id: "region-0".into(),
            family_id: "traditional".into(),
        }],
        ..Default::default()
    }
}

/// Regression (packet 241, step 17). Two demands on one object/region that
/// both reach a layer must be published as ONE entry for that layer, carrying
/// the union of their geometry and both body identities.
///
/// `SupportPlanIR::duplicate_region_identity` (`crates/slicer-ir/src/slice_ir.rs`),
/// checked by `Blackboard::commit_support_plan`
/// (`crates/slicer-runtime/src/blackboard.rs`), admits exactly one entry per
/// `(global_layer_index, object_id, region_id)`. Before
/// `merge_region_identity_entries` this planner emitted one entry per
/// candidate per layer and left the union to host
/// `union_same_family_entries` (`crates/slicer-wasm-host/src/support_aggregation.rs`),
/// whose key is `same_body` or the bounding-box-centroid routing cell and
/// never `region_id` - so two columns of one region whose centroids fell in
/// different cells reached the blackboard as duplicates and the whole prepass
/// was rejected. Runs in BOTH rasterizer modes: the emission path is shared,
/// and only the `agg` halo made the wedge cross a cell boundary.
#[test]
fn two_candidates_in_one_region_are_published_as_one_entry_per_layer() {
    let first = vec![ExPolygon {
        contour: rect_mm(2.0, 2.0, 4.0, 4.0),
        holes: vec![],
    }];
    let second = vec![ExPolygon {
        contour: rect_mm(16.0, 16.0, 18.0, 18.0),
        holes: vec![],
    }];
    // A bar that touches neither column, so both descend to the plate.
    let occupancy = vec![ExPolygon {
        contour: rect_mm(9.0, 0.0, 10.0, 20.0),
        holes: vec![],
    }];

    for mode in ["legacy_semantic", "agg"] {
        let both = plan_with(
            Some(mode),
            &analysis_fixture_two_candidates(first.clone(), second.clone(), occupancy.clone()),
        );

        let mut identities = HashMap::<(i32, String, String), usize>::new();
        for entry in both.entries() {
            *identities
                .entry((
                    entry.global_layer_index,
                    entry.object_id.clone(),
                    entry.region_id.clone(),
                ))
                .or_default() += 1;
        }
        let duplicated: Vec<_> = identities.iter().filter(|(_, count)| **count > 1).collect();
        assert!(
            duplicated.is_empty(),
            "{mode}: every (layer, object, region) identity must be published once, got {duplicated:?}"
        );

        // Both demands survive the union, in identity and in geometry.
        for entry in both.entries().iter().filter(|e| e.decline_reason.is_none()) {
            assert_eq!(
                entry.body_ids,
                vec![
                    "traditional-body-obj-1-7".to_string(),
                    "traditional-body-obj-1-8".to_string()
                ],
                "{mode}: the merged entry must carry both bodies"
            );
        }

        // Nothing is dropped: the merged plan covers exactly the two
        // single-candidate plans, layer for layer and unit for unit. The
        // columns are 12 mm apart, far outside one macro-block extent, so the
        // union cannot overlap and the areas add.
        let only_first = plan_with(
            Some(mode),
            &analysis_fixture(first.clone(), occupancy.clone()),
        );
        let only_second = plan_with(
            Some(mode),
            &analysis_fixture(second.clone(), occupancy.clone()),
        );
        let merged_areas = area_by_layer(&both);
        let first_areas = area_by_layer(&only_first);
        let second_areas = area_by_layer(&only_second);
        assert_eq!(
            merged_areas.iter().map(|(layer, _)| *layer).collect::<Vec<_>>(),
            first_areas.iter().map(|(layer, _)| *layer).collect::<Vec<_>>(),
            "{mode}: merging must not change which layers are emitted"
        );
        for ((layer, merged), ((_, alone_first), (_, alone_second))) in merged_areas
            .iter()
            .zip(first_areas.iter().zip(second_areas.iter()))
        {
            assert_eq!(
                *merged,
                alone_first + alone_second,
                "{mode}: layer {layer} lost area in the union"
            );
        }
    }
}
