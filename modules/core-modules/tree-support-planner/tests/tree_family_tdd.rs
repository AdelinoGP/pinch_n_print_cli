//! Contract tests for the tree support family seam.

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
    RegionSegmentationViewEntry, SupportAnalysisCandidate, SupportAnalysisView,
    SupportFamilyAssignment, SupportGeometryView, SupportGeometryViewEntry,
};
use slicer_sdk::traits::PrepassModule;
use slicer_wasm_host::exact_z_query::ExactZQueryService;
use slicer_wasm_host::support_aggregation::{
    aggregate_declined_support_plans, aggregate_support_plan_irs_with_diagnostics,
};
use tree_support_planner::{body_overlaps_occupancy, tapered_radius};

fn pillar_occupancy() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(-10.0, 0.0),
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(0.0, 20.0),
                Point2::from_mm(-10.0, 20.0),
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

fn planner_config(enabled: bool) -> ConfigView {
    planner_config_with_diameter(enabled, 5.0)
}

fn planner_config_with_diameter(enabled: bool, branch_diameter: f64) -> ConfigView {
    let mut values = HashMap::<ConfigKey, ConfigValue>::new();
    values.insert("enable_support".into(), ConfigValue::Bool(enabled));
    values.insert("support_family".into(), ConfigValue::String("tree".into()));
    values.insert("support_raft_layers".into(), ConfigValue::Int(0));
    values.insert(
        "tree_support_branch_diameter".into(),
        ConfigValue::Float(branch_diameter),
    );
    values.insert(
        "tree_support_branch_diameter_angle".into(),
        ConfigValue::Float(5.0),
    );
    values.insert(
        "tree_support_branch_distance".into(),
        ConfigValue::Float(1.0),
    );
    values.insert("tree_support_wall_count".into(), ConfigValue::Int(1));
    values.insert("tree_support_branch_angle".into(), ConfigValue::Float(45.0));
    ConfigView::from_map(values)
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

fn overhang(object_id: &str, x: f32, y: f32, size: f32) -> MeshObjectView {
    MeshObjectView {
        object_id: object_id.into(),
        vertices: vec![
            [0.0, 0.0, 0.0],
            [x, y, 1.8],
            [x + size, y, 1.8],
            [x + size, y + size, 1.8],
            [x, y + size, 1.8],
        ],
        triangles: vec![[1, 3, 2], [1, 4, 3]],
        paint_layers: vec![],
    }
}

fn two_overhangs(object_id: &str) -> MeshObjectView {
    MeshObjectView {
        object_id: object_id.into(),
        vertices: vec![
            [0.0, 0.0, 0.0],
            [0.0, 10.0, 1.8],
            [2.0, 10.0, 1.8],
            [2.0, 12.0, 1.8],
            [0.0, 12.0, 1.8],
            [6.0, 10.0, 1.8],
            [8.0, 10.0, 1.8],
            [8.0, 12.0, 1.8],
            [6.0, 12.0, 1.8],
        ],
        triangles: vec![[1, 3, 2], [1, 4, 3], [5, 7, 6], [5, 8, 7]],
        paint_layers: vec![],
    }
}

fn run_planner(
    enabled: bool,
    object: MeshObjectView,
    geometry: SupportGeometryView,
) -> SupportGeometryOutput {
    let planner = tree_support_planner::SupportPlanner::from_config(&planner_config(enabled))
        .expect("from_config");
    let mut output = SupportGeometryOutput::new();
    planner
        .run_support_geometry(
            &[object.clone()],
            &layer_plan(),
            &regions(&object.object_id),
            &geometry,
            &mut output,
            &ConfigView::new(),
        )
        .expect("run_support_geometry");
    output
}

fn run_planner_with_analysis(
    enabled: bool,
    object: MeshObjectView,
    analysis: SupportAnalysisView,
) -> SupportGeometryOutput {
    let planner = tree_support_planner::SupportPlanner::from_config(&planner_config(enabled))
        .expect("from_config");
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

fn run_planner_with_analysis_and_diameter(
    enabled: bool,
    object: MeshObjectView,
    analysis: SupportAnalysisView,
    branch_diameter: f64,
) -> SupportGeometryOutput {
    let planner = tree_support_planner::SupportPlanner::from_config(&planner_config_with_diameter(
        enabled,
        branch_diameter,
    ))
    .expect("from_config");
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

fn mm_point(point: &Point2) -> (f32, f32) {
    (point.x as f32 / 10_000.0, point.y as f32 / 10_000.0)
}

fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

fn point_segment_distance(point: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let length_squared = dx * dx + dy * dy;
    let t = if length_squared == 0.0 {
        0.0
    } else {
        ((point.0 - a.0) * dx + (point.1 - a.1) * dy) / length_squared
    };
    let t = t.clamp(0.0, 1.0);
    distance(point, (a.0 + t * dx, a.1 + t * dy))
}

fn run_blocked_planner() -> SupportGeometryOutput {
    let object = overhang("declined", 0.0, 0.0, 4.0);
    let planner = tree_support_planner::SupportPlanner::from_config(&planner_config(true))
        .expect("from_config");
    let mut output = SupportGeometryOutput::new();
    planner
        .run_support_geometry_with_analysis(
            &[object],
            &layer_plan(),
            &regions("declined"),
            &SupportAnalysisView {
                candidates: vec![SupportAnalysisCandidate {
                    id: 7,
                    object_id: "declined".into(),
                    region_id: "0".into(),
                    global_layer_index: 8,
                    z_units: slicer_ir::mm_to_units(1.8),
                    blocked: true,
                    ..Default::default()
                }],
                ..Default::default()
            },
            &SupportGeometryView::default(),
            &mut output,
            &ConfigView::new(),
        )
        .expect("run_support_geometry_with_analysis");
    output
}

fn declined_entry(reason: SupportPlanDeclineReason) -> SupportPlanEntry {
    SupportPlanEntry {
        global_layer_index: 0,
        object_id: "object-a".into(),
        region_id: 7,
        family_id: "tree".into(),
        demand_ids: vec!["unroutable-demand".into()],
        body_ids: vec![],
        anchor_layer_index: 0,
        anchor_z: 0,
        roles: vec![],
        skeleton: None,
        capabilities: vec![],
        provenance: vec!["tree-planner".into()],
        decline_reason: Some(reason),
    }
}

#[test]
fn distributed_contacts() {
    let candidate_region = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(4.0, 0.0),
                Point2::from_mm(4.0, 4.0),
                Point2::from_mm(0.0, 4.0),
            ],
        },
        holes: vec![],
    };
    let output = run_planner_with_analysis_and_diameter(
        true,
        overhang("distributed", 0.0, 0.0, 4.0),
        SupportAnalysisView {
            candidates: vec![SupportAnalysisCandidate {
                id: 11,
                object_id: "distributed".into(),
                region_id: "0".into(),
                global_layer_index: 8,
                z_units: slicer_ir::mm_to_units(1.8),
                geometry: vec![candidate_region.clone()],
                ..Default::default()
            }],
            ..Default::default()
        },
        1.0,
    );
    assert!(
        output.entries().len() >= 2,
        "planner must emit multiple layers"
    );
    for entry in output.entries() {
        assert_eq!(entry.family_id, "tree");
        assert!(!entry.demand_ids.is_empty());
        assert!(!entry.body_ids.is_empty());
        assert!(entry.roles.iter().any(|role| !role.regions.is_empty()));
        assert!(entry.skeleton.as_ref().is_some());
    }
    let mut classes = [0_u32; 3];
    for point in output
        .entries()
        .iter()
        .filter(|entry| entry.global_layer_index == 8)
        .flat_map(|entry| entry.skeleton.as_ref().unwrap().points.iter())
        .filter(|point| (point.z - 1.8).abs() < 0.001)
    {
        let p = (point.x, point.y);
        let vertices = [(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)];
        let vertex = vertices.iter().any(|vertex| distance(p, *vertex) < 0.01);
        let edge = (0..vertices.len()).any(|i| {
            point_segment_distance(p, vertices[i], vertices[(i + 1) % vertices.len()]) < 0.01
        });
        if vertex {
            classes[0] += 1;
        } else if edge {
            classes[1] += 1;
        } else if p.0 > 0.0 && p.0 < 4.0 && p.1 > 0.0 && p.1 < 4.0 {
            classes[2] += 1;
        }
    }
    assert!(
        classes.iter().filter(|count| **count >= 2).count() >= 2,
        "contacts must span at least two corner/contour/interior classes: {classes:?}"
    );

    let angles: Vec<f32> = output
        .entries()
        .iter()
        .filter_map(|entry| entry.skeleton.as_ref())
        .flat_map(|skeleton| skeleton.points.windows(2))
        .filter_map(|segment| {
            let dx = segment[1].x - segment[0].x;
            let dy = segment[1].y - segment[0].y;
            (dx.abs() > 0.001 || dy.abs() > 0.001).then(|| dy.atan2(dx))
        })
        .collect();
    assert!(
        angles.windows(2).any(|pair| {
            let diff = (pair[0] - pair[1]).abs();
            let diff = if diff > std::f32::consts::PI {
                2.0 * std::f32::consts::PI - diff
            } else {
                diff
            };
            diff > 10.0_f32.to_radians()
        }),
        "planned skeleton must contain branching directions, got {angles:?}"
    );
}

