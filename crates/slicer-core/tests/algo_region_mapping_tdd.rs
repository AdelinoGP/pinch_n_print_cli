#![allow(missing_docs)]

use std::collections::{BTreeMap, HashSet};

use slicer_core::algos::region_mapping::{
    execute_region_mapping_inner, execute_region_mapping_with_cap, RegionMappingError,
    RegionMappingPlanProjection, DEFAULT_REGION_MAP_CAP,
};
use slicer_ir::{
    is_modifier_namespace_id, ActiveRegion, ConfigValue, FacetPaintData, GlobalLayer,
    IndexedTriangleSet, LayerPlanIR, ObjectMesh, PaintLayer, PaintSemantic, PaintValue, Point3,
    RegionKey, RegionMapIR, ResolvedConfig, SemVer,
};
use slicer_scheduler::manifest::RegionSplitValueType;
use slicer_scheduler::region_split::AggregatedRegionSplitEntry;

// ---- helpers ----------------------------------------------------------------

fn sv(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn make_layer_plan() -> LayerPlanIR {
    // 2 layers, 2 objects ("obj_a", "obj_b"), 2 regions each → 4 active_regions per layer
    let mut plan = LayerPlanIR {
        schema_version: sv(1, 0, 0),
        global_layers: Vec::new(),
        object_participation: Default::default(),
    };

    for layer_idx in 0u32..2 {
        let mut active_regions = Vec::new();
        for obj_id in &["obj_a", "obj_b"] {
            for region_id in 0u64..2 {
                active_regions.push(ActiveRegion {
                    object_id: obj_id.to_string(),
                    region_id,
                    resolved_config: ResolvedConfig::default(),
                    effective_layer_height: 0.2,
                    catchup_z_bottom: 0.0,
                    tool_index: 0,
                    ..Default::default()
                });
            }
        }
        plan.global_layers.push(GlobalLayer {
            index: layer_idx,
            z: layer_idx as f32 * 0.2,
            active_regions,
            has_nonplanar: false,
            ..Default::default()
        });
    }
    plan
}

fn no_objects() -> Vec<ObjectMesh> {
    Vec::new()
}

fn no_paint_configs() -> BTreeMap<PaintSemantic, ResolvedConfig> {
    BTreeMap::new()
}

fn empty_aggregated() -> BTreeMap<String, AggregatedRegionSplitEntry> {
    BTreeMap::new()
}

/// Convert a canonical semantic name into the matching `PaintSemantic` variant.
///
/// Mirrors the kernel's `paint_semantic_namespace_key`: `material`,
/// `fuzzy_skin`, `support_enforcer`, `support_blocker` map to their built-in
/// variants; everything else becomes `PaintSemantic::Custom(name)`.
fn semantic_for(name: &str) -> PaintSemantic {
    match name {
        "material" => PaintSemantic::Material,
        "fuzzy_skin" => PaintSemantic::FuzzySkin,
        "support_enforcer" => PaintSemantic::SupportEnforcer,
        "support_blocker" => PaintSemantic::SupportBlocker,
        other => PaintSemantic::Custom(other.to_string()),
    }
}

/// Construct a synthetic `ObjectMesh` whose `paint_data` carries one `PaintLayer`
/// per supplied (semantic_name, distinct_values) pair, with each value tucked
/// into `facet_values` so `scan_paint_data` will surface it.
fn painted_object(object_id: &str, paints: &[(&str, Vec<PaintValue>)]) -> ObjectMesh {
    let layers: Vec<PaintLayer> = paints
        .iter()
        .map(|(sem_name, values)| PaintLayer {
            semantic: semantic_for(sem_name),
            facet_values: values.iter().cloned().map(Some).collect(),
            strokes: Vec::new(),
        })
        .collect();

    let mut obj = ObjectMesh {
        id: object_id.to_string(),
        ..Default::default()
    };
    if !layers.is_empty() {
        obj.paint_data = Some(FacetPaintData { layers });
    }
    obj
}

/// A 1 mm-tall axis-aligned cube spanning `x0..x1` in X, `0..10` in Y and
/// `0..1` in Z (mid-plane slice at z = 0.5 is a non-empty square). Both the
/// kernel mint under test and the Tier-2 geometry split slice modifier meshes
/// with `slicer_core::slice_mesh_ex`, so a fixture mesh must be a solid, not
/// the flat triangle used for object meshes elsewhere.
fn modifier_cube_mesh(x0: f32, x1: f32) -> IndexedTriangleSet {
    let y1 = 10.0f32;
    let z1 = 1.0f32;
    let v = |x: f32, y: f32, z: f32| Point3 { x, y, z };
    IndexedTriangleSet {
        vertices: vec![
            v(x0, 0.0, 0.0),
            v(x1, 0.0, 0.0),
            v(x1, y1, 0.0),
            v(x0, y1, 0.0),
            v(x0, 0.0, z1),
            v(x1, 0.0, z1),
            v(x1, y1, z1),
            v(x0, y1, z1),
        ],
        indices: vec![
            0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 2, 3, 7, 2, 7, 6, 0, 4, 7, 0, 7,
            3, 1, 2, 6, 1, 6, 5,
        ],
    }
}

/// Build a parameter-modifier volume with a single `config_delta` field.
fn modifier_volume(
    id: &str,
    x0: f32,
    x1: f32,
    key: &str,
    value: &str,
) -> slicer_ir::ModifierVolume {
    // exhaustive: ModifierVolume has no Default impl; every field is a fixture input
    slicer_ir::ModifierVolume {
        id: id.to_string(),
        mesh: modifier_cube_mesh(x0, x1),
        config_delta: slicer_ir::ConfigDelta {
            fields: std::collections::HashMap::from([(
                key.to_string(),
                ConfigValue::String(value.to_string()),
            )]),
        },
        priority: 0,
        applies_to: slicer_ir::ModifierScope::AllFeatures,
        // exhaustive: ModifierVolume fixture preserves every field explicitly
    }
}

/// Ticket 18 test (1): two modifiers with DISTINCT `support_type` deltas on one
/// object. The base region keeps the global family (pure base config); each
/// minted sub-region carries only its own modifier's delta, keyed by the same
/// deterministic `(base_region_id, object_id, footprint polygons)` hash the
/// Tier-2 geometry split uses — the region-map entries must match the arena
/// `SlicedRegion`s the pipeline will mint.
#[test]
fn modifier_sub_regions_carry_only_their_own_delta() {
    let mut plan = LayerPlanIR {
        schema_version: sv(1, 0, 0),
        global_layers: Vec::new(),
        object_participation: Default::default(),
    };
    plan.global_layers.push(GlobalLayer {
        index: 0,
        z: 0.5,
        active_regions: vec![ActiveRegion {
            object_id: "obj_a".to_string(),
            region_id: 0,
            resolved_config: ResolvedConfig::default(),
            effective_layer_height: 0.2,
            catchup_z_bottom: 0.0,
            tool_index: 0,
            ..Default::default()
        }],
        has_nonplanar: false,
        ..Default::default()
    });

    // Two side-by-side parameter modifiers: tree(auto) at x 0..10 and
    // normal(auto) at x 10..20 — distinct footprints, distinct deltas.
    let mv_a = modifier_volume("mod-a", 0.0, 10.0, "support_type", "tree(auto)");
    let mv_b = modifier_volume("mod-b", 10.0, 20.0, "support_type", "normal(auto)");
    let objects = vec![ObjectMesh {
        id: "obj_a".to_string(),
        modifier_volumes: vec![mv_a.clone(), mv_b.clone()],
        ..Default::default()
    }];

    let stage_invocations: Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> = vec![];
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    let region_map = execute_region_mapping_with_cap(
        &plan,
        &projection,
        &no_paint_configs(),
        &empty_aggregated(),
        &objects,
        DEFAULT_REGION_MAP_CAP,
    )
    .expect("region mapping must succeed");

    // Base entry + one minted entry per distinct footprint.
    let keys: Vec<&RegionKey> = region_map
        .entries
        .keys()
        .filter(|k| k.object_id == "obj_a")
        .collect();
    assert_eq!(
        keys.len(),
        3,
        "expected base + 2 minted sub-region entries, got {keys:?}"
    );

    // Base region keeps the global family: the pure base config carries no
    // `support_type` extension from either modifier.
    let base_key = RegionKey {
        global_layer_index: 0,
        object_id: "obj_a".to_string(),
        region_id: 0,
        variant_chain: Vec::new(),
    };
    let base_cfg = region_map.config_for(&base_key);
    assert!(
        !base_cfg.extensions.contains_key("support_type"),
        "base region must keep the global family; got extensions={:?}",
        base_cfg.extensions
    );

    // The minted ids must be exactly the Tier-2 ids (same slice + hash) and
    // live in the modifier namespace.
    let slice_at = |mv: &slicer_ir::ModifierVolume| {
        slicer_core::slice_mesh_ex(&mv.mesh, &[0.5])
            .into_iter()
            .next()
            .expect("modifier cube must slice at z=0.5")
    };
    let expected_a = slicer_ir::modifier_sub_region_id(0, "obj_a", &slice_at(&mv_a));
    let expected_b = slicer_ir::modifier_sub_region_id(0, "obj_a", &slice_at(&mv_b));
    assert_ne!(
        expected_a, expected_b,
        "distinct footprints must mint distinct sub-region ids"
    );
    for (sub_id, expected_type) in [(expected_a, "tree(auto)"), (expected_b, "normal(auto)")] {
        assert!(
            slicer_ir::is_modifier_namespace_id(sub_id),
            "minted id {sub_id} must be in the modifier namespace"
        );
        let key = RegionKey {
            global_layer_index: 0,
            object_id: "obj_a".to_string(),
            region_id: sub_id,
            variant_chain: Vec::new(),
        };
        let cfg = region_map.config_for(&key);
        assert_eq!(
            cfg.extensions.get("support_type"),
            Some(&ConfigValue::String(expected_type.to_string())),
            "sub-region {sub_id} must carry only its own modifier's support_type; got extensions={:?}",
            cfg.extensions
        );
    }
}

/// Identical footprints collapse to one modifier sub-region id. The first
/// modifier in document order must own that geometry when priorities tie, so
/// config precedence and geometry ownership select the same modifier.
#[test]
fn identical_modifier_footprints_first_equal_priority_wins() {
    let plan = LayerPlanIR {
        schema_version: sv(1, 0, 0),
        global_layers: vec![GlobalLayer {
            index: 0,
            z: 0.5,
            active_regions: vec![ActiveRegion {
                object_id: "obj_a".to_string(),
                region_id: 0,
                ..Default::default()
            }],
            ..Default::default()
        }],
        ..Default::default()
    };
    let first = modifier_volume("first", 0.0, 10.0, "support_type", "tree(auto)");
    let second = modifier_volume("second", 0.0, 10.0, "support_type", "normal(auto)");
    let objects = vec![ObjectMesh {
        id: "obj_a".to_string(),
        modifier_volumes: vec![
            slicer_ir::ModifierVolume {
                priority: 7,
                ..first
            },
            slicer_ir::ModifierVolume {
                priority: 7,
                ..second
            },
        ],
        ..Default::default()
    }];
    let stage_invocations: Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> = vec![];
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };

    let region_map = execute_region_mapping_with_cap(
        &plan,
        &projection,
        &no_paint_configs(),
        &empty_aggregated(),
        &objects,
        DEFAULT_REGION_MAP_CAP,
    )
    .expect("region mapping must succeed");
    let footprint = slicer_core::slice_mesh_ex(&objects[0].modifier_volumes[0].mesh, &[0.5])
        .into_iter()
        .next()
        .expect("modifier cube must slice at z=0.5");
    let sub_id = slicer_ir::modifier_sub_region_id(0, "obj_a", &footprint);
    let key = RegionKey {
        global_layer_index: 0,
        object_id: "obj_a".to_string(),
        region_id: sub_id,
        variant_chain: Vec::new(),
    };
    assert_eq!(
        region_map.config_for(&key).extensions.get("support_type"),
        Some(&ConfigValue::String("tree(auto)".to_string()))
    );
}

