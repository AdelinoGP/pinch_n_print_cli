//! Contract tests for the traditional support family seam.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{ConfigKey, ConfigValue, ConfigView};
use slicer_ir::{
    ExPolygon, IndexedTriangleSet, MeshIR, ObjectMesh, Point2, Point3, Polygon,
    SupportAnalysisIR, SupportPlanDeclineReason, SupportPlanEntry, SupportPlanIR, SupportPlanRole,
    SupportPlanRoleRegion, Transform3d,
};
use slicer_sdk::host::{self, ClipOperation};
use slicer_sdk::prepass_builders::SupportGeometryOutput;
use slicer_sdk::prepass_types::{
    LayerPlanView, LayerPlanViewEntry, MeshObjectView, RegionSegmentationView,
    RegionSegmentationViewEntry, SupportAnalysisCandidate, SupportAnalysisGeometryEntry,
    SupportAnalysisView, SupportFamilyAssignment, SupportGeometryView,
    SupportPlanEntry as SdkSupportPlanEntry,
};
use slicer_sdk::traits::PrepassModule;
use slicer_wasm_host::exact_z_query::ExactZQueryService;
use slicer_wasm_host::support_aggregation::{
    aggregate_declined_support_plans, aggregate_support_plan_irs_with_policy_attributed,
    FamilyConflictPolicy, SupportPlanProducer,
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

fn run_planner_with_layer_plan(
    config: ConfigView,
    object: MeshObjectView,
    analysis: SupportAnalysisView,
    layers: LayerPlanView,
) -> SupportGeometryOutput {
    let planner = SupportPlanner::from_config(&config).expect("from_config");
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
        output.entries().iter().any(|e| e
            .roles
            .iter()
            .any(|r| r.role == SupportPlanRole::SupportBody)),
        "a multi-layer column must carry body geometry below its interface band"
    );
    assert!(
        output.entries().len() >= 2,
        "contact area must derive body/interface roles across layers"
    );
}

#[test]
fn disabled_independent_height_copies_object_layer_print_z_exactly() {
    // Packet 239c AC-3 + AC-2: the enabled/disabled off-grid matrix at module
    // level, run through the planner's native run_support_geometry path.
    //
    // Disabled (AC-3, canonical `PrintObjectSupportMaterial::
    // bottom_contact_layer` disabled path calling `sync_gap_with_object_layer`):
    // every emitted `SupportPlanEntry.anchor_z` equals
    // `mm_to_units(layer_plan.layers[entry.anchor_layer_index].z)` with
    // INTEGER equality — no tolerance window.
    //
    // Enabled (AC-2, canonical `generate_support_layers`
    // (`SupportCommon.cpp`) stepping: `n_layers_extra = ceil((dist - EPSILON)
    // / max_support_layer_height)`, `step = dist / n_layers_extra`,
    // `print_z = bottom_z + k * step`): with a pitch of 0.1 mm against a
    // 0.2 mm object grid and a candidate spanning layers 0..=8, the gap
    // between adjacent support rows (0.2 mm) demands n = ceil((0.2 - 1e-4) /
    // 0.1) = 2 rows per gap, so one strictly-between plane (step 0.1 mm)
    // is inserted under every row the planner emits; at least one such
    // plane differs from its object layer's Z by more than
    // `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS` (10 units).
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
    let object = overhang_object("contact");

    // Disabled: exact copy. Integer equality on i64 units.
    let disabled = run_planner_with_config(
        {
            let base = planner_config(true);
            let mut values = base
                .keys()
                .into_iter()
                .filter_map(|key| base.get(&key).map(|value| (key, value.clone())))
                .collect::<HashMap<ConfigKey, ConfigValue>>();
            values.insert(
                "independent_support_layer_height".into(),
                ConfigValue::Bool(false),
            );
            ConfigView::from_map(values)
        },
        object.clone(),
        analysis.clone(),
    );
    assert!(
        !disabled.entries().is_empty(),
        "traditional contact must produce plan entries"
    );
    for entry in disabled.entries() {
        let grid_z = layer_plan().layers[entry.anchor_layer_index as usize].z;
        assert_eq!(
            entry.anchor_z,
            slicer_ir::mm_to_units(grid_z),
            "disabled branch must copy the object layer print_z exactly \
             (integer equality, canonical sync_gap_with_object_layer); \
             entry at layer {} had anchor_z {}",
            entry.global_layer_index,
            entry.anchor_z
        );
    }

    // A raw zero pitch bypasses both coarse replacement and off-grid rows.
    let zero_pitch = run_planner_with_analysis(true, object.clone(), analysis.clone());
    assert!(zero_pitch.entries().iter().all(|entry| {
        entry.anchor_z
            == slicer_ir::mm_to_units(layer_plan().layers[entry.anchor_layer_index as usize].z)
    }));

    // Enabled: at least one off-grid plane, strictly increasing per body.
    let enabled = run_planner_with_config(
        {
            // Note: `independent_support_layer_height` defaults to true, and
            // the config here sets an explicit finer pitch (0.1 mm).
            planner_config_with(true, 0.0, 0.1)
        },
        object.clone(),
        analysis.clone(),
    );
    let tolerance_units = slicer_ir::AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS;
    let off_grid: Vec<i64> = enabled
        .entries()
        .iter()
        .filter(|entry| {
            let grid_z = layer_plan().layers[entry.anchor_layer_index as usize].z;
            entry.anchor_z.abs_diff(slicer_ir::mm_to_units(grid_z)) > tolerance_units as u64
        })
        .map(|entry| entry.anchor_z)
        .collect();
    assert!(
        !off_grid.is_empty(),
        "enabled branch must produce at least one off-grid anchor_z \
         (>{tolerance_units} units from its object layer's Z); got {:?}",
        enabled
            .entries()
            .iter()
            .map(|e| (e.global_layer_index, e.anchor_z))
            .collect::<Vec<_>>()
    );
    // Free-floating planes must be strictly increasing within the object's
    // emitted sequence ordered by declared plane.
    let mut planes: Vec<i64> = enabled.entries().iter().map(|e| e.anchor_z).collect();
    planes.sort_unstable();
    planes.dedup();
    let sorted = {
        let mut sorted = planes.clone();
        sorted.sort_unstable();
        sorted == planes
    };
    assert!(
        sorted,
        "declared planes must be distinct and strictly ordered; got {planes:?}"
    );
}

