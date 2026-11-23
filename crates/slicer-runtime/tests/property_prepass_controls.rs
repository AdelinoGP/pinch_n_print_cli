//! Deterministic controls for the pre-pass packet's runtime-owned seams.
//!
//! Every test here is RNG- and context-independent. They drive the **real**
//! production join — `commit_region_mapping_builtin` → `execute_paint_segmentation`
//! → `region_map_key_for_sliced_region` → `build_layer_selection_index` — because
//! a passing seam in isolation cannot prove that the shared ordered chain
//! identity survives the whole stage sequence.
//!
//! Run with:
//! `cargo test -p slicer-runtime --features property_testing --test property_prepass_controls`

#![allow(missing_docs)]

mod property_prepass_common;

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use slicer_config::{
    assemble_registry, ConfigIngestor, ConfigSchemaRegistry, ConfigScope, ExpansionContext,
    HostChannels, LayerRangeInput, ModuleDeclaration, ScopeDelta, ScopedConfig,
};
use slicer_core::algos::paint_segmentation::execute_paint_segmentation;
use slicer_core::algos::region_mapping::RegionResolutionAuthority;
use slicer_ir::config_schema::{ConfigFieldEntry, ConfigSchema};
use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, FacetPaintData, GlobalLayer, IndexedTriangleSet,
    LayerPlanIR, MeshIR, ObjectMesh, PaintLayer, PaintSemantic, PaintValue, Point2, Point3,
    Polygon, RegionKey, ResolvedConfig, SemVer, SliceIR, SlicedRegion, Transform3d,
};
use slicer_runtime::{commit_region_mapping_builtin, Blackboard, ExecutionPlan};
use slicer_scheduler::region_split::AggregatedRegionSplitEntry;
use slicer_scheduler::RegionSplitValueType;
use slicer_wasm_host::selection::{
    build_layer_selection_index, region_map_key_for_sliced_region, LayerSelectionIndexInput,
    LayerSelectionModuleInput, LayerTargetSource, SelectionError, STAGES,
};

const OBJECT_ID: &str = "obj_plate";
const MODULE_ID: &str = "com.pbt.identity";

/// The approved ordered chain for the discriminating witness:
/// `material` (priority 100) precedes `fuzzy_skin` (priority 200).
fn combined_chain() -> Vec<(String, PaintValue)> {
    vec![
        ("material".to_string(), PaintValue::ToolIndex(0)),
        ("fuzzy_skin".to_string(), PaintValue::Flag(true)),
    ]
}

fn unit_matrix() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    }
}

/// An open square tube `[2,4] × [2,4] × [0,1]` whose four vertical walls carry
/// both paint semantics. Open walls keep the default-tool projection of the
/// top/bottom faces out of the fixture.
fn painted_tube() -> IndexedTriangleSet {
    let mut vertices = Vec::new();
    for z in [0.0_f32, 1.0] {
        for (dx, dy) in [(0.0_f32, 0.0_f32), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0)] {
            vertices.push(Point3 {
                x: 2.0 + dx,
                y: 2.0 + dy,
                z,
            });
        }
    }
    let mut indices = Vec::new();
    for corner in 0..4u32 {
        let a = corner;
        let b = (corner + 1) % 4;
        indices.extend([a, b, b + 4, a, b + 4, a + 4]);
    }
    IndexedTriangleSet { vertices, indices }
}

/// The tube's 2 mm × 2 mm footprint at the layer plane, as a closed contour.
fn footprint_square() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(2.0, 2.0),
                Point2::from_mm(4.0, 2.0),
                Point2::from_mm(4.0, 4.0),
                Point2::from_mm(2.0, 4.0),
            ],
        },
        holes: Vec::new(),
    }
}

/// A mesh carrying `material`/`ToolIndex(0)` and `fuzzy_skin`/`Flag(true)` on
/// every wall facet — a nonempty, fully overlapping paint scene.
fn painted_mesh() -> Arc<MeshIR> {
    let tube = painted_tube();
    let facets = tube.indices.len() / 3;
    Arc::new(MeshIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        objects: vec![ObjectMesh {
            id: OBJECT_ID.to_string(),
            mesh: tube,
            transform: unit_matrix(),
            paint_data: Some(FacetPaintData {
                layers: vec![
                    PaintLayer {
                        semantic: PaintSemantic::Material,
                        facet_values: vec![Some(PaintValue::ToolIndex(0)); facets],
                        strokes: Vec::new(),
                    },
                    PaintLayer {
                        semantic: PaintSemantic::FuzzySkin,
                        facet_values: vec![Some(PaintValue::Flag(true)); facets],
                        strokes: Vec::new(),
                    },
                ],
            }),
            ..ObjectMesh::default()
        }],
        ..MeshIR::default()
    })
}

/// The pre-paint slice for the single layer.
fn pre_paint_slice() -> SliceIR {
    SliceIR {
        schema_version: slicer_ir::CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        z: 0.5,
        regions: vec![SlicedRegion {
            object_id: OBJECT_ID.to_string(),
            region_id: 0,
            polygons: vec![footprint_square()],
            ..SlicedRegion::default()
        }],
    }
}

/// The declared region-split aggregate: `material` 100, `fuzzy_skin` 200.
fn aggregate() -> BTreeMap<String, AggregatedRegionSplitEntry> {
    BTreeMap::from([
        (
            "material".to_string(),
            AggregatedRegionSplitEntry {
                priority: 100,
                value_type: RegionSplitValueType::ToolIndex,
                declaring_modules: Vec::new(),
            },
        ),
        (
            "fuzzy_skin".to_string(),
            AggregatedRegionSplitEntry {
                priority: 200,
                value_type: RegionSplitValueType::Flag,
                declaring_modules: Vec::new(),
            },
        ),
    ])
}