#[test]
fn modifier_parent_bound_only_applies_to_effective_parameter_footprints() {
    let no_modifiers = ObjectMesh {
        id: "obj_a".to_string(),
        ..Default::default()
    };
    assert!(map_single_region_with_object(no_modifiers, 0.5).is_ok());

    let mut support_only = modifier_volume("support-only", 0.0, 10.0, "support_type", "tree(auto)");
    support_only.config_delta.fields.insert(
        "subtype".to_string(),
        ConfigValue::String("support_enforcer".to_string()),
    );
    let support_object = ObjectMesh {
        id: "obj_a".to_string(),
        modifier_volumes: vec![support_only],
        ..Default::default()
    };
    assert!(map_single_region_with_object(support_object, 0.5).is_ok());

    let mut empty_mesh = modifier_volume("empty", 0.0, 10.0, "support_type", "tree(auto)");
    empty_mesh.mesh = IndexedTriangleSet::default();
    let empty_object = ObjectMesh {
        id: "obj_a".to_string(),
        modifier_volumes: vec![empty_mesh],
        ..Default::default()
    };
    assert!(map_single_region_with_object(empty_object, 0.5).is_ok());

    let out_of_layer = ObjectMesh {
        id: "obj_a".to_string(),
        modifier_volumes: vec![modifier_volume(
            "out-of-layer",
            0.0,
            10.0,
            "support_type",
            "tree(auto)",
        )],
        ..Default::default()
    };
    assert!(map_single_region_with_object(out_of_layer, 2.0).is_ok());

    let intersecting = ObjectMesh {
        id: "obj_a".to_string(),
        modifier_volumes: vec![modifier_volume(
            "parameter",
            0.0,
            10.0,
            "support_type",
            "tree(auto)",
        )],
        ..Default::default()
    };
    assert!(matches!(
        map_single_region_with_object(intersecting, 0.5),
        Err(RegionMappingError::ModifierParentRegionIdOutOfRange { .. })
    ));
}

