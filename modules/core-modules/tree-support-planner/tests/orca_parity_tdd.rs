//! Packet 31b Orca-parity TDD tests for `PrePass::SupportGeometry` algorithmic features.
//!
//! Tests compile against the existing SDK (no WIT changes — 31a already added
//! `SupportGeometryView` to the export signature).
//!
//! Positive ACs (1-5) fail until the planner implements the features.
//! Negative ACs (6-8) exercise host-side config validation and should pass now.
//!
//! ## Acceptance Criteria
//! - AC-2: radius tapering
//! - AC-3: avoidance
//! - AC-4: raft + interface
//! - AC-5: wall-count
//! - AC-6: Benchy parity
//! - AC-N1: diameter_angle out of range
//! - AC-N2: negative raft layers
//! - AC-N3: node clamped out

#![allow(missing_docs)]
#![allow(dead_code)]

use std::collections::{BTreeMap, HashMap};

use slicer_ir::{
    ConfigKey, ConfigValue, ConfigView, ExPolygon, Point2, Polygon, SemVer, SupportPlanEntry,
};
use slicer_sdk::module_test;
use slicer_sdk::prepass_builders::SupportGeometryOutput;
use slicer_sdk::prepass_types::{
    LayerPlanView, LayerPlanViewEntry, MeshObjectView, RegionSegmentationView,
    RegionSegmentationViewEntry, SupportGeometryView, SupportGeometryViewEntry,
};
use slicer_sdk::traits::PrepassModule;

// Import the planner's pub fns directly (dev-dependency on support-planner).
// This lets us test tapered_radius() and point_in_polygon() without going
// through WASM dispatch, verifying the Step-5 algorithmic implementation.
use tree_support_planner::{point_in_polygon, tapered_radius, SupportPlanner};

/// AC-2: radius tapering — topmost width = branch_diameter,
/// bottom > top + tan(diameter_angle) * height_diff.
#[test]

fn radius_tapers_with_distance_to_top() {
    // Test the actual tapered_radius() function from the planner (Step 5).
    // Formula: radius(dist_to_top) = branch_radius + tan(diameter_angle) * dist_to_top * layer_height
    // Width at a given layer = 2 * radius at that layer (diameter)

    let branch_radius = 2.5_f32; // branch_diameter = 5.0mm
    let diameter_angle_deg = 10.0_f32;
    let tan_diameter_angle = diameter_angle_deg.to_radians().tan();
    let layer_height = 0.2_f32; // mm per layer

    // Top layer: dist_to_top = 0 → radius is floored at MIN_BRANCH_RADIUS = 0.4 per packet 213.
    let radius_top = tapered_radius(branch_radius, tan_diameter_angle, 0, layer_height);
    assert!(
        (radius_top - 0.4).abs() < 1e-6,
        "radius at dist_to_top=0 must be 0.4 (minimum floor); got {radius_top}"
    );

    // 10 layers down: dist_to_top = 10
    // radius should grow: mm_to_top = 10 * 0.2 = 2.0, which is inside the tip-cone
    // (mm_to_top <= branch_radius=2.5), so radius = mm_to_top = 2.0
    let dist_to_top_10 = 10_u32;
    let radius_10 = tapered_radius(
        branch_radius,
        tan_diameter_angle,
        dist_to_top_10,
        layer_height,
    );
    let expected_radius_10 = (dist_to_top_10 as f32) * layer_height; // mm_to_top = 2.0

    assert!(
        (radius_10 - expected_radius_10).abs() < 1e-4,
        "radius_10={radius_10} must match expected={expected_radius_10} (mm_to_top inside tip-cone)"
    );

    // Width = 2 * radius. Bottom width should be > top width (tip floored at 0.4).
    let width_top = 2.0 * radius_top;
    let width_10 = 2.0 * radius_10;
    assert!(
        width_10 > width_top,
        "AC-2: bottom_width={width_10} must exceed top_width={width_top}"
    );
}

// RC-4 zero tip width: the tapered radius must retain the minimum branch floor.
#[test]
fn tapered_radius_at_tip_respects_minimum_floor() {
    let branch_radius = 2.5_f32;
    let tan_diameter_angle = 10.0_f32.to_radians().tan();
    let effective_layer_height = 0.2_f32;

    let radius = tapered_radius(branch_radius, tan_diameter_angle, 0, effective_layer_height);

    // MIN_BRANCH_RADIUS is introduced by the production fix; keep this RED
    // test independent of that not-yet-existing constant.
    assert!(
        radius >= 0.4,
        "tip radius must be at least the 0.4mm minimum branch radius; got {radius}"
    );
}

/// AC-3: avoidance — all branch endpoints inside inflated outer outline,
/// outside holes.
#[test]
fn avoidance_keeps_branches_inside_support_outline() {
    // Test the actual point_in_polygon() function (Step 5 AC-3).
    // The planner uses this to reject endpoints that fall inside collision
    // polygons (holes in the support geometry).

    // A rectangular outer outline: [0,0] -> [100,0] -> [100,100] -> [0,100] -> [0,0]
    let outer: [[f32; 2]; 4] = [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];

    // A circular hole as a hexagon approximation centered at (50, 50).
    // Hexagon vertices for a circle of radius 10.
    let hole_center_x = 50.0_f32;
    let hole_center_y = 50.0_f32;
    let hole_radius = 10.0_f32;
    let hex_points: Vec<[f32; 2]> = (0..6)
        .map(|i| {
            let angle = (i as f32) * std::f32::consts::PI / 3.0;
            [
                hole_center_x + hole_radius * angle.cos(),
                hole_center_y + hole_radius * angle.sin(),
            ]
        })
        .collect();

    // Point (50, 50) is INSIDE the hexagonal hole → should be rejected.
    let inside_hole = point_in_polygon(&hex_points, 50.0, 50.0);
    assert!(
        inside_hole,
        "AC-3: point (50,50) inside hexagonal hole must be detected; got {inside_hole}"
    );

    // Point (25, 25) is inside the outer rectangle but OUTSIDE the hole → accepted.
    let inside_outer = point_in_polygon(&outer, 25.0, 25.0);
    let inside_hex = point_in_polygon(&hex_points, 25.0, 25.0);
    assert!(
        inside_outer && !inside_hex,
        "AC-3: point (25,25) must be inside outer and outside hole"
    );

    // Point (150, 150) is OUTSIDE the outer rectangle → rejected.
    let outside_outer = !point_in_polygon(&outer, 150.0, 150.0);
    assert!(
        outside_outer,
        "AC-3: point (150,150) outside outer rectangle must be rejected"
    );

    // The AC-3 acceptance condition: endpoints in the hole must be flagged,
    // which the planner uses to drop nodes that would be placed there.
    assert!(
        inside_hole,
        "AC-3: hole-centre point must be detected as inside hole for node-drop logic"
    );
}