#[test]
fn radius_aware_collision() {
    // Prior defect fixture: a pillar at X -10..0, Y 0..20, Z 0..30,
    // with an overhang above its right edge.
    let pillar = pillar_occupancy();
    let mut blocking = pillar.clone();
    blocking.contour.points[1].x = slicer_ir::mm_to_units(5.0);
    blocking.contour.points[2].x = slicer_ir::mm_to_units(5.0);
    let output = run_planner(
        true,
        two_overhangs("radius"),
        SupportGeometryView {
            entries: (0..10)
                .map(|layer| SupportGeometryViewEntry {
                    global_support_layer_index: layer,
                    object_id: "radius".into(),
                    region_id: "0".into(),
                    outlines: vec![blocking.clone()],
                })
                .collect(),
        },
    );
    assert!(
        output.diagnostics().iter().any(|d| d.code == 1203),
        "planner diagnostics: {:?}, entries: {}",
        output.diagnostics(),
        output.entries().len()
    );
    let mut emitted_survivor = false;
    for body in output
        .entries()
        .iter()
        .flat_map(|entry| entry.roles.iter())
        .flat_map(|role| role.regions.iter())
    {
        assert_eq!(body.contour.points.len(), 16);
        let center = body
            .contour
            .points
            .iter()
            .map(mm_point)
            .fold((0.0, 0.0), |sum, point| {
                (sum.0 + point.0 / 16.0, sum.1 + point.1 / 16.0)
            });
        let radii: Vec<f32> = body
            .contour
            .points
            .iter()
            .map(|point| distance(mm_point(point), center))
            .collect();
        let local_radius = radii.iter().copied().fold(f32::INFINITY, f32::min);
        // Floor guards against degenerate/zero-radius bodies. Retargeted from
        // 0.39 mm to 0.3 mm (packet 224, RC-15 contact-sampling port): the
        // legitimate swept-capsule body between a 0.4 mm contact tip and a
        // larger propagated node (16-vertex cap) has min radius 0.3366 mm,
        // which the stale 0.39 floor rejected.
        assert!(
            local_radius >= 0.3,
            "body lost its local radius: {local_radius}"
        );
        assert!(radii.iter().all(|radius| *radius >= local_radius - 0.001));
        assert!(!body_overlaps_occupancy(
            &[pillar.clone()],
            center.0,
            center.1,
            local_radius
        ));
        if center.0 >= 5.0 {
            emitted_survivor = true;
        } else {
            panic!("colliding pillar body or fallback was emitted at {center:?}");
        }
    }
    assert!(
        emitted_survivor,
        "non-colliding fixture body should remain emitted"
    );
}