/// Modifier and paint partitions must compose rather than applying the
/// modifier delta to the whole painted variant. The composed key is rooted in
/// the painted parent so the prepass splitter can materialize the matching
/// wall-less child later.
#[test]
fn painted_modifier_entries_are_spatially_composed() {
    let mut plan = single_region_plan("obj_a");
    plan.global_layers[0].z = 0.5;
    let mut object = painted_object("obj_a", &[("material", vec![PaintValue::ToolIndex(1)])]);
    object.modifier_volumes = vec![modifier_volume(
        "modifier",
        0.0,
        10.0,
        "support_type",
        "tree(auto)",
    )];
    let projection = RegionMappingPlanProjection {
        stage_invocations: &[],
    };

    let region_map = execute_region_mapping_with_cap(
        &plan,
        &projection,
        &no_paint_configs(),
        &aggregated(&["material"]),
        &[object],
        DEFAULT_REGION_MAP_CAP,
    )
    .expect("region mapping must succeed");
    let paint_chain = vec![("material".to_string(), PaintValue::ToolIndex(1))];
    let painted_key = region_map
        .entries
        .keys()
        .find(|key| key.variant_chain == paint_chain && !is_modifier_namespace_id(key.region_id))
        .expect("painted parent entry must exist");
    assert!(
        !region_map
            .config_for(painted_key)
            .extensions
            .contains_key("support_type"),
        "modifier support_type must not leak across the painted parent"
    );

    let composed = region_map
        .entries
        .keys()
        .filter(|key| key.variant_chain == paint_chain && is_modifier_namespace_id(key.region_id))
        .collect::<Vec<_>>();
    assert_eq!(
        composed.len(),
        1,
        "painted modifier coverage needs one composed RegionMap entry"
    );
    assert_eq!(
        region_map
            .config_for(composed[0])
            .extensions
            .get("support_type"),
        Some(&ConfigValue::String("tree(auto)".to_string()))
    );
}

#[test]
fn painted_variant_ids_stay_outside_modifier_namespace() {
    let chain = vec![
        ("material".to_string(), PaintValue::ToolIndex(1)),
        ("fuzzy_skin".to_string(), PaintValue::Flag(true)),
    ];
    let id = slicer_core::algos::paint_segmentation::paint_variant_region_id(0, &chain);

    assert!(!is_modifier_namespace_id(id));
    assert!(slicer_ir::modifier_sub_region_id_fits(id));
}

#[test]
fn painted_variant_id_at_modifier_parent_limit_never_becomes_sentinel() {
    let modifier_stride = slicer_ir::MODIFIER_VARIANT_REGION_ID_STRIDE;
    let max_parent = ((1u64 << 63) - 2 - (modifier_stride - 1)) / modifier_stride;
    let chain = vec![("material".to_string(), PaintValue::ToolIndex(1))];

    assert!(slicer_ir::modifier_sub_region_id_fits(max_parent));
    let id = slicer_core::algos::paint_segmentation::paint_variant_region_id(max_parent, &chain);

    assert_ne!(id, slicer_ir::MODIFIER_FOOTPRINT_REGION_ID);
    assert!(!is_modifier_namespace_id(id));
    assert_eq!(
        id / slicer_core::algos::paint_segmentation::PAINT_VARIANT_REGION_ID_STRIDE,
        max_parent
    );
}

/// Build an `aggregated_region_split` BTreeMap declaring the given semantics
/// with a default-ish entry (priority 100, ToolIndex value-type).
fn aggregated(semantics: &[&str]) -> BTreeMap<String, AggregatedRegionSplitEntry> {
    let mut out = BTreeMap::new();
    for name in semantics {
        out.insert(
            (*name).to_string(),
            AggregatedRegionSplitEntry {
                priority: 100,
                value_type: RegionSplitValueType::ToolIndex,
                declaring_modules: Vec::new(),
            },
        );
    }
    out
}

/// One-layer one-object one-region plan, used by the new variant-chain tests
/// where the cross-product cardinality assertions are sensitive to baseline shape.
fn single_region_plan(object_id: &str) -> LayerPlanIR {
    let mut plan = LayerPlanIR {
        schema_version: sv(1, 0, 0),
        global_layers: Vec::new(),
        object_participation: Default::default(),
    };
    plan.global_layers.push(GlobalLayer {
        index: 0,
        z: 0.0,
        active_regions: vec![ActiveRegion {
            object_id: object_id.to_string(),
            region_id: 0,
            resolved_config: ResolvedConfig::default(),
            effective_layer_height: 0.2,
            catchup_z_bottom: 0.0,
            tool_index: 0,
            ..Default::default()
        }],
        has_nonplanar: false,
        ..Default::default()
    });
    plan
}

