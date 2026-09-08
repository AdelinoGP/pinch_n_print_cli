//! Contract tests for the raft module's declared configuration surface.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{ConfigValue, ConfigView, RegionKey, RegionPlan, SemVer};
use slicer_runtime::{
    bind_module_config_view, build_execution_plan, ConfigFieldEntry, ConfigSchema,
    ExecutionModuleBinding, ExecutionPlanError, ExecutionPlanRequest, LoadDiagnostic, LoadedModule,
    LoadedModuleBuilder, SortedStageModules,
};

const KEYS: [(&str, &str, f64, &str); 3] = [
    ("raft_contact_distance", "0.1", 0.1, "Raft Contact Distance"),
    ("raft_expansion", "1.5", 1.5, "Raft Expansion"),
    (
        "raft_first_layer_expansion",
        "2.0",
        2.0,
        "Raft First Layer Expansion",
    ),
];

fn raft_paths() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../modules/core-modules/raft-default/raft-default.toml");
    (manifest.clone(), manifest.with_extension("wasm"))
}

fn schema_module(keys: &[&str]) -> LoadedModule {
    let entries = keys
        .iter()
        .map(|key| {
            (
                (*key).to_string(),
                ConfigFieldEntry {
                    field_type: "float".to_string(),
                    ..Default::default()
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    LoadedModuleBuilder::new(
        "com.core.raft-default",
        SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        "Layer::Infill",
        slicer_schema::TIER_LAYER,
        PathBuf::from("fixtures/mod.wasm"),
    )
    .min_host_version(SemVer {
        major: 0,
        minor: 1,
        patch: 0,
    })
    .min_ir_schema(SemVer {
        major: 4,
        minor: 9,
        patch: 0,
    })
    .max_ir_schema(SemVer {
        major: 5,
        minor: 0,
        patch: 0,
    })
    .config_schema(ConfigSchema { entries })
    .build()
}

#[test]
fn raft_keys_declared_and_wired() {
    let (manifest, wasm) = raft_paths();
    let module =
        slicer_runtime::load_module_from_paths(&manifest, &wasm).expect("raft manifest must load");
    let source = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../modules/core-modules/raft-default/src/lib.rs"),
    )
    .expect("raft source must be readable");

    let mut values = HashMap::new();
    for (key, default, value, display) in KEYS {
        let entry = module
            .config_schema()
            .entries
            .get(key)
            .unwrap_or_else(|| panic!("{key} must be declared"));
        assert_eq!(entry.field_type, "float");
        assert_eq!(entry.default.as_deref(), Some(default));
        assert_eq!(entry.min, Some(0.0));
        assert_eq!(entry.max, None);
        assert_eq!(entry.display.as_deref(), Some(display));
        assert_eq!(entry.group.as_deref(), Some("Raft"));
        assert!(
            source.contains(&format!("value(\"{key}\"")),
            "{key} must be read"
        );
        values.insert(key.to_string(), ConfigValue::Float(value));
    }

    let view = bind_module_config_view(&module, &values);
    assert_eq!(view.get_float("raft_contact_distance"), Some(0.1));
    assert_eq!(view.get_float("raft_expansion"), Some(1.5));
    assert_eq!(view.get_float("raft_first_layer_expansion"), Some(2.0));
}

#[test]
fn undeclared_raft_key_is_rejected_not_defaulted() {
    let module = schema_module(&["raft_contact_distance", "raft_first_layer_expansion"]);
    let view = Arc::new(ConfigView::from_map(HashMap::from([
        ("raft_contact_distance".to_string(), ConfigValue::Float(0.1)),
        ("raft_expansion".to_string(), ConfigValue::Float(1.5)),
    ])));
    let request = ExecutionPlanRequest {
        sorted_stages: vec![SortedStageModules {
            stage_id: "Layer::Infill".to_string(),
            module_ids: vec![module.id().to_string()],
        }],
        module_bindings: vec![ExecutionModuleBinding {
            module,
            config_view: view,
        }],
        global_layers: Arc::new(Vec::new()),
        region_plans: Arc::new(HashMap::<RegionKey, RegionPlan>::new()),
    };
    let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
    let error = build_execution_plan(&request, &mut diagnostics)
        .expect_err("consumed raft key absent from schema must fail closed");
    assert!(matches!(
        error,
        ExecutionPlanError::UndeclaredConfigKey { key, .. } if key == "raft_expansion"
    ));
}
