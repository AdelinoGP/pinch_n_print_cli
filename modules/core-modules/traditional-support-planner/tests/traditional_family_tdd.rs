//! Contract tests for the traditional support family seam.

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{ConfigKey, ConfigValue, ConfigView};
use slicer_ir::{
    ExPolygon, IndexedTriangleSet, MeshIR, ObjectMesh, Point2, Point3, Polygon,
    SupportPlanDeclineReason, SupportPlanEntry, SupportPlanIR, SupportPlanRole,
    SupportPlanRoleRegion, Transform3d,
};
use slicer_sdk::prepass_builders::SupportGeometryOutput;
use slicer_sdk::prepass_types::{
    LayerPlanView, LayerPlanViewEntry, MeshObjectView, RegionSegmentationView,
    RegionSegmentationViewEntry, SupportAnalysisCandidate, SupportAnalysisGeometryEntry,
    SupportAnalysisView, SupportFamilyAssignment, SupportGeometryView,
};
use slicer_sdk::traits::PrepassModule;
use slicer_wasm_host::exact_z_query::ExactZQueryService;
use slicer_wasm_host::support_aggregation::{
    aggregate_declined_support_plans, aggregate_support_plan_irs_with_diagnostics,
};
use traditional_support_planner::SupportPlanner;

fn contact_region() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(4.0, 0.0),
                Point2::from_mm(4.0, 4.0),
                Point2::from_mm(0.0, 4.0),
            ],
        },
        holes: vec![],
    }
}

fn obstacle_region() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(10.0, 0.0),
                Point2::from_mm(14.0, 0.0),
                Point2::from_mm(14.0, 4.0),
                Point2::from_mm(10.0, 4.0),
            ],
        },
        holes: vec![],
    }
}