fn unencodable_modifier_parent_id() -> u64 {
    let stride = slicer_ir::MODIFIER_VARIANT_REGION_ID_STRIDE;
    let max_parent = ((1_u64 << 63) - 2 - (stride - 1)) / stride;
    max_parent + 1
}

fn map_single_region_with_object(
    object: ObjectMesh,
    layer_z: f32,
) -> Result<RegionMapIR, RegionMappingError> {
    let mut plan = single_region_plan(&object.id);
    plan.global_layers[0].z = layer_z;
    plan.global_layers[0].active_regions[0].region_id = unencodable_modifier_parent_id();
    let projection = RegionMappingPlanProjection {
        stage_invocations: &[],
    };
    execute_region_mapping_with_cap(
        &plan,
        &projection,
        &no_paint_configs(),
        &empty_aggregated(),
        &[object],
        DEFAULT_REGION_MAP_CAP,
    )
}

// ---- existing tests ---------------------------------------------------------

/// AC-8: basic shape — 2 layers × 2 objects × 2 regions = 8 entries
#[test]
fn region_map_has_expected_entry_count() {
    let plan = make_layer_plan();
    let stage_invocations: Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> = vec![];
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    let configs = no_paint_configs();
    let agg = empty_aggregated();
    let objects = no_objects();

    let result = execute_region_mapping_with_cap(
        &plan,
        &projection,
        &configs,
        &agg,
        &objects,
        DEFAULT_REGION_MAP_CAP,
    );

    let region_map: RegionMapIR = result.expect("region mapping must succeed");
    // 2 layers × 2 objects × 2 regions = 8 entries
    assert_eq!(
        region_map.entries.len(),
        8,
        "expected 8 entries, got {}",
        region_map.entries.len()
    );
}

/// AC-8: each (layer, object, region) key is present and uniquely addressable
#[test]
fn region_map_keys_are_correct() {
    let plan = make_layer_plan();
    let stage_invocations: Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> = vec![];
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    let configs = no_paint_configs();
    let agg = empty_aggregated();
    let objects = no_objects();

    let region_map = execute_region_mapping_with_cap(
        &plan,
        &projection,
        &configs,
        &agg,
        &objects,
        DEFAULT_REGION_MAP_CAP,
    )
    .unwrap();

    // Spot-check a few expected keys
    let expected_keys = [
        RegionKey {
            global_layer_index: 0,
            object_id: "obj_a".to_string(),
            region_id: 0,
            variant_chain: Vec::new(),
        },
        RegionKey {
            global_layer_index: 0,
            object_id: "obj_a".to_string(),
            region_id: 1,
            variant_chain: Vec::new(),
        },
        RegionKey {
            global_layer_index: 1,
            object_id: "obj_b".to_string(),
            region_id: 0,
            variant_chain: Vec::new(),
        },
        RegionKey {
            global_layer_index: 1,
            object_id: "obj_b".to_string(),
            region_id: 1,
            variant_chain: Vec::new(),
        },
    ];
    for key in &expected_keys {
        assert!(
            region_map.entries.contains_key(key),
            "missing key: {:?}",
            key
        );
    }
}

/// AC-8: cap exceeded produces the correct error variant
#[test]
fn region_map_cap_exceeded_returns_error() {
    let plan = make_layer_plan(); // 8 entries
    let stage_invocations: Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> = vec![];
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    let configs = no_paint_configs();
    let agg = empty_aggregated();
    let objects = no_objects();

    // Cap of 3 is below the 8 entries we have
    let result = execute_region_mapping_with_cap(&plan, &projection, &configs, &agg, &objects, 3);

    match result {
        Err(RegionMappingError::CapExceeded {
            entry_count, cap, ..
        }) => {
            assert_eq!(entry_count, 8);
            assert_eq!(cap, 3);
        }
        other => panic!("expected CapExceeded, got {:?}", other),
    }
}

/// AC-8: empty layer plan produces empty entries map
#[test]
fn empty_layer_plan_produces_empty_map() {
    let plan = LayerPlanIR::default();
    let stage_invocations: Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> = vec![];
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    let configs = no_paint_configs();
    let agg = empty_aggregated();
    let objects = no_objects();

    let region_map = execute_region_mapping_with_cap(
        &plan,
        &projection,
        &configs,
        &agg,
        &objects,
        DEFAULT_REGION_MAP_CAP,
    )
    .unwrap();

    assert!(
        region_map.entries.is_empty(),
        "expected no entries for empty plan"
    );
}

// ---- AC-2 paint-scan test ---------------------------------------------------

/// AC-2: `scan_paint_data` extracts distinct PaintValues per opted-in semantic
/// per object. We drive the private helper indirectly through the kernel — an
/// object painted with 4 distinct ToolIndex values for `material` must yield
/// exactly 5 chains (1 empty + 4 single-value variants) per (layer, region).
#[test]
fn region_mapping_paint_scan() {
    let plan = single_region_plan("obj_a");
    let stage_invocations: Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> = vec![];
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    let configs = no_paint_configs();
    let agg = aggregated(&["material"]);

    // Four distinct tool indices, each repeated multiple times to exercise
    // the de-dup path inside scan_paint_data.
    let paints = vec![(
        "material",
        vec![
            PaintValue::ToolIndex(1),
            PaintValue::ToolIndex(1),
            PaintValue::ToolIndex(2),
            PaintValue::ToolIndex(2),
            PaintValue::ToolIndex(3),
            PaintValue::ToolIndex(4),
        ],
    )];
    let objects = vec![painted_object("obj_a", &paints)];

    let region_map = execute_region_mapping_with_cap(
        &plan,
        &projection,
        &configs,
        &agg,
        &objects,
        DEFAULT_REGION_MAP_CAP,
    )
    .expect("region mapping must succeed");

    // 1 layer × 1 region × (1 + 4) chains = 5 entries.
    assert_eq!(region_map.entries.len(), 5);

    let expected: HashSet<Vec<(String, PaintValue)>> = [
        vec![],
        vec![("material".to_string(), PaintValue::ToolIndex(1))],
        vec![("material".to_string(), PaintValue::ToolIndex(2))],
        vec![("material".to_string(), PaintValue::ToolIndex(3))],
        vec![("material".to_string(), PaintValue::ToolIndex(4))],
    ]
    .into_iter()
    .collect();

    let actual: HashSet<Vec<(String, PaintValue)>> = region_map
        .entries
        .keys()
        .map(|k| k.variant_chain.clone())
        .collect();
    assert_eq!(actual, expected, "variant_chains mismatch");
}