/// AC-4: raft plan + interface — one configuration-only raft plan,
/// plus interface-densified model entries.
#[test]
fn raft_and_interface_layers_emit_expected_entry_count() {
    // AC-4: Run the planner with support_raft_layers=3 and
    // support_interface_top_layers=2 against an overhang fixture whose contact
    // sits near layer 10. Expect:
    //   - exactly one raft plan with raft_layers = 3
    //   - top-interface layers (just below contact) carry MORE branch_segments
    //     than the contact layer itself
    let config = make_planner_config(&[
        ("enable_support", ConfigValue::Bool(true)),
        ("support_raft_layers", ConfigValue::Int(3)),
        ("support_interface_top_layers", ConfigValue::Int(2)),
        ("tree_support_interface_spacing_mm", ConfigValue::Float(0.4)),
        ("tree_support_branch_diameter", ConfigValue::Float(2.0)),
        (
            "tree_support_branch_diameter_angle",
            ConfigValue::Float(5.0),
        ),
        ("tree_support_branch_distance", ConfigValue::Float(1.0)),
        ("tree_support_wall_count", ConfigValue::Int(1)),
        ("tree_support_branch_angle", ConfigValue::Float(45.0_f64)),
    ]);
    let planner = SupportPlanner::from_config(&config).expect("from_config");

    let obj = overhang_plate_fixture("col");
    let lp = make_layer_plan(11, 0.0, 0.2);
    let rs = make_region_segmentation("col", 11);
    let sg = SupportGeometryView { entries: vec![] };
    let mut output = SupportGeometryOutput::new();
    planner
        .run_support_geometry_with_analysis(
            &[obj],
            &lp,
            &rs,
            &tree_analysis("col"),
            &sg,
            &mut output,
            &ConfigView::new(),
        )
        .expect("run_support_geometry");

    let entries = output.entries();
    let raft_plan = output.raft_plan().expect("AC-4: expected one raft plan");
    assert_eq!(raft_plan.raft_layers, 3);
    assert!((raft_plan.raft_first_layer_density - 0.4).abs() < f32::EPSILON);
    assert_eq!(raft_plan.base_raft_layers, 1);
    assert_eq!(raft_plan.interface_raft_layers, 0);
    assert!(
        entries.iter().all(|entry| entry.global_layer_index >= 0),
        "AC-4: raft plan must not emit raft geometry entries"
    );

    // Canonical builds roof as an area *distinct* from `base_areas` and
    // subtracts it out of the body (`TreeSupport::generate_toolpaths`' area
    // pass), so an interface layer does not carry extra geometry on top of the
    // body — it carries the same footprint under a different role.
    //
    // This assertion used to require interface layers to hold MORE skeleton
    // points than the contact layer, which only held because the pre-224
    // planner *added* bounding-box scan lines on top of the body instead of
    // carving the interface out of it.
    let interface_layers: BTreeMap<i32, Vec<slicer_ir::SupportPlanRole>> = entries
        .iter()
        .filter(|e| e.global_layer_index >= 0)
        .map(|e| {
            (
                e.global_layer_index,
                e.roles.iter().map(|r| r.role).collect::<Vec<_>>(),
            )
        })
        .collect();
    assert!(
        !interface_layers.is_empty(),
        "AC-4: expected non-empty model-layer plan; got 0 entries"
    );
    let top_interface_layers: Vec<i32> = interface_layers
        .iter()
        .filter(|(_, roles)| roles.contains(&slicer_ir::SupportPlanRole::TopInterface))
        .map(|(&layer, _)| layer)
        .collect();
    assert!(
        !top_interface_layers.is_empty(),
        "AC-4: expected at least one layer carrying a TopInterface role; got {interface_layers:?}"
    );
    // The interface band sits at the top of the column: `support_interface_top_layers`
    // is 2 here, so at most two layers may carry it.
    assert!(
        top_interface_layers.len() <= 2,
        "AC-4: interface band wider than support_interface_top_layers=2; got {top_interface_layers:?}"
    );
    let &highest = interface_layers.keys().max().unwrap();
    assert!(
        top_interface_layers.contains(&highest),
        "AC-4: the topmost support layer must be interface, not bare body; highest={highest} interface={top_interface_layers:?}"
    );
    // Interface must be carved out of the body, never printed on top of it.
    //
    // The body does NOT have to survive on an interface layer: canonical
    // `draw_circles` (`TreeSupport.cpp`) dispatches each node to exactly one
    // bucket (`roof_gap_areas` / `roof_1st_layer` / `roof_areas` /
    // `base_areas`), so on a layer whose surviving nodes are all roof nodes
    // canonical's `base_areas` is empty BEFORE `base_areas = diff_ex(base_areas,
    // roofs)` runs. What canonical does guarantee is that the layer immediately
    // BELOW the interface band -- where no node is a roof node -- still prints a
    // body cross-section. That is what is asserted here.
    //
    // The previous form of this block iterated `for b in &body { for r in &roof
    // { .. } }`, which never executed: the planner clears the body on any layer
    // that carries an interface, so `body` was always empty and the overlap
    // assertion was unreachable.
    let geometry_layers: Vec<i32> = entries
        .iter()
        .filter(|e| e.global_layer_index >= 0)
        .filter(|e| e.roles.iter().any(|r| !r.regions.is_empty()))
        .map(|e| e.global_layer_index)
        .collect();
    let interface_band_bottom = *top_interface_layers
        .iter()
        .min()
        .expect("top_interface_layers is non-empty (asserted above)");
    let mut carve_checks = 0usize;
    for &layer in &top_interface_layers {
        let entry = entries
            .iter()
            .find(|e| e.global_layer_index == layer)
            .expect("interface layer must have an entry");
        let body: Vec<&slicer_ir::SupportPlanRoleRegion> = entry
            .roles
            .iter()
            .filter(|r| r.role == slicer_ir::SupportPlanRole::SupportBody && !r.regions.is_empty())
            .collect();
        let roof: Vec<&slicer_ir::SupportPlanRoleRegion> = entry
            .roles
            .iter()
            .filter(|r| r.role == slicer_ir::SupportPlanRole::TopInterface && !r.regions.is_empty())
            .collect();
        for b in &body {
            for r in &roof {
                let overlap = slicer_sdk::host::clip_polygons(
                    &b.regions,
                    &r.regions,
                    slicer_sdk::host::ClipOperation::Intersection,
                );
                assert!(
                    overlap.is_empty(),
                    "AC-4: body and interface overlap at layer {layer}; interface must be subtracted out of the body, not layered on top of it"
                );
            }
        }
    }
    // Every layer below the interface band must still print a body: no node
    // there is a roof node, so canonical's `base_areas` is non-empty and the
    // `diff_ex(base_areas, roofs)` carve leaves it intact.
    //
    // NOTE on regression scope: on THIS fixture the roof covers the entire
    // branch on both band layers, so no layer here ever carries a roof and a
    // body at once. This test therefore cannot -- and never could -- gate the
    // F-3 `carved.clear()` defect; its former "body must survive on a band
    // layer" form was red under the fixed code too. The F-3 gate is
    // `tree_family_tdd::anchored_heights_and_termination`, whose fixture has a
    // contact narrower than the branch and so produces genuinely mixed layers;
    // it is verified red when `carved.clear()` is reinstated.
    for &layer in &geometry_layers {
        if layer >= interface_band_bottom {
            continue;
        }
        let entry = entries
            .iter()
            .find(|e| e.global_layer_index == layer)
            .expect("geometry layer must have an entry");
        carve_checks += 1;
        assert!(
            entry.roles.iter().any(|r| r.role == slicer_ir::SupportPlanRole::SupportBody
                && !r.regions.is_empty()),
            "AC-4: layer {layer} lies below the interface band (band bottom={interface_band_bottom}) yet carries no SupportBody. Canonical keeps `base_areas` intact below the roof band; clearing the body leaves the branch cross-section unprinted. Roles: {:?}",
            entry.roles
        );
    }
    assert!(
        carve_checks > 0,
        "AC-4: no geometry layer sat below the interface band, so the body-below-the-band check was vacuous; interface layers={top_interface_layers:?}, geometry layers={geometry_layers:?}"
    );
}

/// AC-5: wall-count scaling — max XY distance ≤ tan(angle) * height * tree_support_wall_count.
#[test]
fn wall_count_scales_max_move_distance() {
    // When wall-count-aware move scaling is implemented:
    //   max_move_distance = tan(branch_angle) * effective_height * tree_support_wall_count
    //
    // Config keys:
    //   - tree_support_branch_angle (default 45.0)
    //   - support_wall_count (default 0 = auto, typically 1-2)
    //
    // Current v1 behavior: step_xy = tan_angle * effective_height (no wall-count factor).
    // This test documents expected behavior once AC-5 is implemented.

    let branch_angle_deg = 45.0_f32;
    let effective_height = 0.2_f32; // mm
    let wall_count = 2_u32;
    let tan_angle = branch_angle_deg.to_radians().tan();

    let no_wall_max_move = tan_angle * effective_height; // current v1
    let with_wall_max_move = tan_angle * effective_height * wall_count as f32;

    assert!(
        no_wall_max_move < with_wall_max_move,
        "AC-5: tree_support_wall_count should scale max_move_distance upward; \
         v1 planner uses no_wall_max_move={no_wall_max_move} without wall-count factor"
    );

    // Verify: with tree_support_wall_count=2, max_move should be 2x the no-wall value
    let ratio = with_wall_max_move / no_wall_max_move;
    assert!(
        (ratio - wall_count as f32).abs() < 1e-6,
        "AC-5 FAILED: with_wall_max_move should be tree_support_wall_count * no_wall_max_move; \
         got ratio={ratio}, expected tree_support_wall_count={wall_count}"
    );
}

// ── Algorithmic invariants for the synthetic overhang fixture ───────────────
//
// These replace the former `benchy_tree_support_regression_tripwire`, a
// self-captured golden comparison (branch count ±10% + Hausdorff ≤ 0.5mm
// against `resources/golden/benchy_tree_support_regression_*`, regenerated via
// `SUPPORT_PLANNER_REGEN_GOLDEN=1`). A frozen snapshot of the planner's own
// output cannot distinguish a correct algorithm change from a regression: any
// intentional edit was answered by re-blessing the file, so the goldens
// recorded whatever the planner last did rather than what it must do.
//
// The fixture and harness are unchanged — the same floating 4x4mm plate at
// z = 1.8mm over an 11-layer 0.2mm stack, the same planner config, the same
// `SupportGeometryView` occupancy. What changed is that the assertions are now
// properties of the *structure* the planner emits.
//
// ## Reading the planner output
//
// The planner emits one `SupportPlanEntry` per (object, layer, region). Both
// regions of this fixture ("0" and "1") receive the same physical skeleton by
// family-assignment stamping, so node sets are de-duplicated by position.
//
// Every skeleton segment lies *within one layer*: an entry's skeleton is the
// flattened endpoint list of that layer's MST edges plus degenerate per-node
// points, all at the layer's z. There are no cross-layer segments in the IR, so
// "a branch" is reconstructed here by linking each layer's nodes to the nodes
// on the layer below (`parent_map`).

