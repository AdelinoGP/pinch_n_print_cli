//! Integration coverage for the default raft footprint synthesizer.

use raft_default::RaftDefault;
use serde_json::to_vec;
use slicer_ir::{
    ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, InfillIR, InfillRegion,
    LayerCollectionIR, LayerStageCommit, LoopType, PerimeterIR, PerimeterRegion, Point2,
    Point3WithWidth, Polygon, RaftPlan, SliceIR, SlicedRegion, SupportEntry, SupportIR,
    SupportPlanIR, WallBoundaryType, WallFeatureFlags, WallLoop, WidthProfile,
};
use slicer_runtime::{
    apply_for_test, build_wasm_instance_pool, CompiledModuleBuilder, LayerArena,
    LoadedModuleBuilder, StageApplyContext, WasmArtifactMetadata, WasmRuntimeDispatcher,
};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::test_support::fixtures::SliceRegionViewBuilder;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use std::sync::Arc;

use crate::common::{layer_input, wasm_cache};

fn square() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(10.0, 0.0),
                Point2::from_mm(10.0, 10.0),
                Point2::from_mm(0.0, 10.0),
            ],
        },
        holes: Vec::new(),
    }
}

fn plan() -> Arc<SupportPlanIR> {
    Arc::new(SupportPlanIR {
        raft_plan: Some(RaftPlan {
            raft_layers: 3,
            raft_first_layer_density: 1.0,
            base_raft_layers: 1,
            interface_raft_layers: 1,
        }),
        ..SupportPlanIR::default()
    })
}

fn semver(major: u32, minor: u32, patch: u32) -> slicer_ir::SemVer {
    slicer_ir::SemVer {
        major,
        minor,
        patch,
    }
}

fn run(layer: u32, raft: bool) -> InfillOutputBuilder {
    run_with_config(layer, raft, &[])
}

fn run_with_config(layer: u32, raft: bool, config: &[(&str, f64)]) -> InfillOutputBuilder {
    let mut region = SliceRegionViewBuilder::new()
        .object_id("fixture")
        .region_id(0)
        .z(0.2)
        .add_polygon(square())
        .build();
    region.set_held_claims(vec!["claim:raft-fill".to_string()]);
    let paint = PaintRegionLayerView::new(layer)
        .with_support_plan(plan())
        .with_is_raft(raft);
    let config = config
        .iter()
        .map(|(key, value)| ((*key).to_string(), slicer_ir::ConfigValue::Float(*value)))
        .collect();
    let config_view = ConfigView::from_map(config);
    let module = RaftDefault::from_config(&config_view).unwrap();
    let mut output = InfillOutputBuilder::new();
    module
        .run_infill(layer, &[region], &paint, &mut output, &config_view)
        .unwrap();
    output
}

fn run_wasm(layer: u32) -> Vec<Vec<ExPolygon>> {
    let mut arena = LayerArena::new();
    let region = SlicedRegion {
        object_id: "fixture".into(),
        region_id: 0,
        polygons: vec![square()],
        ..Default::default()
    };
    arena
        .set_slice(SliceIR {
            z: 0.2,
            regions: vec![region],
            ..Default::default()
        })
        .unwrap();
    let mut blackboard = slicer_runtime::Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 1);
    blackboard.commit_support_plan(plan()).unwrap();
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let loaded = LoadedModuleBuilder::new(
        "raft-default",
        semver(1, 0, 0),
        "Layer::Infill",
        String::new(),
        std::path::PathBuf::from("modules/core-modules/raft-default/raft-default.wasm"),
    )
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(2, 0, 0))
    .layer_parallel_safe(true)
    .build();
    let pool = Arc::new(
        build_wasm_instance_pool(
            loaded.id(),
            loaded.stage(),
            true,
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .unwrap(),
    );
    let module = CompiledModuleBuilder::new("raft-default")
        .config_view(Arc::new(ConfigView::new()))
        .claims(vec!["claim:raft-fill".into()])
        .build();
    let live = slicer_runtime::CompiledModuleLive::new(
        module.module_id(),
        pool,
        Some(wasm_cache::compiled_component_at(
            &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..\\..\\modules\\core-modules\\raft-default\\raft-default.wasm"),
        )),
        module.claims(),
        Arc::clone(module.config_view()),
    );
    let layer_ir = slicer_ir::GlobalLayer {
        index: layer,
        z: 0.2,
        is_raft: true,
        ..Default::default()
    };
    let commit = <WasmRuntimeDispatcher as slicer_runtime::LayerStageRunner>::run_stage(
        &dispatcher,
        &"Layer::Infill".into(),
        &layer_ir,
        &live,
        layer_input(&blackboard, &arena),
    )
    .unwrap()
    .expect("raft guest must emit a commit");
    apply_for_test(
        &mut arena,
        commit,
        // exhaustive: wasm parity fixture pins every dispatch context field.
        &StageApplyContext {
            stage_id: "Layer::Infill",
            module_id: "raft-default",
            layer_index: layer,
            seam_plan: None,
            config_view: Some(module.config_view()),
            committed_slices: None,
        },
    )
    .unwrap();
    vec![arena.slice().unwrap().regions[0].raft_fill.clone()]
}

