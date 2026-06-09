//! Red-first TDD: each fill-claim infill module (rectilinear, gyroid, lightning)
//! must emit each role over its dedicated host-partitioned polygon and ONLY that
//! polygon — never over the wall-inset outline, never over sibling roles.
//!
//! Contract (per `docs/specs/infill-fill-partition-plan.md` Q3 + Q7):
//! - SparseInfill paths confined to `region.sparse_infill_area()`.
//! - TopSolidInfill confined to `region.top_solid_fill()`.
//! - BottomSolidInfill confined to `region.bottom_solid_fill()`.
//! - BridgeInfill confined to `region.bridge_areas()`.
//! - When a claim's source polygon is empty, zero paths of that role emit.
//!
//! The current per-region role-pick (top_shell_index.is_some() ladder) in each
//! module IS the bug under fix — every test here is red until Phase 2.2 lands.

use gyroid_infill::GyroidInfill;
use lightning_infill::LightningInfill;
use rectilinear_infill::RectilinearInfill;
use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, Point2, Polygon,
};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::test_support::fixtures::SliceRegionViewBuilder;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::SliceRegionView;

// ── fixture helpers ──────────────────────────────────────────────────────────

fn square(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(min_x, min_y),
                Point2::from_mm(max_x, min_y),
                Point2::from_mm(max_x, max_y),
                Point2::from_mm(min_x, max_y),
            ],
        },
        holes: Vec::new(),
    }
}

fn min_density_config() -> ConfigView {
    let mut map = std::collections::HashMap::new();
    map.insert("infill_density".into(), ConfigValue::Float(0.5));
    map.insert("line_width".into(), ConfigValue::Float(0.4));
    ConfigView::from_map(map)
}

/// AABB containment test in mm space. Returns true if every point of `path`
/// lies inside the AABB of any polygon in `containers` (with epsilon).
fn path_inside_any(path: &ExtrusionPath3D, containers: &[ExPolygon]) -> bool {
    if containers.is_empty() {
        return false;
    }
    for pt in &path.points {
        if !point_in_any_aabb_mm(pt.x, pt.y, containers) {
            return false;
        }
    }
    true
}

fn point_in_any_aabb_mm(x_mm: f32, y_mm: f32, polys: &[ExPolygon]) -> bool {
    const EPS: f32 = 0.001; // 1 µm tolerance
    for ep in polys {
        let (mut min_x, mut min_y) = (f32::INFINITY, f32::INFINITY);
        let (mut max_x, mut max_y) = (f32::NEG_INFINITY, f32::NEG_INFINITY);
        for p in &ep.contour.points {
            let px = p.x as f32 / 10_000.0; // unit→mm
            let py = p.y as f32 / 10_000.0;
            if px < min_x {
                min_x = px;
            }
            if py < min_y {
                min_y = py;
            }
            if px > max_x {
                max_x = px;
            }
            if py > max_y {
                max_y = py;
            }
        }
        if x_mm >= min_x - EPS && x_mm <= max_x + EPS && y_mm >= min_y - EPS && y_mm <= max_y + EPS
        {
            return true;
        }
    }
    false
}

/// Build a region with the four canonical polygons placed in disjoint quadrants
/// of a 10×10 wall_inset. Each quadrant is 5×5 = 25 mm².
fn region_with_disjoint_quadrants() -> SliceRegionView {
    let bottom_left = square(0.0, 0.0, 5.0, 5.0);
    let bottom_right = square(5.0, 0.0, 10.0, 5.0);
    let top_left = square(0.0, 5.0, 5.0, 10.0);
    let top_right = square(5.0, 5.0, 10.0, 10.0);
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(0)
        .z(0.2)
        .effective_layer_height(0.2)
        .add_polygon(square(0.0, 0.0, 10.0, 10.0))
        // sparse → bottom_left quadrant
        .sparse_infill_area(vec![bottom_left])
        // top → top_left quadrant; top_shell_index participation
        .top_shell_index(Some(0))
        .top_solid_fill(vec![top_left])
        // bottom → bottom_right quadrant; bottom_shell_index participation
        .bottom_shell_index(Some(0))
        .bottom_solid_fill(vec![bottom_right])
        // bridge → top_right quadrant
        .is_bridge(true)
        .bridge_areas(vec![top_right])
        .bridge_orientation_deg(0.0)
        .build()
}

