//! Actual generated SDK adapters, not a local reconstruction of the WASM leg.
#![allow(missing_docs)]

use sdk_infill_postprocess_view_guest as infill_guest;
use sdk_layer_pathopt_guest as guest;

use slicer_ir::*;
use slicer_wasm_host::{
    CompiledModuleLive, LayerStageInput, LayerStageRunner, WasmInstancePool, WasmRuntimeDispatcher,
};
use std::{collections::HashMap, sync::Arc};

fn fixture() -> (SliceIR, PerimeterIR, LayerCollectionIR, Arc<RegionMapIR>) {
    let roles = [
        ExtrusionRole::OuterWall,
        ExtrusionRole::Skirt,
        ExtrusionRole::Brim,
        ExtrusionRole::PrimeTower,
        ExtrusionRole::InternalSolidInfill,
        ExtrusionRole::Custom("third-party/role@1".into()),
    ];
    // exhaustive: distinguish every point field across the actual transport adapters
    let point = Point3WithWidth {
        x: 1.0,
        y: 2.0,
        z: 0.2,
        width: 0.6,
        flow_factor: 0.8,
        overhang_quartile: Some(2),
        dist_to_top_mm: 3.0,
        overhang_distance_mm: Some(0.7),
    };
    // exhaustive: distinctive transport fixture preserves all path fields
    let path = ExtrusionPath3D {
        points: vec![point],
        role: ExtrusionRole::OuterWall,
        speed_factor: 0.9,
        tool_index: Some(2),
        order_lock: Some(77),
    };
    let chain = vec![("material".into(), PaintValue::ToolIndex(2))];
    let slice = SliceIR {
        regions: vec![SlicedRegion {
            object_id: "obj".into(),
            region_id: 9,
            variant_chain: chain.clone(),
            ..Default::default()
        }],
        z: 0.2,
        ..Default::default()
    };
    let mut perimeter = PerimeterIR {
        regions: vec![PerimeterRegion {
            object_id: "obj".into(),
            region_id: 9,
            variant_chain: chain.clone(),
            // exhaustive: wall projection fixture has no safe Default
            walls: vec![WallLoop {
                perimeter_index: 3,
                loop_type: LoopType::Outer,
                path: path.clone(),
                width_profile: WidthProfile { widths: vec![0.6] },
                feature_flags: vec![WallFeatureFlags::default()],
                boundary_type: WallBoundaryType::ExteriorSurface,
            }],
            seam_candidates: vec![SeamCandidate {
                position: point,
                score: 0.25,
                reason: SeamReason::Sharp,
            }],
            resolved_seam: Some(SeamPosition {
                point,
                wall_index: 3,
            }),
            ..Default::default()
        }],
        ..Default::default()
    };
    let wall = perimeter.regions[0].walls[0].clone();
    perimeter.regions[0]
        .walls
        .extend(roles[1..].iter().map(|role| WallLoop {
            path: ExtrusionPath3D {
                role: role.clone(),
                ..wall.path.clone()
            },
            ..wall.clone()
        }));
    let collection = LayerCollectionIR {
        ordered_entities: roles
            .into_iter()
            .enumerate()
            .map(|(index, role)| {
                // exhaustive: ordered transport fixture pins every entity field
                PrintEntity {
                    entity_id: 5 + index as u64,
                    path: ExtrusionPath3D {
                        role: role.clone(),
                        ..path.clone()
                    },
                    role,
                    region_key: RegionKey {
                        global_layer_index: 0,
                        object_id: "obj".into(),
                        region_id: 9,
                        variant_chain: chain.clone(),
                    },
                    topo_order: index as u32,
                    tool_index: 2,
                }
            })
            .collect(),
        ..Default::default()
    };
    let mut map = RegionMapIR::default();
    let mut resolved = ResolvedConfig::default();
    resolved
        .extensions
        .insert("infill_anchor_max".into(), ConfigValue::Float(0.0));
    let id = map.intern_config(resolved);
    map.entries.insert(
        RegionKey {
            global_layer_index: 0,
            object_id: "obj".into(),
            region_id: 9,
            variant_chain: chain,
        },
        RegionPlan {
            config: id,
            ..Default::default()
        },
    );
    (slice, perimeter, collection, Arc::new(map))
}

fn observation(native: bool, infill: bool) -> String {
    observation_with_region_map(native, infill, true)
}