#[test]
fn raft_writes_nothing_on_non_raft_layer() {
    assert!(run(3, false).raft_fill().is_empty());
}

#[test]
fn raft_fill_is_deterministic_across_two_runs() {
    let native1 = to_vec(run(0, true).raft_fill()).unwrap();
    let native2 = to_vec(run(0, true).raft_fill()).unwrap();
    let wasm1 = to_vec(&run_wasm(0)).unwrap();
    let wasm2 = to_vec(&run_wasm(0)).unwrap();
    assert_eq!(native1, native2);
    assert_eq!(wasm1, wasm2);
    assert_eq!(native1, wasm1);
    assert_eq!(native1, wasm2);
    assert_eq!(native2, wasm1);
    assert_eq!(native2, wasm2);
}

#[test]
fn raft_band_fill_lines_cover_region() {
    let native = run_with_config(0, true, &[("raft_line_spacing", 0.5)]);
    let native_lines = native
        .raft_fill()
        .iter()
        .flatten()
        .filter(|polygon| polygon.contour.points.len() == 2)
        .count();
    let wasm_lines = run_wasm(0)
        .iter()
        .flatten()
        .filter(|polygon| polygon.contour.points.len() == 2)
        .count();
    assert!(
        native_lines >= 19,
        "expected hatch lines across the 10 mm band"
    );
    assert_eq!(native_lines, wasm_lines);
    assert_eq!(
        to_vec(native.raft_fill()).unwrap(),
        to_vec(&run_wasm(0)).unwrap()
    );
}

fn area(output: &InfillOutputBuilder) -> i64 {
    output
        .raft_fill()
        .iter()
        .flat_map(|group| group.iter())
        .map(|polygon| {
            polygon
                .contour
                .points
                .iter()
                .zip(polygon.contour.points.iter().cycle().skip(1))
                .take(polygon.contour.points.len())
                .map(|(a, b)| (a.x * b.y - b.x * a.y).abs())
                .sum::<i64>()
        })
        .sum()
}

#[test]
fn raft_first_layer_expansion_exceeds_upper_layers() {
    assert!(area(&run(0, true)) > area(&run(1, true)));
    assert!(area(&run(0, true)) > area(&run(2, true)));
    let close = area(&run_with_config(2, true, &[("raft_contact_distance", 0.1)]));
    let far = area(&run_with_config(2, true, &[("raft_contact_distance", 0.8)]));
    assert_ne!(
        close, far,
        "interface footprint must respond to contact distance"
    );
}

