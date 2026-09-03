//! Contract tests for the tree support family seam.

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{ConfigKey, ConfigValue, ConfigView};
use slicer_ir::{
    ExPolygon, IndexedTriangleSet, MeshIR, ObjectMesh, Point2, Point3, Polygon,
    SupportPlanDeclineReason, SupportPlanEntry, SupportPlanIR, SupportPlanRole,
    SupportPlanRoleRegion, Transform3d,
};
use slicer_sdk::host::{self, ClipOperation};
use slicer_sdk::prepass_builders::SupportGeometryOutput;
use slicer_sdk::prepass_types::{
    LayerPlanView, LayerPlanViewEntry, MeshObjectView, RegionSegmentationView,
    RegionSegmentationViewEntry, SupportAnalysisCandidate, SupportAnalysisGeometryEntry,
    SupportAnalysisView, SupportFamilyAssignment, SupportGeometryView, SupportGeometryViewEntry,
};
use slicer_sdk::traits::PrepassModule;
use slicer_wasm_host::exact_z_query::ExactZQueryService;
use slicer_wasm_host::support_aggregation::{
    aggregate_declined_support_plans, aggregate_support_plan_irs_with_diagnostics,
};
use tree_support_planner::{
    body_overlaps_occupancy, build_roles, interface_adjusted_radius, tapered_radius,
};

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

fn planner_config_with_angle(branch_angle_deg: f64) -> ConfigView {
    // Packet 224 step 5 (F-13): canonical `DO_NOT_MOVER_UNDER_MM` is 5 mm for
    // the non-slim tree styles and 0 for slim. Below that `print_z` a branch is
    // forbidden to converge on its neighbours at all. This fixture is 2 mm tall
    // (10 layers at 0.2 mm), so under the default style NEITHER angle can
    // produce any convergence and the ordering the test asserts is vacuous —
    // both runs measure zero. Selecting the slim style is the only way to make
    // the fixture exercise convergence at all; the assertions themselves
    // (steeper angle converges further, and no layer step exceeds the
    // per-layer budget) are unchanged.
    planner_config_full_with(
        true,
        5.0,
        branch_angle_deg,
        &[("support_style", ConfigValue::String("tree_slim".into()))],
    )
}

fn planner_config_with_diameter(enabled: bool, branch_diameter: f64) -> ConfigView {
    planner_config_full(enabled, branch_diameter, 45.0)
}

fn planner_config_full(enabled: bool, branch_diameter: f64, branch_angle_deg: f64) -> ConfigView {
    planner_config_full_with(enabled, branch_diameter, branch_angle_deg, &[])
}

