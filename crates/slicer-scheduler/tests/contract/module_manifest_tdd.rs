#![allow(missing_docs)]

// ── AC-N1 (packet 150): live manifest validation of percent/float_or_percent
// config-schema defaults ─────────────────────────────────────────────────

/// Builds a minimal valid module manifest whose `[config.schema.<key>]`
/// entry is `type = "percent"` with the given raw TOML `default` literal
/// (caller supplies quoting, e.g. `"\"25%\""` or `"\"abc%\""`).
fn percent_type_manifest_toml(id: &str, key: &str, default_literal: &str) -> String {
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

[config.schema.{key}]
type = "percent"
default = {default_literal}

[hints]
layer-parallel-safe = true
"#
    )
}

fn write_percent_fixture(dir: &std::path::Path, stem: &str, manifest: &str) -> std::path::PathBuf {
    let manifest_path = dir.join(format!("{stem}.toml"));
    std::fs::write(&manifest_path, manifest).expect("write manifest fixture");
    std::fs::write(manifest_path.with_extension("wasm"), b"placeholder wasm")
        .expect("write wasm fixture");
    manifest_path
}

#[test]
fn config_percent_type_rejects_malformed_percent_default_naming_the_key() {
    let temp_dir = tempfile::Builder::new()
        .prefix("config-percent-type-reject-")
        .tempdir()
        .expect("create temp fixture dir");
    let manifest_path = write_percent_fixture(
        temp_dir.path(),
        "bad-percent",
        &percent_type_manifest_toml("com.community.bad-percent", "min_feature_size", "\"abc%\""),
    );

    let error = slicer_scheduler::load_module_from_paths(
        &manifest_path,
        &manifest_path.with_extension("wasm"),
    )
    .expect_err("malformed percent default must be rejected, not silently coerced to 0");

    assert!(
        error.message.contains("min_feature_size"),
        "error should name the offending key: {error:?}"
    );
    assert!(
        error.message.contains("abc%"),
        "error should surface the malformed value: {error:?}"
    );
}

#[test]
fn config_percent_type_accepts_well_formed_percent_default() {
    let temp_dir = tempfile::Builder::new()
        .prefix("config-percent-type-accept-")
        .tempdir()
        .expect("create temp fixture dir");
    let manifest_path = write_percent_fixture(
        temp_dir.path(),
        "good-percent",
        &percent_type_manifest_toml("com.community.good-percent", "min_feature_size", "\"25%\""),
    );

    let module = slicer_scheduler::load_module_from_paths(
        &manifest_path,
        &manifest_path.with_extension("wasm"),
    )
    .expect("well-formed \"25%\" percent default must load");

    let entry = module
        .config_schema()
        .entries
        .get("min_feature_size")
        .expect("min_feature_size entry present");
    assert_eq!(entry.field_type, "percent");

    // Directly exercise the parser `parse_config_field_entry` defers to,
    // proving the default actually parses into `ConfigValue::Percent(25.0)`
    // rather than merely being accepted as an opaque string.
    let parsed = slicer_scheduler::manifest::parse_percent_default(
        "min_feature_size",
        "percent",
        Some(&toml::Value::String("25%".to_string())),
        &manifest_path,
    )
    .expect("well-formed 25% default parses");
    match parsed {
        slicer_ir::ConfigValue::Percent(n) => assert_eq!(n, 25.0),
        other => panic!("expected ConfigValue::Percent(25.0), got {other:?}"),
    }
}

/// Like [`percent_type_manifest_toml`] but with `type = "float_or_percent"`,
/// used to exercise the bare-numeric-string default path.
fn float_or_percent_type_manifest_toml(id: &str, key: &str, default_literal: &str) -> String {
    let mut manifest = percent_type_manifest_toml(id, key, default_literal);
    manifest = manifest.replace("type = \"percent\"", "type = \"float_or_percent\"");
    manifest
}

#[test]
fn float_or_percent_bare_numeric_string_default_is_accepted() {
    let temp_dir = tempfile::Builder::new()
        .prefix("config-fop-bare-")
        .tempdir()
        .expect("create temp fixture dir");
    let manifest_path = write_percent_fixture(
        temp_dir.path(),
        "good-fop",
        &float_or_percent_type_manifest_toml(
            "com.community.good-fop",
            "overhang_reverse_threshold",
            "\"0.0\"",
        ),
    );

    let module = slicer_scheduler::load_module_from_paths(
        &manifest_path,
        &manifest_path.with_extension("wasm"),
    )
    .expect("bare numeric string \"0.0\" float_or_percent default must load (regression D-104h)");

    let entry = module
        .config_schema()
        .entries
        .get("overhang_reverse_threshold")
        .expect("overhang_reverse_threshold entry present");
    assert_eq!(entry.field_type, "float_or_percent");

    let parsed = slicer_scheduler::manifest::parse_percent_default(
        "overhang_reverse_threshold",
        "float_or_percent",
        Some(&toml::Value::String("0.0".to_string())),
        &manifest_path,
    )
    .expect("bare numeric string \"0.0\" default parses");
    match parsed {
        slicer_ir::ConfigValue::FloatOrPercent { value, is_percent } => {
            assert_eq!(value, 0.0);
            assert!(!is_percent, "bare numeric string must be is_percent: false");
        }
        other => panic!("expected ConfigValue::FloatOrPercent, got {other:?}"),
    }
}

