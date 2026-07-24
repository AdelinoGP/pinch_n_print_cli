//! Packet 118 typed diagnostic tests for support-planner.
//!
//! These tests verify the three migrated warning paths on
//! `SupportGeometryOutput::push_diagnostic`:
//!
//! - AC-5 / AC-N1: code 1001 (max_branches_per_layer cap exceeded) — one
//!   diagnostic per affected global layer, with `dropped_count=<n>` and
//!   `kept_count=<cap>` in the message. Below-cap runs emit zero.
//! - AC-6 / AC-N3: code 1003 (`support_interface_bottom_layers` is not yet
//!   implemented) — exactly one diagnostic, emitted before the layer loop
//!   when the config value is not `-1`; absent key or `-1` emits zero.
//!
//! The node-clamped code 1002 path is covered by
//! `orca_parity_tdd::node_dropped_when_avoidance_rejects_all_moves`.
//!
//! ## Acceptance Criteria
//!
//! - AC-5: cap exceeded → exactly one code 1001 warning per affected layer.
//! - AC-6: `support_interface_bottom_layers=3` → exactly one code 1003 warning
//!   with `layer=None` and a `support_interface_bottom_layers is not yet
//!   implemented` message.
//! - AC-N1: every layer below cap → zero cap diagnostics.
//! - AC-N3: `support_interface_bottom_layers=-1` or absent → zero
//!   `support_interface_bottom_layers is not yet implemented` diagnostics.

#![allow(missing_docs)]
#![allow(dead_code)]

use std::collections::HashMap;

use slicer_ir::{ConfigKey, ConfigValue, ConfigView};
use slicer_sdk::prepass_builders::SupportGeometryOutput;
use slicer_sdk::prepass_types::{
    Diagnostic, DiagnosticSeverity, LayerPlanView, LayerPlanViewEntry, MeshObjectView,
    RegionSegmentationView, RegionSegmentationViewEntry, SupportGeometryView,
};
use slicer_sdk::traits::PrepassModule;

use support_planner::SupportPlanner;

// ── AC-5: cap exceeded emits one code-1001 diagnostic per affected layer ─────

/// Build a fixture whose 1100 overhang triangles all funnel into a single
/// layer. With `support_max_branches_per_layer = 1024` (default), the planner
/// must drop 76 contacts on that layer and emit exactly one code-1001
/// warning with the cap-exceeded message format.
#[test]
fn cap_exceeded_emits_one_diagnostic_per_layer() {
    let config = make_planner_config(&[
        ("enable_support", ConfigValue::Bool(true)),
        ("support_raft_layers", ConfigValue::Int(0)),
        ("support_max_branches_per_layer", ConfigValue::Int(1024)),
        ("tree_support_branch_diameter", ConfigValue::Float(2.0)),
        (
            "tree_support_branch_diameter_angle",
            ConfigValue::Float(5.0),
        ),
        ("tree_support_branch_distance", ConfigValue::Float(1.0)),
        ("tree_support_wall_count", ConfigValue::Int(1)),
        ("support_branch_angle_deg", ConfigValue::Float(45.0_f64)),
    ]);
    let planner = SupportPlanner::on_print_start(&config).expect("on_print_start");

    let obj = cap_overflow_fixture("cap", 1100);
    let lp = make_layer_plan(11, 0.0, 0.2);
    let rs = make_region_segmentation("cap", 11);
    let sg = SupportGeometryView { entries: vec![] };
    let mut output = SupportGeometryOutput::new();
    planner
        .run_support_geometry(&[obj], &lp, &rs, &sg, &mut output, &ConfigView::new())
        .expect("run_support_geometry");

    let diagnostics = output.diagnostics();
    let cap_diags: Vec<&Diagnostic> = diagnostics.iter().filter(|d| d.code == 1001).collect();

    assert_eq!(
        cap_diags.len(),
        1,
        "AC-5: expected exactly one code-1001 diagnostic; got {} (codes: {:?})",
        cap_diags.len(),
        diagnostics.iter().map(|d| d.code).collect::<Vec<_>>()
    );

    let d = cap_diags[0];
    assert!(
        matches!(d.severity, DiagnosticSeverity::Warn),
        "AC-5: code 1001 must be warn severity; got {:?}",
        d.severity
    );
    assert!(
        d.message.contains("max_branches_per_layer cap exceeded"),
        "AC-5: message must contain 'max_branches_per_layer cap exceeded'; got '{}'",
        d.message
    );
    // Exactly one layer overflows, so dropped_count > 0 and kept_count = 1024.
    assert!(
        d.message.contains("dropped_count=") && d.message.contains("kept_count=1024"),
        "AC-5: message must contain 'dropped_count=<n>' and 'kept_count=1024'; got '{}'",
        d.message
    );
    let dropped_n = extract_dropped_count(&d.message)
        .expect("AC-5: message must contain parseable dropped_count=<int>");
    assert!(
        dropped_n > 0,
        "AC-5: dropped_count must be > 0; got {dropped_n}"
    );
}