fn layer_plan() -> LayerPlanIR {
    LayerPlanIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layers: vec![GlobalLayer {
            index: 0,
            z: 0.5,
            active_regions: vec![slicer_ir::ActiveRegion {
                object_id: OBJECT_ID.to_string(),
                region_id: 0,
                resolved_config: ResolvedConfig::default(),
                effective_layer_height: 0.2,
                ..slicer_ir::ActiveRegion::default()
            }],
            ..GlobalLayer::default()
        }],
        object_participation: HashMap::new(),
    }
}

fn int_field() -> ConfigFieldEntry {
    ConfigFieldEntry {
        field_type: "int".to_owned(),
        ..ConfigFieldEntry::default()
    }
}

fn float_field() -> ConfigFieldEntry {
    ConfigFieldEntry {
        field_type: "float".to_owned(),
        ..ConfigFieldEntry::default()
    }
}

/// Admit the two observed keys at every scope.
fn registry() -> ConfigSchemaRegistry {
    assemble_registry(
        &[ModuleDeclaration {
            module_id: "dev.pinch.test.prepass".to_owned(),
            schema: ConfigSchema {
                entries: BTreeMap::from([
                    ("wall_count".to_owned(), int_field()),
                    ("infill_density".to_owned(), float_field()),
                ]),
            },
            ..ModuleDeclaration::default()
        }],
        &HostChannels::from_parts(Vec::new(), Vec::new(), Vec::new()),
    )
    .expect("the prepass fixture registry must assemble")
    .registry
}

/// Typed scope world: the layer range supplies the BASE values, the two paint
/// semantics supply their overlays. There is deliberately no tool scope.
fn scoped_world(registry: &ConfigSchemaRegistry) -> ScopedConfig {
    let mut ingestor = ConfigIngestor::new(registry);
    ingestor
        .ingest_layer_ranges(&[
            // exhaustive: transport fixture pins every LayerRangeInput field.
            LayerRangeInput {
                object_id: OBJECT_ID.to_owned(),
                source_index: 0,
                min_z: 0.0,
                max_z: 1.0,
                values: [
                    ("wall_count".to_owned(), "2".to_owned()),
                    ("infill_density".to_owned(), "0.25".to_owned()),
                ]
                .into_iter()
                .collect(),
            },
        ])
        .expect("the admitted layer range must load");
    let mut scoped = ingestor.finish().scoped;
    scoped.deltas.insert(
        ConfigScope::PaintSemantic("fuzzy_skin".to_owned()),
        ScopeDelta {
            values: BTreeMap::from([
                ("wall_count".to_owned(), ConfigValue::Int(3)),
                ("infill_density".to_owned(), ConfigValue::Float(0.5)),
            ]),
        },
    );
    scoped.deltas.insert(
        ConfigScope::PaintSemantic("material".to_owned()),
        ScopeDelta {
            values: BTreeMap::from([("wall_count".to_owned(), ConfigValue::Int(7))]),
        },
    );
    scoped
}

/// Commit the map through the real builtin with the layer-range authority.
fn commit_scene() -> (Arc<MeshIR>, Arc<slicer_ir::RegionMapIR>) {
    let mesh = painted_mesh();
    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 1);
    blackboard
        .commit_layer_plan(Arc::new(layer_plan()))
        .expect("commit layer plan");

    let plan = ExecutionPlan {
        aggregated_region_split: aggregate(),
        ..ExecutionPlan::default()
    };

    let mut resolved_configs: BTreeMap<String, ResolvedConfig> = BTreeMap::new();
    resolved_configs.insert(OBJECT_ID.to_string(), ResolvedConfig::default());
    let default_config = ResolvedConfig::default();
    let paint_configs: BTreeMap<PaintSemantic, ResolvedConfig> = BTreeMap::new();
    let tool_configs: BTreeMap<u32, ResolvedConfig> = BTreeMap::new();

    let registry = registry();
    let scoped = scoped_world(&registry);
    let expansion = ExpansionContext {
        nozzle_diameter_mm: 0.4,
        ..ExpansionContext::default()
    };

    commit_region_mapping_builtin(
        &plan,
        &mut blackboard,
        &resolved_configs,
        &default_config,
        &paint_configs,
        &tool_configs,
        Some(RegionResolutionAuthority {
            registry: &registry,
            scoped: &scoped,
            expansion: &expansion,
        }),
    )
    .expect("commit_region_mapping_builtin must succeed");

    let map = blackboard
        .region_map()
        .expect("RegionMapIR must be committed");
    (mesh, Arc::clone(map))
}