#[test]
fn perimeter_modules_declare_arc_tolerance() {
    let path = "../../modules/core-modules/classic-perimeters/classic-perimeters.toml";
    let abs_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path);
    let manifest_text = std::fs::read_to_string(&abs_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {}", abs_path.display(), e));
    let parsed: toml::Value = toml::from_str(&manifest_text)
        .unwrap_or_else(|e| panic!("toml parse error in {}: {}", abs_path.display(), e));
    let schema = &parsed["config"]["schema"]["perimeter_arc_tolerance"];
    assert_eq!(schema["type"].as_str(), Some("float"), "type in {}", path);
    assert_eq!(
        schema["default"].as_float(),
        Some(0.0125),
        "default in {}",
        path
    );
    assert_eq!(schema["min"].as_float(), Some(0.0), "min in {}", path);
    assert_eq!(schema["max"].as_float(), Some(1.0), "max in {}", path);
}

// ── Core region-split manifests must stay dispatch-transparent (ADR-0071) ──

/// The three shipped core manifests that declare `[[region_split]]` must NOT
/// set `paint_only`, and no OTHER core manifest may set it either. Each core
/// module runs on unpainted layers:
/// - `classic-perimeters` / `arachne-perimeters` generate walls for every
///   layer, painted or not;
/// - `fuzzy-skin` gates on `apply_to_all || feature_flags.fuzzy_skin`, so
///   `apply_to_all = true` must still invoke it on an unpainted layer.
///
/// Regressing this pin (setting `paint_only = true`) would skip the module on
/// every layer with no matching `variant_chain` — the exact defect ADR-0071
/// records. Fixture manifests may set it; core shipping manifests may not.
#[test]
fn core_manifests_are_not_paint_only() {
    const DECLARING: &[(&str, &str)] = &[
        ("classic-perimeters", "material"),
        ("arachne-perimeters", "material"),
        ("fuzzy-skin", "fuzzy_skin"),
    ];

    // The three declared expectations must actually declare their semantics.
    for (module, semantic) in DECLARING {
        let relative = format!("../../modules/core-modules/{module}/{module}.toml");
        let abs_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(&relative);
        let manifest_text = std::fs::read_to_string(&abs_path)
            .unwrap_or_else(|e| panic!("cannot read {}: {}", abs_path.display(), e));
        let parsed: toml::Value = toml::from_str(&manifest_text)
            .unwrap_or_else(|e| panic!("toml parse error in {}: {}", abs_path.display(), e));

        let declared: Vec<&str> = parsed["region_split"]
            .as_array()
            .unwrap_or_else(|| panic!("{module} must declare [[region_split]]"))
            .iter()
            .filter_map(|entry| entry.get("semantic").and_then(|s| s.as_str()))
            .collect();
        assert!(
            declared.contains(semantic),
            "{module} must declare semantic {semantic}; got {declared:?}"
        );
    }

    // Sweep every core manifest, not just the three above: a NEW core module
    // that sets paint_only would be skipped on all unpainted layers, and a
    // hard-coded roster would stay green. Core modules are never paint-only.
    let core_dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../modules/core-modules");
    let mut checked = 0usize;
    for entry in std::fs::read_dir(&core_dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", core_dir.display()))
    {
        let module_dir = entry
            .unwrap_or_else(|e| panic!("cannot read core-module dir entry: {e}"))
            .path();
        if !module_dir.is_dir() {
            continue;
        }
        let stem = module_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_else(|| panic!("core module dir is not UTF-8: {module_dir:?}"));
        let manifest_path = module_dir.join(format!("{stem}.toml"));
        let Ok(manifest_text) = std::fs::read_to_string(&manifest_path) else {
            continue; // a dir without a same-stem manifest is not a module
        };
        let parsed: toml::Value = toml::from_str(&manifest_text)
            .unwrap_or_else(|e| panic!("toml parse error in {}: {e}", manifest_path.display()));
        assert_eq!(
            parsed.get("paint_only").and_then(toml::Value::as_bool),
            None,
            "{} must not set paint_only: core modules run on unpainted layers \
             (ADR-0071)",
            manifest_path.display()
        );
        checked += 1;
    }
    assert!(
        checked >= DECLARING.len(),
        "the core-module sweep must cover at least the three declaring manifests, \
         found {checked}"
    );
}