#[test]
fn raft_geometry_orders_before_model_layers() {
    let raft_layers = 3u32;
    let mut layers = Vec::new();
    for index in 0..4u32 {
        let module_output = run(index, index < raft_layers);
        let raft_fill: Vec<_> = module_output
            .raft_fill()
            .iter()
            .flatten()
            .cloned()
            .collect();
        assert_eq!(
            raft_fill.is_empty(),
            index >= raft_layers,
            "raft-default output must exist only in the raft band"
        );
        let slice = SliceIR {
            global_layer_index: index,
            z: 0.2 + index as f32 * 0.2,
            regions: vec![SlicedRegion {
                object_id: "fixture".into(),
                region_id: 0,
                raft_fill,
                ..Default::default()
            }],
            ..Default::default()
        };
        let model = InfillIR {
            global_layer_index: index,
            regions: if index >= raft_layers {
                vec![InfillRegion {
                    object_id: "fixture".into(),
                    region_id: 0,
                    sparse_infill: vec![ExtrusionPath3D {
                        role: ExtrusionRole::SparseInfill,
                        points: vec![
                            Point3WithWidth {
                                x: 1.0,
                                y: 1.0,
                                z: slice.z,
                                ..Default::default()
                            },
                            Point3WithWidth {
                                x: 2.0,
                                y: 1.0,
                                z: slice.z,
                                ..Default::default()
                            },
                        ],
                        ..slicer_sdk::test_support::fixtures::extrusion_path3d_base(
                            ExtrusionRole::SparseInfill,
                        )
                    }],
                    ..Default::default()
                }]
            } else {
                Vec::new()
            },
            ..Default::default()
        };
        let (entities, identities) =
            slicer_runtime::layer_executor::assemble_ordered_entities_with_support_identities(
                index,
                index < raft_layers,
                None,
                Some(&model),
                None,
                None,
                Some(&slice),
                Default::default(),
            );
        layers.push(LayerCollectionIR {
            global_layer_index: index,
            z: slice.z,
            ordered_entities: entities,
            support_entity_identities: identities,
            ..Default::default()
        });
    }
    for (index, layer) in layers.iter().enumerate() {
        let positions: Vec<_> = layer
            .ordered_entities
            .iter()
            .enumerate()
            .filter(|(_, entity)| entity.role == ExtrusionRole::RaftInfill)
            .map(|(position, _)| position)
            .collect();
        assert_eq!(
            positions.is_empty(),
            index as u32 >= raft_layers,
            "raft geometry must exist only in the raft band"
        );
    }
    let first_raft_layer = layers.iter().position(|layer| {
        layer
            .ordered_entities
            .iter()
            .any(|entity| entity.role == ExtrusionRole::RaftInfill)
    });
    let first_model_layer = layers.iter().position(|layer| {
        layer
            .ordered_entities
            .iter()
            .any(|entity| entity.role == ExtrusionRole::SparseInfill)
    });
    assert!(first_raft_layer < first_model_layer);
}

fn extrusion_path(role: ExtrusionRole, z: f32) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: vec![
            Point3WithWidth {
                x: 1.0,
                y: 1.0,
                z,
                ..Default::default()
            },
            Point3WithWidth {
                x: 2.0,
                y: 1.0,
                z,
                ..Default::default()
            },
        ],
        ..slicer_sdk::test_support::fixtures::extrusion_path3d_base(role)
    }
}

#[test]
fn raft_band_emits_only_raft_content() {
    let raft_layers = 3u32;
    let native_fill = run(0, true)
        .raft_fill()
        .iter()
        .flatten()
        .cloned()
        .collect::<Vec<_>>();
    let wasm_fill = run_wasm(0).into_iter().flatten().collect::<Vec<_>>();

    for (leg, raft_fill) in [("native", native_fill), ("wasm", wasm_fill)] {
        for index in 0..=raft_layers {
            let z = 0.2 + index as f32 * 0.2;
            let wall_path = extrusion_path(ExtrusionRole::OuterWall, z);
            let perimeter = PerimeterIR {
                global_layer_index: index,
                regions: vec![PerimeterRegion {
                    object_id: "fixture".into(),
                    region_id: 0,
                    walls: vec![
                        // exhaustive: fixture pins every field
                        WallLoop {
                            perimeter_index: 0,
                            loop_type: LoopType::Outer,
                            path: wall_path,
                            width_profile: WidthProfile::default(),
                            feature_flags: vec![WallFeatureFlags::default()],
                            boundary_type: WallBoundaryType::Interior,
                        },
                    ],
                    ..Default::default()
                }],
                ..Default::default()
            };
            let infill = InfillIR {
                global_layer_index: index,
                regions: vec![InfillRegion {
                    object_id: "fixture".into(),
                    region_id: 0,
                    solid_infill: vec![extrusion_path(ExtrusionRole::InternalSolidInfill, z)],
                    ironing: vec![extrusion_path(ExtrusionRole::Ironing, z)],
                    ..Default::default()
                }],
                ..Default::default()
            };
            let support = SupportIR {
                global_layer_index: index,
                entries: vec![SupportEntry {
                    object_id: "fixture".into(),
                    region_id: 0,
                    paths: vec![extrusion_path(ExtrusionRole::SupportMaterial, z)],
                    ..Default::default()
                }],
                ..Default::default()
            };
            let slice = SliceIR {
                global_layer_index: index,
                z,
                regions: vec![SlicedRegion {
                    object_id: "fixture".into(),
                    region_id: 0,
                    raft_fill: if index < raft_layers {
                        raft_fill.clone()
                    } else {
                        Vec::new()
                    },
                    ..Default::default()
                }],
                ..Default::default()
            };

            let (entities, support_identities) =
                slicer_runtime::layer_executor::assemble_ordered_entities_with_support_identities(
                    index,
                    index < raft_layers,
                    Some(&perimeter),
                    Some(&infill),
                    Some(&support),
                    None,
                    Some(&slice),
                    Default::default(),
                );

            if index < raft_layers {
                assert!(!entities.is_empty(), "{leg} raft layer {index} is empty");
                assert!(
                    entities
                        .iter()
                        .all(|entity| entity.role == ExtrusionRole::RaftInfill),
                    "{leg} raft layer {index} emitted non-raft content: {:?}",
                    entities
                        .iter()
                        .map(|entity| &entity.role)
                        .collect::<Vec<_>>()
                );
                assert!(support_identities.is_empty());
            } else {
                assert!(
                    entities
                        .iter()
                        .any(|entity| entity.role == ExtrusionRole::OuterWall),
                    "{leg} first non-band layer lost model walls"
                );
            }
        }
    }
}

