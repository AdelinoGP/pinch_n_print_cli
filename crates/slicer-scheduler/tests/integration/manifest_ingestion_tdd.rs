#![allow(missing_docs)]

use std::fs;
use std::path::{Path, PathBuf};

use slicer_ir::SemVer;
use slicer_scheduler::{
    load_module_from_paths, load_modules_from_roots, DiagnosticLevel, LoadErrorKind, LoadedModule,
};
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
        module.ir_reads(),
        &[
            String::from("SliceIR.regions.infill_areas"),
            String::from("RegionMapIR"),
        ]
    );
    assert_eq!(
        module.ir_writes(),
        &[String::from("InfillIR.regions.sparse_infill")]
    );
    assert_eq!(module.claims(), &[String::from("infill-generator")]);
    assert_eq!(module.requires_claims(), &[String::from("region-map")]);
    assert_eq!(
        module.incompatible_with(),
        &[String::from("com.community.lines-*")]
    );
    assert_eq!(
        module.requires_modules(),
        &[String::from("com.community.support-prep")]
    );
    assert_eq!(module.overridable_per_region(), &[String::from("density")]);
    assert_eq!(module.overridable_per_layer(), &[String::from("density")]);
    assert_eq!(module.min_host_version(), semver(0, 5, 0));
    assert_eq!(module.min_ir_schema(), semver(1, 2, 0));
    assert_eq!(module.max_ir_schema(), semver(2, 0, 0));
    assert!(module.layer_parallel_safe());
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
    assert_eq!(report.modules[0].id(), "com.community.duplicate");
    assert_eq!(report.modules[0].stage(), "Layer::Infill");

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

// â”€â”€ WIT world allowlist validation â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn wit_world_mismatch_rejects_invalid_package_name() {
    // The old (pre-consolidation) package name must be rejected.
    let fixture = ModuleFixture::new("wit-world-bad-pkg");
    let manifest_path = fixture.write_module(
        "bad-pkg-module",
        valid_manifest_toml(
            "com.community.bad-pkg",
            "Layer::Infill",
            "slicer:layer-world@1.0.0", // wrong â€” canonical is slicer:world-layer@1.0.0
            true,
        ),
        true,
    );

    let error = load_module_from_paths(&manifest_path, &manifest_path.with_extension("wasm"))
        .expect_err("non-allowlisted wit_world should be rejected during ingestion");

    assert_eq!(error.path, manifest_path);
    assert_eq!(error.field.as_deref(), Some("module.wit-world"));
    assert_eq!(error.kind, LoadErrorKind::Validation);
    assert!(
        error.message.contains("slicer:layer-world@1.0.0"),
        "error should name the invalid wit_world value: {error:?}"
    );
    assert!(
        error.message.contains("slicer:world-layer@1.0.0"),
        "error should list the canonical wit_world values: {error:?}"
    );
}

#[test]
fn wit_world_major_version_mismatch_rejects_future_major() {
    // A future major version of a canonical world must also be rejected.
    let fixture = ModuleFixture::new("wit-world-future-major");
    let manifest_path = fixture.write_module(
        "future-major-module",
        valid_manifest_toml(
            "com.community.future-major",
            "Layer::Infill",
            "slicer:world-layer@2.0.0", // future major â€” not in allowlist
            true,
        ),
        true,
    );

    let error = load_module_from_paths(&manifest_path, &manifest_path.with_extension("wasm"))
        .expect_err("future major version should be rejected during ingestion");

    assert_eq!(error.path, manifest_path);
    assert_eq!(error.field.as_deref(), Some("module.wit-world"));
    assert_eq!(error.kind, LoadErrorKind::Validation);
    assert!(
        error.message.contains("slicer:world-layer@2.0.0"),
        "error should name the invalid wit_world value: {error:?}"
    );
}

#[test]
fn lexical_order_within_one_root_deterministically_breaks_duplicate_ids() {
    let fixture = ModuleFixture::new("duplicate-same-root");

    write_module_in(
        fixture.root(),
        "00-first",
        &valid_manifest_toml(
            "com.community.same-root-duplicate",
            "Layer::Infill",
            "slicer:world-layer@1.0.0",
            true,
        ),
        true,
    );
    let losing_manifest = write_module_in(
        fixture.root(),
        "99-second",
        &valid_manifest_toml(
            "com.community.same-root-duplicate",
            "Layer::Support",
            "slicer:world-layer@1.0.0",
            true,
        ),
        true,
    );

    let report = load_modules_from_roots(&[fixture.root().to_path_buf()])
        .expect("duplicate ids in one root should resolve deterministically");

    assert_eq!(report.modules.len(), 1);
    assert_eq!(report.modules[0].id(), "com.community.same-root-duplicate");
    assert_eq!(report.modules[0].stage(), "Layer::Infill");

    let warning = report
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.level == DiagnosticLevel::Warning)
        .expect("later duplicate in lexical order should emit a warning");
    assert_eq!(warning.path, losing_manifest);
    assert_eq!(warning.field.as_deref(), Some("module.id"));
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
    assert_eq!(report.modules[0].id(), "com.community.finalizer");
    assert!(!report.modules[0].layer_parallel_safe());

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
    assert_eq!(module.id(), "com.community.tpms-infill");
    assert_eq!(module.version(), semver(1, 2, 0));
    assert_eq!(module.stage(), "Layer::Infill");
    assert_eq!(module.wit_world(), "slicer:world-layer@1.0.0");
    assert_eq!(module.wasm_path(), wasm_path);
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