/// AC-2: the real wrapper commits the four chains, material before fuzzy.
#[test]
fn prepass_regionmap_runtime_wrapper() {
    let (_, map) = commit_scene();

    let chains: Vec<Vec<(String, PaintValue)>> = map
        .entries
        .keys()
        .map(|key| key.variant_chain.clone())
        .collect();

    let mut expected = vec![
        vec![],
        vec![("material".to_string(), PaintValue::ToolIndex(0))],
        vec![("fuzzy_skin".to_string(), PaintValue::Flag(true))],
        combined_chain(),
    ];
    expected.sort();
    let mut actual = chains;
    actual.sort();
    assert_eq!(
        actual, expected,
        "the committed map must hold exactly the four named chains"
    );

    // The combined key's ordered vector is the material-then-fuzzy identity.
    let combined = combined_chain();
    assert!(
        map.entries.keys().any(|key| key.variant_chain == combined),
        "the combined chain must be material-then-fuzzy"
    );
    assert!(
        !map.entries
            .keys()
            .any(|key| { key.variant_chain.len() == 2 && key.variant_chain != combined }),
        "no second (name-ordered) combined key may exist"
    );

    // The compatibility paint overlay does not change the chain identity, and
    // the authority path resolved the layered scopes: fuzzy then material for
    // `wall_count` (material wins), fuzzy-only for `infill_density`.
    let combined_key = map
        .entries
        .keys()
        .find(|key| key.variant_chain == combined)
        .expect("combined key")
        .clone();
    let config = map.config_for(&combined_key);
    assert_eq!(config.wall_count, 7, "material's 7 walls must win");
    assert_eq!(
        config.infill_density, 0.5,
        "fuzzy_skin's 0.5 density applies"
    );

    let base_key = RegionKey {
        global_layer_index: 0,
        object_id: OBJECT_ID.to_string(),
        region_id: 0,
        variant_chain: Vec::new(),
    };
    let base = map.config_for(&base_key);
    assert_eq!(base.wall_count, 2, "BASE keeps the layer-range value");
    assert_eq!(
        base.infill_density, 0.25,
        "BASE keeps the layer-range value"
    );
}

/// AC-3 / AC-N4: map → paint → **actual selection** identity, with both
/// mutations rejected by the real builder.
#[test]
fn prepass_paint_map_identity_integration() {
    let (mesh, map) = commit_scene();

    // --- RegionMapping → paint segmentation ------------------------------
    let painted = execute_paint_segmentation(
        Arc::clone(&mesh),
        Arc::new(vec![pre_paint_slice()]),
        Arc::clone(&map),
    )
    .expect("paint segmentation must succeed on the committed map");

    let combined = combined_chain();
    let painted_region = painted[0]
        .regions
        .iter()
        .find(|region| region.variant_chain == combined)
        .expect("the combined painted region must exist")
        .clone();
    assert!(
        !painted_region.polygons.is_empty(),
        "the overlapping paint must produce nonempty geometry"
    );

    // --- Map-space identity ----------------------------------------------
    let map_key = region_map_key_for_sliced_region(0, &painted_region);
    assert!(
        map.entries.contains_key(&map_key),
        "the painted region's map-space key {map_key:?} must exist in the committed map"
    );
    assert_eq!(map_key.region_id, 0, "base region id 0 is the map address");
    assert_eq!(map_key.variant_chain, combined);

    // --- Actual selection -------------------------------------------------
    let invocation_view = ConfigView::from_map(HashMap::from([
        ("wall_count".to_string(), ConfigValue::Int(1)),
        ("infill_density".to_string(), ConfigValue::Float(0.9)),
    ]));
    // exhaustive: static binding fixture pins stage, module, claims and view.
    let module = LayerSelectionModuleInput {
        stage_id: "Layer::Infill",
        module_id: MODULE_ID,
        claims: &[],
        paint_semantics: None,
        invocation_config: &invocation_view,
    };
    let global_layers = vec![GlobalLayer {
        index: 0,
        z: 0.5,
        ..GlobalLayer::default()
    }];
    let painted_slices = vec![painted[0].clone()];

    // exhaustive: every index input is explicitly the real scene.
    let index = build_layer_selection_index(LayerSelectionIndexInput {
        stage_roster: STAGES,
        global_layers: &global_layers,
        modules: std::slice::from_ref(&module),
        slices: &painted_slices,
        region_map: Some(&map),
        support_plan: None,
        perimeters: &[],
    })
    .expect("the committed scene must build a selection index");

    let selection = index
        .selection_for(0, MODULE_ID)
        .expect("the bound module must have a selection");
    let target = selection
        .targets
        .iter()
        .find(|target| {
            matches!(
                &target.source,
                LayerTargetSource::Model { key, .. }
                    if key.variant_chain == combined
            )
        })
        .expect("the combined region must be selected as a model target");
    let LayerTargetSource::Model { key, members } = &target.source else {
        unreachable!("matched on Model above");
    };
    assert_eq!(
        key.region_id, painted_region.region_id,
        "the selected source-space id must be the painted region's own id"
    );
    assert_eq!(
        key.variant_chain, combined,
        "the chain must not be reordered"
    );
    assert_eq!(members, &vec![key.clone()]);
    assert_eq!(
        target.config.get_int("wall_count"),
        Some(7),
        "the selected view must carry material's 7 walls, not BASE's 2 or fuzzy's 3"
    );
    assert_eq!(
        target.config.get_float("infill_density"),
        Some(0.5),
        "the selected view must carry fuzzy_skin's 0.5 density"
    );

    // --- Negative 1: reversed chain, real slice, real builder -------------
    let mut reversed_painted = painted[0].clone();
    let mut reversed = combined.clone();
    reversed.reverse();
    let reversed_region = reversed_painted
        .regions
        .iter_mut()
        .find(|region| region.variant_chain == combined)
        .expect("the combined region must exist");
    reversed_region.variant_chain = reversed.clone();
    let reversed_slices = vec![reversed_painted];
    // exhaustive: every index input is explicitly the real scene.
    let error = build_layer_selection_index(LayerSelectionIndexInput {
        stage_roster: STAGES,
        global_layers: &global_layers,
        modules: std::slice::from_ref(&module),
        slices: &reversed_slices,
        region_map: Some(&map),
        support_plan: None,
        perimeters: &[],
    })
    .expect_err("a reversed chain has no exact map entry");
    assert_eq!(error.stage_id, "Layer::Infill");
    assert_eq!(error.module_id, MODULE_ID);
    assert_eq!(
        error.error,
        SelectionError::MissingModelConfig {
            key: RegionKey {
                global_layer_index: 0,
                object_id: OBJECT_ID.to_string(),
                region_id: 0,
                variant_chain: reversed,
            }
        },
        "the error must name the exact missing map-space key"
    );

    // --- Negative 2: combined map entry removed, slice untouched ----------
    let mut trimmed = (*map).clone();
    let removed = trimmed.entries.remove(&RegionKey {
        global_layer_index: 0,
        object_id: OBJECT_ID.to_string(),
        region_id: 0,
        variant_chain: combined.clone(),
    });
    assert!(removed.is_some(), "the combined entry must exist first");
    // exhaustive: every index input is explicitly the real scene.
    let error = build_layer_selection_index(LayerSelectionIndexInput {
        stage_roster: STAGES,
        global_layers: &global_layers,
        modules: std::slice::from_ref(&module),
        slices: &painted_slices,
        region_map: Some(&trimmed),
        support_plan: None,
        perimeters: &[],
    })
    .expect_err("a removed combined entry must fail exact lookup");
    assert_eq!(error.stage_id, "Layer::Infill");
    assert_eq!(error.module_id, MODULE_ID);
    assert_eq!(
        error.error,
        SelectionError::MissingModelConfig {
            key: RegionKey {
                global_layer_index: 0,
                object_id: OBJECT_ID.to_string(),
                region_id: 0,
                variant_chain: combined,
            }
        }
    );
}

