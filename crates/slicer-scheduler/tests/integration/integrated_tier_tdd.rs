use std::fs;
use std::path::{Path, PathBuf};

use slicer_scheduler::{
    load_modules_from_roots, load_modules_from_roots_with_integrated, DiagnosticLevel,
    IntegratedModuleRegistration, ModuleProvenance,
};
use tempfile::TempDir;

const MODULE_ID: &str = "com.community.integrated-tier-fixture";

fn manifest(id: &str, display_name: &str) -> String {
    format!(
        r#"
[module]
id = "{id}"
version = "1.2.0"
display-name = "{display_name}"
description = "fixture manifest"
author = "community"
license = "MIT"
homepage = "https://example.invalid/{id}"

[stage]
id = "Layer::Infill"

[ir-access]
reads = ["SliceIR.regions.infill_areas", "RegionMapIR"]
writes = ["InfillIR.regions.sparse_infill"]

[claims]
holds = ["infill-generator"]
requires = ["region-map"]

[compatibility]
incompatible-with = ["com.community.lines-*"]
requires = ["com.community.support-prep"]
min-host-version = "0.5.0"
min-ir-schema = "1.2.0"
max-ir-schema = "2.0.0"

[config.schema]

[config.overridable-per-region]
keys = ["density"]

[config.overridable-per-layer]
keys = ["density"]

[hints]
layer-parallel-safe = true
"#
    )
}

const INTEGRATED_MANIFEST: &str = r#"
[module]
id = "com.community.integrated-tier-fixture"
version = "1.2.0"
display-name = "Integrated"
description = "fixture manifest"
author = "community"
license = "MIT"
homepage = "https://example.invalid/com.community.integrated-tier-fixture"

[stage]
id = "Layer::Infill"

[ir-access]
reads = ["SliceIR.regions.infill_areas", "RegionMapIR"]
writes = ["InfillIR.regions.sparse_infill"]

[claims]
holds = ["infill-generator"]
requires = ["region-map"]

[compatibility]
incompatible-with = ["com.community.lines-*"]
requires = ["com.community.support-prep"]
min-host-version = "0.5.0"
min-ir-schema = "1.2.0"
max-ir-schema = "2.0.0"

[config.schema]

[config.overridable-per-region]
keys = ["density"]

[config.overridable-per-layer]
keys = ["density"]

[hints]
layer-parallel-safe = true
"#;

struct ModuleFixture {
    temp_dir: TempDir,
}

impl ModuleFixture {
    fn new(label: &str) -> Self {
        Self {
            temp_dir: tempfile::Builder::new()
                .prefix(&format!("integrated-tier-{label}-"))
                .tempdir()
                .expect("create temp fixture dir"),
        }
    }

    fn root(&self) -> &Path {
        self.temp_dir.path()
    }

    fn write_module(&self, stem: &str, text: &str) {
        fs::write(self.root().join(format!("{stem}.toml")), text).expect("write manifest fixture");
        fs::write(
            self.root().join(format!("{stem}.wasm")),
            b"placeholder wasm",
        )
        .expect("write wasm fixture");
    }
}

fn registration(display_name: &'static str) -> IntegratedModuleRegistration {
    let _ = display_name;
    IntegratedModuleRegistration {
        manifest_toml: INTEGRATED_MANIFEST,
        origin_label: "core-integrated",
    }
}

#[test]
fn integrated_manifest_ingests_without_wasm() {
    let report = load_modules_from_roots_with_integrated(&[], &[registration("Integrated")])
        .expect("integrated manifest should load without a wasm file");

    assert_eq!(report.modules.len(), 1);
    let module = &report.modules[0];
    assert_eq!(module.id(), MODULE_ID);
    assert_eq!(module.provenance(), ModuleProvenance::Integrated);
    assert!(!module.placeholder_wasm());
}

#[test]
fn external_root_overrides_integrated_tier() {
    let fixture = ModuleFixture::new("override");
    fixture.write_module("external", &manifest(MODULE_ID, "External"));
    let roots = vec![fixture.root().to_path_buf()];
    let report = load_modules_from_roots_with_integrated(&roots, &[registration("Integrated")])
        .expect("external and integrated modules should load");
    let plain = load_modules_from_roots(&roots).expect("plain external scan should load");

    assert_eq!(report.modules.len(), 1);
    assert_eq!(report.modules[0], plain.modules[0]);
    assert_eq!(report.modules[0].provenance(), ModuleProvenance::External);
}

#[test]
fn external_shadow_diagnostic_names_integrated_loser() {
    let fixture = ModuleFixture::new("diagnostic");
    fixture.write_module("external", &manifest(MODULE_ID, "External"));
    let report = load_modules_from_roots_with_integrated(
        &[fixture.root().to_path_buf()],
        &[registration("Integrated")],
    )
    .expect("external and integrated modules should load");

    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.level == DiagnosticLevel::Warning
            && diagnostic.field.as_deref() == Some("module.id")
            && diagnostic.message
                == format!("external module {MODULE_ID} shadows integrated module {MODULE_ID}")
    }));
}

#[test]
fn empty_integrated_registry_is_identity() {
    let roots = vec![PathBuf::from("modules/core-modules")];
    let with_empty = load_modules_from_roots_with_integrated(&roots, &[])
        .expect("empty integrated registry should be accepted");
    let plain = load_modules_from_roots(&roots).expect("plain scan should load");

    assert_eq!(with_empty, plain);
}