#[test]
fn raft_mints_no_anchored_entities() {
    let output = run(0, true);
    assert!(!output.raft_fill().is_empty());
    assert!(output.raft_fill_origins().iter().all(Option::is_some));
    let slice = SliceIR {
        z: 0.2,
        regions: vec![SlicedRegion {
            object_id: "fixture".into(),
            region_id: 0,
            raft_fill: output.raft_fill().iter().flatten().cloned().collect(),
            ..Default::default()
        }],
        ..Default::default()
    };
    let (entities, support_identities) =
        slicer_runtime::layer_executor::assemble_ordered_entities_with_support_identities(
            0,
            true,
            None,
            None,
            None,
            None,
            Some(&slice),
            Default::default(),
        );
    assert!(!entities.is_empty());
    assert!(entities
        .iter()
        .all(|entity| entity.role == ExtrusionRole::RaftInfill));
    assert!(
        support_identities.is_empty(),
        "raft output must mint no anchored/support entities"
    );
    assert!(entities
        .iter()
        .all(|entity| entity.role == ExtrusionRole::RaftInfill));
}

#[test]
fn raft_fill_reaches_sliced_region_after_dispatch() {
    let object_id = "raft-transport-object";
    let region_id = 7;
    let polygon = square();

    // Start at the native SDK boundary: this is the same builder operation a
    // module uses, including the explicit source-region identity.
    let mut builder = InfillOutputBuilder::new();
    builder.begin_region(object_id, region_id);
    builder
        .push_raft_fill(vec![polygon.clone()])
        .expect("native raft-fill push");
    assert_eq!(builder.raft_fill(), &[vec![polygon.clone()]]);
    let origin = builder
        .raft_fill_origins()
        .first()
        .and_then(Option::as_ref)
        .expect("raft-fill push must retain its origin");
    assert_eq!(origin.object_id, object_id);
    assert_eq!(origin.region_id, region_id);

    // Marshal the native builder payload through the same conversion used at
    // the dispatch boundary, then exercise the real LayerStageCommit::Infill
    // consumer.  No raft-default geometry is involved in this transport test.
    let collected = slicer_wasm_host::marshal::InfillOutputCollected {
        raft_fill: builder
            .raft_fill()
            .iter()
            .map(|group| slicer_wasm_host::marshal::ir_to_wit_expolygons(group))
            .collect(),
        raft_fill_origins: vec![Some(slicer_wasm_host::marshal::OriginId {
            object_id: object_id.to_string(),
            region_id,
        })],
        ..Default::default()
    };
    let infill = slicer_wasm_host::marshal::convert_infill_output(&collected, 0, None)
        .expect("infill output marshal");

    let mut arena = LayerArena::new();
    arena
        .set_slice(SliceIR {
            regions: vec![SlicedRegion {
                object_id: object_id.into(),
                region_id,
                ..Default::default()
            }],
            ..Default::default()
        })
        .expect("stage slice");
    apply_for_test(
        &mut arena,
        LayerStageCommit::Infill(infill),
        // exhaustive: transport fixture explicitly pins all apply context metadata.
        &StageApplyContext {
            stage_id: "Layer::Infill",
            module_id: "raft-transport-test",
            layer_index: 0,
            seam_plan: None,
            config_view: None,
            committed_slices: None,
        },
    )
    .expect("infill commit");

    let slice = arena.slice().expect("committed slice");
    assert_eq!(slice.regions[0].raft_fill, vec![polygon]);
}