// ---------------------------------------------------------------------------
// Pre-pass wrapper controls: support geometry absence, overhang object
// delivery, and analysis/territory mutations.
// ---------------------------------------------------------------------------

/// `prepass_support_wrapper_absence_controls` — the two real wrapper errors.
#[test]
fn prepass_support_wrapper_absence_controls() {
    use slicer_core::algos::support_geometry::SupportGeometryBuiltinError;
    use slicer_runtime::builtins::support_geometry_producer::commit_support_geometry_builtin;

    // No layer plan yet.
    let mut blackboard = Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 1);
    match commit_support_geometry_builtin(&mut blackboard) {
        Err(SupportGeometryBuiltinError::NoLayerPlan) => {}
        other => panic!("expected NoLayerPlan, got {other:?}"),
    }

    // A plan without committed slices.
    blackboard
        .commit_layer_plan(Arc::new(slicer_ir::LayerPlanIR {
            schema_version: slicer_ir::SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            global_layers: Vec::new(),
            object_participation: std::collections::HashMap::new(),
        }))
        .expect("empty layer plan must commit");
    match commit_support_geometry_builtin(&mut blackboard) {
        Err(SupportGeometryBuiltinError::MissingSliceIR) => {}
        other => panic!("expected MissingSliceIR, got {other:?}"),
    }
}