/// Per-tool geometry: a `tool_config:<n>:<key>` overlay is applied to the
/// painted variant chain whose material entry carries `ToolIndex(n)`, at highest
/// precedence (OrcaSlicer filament-override-last). The painted tool-1 region's
/// `RegionPlan.config` must carry the per-tool `line_width`, while tool-2 (no
/// override) and the base (empty) chain keep the default. This is the one place
/// a painted tool is known before perimeter generation, so it is where per-tool
/// geometry composes — no pipeline reordering required.
#[test]
fn region_mapping_applies_per_tool_config_overlay_to_painted_tool() {
    let plan = single_region_plan("obj_a");
    let stage_invocations: Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> = vec![];
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    let configs = no_paint_configs();
    let agg = aggregated(&["material"]);

    // One object painted with material tool 1 and tool 2.
    let paints = vec![(
        "material",
        vec![PaintValue::ToolIndex(1), PaintValue::ToolIndex(2)],
    )];
    let objects = vec![painted_object("obj_a", &paints)];

    // Per-tool override: tool 1 gets a wider line; tool 2 has no override.
    let default_line_width = ResolvedConfig::default().line_width;
    let tool1_width = default_line_width + 0.2;
    let mut tool_configs: BTreeMap<u32, ResolvedConfig> = BTreeMap::new();
    tool_configs.insert(
        1,
        ResolvedConfig {
            line_width: tool1_width,
            ..ResolvedConfig::default()
        },
    );

    let region_map = execute_region_mapping_inner(
        &plan,
        &projection,
        &configs,
        &agg,
        &objects,
        None,
        &tool_configs,
        DEFAULT_REGION_MAP_CAP,
    )
    .expect("region mapping must succeed");

    let key_for = |chain: Vec<(String, PaintValue)>| RegionKey {
        global_layer_index: 0,
        object_id: "obj_a".to_string(),
        region_id: 0,
        variant_chain: chain,
    };

    // Tool 1's painted region carries the per-tool line_width override.
    let tool1_key = key_for(vec![("material".to_string(), PaintValue::ToolIndex(1))]);
    assert_eq!(
        region_map.config_for(&tool1_key).line_width,
        tool1_width,
        "tool-1 painted region must carry the tool_config:1 line_width override"
    );

    // Tool 2 (no override) and the base chain keep the default line_width.
    let tool2_key = key_for(vec![("material".to_string(), PaintValue::ToolIndex(2))]);
    assert_eq!(
        region_map.config_for(&tool2_key).line_width,
        default_line_width,
        "tool-2 has no override and must keep the default line_width"
    );
    let base_key = key_for(vec![]);
    assert_eq!(
        region_map.config_for(&base_key).line_width,
        default_line_width,
        "the base (unpainted) chain must keep the default line_width"
    );
}

/// Per-tool precedence ORDER: when the SAME key (`line_width`) is set by BOTH a
/// per-paint-semantic config (`Material`) AND a `tool_config:<n>:<key>` override,
/// the per-tool value must WIN (precedence `... < per_paint_semantic < per_tool`,
/// OrcaSlicer filament-override-last). The sibling AC-11 test only proves
/// override-vs-default; this test proves the override-vs-override ORDER, which is
/// the actual contract from `design.md` §Selected Approach #3.
#[test]
fn region_mapping_per_tool_config_overrides_paint_semantic_on_same_key() {
    let plan = single_region_plan("obj_a");
    let stage_invocations: Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> = vec![];
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    let agg = aggregated(&["material"]);

    // One object painted material tool 1 and tool 2.
    let paints = vec![(
        "material",
        vec![PaintValue::ToolIndex(1), PaintValue::ToolIndex(2)],
    )];
    let objects = vec![painted_object("obj_a", &paints)];

    let default_line_width = ResolvedConfig::default().line_width;
    // Paint-semantic override for ALL material chains: a DISTINCT line_width.
    let paint_width = default_line_width + 0.5;
    // Per-tool override for tool 1 only: yet another DISTINCT line_width.
    let tool1_width = default_line_width + 0.2;

    let mut configs: BTreeMap<PaintSemantic, ResolvedConfig> = BTreeMap::new();
    configs.insert(
        PaintSemantic::Material,
        ResolvedConfig {
            line_width: paint_width,
            ..ResolvedConfig::default()
        },
    );
    let mut tool_configs: BTreeMap<u32, ResolvedConfig> = BTreeMap::new();
    tool_configs.insert(
        1,
        ResolvedConfig {
            line_width: tool1_width,
            ..ResolvedConfig::default()
        },
    );

    let region_map = execute_region_mapping_inner(
        &plan,
        &projection,
        &configs,
        &agg,
        &objects,
        None,
        &tool_configs,
        DEFAULT_REGION_MAP_CAP,
    )
    .expect("region mapping must succeed");

    let key_for = |chain: Vec<(String, PaintValue)>| RegionKey {
        global_layer_index: 0,
        object_id: "obj_a".to_string(),
        region_id: 0,
        variant_chain: chain,
    };

    // Tool 1: paint-semantic AND per-tool both set line_width — per-tool WINS.
    let tool1_key = key_for(vec![("material".to_string(), PaintValue::ToolIndex(1))]);
    assert_eq!(
        region_map.config_for(&tool1_key).line_width,
        tool1_width,
        "per-tool override must win over the per-paint-semantic value on the same key"
    );

    // Tool 2: only the paint-semantic override applies (no per-tool override).
    let tool2_key = key_for(vec![("material".to_string(), PaintValue::ToolIndex(2))]);
    assert_eq!(
        region_map.config_for(&tool2_key).line_width,
        paint_width,
        "tool-2 has no per-tool override, so the per-paint-semantic value applies"
    );
}

// ---- AC-9 / AC-N1 / AC-N3 tests --------------------------------------------