fn validation_mesh() -> MeshIR {
    MeshIR {
        objects: vec![ObjectMesh {
            id: "object-a".into(),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    Point3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 10.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 10.0,
                        y: 10.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 0.0,
                        y: 10.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 0.0,
                        y: 0.0,
                        z: 10.0,
                    },
                    Point3 {
                        x: 10.0,
                        y: 0.0,
                        z: 10.0,
                    },
                    Point3 {
                        x: 10.0,
                        y: 10.0,
                        z: 10.0,
                    },
                    Point3 {
                        x: 0.0,
                        y: 10.0,
                        z: 10.0,
                    },
                ],
                indices: vec![
                    0, 1, 2, 0, 2, 3, 4, 6, 5, 4, 7, 6, 0, 4, 5, 0, 5, 1, 1, 5, 6, 1, 6, 2, 2, 6,
                    7, 2, 7, 3, 3, 7, 4, 3, 4, 0,
                ],
            },
            transform: Transform3d {
                matrix: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
            },
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn planner_config_with(
    enabled: bool,
    top_distance_mm: f32,
    support_layer_height_mm: f32,
) -> ConfigView {
    let mut values = HashMap::<ConfigKey, ConfigValue>::new();
    values.insert("enable_support".into(), ConfigValue::Bool(enabled));
    values.insert(
        "support_family".into(),
        ConfigValue::String("traditional".into()),
    );
    values.insert("support_interface_top_layers".into(), ConfigValue::Int(2));
    values.insert(
        "support_interface_bottom_layers".into(),
        ConfigValue::Int(1),
    );
    values.insert(
        "support_base_pattern".into(),
        ConfigValue::String("rectilinear".into()),
    );
    values.insert(
        "support_top_z_distance_mm".into(),
        ConfigValue::Float(top_distance_mm.into()),
    );
    values.insert(
        "support_layer_height_mm".into(),
        ConfigValue::Float(support_layer_height_mm.into()),
    );
    ConfigView::from_map(values)
}

fn planner_config(enabled: bool) -> ConfigView {
    planner_config_with(enabled, 0.0, 0.0)
}

fn layer_plan() -> LayerPlanView {
    LayerPlanView {
        layers: (0..10)
            .map(|i| LayerPlanViewEntry {
                global_layer_index: i,
                z: (i as f32 + 1.0) * 0.2,
                effective_layer_height: 0.2,
            })
            .collect(),
    }
}

fn regions(object_id: &str) -> RegionSegmentationView {
    RegionSegmentationView {
        entries: (0..10)
            .map(|layer_index| RegionSegmentationViewEntry {
                object_id: object_id.into(),
                layer_index,
                region_ids: vec!["0".into()],
            })
            .collect(),
        region_support_configs: vec![],
    }
}

// `minimal_object` (an empty mesh: no vertices, no triangles) was removed by
// packet 224. Its only remaining caller relied on the planner's mesh-facet
// contact derivation declining any mesh with no downward facets — a code path
// that no longer exists, since contact detection moved to
// `PrePass::SupportAnalysis`. An empty-mesh fixture can no longer express
// anything about this planner's behaviour.

fn overhang_object(object_id: &str) -> MeshObjectView {
    MeshObjectView {
        object_id: object_id.into(),
        vertices: vec![[0.0, 0.0, 1.8], [4.0, 0.0, 1.8], [4.0, 4.0, 1.8]],
        // Winding gives the facet a downward-facing normal.
        triangles: vec![[0, 2, 1]],
        paint_layers: vec![],
    }
}

fn traditional_assignment(object_id: &str) -> SupportFamilyAssignment {
    SupportFamilyAssignment {
        object_id: object_id.into(),
        region_id: "0".into(),
        family_id: "traditional".into(),
    }
}

fn run_planner_with_analysis(
    enabled: bool,
    object: MeshObjectView,
    analysis: SupportAnalysisView,
) -> SupportGeometryOutput {
    let planner = SupportPlanner::from_config(&planner_config(enabled)).expect("from_config");
    let mut output = SupportGeometryOutput::new();
    planner
        .run_support_geometry_with_analysis(
            &[object.clone()],
            &layer_plan(),
            &regions(&object.object_id),
            &analysis,
            &SupportGeometryView::default(),
            &mut output,
            &ConfigView::new(),
        )
        .expect("run_support_geometry_with_analysis");
    output
}

fn run_planner_with_config(
    config: ConfigView,
    object: MeshObjectView,
    analysis: SupportAnalysisView,
) -> SupportGeometryOutput {
    let planner = SupportPlanner::from_config(&config).expect("from_config");
    let mut output = SupportGeometryOutput::new();
    planner
        .run_support_geometry_with_analysis(
            &[object.clone()],
            &layer_plan(),
            &regions(&object.object_id),
            &analysis,
            &SupportGeometryView::default(),
            &mut output,
            &ConfigView::new(),
        )
        .expect("run_support_geometry_with_analysis");
    output
}

#[test]
fn contact_area_planning() {
    let object = overhang_object("contact");
    let analysis = SupportAnalysisView {
        candidates: vec![SupportAnalysisCandidate {
            id: 1,
            object_id: "contact".into(),
            region_id: "0".into(),
            global_layer_index: 8,
            z_units: slicer_ir::mm_to_units(1.8),
            geometry: vec![contact_region()],
            ..Default::default()
        }],
        family_assignments: vec![traditional_assignment("contact")],
        ..Default::default()
    };
    let output = run_planner_with_analysis(true, object, analysis);
    assert!(
        !output.entries().is_empty(),
        "traditional contact must produce plan entries"
    );
    let top_geometry_layer = output
        .entries()
        .iter()
        .filter(|entry| entry.roles.iter().any(|role| !role.regions.is_empty()))
        .map(|entry| entry.global_layer_index)
        .max()
        .expect("planner must emit geometry on at least one layer");
    // Canonical `generate_interface_layers` (`SupportCommon.cpp`) builds the
    // interface as `intersection(intermediate_layer.polygons, <projected top
    // contacts>)` and then does `intermediate_layer.polygons =
    // diff(intermediate_layer.polygons, layer_new.polygons)`. For a straight
    // column the intersection covers the whole cross-section, so that `diff` is
    // EMPTY and `generate_support_layers` (same file) skips the now-empty
    // intermediate layer via its `if (! layer.polygons.empty())` guard --
    // canonical prints no body on an interface layer of a uniform column.
    // The "body survives the carve" invariant therefore holds strictly BELOW
    // the top-interface band, not on it.
    let interface_band_bottom = output
        .entries()
        .iter()
        .filter(|entry| {
            entry
                .roles
                .iter()
                .any(|role| role.role == SupportPlanRole::TopInterface && !role.regions.is_empty())
        })
        .map(|entry| entry.global_layer_index)
        .min()
        .unwrap_or(top_geometry_layer);
    for entry in output.entries() {
        assert_eq!(entry.family_id, "traditional");
        assert!(!entry.demand_ids.is_empty());
        assert!(!entry.body_ids.is_empty());
        if entry.global_layer_index < interface_band_bottom {
            // Below the interface band nothing is projected into the
            // intermediate layer, so canonical's base remainder is the full
            // cross-section and a body must be printed. The widened form (`any
            // non-empty role`) passed even though this planner emits exactly
            // one role per entry, so body and interface can never coexist — the
            // defect the assertion was supposed to catch.
            assert!(
                entry.roles.iter().any(|role| role.role == SupportPlanRole::SupportBody
                    && !role.regions.is_empty()),
                "entry at layer {} is below the top-interface band yet carries no SupportBody geometry. Canonical leaves the intermediate layer untouched below the band, so it must still print a body cross-section. Roles: {:?}",
                entry.global_layer_index,
                entry.roles
            );
        } else {
            assert!(
                entry.roles.iter().any(|role| !role.regions.is_empty()),
                "topmost support layer {} carries no printable geometry at all",
                entry.global_layer_index
            );
        }
    }
    assert!(
        output
            .entries()
            .iter()
            .any(|e| e.roles.iter().any(|r| r.role == SupportPlanRole::SupportBody)),
        "a multi-layer column must carry body geometry below its interface band"
    );
    assert!(
        output.entries().len() >= 2,
        "contact area must derive body/interface roles across layers"
    );
}

#[test]
fn base_interface_obstacle() {
    let object = overhang_object("base");
    let analysis = SupportAnalysisView {
        candidates: vec![
            SupportAnalysisCandidate {
                id: 1,
                object_id: "base".into(),
                region_id: "0".into(),
                global_layer_index: 8,
                z_units: slicer_ir::mm_to_units(1.8),
                geometry: vec![contact_region()],
                ..Default::default()
            },
            SupportAnalysisCandidate {
                id: 2,
                object_id: "base".into(),
                region_id: "0".into(),
                global_layer_index: 8,
                z_units: slicer_ir::mm_to_units(1.8),
                geometry: vec![obstacle_region()],
                ..Default::default()
            },
        ],
        model_occupancy: vec![SupportAnalysisGeometryEntry {
            global_support_layer_index: 5,
            object_id: "base".into(),
            region_id: "0".into(),
            polygons: vec![obstacle_region()],
        }],
        // A bottom interface exists only where the column lands ON the model.
        // Without a termination surface this column runs to the build plate,
        // and a floor there would be dense interface against bare plate.
        termination_surfaces: vec![SupportAnalysisGeometryEntry {
            global_support_layer_index: 2,
            object_id: "base".into(),
            region_id: "0".into(),
            polygons: vec![contact_region()],
        }],
        family_assignments: vec![traditional_assignment("base")],
        ..Default::default()
    };
    let output = run_planner_with_analysis(true, object, analysis);

    let body_entries: Vec<_> = output
        .entries()
        .iter()
        .filter(|e| e.demand_ids.iter().any(|d| d == "demand-1"))
        .collect();
    assert!(
        body_entries.len() >= 2,
        "base polygons must propagate through eligible layers"
    );
    assert!(
        body_entries.iter().any(|e| e
            .roles
            .iter()
            .any(|r| r.role == SupportPlanRole::TopInterface)),
        "top interface must honor support_interface_top_layers"
    );
    assert!(
        body_entries.iter().any(|e| e
            .roles
            .iter()
            .any(|r| r.role == SupportPlanRole::BottomInterface)),
        "bottom interface must honor support_interface_bottom_layers"
    );
    assert!(
        body_entries.iter().all(|e| e
            .capabilities
            .iter()
            .any(|c| c.contains("traditional-base-pattern"))),
        "base pattern must be recorded in capabilities"
    );
    assert!(
        output.entries().iter().any(|e| {
            e.demand_ids.iter().any(|d| d == "demand-2")
                && e.decline_reason == Some(SupportPlanDeclineReason::NoRoute)
                && e.roles.is_empty()
        }),
        "obstacle candidate must be structurally declined"
    );
}

/// A column that lands on the build plate carries no bottom interface: there
/// is no model surface beneath to interface with. Before packet 224 the floor
/// band was applied unconditionally, so a plate-terminated column printed dense
/// interface on its first layers — visible as `;TYPE:Support interface` at
/// Z 0.2 and 0.4 on the decisive fixture.
#[test]
fn plate_termination_emits_no_bottom_interface() {
    let object = overhang_object("plate-term");
    let analysis = SupportAnalysisView {
        candidates: vec![SupportAnalysisCandidate {
            id: 1,
            object_id: "plate-term".into(),
            region_id: "0".into(),
            global_layer_index: 8,
            z_units: slicer_ir::mm_to_units(1.8),
            geometry: vec![contact_region()],
            ..Default::default()
        }],
        // No termination surfaces: the column runs to the plate.
        family_assignments: vec![traditional_assignment("plate-term")],
        ..Default::default()
    };
    let output = run_planner_with_analysis(true, object, analysis);
    assert!(
        !output.entries().is_empty(),
        "plate-terminated column must still be planned"
    );
    assert!(
        !output.entries().iter().any(|e| e
            .roles
            .iter()
            .any(|r| r.role == SupportPlanRole::BottomInterface)),
        "a plate-terminated column must carry no BottomInterface role; got {:?}",
        output
            .entries()
            .iter()
            .map(|e| e.roles.iter().map(|r| r.role).collect::<Vec<_>>())
            .collect::<Vec<_>>()
    );
}

#[test]
fn anchored_termination() {
    let object = overhang_object("anchored");
    let analysis = SupportAnalysisView {
        candidates: vec![
            SupportAnalysisCandidate {
                id: 41,
                object_id: "anchored".into(),
                region_id: "0".into(),
                global_layer_index: 8,
                z_units: slicer_ir::mm_to_units(1.8),
                geometry: vec![contact_region()],
                ..Default::default()
            },
            SupportAnalysisCandidate {
                id: 42,
                object_id: "anchored".into(),
                region_id: "0".into(),
                global_layer_index: 8,
                z_units: slicer_ir::mm_to_units(1.8),
                geometry: vec![contact_region()],
                ..Default::default()
            },
        ],
        family_assignments: vec![traditional_assignment("anchored")],
        ..Default::default()
    };
    let output = run_planner_with_analysis(true, object, analysis);
    assert!(!output.entries().is_empty());
    assert!(output
        .entries()
        .iter()
        .any(|e| e.demand_ids.contains(&"demand-41".to_string())));
    assert!(output
        .entries()
        .iter()
        .any(|e| e.demand_ids.contains(&"demand-42".to_string())));
    let lowest_layer = output
        .entries()
        .iter()
        .map(|entry| entry.global_layer_index)
        .min()
        .unwrap();
    assert_eq!(
        lowest_layer, 0,
        "termination must reach the plate-side layer"
    );
    let top_geometry_layer = output
        .entries()
        .iter()
        .filter(|entry| entry.roles.iter().any(|role| !role.regions.is_empty()))
        .map(|entry| entry.global_layer_index)
        .max()
        .expect("planner must emit geometry on at least one layer");
    // Canonical `generate_interface_layers` (`SupportCommon.cpp`) builds the
    // interface as `intersection(intermediate_layer.polygons, <projected top
    // contacts>)` and then does `intermediate_layer.polygons =
    // diff(intermediate_layer.polygons, layer_new.polygons)`. For a straight
    // column the intersection covers the whole cross-section, so that `diff` is
    // EMPTY and `generate_support_layers` (same file) skips the now-empty
    // intermediate layer via its `if (! layer.polygons.empty())` guard --
    // canonical prints no body on an interface layer of a uniform column.
    // The "body survives the carve" invariant therefore holds strictly BELOW
    // the top-interface band, not on it.
    let interface_band_bottom = output
        .entries()
        .iter()
        .filter(|entry| {
            entry
                .roles
                .iter()
                .any(|role| role.role == SupportPlanRole::TopInterface && !role.regions.is_empty())
        })
        .map(|entry| entry.global_layer_index)
        .min()
        .unwrap_or(top_geometry_layer);
    for entry in output.entries() {
        assert_eq!(
            entry.anchor_z,
            slicer_ir::mm_to_units((entry.global_layer_index as f32 + 1.0) * 0.2)
        );
        assert_eq!(entry.anchor_layer_index, entry.global_layer_index as u32);
        if entry.global_layer_index < interface_band_bottom {
            // Below the interface band nothing is projected into the
            // intermediate layer, so canonical's base remainder is the full
            // cross-section and a body must be printed. The widened form (`any
            // non-empty role`) passed even though this planner emits exactly
            // one role per entry, so body and interface can never coexist — the
            // defect the assertion was supposed to catch.
            assert!(
                entry.roles.iter().any(|role| role.role == SupportPlanRole::SupportBody
                    && !role.regions.is_empty()),
                "entry at layer {} is below the top-interface band yet carries no SupportBody geometry. Canonical leaves the intermediate layer untouched below the band, so it must still print a body cross-section. Roles: {:?}",
                entry.global_layer_index,
                entry.roles
            );
        } else {
            assert!(
                entry.roles.iter().any(|role| !role.regions.is_empty()),
                "topmost support layer {} carries no printable geometry at all",
                entry.global_layer_index
            );
        }
    }
}

#[test]
fn disabled_and_declined() {
    let object = overhang_object("disabled");
    let analysis = SupportAnalysisView {
        candidates: vec![SupportAnalysisCandidate {
            id: 1,
            object_id: "disabled".into(),
            region_id: "0".into(),
            global_layer_index: 8,
            z_units: slicer_ir::mm_to_units(1.8),
            geometry: vec![contact_region()],
            ..Default::default()
        }],
        family_assignments: vec![traditional_assignment("disabled")],
        ..Default::default()
    };
    let disabled = run_planner_with_analysis(false, object.clone(), analysis);
    assert!(disabled.entries().iter().any(|entry| {
        entry.decline_reason == Some(SupportPlanDeclineReason::DeclinedPolicy)
            && entry.demand_ids.is_empty()
            && entry.body_ids.is_empty()
            && entry.roles.is_empty()
            && entry.provenance == vec!["traditional-support-planner"]
    }));
    assert!(disabled.diagnostics().is_empty());

    let blocked_analysis = SupportAnalysisView {
        candidates: vec![SupportAnalysisCandidate {
            id: 7,
            object_id: "disabled".into(),
            region_id: "0".into(),
            global_layer_index: 8,
            z_units: slicer_ir::mm_to_units(1.8),
            blocked: true,
            ..Default::default()
        }],
        family_assignments: vec![traditional_assignment("disabled")],
        ..Default::default()
    };
    let blocked = run_planner_with_analysis(true, object, blocked_analysis);
    assert!(blocked
        .entries()
        .iter()
        .any(|entry| entry.decline_reason == Some(SupportPlanDeclineReason::Blocked)));
    for entry in blocked.entries() {
        if entry.decline_reason.is_some() {
            assert!(entry.roles.is_empty());
            assert!(entry.body_ids.is_empty());
        }
    }

    // exhaustive: support-plan identity fixture; SupportPlanEntry has no Default impl and FRU would let a new plan field default silently
    let declined = SupportPlanEntry {
        global_layer_index: 0,
        object_id: "object-a".into(),
        region_id: 7,
        family_id: "traditional".into(),
        demand_ids: vec!["unroutable-demand".into()],
        body_ids: vec![],
        anchor_layer_index: 0,
        anchor_z: 0,
        roles: vec![],
        skeleton: None,
        capabilities: vec![],
        provenance: vec!["traditional-support-planner".into()],
        decline_reason: Some(SupportPlanDeclineReason::NoRoute),
    };
    let recorded = aggregate_declined_support_plans(&[SupportPlanIR {
        entries: vec![declined],
        ..Default::default()
    }]);
    assert_eq!(recorded.declined.len(), 1);
    assert_eq!(
        recorded.declined[0].reason,
        SupportPlanDeclineReason::NoRoute
    );
    assert!(recorded.support_paths.is_empty());
}

#[test]
fn top_z_distance_lowers_contact_start() {
    let object = overhang_object("top-distance");
    let analysis = SupportAnalysisView {
        candidates: vec![SupportAnalysisCandidate {
            id: 1,
            object_id: "top-distance".into(),
            region_id: "0".into(),
            global_layer_index: 8,
            z_units: slicer_ir::mm_to_units(1.8),
            geometry: vec![contact_region()],
            ..Default::default()
        }],
        family_assignments: vec![traditional_assignment("top-distance")],
        ..Default::default()
    };
    let output = run_planner_with_config(planner_config_with(true, 0.5, 0.0), object, analysis);
    let highest = output
        .entries()
        .iter()
        .filter(|entry| entry.decline_reason.is_none())
        .map(|entry| entry.global_layer_index)
        .max()
        .unwrap();
    // The candidate's layer is the first layer that *contains* the overhang,
    // so the overhanging surface is at the bottom of layer 8 — the top of
    // layer 7, z = 1.6. A 0.5 mm gap requires the topmost support layer's own
    // top to sit at or below 1.1, which is layer 4 (top z = 1.0); layer 5's top
    // is 1.2 and would leave only 0.4 mm.
    //
    // This asserted `8 - ceil(0.5/0.2) = 5`, which measured the gap from the
    // overhang layer itself rather than from the surface, and divided by
    // `effective_layer_height` — a field that is unreliable in the guest view
    // and evaluated such that the production gap came out as ZERO. Measured on
    // the decisive fixture: support fused to the model at Z=25.0 with the
    // overhang underside also at 25.0. With the Z-walk the top contact lands at
    // 24.8, which is exactly where OrcaSlicer puts it in
    // `tmp/SupportTest_Normal_Orca.gcode`.
    assert_eq!(
        highest, 4,
        "a 0.5 mm gap below the overhang surface (z=1.6) admits layer 4 (top z=1.0), not layer 5 (top z=1.2)"
    );
}

#[test]
fn support_layer_height_controls_body_spacing() {
    let object = overhang_object("support-height");
    let analysis = SupportAnalysisView {
        candidates: vec![SupportAnalysisCandidate {
            id: 1,
            object_id: "support-height".into(),
            region_id: "0".into(),
            global_layer_index: 8,
            z_units: slicer_ir::mm_to_units(1.8),
            geometry: vec![contact_region()],
            ..Default::default()
        }],
        family_assignments: vec![traditional_assignment("support-height")],
        ..Default::default()
    };
    let output = run_planner_with_config(planner_config_with(true, 0.0, 0.6), object, analysis);
    let mut layers: Vec<_> = output
        .entries()
        .iter()
        .filter(|entry| entry.decline_reason.is_none())
        .map(|entry| entry.global_layer_index)
        .collect();
    layers.sort_unstable_by(|a, b| b.cmp(a));
    // Shifted down one layer from the previous `[8, 7, 5, 2, 0]`: layer 8 is
    // the layer that *contains* the overhang, so support may not print there
    // even at a zero top gap. Layer 0 is the termination layer, which now
    // always prints — it used to be dropped whenever it failed the
    // support-layer-height modulo, leaving the column stopping short of the
    // plate.
    // G-18 (238c, design.md §Plan Corrections item 4): with a positive
    // configured bottom count the traditional top band widens by one layer,
    // so top=2/bottom=1 here yields interface layers 7/6/5 instead of 7/6.
    assert_eq!(
        layers,
        vec![7, 6, 5, 4, 1, 0],
        "support body layers use every third model layer, interfaces retain their G-18-widened bands, and the termination layer always prints"
    );
}

#[test]
fn model_termination_surface_stops_descent() {
    let object = overhang_object("model-termination");
    let analysis = SupportAnalysisView {
        candidates: vec![SupportAnalysisCandidate {
            id: 1,
            object_id: "model-termination".into(),
            region_id: "0".into(),
            global_layer_index: 8,
            z_units: slicer_ir::mm_to_units(1.8),
            geometry: vec![contact_region()],
            ..Default::default()
        }],
        termination_surfaces: vec![SupportAnalysisGeometryEntry {
            global_support_layer_index: 4,
            object_id: "model-termination".into(),
            region_id: "0".into(),
            polygons: vec![contact_region()],
        }],
        family_assignments: vec![traditional_assignment("model-termination")],
        ..Default::default()
    };
    let output = run_planner_with_analysis(true, object, analysis);
    let entries: Vec<_> = output
        .entries()
        .iter()
        .filter(|entry| entry.decline_reason.is_none())
        .collect();
    assert_eq!(
        entries.iter().map(|entry| entry.global_layer_index).min(),
        Some(4)
    );
    let termination = entries
        .iter()
        .find(|entry| entry.global_layer_index == 4)
        .unwrap();
    assert_eq!(termination.anchor_layer_index, 4);
    assert_eq!(termination.anchor_z, slicer_ir::mm_to_units(1.0));
}

#[test]
fn invalid_body_rejected() {
    let object = overhang_object("invalid");
    let analysis = SupportAnalysisView {
        candidates: vec![SupportAnalysisCandidate {
            id: 1,
            object_id: "invalid".into(),
            region_id: "0".into(),
            global_layer_index: 8,
            z_units: slicer_ir::mm_to_units(1.8),
            geometry: vec![contact_region()],
            ..Default::default()
        }],
        model_occupancy: vec![SupportAnalysisGeometryEntry {
            global_support_layer_index: 5,
            object_id: "invalid".into(),
            region_id: "0".into(),
            polygons: vec![contact_region()],
        }],
        family_assignments: vec![traditional_assignment("invalid")],
        ..Default::default()
    };
    let output = run_planner_with_analysis(true, object, analysis);
    assert!(
        output
            .diagnostics()
            .iter()
            .any(|d| d.code == 1203 && d.message.contains("complete body")),
        "planner diagnostics: {:?}",
        output.diagnostics()
    );
    assert!(
        output
            .entries()
            .iter()
            .flat_map(|entry| entry.roles.iter())
            .flat_map(|role| role.regions.iter())
            .next()
            .is_none(),
        "colliding demand must emit no body/interface polygons"
    );

    // Genuinely oversized: one unit wider AND taller than ROUTING_CELL_SIZE
    // (1 << 20), so it fits in no cell-sized territory wherever it is placed.
    // (Before packet 224 this fixture was a 1_000-unit body parked across the
    // x = 1 << 20 grid line, which pinned the absolute-grid defect rather than
    // the size contract; the extent-based `in_routing_cell` now retains such a
    // body, as it should.) It is also kept clear of the validation mesh's
    // 0..100_000-unit footprint so occupancy cannot be the cause of the drop.
    const OVERSIZE: i64 = (1 << 20) + 1;
    let crossing = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 {
                    x: 1_048_076,
                    y: 5_000,
                },
                Point2 {
                    x: 1_048_076 + OVERSIZE,
                    y: 5_000,
                },
                Point2 {
                    x: 1_048_076 + OVERSIZE,
                    y: 5_000 + OVERSIZE,
                },
                Point2 {
                    x: 1_048_076,
                    y: 5_000 + OVERSIZE,
                },
            ],
        },
        holes: vec![],
    };
    let plan = SupportPlanIR {
        // exhaustive: support-plan identity fixture; SupportPlanEntry has no Default impl and FRU would let a new plan field default silently
        entries: vec![SupportPlanEntry {
            global_layer_index: 0,
            object_id: "object-a".into(),
            region_id: 9,
            family_id: "traditional".into(),
            demand_ids: vec!["spans-cell".into()],
            body_ids: vec!["spans-cell-body".into()],
            anchor_layer_index: 0,
            anchor_z: 4_321,
            roles: vec![
                SupportPlanRoleRegion {
                    role: SupportPlanRole::SupportBody,
                    regions: vec![crossing.clone()],
                },
                SupportPlanRoleRegion {
                    role: SupportPlanRole::TopInterface,
                    regions: vec![crossing],
                },
            ],
            skeleton: None,
            capabilities: vec![],
            provenance: vec!["traditional-support-planner".into()],
            decline_reason: None,
        }],
        ..Default::default()
    };
    let exact_z = ExactZQueryService::new(Arc::new(validation_mesh()));
    let (aggregated, diagnostics) =
        aggregate_support_plan_irs_with_diagnostics(vec![plan], &exact_z);
    assert!(
        aggregated.entries.is_empty(),
        "complete crossing body is dropped"
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.message.contains("spans-cell") && diagnostic.message.contains("routing-cell")
    }));
    assert!(aggregated
        .entries
        .iter()
        .flat_map(|entry| entry.roles.iter())
        .all(|role| role.regions.is_empty()));
}