/// `prepass_overhang_object_delivery` — the real wrapper publishes each
/// object's own previous-layer boundaries and quartile bands.
#[test]
fn prepass_overhang_object_delivery() {
    use slicer_ir::{
        ExPolygon, GlobalLayer, LayerPlanIR, Point2, Polygon, SemVer, SliceIR, SlicedRegion,
    };
    use slicer_runtime::builtins::overhang_annotation_producer::commit_overhang_annotation_builtin;

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
            holes: Vec::new(),
        }
    }

    let layer_plan = LayerPlanIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layers: vec![
            GlobalLayer {
                index: 0,
                z: 0.2,
                ..GlobalLayer::default()
            },
            GlobalLayer {
                index: 1,
                z: 0.4,
                ..GlobalLayer::default()
            },
        ],
        object_participation: std::collections::HashMap::new(),
    };
    let slices = vec![
        SliceIR {
            global_layer_index: 0,
            z: 0.2,
            regions: vec![
                SlicedRegion {
                    object_id: "obj_a".to_owned(),
                    region_id: 0,
                    polygons: vec![rect(0.0, 0.0, 10.0, 10.0)],
                    ..SlicedRegion::default()
                },
                SlicedRegion {
                    object_id: "obj_b".to_owned(),
                    region_id: 0,
                    polygons: vec![rect(20.0, 0.0, 26.0, 6.0)],
                    ..SlicedRegion::default()
                },
            ],
            ..SliceIR::default()
        },
        SliceIR {
            global_layer_index: 1,
            z: 0.4,
            regions: vec![
                SlicedRegion {
                    object_id: "obj_a".to_owned(),
                    region_id: 0,
                    polygons: vec![rect(0.0, 0.0, 11.0, 10.0)],
                    ..SlicedRegion::default()
                },
                SlicedRegion {
                    object_id: "obj_b".to_owned(),
                    region_id: 0,
                    polygons: vec![rect(20.0, 0.0, 27.0, 6.0)],
                    ..SlicedRegion::default()
                },
            ],
            ..SliceIR::default()
        },
    ];

    // The producer iterates the mesh's objects, so both must exist there.
    let mesh = slicer_ir::MeshIR {
        objects: vec![
            slicer_ir::ObjectMesh {
                id: "obj_a".to_owned(),
                ..slicer_ir::ObjectMesh::default()
            },
            slicer_ir::ObjectMesh {
                id: "obj_b".to_owned(),
                ..slicer_ir::ObjectMesh::default()
            },
        ],
        ..slicer_ir::MeshIR::default()
    };
    let mut blackboard = Blackboard::new(Arc::new(mesh), 2);
    blackboard
        .commit_layer_plan(Arc::new(layer_plan))
        .expect("layer plan");
    blackboard
        .commit_surface_classification(Arc::new(slicer_ir::SurfaceClassificationIR::default()))
        .expect("surface classification");
    blackboard
        .commit_slice_ir(Arc::new(slices))
        .expect("slices");

    let mut raw = HashMap::new();
    raw.insert("line_width".to_owned(), ConfigValue::Float(0.4));
    commit_overhang_annotation_builtin(&mut blackboard, &raw)
        .expect("overhang annotation must succeed");

    let published = blackboard
        .surface_classification()
        .expect("the wrapper must publish a classification");
    // Each object's published geometry must be ITS OWN: the previous-boundary
    // entry is that object's previous rectangle, and every band polygon sits
    // inside that object's current rectangle. A foreign-object or current-layer
    // substitute cannot satisfy both.
    let expected_previous = [
        ("obj_a", (0.0_f32, 0.0, 10.0, 10.0)),
        ("obj_b", (20.0, 0.0, 26.0, 6.0)),
    ];
    let expected_current = [
        ("obj_a", (0.0_f32, 0.0, 11.0, 10.0)),
        ("obj_b", (20.0, 0.0, 27.0, 6.0)),
    ];
    for ((object, previous), (_, current)) in expected_previous.iter().zip(expected_current.iter())
    {
        let boundaries = published
            .prev_layer_boundaries
            .get(*object)
            .and_then(|layers| layers.get(&1))
            .unwrap_or_else(|| panic!("{object} must carry layer 1 boundaries"));
        assert_eq!(
            boundaries.len(),
            1,
            "{object}'s previous-boundary entry must be its own single rectangle"
        );
        let xs: Vec<f32> = boundaries[0]
            .contour
            .points
            .iter()
            .map(|point| point.x as f32 / 10_000.0)
            .collect();
        let ys: Vec<f32> = boundaries[0]
            .contour
            .points
            .iter()
            .map(|point| point.y as f32 / 10_000.0)
            .collect();
        let observed = (
            xs.iter().cloned().fold(f32::MAX, f32::min),
            ys.iter().cloned().fold(f32::MAX, f32::min),
            xs.iter().cloned().fold(f32::MIN, f32::max),
            ys.iter().cloned().fold(f32::MIN, f32::max),
        );
        for (got, want) in [
            (observed.0, previous.0),
            (observed.1, previous.1),
            (observed.2, previous.2),
            (observed.3, previous.3),
        ] {
            assert!(
                (got - want).abs() < 1e-3,
                "{object}'s previous-boundary rectangle {observed:?} != {previous:?}"
            );
        }

        let bands = published
            .overhang_quartile_polygons
            .get(*object)
            .and_then(|layers| layers.get(&1))
            .unwrap_or_else(|| panic!("{object} must carry layer 1 bands"));
        assert!(
            bands
                .iter()
                .all(|band| band.quartile >= 1 && band.quartile <= 4),
            "{object}'s quartiles must stay in 1..=4"
        );
        for band in bands {
            for polygon in &band.polygons {
                for point in &polygon.contour.points {
                    let (x, y) = (point.x as f32 / 10_000.0, point.y as f32 / 10_000.0);
                    assert!(
                        x >= current.0 - 1e-3
                            && x <= current.2 + 1e-3
                            && y >= current.1 - 1e-3
                            && y <= current.3 + 1e-3,
                        "{object} band {} leaves its own current rectangle at ({x}, {y})",
                        band.quartile
                    );
                }
            }
        }
    }
}