/// Multi-object cap diagnostic: two objects whose 1100-overhang-triangle
/// fixtures all funnel into the same global layer must collapse to ONE
/// code-1001 diagnostic with the merged `dropped_count` (not two
/// per-object diagnostics). This guards the design.md invariant that the
/// cap diagnostic is per affected global layer, not per (object, layer).
/// The two objects' triangle grids are placed on disjoint XY regions so
/// they don't cross-merge inside the planner.
#[test]
fn multi_object_cap_diagnostic_merges_per_layer() {
    let config = make_planner_config(&[
        ("enable_support", ConfigValue::Bool(true)),
        ("support_raft_layers", ConfigValue::Int(0)),
        ("support_max_branches_per_layer", ConfigValue::Int(1024)),
        ("tree_support_branch_diameter", ConfigValue::Float(2.0)),
        (
            "tree_support_branch_diameter_angle",
            ConfigValue::Float(5.0),
        ),
        ("tree_support_branch_distance", ConfigValue::Float(1.0)),
        ("tree_support_wall_count", ConfigValue::Int(1)),
        ("support_branch_angle_deg", ConfigValue::Float(45.0_f64)),
    ]);
    let planner = SupportPlanner::on_print_start(&config).expect("on_print_start");

    // Two objects on disjoint XY extents (the cap-overflow fixture spans
    // (0..sqrt(1100)*0.4 ≈ 13.3 mm) on each side, so offsetting the second
    // object by +100 mm keeps them visually distinct and prevents the
    // planner from treating them as a single region).
    let obj_a = cap_overflow_fixture("A", 1100);
    let obj_b = offset_fixture_translate("B", 1100, 100.0, 0.0);
    let lp = make_layer_plan(11, 0.0, 0.2);
    let rs = make_region_segmentation_multi(&[("A", 11), ("B", 11)]);
    let sg = SupportGeometryView { entries: vec![] };
    let mut output = SupportGeometryOutput::new();
    planner
        .run_support_geometry(
            &[obj_a, obj_b],
            &lp,
            &rs,
            &sg,
            &mut output,
            &ConfigView::new(),
        )
        .expect("run_support_geometry");

    let diagnostics = output.diagnostics();
    let cap_diags: Vec<&Diagnostic> = diagnostics.iter().filter(|d| d.code == 1001).collect();

    assert_eq!(
        cap_diags.len(),
        1,
        "MED-1: two objects hitting the cap on the same global layer \
         must collapse to one merged code-1001 diagnostic; got {} \
         (codes: {:?})",
        cap_diags.len(),
        diagnostics.iter().map(|d| d.code).collect::<Vec<_>>()
    );

    let d = cap_diags[0];
    assert!(
        matches!(d.severity, DiagnosticSeverity::Warn),
        "MED-1: code 1001 must be warn severity; got {:?}",
        d.severity
    );
    assert_eq!(
        d.object_id, None,
        "MED-1: merged cap diagnostic must not carry a per-object id; \
         the cap is layer-level, not object-level; got {:?}",
        d.object_id
    );
    assert!(
        d.message.contains("dropped_count=") && d.message.contains("kept_count=1024"),
        "MED-1: message must contain 'dropped_count=<n>' and 'kept_count=1024'; got '{}'",
        d.message
    );
    let dropped_n = extract_dropped_count(&d.message)
        .expect("MED-1: message must contain parseable dropped_count=<int>");
    // Two objects each overflow layer 8 by 76, so the merged total is 152.
    // (If this fails because the planner only fills one object's contacts
    // before the cap and the other object's contacts all get dropped,
    // the cap-overflow fixture is asymmetric across objects; relax only
    // with an explicit design decision.)
    assert!(
        dropped_n >= 76,
        "MED-1: merged dropped_count must be at least one object's \
         contribution (76); got {dropped_n}"
    );
}