#[test]
fn coarse_pitch_produces_free_floating_anchor_z() {
    let object = overhang_object("coarse-traditional");
    let analysis = SupportAnalysisView {
        candidates: vec![SupportAnalysisCandidate {
            id: 1,
            object_id: "coarse-traditional".into(),
            region_id: "0".into(),
            global_layer_index: 8,
            z_units: slicer_ir::mm_to_units(1.8),
            geometry: vec![contact_region()],
            ..Default::default()
        }],
        termination_surfaces: vec![SupportAnalysisGeometryEntry {
            global_support_layer_index: 2,
            object_id: "coarse-traditional".into(),
            region_id: "0".into(),
            polygons: vec![contact_region()],
        }],
        family_assignments: vec![traditional_assignment("coarse-traditional")],
        ..Default::default()
    };
    let output = run_planner_with_config(planner_config_with(true, 0.0, 0.3), object, analysis);
    let planes: Vec<_> = output
        .entries()
        .iter()
        .map(|entry| entry.anchor_z)
        .collect();
    assert_eq!(planes, vec![6000, 9000, 12000, 14000, 16000]);
    assert!(planes.windows(2).all(|pair| pair[0] < pair[1]));

    let synthesized = output
        .entries()
        .iter()
        .find(|entry| entry.anchor_z == 9000)
        .expect("traditional midpoint must be synthesized off-grid");
    assert!(synthesized.global_layer_index < 0);
    assert_eq!(
        synthesized.anchor_layer_index, 3,
        "0.9 mm ties the 0.8 and 1.0 mm layers, so the lower index wins"
    );
    assert!(synthesized
        .roles
        .iter()
        .all(|role| role.role == SupportPlanRole::SupportBody));
    assert!(
        synthesized.anchor_z.abs_diff(slicer_ir::mm_to_units(
            layer_plan().layers[synthesized.anchor_layer_index as usize].z
        )) > slicer_ir::AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS as u64
    );

    for plane in [6000, 12000, 14000, 16000] {
        let entry = output
            .entries()
            .iter()
            .find(|entry| entry.anchor_z == plane)
            .unwrap();
        assert!(
            entry.roles.iter().any(|role| matches!(
                role.role,
                SupportPlanRole::TopInterface | SupportPlanRole::BottomInterface
            )),
            "genuine interface plane {plane} must survive"
        );
    }
    assert!(
        !planes.contains(&8000) && !planes.contains(&10000),
        "strictly interior body rows must be replaced"
    );
    let interface_planes: Vec<_> = output
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
    assert!(
        interface_planes.windows(2).any(|pair| {
            output.entries().iter().any(|entry| {
                entry.anchor_z > pair[0]
                    && entry.anchor_z < pair[1]
                    && entry.global_layer_index < 0
                    && entry
                        .roles
                        .iter()
                        .all(|role| role.role == SupportPlanRole::SupportBody)
            })
        }),
        "a body-bearing interface span must use interface planes as brackets"
    );
    let fallback = run_adjacent_interface_fixture();
    assert_eq!(
        fallback
            .entries()
            .iter()
            .map(|entry| entry.anchor_z)
            .collect::<Vec<_>>(),
        vec![2000, 4800, 7600, 10400, 12000, 13200, 14000, 16000],
        "AC-3 endpoint fallback must preserve the exact ordered plane sequence"
    );
    assert_eq!(
        fallback
            .entries()
            .iter()
            .filter(|entry| entry.global_layer_index < 0)
            .map(|entry| entry.anchor_z)
            .collect::<Vec<_>>(),
        vec![4800, 7600, 10400, 13200],
        "AC-3 endpoint fallback must emit the exact EPSILON-biased off-grid planes"
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
        vec![12000, 14000, 16000],
        "AC-3 endpoint fallback must retain every protected interface plane"
    );

    // AC-3's promised range-local `support_step` neutralization (D3), asserted
    // on the production path: the coarse region's rows bypass the legacy
    // `support_step` gate while rows outside the coarse ranges retain the
    // computed decimation. `run_step_local_fixture` gives `support_step = 3`
    // (0.3 mm pitch over a 0.1 mm model layer). The coarse region "0" keeps its
    // real endpoint brackets at layers 4 and 6 — layer 4 would be removed by
    // `(6 - 4) = 2` not being a multiple of 3, but it is inside the coarse
    // range so it bypasses the gate. The finer region "1" outside the coarse
    // range keeps only its `support_step = 3` rows (termination layer 0, layer
    // 1, and the interface layer 4); layers 2 and 3 are decimated away.
    let step_local = run_step_local_fixture();
    assert_eq!(
        step_local
            .entries()
            .iter()
            .filter(|entry| entry.global_layer_index >= 0)
            .map(|entry| (entry.region_id.as_str(), entry.anchor_layer_index))
            .collect::<Vec<_>>(),
        vec![("1", 0), ("1", 1), ("0", 4), ("1", 4), ("0", 6)],
        "coarse-range rows must bypass legacy support_step while rows outside \
         the coarse ranges retain the computed decimation"
    );
}

fn run_adjacent_interface_fixture() -> SupportGeometryOutput {
    let object = overhang_object("adjacent-interface-traditional");
    let analysis = SupportAnalysisView {
        candidates: vec![SupportAnalysisCandidate {
            id: 1,
            object_id: "adjacent-interface-traditional".into(),
            region_id: "0".into(),
            global_layer_index: 8,
            z_units: slicer_ir::mm_to_units(1.8),
            geometry: vec![contact_region()],
            ..Default::default()
        }],
        family_assignments: vec![traditional_assignment("adjacent-interface-traditional")],
        ..Default::default()
    };
    let base = planner_config_with(true, 0.0, 0.3);
    let mut values = base
        .keys()
        .into_iter()
        .filter_map(|key| base.get(&key).map(|value| (key, value.clone())))
        .collect::<HashMap<ConfigKey, ConfigValue>>();
    values.insert("support_interface_top_layers".into(), ConfigValue::Int(3));
    values.insert(
        "support_interface_bottom_layers".into(),
        ConfigValue::Int(0),
    );
    run_planner_with_config(ConfigView::from_map(values), object, analysis)
}

#[test]
fn coarse_adjacent_interface_cluster_uses_endpoint_fallback() {
    let output = run_adjacent_interface_fixture();
    let planes_and_roles: Vec<_> = output
        .entries()
        .iter()
        .map(|entry| {
            (
                entry.anchor_z,
                entry.global_layer_index,
                entry.roles.iter().map(|role| role.role).collect::<Vec<_>>(),
            )
        })
        .collect();

    assert_eq!(
        planes_and_roles,
        vec![
            (2000, 0, vec![SupportPlanRole::SupportBody]),
            (4800, i32::MIN, vec![SupportPlanRole::SupportBody]),
            (7600, i32::MIN + 1, vec![SupportPlanRole::SupportBody]),
            (10400, i32::MIN + 2, vec![SupportPlanRole::SupportBody]),
            (12000, 5, vec![SupportPlanRole::TopInterface]),
            (13200, i32::MIN + 3, vec![SupportPlanRole::SupportBody]),
            (14000, 6, vec![SupportPlanRole::TopInterface]),
            (16000, 7, vec![SupportPlanRole::TopInterface]),
        ],
        "an adjacent interface cluster has no body-bearing interface span, so the run endpoints must bracket the off-grid stack while every genuine interface row survives"
    );
}