/// `prepass_support_analysis_territory_controls` — the analysis and territory
/// models reject their named mutations.
#[test]
fn prepass_support_analysis_territory_controls() {
    use slicer_ir::{ExPolygon, Point2, Polygon};

    // --- Analysis: a wrong occupancy key is rejected ----------------------
    let case = property_prepass_common::analysis::AnalysisCase {
        objects: vec!["obj_0".to_owned()],
        layers: vec![property_prepass_common::analysis::AnalysisLayer {
            index: 0,
            regions: vec![(
                0,
                property_prepass_common::analysis::AnalysisRegion {
                    region_id: 0,
                    chain: 0,
                    rect: [0, 0, 2, 2],
                },
            )],
        }],
        enabled: true,
        support_line_width_mm: 0.5,
        no_support_object: None,
    };
    property_prepass_common::analysis::verify(&case)
        .expect("the live analysis witness must satisfy the model");

    let mut disabled = case.clone();
    disabled.enabled = false;
    property_prepass_common::analysis::verify(&disabled)
        .expect("the disabled mode must publish settings only");

    // A different literal width changes the published clearance, so the model
    // must reject a case built with the old value.
    let mut wrong_width = case.clone();
    wrong_width.support_line_width_mm = 0.9;
    property_prepass_common::analysis::verify(&wrong_width)
        .expect("the clearance must follow the declared literal width");

    // Mutations of a REAL analysis must be rejected.
    let analysis = property_prepass_common::analysis::committed(&case)
        .expect("the witness must commit an analysis");
    property_prepass_common::analysis::verify_observation(&case, &analysis)
        .expect("the committed analysis must satisfy the model");

    // 1. A wrong occupancy object.
    let mut moved_occupancy = analysis.clone();
    let (key, polygons) = moved_occupancy
        .model_occupancy
        .iter()
        .next()
        .map(|(key, polygons)| (key.clone(), polygons.clone()))
        .expect("occupancy must be nonempty");
    moved_occupancy.model_occupancy.remove(&key);
    let mut moved_key = key.clone();
    moved_key.object_id = "obj_foreign".to_owned();
    moved_occupancy.model_occupancy.insert(moved_key, polygons);
    assert!(
        property_prepass_common::analysis::verify_observation(&case, &moved_occupancy).is_err(),
        "a wrong occupancy object must fail the model"
    );

    // 2. A changed clearance literal.
    let mut wrong_clearance = analysis.clone();
    wrong_clearance.shared_settings.insert(
        slicer_wasm_host::support_territory::SUPPORT_TERRITORY_CLEARANCE_KEY.to_owned(),
        "1.5".to_owned(),
    );
    assert!(
        property_prepass_common::analysis::verify_observation(&case, &wrong_clearance).is_err(),
        "an omitted or altered clearance must fail the model"
    );

    // 3. A wrong candidate source.
    // (The single-layer witness has no candidates, so build a two-layer one.)
    let candidate_case = property_prepass_common::analysis::AnalysisCase {
        objects: vec!["obj_0".to_owned()],
        layers: vec![
            property_prepass_common::analysis::AnalysisLayer {
                index: 0,
                regions: vec![(
                    0,
                    property_prepass_common::analysis::AnalysisRegion {
                        region_id: 0,
                        chain: 0,
                        rect: [0, 0, 10, 10],
                    },
                )],
            },
            property_prepass_common::analysis::AnalysisLayer {
                index: 1,
                regions: vec![(
                    0,
                    property_prepass_common::analysis::AnalysisRegion {
                        region_id: 0,
                        chain: 0,
                        rect: [0, 0, 11, 10],
                    },
                )],
            },
        ],
        enabled: true,
        support_line_width_mm: 0.5,
        no_support_object: None,
    };
    let candidate_analysis = property_prepass_common::analysis::committed(&candidate_case)
        .expect("the candidate witness must commit");
    property_prepass_common::analysis::verify_observation(&candidate_case, &candidate_analysis)
        .expect("the candidate witness must satisfy the model");
    assert!(
        !candidate_analysis.candidates.is_empty(),
        "the overhanging upper region must publish a candidate"
    );
    let mut wrong_candidate = candidate_analysis.clone();
    wrong_candidate.candidates[0].source.region_id = 99;
    assert!(
        property_prepass_common::analysis::verify_observation(&candidate_case, &wrong_candidate)
            .is_err(),
        "a wrong candidate source must fail the model"
    );

    // 4. A dropped candidate.
    let mut dropped = candidate_analysis.clone();
    dropped.candidates.clear();
    assert!(
        property_prepass_common::analysis::verify_observation(&candidate_case, &dropped).is_err(),
        "a dropped candidate must fail the model"
    );

    // --- Territory: clearance and absent-territory mutations --------------
    use slicer_wasm_host::support_territory::TerritoryClipper;
    let empty = slicer_ir::SupportAnalysisIR::default();
    assert!(
        TerritoryClipper::from_ir(&empty).is_none(),
        "absent territory must not produce a clipper"
    );

    // A fabricated geometry request for absent territory is `None`, not an
    // empty success.
    let body = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: 0, y: 0 },
                Point2 { x: 10_000, y: 0 },
                Point2 {
                    x: 10_000,
                    y: 10_000,
                },
                Point2 { x: 0, y: 10_000 },
            ],
        },
        holes: Vec::new(),
    };
    let analysis = slicer_ir::SupportAnalysisIR {
        support_territory: std::collections::HashMap::from([(
            slicer_ir::SupportGeometryKey {
                global_support_layer_index: 0,
                object_id: "obj_0".to_owned(),
                region_id: 7,
                variant_chain: Vec::new(),
            },
            vec![body.clone()],
        )]),
        family_assignments: std::collections::BTreeMap::from([(
            slicer_ir::slice_ir::SupportSourceKey {
                object_id: "obj_0".to_owned(),
                region_id: 7,
                variant_chain: Vec::new(),
            },
            "family_1".to_owned(),
        )]),
        shared_settings: std::collections::BTreeMap::from([(
            slicer_wasm_host::support_territory::SUPPORT_TERRITORY_CLEARANCE_KEY.to_owned(),
            "0.4".to_owned(),
        )]),
        ..slicer_ir::SupportAnalysisIR::default()
    };
    let clipper = TerritoryClipper::from_ir(&analysis).expect("territory must clip");
    let clipped = clipper
        .clip("obj_0", 0, "0", &[], "family_0", &[body.clone()])
        .expect("a territory-bearing layer must clip");
    assert!(
        clipped.is_empty(),
        "a body fully inside foreign territory must be removed entirely"
    );
    // The own family keeps the whole body.
    let kept = clipper
        .clip("obj_0", 0, "0", &[], "family_1", &[body.clone()])
        .expect("the owning family must clip");
    assert_eq!(kept.len(), 1, "the owning family keeps its territory");
}