// â”€â”€ Subdirectory discovery â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn write_module_in_subdir(
    root: &Path,
    dir_name: &str,
    stem: &str,
    manifest: &str,
    with_wasm: bool,
) -> PathBuf {
    let subdir = root.join(dir_name);
    fs::create_dir_all(&subdir).expect("create module subdirectory");
    write_module_in(&subdir, stem, manifest, with_wasm)
}

#[test]
fn discovery_finds_manifests_in_immediate_subdirectories() {
    let fixture = ModuleFixture::new("subdir-discovery");

    // Module A in a subdirectory
    write_module_in_subdir(
        fixture.root(),
        "my-infill",
        "my-infill",
        &valid_manifest_toml(
            "com.core.my-infill",
            "Layer::Infill",
            "slicer:world-layer@1.0.0",
            true,
        ),
        true,
    );

    // Module B in a different subdirectory
    write_module_in_subdir(
        fixture.root(),
        "my-support",
        "my-support",
        &valid_manifest_toml(
            "com.core.my-support",
            "Layer::Support",
            "slicer:world-layer@1.0.0",
            true,
        ),
        true,
    );

    let report = load_modules_from_roots(&[fixture.root().to_path_buf()])
        .expect("subdirectory modules should be discoverable");

    assert_eq!(
        report.modules.len(),
        2,
        "should discover both modules in subdirectories"
    );

    let ids: Vec<&str> = report.modules.iter().map(|m| m.id()).collect();
    assert!(ids.contains(&"com.core.my-infill"));
    assert!(ids.contains(&"com.core.my-support"));
}

#[test]
fn discovery_excludes_cargo_toml_in_subdirectories() {
    let fixture = ModuleFixture::new("cargo-exclusion");

    let subdir = fixture.root().join("my-module");
    fs::create_dir_all(&subdir).expect("create module subdir");

    // Write Cargo.toml (should be ignored)
    fs::write(subdir.join("Cargo.toml"), "[package]\nname = \"my-module\"")
        .expect("write Cargo.toml");

    // Write the actual module manifest
    write_module_in(
        &subdir,
        "my-module",
        &valid_manifest_toml(
            "com.core.my-module",
            "Layer::Infill",
            "slicer:world-layer@1.0.0",
            true,
        ),
        true,
    );

    let report = load_modules_from_roots(&[fixture.root().to_path_buf()])
        .expect("discovery should load module but skip Cargo.toml");

    assert_eq!(report.modules.len(), 1);
    assert_eq!(report.modules[0].id(), "com.core.my-module");
}

#[test]
fn discovery_mixes_flat_and_subdirectory_manifests() {
    let fixture = ModuleFixture::new("mixed-layout");

    // Flat manifest in root
    write_module_in(
        fixture.root(),
        "flat-module",
        &valid_manifest_toml(
            "com.community.flat",
            "Layer::Infill",
            "slicer:world-layer@1.0.0",
            true,
        ),
        true,
    );

    // Subdirectory manifest
    write_module_in_subdir(
        fixture.root(),
        "subdir-module",
        "subdir-module",
        &valid_manifest_toml(
            "com.core.subdir",
            "Layer::Perimeters",
            "slicer:world-layer@1.0.0",
            true,
        ),
        true,
    );

    let report = load_modules_from_roots(&[fixture.root().to_path_buf()])
        .expect("should discover both flat and subdirectory modules");

    assert_eq!(report.modules.len(), 2);
    let ids: Vec<&str> = report.modules.iter().map(|m| m.id()).collect();
    assert!(ids.contains(&"com.community.flat"));
    assert!(ids.contains(&"com.core.subdir"));
}

