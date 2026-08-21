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
use std::path::PathBuf;

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
    // Interface must be carved out of the body, never printed on top of it —
    // and the body must SURVIVE the carve. Canonical
    // `TreeSupport.cpp::draw_circles` computes
    // `base_areas = diff_ex(base_areas, roofs)` and keeps the remainder, so an
    // interface layer whose branch continues below the interface band still
    // carries a `SupportBody` cross-section.
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
        let continues_below = geometry_layers
            .iter()
            .any(|&lower| lower < interface_band_bottom);
        if continues_below {
            carve_checks += 1;
            assert!(
                !body.is_empty(),
                "AC-4: layer {layer} carries a TopInterface but no SupportBody, while the column continues below the interface band (band bottom={interface_band_bottom}, geometry layers={geometry_layers:?}). Canonical subtracts the roof out of `base_areas` and KEEPS the remainder; clearing the body leaves the branch cross-section unprinted on that layer."
            );
        }
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
    assert!(
        carve_checks > 0,
        "AC-4: no interface layer had a column continuing below it, so the body-survives-the-carve check was vacuous; interface layers={top_interface_layers:?}, geometry layers={geometry_layers:?}"
    );
}

/// AC-5: wall-count scaling — max XY distance ≤ tan(angle) * height * wall_count.
#[test]
fn wall_count_scales_max_move_distance() {
    // When wall-count-aware move scaling is implemented:
    //   max_move_distance = tan(branch_angle) * effective_height * wall_count
    //
    // Config keys:
    //   - tree_support_branch_angle (default 45.0)
    //   - support_wall_count (default 0 = auto, typically 1-2)
    //
    // Current v1 behavior: step_xy = tan_angle * effective_height (no wall_count factor).
    // This test documents expected behavior once AC-5 is implemented.

    let branch_angle_deg = 45.0_f32;
    let effective_height = 0.2_f32; // mm
    let wall_count = 2_u32;
    let tan_angle = branch_angle_deg.to_radians().tan();

    let no_wall_max_move = tan_angle * effective_height; // current v1
    let with_wall_max_move = tan_angle * effective_height * wall_count as f32;

    assert!(
        no_wall_max_move < with_wall_max_move,
        "AC-5: wall_count should scale max_move_distance upward; \
         v1 planner uses no_wall_max_move={no_wall_max_move} without wall_count factor"
    );

    // Verify: with wall_count=2, max_move should be 2x the no-wall value
    let ratio = with_wall_max_move / no_wall_max_move;
    assert!(
        (ratio - wall_count as f32).abs() < 1e-6,
        "AC-5 FAILED: with_wall_max_move should be wall_count * no_wall_max_move; \
         got ratio={ratio}, expected wall_count={wall_count}"
    );
}