#[test]
fn coarse_candidates_within_epsilon_group_before_identity_assignment() {
    let object = overhang_object("grouped-traditional");
    let analysis = SupportAnalysisView {
        candidates: vec![(1, "0", 6), (2, "1", 7)]
            .into_iter()
            .map(
                |(id, region_id, global_layer_index)| SupportAnalysisCandidate {
                    id,
                    object_id: "grouped-traditional".into(),
                    region_id: region_id.into(),
                    global_layer_index,
                    // The coarse stack's production Z is derived from the layer
                    // plan's bracket rows, not from the candidate's declared
                    // `z_units` (that field only feeds declined entries). Keep
                    // the declaration consistent with the grid anyway.
                    z_units: slicer_ir::mm_to_units(if id == 1 { 1.2003 } else { 1.4 }),
                    geometry: vec![contact_region()],
                    ..Default::default()
                },
            )
            .collect(),
        family_assignments: ["0", "1"]
            .into_iter()
            .map(|region_id| SupportFamilyAssignment {
                object_id: "grouped-traditional".into(),
                region_id: region_id.into(),
                family_id: "traditional".into(),
            })
            .collect(),
        ..Default::default()
    };
    let layers = LayerPlanView {
        layers: [0.2, 0.4, 0.6, 0.8, 1.0, 1.2, 1.2003, 1.4, 1.6, 1.8]
            .into_iter()
            .enumerate()
            .map(|(i, z)| LayerPlanViewEntry {
                global_layer_index: i as u32,
                z,
                effective_layer_height: 0.2,
            })
            .collect(),
    };
    let config = {
        let base = planner_config_with(true, 0.0, 0.3);
        let mut values = base
            .keys()
            .into_iter()
            .filter_map(|key| base.get(&key).map(|value| (key, value.clone())))
            .collect::<HashMap<ConfigKey, ConfigValue>>();
        values.insert("support_interface_top_layers".into(), ConfigValue::Int(1));
        values.insert(
            "support_interface_bottom_layers".into(),
            ConfigValue::Int(0),
        );
        ConfigView::from_map(values)
    };
    let output = run_planner_with_layer_plan(config, object, analysis, layers.clone());

    // The two assigned regions end their coarse brackets on grid layers 5 and 6
    // whose Z values (`0.2, ..., 1.2, 1.2003, ...`) give the traditional-family
    // stepping (`raft_and_intermediate_support_layers`, `SupportMaterial.cpp`)
    // a distinct n=4 plane spacing. The first stepped plane of each bracket is
    // `(3*below_units + above_units)/4`; below is layer 0 at 0.2 mm.
    let below_units = slicer_ir::mm_to_units(layers.layers[0].z);
    let above_units_0 = slicer_ir::mm_to_units(layers.layers[5].z);
    let above_units_1 = slicer_ir::mm_to_units(layers.layers[6].z);
    let raw_mm =
        |above_units: i64| (3 * below_units + above_units) as f64 / 4.0 / slicer_ir::UNITS_PER_MM;
    let raw_0 = raw_mm(above_units_0);
    let raw_1 = raw_mm(above_units_1);
    assert!(
        (raw_0 - raw_1).abs() < 1e-4,
        "production-derived raw Z must be within the canonical grouping EPSILON \
         (1e-4 mm); got {raw_0} and {raw_1}"
    );
    assert_ne!(
        raw_0, raw_1,
        "the two production candidates must consume genuinely distinct raw Z values"
    );
    assert_ne!(
        slicer_ir::mm_to_units(raw_0 as f32),
        slicer_ir::mm_to_units(raw_1 as f32),
        "within-EPSILON raw Z must still map to distinct integer plane identities \
         before grouping, so the fold is observable: got {raw_0} and {raw_1}"
    );

    // Canonical EPSILON candidate grouping (`generate_support_layers`,
    // `SupportCommon.cpp`) folds the two raw planes into ONE midpoint plane.
    // No intermediate integer identity is ever emitted for either raw value.
    let midpoint = slicer_ir::mm_to_units(((raw_0 + raw_1) / 2.0) as f32);
    let grouped: Vec<_> = output
        .entries()
        .iter()
        .filter(|entry| entry.anchor_z == midpoint)
        .collect();
    assert_eq!(
        grouped.len(),
        2,
        "the distinct production candidates must fold into one midpoint plane"
    );
    assert!(
        output
            .entries()
            .iter()
            .filter(|entry| entry.anchor_z == slicer_ir::mm_to_units(raw_1 as f32))
            .all(|entry| entry.anchor_z == midpoint),
        "neither pre-group raw integer identity may survive as a separate plane"
    );
    assert_eq!(grouped[0].global_layer_index, grouped[1].global_layer_index);
    assert_ne!(grouped[0].body_ids, grouped[1].body_ids);
    assert_eq!(
        grouped
            .iter()
            .map(|entry| entry.region_id.as_str())
            .collect::<Vec<_>>(),
        vec!["0", "1"],
        "both production candidates must survive midpoint grouping"
    );
    assert!(grouped.iter().all(|entry| {
        entry
            .roles
            .iter()
            .all(|role| role.role == SupportPlanRole::SupportBody)
    }));
    assert!(
        output
            .entries()
            .iter()
            .filter(|entry| entry.anchor_z == midpoint)
            .count()
            == grouped.len(),
        "exactly one grouped synthesized plane exists at the midpoint, not one per region"
    );

    for (region, interface_z) in [("0", above_units_0), ("1", above_units_1)] {
        let interface = output
            .entries()
            .iter()
            .find(|entry| entry.region_id == region && entry.anchor_z == interface_z)
            .expect("the run's lone interface must survive as a bracket");
        assert!(interface
            .roles
            .iter()
            .any(|role| role.role == SupportPlanRole::TopInterface));
    }
}

#[test]
fn adaptive_local_gap_stays_finer() {
    let object = overhang_object("adaptive-traditional");
    let analysis = SupportAnalysisView {
        candidates: vec![SupportAnalysisCandidate {
            id: 1,
            object_id: "adaptive-traditional".into(),
            region_id: "0".into(),
            global_layer_index: 5,
            z_units: slicer_ir::mm_to_units(1.8),
            geometry: vec![contact_region()],
            ..Default::default()
        }],
        family_assignments: vec![traditional_assignment("adaptive-traditional")],
        ..Default::default()
    };
    let layers = LayerPlanView {
        layers: (0..6)
            .map(|i| LayerPlanViewEntry {
                global_layer_index: i,
                z: (i as f32 + 1.0) * 0.3,
                effective_layer_height: 0.1,
            })
            .collect(),
    };
    let output = run_planner_with_layer_plan(
        planner_config_with(true, 0.0, 0.2),
        object,
        analysis,
        layers,
    );
    assert_eq!(
        output
            .entries()
            .iter()
            .map(|entry| entry.anchor_z)
            .collect::<Vec<_>>(),
        vec![15000, 13500, 12000, 10500, 9000, 5000, 7000, 3000],
        "the finer path first applies support_step=2, then inserts planes between surviving rows"
    );
    assert_eq!(
        output
            .entries()
            .iter()
            .filter(|entry| entry.global_layer_index >= 0)
            .count(),
        4,
        "the finer path must retain the legacy support_step=2 rows plus interface rows"
    );
    assert!(
        output
            .entries()
            .iter()
            .filter(|entry| entry.global_layer_index < 0)
            .all(|entry| matches!(entry.anchor_z, 5000 | 7000 | 10500 | 13500)),
        "the finer path must not contain coarse-only planes"
    );
}

