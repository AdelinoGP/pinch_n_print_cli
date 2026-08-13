//! AC-N4: an external classic-perimeters module must remain WASM-dispatched.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use classic_perimeters::ClassicPerimeters;
use slicer_scheduler::{IntegratedModuleRegistration, ModuleProvenance};
use slicer_wasm_host::execution_plan_live::load_live_modules_for_plan_with_integrated;
use tempfile::TempDir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root canonicalize")
}

fn write_module_with_wasm(root: &Path, stem: &str, manifest: &str, wasm_bytes: &[u8]) {
    fs::write(root.join(format!("{stem}.toml")), manifest).expect("write manifest");
    fs::write(root.join(format!("{stem}.wasm")), wasm_bytes).expect("write wasm");
}

fn minimal_component_bytes() -> Vec<u8> {
    wat::parse_str("(component (core module))").expect("wat parse")
}

#[test]
fn hybrid_pilot_external_override_forces_wasm() {
    let dir = TempDir::new().unwrap();
    let id = "com.core.classic-perimeters";
    let manifest_path = repo_root()
        .join("modules")
        .join("core-modules")
        .join("classic-perimeters")
        .join("classic-perimeters.toml");
    let manifest = fs::read_to_string(manifest_path).expect("read classic-perimeters manifest");
    write_module_with_wasm(
        dir.path(),
        "classic-perimeters",
        &manifest,
        &minimal_component_bytes(),
    );

    let manifest_toml: &'static str = Box::leak(manifest.into_boxed_str());
    let registration = IntegratedModuleRegistration {
        manifest_toml,
        origin_label: "integrated://classic-perimeters",
    };
    let out = load_live_modules_for_plan_with_integrated(
        std::slice::from_ref(&PathBuf::from(dir.path())),
        1,
        &HashMap::new(),
        false,
        std::slice::from_ref(&registration),
        &[(id.to_string(), ClassicPerimeters::__slicer_native_entry())],
    )
    .expect("external override must load");

    let binding = out
        .bindings
        .iter()
        .find(|b| b.module.id() == id)
        .expect("classic-perimeters binding present");
    assert_eq!(binding.module.provenance(), ModuleProvenance::External);
    assert!(binding.native_entry.is_none());
    assert!(binding.wasm_component.is_some());
}