fn observation_with_region_map(native: bool, infill: bool, with_map: bool) -> String {
    observation_with_alias_owner(native, infill, with_map, None)
}

fn observation_with_alias_owner(
    native: bool,
    infill: bool,
    with_map: bool,
    alias_owner: Option<bool>,
) -> String {
    let (mut slice, perimeter, collection, mut map) = fixture();
    if infill {
        let polygon = ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(1.0, 2.0),
                    Point2::from_mm(3.0, 2.0),
                    Point2::from_mm(2.0, 4.0),
                ],
            },
            holes: Vec::new(),
        };
        slice.regions[0].sparse_infill_area = vec![polygon.clone()];
        slice.regions[0].top_solid_fill = vec![polygon.clone()];
        slice.regions[0].bottom_solid_fill = vec![polygon.clone()];
        slice.regions[0].bridge_areas = vec![polygon.clone()];
        slice.regions[0].raft_fill = vec![polygon];
        slice.regions.push(SlicedRegion {
            object_id: "obj".into(),
            region_id: 9_000_001,
            variant_chain: vec![
                ("material".into(), PaintValue::ToolIndex(3)),
                ("custom".into(), PaintValue::Custom("payload".into())),
            ],
            ..Default::default()
        });
        slice.regions.push(SlicedRegion {
            object_id: "missing".into(),
            region_id: 4,
            ..Default::default()
        });
    }
    let engine = crate::common::wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(engine.clone());
    let component = if native {
        None
    } else {
        Some(Arc::new(
            engine
                .compile_component(
                    &std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
                        if infill {
                            "test-guests/sdk-infill-postprocess-view-guest.component.wasm"
                        } else {
                            "test-guests/sdk-layer-pathopt-guest.component.wasm"
                        },
                    ))
                    .expect("SDK guest artifact required"),
                )
                .expect("compile actual SDK guest"),
        ))
    };
    let module_id = "sdk-view-parity".to_owned();
    let mut config = Arc::new(ConfigView::from_map(HashMap::from([
        ("emit_view_witness".into(), ConfigValue::Int(1)),
        ("infill_anchor_max".into(), ConfigValue::Float(20.0)),
        ("layer_height".into(), ConfigValue::Float(0.37)),
        ("infill_density".into(), ConfigValue::Float(0.31)),
    ])));
    if let Some(owner) = alias_owner {
        use slicer_config::{
            assemble_registry, resolve_scope_stack, ConfigIngestor, ConfigScope, ExpansionContext,
            HostChannels, ModuleDeclaration, ResolutionTarget,
        };
        use slicer_scheduler::{bind_module_config_view, LoadedModuleBuilder};
        let alias_schema = ConfigSchema {
            entries: std::collections::BTreeMap::from([
                (
                    "support_overhang_angle".into(),
                    ConfigFieldEntry {
                        field_type: "float".into(),
                        default: Some("30".into()),
                        ..Default::default()
                    },
                ),
                (
                    "emit_view_witness".into(),
                    ConfigFieldEntry {
                        field_type: "int".into(),
                        default: Some("1".into()),
                        ..Default::default()
                    },
                ),
            ]),
        };
        let registry = assemble_registry(
            &[ModuleDeclaration {
                module_id: module_id.clone(),
                schema: alias_schema.clone(),
                ..Default::default()
            }],
            &HostChannels::from_live(),
        )
        .unwrap()
        .registry;
        let mut ingestor = ConfigIngestor::new(&registry);
        ingestor
            .ingest_delta(
                ConfigScope::Global,
                &HashMap::from([("support_threshold_angle".into(), ConfigValue::Float(41.0))]),
            )
            .unwrap();
        ingestor
            .ingest_delta(
                ConfigScope::Object("obj".into()),
                &HashMap::from([("support_threshold_angle".into(), ConfigValue::Float(47.0))]),
            )
            .unwrap();
        let scoped = ingestor.finish().scoped;
        let expansion = ExpansionContext {
            nozzle_diameter_mm: 0.4,
            ..Default::default()
        };
        let global =
            resolve_scope_stack(&registry, &scoped, &ResolutionTarget::default(), &expansion)
                .unwrap();
        let regional = resolve_scope_stack(
            &registry,
            &scoped,
            &ResolutionTarget {
                object_id: "obj".into(),
                ..Default::default()
            },
            &expansion,
        )
        .unwrap();
        let mut schema = alias_schema;
        if !owner {
            schema.entries.remove("support_overhang_angle");
            schema.entries.insert(
                "support_threshold_angle".into(),
                ConfigFieldEntry {
                    field_type: "float".into(),
                    ..Default::default()
                },
            );
        }
        let loaded = LoadedModuleBuilder::new(
            &module_id,
            SemVer::default(),
            "Layer::PathOptimization",
            "",
            "unused.wasm",
        )
        .config_schema(schema)
        .build();
        config = bind_module_config_view(&loaded, &global);
        let mut regional_map = RegionMapIR::default();
        let id = regional_map.intern_config(regional);
        regional_map.entries.insert(
            collection.ordered_entities[0].region_key.clone(),
            RegionPlan {
                config: id,
                ..Default::default()
            },
        );
        map = Arc::new(regional_map);
    }
    let mut module = CompiledModuleLive::new(
        &module_id,
        WasmInstancePool::placeholder(),
        component,
        &[],
        config,
    );
    if native {
        module = module.with_native_entry(if infill {
            infill_guest::SdkInfillPostprocessViewGuest::__slicer_native_entry()
        } else {
            guest::SdkLayerPathoptGuest::__slicer_native_entry()
        });
    }
    // exhaustive: stage borrow input has no Default
    let input = LayerStageInput {
        mesh: Arc::new(MeshIR::default()),
        paint_regions: None,
        seam_plan: None,
        support_plan: None,
        lightning_tree_ir: None,
        region_map: with_map.then_some(map),
        slice: Some(&slice),
        perimeter: Some(&perimeter),
        layer_collection: Some(&collection),
        surface_classification: None,
        prepared_regions: None,
        prepared_perimeter_source_regions: None,
        infill: None,
    };
    let error = dispatcher
        .run_stage(
            &if infill {
                "Layer::InfillPostProcess"
            } else {
                "Layer::PathOptimization"
            }
            .to_owned(),
            &GlobalLayer {
                z: 0.2,
                ..Default::default()
            },
            &module,
            input,
        )
        .expect_err("diagnostic SDK guest returns its observed input");
    assert_eq!(
        perimeter.regions[0].walls[0].width_profile.widths,
        vec![0.6],
        "projection must not rewrite committed width profiles"
    );
    assert_eq!(
        perimeter.regions[0].walls[0].path.points[0].dist_to_top_mm, 3.0,
        "projection must not rewrite committed point metadata"
    );
    assert_eq!(
        perimeter.regions[0].seam_candidates[0].reason,
        SeamReason::Sharp,
        "projection must not rewrite committed seam reasons"
    );
    assert!(
        !collection.ordered_entities[0]
            .region_key
            .variant_chain
            .is_empty(),
        "projection must not erase committed variant identity"
    );
    let LayerStageError::FatalModule { message, .. } = error else {
        panic!("unexpected dispatch error: {error:?}")
    };
    message[message
        .find("SDK-VIEW")
        .expect("actual guest witness required")..]
        .to_owned()
}