#[test]
fn mixed_coarse_and_finer_ranges_are_ordered_as_one_object_stack() {
    let object = overhang_object("mixed-traditional");
    let analysis = SupportAnalysisView {
        candidates: vec![(1, "0", 3), (2, "1", 5)]
            .into_iter()
            .map(
                |(id, region_id, global_layer_index)| SupportAnalysisCandidate {
                    id,
                    object_id: "mixed-traditional".into(),
                    region_id: region_id.into(),
                    global_layer_index,
                    z_units: slicer_ir::mm_to_units(if id == 1 { 0.8 } else { 1.4 }),
                    geometry: vec![contact_region()],
                    ..Default::default()
                },
            )
            .collect(),
        family_assignments: ["0", "1"]
            .into_iter()
            .map(|region_id| SupportFamilyAssignment {
                object_id: "mixed-traditional".into(),
                region_id: region_id.into(),
                family_id: "traditional".into(),
            })
            .collect(),
        ..Default::default()
    };
    let layers = LayerPlanView {
        layers: [0.2, 0.4, 0.6, 0.8, 1.1, 1.4]
            .into_iter()
            .enumerate()
            .map(|(i, z)| LayerPlanViewEntry {
                global_layer_index: i as u32,
                z,
                effective_layer_height: if i < 4 { 0.2 } else { 0.3 },
            })
            .collect(),
    };
    let config = {
        let base = planner_config_with(true, 0.0, 0.2);
        let mut values = base
            .keys()
            .into_iter()
            .filter_map(|key| base.get(&key).map(|value| (key, value.clone())))
            .collect::<HashMap<ConfigKey, ConfigValue>>();
        values.insert("support_interface_top_layers".into(), ConfigValue::Int(1));
        values.insert(
            "support_interface_bottom_layers".into(),
            ConfigValue::Int(0),
        );
        ConfigView::from_map(values)
    };
    let output = run_planner_with_layer_plan(config, object, analysis, layers);
    let planes: Vec<_> = output
        .entries()
        .iter()
        .map(|entry| entry.anchor_z)
        .collect();

    assert_eq!(
        planes,
        vec![2000, 2000, 4000, 4000, 6000, 6000, 8000, 9500, 11000],
        "mixed coarse/finer output must preserve every row and synthesized stack plane"
    );
    assert_eq!(
        output
            .entries()
            .iter()
            .filter(|entry| entry.body_ids == ["traditional-body-mixed-traditional-1"])
            .map(|entry| entry.anchor_z)
            .collect::<Vec<_>>(),
        vec![2000, 4000, 6000],
        "the finer candidate's original multiplicity and order must survive"
    );
    assert_eq!(
        output
            .entries()
            .iter()
            .filter(|entry| entry.body_ids == ["traditional-body-mixed-traditional-2"])
            .map(|entry| entry.anchor_z)
            .collect::<Vec<_>>(),
        vec![2000, 4000, 6000, 8000, 9500, 11000],
        "the coarse candidate must retain its original rows and coarse stack order"
    );
    assert!(
        output.entries().iter().any(|entry| {
            entry.body_ids == ["traditional-body-mixed-traditional-2"] && entry.anchor_z == 9500
        }),
        "the adaptive range must retain its 239c finer candidate"
    );
}

#[test]
fn coarse_bracket_neutralizes_support_step_only_inside_its_range() {
    let output = run_step_local_fixture();

    assert_eq!(
        output
            .entries()
            .iter()
            .map(|entry| entry.anchor_z)
            .collect::<Vec<_>>(),
        vec![2000, 4000, 6000, 9000, 12000, 15000, 18000, 18000, 20000, 22000],
        "the coarse region emits its endpoint-bracket stack while the finer region applies support_step=3 before deriving intermediate planes"
    );
    assert_eq!(
        output
            .entries()
            .iter()
            .filter(|entry| entry.global_layer_index >= 0)
            .map(|entry| entry.anchor_layer_index)
            .collect::<Vec<_>>(),
        vec![0, 1, 4, 4, 6],
        "the finer region keeps only its support_step=3 rows while the coarse region keeps its real brackets"
    );
    assert!(output.entries().iter().any(|entry| entry.region_id == "0"
        && entry.anchor_z == 20000
        && entry.global_layer_index < 0
        && entry
            .roles
            .iter()
            .all(|role| role.role == SupportPlanRole::SupportBody)));
}

#[test]
fn coarse_synthesized_rows_use_height_local_geometry() {
    let output = run_step_local_fixture();
    let lower = output
        .entries()
        .iter()
        .find(|entry| {
            entry.region_id == "0"
                && entry.anchor_z == 18000
                && entry.body_ids == ["traditional-body-step-local-traditional-1"]
        })
        .expect("coarse lower bracket must retain its source membership");
    let synthesized = output
        .entries()
        .iter()
        .find(|entry| {
            entry.region_id == "0"
                && entry.anchor_z == 20000
                && entry.global_layer_index < 0
                && entry.body_ids == ["traditional-body-step-local-traditional-1"]
        })
        .expect("coarse synthesized row must retain its source membership");
    let body_regions = |entry: &SdkSupportPlanEntry| {
        entry
            .roles
            .iter()
            .find(|role| role.role == SupportPlanRole::SupportBody)
            .expect("entry must retain SupportBody role")
            .regions
            .clone()
    };

    assert_ne!(
        body_regions(synthesized),
        body_regions(lower),
        "the synthesized coarse row must use geometry at its own height"
    );
}

/// The production D3 fixture: two regions of one object, one coarse (region
/// "0", terminating on the model at layer 4) and one finer (region "1", running
/// to the plate). With `support_layer_height_mm = 0.3` over a 0.1 mm model
/// layer, `support_step = round(0.3 / 0.1) = 3`. The coarse region's endpoint
/// bracket (layers 4..=6) satisfies the binding coarse predicate (pitch 3000
/// units >= `local_support_gap` 2000 units), so its rows bypass the
/// `support_step` gate; the finer region's rows outside the coarse range retain
/// the computed `support_step = 3` decimation.
fn run_step_local_fixture() -> SupportGeometryOutput {
    let object = overhang_object("step-local-traditional");
    let analysis = SupportAnalysisView {
        candidates: vec![(1, "0", 7), (2, "1", 5)]
            .into_iter()
            .map(
                |(id, region_id, global_layer_index)| SupportAnalysisCandidate {
                    id,
                    object_id: "step-local-traditional".into(),
                    region_id: region_id.into(),
                    global_layer_index,
                    z_units: slicer_ir::mm_to_units(if id == 1 { 2.4 } else { 2.0 }),
                    geometry: vec![contact_region()],
                    ..Default::default()
                },
            )
            .collect(),
        termination_surfaces: vec![SupportAnalysisGeometryEntry {
            global_support_layer_index: 4,
            object_id: "step-local-traditional".into(),
            region_id: "0".into(),
            polygons: vec![contact_region()],
        }],
        model_occupancy: vec![SupportAnalysisGeometryEntry {
            global_support_layer_index: 4,
            object_id: "step-local-traditional".into(),
            region_id: "0".into(),
            polygons: vec![ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2::from_mm(0.0, 0.0),
                        Point2::from_mm(2.0, 0.0),
                        Point2::from_mm(2.0, 4.0),
                        Point2::from_mm(0.0, 4.0),
                    ],
                },
                holes: vec![],
            }],
        }],
        family_assignments: ["0", "1"]
            .into_iter()
            .map(|region_id| SupportFamilyAssignment {
                object_id: "step-local-traditional".into(),
                region_id: region_id.into(),
                family_id: "traditional".into(),
            })
            .collect(),
        ..Default::default()
    };
    let layers = LayerPlanView {
        layers: [0.2, 0.6, 1.0, 1.4, 1.8, 2.0, 2.2, 2.4]
            .into_iter()
            .enumerate()
            .map(|(i, z)| LayerPlanViewEntry {
                global_layer_index: i as u32,
                z,
                effective_layer_height: 0.1,
            })
            .collect(),
    };
    let config = {
        let base = planner_config_with(true, 0.0, 0.3);
        let mut values = base
            .keys()
            .into_iter()
            .filter_map(|key| base.get(&key).map(|value| (key, value.clone())))
            .collect::<HashMap<ConfigKey, ConfigValue>>();
        values.insert("support_interface_top_layers".into(), ConfigValue::Int(1));
        values.insert(
            "support_interface_bottom_layers".into(),
            ConfigValue::Int(0),
        );
        ConfigView::from_map(values)
    };
    run_planner_with_layer_plan(config, object, analysis, layers)
}