/// All collected paths (sparse + solid + ironing) for assertion.
fn collect_all_paths(output: &InfillOutputBuilder) -> Vec<ExtrusionPath3D> {
    output
        .sparse_paths()
        .iter()
        .chain(output.solid_paths().iter())
        .cloned()
        .collect()
}

fn paths_with_role(paths: &[ExtrusionPath3D], role: ExtrusionRole) -> Vec<ExtrusionPath3D> {
    paths
        .iter()
        .filter(|p| p.role == role && p.points.len() > 1)
        .cloned()
        .collect()
}

// ── module dispatch table ────────────────────────────────────────────────────

enum FillModule {
    Rectilinear(RectilinearInfill),
    Gyroid(GyroidInfill),
    Lightning(LightningInfill),
}

impl FillModule {
    fn run(
        &self,
        layer_index: u32,
        regions: &[SliceRegionView],
        output: &mut InfillOutputBuilder,
        config: &ConfigView,
    ) {
        match self {
            Self::Rectilinear(m) => m.run_infill(layer_index, regions, output, config).unwrap(),
            Self::Gyroid(m) => m.run_infill(layer_index, regions, output, config).unwrap(),
            Self::Lightning(m) => m.run_infill(layer_index, regions, output, config).unwrap(),
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Rectilinear(_) => "rectilinear-infill",
            Self::Gyroid(_) => "gyroid-infill",
            Self::Lightning(_) => "lightning-infill",
        }
    }
}

fn all_three_modules() -> Vec<FillModule> {
    let cfg = min_density_config();
    vec![
        FillModule::Rectilinear(RectilinearInfill::on_print_start(&cfg).unwrap()),
        FillModule::Gyroid(GyroidInfill::on_print_start(&cfg).unwrap()),
        FillModule::Lightning(LightningInfill::on_print_start(&cfg).unwrap()),
    ]
}

// ── AC-7: per-role paths confined to per-role source polygon ─────────────────

#[test]
fn ac7_each_role_confined_to_its_own_canonical_polygon_for_all_three_modules() {
    let region = region_with_disjoint_quadrants();

    let bottom_left = vec![square(0.0, 0.0, 5.0, 5.0)];
    let bottom_right = vec![square(5.0, 0.0, 10.0, 5.0)];
    let top_left = vec![square(0.0, 5.0, 5.0, 10.0)];
    let top_right = vec![square(5.0, 5.0, 10.0, 10.0)];

    for module in all_three_modules() {
        let mut output = InfillOutputBuilder::new();
        module.run(
            0,
            std::slice::from_ref(&region),
            &mut output,
            &min_density_config(),
        );
        let all = collect_all_paths(&output);

        let sparse = paths_with_role(&all, ExtrusionRole::SparseInfill);
        let top = paths_with_role(&all, ExtrusionRole::TopSolidInfill);
        let bot = paths_with_role(&all, ExtrusionRole::BottomSolidInfill);
        let br = paths_with_role(&all, ExtrusionRole::BridgeInfill);

        for p in &sparse {
            assert!(
                path_inside_any(p, &bottom_left),
                "[{}] SparseInfill path escaped sparse_infill_area (bottom_left quadrant)",
                module.name()
            );
        }
        for p in &top {
            assert!(
                path_inside_any(p, &top_left),
                "[{}] TopSolidInfill path escaped top_solid_fill (top_left quadrant)",
                module.name()
            );
        }
        for p in &bot {
            assert!(
                path_inside_any(p, &bottom_right),
                "[{}] BottomSolidInfill path escaped bottom_solid_fill (bottom_right quadrant)",
                module.name()
            );
        }
        for p in &br {
            assert!(
                path_inside_any(p, &top_right),
                "[{}] BridgeInfill path escaped bridge_areas (top_right quadrant)",
                module.name()
            );
        }
    }
}