#[test]
fn actual_sdk_regional_alias_projects_nondefault_canonical_value_only_to_owner() {
    for native in [false, true] {
        let owner = observation_with_alias_owner(native, false, true, Some(true));
        assert!(
            owner.contains("stage_alias=Some(Float(41.0)) stage_canonical=None"),
            "stage owned alias: {owner}"
        );
        assert!(
            owner.contains("alias=Some(Float(47.0)) canonical=None"),
            "regional owned alias must not retain registry default 30: {owner}"
        );
        let nonowner = observation_with_alias_owner(native, false, true, Some(false));
        assert!(
            nonowner.contains("stage_alias=None stage_canonical=Some(Float(41.0))"),
            "stage nonowner: {nonowner}"
        );
        assert!(
            nonowner.contains("alias=None canonical=Some(Float(47.0))"),
            "regional nonowner must not receive alias: {nonowner}"
        );
    }
}

#[test]
fn actual_sdk_infill_postprocess_without_region_map_inherits_declared_stage_settings() {
    let wasm = observation_with_region_map(false, true, false);
    let native = observation_with_region_map(true, true, false);
    assert!(
        wasm.contains("region=obj:9 chain=[(\"material\", ToolIndex(2))] anchor=Some(Float(20.0))"),
        "missing region config must inherit stage anchor: {wasm}"
    );
    assert!(
        wasm.contains("height=Some(Float(0.37)) density=Some(Float(0.31))"),
        "stage height and density must remain bound: {wasm}"
    );
    assert_eq!(native, wasm, "actual SDK stage-only config fallback parity");
}