/// A candidate whose downward route is closed by the model records a structured
/// `NoRoute` decline rather than vanishing from the plan.
///
/// **Retargeted by packet 224.** This test was `fully_covered_candidate_is_declined`
/// and placed its occupancy at layer 9 — *above* the layer-8 contact — asserting
/// the planner declines a contact covered from above. The planner never
/// implemented that rule. It passed because its fixture is `minimal_object`, an
/// empty mesh with no vertices or triangles, and the planner's since-removed
/// mesh-facet contact derivation declined any mesh with no downward facets. The
/// name described coverage; the mechanism was "empty mesh".
///
/// Judging whether a region is an overhang at all now belongs to
/// `PrePass::SupportAnalysis` — a region covered by the layer above is not an
/// overhang, so `detect_support_overhangs` never emits a candidate for it. That
/// is gated by `straight_column_yields_no_support_candidates` and
/// `support_analysis_populates_all_derivable_inputs` in
/// `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`.
///
/// What the planner owns is routing: given a real contact, does a route to a
/// termination surface exist? This test now pins that, with occupancy *below*
/// the contact where the planner can actually act on it.
#[test]
fn candidate_with_no_downward_route_is_declined() {
    let object = overhang_object("blocked-route");
    let full_region = contact_region();
    let analysis = SupportAnalysisView {
        candidates: vec![SupportAnalysisCandidate {
            id: 1,
            object_id: "blocked-route".into(),
            region_id: "0".into(),
            global_layer_index: 8,
            z_units: slicer_ir::mm_to_units(1.8),
            geometry: vec![full_region.clone()],
            ..Default::default()
        }],
        // Model occupancy BELOW the contact, covering the whole contact area, so
        // no descending route survives the per-layer trim.
        model_occupancy: vec![SupportAnalysisGeometryEntry {
            global_support_layer_index: 4,
            object_id: "blocked-route".into(),
            region_id: "0".into(),
            polygons: vec![full_region],
        }],
        family_assignments: vec![traditional_assignment("blocked-route")],
        ..Default::default()
    };
    let output = run_planner_with_analysis(true, object, analysis);
    assert!(
        output
            .entries()
            .iter()
            .any(|entry| entry.decline_reason == Some(SupportPlanDeclineReason::NoRoute)),
        "a candidate with no downward route must record a structured decline, got {:?}",
        output.entries()
    );
    assert!(
        !output
            .entries()
            .iter()
            .any(|entry| entry.decline_reason.is_none()
                && entry.roles.iter().any(|role| !role.regions.is_empty())),
        "a blocked candidate must not emit a non-declined entry with roles"
    );
}