/// AC-9 (a): an unpainted object with a non-empty aggregated_region_split still
/// produces exactly one entry per (layer, ActiveRegion) with `variant_chain ==
/// vec![]` — the object simply doesn't drive any cross-product axis.
#[test]
fn region_mapping_emits_empty_chain_for_unpainted_object() {
    let plan = make_layer_plan();
    let stage_invocations: Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> = vec![];
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    let configs = no_paint_configs();
    let agg = aggregated(&["material"]);
    // No paint_data on either object.
    let objects = vec![
        ObjectMesh {
            id: "obj_a".to_string(),
            ..Default::default()
        },
        ObjectMesh {
            id: "obj_b".to_string(),
            ..Default::default()
        },
    ];

    let region_map = execute_region_mapping_with_cap(
        &plan,
        &projection,
        &configs,
        &agg,
        &objects,
        DEFAULT_REGION_MAP_CAP,
    )
    .expect("region mapping must succeed");

    // 2 layers × 2 objects × 2 regions × 1 chain = 8 entries.
    assert_eq!(region_map.entries.len(), 8);
    for key in region_map.entries.keys() {
        assert!(
            key.variant_chain.is_empty(),
            "expected empty variant_chain, got {:?}",
            key.variant_chain
        );
    }
}

/// AC-9 (b): a single semantic with N distinct values yields exactly N+1 chains
/// per (layer, ActiveRegion).
#[test]
fn region_mapping_emits_n_plus_1_chains_for_single_semantic_n_distinct_values() {
    let plan = single_region_plan("obj_a");
    let stage_invocations: Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> = vec![];
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    let configs = no_paint_configs();
    let agg = aggregated(&["material"]);

    let paints = vec![(
        "material",
        vec![
            PaintValue::ToolIndex(1),
            PaintValue::ToolIndex(2),
            PaintValue::ToolIndex(3),
        ],
    )];
    let objects = vec![painted_object("obj_a", &paints)];

    let region_map = execute_region_mapping_with_cap(
        &plan,
        &projection,
        &configs,
        &agg,
        &objects,
        DEFAULT_REGION_MAP_CAP,
    )
    .expect("region mapping must succeed");

    // 1 × 1 × (1 + 3) = 4 entries.
    assert_eq!(region_map.entries.len(), 4);
    let expected: HashSet<Vec<(String, PaintValue)>> = [
        vec![],
        vec![("material".to_string(), PaintValue::ToolIndex(1))],
        vec![("material".to_string(), PaintValue::ToolIndex(2))],
        vec![("material".to_string(), PaintValue::ToolIndex(3))],
    ]
    .into_iter()
    .collect();
    let actual: HashSet<Vec<(String, PaintValue)>> = region_map
        .entries
        .keys()
        .map(|k| k.variant_chain.clone())
        .collect();
    assert_eq!(actual, expected);
}

/// AC-9 (c) / AC-3 / AC-4: two semantics (`material` with 2 distinct ToolIndex
/// values, `fuzzy_skin` with 1 Flag value) produce `(1+2) × (1+1) = 6` chains
/// per (layer, ActiveRegion). Verifies SET membership of the enumerated chains.
#[test]
fn region_mapping_two_semantics_produces_cross_product_cardinality() {
    let plan = single_region_plan("obj_a");
    let stage_invocations: Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> = vec![];
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    let configs = no_paint_configs();
    let agg = aggregated(&["material", "fuzzy_skin"]);

    let paints = vec![
        (
            "material",
            vec![PaintValue::ToolIndex(1), PaintValue::ToolIndex(2)],
        ),
        ("fuzzy_skin", vec![PaintValue::Flag(true)]),
    ];
    let objects = vec![painted_object("obj_a", &paints)];

    let region_map = execute_region_mapping_with_cap(
        &plan,
        &projection,
        &configs,
        &agg,
        &objects,
        DEFAULT_REGION_MAP_CAP,
    )
    .expect("region mapping must succeed");

    // 1 × 1 × (1+2) × (1+1) = 6 entries.
    assert_eq!(region_map.entries.len(), 6);

    // Canonical axis order (from BTreeMap keys): "fuzzy_skin" < "material".
    // Expected six chains:
    //   []
    //   [fuzzy_skin=Flag(true)]
    //   [material=ToolIndex(1)]
    //   [material=ToolIndex(2)]
    //   [fuzzy_skin=Flag(true), material=ToolIndex(1)]
    //   [fuzzy_skin=Flag(true), material=ToolIndex(2)]
    let fs = "fuzzy_skin".to_string();
    let mat = "material".to_string();
    let expected: HashSet<Vec<(String, PaintValue)>> = [
        vec![],
        vec![(fs.clone(), PaintValue::Flag(true))],
        vec![(mat.clone(), PaintValue::ToolIndex(1))],
        vec![(mat.clone(), PaintValue::ToolIndex(2))],
        vec![
            (fs.clone(), PaintValue::Flag(true)),
            (mat.clone(), PaintValue::ToolIndex(1)),
        ],
        vec![
            (fs.clone(), PaintValue::Flag(true)),
            (mat.clone(), PaintValue::ToolIndex(2)),
        ],
    ]
    .into_iter()
    .collect();
    let actual: HashSet<Vec<(String, PaintValue)>> = region_map
        .entries
        .keys()
        .map(|k| k.variant_chain.clone())
        .collect();
    assert_eq!(actual, expected, "expected cross-product chains");
}