#[test]
fn actual_sdk_guest_and_native_observe_current_wire_projection() {
    let wasm = observation(false, false);
    let native = observation(true, false);
    assert!(
        wasm.contains(" ordered=OrderedEntityView"),
        "nonempty ordered snapshot: {wasm}"
    );
    // docs/03_wit_and_manifest.md pins lossless reserved-tag decoding in both
    // ordered entity and path views; genuine third-party Custom stays Custom.
    for role in [
        "Skirt",
        "Brim",
        "PrimeTower",
        "InternalSolidInfill",
        "Custom(\"third-party/role@1\")",
    ] {
        assert!(
            wasm.contains(&format!("path_role={role} ")),
            "analytic path role {role}: {wasm}"
        );
        assert!(
            wasm.contains(&format!("role: {role}, start_point")),
            "analytic ordered entity role {role}: {wasm}"
        );
    }
    assert!(
        wasm.contains("anchor=Some(Float(0.0))"),
        "per-region override must survive SDK adapter: {wasm}"
    );
    assert!(
        native.contains("profile=[0.4]"),
        "native must not expose a profile absent from WIT: {native}"
    );
    assert!(
        native.contains("reason: Aligned"),
        "candidate reason is not carried by WIT: {native}"
    );
    assert!(
        native.contains("region_id: 9, variant_chain: []"),
        "ordered identity is wire-observable: {native}"
    );
    assert!(
        wasm.contains("width: 0.6"),
        "actual path widths must survive: {wasm}"
    );
    assert!(
        wasm.contains("score: 0.25, reason: Aligned"),
        "candidate score survives: {wasm}"
    );
    assert!(wasm.contains("width: 0.0, flow_factor: 1.0, overhang_quartile: None, dist_to_top_mm: 0.0, overhang_distance_mm: None"), "candidate metadata is projected: {wasm}");
    assert!(
        wasm.contains("chain=[(\"material\", ToolIndex(2))]"),
        "perimeter identity survives: {wasm}"
    );
    assert!(
        wasm.contains("wall_index: 3"),
        "resolved seam survives: {wasm}"
    );
    assert!(
        wasm.contains("order_lock: Some(77)"),
        "ordering lock survives: {wasm}"
    );
    assert_eq!(
        native, wasm,
        "same module through native and real WASM dispatch"
    );
}

#[test]
fn actual_sdk_infill_postprocess_guest_and_native_observe_donor_projection() {
    let wasm = observation(false, true);
    let native = observation(true, true);
    assert!(
        wasm.contains("anchor=Some(Float(0.0))"),
        "region config: {wasm}"
    );
    assert!(
        wasm.contains("widths: [0.4]"),
        "nonempty wall profile: {wasm}"
    );
    assert!(
        wasm.contains("score: 0.25, reason: Aligned"),
        "nonempty candidates: {wasm}"
    );
    assert!(wasm.contains("wall_index: 3"), "resolved seam: {wasm}");
    assert!(wasm.contains("region=obj:9000001 chain=[(\"material\", ToolIndex(3)), (\"custom\", ToolIndex(0))] anchor=Some(Float(20.0))"), "virtual variant and stage fallback: {wasm}");
    assert!(
        wasm.contains("tool=3 donor=Some(9)"),
        "borrowed nonempty wall donor: {wasm}"
    );
    assert!(wasm.contains("region=missing:4 chain=[] anchor=Some(Float(20.0)) walls=[] candidates=[] resolved=None"), "missing donor: {wasm}");
    assert!(
        wasm.contains("sparse=[ExPolygon")
            && wasm.contains("top=[ExPolygon")
            && wasm.contains("bottom=[ExPolygon")
            && wasm.contains("bridge=[ExPolygon")
            && wasm.contains("raft=[ExPolygon"),
        "nonempty partition payloads: {wasm}"
    );
    assert_eq!(native, wasm, "real SDK infill-postprocess transport parity");
}