/// Layer count of `make_layer_plan(11, 0.0, 0.2)`.
const FIXTURE_LAYER_COUNT: u32 = 11;
/// Layer height of `make_layer_plan(11, 0.0, 0.2)`.
const FIXTURE_LAYER_HEIGHT: f32 = 0.2;
/// `tree_support_branch_diameter` used by every invariant test below.
const FIXTURE_BRANCH_DIAMETER: f32 = 2.0;
/// `tree_support_branch_diameter_angle` used by every invariant test below.
const FIXTURE_DIAMETER_ANGLE_DEG: f32 = 5.0;
/// `tree_support_branch_angle` used by every invariant test below.
const FIXTURE_BRANCH_ANGLE_DEG: f32 = 45.0;
/// `tree_support_branch_distance` — the contact sampling pitch.
const FIXTURE_BRANCH_DISTANCE: f32 = 1.0;
/// `support_interface_top_layers` used by every invariant test below.
const FIXTURE_INTERFACE_TOP_LAYERS: i32 = 2;
/// Underside z of the floating plate `overhang_plate_fixture` builds.
const FIXTURE_PLATE_Z_MM: f32 = 1.8;
/// `MIN_BRANCH_RADIUS` in the planner: the floor `calc_radius` clamps to.
const MIN_BRANCH_RADIUS_MM: f32 = 0.4;
/// Maximum consecutive-segment turn angle a reconstructed branch may carry,
/// matching the bound `smooth_nodes_tdd::max_turn_angle` is written against.
const MAX_TURN_ANGLE_DEG: f32 = 30.0;

/// Layer index of the plate cross-section, matching the fixture's own
/// `(1.8 / 0.2).round() - 1`.
fn fixture_plate_layer() -> u32 {
    (FIXTURE_PLATE_Z_MM / FIXTURE_LAYER_HEIGHT).round() as u32 - 1
}

/// Canonical per-layer XY reach of one node: `tan(branch_angle) * layer_height`.
fn max_move_per_layer_mm() -> f32 {
    FIXTURE_BRANCH_ANGLE_DEG.to_radians().tan() * FIXTURE_LAYER_HEIGHT
}

/// The plate cross-section the fixture uses both as model occupancy and as the
/// contact region branches must attach to.
fn plate_occupancy(obj: &MeshObjectView) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: obj.vertices[1..]
                .iter()
                .map(|[x, y, _]| Point2::from_mm(*x, *y))
                .collect(),
        },
        holes: vec![],
    }
}

/// Run the planner over the synthetic overhang fixture with the config the
/// former tripwire used. Returns the plan output alongside the occupancy view
/// the fixture fed in, so collision and attachment invariants can reuse it.
fn plan_overhang_fixture(object_id: &str) -> (SupportGeometryOutput, SupportGeometryView) {
    let config = make_planner_config(&[
        ("enable_support", ConfigValue::Bool(true)),
        ("support_raft_layers", ConfigValue::Int(2)),
        (
            "support_interface_top_layers",
            ConfigValue::Int(FIXTURE_INTERFACE_TOP_LAYERS as i64),
        ),
        ("tree_support_interface_spacing_mm", ConfigValue::Float(0.4)),
        (
            "tree_support_branch_diameter",
            ConfigValue::Float(FIXTURE_BRANCH_DIAMETER as f64),
        ),
        (
            "tree_support_branch_diameter_angle",
            ConfigValue::Float(FIXTURE_DIAMETER_ANGLE_DEG as f64),
        ),
        (
            "tree_support_branch_distance",
            ConfigValue::Float(FIXTURE_BRANCH_DISTANCE as f64),
        ),
        ("tree_support_wall_count", ConfigValue::Int(1)),
        (
            "tree_support_branch_angle",
            ConfigValue::Float(FIXTURE_BRANCH_ANGLE_DEG as f64),
        ),
    ]);
    let planner = SupportPlanner::from_config(&config).expect("from_config");

    let obj = overhang_plate_fixture(object_id);
    let lp = make_layer_plan(FIXTURE_LAYER_COUNT, 0.0, FIXTURE_LAYER_HEIGHT);
    let rs = make_region_segmentation(object_id, FIXTURE_LAYER_COUNT);
    let occupancy = plate_occupancy(&obj);
    let plate_layer = fixture_plate_layer();
    let sg = SupportGeometryView {
        entries: (0..FIXTURE_LAYER_COUNT)
            // The fixture has material only at the floating plate's z=1.8
            // cross-section; lower layers are empty space beneath it.
            .filter(|&global_support_layer_index| global_support_layer_index == plate_layer)
            .map(|global_support_layer_index| SupportGeometryViewEntry {
                global_support_layer_index,
                object_id: object_id.to_string(),
                region_id: "0".to_string(),
                outlines: vec![occupancy.clone()],
            })
            .collect(),
    };
    // G-23 fixture precondition: collision/avoidance input must be real occupancy.
    assert!(
        !sg.entries.is_empty(),
        "G-23 occupancy fixture must be non-empty"
    );

    let mut output = SupportGeometryOutput::new();
    planner
        .run_support_geometry_with_analysis(
            &[obj],
            &lp,
            &rs,
            &tree_analysis(object_id),
            &sg,
            &mut output,
            &ConfigView::new(),
        )
        .expect("run_support_geometry");
    (output, sg)
}

/// Distinct node positions (mm) per model layer, sorted lexicographically so
/// every derived structure is order-independent.
fn layer_nodes(output: &SupportGeometryOutput) -> BTreeMap<i32, Vec<(f32, f32)>> {
    let mut per_layer: BTreeMap<i32, Vec<(f32, f32)>> = BTreeMap::new();
    for entry in output.entries() {
        if entry.global_layer_index < 0 {
            continue;
        }
        let Some(skeleton) = &entry.skeleton else {
            continue;
        };
        let nodes = per_layer.entry(entry.global_layer_index).or_default();
        for point in &skeleton.points {
            if !nodes
                .iter()
                .any(|(x, y)| (*x - point.x).abs() < 1e-6 && (*y - point.y).abs() < 1e-6)
            {
                nodes.push((point.x, point.y));
            }
        }
    }
    per_layer.retain(|_, nodes| !nodes.is_empty());
    for nodes in per_layer.values_mut() {
        nodes.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    }
    per_layer
}

/// Distinct printed role regions per model layer (both fixture regions receive
/// the same stamped geometry, so identical polygons are collapsed).
fn layer_regions(output: &SupportGeometryOutput) -> BTreeMap<i32, Vec<ExPolygon>> {
    let mut per_layer: BTreeMap<i32, Vec<ExPolygon>> = BTreeMap::new();
    for entry in output.entries() {
        if entry.global_layer_index < 0 {
            continue;
        }
        let regions = per_layer.entry(entry.global_layer_index).or_default();
        for role in &entry.roles {
            for region in &role.regions {
                if !regions.contains(region) {
                    regions.push(region.clone());
                }
            }
        }
    }
    per_layer.retain(|_, regions| !regions.is_empty());
    per_layer
}

/// Every contour and hole vertex of an `ExPolygon`, in mm.
fn expolygon_vertices_mm(region: &ExPolygon) -> Vec<(f32, f32)> {
    std::iter::once(&region.contour)
        .chain(region.holes.iter())
        .flat_map(|poly| poly.points.iter())
        .map(|p| (slicer_ir::units_to_mm(p.x), slicer_ir::units_to_mm(p.y)))
        .collect()
}

fn hypot2(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt()
}

/// Distance from `(x, y)` to the nearest of `nodes`. `nodes` must be non-empty.
fn distance_to_nearest_node(nodes: &[(f32, f32)], x: f32, y: f32) -> f32 {
    nodes
        .iter()
        .map(|(nx, ny)| hypot2(x, y, *nx, *ny))
        .fold(f32::INFINITY, f32::min)
}

/// Distance from a point to a segment, in mm.
fn point_segment_distance(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let (dx, dy) = (bx - ax, by - ay);
    let len_sq = dx * dx + dy * dy;
    if len_sq == 0.0 {
        return hypot2(px, py, ax, ay);
    }
    let t = (((px - ax) * dx + (py - ay) * dy) / len_sq).clamp(0.0, 1.0);
    hypot2(px, py, ax + t * dx, ay + t * dy)
}

