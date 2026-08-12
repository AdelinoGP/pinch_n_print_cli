//! AC-N2: every newly integrated module remains WASM-dispatched when overridden.

#![allow(missing_docs)]

use std::fs;
use std::path::{Path, PathBuf};

use slicer_integrated_modules::{integrated_registrations, native_entries};
use slicer_scheduler::ModuleProvenance;
use slicer_wasm_host::execution_plan_live::load_live_modules_for_plan_with_integrated;
use tempfile::TempDir;

const MODULES: &[(&str, &str)] = &[
    ("com.core.fuzzy-skin", "fuzzy-skin"),
    ("com.core.gyroid-infill", "gyroid-infill"),
    ("com.core.infill-linker", "infill-linker"),
    ("com.core.layer-planner-default", "layer-planner-default"),
    ("com.core.lightning-infill", "lightning-infill"),
    ("com.core.machine-gcode-emit", "machine-gcode-emit"),
    (
        "com.core.overhang-classifier-default",
        "overhang-classifier-default",
    ),
    (
        "com.core.path-optimization-default",
        "path-optimization-default",
    ),
    ("com.core.part-cooling", "part-cooling"),
    ("com.core.rectilinear-infill", "rectilinear-infill"),
    ("com.core.seam-placer", "seam-placer"),
    ("com.core.seam-planner-default", "seam-planner-default"),
    ("com.core.skirt-brim", "skirt-brim"),
    (
        "com.core.support-surface-ironing",
        "support-surface-ironing",
    ),
    ("com.core.top-surface-ironing", "top-surface-ironing"),
    ("com.core.traditional-support", "traditional-support"),
    ("com.core.tree-support", "tree-support"),
    ("com.core.wipe-tower", "wipe-tower"),
];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root canonicalize")
}

fn minimal_component_bytes() -> Vec<u8> {
    wat::parse_str("(component (core module))").expect("wat parse")
}

fn write_external_module(root: &Path, stem: &str, manifest: &str) {
    fs::write(root.join(format!("{stem}.toml")), manifest).expect("write manifest");
    fs::write(root.join(format!("{stem}.wasm")), minimal_component_bytes()).expect("write wasm");
}

#[test]
fn full_coverage_external_override_forces_wasm() {
    let mut manifests = Vec::with_capacity(MODULES.len());

    for (id, stem) in MODULES {
        let manifest_path = repo_root()
            .join("modules")
            .join("core-modules")
            .join(stem)
            .join(format!("{stem}.toml"));
        let manifest = fs::read_to_string(manifest_path).expect("read module manifest");
        assert!(manifest.contains(&format!("id           = \"{id}\"")));
        manifests.push((*stem, manifest.clone()));
    }

    let registrations = integrated_registrations();
    let entries = native_entries();
    assert_eq!(registrations.len(), MODULES.len());
    assert_eq!(entries.len(), MODULES.len());

    for ((id, stem), (_, manifest)) in MODULES.iter().zip(manifests) {
        let dir = TempDir::new().unwrap();
        write_external_module(dir.path(), stem, &manifest);
        let registration = registrations
            .iter()
            .find(|registration| {
                registration
                    .manifest_toml
                    .contains(&format!("id           = \"{id}\""))
            })
            .unwrap_or_else(|| panic!("integrated registration present: {id}"));
        let out = load_live_modules_for_plan_with_integrated(
            std::slice::from_ref(&PathBuf::from(dir.path())),
            1,
            &std::collections::HashMap::new(),
            false,
            std::slice::from_ref(registration),
            &entries,
        )
        .expect("external override must load");

        let binding = out
            .bindings
            .iter()
            .find(|binding| binding.module.id() == *id)
            .unwrap_or_else(|| panic!("external override binding present: {id}"));
        assert_eq!(binding.module.provenance(), ModuleProvenance::External);
        assert!(
            binding.native_entry.is_none(),
            "{id} must not use native dispatch"
        );
        assert!(
            binding.wasm_component.is_some(),
            "{id} must use WASM dispatch"
        );
    }
}
