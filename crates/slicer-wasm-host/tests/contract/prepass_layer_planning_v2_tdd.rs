use slicer_sdk::traits::LayerPlanningObject;
use slicer_wasm_host::dispatch::{
    adapt_layer_planning_object_configs, validate_layer_planning_object_configs,
};
use slicer_wasm_host::host::prepass_layer_planning::exports::slicer::prepass_layer_planning::layer_planning::ObjectLayerConfig;

fn hand_authored_v2_fixture() -> (Vec<String>, Vec<ObjectLayerConfig>) {
    let object_ids = vec!["object-a".to_string(), "object-b".to_string()];
    let object_configs = vec![
        ObjectLayerConfig {
            object_id: "object-a".to_string(),
            object_height: 12.5,
            layer_height: 0.2,
            first_layer_height: 0.24,
            support_raft_layers: 0,
            layer_zs: Vec::new(),
        },
        ObjectLayerConfig {
            object_id: "object-b".to_string(),
            object_height: 8.75,
            layer_height: 0.16,
            first_layer_height: 0.24,
            support_raft_layers: 2,
            layer_zs: Vec::new(),
        },
    ];

    (object_ids, object_configs)
}

#[test]
pub fn layer_planning_v2_accepts_matching_five_field_object_configs() {
    let (object_ids, object_configs) = hand_authored_v2_fixture();

    validate_layer_planning_object_configs(&object_ids, &object_configs)
        .expect("matching v2 object/config records must be accepted");
}

#[test]
pub fn layer_planning_v2_adapter_carries_every_field_without_config_lookup() {
    // exhaustive: the adapter contract requires independent evidence for every field.
    let typed = vec![LayerPlanningObject {
        object_id: "object-a".to_string(),
        object_height: 12.5,
        layer_height: 0.2,
        first_layer_height: 0.24,
        support_raft_layers: 3,
        layer_zs: Vec::new(),
    }];

    let adapted = adapt_layer_planning_object_configs(&typed);
    assert_eq!(adapted.len(), 1);
    assert_eq!(adapted[0].object_id, typed[0].object_id);
    assert_eq!(adapted[0].object_height, typed[0].object_height);
    assert_eq!(adapted[0].layer_height, typed[0].layer_height);
    assert_eq!(adapted[0].first_layer_height, typed[0].first_layer_height);
    assert_eq!(adapted[0].support_raft_layers, typed[0].support_raft_layers);
}

/// AC-7: the host-derived `layer_zs` schedule survives the `@3.0.0` WIT
/// boundary — the typed `LayerPlanningObject.layer_zs` reaches
/// `ObjectLayerConfig.layer_zs` exactly, in order and without a config lookup
/// re-deriving it from the scalar heights.
///
/// The expected vector is a literal host schedule; `layer_height`/`object_height`
/// are deliberately inconsistent with it (0.2 mm steps cannot reach these tops)
/// so an adapter that ignored the field and recomputed from the scalars could
/// not produce it.
#[test]
pub fn layer_zs_round_trip_through_wit_record() {
    // exhaustive: this WIT round-trip fixture pins every field explicitly
    let typed = vec![LayerPlanningObject {
        object_id: "object-layer-range".to_string(),
        object_height: 1.0,
        layer_height: 0.2,
        first_layer_height: 0.2,
        support_raft_layers: 0,
        layer_zs: vec![0.2, 0.4, 0.5, 0.6, 0.7, 0.8, 1.0],
    }];

    let adapted = adapt_layer_planning_object_configs(&typed);
    assert_eq!(adapted.len(), 1);
    assert_eq!(
        adapted[0].layer_zs,
        vec![0.2, 0.4, 0.5, 0.6, 0.7, 0.8, 1.0],
        "the host-derived top-Z schedule must cross the @3.0.0 record boundary \
         verbatim: same length, same order, same values"
    );

    // The empty schedule is the uniform-fallback signal and must survive as an
    // empty list, not be materialized into a derived uniform sequence.
    let uniform = vec![LayerPlanningObject {
        layer_zs: Vec::new(),
        ..typed[0].clone()
    }];
    let adapted_uniform = adapt_layer_planning_object_configs(&uniform);
    assert!(
        adapted_uniform[0].layer_zs.is_empty(),
        "an empty host schedule must cross the boundary empty, so the guest \
         keeps its uniform formula; got {:?}",
        adapted_uniform[0].layer_zs
    );
}

#[test]
pub fn layer_planning_v2_rejects_mismatched_or_omitted_configs() {
    let (object_ids, mut object_configs) = hand_authored_v2_fixture();
    object_configs[1].object_id = "wrong-object".to_string();

    let mismatch = validate_layer_planning_object_configs(&object_ids, &object_configs)
        .expect_err("a mismatched object/config ID must be rejected");
    assert!(
        mismatch.contains("ID mismatch at index 1"),
        "unexpected mismatch error: {mismatch}"
    );

    let (_, mut object_configs) = hand_authored_v2_fixture();
    object_configs.pop();
    let omission = validate_layer_planning_object_configs(&object_ids, &object_configs)
        .expect_err("an omitted object config must be rejected");
    assert!(
        omission.contains("count mismatch: 2 object IDs, 1 object configs"),
        "unexpected omission error: {omission}"
    );
}
