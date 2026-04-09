#![allow(missing_docs)]

use std::fs;
use std::path::{Path, PathBuf};

use slicer_host::{
    load_module_from_paths, load_modules_from_roots, DiagnosticLevel, LoadErrorKind, LoadedModule,
};
use slicer_ir::SemVer;
use tempfile::TempDir;

#[test]
fn valid_manifest_is_normalized_into_loaded_module_runtime_fields() {
    let fixture = ModuleFixture::new("valid-manifest");
    let manifest_path = fixture.write_module(
        "tpms",
        valid_manifest_toml(
            "com.community.tpms-infill",
            "Layer::Infill",
            "slicer:world-layer@1.0.0",
            true,
        ),
        true,
    );
    let wasm_path = manifest_path.with_extension("wasm");

    let module = load_module_from_paths(&manifest_path, &wasm_path)
        .expect("valid manifest + paired wasm should load into a runtime record");

    assert_loaded_module_basics(&module, &wasm_path);
    assert_eq!(
        module.ir_reads,
        vec![
            String::from("SliceIR.regions.infill_areas"),
            String::from("RegionMapIR"),
        ]
    );
    assert_eq!(
        module.ir_writes,
        vec![String::from("InfillIR.regions.sparse_infill")]
    );
    assert_eq!(module.claims, vec![String::from("infill-generator")]);
    assert_eq!(module.requires_claims, vec![String::from("region-map")]);
    assert_eq!(
        module.incompatible_with,
        vec![String::from("com.community.lines-*")]
    );
    assert_eq!(
        module.requires_modules,
        vec![String::from("com.community.support-prep")]
    );
    assert_eq!(module.overridable_per_region, vec![String::from("density")]);
    assert_eq!(module.overridable_per_layer, vec![String::from("density")]);
    assert_eq!(module.min_host_version, semver(0, 5, 0));
    assert_eq!(module.min_ir_schema, semver(1, 2, 0));
    assert_eq!(module.max_ir_schema, semver(2, 0, 0));
    assert!(module.layer_parallel_safe);
}

#[test]
fn unknown_stage_is_a_fatal_structured_error_with_path_and_field_context() {
    let fixture = ModuleFixture::new("unknown-stage");
    let manifest_path = fixture.write_module(
        "bad-stage",
        valid_manifest_toml(
            "com.community.bad-stage",
            "Layer::TypoStage",
            "slicer:world-layer@1.0.0",
            true,
        ),
        true,
    );

    let error = load_module_from_paths(&manifest_path, &manifest_path.with_extension("wasm"))
        .expect_err("unknown stage should be rejected during ingestion");

    assert_eq!(error.path, manifest_path);
    assert_eq!(error.field.as_deref(), Some("stage.id"));
    assert_ne!(error.kind, LoadErrorKind::NotImplemented);
    assert!(
        error.message.contains("Layer::TypoStage"),
        "error should name the unknown stage: {error:?}"
    );
}

#[test]
fn manifest_is_not_loadable_without_same_stem_wasm_beside_it() {
    let fixture = ModuleFixture::new("missing-wasm");
    let manifest_path = fixture.write_module(
        "missing-wasm",
        valid_manifest_toml(
            "com.community.missing-wasm",
            "Layer::Infill",
            "slicer:world-layer@1.0.0",
            true,
        ),
        false,
    );

    let error = load_module_from_paths(&manifest_path, &manifest_path.with_extension("wasm"))
        .expect_err("manifest without same-stem wasm should be rejected");

    assert_eq!(error.path, manifest_path);
    assert_eq!(error.field.as_deref(), Some("wasm_path"));
    assert!(
        error.message.contains("same-stem") || error.message.contains(".wasm"),
        "error should explain the missing paired wasm requirement: {error:?}"
    );
}

#[test]
fn higher_precedence_root_wins_duplicate_module_ids_and_emits_warning() {
    let fixture = ModuleFixture::new("duplicate-roots");
    let high = fixture.root().join("01-cli-root");
    let low = fixture.root().join("02-config-root");
    fs::create_dir_all(&high).expect("create high precedence root");
    fs::create_dir_all(&low).expect("create low precedence root");

    write_module_in(
        &high,
        "shared-module",
        &valid_manifest_toml(
            "com.community.duplicate",
            "Layer::Infill",
            "slicer:world-layer@1.0.0",
            true,
        ),
        true,
    );
    write_module_in(
        &low,
        "shared-module",
        &valid_manifest_toml(
            "com.community.duplicate",
            "Layer::Support",
            "slicer:world-layer@1.0.0",
            true,
        ),
        true,
    );

    let report = load_modules_from_roots(&[high.clone(), low.clone()])
        .expect("duplicate ids across roots should warn, not abort the full scan");

    assert_eq!(
        report.modules.len(),
        1,
        "earlier root should win duplicate resolution"
    );
    assert_eq!(report.modules[0].id, "com.community.duplicate");
    assert_eq!(report.modules[0].stage, "Layer::Infill");

    let warning = report
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.level == DiagnosticLevel::Warning)
        .expect("later duplicate should emit a warning diagnostic");
    assert_eq!(warning.field.as_deref(), Some("module.id"));
    assert_eq!(warning.path, low.join("shared-module.toml"));
    assert!(
        warning.message.contains("com.community.duplicate"),
        "warning should name the duplicate module id: {warning:?}"
    );
}