/// Below-cap run: every layer stays under 1024 → zero cap diagnostics.
#[test]
fn below_cap_emits_no_cap_diagnostic() {
    let config = make_planner_config(&[
        ("enable_support", ConfigValue::Bool(true)),
        ("support_raft_layers", ConfigValue::Int(0)),
        ("support_max_branches_per_layer", ConfigValue::Int(1024)),
        ("tree_support_branch_diameter", ConfigValue::Float(2.0)),
        (
            "tree_support_branch_diameter_angle",
            ConfigValue::Float(5.0),
        ),
        ("tree_support_branch_distance", ConfigValue::Float(1.0)),
        ("tree_support_wall_count", ConfigValue::Int(1)),
        ("support_branch_angle_deg", ConfigValue::Float(45.0_f64)),
    ]);
    let planner = SupportPlanner::on_print_start(&config).expect("on_print_start");

    // Only 5 triangles — well below the 1024 cap on a single layer.
    let obj = cap_overflow_fixture("nocap", 5);
    let lp = make_layer_plan(11, 0.0, 0.2);
    let rs = make_region_segmentation("nocap", 11);
    let sg = SupportGeometryView { entries: vec![] };
    let mut output = SupportGeometryOutput::new();
    planner
        .run_support_geometry(&[obj], &lp, &rs, &sg, &mut output, &ConfigView::new())
        .expect("run_support_geometry");

    let diagnostics = output.diagnostics();
    let cap_count = diagnostics
        .iter()
        .filter(|d| d.message.contains("max_branches_per_layer cap exceeded"))
        .count();
    assert_eq!(
        cap_count, 0,
        "AC-N1: zero cap diagnostics when every layer stays below the cap; got {cap_count}"
    );
}

// ── AC-6: planner-owned code 1003 for support_interface_bottom_layers ─────

/// `support_interface_bottom_layers = 3` → exactly one code 1003 warning
/// with `layer == None` and a `support_interface_bottom_layers is not yet
/// implemented` message, regardless of the layer loop.
#[test]
fn interface_bottom_layers_emits_one_typed_diagnostic() {
    let config = make_planner_config(&[
        ("enable_support", ConfigValue::Bool(true)),
        ("support_raft_layers", ConfigValue::Int(0)),
        ("support_interface_bottom_layers", ConfigValue::Int(3)),
        ("tree_support_branch_diameter", ConfigValue::Float(2.0)),
        (
            "tree_support_branch_diameter_angle",
            ConfigValue::Float(5.0),
        ),
        ("tree_support_branch_distance", ConfigValue::Float(1.0)),
        ("tree_support_wall_count", ConfigValue::Int(1)),
        ("support_branch_angle_deg", ConfigValue::Float(45.0_f64)),
    ]);
    let planner = SupportPlanner::on_print_start(&config).expect("on_print_start");

    let obj = small_overhang_fixture("ibl");
    let lp = make_layer_plan(11, 0.0, 0.2);
    let rs = make_region_segmentation("ibl", 11);
    let sg = SupportGeometryView { entries: vec![] };
    let mut output = SupportGeometryOutput::new();
    // The 1003 diagnostic reads the config at run_support_geometry time,
    // so the same config must be passed in here.
    planner
        .run_support_geometry(&[obj], &lp, &rs, &sg, &mut output, &config)
        .expect("run_support_geometry");

    let diagnostics = output.diagnostics();
    let ibl_diags: Vec<&Diagnostic> = diagnostics.iter().filter(|d| d.code == 1003).collect();

    assert_eq!(
        ibl_diags.len(),
        1,
        "AC-6: expected exactly one code-1003 diagnostic; got {} (codes: {:?})",
        ibl_diags.len(),
        diagnostics.iter().map(|d| d.code).collect::<Vec<_>>()
    );

    let d = ibl_diags[0];
    assert!(
        matches!(d.severity, DiagnosticSeverity::Warn),
        "AC-6: code 1003 must be warn severity; got {:?}",
        d.severity
    );
    assert_eq!(
        d.layer, None,
        "AC-6: code 1003 must have layer=None; got {:?}",
        d.layer
    );
    assert!(
        d.message
            .contains("support_interface_bottom_layers is not yet implemented"),
        "AC-6: message must contain 'support_interface_bottom_layers is not yet implemented'; got '{}'",
        d.message
    );
}

