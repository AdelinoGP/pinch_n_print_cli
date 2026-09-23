use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    BoundingBox3, ConfigDelta, ConfigValue, IndexedTriangleSet, MeshIR, ModifierKind,
    ModifierVolume, ObjectMesh, Point3, SemVer, Transform3d,
};
use slicer_runtime::run::{prepare_prepass_context, run_slice_with_collector, SliceRunOptions};

const OBJECT_ID: &str = "obj-a";

fn box_mesh(min: f32, max: f32) -> IndexedTriangleSet {
    let point = |x, y, z| Point3 { x, y, z };
    IndexedTriangleSet {
        vertices: vec![
            point(min, min, min),
            point(max, min, min),
            point(max, max, min),
            point(min, max, min),
            point(min, min, max),
            point(max, min, max),
            point(max, max, max),
            point(min, max, max),
        ],
        indices: vec![
            0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 2, 3, 7, 2, 7, 6, 0, 4, 7, 0, 7,
            3, 1, 2, 6, 1, 6, 5,
        ],
    }
}

fn mesh_with_modifier(kind: ModifierKind, fields: HashMap<String, ConfigValue>) -> Arc<MeshIR> {
    let modifier_mesh = if matches!(
        kind,
        ModifierKind::SupportEnforcer | ModifierKind::SupportBlocker
    ) {
        box_mesh(0.0, 10.0)
    } else {
        box_mesh(2.0, 8.0)
    };
    let modifier = ModifierVolume::new(
        "mod-a".to_owned(),
        modifier_mesh,
        ConfigDelta { fields },
        0,
        kind,
    );
    let mut object = ObjectMesh {
        id: OBJECT_ID.to_owned(),
        mesh: box_mesh(0.0, 10.0),
        transform: Transform3d {
            matrix: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        },
        world_z_extent: Some((0.0, 10.0)),
        ..ObjectMesh::default()
    };
    object.modifier_volumes.push(modifier);
    let mut mesh = MeshIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 200.0,
                y: 200.0,
                z: 200.0,
            },
        },
        ..MeshIR::default()
    };
    mesh.objects.push(object);
    Arc::new(mesh)
}

fn runtime_options(mesh: Arc<MeshIR>) -> SliceRunOptions {
    SliceRunOptions {
        mesh,
        model_label: "typed-modifier-kind-tdd".to_owned(),
        no_default_module_paths: true,
        ..SliceRunOptions::default()
    }
}

fn modifier_delta_snapshot_bytes(mesh: &MeshIR) -> Vec<u8> {
    let mut fields = mesh.objects[0].modifier_volumes[0]
        .config_delta
        .fields
        .iter()
        .map(|(key, value)| (key.as_str(), format!("{value:?}")))
        .collect::<Vec<_>>();
    fields.sort_by(|left, right| left.0.cmp(right.0));

    let mut snapshot = Vec::new();
    for (key, value) in fields {
        snapshot.extend_from_slice(&(key.len() as u64).to_le_bytes());
        snapshot.extend_from_slice(key.as_bytes());
        snapshot.extend_from_slice(&(value.len() as u64).to_le_bytes());
        snapshot.extend_from_slice(value.as_bytes());
    }
    snapshot
}

fn assert_typed_modifier_resolution(context: &slicer_runtime::run::PrepassContext) {
    let layer_plan = context
        .blackboard
        .layer_plan()
        .expect("prepass must publish a layer plan");
    let layer = layer_plan
        .global_layers
        .iter()
        .find(|layer| {
            layer
                .active_regions
                .iter()
                .any(|region| region.region_id != 0)
        })
        .expect("a modifier must produce an active modifier region");
    let base = layer
        .active_regions
        .iter()
        .find(|region| region.region_id == 0)
        .expect("the base region must remain active");
    let base_wall_count = base.resolved_config.wall_count;
    let modifier = layer
        .active_regions
        .iter()
        .find(|region| region.region_id != 0)
        .expect("the modifier region must be active");

    assert_eq!(
        modifier.resolved_config.wall_count, 4,
        "the modifier host field wall_loops must resolve to integer 4"
    );
    // Stamp contract: this modifier region is prepass stamp-composed, and the stamp loop copies
    // every non-routing delta field into the sub-region extensions (`subtype` is the sole
    // reserved-key routing exclusion). The stamped value is the delta field as ingested
    // (`String("4")` here, not the interned `Int(4)`); typed consumption of that same field is
    // proven separately by `wall_count == 4`, so the pair below covers delivery + typed routing.
    assert_eq!(
        modifier.resolved_config.extensions.get("wall_loops"),
        Some(&ConfigValue::String("4".to_owned())),
        "stamped delivery must carry the wall_loops delta field into the modifier region extensions"
    );
    match modifier.resolved_config.extensions.get("bridge_line_width") {
        Some(ConfigValue::String(value)) => assert_eq!(value, "0.75"),
        other => panic!(
            "stamped delivery must carry the bridge_line_width delta field verbatim, got {other:?}"
        ),
    }
    assert_eq!(
        base.resolved_config.wall_count, base_wall_count,
        "modifier resolution must not mutate the base region"
    );
}