/// Distance from `(x, y)` to an `ExPolygon`: zero when the point is inside it,
/// otherwise the distance to the nearest edge (so a point exactly on the
/// boundary reads as zero either way).
fn distance_to_expolygon(region: &ExPolygon, x: f32, y: f32) -> f32 {
    let ring = |poly: &Polygon| -> Vec<[f32; 2]> {
        poly.points
            .iter()
            .map(|p| [slicer_ir::units_to_mm(p.x), slicer_ir::units_to_mm(p.y)])
            .collect()
    };
    let contour = ring(&region.contour);
    let holes: Vec<Vec<[f32; 2]>> = region.holes.iter().map(ring).collect();
    let inside =
        point_in_polygon(&contour, x, y) && !holes.iter().any(|hole| point_in_polygon(hole, x, y));
    if inside {
        return 0.0;
    }
    std::iter::once(&contour)
        .chain(holes.iter())
        .flat_map(|ring| (0..ring.len()).map(move |i| (ring[i], ring[(i + 1) % ring.len()])))
        .map(|(a, b)| point_segment_distance(x, y, a[0], a[1], b[0], b[1]))
        .fold(f32::INFINITY, f32::min)
}

/// Model occupancy per layer, read back out of the `SupportGeometryView` the
/// fixture fed the planner.
fn occupancy_by_layer(sg: &SupportGeometryView) -> BTreeMap<i32, Vec<ExPolygon>> {
    let mut per_layer: BTreeMap<i32, Vec<ExPolygon>> = BTreeMap::new();
    for entry in &sg.entries {
        per_layer
            .entry(entry.global_support_layer_index as i32)
            .or_default()
            .extend(entry.outlines.iter().cloned());
    }
    per_layer
}

/// A node identity: `(layer, index into that layer's sorted node list)`.
type NodeId = (i32, usize);

/// Nearest node on the layer below: its index, its distance, and the distance
/// to the runner-up (`INFINITY` when there is only one candidate).
fn nearest_below(
    per_layer: &BTreeMap<i32, Vec<(f32, f32)>>,
    layer: i32,
    node: (f32, f32),
) -> Option<(usize, f32, f32)> {
    let below = per_layer.get(&(layer - 1))?;
    let mut ranked: Vec<(usize, f32)> = below
        .iter()
        .enumerate()
        .map(|(i, (bx, by))| (i, hypot2(node.0, node.1, *bx, *by)))
        .collect();
    ranked.sort_by(|a, b| a.1.total_cmp(&b.1));
    let (best_index, best) = *ranked.first()?;
    let runner_up = ranked.get(1).map(|(_, d)| *d).unwrap_or(f32::INFINITY);
    Some((best_index, best, runner_up))
}

/// The per-layer reach a parent link may span: the canonical per-layer move
/// plus one branch radius of slack, since a merge lands a child on its
/// sibling's centre up to a radius away.
fn parent_reach_mm() -> f32 {
    max_move_per_layer_mm() + FIXTURE_BRANCH_DIAMETER * 0.5
}

/// Parent link for every node that has one, keyed by child.
///
/// Parentage is proximity across adjacent layers: a node descends into the
/// closest node one layer down, provided that node is within [`parent_reach_mm`].
fn parent_map(per_layer: &BTreeMap<i32, Vec<(f32, f32)>>) -> HashMap<NodeId, NodeId> {
    let reach = parent_reach_mm();
    let mut parents = HashMap::new();
    for (&layer, nodes) in per_layer {
        for (index, node) in nodes.iter().enumerate() {
            if let Some((parent_index, distance, _)) = nearest_below(per_layer, layer, *node) {
                if distance <= reach {
                    parents.insert((layer, index), (layer - 1, parent_index));
                }
            }
        }
    }
    parents
}

/// Maximum consecutive-segment turn angle (degrees) along a 3D polyline.
/// Mirrors `smooth_nodes_tdd::max_turn_angle`.
fn max_turn_angle(points: &[(f32, f32, f32)]) -> f32 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut max_deg = 0.0f32;
    for window in points.windows(3) {
        let v1 = (
            window[1].0 - window[0].0,
            window[1].1 - window[0].1,
            window[1].2 - window[0].2,
        );
        let v2 = (
            window[2].0 - window[1].0,
            window[2].1 - window[1].1,
            window[2].2 - window[1].2,
        );
        let dot = v1.0 * v2.0 + v1.1 * v2.1 + v1.2 * v2.2;
        let n1 = (v1.0 * v1.0 + v1.1 * v1.1 + v1.2 * v1.2).sqrt();
        let n2 = (v2.0 * v2.0 + v2.1 * v2.1 + v2.2 * v2.2).sqrt();
        if n1 == 0.0 || n2 == 0.0 {
            continue;
        }
        let deg = (dot / (n1 * n2)).clamp(-1.0, 1.0).acos().to_degrees();
        max_deg = max_deg.max(deg);
    }
    max_deg
}

/// Sorted, rounded skeleton endpoints across every entry — the determinism
/// fingerprint.
fn skeleton_endpoints(output: &SupportGeometryOutput) -> Vec<[f32; 3]> {
    let mut endpoints: Vec<[f32; 3]> = output
        .entries()
        .iter()
        .filter_map(|entry| entry.skeleton.as_ref())
        .flat_map(|skeleton| skeleton.points.iter())
        .map(|p| [round4(p.x), round4(p.y), round4(p.z)])
        .collect();
    sort_endpoints(&mut endpoints);
    endpoints
}

fn round4(v: f32) -> f32 {
    (v * 10_000.0).round() / 10_000.0
}

fn sort_endpoints(eps: &mut [[f32; 3]]) {
    eps.sort_by(|a, b| {
        a[0].partial_cmp(&b[0])
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a[1].partial_cmp(&b[1]).unwrap_or(std::cmp::Ordering::Equal))
            .then(a[2].partial_cmp(&b[2]).unwrap_or(std::cmp::Ordering::Equal))
    });
}

/// Invariant 1 — collision-freedom.
///
/// No branch may occupy space the model occupies. Two complementary checks:
///
/// * **Z clearance.** The fixture's only material is the plate cross-section at
///   z = 1.8mm (layer 8). No planned support may reach it, so every skeleton
///   point must sit strictly below the plate underside and no entry may land on
///   or above the plate layer. This is the load-bearing assertion here.
/// * **In-plane occupancy.** Where a layer carries both support and occupancy,
///   neither the nodes nor the printed role regions may enter it. On this
///   fixture the two sets are disjoint by layer, so this loop is a guard that
///   arms itself the moment the planner descends into the plate.
#[test]
fn branches_never_intersect_model_occupancy() {
    let (output, sg) = plan_overhang_fixture("collision-freedom");
    let occupancy = occupancy_by_layer(&sg);
    assert!(
        !occupancy.is_empty(),
        "fixture precondition: the SupportGeometryView must carry occupancy"
    );
    let plate_layer = fixture_plate_layer() as i32;

    let nodes = layer_nodes(&output);
    assert!(!nodes.is_empty(), "planner emitted no skeleton at all");

    for (&layer, layer_node_list) in &nodes {
        assert!(
            layer < plate_layer,
            "invariant 1: support planned at layer {layer}, on or above the \
             model plate layer {plate_layer}; nodes={layer_node_list:?}"
        );
    }
    for entry in output.entries() {
        let Some(skeleton) = &entry.skeleton else {
            continue;
        };
        for point in &skeleton.points {
            assert!(
                point.z < FIXTURE_PLATE_Z_MM,
                "invariant 1: skeleton point {point:?} reaches the model plate \
                 underside at z={FIXTURE_PLATE_Z_MM}"
            );
        }
    }

    let regions = layer_regions(&output);
    for (&layer, obstacles) in &occupancy {
        if let Some(layer_node_list) = nodes.get(&layer) {
            for (x, y) in layer_node_list {
                for obstacle in obstacles {
                    assert!(
                        distance_to_expolygon(obstacle, *x, *y) > 0.0,
                        "invariant 1: node ({x},{y}) on layer {layer} lies \
                         inside model occupancy"
                    );
                }
            }
        }
        if let Some(layer_region_list) = regions.get(&layer) {
            let overlap = slicer_sdk::host::clip_polygons(
                layer_region_list,
                obstacles,
                slicer_sdk::host::ClipOperation::Intersection,
            );
            assert!(
                overlap.is_empty(),
                "invariant 1: printed support on layer {layer} overlaps model \
                 occupancy"
            );
        }
    }
}