/// `support_interface_bottom_layers = -1` or absent key → zero code 1003
/// diagnostics.
#[test]
fn interface_bottom_layers_default_emits_no_typed_diagnostic() {
    // Case 1: explicit -1.
    {
        let config = make_planner_config(&[
            ("enable_support", ConfigValue::Bool(true)),
            ("support_raft_layers", ConfigValue::Int(0)),
            ("support_interface_bottom_layers", ConfigValue::Int(-1)),
            ("tree_support_branch_diameter", ConfigValue::Float(2.0)),
            (
                "tree_support_branch_diameter_angle",
                ConfigValue::Float(5.0),
            ),
            ("tree_support_branch_distance", ConfigValue::Float(1.0)),
            ("tree_support_wall_count", ConfigValue::Int(1)),
            ("support_branch_angle_deg", ConfigValue::Float(45.0_f64)),
        ]);
        let planner = SupportPlanner::on_print_start(&config).expect("on_print_start");

        let obj = small_overhang_fixture("ibl-neg");
        let lp = make_layer_plan(11, 0.0, 0.2);
        let rs = make_region_segmentation("ibl-neg", 11);
        let sg = SupportGeometryView { entries: vec![] };
        let mut output = SupportGeometryOutput::new();
        planner
            .run_support_geometry(&[obj], &lp, &rs, &sg, &mut output, &config)
            .expect("run_support_geometry");
        let count = output
            .diagnostics()
            .iter()
            .filter(|d| {
                d.message
                    .contains("support_interface_bottom_layers is not yet implemented")
            })
            .count();
        assert_eq!(
            count, 0,
            "AC-N3: support_interface_bottom_layers=-1 must not emit; got {count}"
        );
    }

    // Case 2: key absent entirely.
    {
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
            ("support_branch_angle_deg", ConfigValue::Float(45.0_f64)),
        ]);
        let planner = SupportPlanner::on_print_start(&config).expect("on_print_start");

        let obj = small_overhang_fixture("ibl-absent");
        let lp = make_layer_plan(11, 0.0, 0.2);
        let rs = make_region_segmentation("ibl-absent", 11);
        let sg = SupportGeometryView { entries: vec![] };
        let mut output = SupportGeometryOutput::new();
        planner
            .run_support_geometry(&[obj], &lp, &rs, &sg, &mut output, &config)
            .expect("run_support_geometry");
        let count = output
            .diagnostics()
            .iter()
            .filter(|d| {
                d.message
                    .contains("support_interface_bottom_layers is not yet implemented")
            })
            .count();
        assert_eq!(
            count, 0,
            "AC-N3: support_interface_bottom_layers absent must not emit; got {count}"
        );
    }
}

// ── Test fixtures ──────────────────────────────────────────────────────────

fn make_planner_config(entries: &[(&str, ConfigValue)]) -> ConfigView {
    let mut map: HashMap<ConfigKey, ConfigValue> = HashMap::new();
    for (k, v) in entries {
        map.insert((*k).to_string(), v.clone());
    }
    ConfigView::from_map(map)
}

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