/// `prepass_paint_community_priority_witness` — the community priority/tie
/// witness retained in this deterministic regression home.
///
/// **Root cause of the family this file guards:** `execute_region_mapping_inner`
/// used to hand the aggregate's name-keyed `BTreeMap` keys to the enumerator, so
/// a chain's pair order was alphabetical even where the declared priority
/// disagreed (`fuzzy_skin` before `material`). Paint composition inherited that
/// order, and selection matched it exactly — so a name-ordered identity looked
/// perfectly consistent while disagreeing with the registry. The fix computes
/// ascending `(priority, name)` locally; this witness pins the community half
/// (`zeta` 1000 before the tied 1100 pair, which breaks lexically) and
/// `prepass_paint_map_identity_integration` pins the material/fuzzy half.
#[test]
fn prepass_paint_community_priority_witness() {
    use slicer_core::algos::region_mapping::{
        execute_region_mapping_with_cap, RegionMappingPlanProjection,
    };
    use slicer_ir::{
        ActiveRegion, FacetPaintData, GlobalLayer, IndexedTriangleSet, LayerPlanIR, ObjectMesh,
        PaintLayer, PaintSemantic, PaintValue, Point3, RegionMapIR, ResolvedConfig,
    };
    use slicer_scheduler::region_split::AggregatedRegionSplitEntry;
    use slicer_scheduler::RegionSplitValueType;
    use std::collections::{BTreeMap, HashMap};

    let layer_plan = LayerPlanIR {
        schema_version: slicer_ir::SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layers: vec![GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: vec![ActiveRegion {
                object_id: "obj_0".to_owned(),
                region_id: 0,
                resolved_config: ResolvedConfig::default(),
                effective_layer_height: 0.2,
                ..ActiveRegion::default()
            }],
            ..GlobalLayer::default()
        }],
        object_participation: HashMap::new(),
    };
    let object = ObjectMesh {
        id: "obj_0".to_owned(),
        mesh: IndexedTriangleSet {
            vertices: vec![
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3 {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3 {
                    x: 1.0,
                    y: 1.0,
                    z: 0.0,
                },
            ],
            indices: vec![0, 1, 2],
        },
        paint_data: Some(FacetPaintData {
            layers: vec![
                PaintLayer {
                    semantic: PaintSemantic::Custom("com.example.zeta".to_owned()),
                    facet_values: vec![Some(PaintValue::ToolIndex(0))],
                    strokes: Vec::new(),
                },
                PaintLayer {
                    semantic: PaintSemantic::Custom("com.example.alpha".to_owned()),
                    facet_values: vec![Some(PaintValue::Custom("alpha".to_owned()))],
                    strokes: Vec::new(),
                },
                PaintLayer {
                    semantic: PaintSemantic::Custom("com.example.beta".to_owned()),
                    facet_values: vec![Some(PaintValue::Flag(true))],
                    strokes: Vec::new(),
                },
            ],
        }),
        ..ObjectMesh::default()
    };
    let aggregated: BTreeMap<String, AggregatedRegionSplitEntry> = [
        ("com.example.zeta", 1000u32, RegionSplitValueType::ToolIndex),
        (
            "com.example.alpha",
            1100,
            RegionSplitValueType::CustomString,
        ),
        ("com.example.beta", 1100, RegionSplitValueType::Flag),
    ]
    .into_iter()
    .map(|(name, priority, value_type)| {
        (
            name.to_owned(),
            AggregatedRegionSplitEntry {
                priority,
                value_type,
                declaring_modules: Vec::new(),
            },
        )
    })
    .collect();
    let stage_invocations: Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> = Vec::new();
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    let map: RegionMapIR = execute_region_mapping_with_cap(
        &layer_plan,
        &projection,
        &BTreeMap::new(),
        &aggregated,
        &[object],
        64,
    )
    .expect("the community witness must map");

    let full = map
        .entries
        .keys()
        .map(|key| key.variant_chain.clone())
        .find(|chain| chain.len() == 3)
        .expect("the full community chain must exist");
    assert_eq!(
        full,
        vec![
            ("com.example.zeta".to_owned(), PaintValue::ToolIndex(0)),
            (
                "com.example.alpha".to_owned(),
                PaintValue::Custom("alpha".to_owned())
            ),
            ("com.example.beta".to_owned(), PaintValue::Flag(true)),
        ],
        "priority 1000 precedes the tied 1100 pair, which breaks lexically"
    );
}

/// Territory isolation across object, layer and chain on the real clipper.
///
/// Each territory entry is a distinct rectangle, so a clip that borrowed another
/// object's, another layer's or another chain's territory returns visibly wrong
/// geometry.
#[test]
fn prepass_support_territory_isolation() {
    use slicer_ir::slice_ir::SupportSourceKey;
    use slicer_ir::{
        ExPolygon, PaintValue, Point2, Polygon, SupportAnalysisIR, SupportGeometryKey,
    };
    use slicer_wasm_host::support_territory::{TerritoryClipper, SUPPORT_TERRITORY_CLEARANCE_KEY};
    use std::collections::{BTreeMap, HashMap};

    fn rect(x0: i64, y0: i64, x1: i64, y1: i64) -> ExPolygon {
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 { x: x0, y: y0 },
                    Point2 { x: x1, y: y0 },
                    Point2 { x: x1, y: y1 },
                    Point2 { x: x0, y: y1 },
                ],
            },
            holes: Vec::new(),
        }
    }
    fn bounds(polys: &[ExPolygon]) -> (i64, i64, i64, i64) {
        let mut b = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
        for polygon in polys {
            for point in &polygon.contour.points {
                b.0 = b.0.min(point.x);
                b.1 = b.1.min(point.y);
                b.2 = b.2.max(point.x);
                b.3 = b.3.max(point.y);
            }
        }
        b
    }

    let chain = vec![("material".to_owned(), PaintValue::ToolIndex(0))];
    let mut support_territory: HashMap<SupportGeometryKey, Vec<ExPolygon>> = HashMap::new();
    let mut assignments: BTreeMap<SupportSourceKey, String> = BTreeMap::new();
    // (object, layer, chain) -> a one-off rectangle and its own family.
    let entries: [(&str, u32, &[(String, PaintValue)], [i64; 4], &str); 4] = [
        ("obj_0", 0, &[], [0, 0, 10, 10], "family_0"),
        ("obj_0", 1, &[], [20, 0, 30, 10], "family_0"),
        ("obj_0", 0, &chain, [40, 0, 50, 10], "family_0"),
        ("obj_1", 0, &[], [60, 0, 70, 10], "family_0"),
    ];
    for (object, layer, entry_chain, r, family) in entries {
        support_territory.insert(
            SupportGeometryKey {
                global_support_layer_index: layer,
                object_id: object.to_owned(),
                region_id: 7,
                variant_chain: entry_chain.to_vec(),
            },
            vec![rect(r[0], r[1], r[2], r[3])],
        );
        assignments.insert(
            SupportSourceKey {
                object_id: object.to_owned(),
                region_id: 7,
                variant_chain: entry_chain.to_vec(),
            },
            family.to_owned(),
        );
    }
    let analysis = SupportAnalysisIR {
        support_territory,
        family_assignments: assignments,
        shared_settings: BTreeMap::from([(
            SUPPORT_TERRITORY_CLEARANCE_KEY.to_owned(),
            "0.4".to_owned(),
        )]),
        ..SupportAnalysisIR::default()
    };
    let clipper = TerritoryClipper::from_ir(&analysis).expect("territory must clip");
    let body = vec![rect(-100, -100, 200, 200)];

    for (object, layer, entry_chain, expected, _) in entries {
        let region_id = "7";
        let clipped = clipper
            .clip(object, layer, region_id, entry_chain, "family_0", &body)
            .unwrap_or_else(|| panic!("{object}/{layer}/{:?} must clip", entry_chain.len()));
        assert_eq!(
            bounds(&clipped),
            (expected[0], expected[1], expected[2], expected[3]),
            "{object} layer {layer} chain {:?} must keep only its own territory",
            entry_chain.len()
        );
    }
}