#[test]
fn core_modules_directory_is_discoverable_and_all_load() {
    let core_modules_root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../modules/core-modules");

    if !core_modules_root.is_dir() {
        // Skip if running from a different context
        return;
    }

    let report = load_modules_from_roots(&[core_modules_root])
        .expect("all core module manifests should load without errors");

    // We expect exactly 20 core modules as of 2026-06-13 (packet 97 deleted the dead
    // mesh-segmentation WASM-guest module; was 21 with overhang-classifier-default).
    assert_eq!(
        report.modules.len(),
        20,
        "expected 20 core modules, got {}: {:?}",
        report.modules.len(),
        report.modules.iter().map(|m| m.id()).collect::<Vec<_>>()
    );

    // Verify no errors in diagnostics (warnings are ok)
    let errors: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.level == DiagnosticLevel::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "core module discovery should produce no errors: {errors:?}"
    );

    // Verify all modules have valid stages
    for module in &report.modules {
        assert!(
            !module.stage().is_empty(),
            "module {} must have a stage",
            module.id()
        );
        assert!(
            !module.wit_world().is_empty(),
            "module {} must have a wit_world",
            module.id()
        );
    }

    // Verify we have modules covering key stages
    let stages: Vec<&str> = report.modules.iter().map(|m| m.stage()).collect();
    assert!(
        stages.contains(&"Layer::Infill"),
        "should have infill modules"
    );
    assert!(
        stages.contains(&"Layer::Perimeters"),
        "should have perimeter modules"
    );
    assert!(
        stages.contains(&"Layer::Support"),
        "should have support modules"
    );
    assert!(
        stages.contains(&"PrePass::LayerPlanning"),
        "should have layer planner"
    );
    assert!(
        stages.contains(&"PostPass::LayerFinalization"),
        "should have finalization modules"
    );
}

#[test]
fn core_module_ids_are_unique() {
    let core_modules_root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../modules/core-modules");

    if !core_modules_root.is_dir() {
        return;
    }

    let report = load_modules_from_roots(&[core_modules_root]).expect("core modules should load");

    // No duplicate warnings should appear (all IDs are unique)
    let dup_warnings: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.level == DiagnosticLevel::Warning && d.message.contains("duplicate"))
        .collect();
    assert!(
        dup_warnings.is_empty(),
        "core modules should have unique IDs, but found duplicate warnings: {dup_warnings:?}"
    );
}

#[test]
fn core_finalization_modules_have_parallel_safe_false() {
    let core_modules_root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../modules/core-modules");

    if !core_modules_root.is_dir() {
        return;
    }

    let report = load_modules_from_roots(&[core_modules_root]).expect("core modules should load");

    for module in &report.modules {
        if module.stage() == "PostPass::LayerFinalization" {
            assert!(
                !module.layer_parallel_safe(),
                "finalization module {} must have layer_parallel_safe=false",
                module.id()
            );
        }
    }
}

// â”€â”€ Placeholder .wasm detection â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn placeholder_wasm_is_detected_during_ingestion() {
    let fixture = ModuleFixture::new("placeholder-detect");
    let manifest_path = fixture.write_module(
        "placeholder-mod",
        valid_manifest_toml(
            "com.test.placeholder",
            "Layer::Infill",
            "slicer:world-layer@1.0.0",
            true,
        ),
        true,
    );

    // Overwrite with the 8-byte WASM magic (the real placeholder pattern)
    fs::write(
        manifest_path.with_extension("wasm"),
        b"\x00asm\x01\x00\x00\x00",
    )
    .unwrap();

    let report = load_modules_from_roots(&[fixture.root().to_path_buf()])
        .expect("placeholder wasm should be discoverable");

    assert_eq!(report.modules.len(), 1);
    assert!(
        report.modules[0].placeholder_wasm(),
        "module with 8-byte stub should have placeholder_wasm=true"
    );

    let warning = report
        .diagnostics
        .iter()
        .find(|d| d.level == DiagnosticLevel::Warning && d.message.contains("placeholder"))
        .expect("placeholder wasm should emit a warning diagnostic");
    assert!(
        warning.message.contains("com.test.placeholder"),
        "warning should name the module: {}",
        warning.message
    );
    assert!(
        warning.message.contains("8 bytes"),
        "warning should state the file size: {}",
        warning.message
    );
}

#[test]
fn real_wasm_is_not_flagged_as_placeholder() {
    let fixture = ModuleFixture::new("real-wasm");
    let manifest_path = fixture.write_module(
        "real-mod",
        valid_manifest_toml(
            "com.test.real",
            "Layer::Infill",
            "slicer:world-layer@1.0.0",
            true,
        ),
        true,
    );

    // Overwrite with something larger than 8 bytes
    fs::write(
        manifest_path.with_extension("wasm"),
        b"\x00asm\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",
    )
    .unwrap();

    let report = load_modules_from_roots(&[fixture.root().to_path_buf()]).unwrap();
    assert_eq!(report.modules.len(), 1);
    assert!(
        !report.modules[0].placeholder_wasm(),
        "module with >8 byte wasm should not be a placeholder"
    );
}