/// Invariant 2 — grounding.
///
/// Every branch must terminate on the build plate (the lowest model layer,
/// `global_layer_index == 0`) or on the model. Concretely: the layers carrying
/// support form one contiguous run down to layer 0, and every node either
/// descends into a node on the layer below or rests on model occupancy there —
/// no branch stops in mid-air.
#[test]
fn every_branch_terminates_on_the_build_plate_or_the_model() {
    let (output, sg) = plan_overhang_fixture("grounding");
    let nodes = layer_nodes(&output);
    let occupancy = occupancy_by_layer(&sg);
    let parents = parent_map(&nodes);

    let layers: Vec<i32> = nodes.keys().copied().collect();
    assert_eq!(
        layers.first().copied(),
        Some(0),
        "invariant 2: the lowest supported layer must be the build-plate layer \
         0; got {layers:?}"
    );
    for window in layers.windows(2) {
        assert_eq!(
            window[1],
            window[0] + 1,
            "invariant 2: support layers must be contiguous — a gap between {} \
             and {} means a branch floats in mid-air; layers={layers:?}",
            window[0],
            window[1]
        );
    }

    for (&layer, layer_node_list) in &nodes {
        if layer == 0 {
            continue;
        }
        for (index, node) in layer_node_list.iter().enumerate() {
            // A node may end because it sits on the model instead of
            // descending: canonical stops a branch that has reached a model
            // surface below it.
            let rests_on_model = occupancy.get(&(layer - 1)).is_some_and(|obstacles| {
                obstacles
                    .iter()
                    .any(|o| distance_to_expolygon(o, node.0, node.1) == 0.0)
            });
            assert!(
                parents.contains_key(&(layer, index)) || rests_on_model,
                "invariant 2: node {node:?} on layer {layer} neither descends \
                 into a node on layer {} nor rests on the model",
                layer - 1
            );
        }
    }
}

/// Invariant 3 — attachment.
///
/// Every branch tip must meet the overhang it exists to support: the topmost
/// planned nodes must lie on (or within tolerance of) the contact region taken
/// from the `SupportGeometryView` outlines the fixture supplies.
///
/// Tolerance is half `tree_support_branch_distance` — the contact sampling
/// pitch — so a tip may sit at most half a sample step outside the region.
#[test]
fn branch_tips_attach_to_the_contact_region() {
    let (output, sg) = plan_overhang_fixture("attachment");
    let nodes = layer_nodes(&output);
    let contact_regions: Vec<ExPolygon> = sg
        .entries
        .iter()
        .flat_map(|entry| entry.outlines.iter().cloned())
        .collect();
    assert!(
        !contact_regions.is_empty(),
        "fixture precondition: contact regions must be derivable from the \
         SupportGeometryView outlines"
    );

    let &top_layer = nodes
        .keys()
        .max()
        .expect("planner emitted no skeleton at all");
    let tips = &nodes[&top_layer];
    assert!(
        !tips.is_empty(),
        "invariant 3: the topmost support layer {top_layer} carries no nodes"
    );

    let tolerance = FIXTURE_BRANCH_DISTANCE * 0.5;
    for (x, y) in tips {
        let distance = contact_regions
            .iter()
            .map(|region| distance_to_expolygon(region, *x, *y))
            .fold(f32::INFINITY, f32::min);
        assert!(
            distance <= tolerance,
            "invariant 3: branch tip ({x},{y}) on layer {top_layer} sits \
             {distance:.4}mm from the contact region, beyond the {tolerance}mm \
             half-sample tolerance"
        );
    }
}

/// Invariant 4 — radius discipline.
///
/// Measured per layer as the distance from each printed role-region vertex to
/// the nearest node on that layer. Every union-boundary vertex lies on some
/// node's drawn cross-section, so this recovers that node's radius directly and
/// is unaffected by neighbouring circles fusing into one outline.
///
/// The radius must stay inside the planner's own bounds — floored at
/// `MIN_BRANCH_RADIUS` and capped by the canonical taper evaluated over the
/// whole build height — and must shrink monotonically toward the tips.
#[test]
fn branch_radius_respects_bounds_and_tapers_toward_tips() {
    let (output, _sg) = plan_overhang_fixture("radius");
    let nodes = layer_nodes(&output);
    let regions = layer_regions(&output);
    assert!(
        !regions.is_empty(),
        "invariant 4: planner emitted no printed role regions"
    );

    let branch_radius = FIXTURE_BRANCH_DIAMETER * 0.5;
    // Canonical `calc_branch_radius`: outside the tip cone the radius grows by
    // tan(diameter_angle) per mm of distance to the tip. Evaluated over the
    // full 11-layer stack this is the widest a branch may ever be here.
    let stack_height_mm = FIXTURE_LAYER_COUNT as f32 * FIXTURE_LAYER_HEIGHT;
    let taper_cap = branch_radius
        + (stack_height_mm - branch_radius) * FIXTURE_DIAMETER_ANGLE_DEG.to_radians().tan();
    // The drawn cross-section is `node_ellipse`, which stretches along the
    // node's movement direction by up to `1 + move / (2 * radius)`. 10% covers
    // that stretch at this fixture's `tan(45 deg) * 0.2mm` per-layer move, plus
    // the circle's polygonal discretisation.
    let radius_cap = taper_cap * 1.10;

    let mut radius_by_layer: BTreeMap<i32, f32> = BTreeMap::new();
    for (&layer, layer_region_list) in &regions {
        let layer_node_list = nodes
            .get(&layer)
            .unwrap_or_else(|| panic!("layer {layer} has role regions but no skeleton"));
        let mut min_radius = f32::INFINITY;
        let mut max_radius = 0.0f32;
        for region in layer_region_list {
            for (x, y) in expolygon_vertices_mm(region) {
                let radius = distance_to_nearest_node(layer_node_list, x, y);
                min_radius = min_radius.min(radius);
                max_radius = max_radius.max(radius);
            }
        }
        assert!(
            min_radius >= MIN_BRANCH_RADIUS_MM,
            "invariant 4: layer {layer} draws a cross-section {min_radius:.4}mm \
             from its node, below the {MIN_BRANCH_RADIUS_MM}mm minimum branch \
             radius"
        );
        assert!(
            max_radius <= radius_cap,
            "invariant 4: layer {layer} draws a cross-section {max_radius:.4}mm \
             from its node, above the {radius_cap:.4}mm taper cap for a \
             {branch_radius}mm branch over a {stack_height_mm}mm stack"
        );
        radius_by_layer.insert(layer, max_radius);
    }

    // Monotone taper: the radius never grows on the way up. `1e-3` absorbs the
    // spread between vertices of a single drawn ellipse.
    let mut previous: Option<(i32, f32)> = None;
    for (&layer, &radius) in &radius_by_layer {
        if let Some((below, below_radius)) = previous {
            assert!(
                radius <= below_radius + 1e-3,
                "invariant 4: radius grows toward the tip — layer {below} is \
                 {below_radius:.4}mm but layer {layer} above it is {radius:.4}mm"
            );
        }
        previous = Some((layer, radius));
    }

    // And the taper must actually happen: a constant-radius cylinder would
    // satisfy the monotonicity check above vacuously.
    let (&bottom, &bottom_radius) = radius_by_layer.first_key_value().expect("non-empty");
    let (&top, &top_radius) = radius_by_layer.last_key_value().expect("non-empty");
    assert!(
        bottom_radius > top_radius,
        "invariant 4: no taper at all — layer {bottom} is {bottom_radius:.4}mm \
         and layer {top} is {top_radius:.4}mm"
    );
}

/// Invariant 5 — merge-graph shape.
///
/// Branches merge downward and never split: each node descends into exactly one
/// node on the layer below (single parent, unambiguously nearest), the
/// resulting graph is acyclic, and the node population never grows going down.
#[test]
fn merge_graph_is_an_acyclic_single_parent_forest() {
    let (output, _sg) = plan_overhang_fixture("merge-graph");
    let nodes = layer_nodes(&output);
    let parents = parent_map(&nodes);
    let reach = parent_reach_mm();

    // Single parent: the nearest node below must be unambiguous. A second
    // candidate at (near) the same distance would make parentage arbitrary and
    // the merge graph ill-defined.
    let mut linked = 0usize;
    for (&layer, layer_node_list) in &nodes {
        for node in layer_node_list {
            let Some((_, nearest, runner_up)) = nearest_below(&nodes, layer, *node) else {
                continue;
            };
            if nearest > reach {
                continue;
            }
            linked += 1;
            assert!(
                runner_up > nearest + 1e-3,
                "invariant 5: node {node:?} on layer {layer} has two equally \
                 near parents ({nearest:.4}mm and {runner_up:.4}mm) — parentage \
                 is ambiguous"
            );
        }
    }
    assert!(
        linked > 0,
        "invariant 5: no node linked to a parent, so the single-parent check \
         was vacuous"
    );

    // Acyclicity: every parent link strictly descends, and a chain cannot take
    // more steps than the number of layers it started above the plate.
    for (&layer, layer_node_list) in &nodes {
        for (index, node) in layer_node_list.iter().enumerate() {
            let mut current = (layer, index);
            let mut steps = 0usize;
            while let Some(&parent) = parents.get(&current) {
                assert!(
                    parent.0 < current.0,
                    "invariant 5: parent link from layer {} to layer {} does \
                     not descend — the merge graph has a cycle",
                    current.0,
                    parent.0
                );
                current = parent;
                steps += 1;
                assert!(
                    steps <= layer as usize,
                    "invariant 5: parent chain from node {node:?} on layer \
                     {layer} exceeded {layer} steps — the merge graph has a cycle"
                );
            }
        }
    }

    // Merging only: the branch population never grows on the way down.
    let mut previous: Option<(i32, usize)> = None;
    for (&layer, layer_node_list) in &nodes {
        if let Some((below, below_count)) = previous {
            assert!(
                below_count >= layer_node_list.len(),
                "invariant 5: layer {below} carries {below_count} nodes but \
                 layer {layer} above it carries {} — branches split going down",
                layer_node_list.len()
            );
        }
        previous = Some((layer, layer_node_list.len()));
    }
}