#[test]
fn real_candidate_contact_derivation() {
    let object = overhang_object("real");
    // Contact geometry comes from the downward-facing mesh facet, not from
    // subtracting the layer-above occupancy.
    let full_region = contact_region();
    let analysis = SupportAnalysisView {
        candidates: vec![SupportAnalysisCandidate {
            id: 1,
            object_id: "real".into(),
            region_id: "0".into(),
            global_layer_index: 8,
            z_units: slicer_ir::mm_to_units(1.8),
            geometry: vec![full_region.clone()],
            ..Default::default()
        }],
        model_occupancy: vec![SupportAnalysisGeometryEntry {
            global_support_layer_index: 8,
            object_id: "real".into(),
            region_id: "0".into(),
            polygons: vec![full_region],
        }],
        family_assignments: vec![traditional_assignment("real")],
        ..Default::default()
    };
    let output = run_planner_with_analysis(true, object, analysis);
    assert!(
        !output.entries().is_empty(),
        "real candidate must not be rejected for touching the model at the contact layer"
    );
    assert!(output
        .entries()
        .iter()
        .any(|entry| entry.roles.iter().any(|role| !role.regions.is_empty())));
    let lowest_layer = output
        .entries()
        .iter()
        .map(|entry| entry.global_layer_index)
        .min()
        .unwrap();
    assert_eq!(
        lowest_layer, 0,
        "body must propagate downward to the plate-side layer"
    );
}