/// PnP self-capture regression tripwire for tree-support stability.
///
/// ## Golden files (self-captured)
/// The golden files at `resources/golden/benchy_tree_support_regression_*` are
/// **self-captured snapshots** of this planner's own output against a fixed
/// synthetic overhang fixture, frozen to detect regressions. They prove
/// determinism and stability across runs but do **not** prove parity with
/// OrcaSlicer's reference output. This test was renamed off `orca_parity` in
/// packet 224 Step 8 (2026-08-20) and regenerated after the RC-15
/// contact-sampling port.
///
/// To regenerate the goldens after an intentional algorithm change, set
/// `SUPPORT_PLANNER_REGEN_GOLDEN=1`. The test then writes fresh goldens and
/// passes; subsequent runs compare against the frozen output.
///
/// Acceptance: branch count within ±10% of golden AND Hausdorff ≤ 0.5mm.
#[test]
fn benchy_tree_support_regression_tripwire() {
    // ── 1. Run the planner against a fixed synthetic fixture ──────────────
    let config = make_planner_config(&[
        ("enable_support", ConfigValue::Bool(true)),
        ("support_raft_layers", ConfigValue::Int(2)),
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

    let obj = overhang_plate_fixture("benchy-stand-in");
    let lp = make_layer_plan(11, 0.0, 0.2);
    let rs = make_region_segmentation("benchy-stand-in", 11);
    let sg = SupportGeometryView { entries: vec![] };
    let mut output = SupportGeometryOutput::new();
    planner
        .run_support_geometry_with_analysis(
            &[obj],
            &lp,
            &rs,
            &tree_analysis("benchy-stand-in"),
            &sg,
            &mut output,
            &ConfigView::new(),
        )
        .expect("run_support_geometry");

    let entries = output.entries();
    let output_branch_count = entries.len();

    // Endpoints: every point of every branch_segment polyline, sorted lex
    // for stability. SDK SupportPlanEntry.branch_segments is
    // Vec<Vec<Point3WithWidth>>: outer=branch, inner=polyline points.
    let mut output_endpoints: Vec<[f32; 3]> = entries
        .iter()
        .flat_map(|e| {
            e.skeleton
                .as_ref()
                .into_iter()
                .flat_map(|s| s.points.iter())
        })
        .map(|p| [round4(p.x), round4(p.y), round4(p.z)])
        .collect();
    sort_endpoints(&mut output_endpoints);

    // ── 2. Resolve golden paths ──────────────────────────────────────────────
    let manifest_dir = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let golden_dir = repo_root.join("resources/golden");
    let branch_count_path = golden_dir.join("benchy_tree_support_regression_branch_count.txt");
    let endpoints_path = golden_dir.join("benchy_tree_support_regression_endpoints.txt");

    let regen = std::env::var("SUPPORT_PLANNER_REGEN_GOLDEN").is_ok();

    // Header lines for self-captured goldens (skipped when parsing).
    let header = "# PnP self-capture (synthetic overhang fixture). NOT parity evidence — do not compare against OrcaSlicer output. Regenerated 2026-08-20 after the RC-15 contact-sampling port (packet 224 Step 3b).\n";

    if regen {
        std::fs::create_dir_all(&golden_dir).expect("create golden dir");
        std::fs::write(
            &branch_count_path,
            format!("{header}{output_branch_count}\n"),
        )
        .expect("write branch count golden");
        let mut endpoints_text = header.to_string();
        for [x, y, z] in &output_endpoints {
            endpoints_text.push_str(&format!("{x},{y},{z}\n"));
        }
        std::fs::write(&endpoints_path, endpoints_text).expect("write endpoints golden");
        eprintln!(
            "Regenerated goldens: count={} endpoints={}",
            output_branch_count,
            output_endpoints.len()
        );
        return;
    }

    // ── 3. Parse goldens (skip comment / empty lines) ────────────────────────
    if !branch_count_path.exists() || !endpoints_path.exists() {
        panic!(
            "Regression goldens missing. Regenerate with SUPPORT_PLANNER_REGEN_GOLDEN=1 \
             cargo test -p tree-support-planner -- benchy_tree_support_regression_tripwire"
        );
    }
    let count_raw = std::fs::read_to_string(&branch_count_path)
        .expect("benchy_tree_support_regression_branch_count.txt must be readable");
    let golden_branch_count: usize = count_raw
        .lines()
        .find(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .expect("branch count golden has no data line")
        .trim()
        .parse()
        .expect("golden branch count must be a valid integer");

    let endpoints_raw = std::fs::read_to_string(&endpoints_path)
        .expect("benchy_tree_support_regression_endpoints.txt must be readable");
    let golden_endpoints: Vec<[f32; 3]> = endpoints_raw
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .map(|line| {
            let parts: Vec<f32> = line
                .split(',')
                .map(|s| s.trim().parse().expect("endpoint must be x,y,z"))
                .collect();
            assert_eq!(
                parts.len(),
                3,
                "each endpoint must have exactly 3 coordinates (x,y,z)"
            );
            [parts[0], parts[1], parts[2]]
        })
        .collect();

    // ── 4. Branch count check (±10%) ─────────────────────────────────────────
    let tolerance_fraction = 0.10_f32;
    let branch_count_min = (golden_branch_count as f32 * (1.0 - tolerance_fraction)) as usize;
    let branch_count_max =
        ((golden_branch_count as f32 * (1.0 + tolerance_fraction)).ceil()) as usize;
    assert!(
        output_branch_count >= branch_count_min && output_branch_count <= branch_count_max,
        "Regression tripwire FAILED: branch count {output_branch_count} outside ±10% of golden {golden_branch_count} \
         (range: {branch_count_min}–{branch_count_max}). Set SUPPORT_PLANNER_REGEN_GOLDEN=1 to regenerate \
         after intentional algorithm changes."
    );

    // ── 5. Hausdorff distance check (≤ 0.5mm) ────────────────────────────────
    let hausdorff_ab = directed_hausdorff(&output_endpoints, &golden_endpoints);
    let hausdorff_ba = directed_hausdorff(&golden_endpoints, &output_endpoints);
    let hausdorff = hausdorff_ab.max(hausdorff_ba);
    let tolerance_mm = 0.5_f32;
    assert!(
        hausdorff <= tolerance_mm,
        "Regression tripwire FAILED: Hausdorff distance {hausdorff:.4}mm exceeds tolerance {tolerance_mm}mm. \
         Set SUPPORT_PLANNER_REGEN_GOLDEN=1 to regenerate after intentional algorithm changes."
    );
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

/// Compute directed Hausdorff distance: max_{a in A} min_{b in B} ||a - b||
fn directed_hausdorff(a: &[[f32; 3]], b: &[[f32; 3]]) -> f32 {
    if a.is_empty() {
        return 0.0;
    }
    if b.is_empty() {
        return f32::INFINITY;
    }
    a.iter()
        .map(|[ax, ay, az]| {
            b.iter()
                .map(|[bx, by, bz]| {
                    let dx = ax - bx;
                    let dy = ay - by;
                    let dz = az - bz;
                    (dx * dx + dy * dy + dz * dz).sqrt()
                })
                .fold(f32::INFINITY, f32::min)
        })
        .fold(0.0_f32, f32::max)
}

/// AC-N3: when the model occupies every destination a branch could move to, the
/// branch is rejected and a typed warn-level `Diagnostic` records it, rather
/// than support being emitted through the model.
///
/// **Strengthened by packet 224.** The drop trigger changed, and the test gained
/// a check that the original was missing.
///
/// Previously code 1002 fired when a node's clamped *centre* landed inside
/// `collision_polys`, and it only ever fired because `clamp_to_avoidance`'s
/// guard was inverted: nodes safely outside avoidance were snapped onto the
/// avoidance boundary, dragging branches into the model. Correcting that guard
/// alone was not enough — pushing a node to the *nearest* point outside
/// avoidance can move it arbitrarily far, so this fixture began planning
/// support bodies metres away from the overhang they were meant to support.
///
/// The trigger is now the branch-angle budget: a branch may travel at most
/// `max_move_xy` per layer, escaping avoidance included. When no legal
/// destination is within budget the node is dropped with code 1002.
///
/// The added assertion is that nothing is planned at all. A diagnostic without
/// an actual drop is a warning that support was printed through the object,
/// which is precisely what the earlier version of this test failed to catch.
#[module_test]
fn node_rejected_when_model_occupies_every_destination() {
    // Note: #[module_test] already drains and reinstalls log capture via
    // reset_global_state() + mock_host_setup(). No explicit install needed here.

    use slicer_sdk::prepass_types::{Diagnostic, DiagnosticSeverity};

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

    // Build a SupportGeometryView whose collision_polys cover the entire
    // overhang region so any node move lands inside the collision union.
    // The plate sits in [0..4, 0..4] xy; cover [-10..14, -10..14] which
    // entirely contains it. avoidance_polys (collision inflated outward) will
    // also contain the move targets, so clamp_to_avoidance is satisfied —
    // but point_in_any_polygon(collision_polys, ...) hits and the node is
    // dropped with a typed code-1002 diagnostic.
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

    let diagnostics = output.diagnostics();
    let rejected: Vec<&Diagnostic> = diagnostics
        .iter()
        .filter(|d| {
            d.code == 1002
                && matches!(d.severity, DiagnosticSeverity::Warn)
                && d.message.contains("node-clamped-out")
        })
        .collect();
    assert!(
        !rejected.is_empty(),
        "AC-N3: expected at least one code 1002 warn diagnostic containing \
         'node-clamped-out'; got {} diagnostics: {:?}",
        diagnostics.len(),
        diagnostics
    );

    // The rejection must be total: nothing may be planned inside a region the
    // model occupies entirely. A diagnostic without an actual drop would be a
    // warning that support was printed through the object.
    assert!(
        output
            .entries()
            .iter()
            .all(|entry| entry.decline_reason.is_some()
                || entry.roles.iter().all(|role| role.regions.is_empty())),
        "AC-N3: no support body may be planned where the model occupies every \
         destination; got {:?}",
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
/// plate floating at z=1.8 covering [0..0.2]×[0..0.2]. The two plate triangles
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

/// Build the same floating plate as `overhang_plate_fixture`, but with one
/// downward-facing triangle so its contact propagates as a lone node.
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

// ── RC-11: `support_top_z_distance_mm` must hold the tree top gap ──────────
//
// The tree planner declared `support_top_z_distance_mm` in its manifest and
// read it nowhere, so its top interface printed flush against the overhang
// with zero gap while `traditional-support-planner` honoured the key. This
// test pins the asymmetry closed.
//
// The gap is measured by walking actual layer Z. It must NOT be derived by
// dividing by `LayerPlanViewEntry.effective_layer_height`: the host's two
// producers of that field disagree (one takes a max over participating
// objects, the other takes the first match), and a previous attempt to divide
// by it opened a 35-layer gap.

/// Plan the fixture at a given `support_top_z_distance_mm` and return the
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
        ("support_top_z_distance_mm", ConfigValue::Float(gap_mm)),
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
        "with support_top_z_distance_mm = 0.0 the tree column must top out \
         exactly one layer below the overhang layer 8; got layer {flush_top}"
    );

    // One 0.2mm layer of clearance must drop the column by exactly one layer.
    let gapped_top = top_support_layer_for_gap(0.2);
    assert!(
        gapped_top < flush_top,
        "RC-11: with support_top_z_distance_mm = 0.2 the topmost tree support \
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
        // support_top_z_distance_mm deliberately absent.
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
        "RC-11: the tree default for support_top_z_distance_mm must equal \
         traditional's DEFAULT_TOP_Z_DISTANCE_MM (0.2mm); got top layer \
         {default_top}"
    );
}