/// AC-9 (d): within any non-empty chain, the `(semantic, value)` pairs appear
/// in `aggregated_region_split` BTreeMap order (alphabetical). We pick names
/// `alpha_semantic` and `zeta_semantic` to make the canonical order
/// unambiguous and assert that for every chain containing both axes, the
/// indices in `canonical_order` are strictly ascending.
#[test]
fn region_mapping_chains_ordered_by_aggregated_region_split_canonical_order() {
    let plan = single_region_plan("obj_a");
    let stage_invocations: Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> = vec![];
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    let configs = no_paint_configs();
    let agg = aggregated(&["alpha_semantic", "zeta_semantic"]);
    let canonical_order = vec!["alpha_semantic".to_string(), "zeta_semantic".to_string()];

    let paints = vec![
        ("alpha_semantic", vec![PaintValue::ToolIndex(1)]),
        ("zeta_semantic", vec![PaintValue::ToolIndex(9)]),
    ];
    let objects = vec![painted_object("obj_a", &paints)];

    let region_map = execute_region_mapping_with_cap(
        &plan,
        &projection,
        &configs,
        &agg,
        &objects,
        DEFAULT_REGION_MAP_CAP,
    )
    .expect("region mapping must succeed");

    // 1 × 1 × (1+1) × (1+1) = 4 entries.
    assert_eq!(region_map.entries.len(), 4);
    for key in region_map.entries.keys() {
        // Every name in the chain must be a known canonical axis...
        let positions: Vec<usize> = key
            .variant_chain
            .iter()
            .map(|(name, _)| {
                canonical_order
                    .iter()
                    .position(|c| c == name)
                    .expect("semantic name must be in canonical_order")
            })
            .collect();
        // ...and the positions must be strictly ascending.
        for w in positions.windows(2) {
            assert!(
                w[0] < w[1],
                "chain {:?} violates canonical order (positions {:?})",
                key.variant_chain,
                positions
            );
        }
    }
}

/// AC-9 (e): two objects with disjoint paint sets produce chains that only
/// reference their own painted semantic. obj_a has only `material`,
/// obj_b has only `fuzzy_skin`; both semantics are declared globally.
#[test]
fn region_mapping_two_objects_with_disjoint_paint_emit_per_object_chains() {
    let mut plan = LayerPlanIR {
        schema_version: sv(1, 0, 0),
        global_layers: Vec::new(),
        object_participation: Default::default(),
    };
    plan.global_layers.push(GlobalLayer {
        index: 0,
        z: 0.0,
        active_regions: vec![
            ActiveRegion {
                object_id: "obj_a".to_string(),
                region_id: 0,
                resolved_config: ResolvedConfig::default(),
                effective_layer_height: 0.2,
                catchup_z_bottom: 0.0,
                tool_index: 0,
                ..Default::default()
            },
            ActiveRegion {
                object_id: "obj_b".to_string(),
                region_id: 0,
                resolved_config: ResolvedConfig::default(),
                effective_layer_height: 0.2,
                catchup_z_bottom: 0.0,
                tool_index: 0,
                ..Default::default()
            },
        ],
        has_nonplanar: false,
        ..Default::default()
    });

    let stage_invocations: Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> = vec![];
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    let configs = no_paint_configs();
    let agg = aggregated(&["material", "fuzzy_skin"]);

    let objects = vec![
        painted_object(
            "obj_a",
            &[(
                "material",
                vec![PaintValue::ToolIndex(1), PaintValue::ToolIndex(2)],
            )],
        ),
        painted_object("obj_b", &[("fuzzy_skin", vec![PaintValue::Flag(true)])]),
    ];

    let region_map = execute_region_mapping_with_cap(
        &plan,
        &projection,
        &configs,
        &agg,
        &objects,
        DEFAULT_REGION_MAP_CAP,
    )
    .expect("region mapping must succeed");

    // obj_a contributes 1 + 2 = 3 chains; obj_b contributes 1 + 1 = 2 chains.
    assert_eq!(region_map.entries.len(), 5);

    for key in region_map.entries.keys() {
        match key.object_id.as_str() {
            "obj_a" => {
                assert!(
                    key.variant_chain
                        .iter()
                        .all(|(name, _)| name != "fuzzy_skin"),
                    "obj_a chain must not reference fuzzy_skin: {:?}",
                    key.variant_chain
                );
            }
            "obj_b" => {
                assert!(
                    key.variant_chain.iter().all(|(name, _)| name != "material"),
                    "obj_b chain must not reference material: {:?}",
                    key.variant_chain
                );
            }
            other => panic!("unexpected object_id: {}", other),
        }
    }
}

/// AC-9 (f) / AC-7b: empty-aggregation overlay equivalence proof.
///
/// With empty `aggregated_region_split` AND no paint_data AND empty
/// `paint_semantic_configs`, every produced chain is `vec![]` and no overlays
/// could have been applied — so the chain-derived `effective_config` is
/// identical to `stamp_modifier_config_deltas(base, modifiers)`. With no
/// modifier_volumes either, that further collapses to the base config.
#[test]
fn region_mapping_chain_derived_overlay_matches_layer_wide_overlay_when_aggregation_empty() {
    let plan = make_layer_plan();
    let stage_invocations: Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> = vec![];
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    let configs = no_paint_configs();
    let agg = empty_aggregated();
    // No paint_data, no modifier_volumes — every region's effective_config
    // must equal the base ResolvedConfig::default() the ActiveRegions carry.
    let objects = vec![
        ObjectMesh {
            id: "obj_a".to_string(),
            ..Default::default()
        },
        ObjectMesh {
            id: "obj_b".to_string(),
            ..Default::default()
        },
    ];

    let region_map = execute_region_mapping_with_cap(
        &plan,
        &projection,
        &configs,
        &agg,
        &objects,
        DEFAULT_REGION_MAP_CAP,
    )
    .expect("region mapping must succeed");

    let base = ResolvedConfig::default();
    for key in region_map.entries.keys() {
        assert!(
            key.variant_chain.is_empty(),
            "expected empty chain, got {:?}",
            key.variant_chain
        );
        let resolved = region_map.config_for(key);
        assert_eq!(
            resolved, &base,
            "chain-derived effective_config must equal base when overlays/modifiers absent"
        );
    }
}

/// AC-N1: empty aggregated_region_split → every chain is empty, even when the
/// object's paint_data contains values for semantics that just didn't opt in.
#[test]
fn region_mapping_empty_aggregation_no_variants() {
    let plan = make_layer_plan();
    let stage_invocations: Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> = vec![];
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    let configs = no_paint_configs();
    let agg = empty_aggregated();

    let objects = vec![
        painted_object(
            "obj_a",
            &[(
                "material",
                vec![PaintValue::ToolIndex(1), PaintValue::ToolIndex(2)],
            )],
        ),
        painted_object("obj_b", &[("fuzzy_skin", vec![PaintValue::Flag(true)])]),
    ];

    let region_map = execute_region_mapping_with_cap(
        &plan,
        &projection,
        &configs,
        &agg,
        &objects,
        DEFAULT_REGION_MAP_CAP,
    )
    .expect("region mapping must succeed");

    // 2 × 2 × 2 = 8 baseline entries, each with an empty variant_chain.
    assert_eq!(region_map.entries.len(), 8);
    for key in region_map.entries.keys() {
        assert!(
            key.variant_chain.is_empty(),
            "expected empty chain (no opted-in semantics), got {:?}",
            key.variant_chain
        );
    }
}