#[test]
fn both_entry_points_registry_type_modifier_deltas() {
    let mesh = mesh_with_modifier(
        ModifierKind::ParameterModifier,
        HashMap::from([
            ("wall_loops".to_owned(), ConfigValue::String("4".to_owned())),
            (
                "bridge_line_width".to_owned(),
                ConfigValue::String("0.75".to_owned()),
            ),
        ]),
    );

    let prepass = prepare_prepass_context(Arc::clone(&mesh), HashMap::new(), &[], true, false)
        .expect("prepass entry point must registry-type the modifier delta");
    assert_typed_modifier_resolution(&prepass);
    let slice = run_slice_with_collector(runtime_options(mesh), None)
        .expect("slice entry point must registry-type the same modifier delta");
    assert!(
        slice.gcode_text.contains("; wall_count = 2"),
        "the slice entry point must preserve the base region's wall count"
    );
}

#[test]
fn invalid_modifier_delta_is_rejected_atomically() {
    let denied = mesh_with_modifier(
        ModifierKind::ParameterModifier,
        HashMap::from([
            ("infill_angle".to_owned(), ConfigValue::Float(45.0)),
            (
                "bed_shape".to_owned(),
                ConfigValue::List(vec![ConfigValue::Float(0.0)]),
            ),
        ]),
    );
    let denied_delta = denied.objects[0].modifier_volumes[0]
        .config_delta
        .fields
        .clone();
    let denied_delta_bytes = modifier_delta_snapshot_bytes(&denied);
    let error = match prepare_prepass_context(Arc::clone(&denied), HashMap::new(), &[], true, false)
    {
        Ok(_) => panic!("bed_shape must be denied at modifier scope"),
        Err(error) => error,
    };
    assert_eq!(
        error.to_string(),
        "modifier config ingestion failed: ScopeDenied { key: \"bed_shape\", scope: Modifier }"
    );
    assert_eq!(
        modifier_delta_snapshot_bytes(&denied),
        denied_delta_bytes,
        "rejected modifier ingest must leave the input delta byte-identical"
    );
    let slice_error = run_slice_with_collector(runtime_options(Arc::clone(&denied)), None)
        .expect_err("slice entry point must reject bed_shape at modifier scope");
    assert_eq!(
        slice_error.to_string(),
        "modifier config ingestion failed: ScopeDenied { key: \"bed_shape\", scope: Modifier }"
    );
    assert_eq!(
        modifier_delta_snapshot_bytes(&denied),
        denied_delta_bytes,
        "slice rejection must leave the input delta byte-identical"
    );
    assert_eq!(
        denied.objects[0].modifier_volumes[0].config_delta.fields, denied_delta,
        "the source modifier delta must remain unchanged after both failures"
    );
    // `prepare_prepass_context` and `run_slice_with_collector` (both in
    // `crates/slicer-runtime/src/run.rs`) return only `SliceRunError` on failure;
    // neither exposes its local ScopedConfig/interner or a post-failure
    // blackboard/layer plan. The error carries the scope/key and the immutable
    // input delta is the strongest post-state observable available here;
    // interner atomicity has no public snapshot to assert.

    let out_of_bounds = mesh_with_modifier(
        ModifierKind::ParameterModifier,
        HashMap::from([
            ("infill_angle".to_owned(), ConfigValue::Float(45.0)),
            ("infill_density".to_owned(), ConfigValue::Float(200.0)),
        ]),
    );
    let out_of_bounds_delta = out_of_bounds.objects[0].modifier_volumes[0]
        .config_delta
        .fields
        .clone();
    let out_of_bounds_delta_bytes = modifier_delta_snapshot_bytes(&out_of_bounds);
    let error =
        match prepare_prepass_context(Arc::clone(&out_of_bounds), HashMap::new(), &[], true, false)
        {
            Ok(_) => panic!("an out-of-bounds declared value must reject the whole modifier delta"),
            Err(error) => error,
        };
    assert_eq!(
        error.to_string(),
        "modifier config ingestion failed: BoundsViolation { key: \"infill_density\", value: 200, min: Some(0.0), max: Some(1.0), scope: Modifier }"
    );
    assert_eq!(
        modifier_delta_snapshot_bytes(&out_of_bounds),
        out_of_bounds_delta_bytes,
        "out-of-bounds rejection must leave the input delta byte-identical"
    );
    let slice_error = run_slice_with_collector(runtime_options(Arc::clone(&out_of_bounds)), None)
        .expect_err("slice entry point must reject an out-of-bounds modifier value");
    assert_eq!(
        slice_error.to_string(),
        "modifier config ingestion failed: BoundsViolation { key: \"infill_density\", value: 200, min: Some(0.0), max: Some(1.0), scope: Modifier }"
    );
    assert_eq!(
        modifier_delta_snapshot_bytes(&out_of_bounds),
        out_of_bounds_delta_bytes,
        "slice bounds rejection must leave the input delta byte-identical"
    );
    assert_eq!(
        out_of_bounds.objects[0].modifier_volumes[0]
            .config_delta
            .fields,
        out_of_bounds_delta,
        "the source modifier delta must remain unchanged after both failures"
    );
}