#[test]
fn adaptive_zero_pitch_preserves_object_grid_output() {
    let object = overhang_object("adaptive-zero-traditional");
    let analysis = SupportAnalysisView {
        candidates: vec![SupportAnalysisCandidate {
            id: 1,
            object_id: "adaptive-zero-traditional".into(),
            region_id: "0".into(),
            global_layer_index: 5,
            z_units: slicer_ir::mm_to_units(1.8),
            geometry: vec![contact_region()],
            ..Default::default()
        }],
        family_assignments: vec![traditional_assignment("adaptive-zero-traditional")],
        ..Default::default()
    };
    let layers = LayerPlanView {
        layers: (0..6)
            .map(|i| LayerPlanViewEntry {
                global_layer_index: i,
                z: (i as f32 + 1.0) * 0.3,
                effective_layer_height: 0.2,
            })
            .collect(),
    };
    let output = run_planner_with_layer_plan(
        planner_config_with(true, 0.0, 0.0),
        object,
        analysis,
        layers,
    );

    assert_eq!(
        output
            .entries()
            .iter()
            .map(|entry| (entry.global_layer_index, entry.anchor_z))
            .collect::<Vec<_>>(),
        vec![(4, 15000), (3, 12000), (2, 9000), (1, 6000), (0, 3000)],
        "raw zero pitch must preserve adaptive object-grid order and multiplicity"
    );
}

#[test]
fn coarse_same_region_sources_keep_distinct_body_membership() {
    let object = overhang_object("same-region-traditional");
    let analysis = SupportAnalysisView {
        candidates: vec![(1, 6), (2, 8)]
            .into_iter()
            .map(|(id, global_layer_index)| SupportAnalysisCandidate {
                id,
                object_id: "same-region-traditional".into(),
                region_id: "0".into(),
                global_layer_index,
                z_units: slicer_ir::mm_to_units(layer_plan().layers[global_layer_index as usize].z),
                geometry: vec![if id == 1 {
                    contact_region()
                } else {
                    obstacle_region()
                }],
                ..Default::default()
            })
            .collect(),
        termination_surfaces: vec![SupportAnalysisGeometryEntry {
            global_support_layer_index: 2,
            object_id: "same-region-traditional".into(),
            region_id: "0".into(),
            polygons: vec![contact_region()],
        }],
        family_assignments: vec![traditional_assignment("same-region-traditional")],
        ..Default::default()
    };
    let config = {
        let base = planner_config_with(true, 0.0, 0.3);
        let mut values = base
            .keys()
            .into_iter()
            .filter_map(|key| base.get(&key).map(|value| (key, value.clone())))
            .collect::<HashMap<ConfigKey, ConfigValue>>();
        values.insert("support_interface_top_layers".into(), ConfigValue::Int(1));
        values.insert(
            "support_interface_bottom_layers".into(),
            ConfigValue::Int(0),
        );
        ConfigView::from_map(values)
    };
    let output = run_planner_with_config(config, object, analysis);
    let body_one = "traditional-body-same-region-traditional-1";
    let body_two = "traditional-body-same-region-traditional-2";

    let synthesized: Vec<_> = output
        .entries()
        .iter()
        .filter(|entry| entry.anchor_z == 14000 && entry.global_layer_index < 0)
        .collect();
    assert_eq!(
        synthesized.len(),
        1,
        "the height-local row at 14000 must retain only body 2"
    );
    assert_eq!(synthesized[0].body_ids, [body_two]);
    assert!(synthesized[0]
        .roles
        .iter()
        .any(|role| role.role == SupportPlanRole::SupportBody));
    assert!(output.entries().iter().any(|entry| {
        entry.body_ids == [body_two]
            && entry.anchor_z > slicer_ir::mm_to_units(layer_plan().layers[5].z)
    }));

    // Packet 241b: one entry per support-region identity is the producer's
    // contract, so the two same-plane sources now arrive as ONE entry. Nothing
    // is lost by that, and this test pins exactly that: the merged entry must
    // carry both body memberships AND the whole planned area of both sources.
    let same_plane: Vec<_> = output
        .entries()
        .iter()
        .filter(|entry| entry.anchor_z == 10000 && entry.global_layer_index < 0)
        .collect();
    assert_eq!(
        same_plane.len(),
        1,
        "one support-region identity must publish exactly one entry"
    );
    let mut memberships = same_plane[0].body_ids.clone();
    memberships.sort();
    assert_eq!(
        memberships,
        vec![body_one.to_string(), body_two.to_string()],
        "both same-plane source body memberships must survive the merge"
    );
    assert!(same_plane[0]
        .roles
        .iter()
        .any(|role| role.role == SupportPlanRole::SupportBody));

    // The two columns carry disjoint geometry (`contact_region` at 0..4mm,
    // `obstacle_region` at 10..14mm), so their union area is their sum. The
    // expected value is taken from the fixture polygons rather than from
    // another planned row: DEV-169 removed the single-membership grid row at
    // z=6000 this previously borrowed as body 1's stand-in (it now merges with
    // the intermediate row that lands on that same plane). The asserted value
    // is unchanged - each fixture polygon is 4mm x 4mm, which is exactly the
    // area that row carried.
    let expected = expolygon_area(&contact_region()) + expolygon_area(&obstacle_region());
    let merged = entry_area(same_plane[0]);
    assert!(
        (merged - expected).abs() <= AREA_TOLERANCE_UNITS2,
        "merged entry must retain the union area of both sources: {merged} vs {expected}"
    );
}