#[test]
fn anchored_heights_and_termination() {
    let demand_region = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(1.0, 1.0),
                Point2::from_mm(1.2, 1.0),
                Point2::from_mm(1.2, 1.2),
                Point2::from_mm(1.0, 1.2),
            ],
        },
        holes: vec![],
    };
    let output = run_planner_with_analysis(
        true,
        overhang("anchored", 0.0, 0.0, 4.0),
        SupportAnalysisView {
            candidates: vec![
                SupportAnalysisCandidate {
                    id: 41,
                    object_id: "anchored".into(),
                    region_id: "0".into(),
                    global_layer_index: 8,
                    z_units: slicer_ir::mm_to_units(1.8),
                    geometry: vec![demand_region.clone()],
                    ..Default::default()
                },
                SupportAnalysisCandidate {
                    id: 42,
                    object_id: "anchored".into(),
                    region_id: "0".into(),
                    global_layer_index: 8,
                    z_units: slicer_ir::mm_to_units(1.8),
                    geometry: vec![demand_region],
                    ..Default::default()
                },
            ],
            ..Default::default()
        },
    );
    assert!(!output.entries().is_empty());
    assert!(
        output.entries().iter().any(
            |entry| entry.demand_ids.contains(&"demand-41-0".to_string())
                && entry.demand_ids.contains(&"demand-42-0".to_string())
        ),
        "merged body must preserve both source demand IDs: {:?}",
        output
            .entries()
            .iter()
            .map(|entry| &entry.demand_ids)
            .collect::<Vec<_>>()
    );
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
        assert!(entry.skeleton.as_ref().is_some());
        // Every entry must carry printable geometry under some role. Since
        // packet 224 that role is not always `SupportBody`: canonical subtracts
        // roof and floor areas out of `base_areas`, so on a layer inside the
        // interface band the body can be fully carved away, leaving only
        // `TopInterface` or `BottomInterface`. Requiring `SupportBody` on every
        // entry would forbid that, which is why this assertion was widened.
        assert!(
            entry.roles.iter().any(|role| !role.regions.is_empty()),
            "entry at layer {} carries no printable geometry under any role: {:?}",
            entry.global_layer_index,
            entry.roles
        );
        let skeleton = entry.skeleton.as_ref().unwrap();
        assert!(!skeleton.points.is_empty());
    }
    assert!(output
        .entries()
        .iter()
        .flat_map(|entry| entry.roles.iter())
        .flat_map(|role| role.regions.iter())
        .any(|region| region.contour.points.len() >= 3));
    let mut layers: Vec<_> = output.entries().iter().collect();
    layers.sort_by_key(|entry| entry.global_layer_index);
    for pair in layers.windows(2) {
        if pair[1].global_layer_index != pair[0].global_layer_index + 1 {
            continue;
        }
        let lower = pair[0].skeleton.as_ref().unwrap();
        let upper = pair[1].skeleton.as_ref().unwrap();
        assert!(
            lower.points.iter().any(|a| {
                upper
                    .points
                    .iter()
                    .any(|b| ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt() <= 1.0)
            }),
            "adjacent body/interface layers must remain connected"
        );
    }
}

