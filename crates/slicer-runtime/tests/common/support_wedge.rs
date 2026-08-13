#![allow(dead_code)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::ConfigValue;
use slicer_runtime::run::PrepassContext;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root must be resolvable")
}

fn wedge_path() -> PathBuf {
    workspace_root()
        .join("resources")
        .join("regression_wedge.stl")
}

fn core_modules_dir() -> PathBuf {
    workspace_root().join("modules").join("core-modules")
}

pub fn prepare_wedge_context(support_enabled: bool) -> PrepassContext {
    prepare_wedge_context_with_overrides(support_enabled, &[])
}

pub fn prepare_wedge_context_with_overrides(
    support_enabled: bool,
    overrides: &[(&str, ConfigValue)],
) -> PrepassContext {
    let model = wedge_path();
    assert!(
        model.exists(),
        "regression_wedge.stl must exist at {}",
        model.display()
    );

    let mesh = Arc::new(
        slicer_model_io::load_model(&model).expect("load regression_wedge.stl must succeed"),
    );

    let mut config: HashMap<String, ConfigValue> = HashMap::new();
    config.insert(
        "enable_support".to_string(),
        ConfigValue::Bool(support_enabled),
    );
    for (key, value) in overrides {
        config.insert((*key).to_string(), value.clone());
    }

    let module_dirs = vec![core_modules_dir()];

    let ctx = slicer_runtime::run::prepare_prepass_context(mesh, config, &module_dirs, true)
        .expect("prepare_prepass_context must succeed");

    if support_enabled {
        let plan = ctx
            .blackboard
            .support_plan()
            .expect("support_plan must be committed when enable_support=true");
        assert!(
            !plan.entries.is_empty(),
            "enable_support=true but SupportPlanIR.entries is empty (len={}) for fixture {}",
            plan.entries.len(),
            model.display()
        );
    }

    ctx
}