#[test]
fn all_modifier_kinds_keep_their_geometry_routes() {
    let config = HashMap::from([
        ("nozzle_diameter".to_owned(), ConfigValue::Float(0.4)),
        ("line_width".to_owned(), ConfigValue::Float(0.4)),
    ]);

    let parameter = mesh_with_modifier(
        ModifierKind::ParameterModifier,
        HashMap::from([("wall_loops".to_owned(), ConfigValue::String("0".to_owned()))]),
    );
    let context = prepare_prepass_context(Arc::clone(&parameter), config.clone(), &[], true, false)
        .expect("parameter modifier must keep its geometry route");
    let layer = context
        .blackboard
        .layer_plan()
        .expect("parameter modifier must publish a layer plan")
        .global_layers
        .iter()
        .find(|layer| {
            layer
                .active_regions
                .iter()
                .any(|region| region.region_id != 0)
        })
        .expect("parameter modifier must stamp a sub-region");
    let sub_region = layer
        .active_regions
        .iter()
        .find(|region| region.region_id != 0)
        .expect("parameter modifier sub-region must be active");
    assert_eq!(
        sub_region.resolved_config.wall_count, 0,
        "wall_loops=0 must stamp a wall-less parameter sub-region"
    );
    assert_eq!(
        sub_region.resolved_config.extensions.get("wall_loops"),
        Some(&ConfigValue::String("0".to_owned())),
        "the wall_loops delta must be stamped into the parameter sub-region"
    );
    let negative = mesh_with_modifier(ModifierKind::NegativePart, HashMap::new());
    let context = prepare_prepass_context(Arc::clone(&negative), config.clone(), &[], true, false)
        .expect("negative part must keep its geometry route");
    let negative_debug = format!("{:#?}", context.blackboard);
    assert!(
        negative_debug.contains("kind: NegativePart"),
        "negative-part kind must be preserved in the prepared geometry context"
    );

    for kind in [ModifierKind::SupportEnforcer, ModifierKind::SupportBlocker] {
        let context = prepare_prepass_context(
            mesh_with_modifier(kind, HashMap::new()),
            config.clone(),
            &[],
            true,
            false,
        )
        .expect("support paint modifier must keep its geometry route");
        let debug = format!("{:#?}", context.blackboard);
        let semantic_entry = format!("{kind:?}: [");
        assert!(
            debug.contains(&semantic_entry),
            "support modifier must publish its {kind:?} paint semantic entry"
        );
    }
}