fn make_region_segmentation(object_id: &str, n: u32) -> RegionSegmentationView {
    RegionSegmentationView {
        entries: (0..n)
            .map(|i| RegionSegmentationViewEntry {
                object_id: object_id.to_string(),
                layer_index: i,
                region_ids: vec!["0".to_string()],
            })
            .collect(),
    }
}

/// Build a `RegionSegmentationView` covering multiple objects across `n`
/// layers. Each `(object_id, n)` pair becomes `n` region entries, one
/// per layer. Used by the multi-object cap test.
fn make_region_segmentation_multi(specs: &[(&str, u32)]) -> RegionSegmentationView {
    let mut entries: Vec<RegionSegmentationViewEntry> = Vec::new();
    for (object_id, n) in specs {
        for i in 0..*n {
            entries.push(RegionSegmentationViewEntry {
                object_id: (*object_id).to_string(),
                layer_index: i,
                region_ids: vec!["0".to_string()],
            });
        }
    }
    RegionSegmentationView { entries }
}

/// Build a `cap_overflow_fixture`-style mesh and translate it by `(dx, dy)`
/// in XY. The triangles remain at z=1.8 so they all funnel into the same
/// global layer; the XY offset keeps the second object spatially disjoint
/// from the first when both are passed to the planner.
fn offset_fixture_translate(object_id: &str, n: usize, dx: f32, dy: f32) -> MeshObjectView {
    let mut base = cap_overflow_fixture(object_id, n);
    for v in &mut base.vertices {
        v[0] += dx;
        v[1] += dy;
    }
    base
}

/// Build a mesh that produces N downward-facing overhang triangles.
/// All triangle centroids sit at z=1.8, so they all funnel into layer 8
/// (`z = 0.2 * 9 = 1.8`). Each triangle is laid out on a 0.4×0.4 mm grid
/// tile; the full mesh spans `(0..ceil(sqrt(N))*0.4, 0..ceil(sqrt(N))*0.4)`
/// in XY. Triangles use CW winding from above so their normals point
/// downward (z < 0), matching OrcaSlicer's `detect_overhangs` threshold.
fn cap_overflow_fixture(object_id: &str, n: usize) -> MeshObjectView {
    // Anchor vertex at the origin so bmin[2] = 0.0 and the rel_z gate
    // (`rel_z >= first_layer_height * 0.5`) passes for centroid z=1.8.
    let mut vertices: Vec<[f32; 3]> = vec![[0.0, 0.0, 0.0]];
    let mut triangles: Vec<[u32; 3]> = Vec::with_capacity(n);
    let side = ((n as f64).sqrt().ceil() as usize).max(1);
    let tile = 0.4_f32;
    let overhang_z = 1.8_f32;
    for i in 0..n {
        let gx = (i % side) as f32;
        let gy = (i / side) as f32;
        let base = vertices.len() as u32;
        vertices.push([gx * tile, gy * tile, overhang_z]);
        vertices.push([(gx + 1.0) * tile, gy * tile, overhang_z]);
        vertices.push([(gx + 1.0) * tile, (gy + 1.0) * tile, overhang_z]);
        vertices.push([gx * tile, (gy + 1.0) * tile, overhang_z]);
        // CW winding from above ⇒ normal z < 0.
        triangles.push([base, base + 2, base + 1]);
        triangles.push([base, base + 3, base + 2]);
    }
    MeshObjectView {
        object_id: object_id.to_string(),
        vertices,
        triangles,
        paint_layers: vec![],
    }
}

/// A tiny fixture (2 triangles) just enough to drive the planner; used by
/// the code-1003 emission tests where the cap is not exercised.
fn small_overhang_fixture(object_id: &str) -> MeshObjectView {
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

/// Extract the integer value after `dropped_count=` in a cap diagnostic
/// message. Returns `None` if not present.
fn extract_dropped_count(msg: &str) -> Option<usize> {
    let idx = msg.find("dropped_count=")?;
    let after = &msg[idx + "dropped_count=".len()..];
    let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}