#[test]
fn core_modules_all_have_placeholder_wasm_flag_set() {
    let core_modules_root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../modules/core-modules");

    if !core_modules_root.is_dir() {
        return;
    }

    let report = load_modules_from_roots(&[core_modules_root]).unwrap();

    // Modules with a real component-model .wasm produced by
    // `cargo xtask build-guests`. They must not be
    // flagged as placeholders; every other core module still is.
    // Modules with a real component-model .wasm produced by
    // `cargo xtask build-guests`. They must not be
    // flagged as placeholders; every other core module still is.
    // `paint-segmentation` and `paint-region-annotator` .wasm still
    // exist on disk (deletion is Step 7), so they remain non-placeholder
    // until that step removes the build artifacts.
    const NON_PLACEHOLDER: &[&str] = &[
        "com.core.layer-planner-default",
        "com.core.paint-segmentation",
        "com.core.path-optimization-default",
        "com.core.classic-perimeters",
        "com.core.rectilinear-infill",
        "com.core.gyroid-infill",
        "com.core.lightning-infill",
        "com.core.traditional-support",
        "com.core.tree-support",
        "com.core.paint-region-annotator",
        "com.core.part-cooling",
        "com.core.seam-placer",
        "com.core.seam-planner-default",
        "com.core.support-planner",
        "com.core.fuzzy-skin",
        "com.core.machine-gcode-emit",
        "com.core.support-surface-ironing",
        "com.core.skirt-brim",
        "com.core.top-surface-ironing",
        "com.core.wipe-tower",
        "com.core.overhang-classifier-default",
    ];

    for module in &report.modules {
        let expected_placeholder = !NON_PLACEHOLDER.contains(&module.id());
        assert_eq!(
            module.placeholder_wasm(),
            expected_placeholder,
            "core module {} placeholder_wasm mismatch (expected {}, got {})",
            module.id(),
            expected_placeholder,
            module.placeholder_wasm()
        );
    }
}

#[test]
fn core_module_placeholder_warnings_include_module_ids() {
    let core_modules_root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../modules/core-modules");

    if !core_modules_root.is_dir() {
        return;
    }

    let report = load_modules_from_roots(&[core_modules_root]).unwrap();

    let placeholder_warnings: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.level == DiagnosticLevel::Warning && d.message.contains("placeholder"))
        .collect();

    // One placeholder warning per core module whose companion .wasm is
    // still an 8-byte stub. Modules built by
    // `cargo xtask build-guests` do not produce a
    // placeholder warning. The count is derived dynamically.
    let total_modules = report.modules.len();
    let real_count = report
        .modules
        .iter()
        .filter(|m| !m.placeholder_wasm())
        .count();
    let expected_placeholder_warnings = total_modules - real_count;
    assert_eq!(
        placeholder_warnings.len(),
        expected_placeholder_warnings,
        "each placeholder core module should emit one placeholder warning \
         (total_modules={total_modules}, real={real_count}, got {})",
        placeholder_warnings.len()
    );
}

// â”€â”€ Deterministic discovery order â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn discovery_order_is_deterministic_across_repeated_scans() {
    let core_modules_root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../modules/core-modules");

    if !core_modules_root.is_dir() {
        return;
    }

    let ids_a: Vec<String> = load_modules_from_roots(&[core_modules_root.clone()])
        .unwrap()
        .modules
        .iter()
        .map(|m| m.id().to_string())
        .collect();

    let ids_b: Vec<String> = load_modules_from_roots(&[core_modules_root])
        .unwrap()
        .modules
        .iter()
        .map(|m| m.id().to_string())
        .collect();

    assert_eq!(ids_a, ids_b, "module discovery order must be deterministic");
}

#[test]
fn discovery_order_is_lexicographic_by_manifest_path() {
    let fixture = ModuleFixture::new("lex-order");

    write_module_in_subdir(
        fixture.root(),
        "zzz-module",
        "zzz-module",
        &valid_manifest_toml(
            "com.test.zzz",
            "Layer::Support",
            "slicer:world-layer@1.0.0",
            true,
        ),
        true,
    );
    write_module_in_subdir(
        fixture.root(),
        "aaa-module",
        "aaa-module",
        &valid_manifest_toml(
            "com.test.aaa",
            "Layer::Infill",
            "slicer:world-layer@1.0.0",
            true,
        ),
        true,
    );
    write_module_in_subdir(
        fixture.root(),
        "mmm-module",
        "mmm-module",
        &valid_manifest_toml(
            "com.test.mmm",
            "Layer::Perimeters",
            "slicer:world-layer@1.0.0",
            true,
        ),
        true,
    );

    let report = load_modules_from_roots(&[fixture.root().to_path_buf()]).unwrap();
    let ids: Vec<&str> = report.modules.iter().map(|m| m.id()).collect();

    assert_eq!(ids, vec!["com.test.aaa", "com.test.mmm", "com.test.zzz"]);
}