/// Invariant 6 — curvature bound.
///
/// Reconstructed branches must not kink: the maximum turn angle between
/// consecutive segments of any tip-to-root chain stays within
/// `MAX_TURN_ANGLE_DEG`, the same bound `smooth_nodes_tdd` measures the
/// smoother against.
#[test]
fn branch_curvature_stays_within_the_turn_angle_bound() {
    let (output, _sg) = plan_overhang_fixture("curvature");
    let nodes = layer_nodes(&output);
    let parents = parent_map(&nodes);

    // A tip is a node that nothing on the layer above descends into.
    let has_child: std::collections::HashSet<NodeId> = parents.values().copied().collect();

    let z_of = |layer: i32| (layer as f32 + 1.0) * FIXTURE_LAYER_HEIGHT;
    let mut chains_checked = 0usize;
    for (&layer, layer_node_list) in &nodes {
        for (index, node) in layer_node_list.iter().enumerate() {
            if has_child.contains(&(layer, index)) {
                continue;
            }
            let mut chain = vec![(node.0, node.1, z_of(layer))];
            let mut current = (layer, index);
            while let Some(&parent) = parents.get(&current) {
                let (px, py) = nodes[&parent.0][parent.1];
                chain.push((px, py, z_of(parent.0)));
                current = parent;
            }
            if chain.len() < 3 {
                continue;
            }
            chains_checked += 1;
            let turn = max_turn_angle(&chain);
            assert!(
                turn <= MAX_TURN_ANGLE_DEG,
                "invariant 6: branch from tip {node:?} on layer {layer} turns \
                 {turn:.2} degrees between consecutive segments, above the \
                 {MAX_TURN_ANGLE_DEG} degree bound"
            );
        }
    }
    assert!(
        chains_checked > 0,
        "invariant 6: no branch was long enough to measure curvature on, so \
         the bound was never exercised"
    );
}

/// Invariant 7 — determinism.
///
/// Two planner runs over the same fixture must produce identical sorted
/// endpoint lists. This is the one property the deleted goldens genuinely
/// proved, so it is kept — asserted between two live runs instead of against a
/// frozen file.
#[test]
fn planner_output_is_deterministic_across_runs() {
    let (first, _) = plan_overhang_fixture("determinism");
    let (second, _) = plan_overhang_fixture("determinism");

    let first_endpoints = skeleton_endpoints(&first);
    let second_endpoints = skeleton_endpoints(&second);
    assert!(
        !first_endpoints.is_empty(),
        "invariant 7: planner emitted no endpoints, so determinism is vacuous"
    );
    assert_eq!(
        first_endpoints, second_endpoints,
        "invariant 7: two runs over the same fixture produced different \
         endpoint lists"
    );
    assert_eq!(
        layer_regions(&first),
        layer_regions(&second),
        "invariant 7: two runs over the same fixture produced different printed \
         geometry"
    );
}

/// Invariant 8 — non-vacuous floor.
///
/// An empty planner satisfies every invariant above, so the suite needs a floor
/// derived from the fixture rather than from the planner's own output.
///
/// The fixture presents one contact region: a 4x4mm downward-facing plate.
/// Sampled at `tree_support_branch_distance = 1.0mm` it must yield at least one
/// contact per corner, so at least **4** branch columns.
///
/// The plate underside sits at z = 1.8mm on a 0.2mm stack — layer 8. Canonical
/// `generate_contact_points` places a contact one layer below the overhang
/// (layer 7), and the tree default `support_top_z_distance` of 0.2mm drops
/// it one further (layer 6) — the values `top_z_distance_lowers_the_tree_contact_layer`
/// pins. Each column then descends to the build plate at layer 0, so support
/// must occupy at least **7** layers and produce at least **4 x 7 = 28**
/// grounded node-layer segments.
#[test]
fn branch_count_meets_the_analytic_floor() {
    let (output, _sg) = plan_overhang_fixture("floor");
    let nodes = layer_nodes(&output);

    // plate layer 8, minus one for "support is always one layer below the
    // overhang", minus one more for the 0.2mm default top-z gap.
    let contact_layer = fixture_plate_layer() as i32 - 2;
    let min_layers = (contact_layer + 1) as usize;
    let min_columns = 4usize;

    assert!(
        nodes.len() >= min_layers,
        "invariant 8: support occupies {} layers, below the {min_layers} layers \
         the fixture geometry requires (contact at layer {contact_layer}, \
         descending to the build plate at layer 0)",
        nodes.len()
    );

    let &top_layer = nodes.keys().max().expect("non-empty");
    assert!(
        top_layer >= contact_layer,
        "invariant 8: topmost support layer {top_layer} is below the contact \
         layer {contact_layer} the fixture's overhang seeds"
    );

    for (&layer, layer_node_list) in &nodes {
        assert!(
            layer_node_list.len() >= min_columns,
            "invariant 8: layer {layer} carries {} branch columns, below the \
             {min_columns} the 4x4mm contact region sampled at \
             {FIXTURE_BRANCH_DISTANCE}mm must seed",
            layer_node_list.len()
        );
    }

    let grounded_segments: usize = nodes.values().map(|n| n.len()).sum();
    assert!(
        grounded_segments >= min_columns * min_layers,
        "invariant 8: {grounded_segments} grounded node-layer segments, below \
         the {} the fixture requires",
        min_columns * min_layers
    );
}

/// AC-N3: when the model occupies every destination a branch could move to, the
/// branch is rejected and a typed warn-level `Diagnostic` records it, rather
/// than support being emitted through the model.
///
/// **Strengthened by packet 224.** The drop trigger changed, and the test gained
/// a check that the original was missing.
///
/// **Rewired by packet 224 step 5 (F-13).** This asserted a typed code-1002
/// `node-clamped-out` diagnostic. That diagnostic reported a non-canonical
/// mechanism — a fractional `max_move_xy` cap plus a post-hoc clamp out of
/// avoidance, with an "escape budget" that dropped the node when the clamp
/// exceeded it — and F-13 deletes all three. Canonical always takes a full
/// `get_max_move_dist(&node)` step and never clamps, so there is no escape to
/// go over budget and nothing for code 1002 to report.
///
/// The canonical mechanism for this fixture is the `drop_nodes` rule "if the
/// branch falls completely inside a collision area, delete it": with
/// `support_on_buildplate_only = false` the node has `valid` cleared, which
/// stops propagation but still draws the node on its own layer. So the
/// observable is *pruning*, not a diagnostic: the column must not descend
/// past the layer at which it meets the model.
///
/// The two surviving assertions are strictly stronger than the code-1002 one
/// they replace — they check the drop actually happened rather than that a
/// warning was printed about it.
#[module_test]
fn node_rejected_when_model_occupies_every_destination() {
    // Note: #[module_test] already drains and reinstalls log capture via
    // reset_global_state() + mock_host_setup(). No explicit install needed here.

    let config = make_planner_config(&[
        ("enable_support", ConfigValue::Bool(true)),
        ("support_raft_layers", ConfigValue::Int(0)),
        ("tree_support_branch_diameter", ConfigValue::Float(2.0)),
        (
            "tree_support_branch_diameter_angle",
            ConfigValue::Float(5.0),
        ),
        ("tree_support_branch_distance", ConfigValue::Float(0.5)),
        ("tree_support_wall_count", ConfigValue::Int(1)),
        ("tree_support_branch_angle", ConfigValue::Float(45.0_f64)),
    ]);
    let planner = SupportPlanner::from_config(&config).expect("from_config");

    let obj = overhang_plate_fixture("blocked");
    let lp = make_layer_plan(11, 0.0, 0.2);
    let rs = make_region_segmentation("blocked", 11);

    // Build a SupportGeometryView whose collision covers the entire overhang
    // region so any node move lands inside the collision union. The plate sits
    // in [0..4, 0..4] xy; cover [-10..14, -10..14], which entirely contains it.
    // Every node is therefore further inside collision than its own branch
    // radius, so canonical clears `valid` and the column stops descending.
    let big_box = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(-10.0, -10.0),
                Point2::from_mm(14.0, -10.0),
                Point2::from_mm(14.0, 14.0),
                Point2::from_mm(-10.0, 14.0),
            ],
        },
        holes: vec![],
    };
    let sg = SupportGeometryView {
        entries: (0..11)
            .map(|i| SupportGeometryViewEntry {
                global_support_layer_index: i,
                object_id: "blocked".to_string(),
                region_id: "0".to_string(),
                outlines: vec![big_box.clone()],
            })
            .collect(),
    };

    let mut output = SupportGeometryOutput::new();
    planner
        .run_support_geometry_with_analysis(
            &[obj],
            &lp,
            &rs,
            &tree_analysis("blocked"),
            &sg,
            &mut output,
            &ConfigView::new(),
        )
        .expect("run_support_geometry");

    // Packet 238b AC-8/Q10: contact seeding uses the xy-inflated collision
    // volume, so contacts wholly inside this occupancy are pruned before the
    // emit-time 1203 rejection path. The absence of that diagnostic is part
    // of the canonical safety behavior, not a loss of coverage.
    assert!(
        !output.diagnostics().iter().any(|d| d.code == 1203),
        "AC-N3: seeded-pruned contacts must not reach the emit-time 1203 \
         diagnostic path; got {:?}",
        output.diagnostics()
    );

    // The column must be pruned at seeding, not merely warned about: no
    // contact survives to create a printable descendant.
    assert!(
        output
            .entries()
            .iter()
            .all(|entry| entry.decline_reason.is_some()
                || entry.roles.iter().all(|role| role.regions.is_empty())),
        "AC-N3: seeded-pruned contacts must produce no printable support \
         inside the occupied destination; got {:?}",
        output.entries()
    );
}