/// `prepass_model_controls` (runtime) — the analysis model rejects structural
/// mutations of a real observation.
#[test]
fn prepass_model_controls() {
    use property_prepass_common::analysis::{
        committed, verify_observation, AnalysisCase, AnalysisLayer, AnalysisRegion,
    };

    let case = AnalysisCase {
        objects: vec!["obj_0".to_owned()],
        layers: vec![
            // Layer 0 is WIDER than layer 1, so the object plate (the union of
            // both bounds) is distinguishable from the top footprint.
            AnalysisLayer {
                index: 0,
                regions: vec![(
                    0,
                    AnalysisRegion {
                        region_id: 0,
                        chain: 0,
                        rect: [0, 0, 12, 10],
                    },
                )],
            },
            AnalysisLayer {
                index: 1,
                regions: vec![(
                    0,
                    AnalysisRegion {
                        region_id: 0,
                        chain: 0,
                        rect: [0, 0, 11, 10],
                    },
                )],
            },
        ],
        enabled: true,
        support_line_width_mm: 0.5,
        no_support_object: None,
    };
    let analysis = committed(&case).expect("the witness must commit");
    verify_observation(&case, &analysis).expect("the live witness must satisfy the model");

    // A dropped envelope plate: the model must notice the count.
    let mut dropped_envelope = analysis.clone();
    if dropped_envelope.baseline_feasible_envelope.is_empty() {
        panic!("the witness must publish an envelope plate");
    }
    dropped_envelope.baseline_feasible_envelope.clear();
    assert!(
        verify_observation(&case, &dropped_envelope).is_err(),
        "a dropped envelope plate must fail the model"
    );

    // A foreign termination surface attached to a non-top key.
    let mut foreign_termination = analysis.clone();
    let non_top = foreign_termination
        .model_occupancy
        .keys()
        .find(|key| key.global_support_layer_index == 0)
        .cloned()
        .expect("the witness must have a layer-0 occupancy key");
    foreign_termination
        .termination_surfaces
        .insert(non_top, Vec::new());
    assert!(
        verify_observation(&case, &foreign_termination).is_err(),
        "a non-top termination must fail the model"
    );

    // A wrong occupancy polygon for an existing key.
    let mut wrong_polygons = analysis.clone();
    wrong_polygons
        .model_occupancy
        .values_mut()
        .next()
        .expect("the witness must have an occupancy entry")
        .clear();
    assert!(
        verify_observation(&case, &wrong_polygons).is_err(),
        "an emptied occupancy entry must fail the model"
    );
}

/// `prepass_owned_shrink_and_bound_controls` (runtime) — the analysis value tree
/// survives complication and simplification.
#[test]
fn prepass_owned_shrink_and_bound_controls() {
    use property_prepass_common::analysis::{strategy, verify, AnalysisCase};
    use proptest::strategy::{Strategy, ValueTree};
    use proptest::test_runner::{Config, RngSeed, TestRunner};

    let mut runner = TestRunner::new(Config {
        rng_seed: RngSeed::Fixed(0x5eed_9001),
        cases: 1,
        ..Config::default()
    });
    let mut tree = strategy(3, 2)
        .new_tree(&mut runner)
        .expect("the analysis strategy must build a tree");
    let mut simplifications = 0usize;
    for _ in 0..4 {
        if !tree.complicate() {
            break;
        }
        let case: AnalysisCase = tree.current();
        verify(&case).expect("a complicated analysis case must stay valid");
    }
    for _ in 0..8 {
        if !tree.simplify() {
            break;
        }
        let case: AnalysisCase = tree.current();
        verify(&case).expect("a simplified analysis case must stay valid");
        simplifications += 1;
    }
    assert!(
        simplifications > 0,
        "the analysis tree must expose at least one simplification"
    );
}