fn planner_config_full_with(
    enabled: bool,
    branch_diameter: f64,
    branch_angle_deg: f64,
    extra: &[(&str, ConfigValue)],
) -> ConfigView {
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
    values.insert(
        "tree_support_branch_angle".into(),
        ConfigValue::Float(branch_angle_deg),
    );
    for (key, value) in extra {
        values.insert((*key).into(), value.clone());
    }
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
    // exhaustive: support-plan identity fixture; SupportPlanEntry has no Default impl and FRU would let a new plan field default silently
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
    let top_geometry_layer = output
        .entries()
        .iter()
        .filter(|entry| entry.roles.iter().any(|role| !role.regions.is_empty()))
        .map(|entry| entry.global_layer_index)
        .max()
        .expect("planner must emit geometry on at least one layer");
    // Canonical `draw_circles` (`TreeSupport.cpp`) dispatches every node to
    // EXACTLY ONE bucket -- `roof_gap_areas`, else `roof_1st_layer` when
    // `support_roof_layers_below == 1`, else `roof_areas`/`roof_base_areas`
    // when `> 1`, else `base_areas` -- and only afterwards computes
    // `base_areas = diff_ex(base_areas, roofs)`. On a layer where every
    // surviving node is a roof node, canonical's `base_areas` is already empty
    // BEFORE the carve, so canonical prints no body cross-section there either.
    // The "body survives the carve" invariant therefore holds strictly BELOW
    // the top-interface band, not on it. The band bottom is derived from the
    // emitted plan (not from config) so the check tracks whatever band the
    // planner actually produced.
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
        assert_eq!(entry.family_id, "tree");
        assert!(!entry.demand_ids.is_empty());
        assert!(!entry.body_ids.is_empty());
        assert!(entry.skeleton.as_ref().is_some());
        if entry.global_layer_index < interface_band_bottom {
            // NOTE on regression scope: this fixture's single contact makes the
            // roof cover the whole branch on every band layer, so no layer here
            // is ever mixed and this test cannot gate the F-3 `carved.clear()`
            // defect. `anchored_heights_and_termination` (below) is that gate.
            //
            // Below the roof band no node is a roof node, so canonical's
            // `base_areas` is non-empty and survives `diff_ex(base_areas,
            // roofs)` intact. The widened `any non-empty role` form passed even
            // when the planner discarded the body on every interface layer.
            assert!(
                entry.roles.iter().any(|role| role.role == SupportPlanRole::SupportBody
                    && !role.regions.is_empty()),
                "entry at layer {} is below the top-interface band yet carries no SupportBody geometry. Canonical's `base_areas` is non-empty below the roof band and survives `diff_ex(base_areas, roofs)`, so the layer must still print a body cross-section. Roles: {:?}",
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
    // The contact seeded from an analysis candidate follows canonical
    // `generate_contact_points`: the node lands on `layer_nr - 1` as the
    // virtual top-Z-gap node (`distance_to_top < 0`), which `draw_circles`
    // diverts into `roof_gap_areas` and never extrudes. The first layer that
    // actually draws the contact set is therefore the topmost layer carrying
    // geometry, not the candidate's own index. Derive it from the plan so the
    // check tracks the emitted band rather than a pinned layer number.
    let contact_layer = top_geometry_layer;
    let mut classes = [0_u32; 3];
    for point in output
        .entries()
        .iter()
        .filter(|entry| entry.global_layer_index == contact_layer)
        .flat_map(|entry| entry.skeleton.as_ref().unwrap().points.iter())
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

/// The branch angle must actually reach the planner and must scale the
/// per-layer lateral move.
///
/// Canonical `TreeSupportData::get_max_move_dist` caps a node's XY travel per
/// layer at `min(tan(branch_angle) * node.height, support_extrusion_width)`, so
/// a LARGER angle permits a LARGER lateral step and a pair of branches
/// converges further over the same number of layers.
///
/// This is asserted as a relationship between two runs, not against a captured
/// number: only the ordering is canonical. It also guards F-21 — 19 test call
/// sites set `support_branch_angle_deg`, a key the planner stopped reading in
/// commit 4d1848eb, and all 19 set 45.0, which is `DEFAULT_BRANCH_ANGLE_DEG`,
/// so every one of them passed by coincidence. A test that never varies the
/// angle cannot notice that the key is dead.
#[test]
fn branch_angle_scales_the_per_layer_lateral_move() {
    fn spread_by_layer(output: &SupportGeometryOutput) -> std::collections::BTreeMap<i32, f32> {
        let mut bounds: std::collections::BTreeMap<i32, (f32, f32)> =
            std::collections::BTreeMap::new();
        for entry in output.entries() {
            let Some(skeleton) = entry.skeleton.as_ref() else {
                continue;
            };
            for point in &skeleton.points {
                let slot = bounds
                    .entry(entry.global_layer_index)
                    .or_insert((f32::INFINITY, f32::NEG_INFINITY));
                slot.0 = slot.0.min(point.x);
                slot.1 = slot.1.max(point.x);
            }
        }
        bounds
            .into_iter()
            .map(|(layer, (min_x, max_x))| (layer, max_x - min_x))
            .collect()
    }

    fn run_at_angle(angle_deg: f64) -> SupportGeometryOutput {
        let object = overhang("angle", 0.0, 0.0, 8.0);
        let planner = tree_support_planner::SupportPlanner::from_config(
            &planner_config_with_angle(angle_deg),
        )
        .expect("from_config");
        let mut output = SupportGeometryOutput::new();
        let region = |x0: f32, x1: f32| ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(x0, 0.0),
                    Point2::from_mm(x1, 0.0),
                    Point2::from_mm(x1, 2.0),
                    Point2::from_mm(x0, 2.0),
                ],
            },
            holes: vec![],
        };
        planner
            .run_support_geometry_with_analysis(
                &[object],
                &layer_plan(),
                &regions("angle"),
                &SupportAnalysisView {
                    candidates: vec![
                        SupportAnalysisCandidate {
                            id: 61,
                            object_id: "angle".into(),
                            region_id: "0".into(),
                            global_layer_index: 8,
                            z_units: slicer_ir::mm_to_units(1.8),
                            geometry: vec![region(0.0, 2.0)],
                            ..Default::default()
                        },
                        SupportAnalysisCandidate {
                            id: 62,
                            object_id: "angle".into(),
                            region_id: "0".into(),
                            global_layer_index: 8,
                            z_units: slicer_ir::mm_to_units(1.8),
                            geometry: vec![region(6.0, 8.0)],
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                },
                &SupportGeometryView::default(),
                &mut output,
                &ConfigView::new(),
            )
            .expect("run_support_geometry_with_analysis");
        output
    }

    let shallow = run_at_angle(30.0);
    let steep = run_at_angle(60.0);
    let shallow_spread = spread_by_layer(&shallow);
    let steep_spread = spread_by_layer(&steep);
    assert!(
        shallow_spread.len() >= 2 && steep_spread.len() >= 2,
        "both runs must plan multiple layers of skeleton before the angle can be compared; shallow={shallow_spread:?} steep={steep_spread:?}"
    );

    let convergence = |spread: &std::collections::BTreeMap<i32, f32>| -> f32 {
        let (_, top) = spread.iter().next_back().expect("non-empty");
        let (_, bottom) = spread.iter().next().expect("non-empty");
        top - bottom
    };
    let shallow_convergence = convergence(&shallow_spread);
    let steep_convergence = convergence(&steep_spread);
    assert!(
        steep_convergence > shallow_convergence,
        "a larger branch angle must permit a larger per-layer lateral move (canonical          `get_max_move_dist` = min(tan(angle) * height, extrusion_width)), so branches at 60 deg          must converge further than at 30 deg over the same layers. Got 60deg={steep_convergence}          vs 30deg={shallow_convergence}; spreads 60deg={steep_spread:?} 30deg={shallow_spread:?}"
    );

    // The cap is an upper bound as well as an ordering: no single layer step may
    // exceed `tan(angle) * layer_height * wall_count` per node, so the spread
    // (two nodes closing on each other) may shrink by at most twice that.
    for (angle_deg, spread) in [(30.0_f32, &shallow_spread), (60.0_f32, &steep_spread)] {
        let budget = 2.0 * angle_deg.to_radians().tan() * 0.2 * 1.0 + 1e-3;
        let layers: Vec<(&i32, &f32)> = spread.iter().collect();
        for pair in layers.windows(2) {
            let ((lower, below), (upper, above)) = (pair[0], pair[1]);
            if *upper != *lower + 1 {
                continue;
            }
            assert!(
                above - below <= budget,
                "branch closed {} mm of lateral distance between layers {lower} and {upper} at                  {angle_deg} deg, exceeding the canonical per-layer budget {budget} mm                  (tan(angle) * layer_height * wall_count, doubled for two converging nodes)",
                above - below
            );
        }
    }
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
    // Packet 238b AC-8/Q10: contacts inside the xy-inflated collision volume
    // are pruned during seeding, before emit-time 1203 diagnostics. The
    // remaining non-colliding overhang must still pass the geometry checks
    // below, proving safety without asserting the retired warning path.
    assert!(
        !output.diagnostics().iter().any(|d| d.code == 1203),
        "seeded-pruned contacts must not reach the emit-time 1203 path: {:?}",
        output.diagnostics()
    );
    let mut emitted_survivor = false;
    for body in output
        .entries()
        .iter()
        .flat_map(|entry| entry.roles.iter())
        .flat_map(|role| role.regions.iter())
    {
        // Vertex count is no longer 16. Packet 224 step 6 (F-33) makes the
        // emit pass draw canonical `draw_circles`' per-node ellipse, whose
        // vertex count is `CIRCLE_RESOLUTION` (100 here, 4 above 200 nodes
        // per layer) before the closing distance-tolerance simplify. Pinning
        // the count would pin this port's pre-canonical 16-gon capsule; what
        // this test is actually about is the *radius*, asserted below.
        let n = body.contour.points.len();
        assert!(n >= 3, "degenerate emitted region with {n} vertices");
        let center = body
            .contour
            .points
            .iter()
            .map(mm_point)
            .fold((0.0, 0.0), |sum, point| {
                (sum.0 + point.0 / n as f32, sum.1 + point.1 / n as f32)
            });
        let radii: Vec<f32> = body
            .contour
            .points
            .iter()
            .map(|point| distance(mm_point(point), center))
            .collect();
        let local_radius = radii.iter().copied().fold(f32::INFINITY, f32::min);
        // Structural regions are now discrete per-node footprints rather than
        // one MST-capsule union. Keep the non-degenerate radius guard, while
        // measuring each footprint independently instead of its old slab waist.
        //
        assert!(
            local_radius >= 0.1,
            "node footprint became degenerate: {local_radius} mm"
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
fn base_interface_band_attributed_in_plan_roles() {
    let object = two_overhangs("base-band");
    let config = planner_config_full_with(
        true,
        5.0,
        30.0,
        &[("num_top_base_interface_layers", ConfigValue::Int(2))],
    );
    let planner = tree_support_planner::SupportPlanner::from_config(&config).unwrap();
    let mut output = SupportGeometryOutput::new();
    planner
        .run_support_geometry(
            &[object.clone()],
            &layer_plan(),
            &regions(&object.object_id),
            &SupportGeometryView::default(),
            &mut output,
            &ConfigView::new(),
        )
        .unwrap();
    assert!(output.entries().iter().any(|entry| {
        entry
            .roles
            .iter()
            .any(|role| role.role == SupportPlanRole::BaseInterface && !role.regions.is_empty())
    }));
    for entry in output.entries() {
        let roles: Vec<_> = entry
            .roles
            .iter()
            .filter(|role| !role.regions.is_empty())
            .map(|role| role.role)
            .collect();
        assert!(
            roles
                .iter()
                .filter(|role| **role == SupportPlanRole::BaseInterface)
                .count()
                <= 1
        );
        assert!(
            !(roles.contains(&SupportPlanRole::BaseInterface)
                && roles.contains(&SupportPlanRole::TopInterface))
        );
        assert!(
            !(roles.contains(&SupportPlanRole::BaseInterface)
                && roles.contains(&SupportPlanRole::SupportBody))
        );
    }
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
    let top_geometry_layer = output
        .entries()
        .iter()
        .filter(|entry| entry.roles.iter().any(|role| !role.regions.is_empty()))
        .map(|entry| entry.global_layer_index)
        .max()
        .expect("planner must emit geometry on at least one layer");
    for entry in output.entries() {
        assert_eq!(
            entry.anchor_z,
            slicer_ir::mm_to_units((entry.global_layer_index as f32 + 1.0) * 0.2)
        );
        assert_eq!(entry.anchor_layer_index, entry.global_layer_index as u32);
        assert!(entry.skeleton.as_ref().is_some());
        // Canonical subtracts roof and floor areas out of `base_areas` and
        // KEEPS the remainder (`draw_circles`, `TreeSupport.cpp`). On THIS
        // fixture the contact area is narrower than the branch, so the roof
        // never covers the whole cross-section and `diff_ex(base_areas, roofs)`
        // always leaves a remainder: layers 6 and 5 are mixed (SupportBody +
        // TopInterface). That makes this the F-3 regression gate -- the
        // pre-fix `carved.clear()` dropped the body on every roof-carrying
        // layer and turned this assertion red. Do NOT narrow it to
        // "below the interface band"; that would exempt exactly the two mixed
        // layers the gate exists to protect.
        if entry.global_layer_index < top_geometry_layer {
            assert!(
                entry.roles.iter().any(|role| role.role == SupportPlanRole::SupportBody
                    && !role.regions.is_empty()),
                "entry at layer {} carries no SupportBody geometry. Canonical carves roof/floor out of `base_areas` and KEEPS the remainder; on this fixture the roof never covers the whole branch, so every layer below the top of the column still prints a body cross-section. Roles: {:?}",
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
        let skeleton = entry.skeleton.as_ref().unwrap();
        assert!(!skeleton.points.is_empty());
        assert_eq!(
            skeleton.wall_counts.len(),
            skeleton.points.len(),
            "wall-count carrier must stay positional"
        );
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
fn enabled_independent_height_produces_free_floating_anchor_z() {
    // Packet 239c AC-2: canonical enabled semantics — free-floating
    // `anchor_z`. With `independent_support_layer_height` enabled (the
    // default) and a support pitch finer than the object layer height, at
    // least one emitted `SupportPlanEntry.anchor_z` differs from
    // `mm_to_units(layer_plan.layers[..].z)` by more than
    // `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS`, planes are
    // distinct and strictly ordered per object, and the intermediate planes
    // follow the canonical `generate_support_layers` stepping.
    //
    // Fixture: a 10-layer 0.2 mm plan (1.8 mm tall tree contact at layer 8)
    // with a 0.1 mm support pitch. Every adjacent pair of support rows
    // (0.2 mm apart) admits n = ceil((0.2 - 1e-4)/0.1) = 2 canonical rows,
    // so one strictly-between plane per adjacent pair is expected.
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
    let analysis = SupportAnalysisView {
        candidates: vec![SupportAnalysisCandidate {
            id: 51,
            object_id: "indep".into(),
            region_id: "0".into(),
            global_layer_index: 8,
            z_units: slicer_ir::mm_to_units(1.8),
            geometry: vec![demand_region],
            ..Default::default()
        }],
        ..Default::default()
    };
    // The default planner already carries `independent_support_layer_height
    // = true`; state the pitch explicitly through the config.
    let planner = tree_support_planner::SupportPlanner::from_config(&planner_config_full_with(
        true,
        5.0,
        45.0,
        &[("support_layer_height_mm", ConfigValue::Float(0.1))],
    ))
    .expect("from_config");
    let mut output = SupportGeometryOutput::new();
    planner
        .run_support_geometry_with_analysis(
            &[overhang("indep", 0.0, 0.0, 4.0)],
            &layer_plan(),
            &regions("indep"),
            &analysis,
            &SupportGeometryView::default(),
            &mut output,
            &ConfigView::new(),
        )
        .expect("run_support_geometry_with_analysis");
    assert!(
        !output.entries().is_empty(),
        "tree contact must produce plan entries"
    );
    let tolerance = slicer_ir::AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS;
    let off_grid: Vec<i64> = output
        .entries()
        .iter()
        .filter(|entry| {
            let grid_z = layer_plan().layers[entry.anchor_layer_index.min(9) as usize].z;
            entry.anchor_z.abs_diff(slicer_ir::mm_to_units(grid_z)) > tolerance as u64
        })
        .map(|entry| entry.anchor_z)
        .collect();
    assert!(
        !off_grid.is_empty(),
        "enabled branch must produce at least one off-grid anchor_z \
         (>{tolerance} units from its object layer's Z); got {:?}",
        output
            .entries()
            .iter()
            .map(|e| (e.global_layer_index, e.anchor_z))
            .collect::<Vec<_>>()
    );
    // Intermediate planes follow the canonical stepping: with a 0.2 mm gap
    // and a 0.1 mm pitch every off-grid plane sits at bottom_z + 0.1 mm for
    // some adjacent pair of grid rows.
    let grid: std::collections::BTreeSet<i64> = layer_plan()
        .layers
        .iter()
        .map(|layer| slicer_ir::mm_to_units(layer.z))
        .collect();
    for plane in &off_grid {
        let between = grid
            .range((plane + 1)..)
            .next()
            .copied()
            .expect("an off-grid plane must sit below some grid plane");
        let step = between - plane;
        assert!(
            plane + step == between && grid.contains(&between),
            "off-grid plane {plane} must sit one canonical step below grid plane {between}"
        );
    }
    // Distinct and strictly increasing when ordered by plane.
    let mut planes: Vec<i64> = output.entries().iter().map(|e| e.anchor_z).collect();
    planes.sort_unstable();
    planes.dedup();
    let distinct_ordered = planes.len() == output.entries().len();
    assert!(
        distinct_ordered,
        "declared planes must be distinct; got {planes:?}"
    );
}

fn run_multi_layer_demand_on_plan(
    pitch_mm: f64,
    layer_plan: &LayerPlanView,
) -> SupportGeometryOutput {
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
    let analysis = SupportAnalysisView {
        candidates: vec![
            SupportAnalysisCandidate {
                id: 81,
                object_id: "coarse".into(),
                region_id: "0".into(),
                global_layer_index: layer_plan.layers[3].global_layer_index,
                z_units: slicer_ir::mm_to_units(layer_plan.layers[3].z),
                geometry: vec![demand_region.clone()],
                ..Default::default()
            },
            SupportAnalysisCandidate {
                id: 82,
                object_id: "coarse".into(),
                region_id: "0".into(),
                global_layer_index: layer_plan.layers[8].global_layer_index,
                z_units: slicer_ir::mm_to_units(layer_plan.layers[8].z),
                geometry: vec![demand_region],
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let planner = tree_support_planner::SupportPlanner::from_config(&planner_config_full_with(
        true,
        5.0,
        45.0,
        &[("support_layer_height_mm", ConfigValue::Float(pitch_mm))],
    ))
    .expect("from_config");
    let mut output = SupportGeometryOutput::new();
    let segmentation = RegionSegmentationView {
        entries: layer_plan
            .layers
            .iter()
            .map(|layer| RegionSegmentationViewEntry {
                object_id: "coarse".into(),
                layer_index: layer.global_layer_index,
                region_ids: vec!["0".into()],
            })
            .collect(),
        region_support_configs: vec![],
    };
    planner
        .run_support_geometry_with_analysis(
            &[overhang("coarse", 0.0, 0.0, 4.0)],
            layer_plan,
            &segmentation,
            &analysis,
            &SupportGeometryView::default(),
            &mut output,
            &ConfigView::new(),
        )
        .expect("run_support_geometry_with_analysis");
    output
}

#[test]
fn coarse_synthesized_rows_use_height_local_geometry() {
    let output = run_multi_layer_demand_on_plan(0.3, &layer_plan());
    let synthesized: Vec<_> = output
        .entries()
        .iter()
        .filter(|entry| entry.global_layer_index < 0)
        .collect();
    let anchors: Vec<_> = synthesized.iter().map(|entry| entry.anchor_z).collect();

    assert!(
        synthesized.len() >= 2,
        "coarse demand must emit at least two synthesized rows; anchors: {anchors:?}"
    );
    assert!(
        synthesized.iter().all(|entry| entry.skeleton.is_some()),
        "coarse synthesized rows must retain skeleton geometry; anchors: {anchors:?}"
    );
    assert!(
        synthesized
            .windows(2)
            .any(|pair| pair[0].skeleton != pair[1].skeleton),
        "coarse synthesized rows must use height-local skeleton geometry; anchors: {anchors:?}"
    );
}

#[test]
fn coarse_pitch_produces_free_floating_anchor_z() {
    let mut non_dense_identity_plan = layer_plan();
    for (layer, global_layer_index) in non_dense_identity_plan
        .layers
        .iter_mut()
        .zip([40, 7, 81, 3, 55, 13, 99, 21, 1, 34])
    {
        layer.global_layer_index = global_layer_index;
    }
    let output = run_multi_layer_demand_on_plan(0.3, &non_dense_identity_plan);
    assert!(
        !output.entries().is_empty(),
        "tree demand must produce entries"
    );
    let tolerance = slicer_ir::AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS;
    assert!(
        output.entries().iter().any(|entry| {
            entry.anchor_z.abs_diff(slicer_ir::mm_to_units(
                non_dense_identity_plan.layers[entry.anchor_layer_index.min(9) as usize].z,
            )) > tolerance as u64
        }),
        "coarse pitch must produce an off-grid anchor_z"
    );
    let planes: Vec<i64> = output
        .entries()
        .iter()
        .map(|entry| entry.anchor_z)
        .collect();
    assert_eq!(
        planes,
        vec![2000, 5000, 8000, 11000, 14000],
        "tree-family ceil(dist / pitch) planes must be emitted in original order"
    );
    assert!(
        planes.windows(2).all(|pair| pair[0] < pair[1]),
        "coarse support planes must be strictly increasing: {planes:?}"
    );
    assert_eq!(
        output
            .entries()
            .iter()
            .map(|entry| (entry.anchor_z, entry.anchor_layer_index))
            .collect::<Vec<_>>(),
        vec![(2000, 0), (5000, 1), (8000, 3), (11000, 4), (14000, 6)],
        "anchor_layer_index must be positional, true-nearest, and choose the lower index on ties"
    );
    let fallback = run_near_distinct_interface_fixture();
    assert_eq!(
        fallback
            .entries()
            .iter()
            .map(|entry| entry.anchor_z)
            .collect::<Vec<_>>(),
        vec![2000, 5000, 8000, 11000, 13999, 14000],
        "AC-2 endpoint fallback must preserve the exact ordered plane sequence"
    );
    assert_eq!(
        fallback
            .entries()
            .iter()
            .filter(|entry| entry.global_layer_index < 0)
            .map(|entry| entry.anchor_z)
            .collect::<Vec<_>>(),
        vec![5000, 8000, 11000],
        "AC-2 endpoint fallback must emit the exact off-grid planes"
    );
    assert_eq!(
        fallback
            .entries()
            .iter()
            .filter(|entry| entry
                .roles
                .iter()
                .any(|role| role.role == SupportPlanRole::TopInterface && !role.regions.is_empty()))
            .map(|entry| entry.anchor_z)
            .collect::<Vec<_>>(),
        vec![13999, 14000],
        "AC-2 endpoint fallback must retain both protected interface planes"
    );
    for entry in output.entries() {
        let is_interface_bracket = entry.roles.iter().any(|role| {
            matches!(
                role.role,
                SupportPlanRole::TopInterface
                    | SupportPlanRole::BaseInterface
                    | SupportPlanRole::BottomInterface
            )
        });
        assert!(
            (is_interface_bracket
                && entry.roles.iter().any(|role| {
                    matches!(
                        role.role,
                        SupportPlanRole::TopInterface
                            | SupportPlanRole::BaseInterface
                            | SupportPlanRole::BottomInterface
                    )
                }))
                || (!is_interface_bracket
                    && entry
                        .roles
                        .iter()
                        .all(|role| role.role == SupportPlanRole::SupportBody)),
            "coarse stack plane {} retained an interface role: {:?}",
            entry.anchor_z,
            entry.roles
        );
        let expected_anchor = non_dense_identity_plan
            .layers
            .iter()
            .enumerate()
            .min_by_key(|(index, layer)| {
                (
                    entry.anchor_z.abs_diff(slicer_ir::mm_to_units(layer.z)),
                    *index,
                )
            })
            .map(|(index, _)| index as u32)
            .unwrap();
        assert_eq!(
            entry.anchor_layer_index, expected_anchor,
            "plane {} must use the true-nearest object layer",
            entry.anchor_z
        );
    }

    // This second production-path fixture has interface planes at 0.8 and
    // 1.6 mm with a surviving body row between them. The expected 1.2 mm
    // plane distinguishes the body-bearing interface-span brackets from the
    // endpoint fallback, which would start at the run's 0.2 mm row.
    let body_span = run_mixed_source_tree_fixture();
    let interface_planes: std::collections::BTreeSet<i64> = body_span
        .entries()
        .iter()
        .filter(|entry| {
            entry.roles.iter().any(|role| {
                matches!(
                    role.role,
                    SupportPlanRole::TopInterface
                        | SupportPlanRole::BaseInterface
                        | SupportPlanRole::BottomInterface
                ) && !role.regions.is_empty()
            })
        })
        .map(|entry| entry.anchor_z)
        .collect();
    assert_eq!(
        interface_planes,
        std::collections::BTreeSet::from([8000, 16000]),
        "body-bearing interface span must use its two interface planes as brackets"
    );
    assert!(body_span.entries().iter().any(|entry| {
        entry.anchor_z == 12000
            && entry.global_layer_index < 0
            && entry
                .roles
                .iter()
                .all(|role| role.role == SupportPlanRole::SupportBody)
    }));
}

fn run_near_distinct_interface_fixture() -> SupportGeometryOutput {
    let object = overhang("near-interface", 0.0, 0.0, 4.0);
    let planner = tree_support_planner::SupportPlanner::from_config(&planner_config_full_with(
        true,
        5.0,
        30.0,
        &[
            ("support_layer_height_mm", ConfigValue::Float(0.3)),
            ("support_interface_top_layers", ConfigValue::Int(2)),
        ],
    ))
    .unwrap();
    let mut near_plan = layer_plan();
    near_plan.layers[4].z = 1.1;
    near_plan.layers[5].z = 1.3999;
    near_plan.layers[6].z = 1.4;
    let mut output = SupportGeometryOutput::new();
    planner
        .run_support_geometry(
            &[object.clone()],
            &near_plan,
            &regions(&object.object_id),
            &SupportGeometryView::default(),
            &mut output,
            &ConfigView::new(),
        )
        .unwrap();
    output
}

#[test]
fn coarse_pitch_preserves_lone_interface_bracket() {
    let object = overhang("one-interface", 0.0, 0.0, 4.0);
    let planner = tree_support_planner::SupportPlanner::from_config(&planner_config_full_with(
        true,
        5.0,
        30.0,
        &[
            ("support_layer_height_mm", ConfigValue::Float(0.3)),
            ("support_interface_top_layers", ConfigValue::Int(1)),
        ],
    ))
    .unwrap();
    let mut output = SupportGeometryOutput::new();
    planner
        .run_support_geometry(
            &[object.clone()],
            &layer_plan(),
            &regions(&object.object_id),
            &SupportGeometryView::default(),
            &mut output,
            &ConfigView::new(),
        )
        .unwrap();
    let interface_entries: Vec<_> = output
        .entries()
        .iter()
        .filter(|entry| {
            entry.roles.iter().any(|role| {
                matches!(
                    role.role,
                    SupportPlanRole::TopInterface
                        | SupportPlanRole::BaseInterface
                        | SupportPlanRole::BottomInterface
                )
            })
        })
        .collect();
    let mut interface_planes: Vec<_> = interface_entries
        .iter()
        .map(|entry| entry.anchor_z)
        .collect();
    interface_planes.sort_unstable();
    interface_planes.dedup();
    assert_eq!(
        interface_planes.len(),
        1,
        "the fixture must exercise Q1's one-interface-plane bracket case"
    );
    let planes: Vec<i64> = output
        .entries()
        .iter()
        .map(|entry| entry.anchor_z)
        .collect();
    assert_eq!(
        planes,
        vec![2000, 5000, 8000, 11000, 14000],
        "lone interface bracket must preserve the original coarse stack order and multiplicity"
    );
    let interface_index = planes
        .iter()
        .position(|plane| *plane == interface_planes[0])
        .expect("lone interface must remain in the emitted stack");
    assert!(
        interface_index > 0,
        "lone interface must have a synthesized plane below it"
    );
    assert_eq!(
        planes[interface_index] - planes[interface_index - 1],
        3000,
        "the synthesized stack must reach the lone interface bracket"
    );
    assert!(output.entries()[interface_index]
        .roles
        .iter()
        .any(|role| role.role == SupportPlanRole::TopInterface));
    assert!(output.entries()[interface_index - 1]
        .roles
        .iter()
        .all(|role| role.role == SupportPlanRole::SupportBody));
    assert!(interface_entries.iter().all(|entry| entry
        .roles
        .iter()
        .any(|role| role.role == SupportPlanRole::TopInterface)));
}

#[test]
fn near_distinct_interface_planes_count_separately() {
    let output = run_near_distinct_interface_fixture();

    let interface_sequence: Vec<_> = output
        .entries()
        .iter()
        .filter(|entry| {
            entry.roles.iter().any(|role| {
                matches!(
                    role.role,
                    SupportPlanRole::TopInterface
                        | SupportPlanRole::BaseInterface
                        | SupportPlanRole::BottomInterface
                )
            })
        })
        .map(|entry| entry.anchor_z)
        .collect();
    assert!(
        interface_sequence == vec![13999, 14000],
        "fixture must retain both near-distinct interface planes in source order: {interface_sequence:?}"
    );
    let synthesized: Vec<_> = output
        .entries()
        .iter()
        .filter(|entry| entry.global_layer_index < 0)
        .map(|entry| entry.anchor_z)
        .collect();
    assert_eq!(
        synthesized,
        vec![5000, 8000, 11000],
        "adjacent interface planes with no interior body row must use the run endpoint fallback"
    );
    let planes: Vec<_> = output
        .entries()
        .iter()
        .map(|entry| entry.anchor_z)
        .collect();
    assert_eq!(
        planes,
        vec![2000, 5000, 8000, 11000, 13999, 14000],
        "endpoint fallback must emit the exact coarse stack while retaining both adjacent interface planes: {planes:?}"
    );
    assert_eq!(
        output
            .entries()
            .iter()
            .filter(|entry| entry
                .roles
                .iter()
                .any(|role| role.role == SupportPlanRole::TopInterface))
            .map(|entry| entry.anchor_z)
            .collect::<Vec<_>>(),
        vec![13999, 14000],
        "both canonical interface brackets must retain TopInterface roles"
    );
    assert!(output
        .entries()
        .iter()
        .filter(|entry| entry.global_layer_index < 0)
        .all(|entry| entry
            .roles
            .iter()
            .all(|role| role.role == SupportPlanRole::SupportBody)));
}

#[test]
fn zero_pitch_sentinel_stays_object_grid() {
    let adaptive_plan = LayerPlanView {
        layers: (0..10)
            .map(|i| LayerPlanViewEntry {
                global_layer_index: i,
                z: 0.2 + i as f32 * 0.3,
                effective_layer_height: if i == 0 { 0.2 } else { 0.3 },
            })
            .collect(),
    };
    let output = run_multi_layer_demand_on_plan(0.0, &adaptive_plan);
    assert!(
        !output.entries().is_empty(),
        "tree demand must produce entries"
    );
    let tolerance = slicer_ir::AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS;
    assert!(
        output.entries().iter().all(|entry| {
            entry.anchor_z.abs_diff(slicer_ir::mm_to_units(
                adaptive_plan.layers[entry.anchor_layer_index.min(9) as usize].z,
            )) <= tolerance as u64
        }),
        "zero pitch sentinel must keep every anchor on the object grid"
    );
    assert_eq!(
        output
            .entries()
            .iter()
            .map(|entry| entry.anchor_z)
            .collect::<Vec<_>>(),
        vec![20000, 17000, 14000, 11000, 8000, 5000, 2000],
        "zero pitch sentinel must preserve the original object-grid order and multiplicity"
    );
}

#[test]
fn adaptive_local_gap_stays_finer() {
    let adaptive_plan = LayerPlanView {
        layers: (0..10)
            .map(|i| LayerPlanViewEntry {
                global_layer_index: i,
                z: 0.2 + i as f32 * 0.3,
                effective_layer_height: if i == 0 { 0.2 } else { 0.3 },
            })
            .collect(),
    };
    let output = run_multi_layer_demand_on_plan(0.2, &adaptive_plan);
    let planes: Vec<i64> = output
        .entries()
        .iter()
        .map(|entry| entry.anchor_z)
        .collect();
    assert_eq!(
        planes,
        vec![20000, 17000, 14000, 11000, 8000, 5000, 2000, 3500, 6500, 9500, 12500, 15500, 18500,],
        "adaptive finer output must preserve exact order and multiplicity"
    );
    for coarse_only in [4000, 6000, 10000, 12000, 16000, 18000] {
        assert!(
            !planes.contains(&coarse_only),
            "adaptive finer output must not contain coarse-only plane {coarse_only}: {planes:?}"
        );
    }
}

#[test]
fn finer_same_region_multi_source_preserves_lower_entry_identity() {
    let output = run_multi_layer_demand_on_plan(0.1, &layer_plan());
    let grid: std::collections::BTreeSet<_> = layer_plan()
        .layers
        .iter()
        .map(|layer| slicer_ir::mm_to_units(layer.z))
        .collect();
    let finer: Vec<_> = output
        .entries()
        .iter()
        .filter(|entry| !grid.contains(&entry.anchor_z))
        .collect();
    assert!(!finer.is_empty());
    for entry in finer {
        let lower = output
            .entries()
            .iter()
            .filter(|candidate| {
                candidate.region_id == entry.region_id
                    && grid.contains(&candidate.anchor_z)
                    && candidate.anchor_z < entry.anchor_z
            })
            .max_by_key(|candidate| candidate.anchor_z)
            .expect("same-region lower source row");
        assert_eq!(entry.demand_ids, lower.demand_ids);
        assert_eq!(entry.body_ids, lower.body_ids);
        assert_eq!(entry.roles, lower.roles);
    }
}

#[test]
fn staggered_region_runs_do_not_pair_across_region_boundaries() {
    let object = overhang("staggered-regions", 0.0, 0.0, 4.0);
    let mut adaptive_plan = layer_plan();
    for (index, layer) in adaptive_plan.layers.iter_mut().enumerate() {
        layer.z = if index <= 4 {
            0.2 + index as f32 * 0.2
        } else {
            1.0 + (index - 4) as f32 * 0.3
        };
        layer.effective_layer_height = if index <= 4 { 0.2 } else { 0.3 };
    }
    let candidate_geometry = ExPolygon {
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
    let analysis = SupportAnalysisView {
        candidates: vec![
            SupportAnalysisCandidate {
                id: 91,
                object_id: object.object_id.clone(),
                region_id: "0".into(),
                global_layer_index: 4,
                z_units: slicer_ir::mm_to_units(adaptive_plan.layers[4].z),
                geometry: vec![candidate_geometry.clone()],
                ..Default::default()
            },
            SupportAnalysisCandidate {
                id: 92,
                object_id: object.object_id.clone(),
                region_id: "1".into(),
                global_layer_index: 8,
                z_units: slicer_ir::mm_to_units(adaptive_plan.layers[8].z),
                geometry: vec![ExPolygon {
                    contour: Polygon {
                        points: vec![
                            Point2::from_mm(2.0, 1.0),
                            Point2::from_mm(2.2, 1.0),
                            Point2::from_mm(2.2, 1.2),
                            Point2::from_mm(2.0, 1.2),
                        ],
                    },
                    holes: vec![],
                }],
                ..Default::default()
            },
        ],
        // Let the planner's native tree fallback assign only regions present in
        // segmentation; explicit assignments would stamp one region's template
        // into every layer of the other region.
        family_assignments: vec![],
        ..Default::default()
    };
    let segmentation = RegionSegmentationView {
        entries: adaptive_plan
            .layers
            .iter()
            .enumerate()
            .map(|(index, layer)| RegionSegmentationViewEntry {
                object_id: object.object_id.clone(),
                layer_index: layer.global_layer_index,
                region_ids: if index < 3 {
                    vec!["0".into()]
                } else if index <= 4 {
                    vec!["0".into(), "1".into()]
                } else {
                    vec!["1".into()]
                },
            })
            .collect(),
        region_support_configs: vec![],
    };
    let planner = tree_support_planner::SupportPlanner::from_config(&planner_config_full_with(
        true,
        5.0,
        45.0,
        &[("support_layer_height_mm", ConfigValue::Float(0.2))],
    ))
    .unwrap();
    let mut output = SupportGeometryOutput::new();
    planner
        .run_support_geometry_with_analysis(
            &[object],
            &adaptive_plan,
            &segmentation,
            &analysis,
            &SupportGeometryView::default(),
            &mut output,
            &ConfigView::new(),
        )
        .unwrap();

    let region_zero: Vec<_> = output
        .entries()
        .iter()
        .filter(|entry| entry.region_id == "0")
        .collect();
    assert!(
        !region_zero.is_empty(),
        "region zero must be assigned and emitted"
    );
    let region_zero_planes: Vec<_> = region_zero.iter().map(|entry| entry.anchor_z).collect();
    let region_one: Vec<_> = output
        .entries()
        .iter()
        .filter(|entry| entry.region_id == "1")
        .collect();
    assert!(
        !region_one.is_empty(),
        "staggered fixture must retain region one"
    );
    let region_one_planes: Vec<_> = region_one.iter().map(|entry| entry.anchor_z).collect();
    assert_eq!(region_zero_planes, vec![2000, 4000, 6000, 8000, 10000]);
    assert_eq!(
        region_one_planes,
        vec![8000, 10000, 11500, 13000, 14500, 16000]
    );
    assert_ne!(region_zero_planes, region_one_planes);
    let region_zero_synthesized: Vec<_> = region_zero
        .iter()
        .filter(|entry| entry.global_layer_index < 0)
        .map(|entry| entry.anchor_z)
        .collect();
    let region_one_synthesized: Vec<_> = region_one
        .iter()
        .filter(|entry| entry.global_layer_index < 0)
        .map(|entry| entry.anchor_z)
        .collect();
    assert_ne!(region_zero_synthesized, region_one_synthesized);
}

#[test]
fn coarse_same_region_sources_keep_geometry_and_membership() {
    let object = overhang("same-region-coarse", 0.0, 0.0, 4.0);
    let square = |x: f32| ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(x, 1.0),
                Point2::from_mm(x + 2.0, 1.0),
                Point2::from_mm(x + 2.0, 3.0),
                Point2::from_mm(x, 3.0),
            ],
        },
        holes: vec![],
    };
    let analysis = SupportAnalysisView {
        candidates: vec![
            SupportAnalysisCandidate {
                id: 101,
                object_id: object.object_id.clone(),
                region_id: "0".into(),
                global_layer_index: 3,
                z_units: slicer_ir::mm_to_units(0.8),
                geometry: vec![square(-100.0)],
                ..Default::default()
            },
            SupportAnalysisCandidate {
                id: 102,
                object_id: object.object_id.clone(),
                region_id: "0".into(),
                global_layer_index: 5,
                z_units: slicer_ir::mm_to_units(1.2),
                geometry: vec![square(0.0)],
                ..Default::default()
            },
            SupportAnalysisCandidate {
                id: 103,
                object_id: object.object_id.clone(),
                region_id: "0".into(),
                global_layer_index: 8,
                z_units: slicer_ir::mm_to_units(1.8),
                geometry: vec![square(100.0)],
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let planner = tree_support_planner::SupportPlanner::from_config(&planner_config_full_with(
        true,
        5.0,
        45.0,
        &[
            ("support_layer_height_mm", ConfigValue::Float(0.3)),
            ("support_interface_top_layers", ConfigValue::Int(1)),
        ],
    ))
    .unwrap();
    let mut output = SupportGeometryOutput::new();
    planner
        .run_support_geometry_with_analysis(
            &[object],
            &layer_plan(),
            &regions("same-region-coarse"),
            &analysis,
            &SupportGeometryView::default(),
            &mut output,
            &ConfigView::new(),
        )
        .unwrap();
    let synthesized: Vec<_> = output
        .entries()
        .iter()
        .filter(|entry| entry.global_layer_index < 0)
        .collect();
    assert_eq!(
        output
            .entries()
            .iter()
            .map(|entry| entry.anchor_z)
            .collect::<Vec<_>>(),
        vec![2000, 4000, 6000, 8000, 11000, 14000],
        "coarse rows must retain original order and multiplicity"
    );
    assert_eq!(
        synthesized.len(),
        2,
        "each bracket pair must produce exactly one synthesized interior plane"
    );
    assert_eq!(
        synthesized
            .iter()
            .map(|entry| (entry.anchor_z, entry.body_ids.clone()))
            .collect::<Vec<_>>(),
        vec![
            (6000, vec!["tree-body-same-region-coarse-1".to_string()]),
            (11000, vec!["tree-body-same-region-coarse-4".to_string()]),
        ],
        "each coarse plane must use the nearest same-region source layer at or below it"
    );
    assert_ne!(
        synthesized[0].body_ids, synthesized[1].body_ids,
        "same-region source layers must retain distinct body membership"
    );
    assert_ne!(
        synthesized[0].skeleton, synthesized[1].skeleton,
        "same-region source layers must retain distinct skeleton geometry"
    );
    assert_ne!(
        synthesized[0].roles[0].regions, synthesized[1].roles[0].regions,
        "same-region source layers must retain distinct contour geometry"
    );
    for synthesized in &synthesized {
        assert!(
            synthesized
                .roles
                .iter()
                .all(|role| role.role == SupportPlanRole::SupportBody),
            "synthesized source roles must be rewritten to SupportBody"
        );
        assert!(
            synthesized.global_layer_index < 0,
            "synthesized source rows must retain a distinct physical-plane identity"
        );
        assert!(
            output
                .entries()
                .iter()
                .filter(|entry| {
                    entry.anchor_z == synthesized.anchor_z
                        && entry.global_layer_index == synthesized.global_layer_index
                })
                .count()
                == output
                    .entries()
                    .iter()
                    .filter(|entry| { entry.anchor_z == synthesized.anchor_z })
                    .count(),
            "entries sharing a physical plane must share its DEV-163 identity"
        );
    }
}

fn run_mixed_source_tree_fixture() -> SupportGeometryOutput {
    let object = overhang("mixed-source-tree", 0.0, 0.0, 12.0);
    let square = |x: f32| ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(x, 1.0),
                Point2::from_mm(x + 2.0, 1.0),
                Point2::from_mm(x + 2.0, 3.0),
                Point2::from_mm(x, 3.0),
            ],
        },
        holes: vec![],
    };
    let analysis = SupportAnalysisView {
        candidates: vec![
            SupportAnalysisCandidate {
                id: 111,
                object_id: object.object_id.clone(),
                region_id: "0".into(),
                global_layer_index: 4,
                z_units: slicer_ir::mm_to_units(1.25),
                geometry: vec![square(1.0)],
                ..Default::default()
            },
            SupportAnalysisCandidate {
                id: 112,
                object_id: object.object_id.clone(),
                region_id: "0".into(),
                global_layer_index: 6,
                z_units: slicer_ir::mm_to_units(1.8),
                geometry: vec![square(8.0)],
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let layers = LayerPlanView {
        layers: [0.2, 0.4, 0.6, 0.8, 1.25, 1.6, 1.8]
            .into_iter()
            .enumerate()
            .map(|(index, z)| LayerPlanViewEntry {
                global_layer_index: index as u32,
                z,
                effective_layer_height: 0.2,
            })
            .collect(),
    };
    let planner = tree_support_planner::SupportPlanner::from_config(&planner_config_full_with(
        true,
        5.0,
        45.0,
        &[
            ("support_layer_height_mm", ConfigValue::Float(0.45)),
            ("support_interface_top_layers", ConfigValue::Int(1)),
            ("support_top_z_distance_mm", ConfigValue::Float(0.0)),
        ],
    ))
    .expect("from_config");
    let mut output = SupportGeometryOutput::new();
    planner
        .run_support_geometry_with_analysis(
            &[object.clone()],
            &layers,
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
fn coarse_mixed_source_entry_rewrites_interface_geometry_to_body() {
    let output = run_mixed_source_tree_fixture();
    let source = output
        .entries()
        .iter()
        .find(|entry| entry.anchor_z == 8000 && entry.global_layer_index >= 0)
        .expect("the lower interface bracket must survive as a real source entry");
    assert!(source
        .roles
        .iter()
        .any(|role| role.role == SupportPlanRole::TopInterface));
    assert!(source
        .roles
        .iter()
        .any(|role| role.role == SupportPlanRole::SupportBody));

    let synthesized = output
        .entries()
        .iter()
        .find(|entry| entry.anchor_z == 12000 && entry.global_layer_index < 0)
        .expect("the mixed source entry must seed the first coarse plane");
    assert!(synthesized
        .roles
        .iter()
        .all(|role| role.role == SupportPlanRole::SupportBody));
    assert_eq!(
        synthesized
            .roles
            .iter()
            .flat_map(|role| role.regions.iter())
            .collect::<Vec<_>>(),
        source
            .roles
            .iter()
            .flat_map(|role| role.regions.iter())
            .collect::<Vec<_>>(),
        "rewriting the mixed source must retain both body and interface geometry"
    );
}

fn run_two_region_demand(pitch_mm: f64, distinct_grouping_brackets: bool) -> SupportGeometryOutput {
    let object = overhang("two-regions", 0.0, 0.0, 4.0);
    let planner = tree_support_planner::SupportPlanner::from_config(&planner_config_full_with(
        true,
        5.0,
        45.0,
        &[("support_layer_height_mm", ConfigValue::Float(pitch_mm))],
    ))
    .expect("from_config");
    let segmentation = RegionSegmentationView {
        entries: (0..10)
            .map(|layer_index| RegionSegmentationViewEntry {
                object_id: object.object_id.clone(),
                layer_index,
                region_ids: vec!["0".into(), "1".into()],
            })
            .collect(),
        region_support_configs: vec![],
    };
    let candidate_geometry = ExPolygon {
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
    let analysis = SupportAnalysisView {
        candidates: vec![
            SupportAnalysisCandidate {
                id: 71,
                object_id: object.object_id.clone(),
                region_id: "0".into(),
                global_layer_index: 8,
                z_units: slicer_ir::mm_to_units(1.8),
                geometry: vec![candidate_geometry.clone()],
                ..Default::default()
            },
            SupportAnalysisCandidate {
                id: 72,
                object_id: object.object_id.clone(),
                region_id: "1".into(),
                global_layer_index: if distinct_grouping_brackets { 9 } else { 8 },
                z_units: if distinct_grouping_brackets {
                    slicer_ir::mm_to_units(2.0)
                } else {
                    slicer_ir::mm_to_units(1.8)
                },
                geometry: vec![candidate_geometry],
                ..Default::default()
            },
        ],
        family_assignments: vec![
            SupportFamilyAssignment {
                object_id: object.object_id.clone(),
                region_id: "0".into(),
                family_id: "tree".into(),
            },
            SupportFamilyAssignment {
                object_id: object.object_id.clone(),
                region_id: "1".into(),
                family_id: "tree".into(),
            },
        ],
        ..Default::default()
    };
    let mut output = SupportGeometryOutput::new();
    let mut grouping_plan = layer_plan();
    // The two candidate contacts terminate their production runs on layers 6
    // and 7 respectively. Keep those consumed bracket Z values distinct but
    // within canonical EPSILON after tree-family stepping.
    if distinct_grouping_brackets {
        grouping_plan.layers[6].z = 1.39928;
        grouping_plan.layers[7].z = 1.39952;
    }
    planner
        .run_support_geometry_with_analysis(
            &[object],
            &grouping_plan,
            &segmentation,
            &analysis,
            &SupportGeometryView::default(),
            &mut output,
            &ConfigView::new(),
        )
        .expect("run_support_geometry_with_analysis");
    output
}

#[test]
fn coarse_candidates_group_before_plane_identity_assignment() {
    let output = run_two_region_demand(0.3, true);
    let grouped: Vec<_> = output
        .entries()
        .iter()
        .filter(|entry| entry.anchor_z == slicer_ir::mm_to_units(0.49985))
        .collect();
    assert_eq!(
        grouped.len(),
        2,
        "within-EPSILON candidates from both regions must survive one physical plane group"
    );
    let candidate_z = [
        0.2_f64 + (1.39928_f64 - 0.2) / 4.0,
        0.2_f64 + (1.39952_f64 - 0.2) / 4.0,
    ];
    assert_ne!(
        candidate_z[0], candidate_z[1],
        "production bracket Z values must yield genuinely distinct candidates"
    );
    assert!(
        (candidate_z[0] - candidate_z[1]).abs() < 1e-4,
        "production-derived candidate Z values must be within EPSILON"
    );
    assert_eq!(
        grouped[0].anchor_z, 4999,
        "EPSILON grouping must produce one physical midpoint plane"
    );
    assert_eq!(grouped[0].global_layer_index, grouped[1].global_layer_index);
    assert_ne!(grouped[0].region_id, grouped[1].region_id);
    assert!(grouped.iter().all(|entry| entry
        .roles
        .iter()
        .all(|role| role.role == SupportPlanRole::SupportBody)));
}

#[test]
fn intermediate_planes_generated_per_support_body_not_per_layer() {
    let output = run_two_region_demand(0.1, false);

    let grid: std::collections::BTreeSet<_> = layer_plan()
        .layers
        .iter()
        .map(|layer| slicer_ir::mm_to_units(layer.z))
        .collect();
    let mut off_grid_by_region: std::collections::BTreeMap<_, Vec<_>> =
        std::collections::BTreeMap::new();
    let mut off_grid_in_order = Vec::new();
    for entry in output.entries() {
        if !grid.contains(&entry.anchor_z) {
            off_grid_in_order.push((entry.region_id.as_str(), entry.anchor_z));
            off_grid_by_region
                .entry(entry.region_id.clone())
                .or_default()
                .push(entry);
        }
    }
    assert!(
        off_grid_in_order.chunks_exact(2).all(|pair| {
            pair[0].0 == "0" && pair[1].0 == "1" && pair[0].1 == pair[1].1
        }),
        "239c finer candidates must retain object-level append order and per-region multiplicity: {off_grid_in_order:?}"
    );
    assert_eq!(
        off_grid_by_region.len(),
        2,
        "each assigned region is a support body"
    );
    let first = &off_grid_by_region["0"];
    let second = &off_grid_by_region["1"];
    assert_eq!(
        first.iter().map(|entry| entry.anchor_z).collect::<Vec<_>>(),
        second
            .iter()
            .map(|entry| entry.anchor_z)
            .collect::<Vec<_>>(),
        "each body must receive every intermediate plane"
    );
    for entry in first.iter().chain(second) {
        // Real planner streams are exact-Z validated before renderer dispatch.
        // Intermediate rows therefore inherit the lower bracket's bottom-up
        // projection; upper-row geometry can overlap model occupancy between
        // object planes.
        let lower = output
            .entries()
            .iter()
            .filter(|candidate| {
                candidate.region_id == entry.region_id
                    && candidate.anchor_z < entry.anchor_z
                    && grid.contains(&candidate.anchor_z)
            })
            .max_by_key(|candidate| candidate.anchor_z)
            .expect("lower bracket");
        assert_eq!(
            entry.roles, lower.roles,
            "intermediate geometry is seeded from the lower row"
        );
    }
}

#[test]
fn mixed_height_contacts_keep_body_and_roof_on_the_same_layer() {
    let square = |x0: f32, side: f32| ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(x0, 0.0),
                Point2::from_mm(x0 + side, 0.0),
                Point2::from_mm(x0 + side, side),
                Point2::from_mm(x0, side),
            ],
        },
        holes: Vec::new(),
    };
    let roles = build_roles(
        &[],
        &[],
        &[],
        &[],
        &[square(0.0, 4.0)],
        &[square(1.0, 2.0)],
        &[],
        &[],
        1.0,
        &[],
        0,
        0.4,
    );

    assert!(
        roles
            .iter()
            .any(|role| role.role == SupportPlanRole::SupportBody && !role.regions.is_empty())
            && roles
                .iter()
                .any(|role| role.role == SupportPlanRole::TopInterface && !role.regions.is_empty()),
        "mixed layer must retain disjoint body and roof roles: {roles:?}"
    );
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
    // The demand must be rejected through a *typed* gate, not silently
    // dropped. Which gate fires moved with packet 224 defect F-34: the
    // contact layer is now canonical's virtual top-Z-gap node
    // (`distance_to_top < 0`), which is drawn into `roof_gap_areas` and never
    // extruded, so there is no body on that layer left to reject with 1203
    // "complete radius". The whole overhang sits inside the blocking
    // occupancy, so the first *real* column layer is instead pushed out by
    // the avoidance gate and dropped with 1002 `node-clamped-out`. The
    // safety property this test exists for -- that nothing is clipped into a
    // filler polygon -- is asserted unconditionally below.
    assert!(
        output.diagnostics().iter().any(|d| (d.code == 1203
            && d.message.contains("complete radius"))
            || (d.code == 1002 && d.message.contains("node-clamped-out"))),
        "expected a typed rejection (1203 complete-radius or 1002 node-clamped-out); got {:?}",
        output
            .diagnostics()
            .iter()
            .map(|d| (d.code, d.message.clone()))
            .collect::<Vec<_>>()
    );
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
        // exhaustive: support-plan identity fixture; SupportPlanEntry has no Default impl and FRU would let a new plan field default silently
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

#[test]
fn branch_radius_clamps_at_canonical_maximum() {
    let radius = tapered_radius(5.0, 1.0, 20, 1.0);
    assert_eq!(radius, 10.0, "branch radius must clamp at 10.0 mm");
}

#[test]
fn radius_raises_to_base_under_interfaces() {
    let base_radius = 2.5;
    let ordinary = tapered_radius(5.0, 1.0, 1, 0.2);
    let with_interfaces = interface_adjusted_radius(ordinary, base_radius, 2, true);
    assert!(with_interfaces >= base_radius);
    assert_eq!(
        interface_adjusted_radius(ordinary, base_radius, 0, true),
        ordinary
    );
}

/// Ticket 19: territory owned by another family is barred like the model.
/// The tree is the base family here; a traditional sub-region owns the right
/// half of the overhang. No tree role area and no skeleton point may enter
/// that half, while the branch still reaches the plate on its own side.
#[test]
fn foreign_territory_bars_tree_roles_and_skeleton() {
    fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> ExPolygon {
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(x0, y0),
                    Point2::from_mm(x1, y0),
                    Point2::from_mm(x1, y1),
                    Point2::from_mm(x0, y1),
                ],
            },
            holes: vec![],
        }
    }
    fn area(poly: &ExPolygon) -> f64 {
        let ring = |points: &[Point2]| -> f64 {
            points
                .iter()
                .enumerate()
                .map(|(i, a)| {
                    let b = &points[(i + 1) % points.len()];
                    (a.x as f64) * (b.y as f64) - (b.x as f64) * (a.y as f64)
                })
                .sum::<f64>()
                .abs()
                * 0.5
        };
        ring(&poly.contour.points) - poly.holes.iter().map(|h| ring(&h.points)).sum::<f64>()
    }
    let foreign = rect(2.0, 0.0, 4.0, 4.0);
    let candidate = SupportAnalysisCandidate {
        id: 19,
        object_id: "territory".into(),
        region_id: "0".into(),
        global_layer_index: 8,
        z_units: slicer_ir::mm_to_units(1.8),
        geometry: vec![rect(0.0, 0.0, 2.0, 4.0)],
        ..Default::default()
    };
    let assignments = vec![
        SupportFamilyAssignment {
            object_id: "territory".into(),
            region_id: "0".into(),
            family_id: "tree".into(),
        },
        SupportFamilyAssignment {
            object_id: "territory".into(),
            region_id: "1".into(),
            family_id: "traditional".into(),
        },
    ];
    let with_territory = run_planner_with_analysis_and_diameter(
        true,
        overhang("territory", 0.0, 0.0, 4.0),
        SupportAnalysisView {
            candidates: vec![candidate.clone()],
            family_assignments: assignments.clone(),
            support_territory: (0..10)
                .map(|layer| SupportAnalysisGeometryEntry {
                    global_support_layer_index: layer,
                    object_id: "territory".into(),
                    region_id: "1".into(),
                    polygons: vec![foreign.clone()],
                })
                .collect(),
            ..Default::default()
        },
        1.0,
    );
    let control = run_planner_with_analysis_and_diameter(
        true,
        overhang("territory", 0.0, 0.0, 4.0),
        SupportAnalysisView {
            candidates: vec![candidate],
            family_assignments: assignments,
            ..Default::default()
        },
        1.0,
    );
    let role_area = |output: &SupportGeometryOutput| -> f64 {
        output
            .entries()
            .iter()
            .flat_map(|entry| entry.roles.iter())
            .flat_map(|role| role.regions.iter())
            .map(area)
            .sum()
    };
    assert!(
        role_area(&control) > 0.0,
        "control must plan support, or the territory assertions prove nothing"
    );
    assert!(
        role_area(&with_territory) > 0.0,
        "tree support must survive on its own side of the boundary"
    );
    assert!(
        with_territory
            .entries()
            .iter()
            .any(|entry| entry.global_layer_index == 0
                && entry.roles.iter().any(|role| !role.regions.is_empty())),
        "branch must still reach the plate outside the foreign half; entries={:?}",
        with_territory
            .entries()
            .iter()
            .map(|e| e.global_layer_index)
            .collect::<Vec<_>>()
    );
    for entry in with_territory.entries() {
        for role in &entry.roles {
            let inside = host::clip_polygons(
                &role.regions,
                &[foreign.clone()],
                ClipOperation::Intersection,
            );
            let inside_area: f64 = inside.iter().map(area).sum();
            assert_eq!(
                inside_area, 0.0,
                "layer {} role {:?} has area inside the foreign half",
                entry.global_layer_index, role.role
            );
        }
        if let Some(skeleton) = &entry.skeleton {
            for point in &skeleton.points {
                let inside_foreign = point.x > 2.0 && point.y > 0.0 && point.y < 4.0;
                assert!(
                    !inside_foreign,
                    "layer {} skeleton point {:?} lies inside the foreign half",
                    entry.global_layer_index, point
                );
            }
        }
    }
}