#[test]
fn slanted_face_contacts_derived_from_facets() {
    let object = MeshObjectView {
        object_id: "ramp".into(),
        vertices: vec![[0.0, 0.0, 0.0], [4.0, 0.0, 1.8], [0.0, 4.0, 1.8]],
        triangles: vec![[0, 2, 1]],
        paint_layers: vec![],
    };
    let analysis = SupportAnalysisView {
        candidates: vec![SupportAnalysisCandidate {
            id: 1,
            object_id: "ramp".into(),
            region_id: "0".into(),
            global_layer_index: 8,
            z_units: slicer_ir::mm_to_units(1.8),
            geometry: vec![contact_region()],
            ..Default::default()
        }],
        family_assignments: vec![traditional_assignment("ramp")],
        ..Default::default()
    };
    let output = run_planner_with_analysis(true, object, analysis);
    assert!(output.entries().iter().any(|entry| {
        entry.decline_reason.is_none() && entry.roles.iter().any(|role| !role.regions.is_empty())
    }));
}

/// F-36. Canonical `bottom_contact_layers_and_layer_support_areas` builds the
/// floor from `intersection(top_surfaces, supports_projected)` expanded by one
/// support-flow width. A column that lands half on a model top surface and half
/// on bare plate must therefore print BottomInterface only over the model half
/// and keep printing body over the rest. Before this fix the planner marked the
/// whole layer cross-section BottomInterface.
#[test]
fn bottom_interface_is_limited_to_the_model_landing_area() {
    // Left half of the 4x4 mm contact column only.
    let landing = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(2.0, 0.0),
                Point2::from_mm(2.0, 4.0),
                Point2::from_mm(0.0, 4.0),
            ],
        },
        holes: vec![],
    };
    let object = overhang_object("partial");
    let analysis = SupportAnalysisView {
        candidates: vec![SupportAnalysisCandidate {
            id: 1,
            object_id: "partial".into(),
            region_id: "0".into(),
            global_layer_index: 8,
            z_units: slicer_ir::mm_to_units(1.8),
            geometry: vec![contact_region()],
            ..Default::default()
        }],
        termination_surfaces: vec![SupportAnalysisGeometryEntry {
            global_support_layer_index: 3,
            object_id: "partial".into(),
            region_id: "0".into(),
            polygons: vec![landing],
        }],
        family_assignments: vec![traditional_assignment("partial")],
        ..Default::default()
    };
    let output = run_planner_with_analysis(true, object, analysis);

    let floor_entry = output
        .entries()
        .iter()
        .find(|entry| {
            entry
                .roles
                .iter()
                .any(|role| role.role == SupportPlanRole::BottomInterface)
        })
        .expect("a column landing on a model top surface must carry a floor");

    let floor_max_x = floor_entry
        .roles
        .iter()
        .filter(|role| role.role == SupportPlanRole::BottomInterface)
        .flat_map(|role| role.regions.iter())
        .flat_map(|expoly| expoly.contour.points.iter())
        .map(|point| point.x)
        .max()
        .expect("floor role must carry geometry");
    // The landing is 2 mm wide; canonical grows it by one flow width (0.4 mm).
    // The column is 4 mm wide, so a whole-layer floor would reach 4 mm.
    assert!(
        floor_max_x < slicer_ir::mm_to_units(4.0),
        "BottomInterface must stop at the model landing area (plus one flow width), \
         not span the whole column cross-section: max x = {floor_max_x}"
    );
    assert!(
        floor_max_x >= slicer_ir::mm_to_units(2.0),
        "BottomInterface must cover the landing area itself: max x = {floor_max_x}"
    );
    assert!(
        floor_entry
            .roles
            .iter()
            .any(|role| role.role == SupportPlanRole::SupportBody && !role.regions.is_empty()),
        "the part of the layer standing on bare plate must keep printing as body: {:?}",
        floor_entry.roles
    );
}

