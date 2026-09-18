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
        },
        ObjectLayerConfig {
            object_id: "object-b".to_string(),
            object_height: 8.75,
            layer_height: 0.16,
            first_layer_height: 0.24,
            support_raft_layers: 2,
        },
    ];

    (object_ids, object_configs)
}

#[test]
fn layer_planning_v2_accepts_matching_five_field_object_configs() {
    let (object_ids, object_configs) = hand_authored_v2_fixture();

    validate_layer_planning_object_configs(&object_ids, &object_configs)
        .expect("matching v2 object/config records must be accepted");
}

#[test]
fn layer_planning_v2_adapter_carries_every_field_without_config_lookup() {
    // exhaustive: the adapter contract requires independent evidence for every field.
    let typed = vec![LayerPlanningObject {
        object_id: "object-a".to_string(),
        object_height: 12.5,
        layer_height: 0.2,
        first_layer_height: 0.24,
        support_raft_layers: 3,
    }];

    let adapted = adapt_layer_planning_object_configs(&typed);
    assert_eq!(adapted.len(), 1);
    assert_eq!(adapted[0].object_id, typed[0].object_id);
    assert_eq!(adapted[0].object_height, typed[0].object_height);
    assert_eq!(adapted[0].layer_height, typed[0].layer_height);
    assert_eq!(adapted[0].first_layer_height, typed[0].first_layer_height);
    assert_eq!(adapted[0].support_raft_layers, typed[0].support_raft_layers);
}

#[test]
fn layer_planning_v2_rejects_mismatched_or_omitted_configs() {
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