#[test]
fn finalization_manifest_true_parallel_hint_warns_and_normalizes_to_serialized_runtime_mode() {
    let fixture = ModuleFixture::new("finalization-hint");
    let manifest_path = fixture.write_module(
        "finalize",
        valid_manifest_toml(
            "com.community.finalizer",
            "PostPass::LayerFinalization",
            "slicer:world-finalization@1.0.0",
            true,
        ),
        true,
    );

    let report = load_modules_from_roots(&[fixture.root().to_path_buf()])
        .expect("finalization module should still load with a warning");

    assert_eq!(report.modules.len(), 1);
    assert_eq!(report.modules[0].id, "com.community.finalizer");
    assert!(!report.modules[0].layer_parallel_safe);

    let warning = report
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.path == manifest_path)
        .expect("finalization parallel hint should emit a deterministic warning");
    assert_eq!(warning.level, DiagnosticLevel::Warning);
    assert_eq!(warning.field.as_deref(), Some("hints.layer-parallel-safe"));
    assert!(warning.message.contains("PostPass::LayerFinalization"));
}

#[test]
fn malformed_schema_error_surfaces_manifest_path_and_field_name() {
    let fixture = ModuleFixture::new("schema-error");
    let manifest_path = fixture.write_module(
        "schema-error",
        r#"
[module]
id = "com.community.schema-error"
version = "1.2"
display-name = "Schema Error"
description = "invalid semver for version"
author = "community"
license = "MIT"
homepage = "https://example.invalid/schema-error"
wit-world = "slicer:world-layer@1.0.0"

[stage]
id = "Layer::Infill"

[ir-access]
reads = []
writes = []

[claims]
holds = []
requires = []

[compatibility]
incompatible-with = []
requires = []
min-host-version = "0.5.0"
min-ir-schema = "1.2.0"
max-ir-schema = "2.0.0"

[config.schema]

[config.overridable-per-region]
keys = []

[config.overridable-per-layer]
keys = []

[hints]
layer-parallel-safe = true
"#
        .to_string(),
        true,
    );

    let error = load_module_from_paths(&manifest_path, &manifest_path.with_extension("wasm"))
        .expect_err("schema errors should surface as structured load failures");

    assert_eq!(error.path, manifest_path);
    assert_eq!(error.field.as_deref(), Some("module.version"));
    assert!(
        error.message.contains("version"),
        "schema error should mention the offending field: {error:?}"
    );
}

fn assert_loaded_module_basics(module: &LoadedModule, wasm_path: &Path) {
    assert_eq!(module.id, "com.community.tpms-infill");
    assert_eq!(module.version, semver(1, 2, 0));
    assert_eq!(module.stage, "Layer::Infill");
    assert_eq!(module.wit_world, "slicer:world-layer@1.0.0");
    assert_eq!(module.wasm_path, wasm_path);
}

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn write_module_in(root: &Path, stem: &str, manifest: &str, with_wasm: bool) -> PathBuf {
    let manifest_path = root.join(format!("{stem}.toml"));
    fs::write(&manifest_path, manifest).expect("write manifest fixture");

    if with_wasm {
        fs::write(root.join(format!("{stem}.wasm")), b"placeholder wasm")
            .expect("write wasm fixture");
    }

    manifest_path
}

fn valid_manifest_toml(
    id: &str,
    stage: &str,
    wit_world: &str,
    layer_parallel_safe: bool,
) -> String {
    format!(
        r#"
[module]
id = "{id}"
version = "1.2.0"
display-name = "Fixture Module"
description = "fixture manifest"
author = "community"
license = "MIT"
homepage = "https://example.invalid/{id}"
wit-world = "{wit_world}"

[stage]
id = "{stage}"

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
layer-parallel-safe = {layer_parallel_safe}
"#
    )
}

struct ModuleFixture {
    temp_dir: TempDir,
}

impl ModuleFixture {
    fn new(label: &str) -> Self {
        let temp_dir = tempfile::Builder::new()
            .prefix(&format!("manifest-ingestion-{label}-"))
            .tempdir()
            .expect("create temp fixture dir");
        Self { temp_dir }
    }

    fn root(&self) -> &Path {
        self.temp_dir.path()
    }

    fn write_module(&self, stem: &str, manifest: String, with_wasm: bool) -> PathBuf {
        write_module_in(self.root(), stem, &manifest, with_wasm)
    }
}