/// AC-N3: a `PaintValue::Scalar(_)` value on an opted-in semantic must
/// produce a `ScalarInRegionSplitFacetValue` error from the kernel.
#[test]
fn region_mapping_no_scalar_in_variant_chain() {
    let plan = single_region_plan("obj_a");
    let stage_invocations: Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> = vec![];
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    let configs = no_paint_configs();
    let agg = aggregated(&["material"]);
    let objects = vec![painted_object(
        "obj_a",
        &[("material", vec![PaintValue::Scalar(0.5)])],
    )];

    let result = execute_region_mapping_with_cap(
        &plan,
        &projection,
        &configs,
        &agg,
        &objects,
        DEFAULT_REGION_MAP_CAP,
    );

    match result {
        Err(RegionMappingError::ScalarInRegionSplitFacetValue { scalar_bits, .. }) => {
            assert_eq!(scalar_bits, 0.5_f32.to_bits(), "expected scalar=0.5");
        }
        other => panic!("expected ScalarInRegionSplitFacetValue, got {:?}", other),
    }
}

/// AC-5: distinct chains that derive identical `ResolvedConfig` (because
/// `paint_semantic_configs` is empty) must intern to a SHARED `ConfigId`.
/// `RegionMapIR.configs` is bounded by the number of distinct configs, NOT
/// the entry count.
#[test]
fn region_mapping_config_interning() {
    let plan = single_region_plan("obj_a");
    let stage_invocations: Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> = vec![];
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    let configs = no_paint_configs();
    let agg = aggregated(&["material"]);

    let paints = vec![(
        "material",
        vec![
            PaintValue::ToolIndex(1),
            PaintValue::ToolIndex(2),
            PaintValue::ToolIndex(3),
        ],
    )];
    let objects = vec![painted_object("obj_a", &paints)];

    let region_map = execute_region_mapping_with_cap(
        &plan,
        &projection,
        &configs,
        &agg,
        &objects,
        DEFAULT_REGION_MAP_CAP,
    )
    .expect("region mapping must succeed");

    // 4 entries — all four interned ResolvedConfigs are equal (no overlays).
    assert_eq!(region_map.entries.len(), 4);
    assert!(
        region_map.configs.len() <= region_map.entries.len(),
        "interner must not produce more configs than entries"
    );

    // Two distinct variant chains should resolve to the SAME ConfigId.
    let empty_key = region_map
        .entries
        .keys()
        .find(|k| k.variant_chain.is_empty())
        .expect("empty chain entry must exist");
    let other_key = region_map
        .entries
        .keys()
        .find(|k| !k.variant_chain.is_empty())
        .expect("non-empty chain entry must exist");
    assert_eq!(
        region_map.entries[empty_key].config, region_map.entries[other_key].config,
        "equivalent ResolvedConfigs must share a ConfigId"
    );
}

/// AC-4: explicit cross-product entry-count formula check.
/// `entries.len() == layers × active_regions × ∏(1 + K_i)`
/// — concretely 1 × 1 × (1+2) × (1+1) = 6.
#[test]
fn region_mapping_cross_product_entry_count() {
    let plan = single_region_plan("obj_a");
    let stage_invocations: Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> = vec![];
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    let configs = no_paint_configs();
    let agg = aggregated(&["material", "fuzzy_skin"]);

    let paints = vec![
        (
            "material",
            vec![PaintValue::ToolIndex(1), PaintValue::ToolIndex(2)],
        ),
        ("fuzzy_skin", vec![PaintValue::Flag(true)]),
    ];
    let objects = vec![painted_object("obj_a", &paints)];

    let region_map = execute_region_mapping_with_cap(
        &plan,
        &projection,
        &configs,
        &agg,
        &objects,
        DEFAULT_REGION_MAP_CAP,
    )
    .expect("region mapping must succeed");

    // 1 layer × 1 active_region × (1 + 2) × (1 + 1) = 6.
    assert_eq!(region_map.entries.len(), 6);
}

/// AC-3: redirect alias — drives the same 6-chain enumeration as
/// `region_mapping_two_semantics_produces_cross_product_cardinality`, but
/// asserts the chain SET to satisfy AC-3's "enumeration" wording.
#[test]
fn region_mapping_enumerate_chains() {
    let plan = single_region_plan("obj_a");
    let stage_invocations: Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> = vec![];
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    let configs = no_paint_configs();
    let agg = aggregated(&["material", "fuzzy_skin"]);

    let paints = vec![
        (
            "material",
            vec![PaintValue::ToolIndex(1), PaintValue::ToolIndex(2)],
        ),
        ("fuzzy_skin", vec![PaintValue::Flag(true)]),
    ];
    let objects = vec![painted_object("obj_a", &paints)];

    let region_map = execute_region_mapping_with_cap(
        &plan,
        &projection,
        &configs,
        &agg,
        &objects,
        DEFAULT_REGION_MAP_CAP,
    )
    .expect("region mapping must succeed");

    let fs = "fuzzy_skin".to_string();
    let mat = "material".to_string();
    let expected: HashSet<Vec<(String, PaintValue)>> = [
        vec![],
        vec![(fs.clone(), PaintValue::Flag(true))],
        vec![(mat.clone(), PaintValue::ToolIndex(1))],
        vec![(mat.clone(), PaintValue::ToolIndex(2))],
        vec![
            (fs.clone(), PaintValue::Flag(true)),
            (mat.clone(), PaintValue::ToolIndex(1)),
        ],
        vec![
            (fs.clone(), PaintValue::Flag(true)),
            (mat.clone(), PaintValue::ToolIndex(2)),
        ],
    ]
    .into_iter()
    .collect();
    let actual: HashSet<Vec<(String, PaintValue)>> = region_map
        .entries
        .keys()
        .map(|k| k.variant_chain.clone())
        .collect();
    assert_eq!(actual, expected);
}