#[test]
fn coarse_source_preference_keeps_mixed_source_memberships() {
    let object_id = "mixed-source-traditional";
    let object = overhang_object(object_id);
    let body_id = format!("traditional-body-{object_id}-1");
    let interface_only_id = format!("traditional-body-{object_id}-2");
    let analysis = SupportAnalysisView {
        candidates: vec![
            SupportAnalysisCandidate {
                id: 1,
                object_id: object_id.into(),
                region_id: "0".into(),
                global_layer_index: 6,
                z_units: slicer_ir::mm_to_units(1.6),
                geometry: vec![obstacle_region()],
                ..Default::default()
            },
            SupportAnalysisCandidate {
                id: 2,
                object_id: object_id.into(),
                region_id: "0".into(),
                global_layer_index: 6,
                z_units: slicer_ir::mm_to_units(1.6),
                geometry: vec![contact_region()],
                ..Default::default()
            },
            // A second record with membership 1 is interface-only at the
            // selected source plane. The body-only record above must win for
            // that membership, while membership 2 remains interface-only.
            SupportAnalysisCandidate {
                id: 1,
                object_id: object_id.into(),
                region_id: "0".into(),
                global_layer_index: 6,
                z_units: slicer_ir::mm_to_units(1.6),
                geometry: vec![contact_region()],
                ..Default::default()
            },
        ],
        termination_surfaces: vec![SupportAnalysisGeometryEntry {
            global_support_layer_index: 3,
            object_id: object_id.into(),
            region_id: "0".into(),
            polygons: vec![contact_region()],
        }],
        family_assignments: vec![traditional_assignment(object_id)],
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
    let config = {
        let base = planner_config_with(true, 0.0, 0.45);
        let mut values = base
            .keys()
            .into_iter()
            .filter_map(|key| base.get(&key).map(|value| (key, value.clone())))
            .collect::<HashMap<ConfigKey, ConfigValue>>();
        values.insert("support_interface_top_layers".into(), ConfigValue::Int(0));
        values.insert(
            "support_interface_bottom_layers".into(),
            ConfigValue::Int(1),
        );
        ConfigView::from_map(values)
    };
    let output = run_planner_with_layer_plan(config, object, analysis, layers);
    let synthesized: Vec<_> = output
        .entries()
        .iter()
        .filter(|entry| entry.anchor_z == 12000 && entry.global_layer_index < 0)
        .collect();

    // Packet 241b: the selected source plane is ONE support-region identity, so
    // it publishes one entry - carrying both the body-only membership and the
    // interface-only membership rather than dropping either.
    assert_eq!(
        synthesized.len(),
        1,
        "one support-region identity must publish exactly one entry"
    );
    let mut memberships = synthesized[0].body_ids.clone();
    memberships.sort();
    assert_eq!(
        memberships,
        vec![body_id.clone(), interface_only_id.clone()],
        "body-only geometry must be preferred per membership without dropping an interface-only membership"
    );
    assert!(synthesized[0]
        .roles
        .iter()
        .all(|role| role.role == SupportPlanRole::SupportBody));
    let mut demands = synthesized[0].demand_ids.clone();
    demands.sort();
    assert_eq!(
        demands,
        vec!["demand-1".to_string(), "demand-2".to_string()],
        "the merged entry must account for both source demands"
    );
    // AC-6: exactly one entry at this `anchor_z` holds both memberships AND its
    // summed `roles[].regions` shoelace area equals the union area of both
    // source contours.
    //
    // The source plane is `anchor_z == 8000`, which packet 241b publishes as a
    // single region-identity entry: its `SupportBody` role carries membership
    // 1's body-only geometry (`obstacle_region`, x 10..14mm) and its
    // `BottomInterface` role carries membership 2's geometry plus the losing
    // duplicate interface record for membership 1 (both `contact_region`,
    // x 0..4mm). Every fixture contour here is therefore either identical to
    // or disjoint from every other, so the UNION over the source regions is
    // exactly the set of distinct source contours - no clipper needed.
    let source = output
        .entries()
        .iter()
        .find(|entry| entry.anchor_z == 8000 && entry.global_layer_index >= 0)
        .expect("the selected source plane must still be published at anchor_z 8000");
    let mut source_memberships = source.body_ids.clone();
    source_memberships.sort();
    assert_eq!(
        source_memberships,
        vec![body_id.clone(), interface_only_id.clone()],
        "the source plane must carry both memberships that feed the merge"
    );
    assert!(
        source
            .roles
            .iter()
            .any(|role| role.role == SupportPlanRole::SupportBody
                && !role.regions.is_empty()),
        "membership 1's body-only source geometry must exist at the source plane"
    );
    assert!(
        source
            .roles
            .iter()
            .any(|role| role.role == SupportPlanRole::BottomInterface
                && !role.regions.is_empty()),
        "membership 2's interface-only source geometry must exist at the source plane"
    );
    let mut union_regions: Vec<ExPolygon> = Vec::new();
    for region in source.roles.iter().flat_map(|role| role.regions.iter()) {
        if !union_regions.contains(region) {
            union_regions.push(region.clone());
        }
    }
    let expected: f64 = union_regions.iter().map(expolygon_area).sum();
    if expected <= 0.0 || union_regions.len() < 2 {
        // Guard: a bug that empties the source side must not read as equality.
        panic!(
            "source union collapsed to {} contour(s) totalling {expected}; the comparison below              would be vacuous",
            union_regions.len()
        );
    }

    // Exact geometry, not just area: every distinct source contour must appear
    // in the merged entry, and the merged entry must carry no extra region.
    let merged_regions: Vec<&ExPolygon> = synthesized[0]
        .roles
        .iter()
        .flat_map(|role| role.regions.iter())
        .collect();
    assert_eq!(
        merged_regions.len(),
        union_regions.len(),
        "the merged entry must carry exactly the distinct source contours"
    );
    for region in &union_regions {
        assert!(
            merged_regions.contains(&region),
            "a source contour was dropped by the merge: {region:?}"
        );
    }

    let merged = entry_area(synthesized[0]);
    assert!(
        (merged - expected).abs() <= AREA_TOLERANCE_UNITS2,
        "merged entry must retain the union area of both sources: {merged} vs {expected}"
    );
}

#[test]
fn coarse_lone_interface_survives_as_bracket() {
    let object = overhang_object("lone-interface-traditional");
    let analysis = SupportAnalysisView {
        candidates: vec![SupportAnalysisCandidate {
            id: 1,
            object_id: "lone-interface-traditional".into(),
            region_id: "0".into(),
            global_layer_index: 6,
            z_units: slicer_ir::mm_to_units(1.4),
            geometry: vec![contact_region()],
            ..Default::default()
        }],
        family_assignments: vec![traditional_assignment("lone-interface-traditional")],
        ..Default::default()
    };
    let config = {
        let base = planner_config_with(true, 0.0, 0.3);
        let mut values = base
            .keys()
            .into_iter()
            .filter_map(|key| base.get(&key).map(|value| (key, value.clone())))
            .collect::<HashMap<ConfigKey, ConfigValue>>();
        values.insert("support_interface_top_layers".into(), ConfigValue::Int(1));
        values.insert(
            "support_interface_bottom_layers".into(),
            ConfigValue::Int(0),
        );
        ConfigView::from_map(values)
    };
    let output = run_planner_with_config(config, object, analysis);
    let interface = output
        .entries()
        .iter()
        .find(|entry| entry.anchor_z == 12000)
        .expect("the genuine lone interface plane must survive");

    assert!(interface
        .roles
        .iter()
        .any(|role| role.role == SupportPlanRole::TopInterface));
    let planes: Vec<_> = output
        .entries()
        .iter()
        .map(|entry| entry.anchor_z)
        .collect();
    assert_eq!(
        planes,
        vec![2000, 4500, 7000, 9500, 12000],
        "the lone interface bracket must retain the coarse stack order and multiplicity"
    );
    let interface_index = planes
        .iter()
        .position(|plane| *plane == interface.anchor_z)
        .unwrap();
    assert!(interface_index > 0);
    assert!(output.entries()[interface_index - 1]
        .roles
        .iter()
        .all(|role| role.role == SupportPlanRole::SupportBody));
    assert!(output.entries()[interface_index - 1].global_layer_index < 0);
    assert!(output.entries().iter().any(|entry| {
        entry.global_layer_index < 0
            && entry.anchor_z < interface.anchor_z
            && entry
                .roles
                .iter()
                .all(|role| role.role == SupportPlanRole::SupportBody)
    }));
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
    let planes: Vec<_> = output
        .entries()
        .iter()
        .filter(|entry| entry.decline_reason.is_none())
        .map(|entry| {
            (
                entry.anchor_z,
                entry.anchor_layer_index,
                entry.roles.iter().map(|role| role.role).collect::<Vec<_>>(),
            )
        })
        .collect();
    assert_eq!(
        planes,
        vec![
            (2000, 0, vec![SupportPlanRole::SupportBody]),
            (6667, 2, vec![SupportPlanRole::SupportBody]),
            (11333, 5, vec![SupportPlanRole::SupportBody]),
            (12000, 5, vec![SupportPlanRole::TopInterface]),
            (14000, 6, vec![SupportPlanRole::TopInterface]),
            (16000, 7, vec![SupportPlanRole::TopInterface]),
        ],
        "the adjacent interface cluster must survive while endpoint fallback emits the exact ordered coarse stack with true-nearest anchors"
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

    // Genuinely oversized: one unit wider AND taller than MAX_BODY_EXTENT_UNITS
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
    // Packet 241b: the merge point is default-deny on support-region ownership,
    // so a plan with no `family_assignments` row is refused before any body is
    // validated. This fixture is about the max-body-extent bound, so it
    // declares the ownership the real manifest declares
    // (`[claims].holds = support-family:traditional` in
    // `traditional-support-planner.toml`) and lets the entry reach
    // `validate_entry`. Without this it would still be dropped - but for the
    // wrong reason, testing nothing about extent.
    let territory = SupportAnalysisIR {
        family_assignments: BTreeMap::from([(
            ("object-a".to_string(), 9u64),
            "traditional".to_string(),
        )]),
        ..Default::default()
    };
    let producer = SupportPlanProducer {
        module_id: "traditional-support-planner".into(),
        claims: vec!["support-family:traditional".into()],
    };
    let (aggregated, diagnostics) = aggregate_support_plan_irs_with_policy_attributed(
        vec![plan],
        vec![producer],
        &exact_z,
        Some(&territory),
        FamilyConflictPolicy::Degrade,
    )
    .expect("the degrading policy never aborts aggregation");
    assert!(
        aggregated.entries.is_empty(),
        "complete crossing body is dropped"
    );
    assert!(
        diagnostics.iter().any(|attributed| {
            attributed.diagnostic.message.contains("spans-cell")
                && attributed.diagnostic.message.contains("max-body-extent")
        }),
        "an owned body larger than the extent bound must be rejected as such: {diagnostics:?}"
    );
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

fn territory_entries(region_id: &str, footprint: &ExPolygon) -> Vec<SupportAnalysisGeometryEntry> {
    (0..10)
        .map(|layer| SupportAnalysisGeometryEntry {
            global_support_layer_index: layer,
            object_id: "territory".into(),
            region_id: region_id.into(),
            polygons: vec![footprint.clone()],
        })
        .collect()
}

/// Ticket 19: a candidate planned for a minted sub-region keeps every role
/// inside that sub-region's own footprint.
#[test]
fn sub_region_column_stays_inside_own_territory() {
    let own = rect(1.0, 0.0, 3.0, 4.0);
    let output = run_planner_with_analysis(
        true,
        overhang_object("territory"),
        SupportAnalysisView {
            candidates: vec![SupportAnalysisCandidate {
                id: 19,
                object_id: "territory".into(),
                region_id: "0".into(),
                global_layer_index: 8,
                z_units: slicer_ir::mm_to_units(1.8),
                geometry: vec![contact_region()],
                ..Default::default()
            }],
            family_assignments: vec![traditional_assignment("territory")],
            support_territory: territory_entries("0", &own),
            ..Default::default()
        },
    );
    assert!(
        output
            .entries()
            .iter()
            .any(|entry| entry.roles.iter().any(|role| !role.regions.is_empty())),
        "column must still be planned inside its own footprint"
    );
    for entry in output.entries() {
        for role in &entry.roles {
            let outside =
                host::clip_polygons(&role.regions, &[own.clone()], ClipOperation::Difference);
            assert!(
                outside.is_empty(),
                "layer {} role {:?} leaves the sub-region footprint: {:?}",
                entry.global_layer_index,
                role.role,
                outside
            );
        }
    }
}

/// Ticket 19: a base-region column keeps clear of territory owned by another
/// family, by at least the line width.
#[test]
fn base_column_keeps_clear_of_foreign_territory() {
    let foreign = rect(2.0, 0.0, 4.0, 4.0);
    let output = run_planner_with_analysis(
        true,
        overhang_object("territory"),
        SupportAnalysisView {
            candidates: vec![SupportAnalysisCandidate {
                id: 19,
                object_id: "territory".into(),
                region_id: "0".into(),
                global_layer_index: 8,
                z_units: slicer_ir::mm_to_units(1.8),
                geometry: vec![contact_region()],
                ..Default::default()
            }],
            family_assignments: vec![
                traditional_assignment("territory"),
                SupportFamilyAssignment {
                    object_id: "territory".into(),
                    region_id: "1".into(),
                    family_id: "tree".into(),
                },
            ],
            support_territory: territory_entries("1", &foreign),
            ..Default::default()
        },
    );
    assert!(
        output
            .entries()
            .iter()
            .any(|entry| entry.roles.iter().any(|role| !role.regions.is_empty())),
        "column must still be planned on its own side"
    );
    for entry in output.entries() {
        for role in &entry.roles {
            let inside = host::clip_polygons(
                &role.regions,
                &[foreign.clone()],
                ClipOperation::Intersection,
            );
            assert!(
                inside.is_empty(),
                "layer {} role {:?} enters the foreign footprint: {:?}",
                entry.global_layer_index,
                role.role,
                inside
            );
            for region in &role.regions {
                for point in &region.contour.points {
                    assert!(
                        point.x <= slicer_ir::mm_to_units(2.0),
                        "layer {} role {:?} point {:?} crosses the boundary",
                        entry.global_layer_index,
                        role.role,
                        point
                    );
                }
            }
        }
    }
}

/// Area comparisons here are exact in principle - the shoelace sum accumulates
/// in `i128`, and these fixtures' areas (~1e9 squared units, 1 unit = 100 nm)
/// are far inside f64's exact-integer range - so this tolerance only absorbs
/// the single halving of an odd doubled area on either side of a comparison.
const AREA_TOLERANCE_UNITS2: f64 = 1.0;

/// Shoelace area of an `ExPolygon` (contour minus holes), in squared canonical
/// units. The doubled area accumulates in `i128`: coordinates reach ~1e6 units,
/// so cross products would overflow `i64` on a large contour.
fn expolygon_area(poly: &ExPolygon) -> f64 {
    let ring = |points: &[Point2]| -> i128 {
        let mut doubled: i128 = 0;
        for index in 0..points.len() {
            let a = &points[index];
            let b = &points[(index + 1) % points.len()];
            doubled += a.x as i128 * b.y as i128 - b.x as i128 * a.y as i128;
        }
        doubled.abs()
    };
    let contour = ring(&poly.contour.points) as f64;
    let holes: f64 = poly.holes.iter().map(|hole| ring(&hole.points) as f64).sum();
    (contour - holes) / 2.0
}

/// Total planned area of one entry, summed across ALL roles.
///
/// `SupportPlanEntry` carries no contour field, and both the producer merge and
/// the host union concatenate role regions per role KIND, so a merged entry
/// routinely carries several roles - summing only the first would silently
/// under-count.
fn entry_area(entry: &SdkSupportPlanEntry) -> f64 {
    entry
        .roles
        .iter()
        .flat_map(|role| role.regions.iter())
        .map(expolygon_area)
        .sum()
}

/// W4 (packet 241b): within one plan, `(global_layer_index, object_id,
/// region_id)` determines `anchor_z`. Two entries sharing that triple while
/// disagreeing on the plane are a producer bug, so the merge refuses to publish
/// rather than fusing two planes into one entry - and rather than leaving the
/// disagreement to surface downstream as an opaque commit rejection carrying no
/// producer context.
#[test]
fn merge_rejects_anchor_z_layer_index_disagreement() {
    let disagreeing = |anchor_z: i64| SdkSupportPlanEntry {
        global_layer_index: 3,
        object_id: "object-a".into(),
        region_id: "0".into(),
        family_id: "traditional".into(),
        demand_ids: vec!["demand-1".into()],
        body_ids: vec!["traditional-body-object-a-1".into()],
        anchor_layer_index: 3,
        anchor_z,
        roles: vec![SupportPlanRoleRegion {
            role: SupportPlanRole::SupportBody,
            regions: vec![contact_region()],
        }],
        ..Default::default()
    };
    let mut entries = vec![disagreeing(10_000), disagreeing(12_000)];
    let result = traditional_support_planner::merge_region_identity_entries(&mut entries);
    assert!(
        result.is_err(),
        "one identity triple resolving to two planes must be rejected, not published"
    );
}

/// DEV-169: a 239c intermediate plane can land exactly on a grid plane that a
/// grid row of the SAME `(object_id, region_id)` still occupies, because the
/// support-step decimation removes one body's grid row at that plane while a
/// different body's row survives there. That is one physical plane of one
/// region, so it must publish ONE entry carrying both body memberships - the
/// same shape planes 8000 and 10000 already produce in this fixture.
///
/// Construction: two bodies whose columns overlap. Body 2 spans layers 0..7,
/// body 1 spans layers 2..5, support pitch 0.3mm over a 0.2mm grid. Body 2's
/// grid row at z=6000 is decimated, then re-inserted by
/// `packet239c_intermediate_planes` across the resulting 4000->8000 gap at the
/// midpoint 6000 - exactly where body 1's grid row still sits.
#[test]
fn coarse_intermediate_plane_on_occupied_grid_plane_publishes_one_entry() {
    let object = overhang_object("same-region-traditional");
    let analysis = SupportAnalysisView {
        candidates: vec![(1, 6), (2, 8)]
            .into_iter()
            .map(|(id, global_layer_index)| SupportAnalysisCandidate {
                id,
                object_id: "same-region-traditional".into(),
                region_id: "0".into(),
                global_layer_index,
                z_units: slicer_ir::mm_to_units(layer_plan().layers[global_layer_index as usize].z),
                geometry: vec![if id == 1 {
                    contact_region()
                } else {
                    obstacle_region()
                }],
                ..Default::default()
            })
            .collect(),
        termination_surfaces: vec![SupportAnalysisGeometryEntry {
            global_support_layer_index: 2,
            object_id: "same-region-traditional".into(),
            region_id: "0".into(),
            polygons: vec![contact_region()],
        }],
        family_assignments: vec![traditional_assignment("same-region-traditional")],
        ..Default::default()
    };
    let config = {
        let base = planner_config_with(true, 0.0, 0.3);
        let mut values = base
            .keys()
            .into_iter()
            .filter_map(|key| base.get(&key).map(|value| (key, value.clone())))
            .collect::<HashMap<ConfigKey, ConfigValue>>();
        values.insert("support_interface_top_layers".into(), ConfigValue::Int(1));
        values.insert(
            "support_interface_bottom_layers".into(),
            ConfigValue::Int(0),
        );
        ConfigView::from_map(values)
    };
    let output = run_planner_with_config(config, object, analysis);
    let body_one = "traditional-body-same-region-traditional-1";
    let body_two = "traditional-body-same-region-traditional-2";

    let at_plane: Vec<_> = output
        .entries()
        .iter()
        .filter(|entry| entry.anchor_z == 6000 && entry.decline_reason.is_none())
        .collect();
    assert_eq!(
        at_plane.len(),
        1,
        "one physical plane of one region must publish exactly one entry, got {:?}",
        at_plane
            .iter()
            .map(|entry| (entry.global_layer_index, entry.body_ids.clone()))
            .collect::<Vec<_>>()
    );
    assert!(
        at_plane[0].global_layer_index >= 0,
        "the surviving grid row's index owns the plane, not a synthesized index"
    );
    let mut memberships = at_plane[0].body_ids.clone();
    memberships.sort();
    assert_eq!(
        memberships,
        vec![body_one.to_string(), body_two.to_string()],
        "both body columns present at this plane must survive the merge"
    );

    // Geometries are disjoint (`contact_region` 0..4mm, `obstacle_region`
    // 10..14mm), so the merged area is their sum. Areas are taken from the
    // fixture polygons, not from another planned row, so this stays valid
    // however the neighbouring rows are decimated.
    let expected = expolygon_area(&contact_region()) + expolygon_area(&obstacle_region());
    let merged = entry_area(at_plane[0]);
    assert!(
        (merged - expected).abs() <= AREA_TOLERANCE_UNITS2,
        "merged entry must retain the union area of both columns: {merged} vs {expected}"
    );
}

/// W4 direction 2, now enforced LITERALLY (DEV-169): within one plan,
/// `(object_id, region_id, anchor_z)` determines `global_layer_index`. Packet
/// 241b had to scope this by index space, because the coarse path could hand a
/// grid row and a synthesized row the same plane; the producer no longer does
/// that (`grid_index_at_plane`), so the check is unscoped and the grid vs.
/// synthesized pairing it used to excuse is now rejected.
#[test]
fn merge_rejects_grid_and_synthesized_rows_claiming_one_plane() {
    let claiming = |global_layer_index: i32| SdkSupportPlanEntry {
        global_layer_index,
        object_id: "object-a".into(),
        region_id: "0".into(),
        family_id: "traditional".into(),
        demand_ids: vec!["demand-1".into()],
        body_ids: vec!["traditional-body-object-a-1".into()],
        anchor_layer_index: 3,
        anchor_z: 6000,
        roles: vec![SupportPlanRoleRegion {
            role: SupportPlanRole::SupportBody,
            regions: vec![contact_region()],
        }],
        ..Default::default()
    };
    let mut entries = vec![claiming(2), claiming(i32::MIN)];
    let result = traditional_support_planner::merge_region_identity_entries(&mut entries);
    assert!(
        result.is_err(),
        "one plane of one region claiming two layer indices must be rejected, not published"
    );
}
