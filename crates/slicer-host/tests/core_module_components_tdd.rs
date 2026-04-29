//! Proves the real on-disk core-module `.wasm` artifacts compile and
//! load as component-model binaries through the production
//! `load_live_modules_for_plan` path (docs/03, docs/04).
//!
//! Floor for the task of "produce real runnable component-model .wasm
//! artifacts for the core modules": this test is green only when the
//! host sees `wasm_component = Some(_)` for `com.core.layer-planner-default`,
//! i.e. the companion `.wasm` is NOT a placeholder and NOT a raw core
//! module (the host only accepts component-model binaries).
//!
//! If this test fails, run: `modules/core-modules/build-core-modules.sh`.

#![allow(missing_docs)]

use std::path::PathBuf;

use slicer_host::load_live_modules_for_plan;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root canonicalize")
}

#[test]
fn layer_planner_default_loads_as_real_component() {
    let core_modules = repo_root().join("modules/core-modules");
    assert!(
        core_modules
            .join("layer-planner-default/layer-planner-default.wasm")
            .exists(),
        "layer-planner-default.wasm missing; run modules/core-modules/build-core-modules.sh"
    );

    let out =
        load_live_modules_for_plan(&[core_modules.clone()], 1).expect("load_live_modules_for_plan");

    let binding = out
        .bindings
        .iter()
        .find(|b| b.module.id == "com.core.layer-planner-default")
        .expect("layer-planner-default binding present after discovery");

    assert!(
        !binding.module.placeholder_wasm,
        "layer-planner-default.wasm must not be a placeholder"
    );
    assert!(
        binding.wasm_component.is_some(),
        "layer-planner-default must compile as a component-model .wasm; \
         load diagnostics = {:?}",
        out.diagnostics
            .iter()
            .filter(|d| d.path.to_string_lossy().contains("layer-planner-default"))
            .collect::<Vec<_>>()
    );
}
