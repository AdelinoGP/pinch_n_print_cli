//! AC-N2: every newly integrated module remains WASM-dispatched when overridden.

#![allow(missing_docs)]

use std::fs;
use std::path::{Path, PathBuf};

use slicer_integrated_modules::{integrated_inventory, integrated_registrations, native_entries};
use slicer_scheduler::ModuleProvenance;
use slicer_wasm_host::execution_plan_live::load_live_modules_for_plan_with_integrated;
use tempfile::TempDir;

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
    let inventory = integrated_inventory();
    let mut manifests = Vec::with_capacity(inventory.len());

    for module in &inventory {
        let id = module.id;
        let stem = id.strip_prefix("com.core.").unwrap();
        let manifest_path = repo_root()
            .join("modules")
            .join("core-modules")
            .join(stem)
            .join(format!("{stem}.toml"));
        let manifest = fs::read_to_string(manifest_path).expect("read module manifest");
        assert!(manifest.contains(&format!("id           = \"{id}\"")));
        manifests.push((stem, manifest.clone()));
    }

    let registrations = integrated_registrations();
    let entries = native_entries();
    assert_eq!(registrations.len(), inventory.len());
    assert_eq!(entries.len(), inventory.len());

    for (module, (stem, manifest)) in inventory.iter().zip(manifests.iter()) {
        let id = module.id;
        let dir = TempDir::new().unwrap();
        write_external_module(dir.path(), stem, manifest);
        if manifest.contains("support-family:tree") {
            let paired = [
                ("tree-support", "tree-support/tree-support.toml"),
                (
                    "tree-support-planner",
                    "tree-support-planner/tree-support-planner.toml",
                ),
            ];
            for (paired_stem, relative_path) in paired {
                if paired_stem != *stem {
                    let paired_manifest = fs::read_to_string(
                        repo_root()
                            .join("modules")
                            .join("core-modules")
                            .join(relative_path),
                    )
                    .expect("read tree family pair");
                    write_external_module(dir.path(), paired_stem, &paired_manifest);
                }
            }
        }
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
            .find(|binding| binding.module.id() == id)
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