/// F-49. Top-interface membership depends only on the layer's distance below
/// the top contact — canonical `generate_interface_layers` counts
/// `top_interface_layers` down from the contact and knows nothing about the
/// build plate. A column shorter than `support_interface_top_layers` used to
/// have its plate layer forced to body by an extra
/// `layer != termination_layer` guard.
#[test]
fn short_plate_column_keeps_its_plate_layer_in_the_top_band() {
    let object = overhang_object("short");
    let analysis = SupportAnalysisView {
        candidates: vec![SupportAnalysisCandidate {
            id: 1,
            object_id: "short".into(),
            region_id: "0".into(),
            // Layer 1 overhang: the column is one printed layer tall (layer 0),
            // shorter than the 2-layer roof band.
            global_layer_index: 1,
            z_units: slicer_ir::mm_to_units(0.4),
            geometry: vec![contact_region()],
            ..Default::default()
        }],
        family_assignments: vec![traditional_assignment("short")],
        ..Default::default()
    };
    let output = run_planner_with_analysis(true, object, analysis);

    let plate_entry = output
        .entries()
        .iter()
        .find(|entry| entry.global_layer_index == 0 && entry.decline_reason.is_none())
        .expect("the plate layer must be planned");
    assert!(
        plate_entry
            .roles
            .iter()
            .any(|role| role.role == SupportPlanRole::TopInterface && !role.regions.is_empty()),
        "the plate layer of a column shorter than the roof band is still inside \
         the top-interface band: {:?}",
        plate_entry.roles
    );
}