// ── AC-8: empty sparse_infill_area → zero SparseInfill paths regardless of flag ─

#[test]
fn ac8_empty_sparse_infill_area_yields_zero_sparse_paths_even_with_top_flag_set() {
    let region = SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(0)
        .z(0.2)
        .effective_layer_height(0.2)
        .add_polygon(square(0.0, 0.0, 10.0, 10.0))
        // sparse_infill_area is empty (pure top layer)
        .sparse_infill_area(Vec::new())
        // top_shell_index flag set + top_solid_fill populated
        .top_shell_index(Some(0))
        .top_solid_fill(vec![square(0.0, 0.0, 10.0, 10.0)])
        .build();

    for module in all_three_modules() {
        let mut output = InfillOutputBuilder::new();
        module.run(
            0,
            std::slice::from_ref(&region),
            &mut output,
            &min_density_config(),
        );
        let sparse = paths_with_role(&collect_all_paths(&output), ExtrusionRole::SparseInfill);
        assert!(
            sparse.is_empty(),
            "[{}] expected 0 SparseInfill paths when sparse_infill_area is empty; got {}",
            module.name(),
            sparse.len()
        );
    }
}

// ── AC-9: all four polygons empty → zero paths, no panic ─────────────────────

#[test]
fn ac9_all_four_polygons_empty_yields_zero_paths_no_panic() {
    let region = SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(0)
        .z(0.2)
        .effective_layer_height(0.2)
        .add_polygon(square(0.0, 0.0, 10.0, 10.0))
        .sparse_infill_area(Vec::new())
        .top_solid_fill(Vec::new())
        .bottom_solid_fill(Vec::new())
        .bridge_areas(Vec::new())
        .build();

    for module in all_three_modules() {
        let mut output = InfillOutputBuilder::new();
        module.run(
            0,
            std::slice::from_ref(&region),
            &mut output,
            &min_density_config(),
        );
        let all = collect_all_paths(&output);
        assert!(
            all.is_empty(),
            "[{}] expected 0 paths when all four polygons empty; got {}",
            module.name(),
            all.len()
        );
    }
}

// ── NEG-1: should_emit gating still works under the new structure ────────────

#[test]
fn neg1_top_solid_fill_populated_but_top_claim_not_held_yields_zero_top_paths() {
    let region = SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(0)
        .z(0.2)
        .effective_layer_height(0.2)
        .add_polygon(square(0.0, 0.0, 10.0, 10.0))
        .top_shell_index(Some(0))
        .top_solid_fill(vec![square(0.0, 0.0, 10.0, 10.0)])
        // sparse + bottom + bridge empty
        .build();

    // Force should_emit to gate out claim:top-fill by populating held_claims
    // with only the sparse-fill claim. The (now non-empty) sparse_infill_area
    // stays empty → zero sparse paths too.
    let mut held_only_sparse = region.clone();
    held_only_sparse.set_held_claims(vec!["claim:sparse-fill".into()]);

    for module in all_three_modules() {
        let mut output = InfillOutputBuilder::new();
        module.run(
            0,
            std::slice::from_ref(&held_only_sparse),
            &mut output,
            &min_density_config(),
        );
        let top_paths = paths_with_role(&collect_all_paths(&output), ExtrusionRole::TopSolidInfill);
        assert!(
            top_paths.is_empty(),
            "[{}] held_claims excludes claim:top-fill → no TopSolidInfill paths; got {}",
            module.name(),
            top_paths.len()
        );
    }
}