#[test]
fn disabled_and_declined() {
    let disabled = run_planner(
        true,
        overhang("disabled", 0.0, 0.0, 4.0),
        SupportGeometryView::default(),
    );
    assert!(!disabled.entries().is_empty());
    let disabled = run_planner(
        false,
        overhang("disabled", 0.0, 0.0, 4.0),
        SupportGeometryView::default(),
    );
    assert!(disabled.entries().is_empty());
    assert!(disabled.diagnostics().is_empty());

    let blocked = run_blocked_planner();
    assert!(blocked
        .entries()
        .iter()
        .any(|entry| entry.decline_reason == Some(SupportPlanDeclineReason::Blocked)));

    let declined = declined_entry(SupportPlanDeclineReason::NoRoute);
    assert_eq!(
        declined.decline_reason,
        Some(SupportPlanDeclineReason::NoRoute)
    );
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
fn invalid_body_rejected() {
    let occupancy = pillar_occupancy();
    let mut blocking = occupancy.clone();
    blocking.contour.points[1].x = slicer_ir::mm_to_units(5.0);
    blocking.contour.points[2].x = slicer_ir::mm_to_units(5.0);
    let radius = tapered_radius(20.0, 1.0, 100, 1.0);
    let output = run_planner(
        true,
        overhang("invalid", 0.0, 0.0, 4.0),
        SupportGeometryView {
            entries: (0..10)
                .map(|layer| SupportGeometryViewEntry {
                    global_support_layer_index: layer,
                    object_id: "invalid".into(),
                    region_id: "0".into(),
                    outlines: vec![blocking.clone()],
                })
                .collect(),
        },
    );

    // The complete footprint intersects exact-Z occupancy.  Rejection must
    // drop the complete body, rather than clipping it into a filler polygon.
    assert!(body_overlaps_occupancy(
        &[occupancy.clone()],
        1.0,
        10.0,
        radius
    ));
    assert!(output
        .diagnostics()
        .iter()
        .any(|d| d.code == 1203 && d.message.contains("complete radius")));
    assert!(
        output
            .entries()
            .iter()
            .flat_map(|entry| entry.roles.iter())
            .flat_map(|role| role.regions.iter())
            .all(|region| {
                let center =
                    region
                        .contour
                        .points
                        .iter()
                        .map(mm_point)
                        .fold((0.0, 0.0), |sum, point| {
                            (
                                sum.0 + point.0 / region.contour.points.len() as f32,
                                sum.1 + point.1 / region.contour.points.len() as f32,
                            )
                        });
                center.0 >= 5.0
            }),
        "no clipped or fallback body may remain for the colliding fixture"
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

    // The host gate owns routing-cell validation. This complete body crosses
    // the 1 << 20-unit cell boundary and must not be clipped or filled.
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
        entries: vec![SupportPlanEntry {
            global_layer_index: 0,
            object_id: "object-a".into(),
            region_id: 9,
            family_id: "tree".into(),
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
            provenance: vec!["tree-planner".into()],
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

#[test]
fn non_tree_family_candidates_are_skipped() {
    // A global `support_type = "normal(auto)"` selection resolves to the
    // traditional family. The tree planner must not plan candidates whose
    // resolved family is not "tree", even though its config-resolved
    // `support_family` would be "traditional".
    let mut values = HashMap::<ConfigKey, ConfigValue>::new();
    values.insert("enable_support".into(), ConfigValue::Bool(true));
    values.insert(
        "support_type".into(),
        ConfigValue::String("normal(auto)".into()),
    );
    values.insert("support_raft_layers".into(), ConfigValue::Int(0));
    let config = ConfigView::from_map(values);

    let planner = tree_support_planner::SupportPlanner::from_config(&config).expect("from_config");
    // Mesh-less object so only the analysis candidate path (not the legacy
    // mesh overhang path) contributes contacts.
    let object = MeshObjectView {
        object_id: "non-tree".into(),
        vertices: vec![],
        triangles: vec![],
        paint_layers: vec![],
    };
    let candidate_region = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(4.0, 0.0),
                Point2::from_mm(4.0, 4.0),
                Point2::from_mm(0.0, 4.0),
            ],
        },
        holes: vec![],
    };
    let mut output = SupportGeometryOutput::new();
    planner
        .run_support_geometry_with_analysis(
            &[object.clone()],
            &layer_plan(),
            &regions(&object.object_id),
            &SupportAnalysisView {
                candidates: vec![SupportAnalysisCandidate {
                    id: 1,
                    object_id: "non-tree".into(),
                    region_id: "0".into(),
                    global_layer_index: 8,
                    z_units: slicer_ir::mm_to_units(1.8),
                    geometry: vec![candidate_region],
                    ..Default::default()
                }],
                family_assignments: vec![SupportFamilyAssignment {
                    object_id: "non-tree".into(),
                    region_id: "0".into(),
                    family_id: "traditional".into(),
                }],
                ..Default::default()
            },
            &SupportGeometryView::default(),
            &mut output,
            &ConfigView::new(),
        )
        .expect("run_support_geometry_with_analysis");
    assert!(
        output.entries().is_empty(),
        "tree planner must not emit entries for a traditional-family candidate"
    );
}
