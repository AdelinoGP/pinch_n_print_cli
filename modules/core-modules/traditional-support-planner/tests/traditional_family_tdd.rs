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

fn minimal_object(object_id: &str) -> MeshObjectView {
    MeshObjectView {
        object_id: object_id.into(),
        vertices: vec![],
        triangles: vec![],
        paint_layers: vec![],
    }
}

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
    for entry in output.entries() {
        assert_eq!(entry.family_id, "traditional");
        assert!(!entry.demand_ids.is_empty());
        assert!(!entry.body_ids.is_empty());
        assert!(entry
            .roles
            .iter()
            .any(|r| r.role == SupportPlanRole::SupportBody));
    }
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
    for entry in output.entries() {
        assert_eq!(
            entry.anchor_z,
            slicer_ir::mm_to_units((entry.global_layer_index as f32 + 1.0) * 0.2)
        );
        assert_eq!(entry.anchor_layer_index, entry.global_layer_index as u32);
        assert!(entry
            .roles
            .iter()
            .any(|r| r.role == SupportPlanRole::SupportBody));
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
    assert_eq!(highest, 5, "ceil(0.5 / 0.2) lowers layer 8 by 3 layers");
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
    assert_eq!(
        layers,
        vec![8, 7, 5, 2, 0],
        "support body layers use every third model layer and interfaces retain their bands"
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

    let crossing = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 {
                    x: 1_048_076,
                    y: 5_000,
                },
                Point2 {
                    x: 1_049_076,
                    y: 5_000,
                },
                Point2 {
                    x: 1_049_076,
                    y: 6_000,
                },
                Point2 {
                    x: 1_048_076,
                    y: 6_000,
                },
            ],
        },
        holes: vec![],
    };
    let plan = SupportPlanIR {
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
        diagnostic.message.contains("spans-cell") && diagnostic.message.contains("routing cell")
    }));
    assert!(aggregated
        .entries
        .iter()
        .flat_map(|entry| entry.roles.iter())
        .all(|role| role.regions.is_empty()));
}

#[test]
fn fully_covered_candidate_is_declined() {
    let object = minimal_object("covered");
    let full_region = contact_region();
    let analysis = SupportAnalysisView {
        candidates: vec![SupportAnalysisCandidate {
            id: 1,
            object_id: "covered".into(),
            region_id: "0".into(),
            global_layer_index: 8,
            z_units: slicer_ir::mm_to_units(1.8),
            geometry: vec![full_region.clone()],
            ..Default::default()
        }],
        model_occupancy: vec![SupportAnalysisGeometryEntry {
            global_support_layer_index: 9,
            object_id: "covered".into(),
            region_id: "0".into(),
            polygons: vec![full_region],
        }],
        family_assignments: vec![traditional_assignment("covered")],
        ..Default::default()
    };
    let output = run_planner_with_analysis(true, object, analysis);
    assert!(
        output
            .entries()
            .iter()
            .any(|entry| entry.decline_reason == Some(SupportPlanDeclineReason::NoRoute)),
        "fully-covered candidate must record a structured decline"
    );
    assert!(
        !output
            .entries()
            .iter()
            .any(|entry| entry.decline_reason.is_none()
                && entry.roles.iter().any(|role| !role.regions.is_empty())),
        "fully-covered candidate must not emit a non-declined entry with roles"
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