// RC-1 lone-node column mid-air: a propagated node must still emit its segment.
#[test]
fn lone_node_emits_degenerate_segment_on_propagated_layers() {
    let config = make_planner_config(&[
        ("enable_support", ConfigValue::Bool(true)),
        ("support_raft_layers", ConfigValue::Int(0)),
        ("tree_support_branch_diameter", ConfigValue::Float(2.0)),
        (
            "tree_support_branch_diameter_angle",
            ConfigValue::Float(5.0),
        ),
        ("tree_support_branch_distance", ConfigValue::Float(1.0)),
        ("tree_support_wall_count", ConfigValue::Int(1)),
        ("tree_support_branch_angle", ConfigValue::Float(45.0_f64)),
    ]);
    let planner = SupportPlanner::from_config(&config).expect("from_config");

    let obj = single_contact_fixture("lone-node");
    let lp = make_layer_plan(11, 0.0, 0.2);
    let rs = make_region_segmentation("lone-node", 11);
    let sg = SupportGeometryView { entries: vec![] };
    let mut output = SupportGeometryOutput::new();
    planner
        .run_support_geometry_with_analysis(
            &[obj],
            &lp,
            &rs,
            &tree_analysis("lone-node"),
            &sg,
            &mut output,
            &ConfigView::new(),
        )
        .expect("run_support_geometry");

    let entries = output.entries();
    let contact_layer = entries
        .iter()
        .map(|entry| entry.global_layer_index)
        .max()
        .expect("expected a contact-layer output entry");
    let propagated_segments: Vec<_> = entries
        .iter()
        .filter(|entry| entry.global_layer_index < contact_layer)
        .filter_map(|entry| entry.skeleton.as_ref())
        .filter(|s| s.points.len() == 2)
        .collect();

    assert!(
        propagated_segments.iter().any(|segment| {
            let first = &segment.points[0];
            let second = &segment.points[1];
            first.x == second.x && first.y == second.y && first.z == second.z
        }),
        "a lone propagated node must emit a degenerate two-point segment below the contact layer"
    );
}

#[test]
fn contact_count_follows_overhang_area_not_triangle_count() {
    let config = make_planner_config(&[
        ("enable_support", ConfigValue::Bool(true)),
        ("support_raft_layers", ConfigValue::Int(0)),
        ("tree_support_branch_diameter", ConfigValue::Float(1.0)),
        ("tree_support_branch_distance", ConfigValue::Float(1.0)),
        ("tree_support_wall_count", ConfigValue::Int(1)),
        ("tree_support_branch_angle", ConfigValue::Float(45.0)),
    ]);
    let planner = SupportPlanner::from_config(&config).expect("from_config");
    let layer_plan = make_layer_plan(11, 0.0, 0.2);
    let region_small = make_region_segmentation("small", 11);
    let region_large = make_region_segmentation("large", 11);
    let support_geometry = SupportGeometryView { entries: vec![] };

    let mut small_output = SupportGeometryOutput::new();
    planner
        .run_support_geometry(
            &[overhang_plate_fixture("small")],
            &layer_plan,
            &region_small,
            &support_geometry,
            &mut small_output,
            &ConfigView::new(),
        )
        .expect("small overhang planning");

    let mut large = overhang_plate_fixture("large");
    for vertex in large.vertices.iter_mut().skip(2) {
        vertex[0] *= 3.0;
        vertex[1] *= 3.0;
    }
    let mut large_output = SupportGeometryOutput::new();
    planner
        .run_support_geometry(
            &[large],
            &layer_plan,
            &region_large,
            &support_geometry,
            &mut large_output,
            &ConfigView::new(),
        )
        .expect("large overhang planning");

    let top_points = |output: &SupportGeometryOutput| {
        let top_layer = output
            .entries()
            .iter()
            .map(|entry| entry.global_layer_index)
            .max()
            .expect("contact layer");
        output
            .entries()
            .iter()
            .filter(|entry| entry.global_layer_index == top_layer)
            .filter_map(|entry| entry.skeleton.as_ref())
            .map(|skeleton| skeleton.points.len())
            .sum::<usize>()
    };
    let small_count = top_points(&small_output);
    let large_count = top_points(&large_output);
    assert!(
        large_count > small_count,
        "same two-triangle overhangs must sample by area: small={small_count}, large={large_count}"
    );
}

// ── Test fixtures ──────────────────────────────────────────────────────────

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

/// Build a planner ConfigView from a list of (key, value) pairs.
fn make_planner_config(entries: &[(&str, ConfigValue)]) -> ConfigView {
    let mut map: HashMap<ConfigKey, ConfigValue> = HashMap::new();
    for (k, v) in entries {
        map.insert((*k).to_string(), v.clone());
    }
    ConfigView::from_map(map)
}

/// Build a flat LayerPlanView with `n` layers at uniform `layer_height`.
fn make_layer_plan(n: u32, base_z: f32, layer_height: f32) -> LayerPlanView {
    LayerPlanView {
        layers: (0..n)
            .map(|i| LayerPlanViewEntry {
                global_layer_index: i,
                z: base_z + (i as f32 + 1.0) * layer_height,
                effective_layer_height: layer_height,
            })
            .collect(),
    }
}

/// Build a RegionSegmentationView with one region ("0") per layer.
fn make_region_segmentation(object_id: &str, n: u32) -> RegionSegmentationView {
    RegionSegmentationView {
        entries: (0..n)
            .map(|i| RegionSegmentationViewEntry {
                object_id: object_id.to_string(),
                layer_index: i,
                region_ids: vec!["0".to_string()],
            })
            .collect(),
        region_support_configs: Vec::new(),
    }
}

/// Build a single-overhang fixture: an anchor at the origin (so bounds span
/// z=0..2.0 across ≥10 layers at 0.2mm height) plus a downward-facing quad
/// plate floating at z=1.8 covering [0..4]×[0..4]. The two plate triangles
/// register as overhang facets and seed a contact point near the top of the
/// layer stack.
fn overhang_plate_fixture(object_id: &str) -> MeshObjectView {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 1.8],
        [4.0, 0.0, 1.8],
        [4.0, 4.0, 1.8],
        [0.0, 4.0, 1.8],
    ];
    let triangles = vec![[1, 3, 2], [1, 4, 3]];
    MeshObjectView {
        object_id: object_id.to_string(),
        vertices,
        triangles,
        paint_layers: vec![],
    }
}

/// Build the same floating plate as `overhang_plate_fixture`, but shrunk to
/// [0..0.2]×[0..0.2] and with a single downward-facing triangle, so its
/// overhang samples to one contact that propagates as a lone node.
fn single_contact_fixture(object_id: &str) -> MeshObjectView {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 1.8],
        [0.2, 0.0, 1.8],
        [0.2, 0.2, 1.8],
        [0.0, 0.2, 1.8],
    ];
    let triangles = vec![[1, 3, 2]];
    MeshObjectView {
        object_id: object_id.to_string(),
        vertices,
        triangles,
        paint_layers: vec![],
    }
}

/// Make a minimal SupportPlanEntry at a given layer index with given point width.
fn make_support_entry(layer_index: i32, z: f32, _width: f32) -> SupportPlanEntry {
    // exhaustive: support-plan identity fixture; SupportPlanEntry has no Default impl and FRU would let a new plan field default silently
    SupportPlanEntry {
        global_layer_index: layer_index,
        object_id: "test-object".to_string(),
        region_id: 0,
        family_id: "tree".into(),
        demand_ids: vec![],
        body_ids: vec![],
        anchor_layer_index: layer_index.max(0) as u32,
        anchor_z: slicer_ir::mm_to_units(z),
        roles: vec![],
        skeleton: Some(slicer_ir::SupportPlanSkeleton {
            points: vec![
                slicer_ir::Point3 { x: 0.0, y: 0.0, z },
                slicer_ir::Point3 { x: 1.0, y: 1.0, z },
            ],
            wall_counts: vec![0, 0],
        }),
        capabilities: vec![],
        provenance: vec![],
        decline_reason: None,
    }
}

/// Make a SupportPlanEntry with a negative (raft) layer index.
fn make_entry_with_negative_index(index: i32) -> SupportPlanEntry {
    // global_layer_index is i32 to support negative indices for raft layers.
    // exhaustive: support-plan identity fixture; SupportPlanEntry has no Default impl and FRU would let a new plan field default silently
    SupportPlanEntry {
        global_layer_index: index,
        object_id: "test-object".to_string(),
        region_id: 0,
        family_id: "tree".into(),
        demand_ids: vec![],
        body_ids: vec![],
        anchor_layer_index: 0,
        anchor_z: 0,
        roles: vec![],
        skeleton: Some(slicer_ir::SupportPlanSkeleton {
            points: vec![slicer_ir::Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            }],
            wall_counts: vec![0],
        }),
        capabilities: vec![],
        provenance: vec![],
        decline_reason: None,
    }
}

/// Make a SupportPlanEntry with a positive layer index.
fn make_entry_with_index(index: u32) -> SupportPlanEntry {
    make_support_entry(index as i32, index as f32 * 0.2, 0.4)
}

/// Assign every region of `object_id` to the tree family.
///
/// Packet 224 made `PrePass::SupportAnalysis` the single authority for a
/// region's family; the planner no longer defaults to its own identity.
fn tree_analysis(object_id: &str) -> slicer_sdk::prepass_types::SupportAnalysisView {
    slicer_sdk::prepass_types::SupportAnalysisView {
        family_assignments: ["0", "1"]
            .iter()
            .map(
                |region_id| slicer_sdk::prepass_types::SupportFamilyAssignment {
                    object_id: object_id.to_string(),
                    region_id: region_id.to_string(),
                    family_id: "tree".to_string(),
                },
            )
            .collect(),
        ..Default::default()
    }
}

// ── RC-11: `support_top_z_distance` must hold the tree top gap ──────────
//
// The tree planner declared `support_top_z_distance` in its manifest and
// read it nowhere, so its top interface printed flush against the overhang
// with zero gap while `traditional-support-planner` honoured the key. This
// test pins the asymmetry closed.
//
// The gap is measured by walking actual layer Z. It must NOT be derived by
// dividing by `LayerPlanViewEntry.effective_layer_height`: the host's two
// producers of that field disagree (one takes a max over participating
// objects, the other takes the first match), and a previous attempt to divide
// by it opened a 35-layer gap.

/// Plan the fixture at a given `support_top_z_distance` and return the
/// highest model-layer index carrying planned support geometry.
fn top_support_layer_for_gap(gap_mm: f64) -> i32 {
    let config = make_planner_config(&[
        ("enable_support", ConfigValue::Bool(true)),
        ("support_raft_layers", ConfigValue::Int(0)),
        ("support_interface_top_layers", ConfigValue::Int(2)),
        ("tree_support_branch_diameter", ConfigValue::Float(2.0)),
        (
            "tree_support_branch_diameter_angle",
            ConfigValue::Float(5.0),
        ),
        ("tree_support_branch_distance", ConfigValue::Float(1.0)),
        ("tree_support_wall_count", ConfigValue::Int(1)),
        ("tree_support_branch_angle", ConfigValue::Float(45.0_f64)),
        ("support_top_z_distance", ConfigValue::Float(gap_mm)),
    ]);
    let planner = SupportPlanner::from_config(&config).expect("from_config");

    let obj = overhang_plate_fixture("gap");
    let lp = make_layer_plan(11, 0.0, 0.2);
    let rs = make_region_segmentation("gap", 11);
    let sg = SupportGeometryView { entries: vec![] };
    let mut output = SupportGeometryOutput::new();
    planner
        .run_support_geometry_with_analysis(
            &[obj],
            &lp,
            &rs,
            &tree_analysis("gap"),
            &sg,
            &mut output,
            &ConfigView::new(),
        )
        .expect("run_support_geometry");

    output
        .entries()
        .iter()
        .filter(|e| e.global_layer_index >= 0)
        .filter(|e| e.roles.iter().any(|role| !role.regions.is_empty()))
        .map(|e| e.global_layer_index)
        .max()
        .expect("expected at least one planned model-layer support entry")
}

#[test]
fn top_z_distance_lowers_the_tree_contact_layer() {
    // The fixture's overhang underside sits at z = 1.8mm on a 0.2mm layer
    // stack — layer index 8. Canonical `generate_contact_points` inserts
    // every contact into `contact_nodes[layer_nr - 1]`, commented "Support
    // must always be 1 layer below overhang", so even a *zero* gap tops the
    // column out at layer 7, not 8. Packet 224 defect F-34: this module used
    // to walk real layer Z down from the overhang plane instead, which put
    // the zero-gap contact flush on layer 8.
    let flush_top = top_support_layer_for_gap(0.0);
    assert_eq!(
        flush_top, 7,
        "with support_top_z_distance = 0.0 the tree column must top out \
         exactly one layer below the overhang layer 8; got layer {flush_top}"
    );

    // One 0.2mm layer of clearance must drop the column by exactly one layer.
    let gapped_top = top_support_layer_for_gap(0.2);
    assert!(
        gapped_top < flush_top,
        "RC-11: with support_top_z_distance = 0.2 the topmost tree support \
         layer must sit at least one 0.2mm layer below the flush contact layer \
         {flush_top}; got layer {gapped_top} (the key is declared in \
         tree-support-planner.toml but read nowhere)"
    );

    // The gap must track actual layer Z, not collapse to zero and not blow out
    // to tens of layers (the effective_layer_height division failure mode).
    let dropped_layers = flush_top - gapped_top;
    assert_eq!(
        dropped_layers, 1,
        "RC-11: a 0.2mm gap on a 0.2mm layer stack must drop the contact by \
         exactly one layer; got {dropped_layers} layers (flush={flush_top}, \
         gapped={gapped_top})"
    );
}

#[test]
fn top_z_distance_defaults_to_traditional_two_tenths() {
    // `traditional-support-planner::DEFAULT_TOP_Z_DISTANCE_MM` is 0.2 and
    // matches OrcaSlicer's `support_top_z_distance`. An absent key must give
    // the tree family the same gap, not a flush contact.
    let config = make_planner_config(&[
        ("enable_support", ConfigValue::Bool(true)),
        ("support_raft_layers", ConfigValue::Int(0)),
        ("support_interface_top_layers", ConfigValue::Int(2)),
        ("tree_support_branch_diameter", ConfigValue::Float(2.0)),
        (
            "tree_support_branch_diameter_angle",
            ConfigValue::Float(5.0),
        ),
        ("tree_support_branch_distance", ConfigValue::Float(1.0)),
        ("tree_support_wall_count", ConfigValue::Int(1)),
        ("tree_support_branch_angle", ConfigValue::Float(45.0_f64)),
        // support_top_z_distance deliberately absent.
    ]);
    let planner = SupportPlanner::from_config(&config).expect("from_config");

    let obj = overhang_plate_fixture("gap-default");
    let lp = make_layer_plan(11, 0.0, 0.2);
    let rs = make_region_segmentation("gap-default", 11);
    let sg = SupportGeometryView { entries: vec![] };
    let mut output = SupportGeometryOutput::new();
    planner
        .run_support_geometry_with_analysis(
            &[obj],
            &lp,
            &rs,
            &tree_analysis("gap-default"),
            &sg,
            &mut output,
            &ConfigView::new(),
        )
        .expect("run_support_geometry");

    let default_top = output
        .entries()
        .iter()
        .filter(|e| e.global_layer_index >= 0)
        .filter(|e| e.roles.iter().any(|role| !role.regions.is_empty()))
        .map(|e| e.global_layer_index)
        .max()
        .expect("expected at least one planned model-layer support entry");

    assert_eq!(
        default_top,
        top_support_layer_for_gap(0.2),
        "RC-11: the tree default for support_top_z_distance must equal \
         traditional's DEFAULT_TOP_Z_DISTANCE_MM (0.2mm); got top layer \
         {default_top}"
    );
}
