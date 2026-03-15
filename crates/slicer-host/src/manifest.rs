//! Manifest ingestion contracts for the host scheduler.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use slicer_ir::{ModuleId, SemVer, StageId};
use toml::Value;

/// Runtime module record produced by manifest ingestion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedModule {
    /// Reverse-domain module identifier.
    pub id: ModuleId,
    /// Module semantic version.
    pub version: SemVer,
    /// Canonical scheduler stage identifier.
    pub stage: StageId,
    /// WIT world exported by the module.
    pub wit_world: String,
    /// Declared IR access paths for reads.
    pub ir_reads: Vec<String>,
    /// Declared IR access paths for writes.
    pub ir_writes: Vec<String>,
    /// Claims held by this module.
    pub claims: Vec<String>,
    /// Claims required from other modules.
    pub requires_claims: Vec<String>,
    /// Explicit incompatibility declarations.
    pub incompatible_with: Vec<String>,
    /// Required peer modules.
    pub requires_modules: Vec<ModuleId>,
    /// Minimum host version accepted by the module.
    pub min_host_version: SemVer,
    /// Inclusive minimum IR schema version.
    pub min_ir_schema: SemVer,
    /// Exclusive maximum IR schema version.
    pub max_ir_schema: SemVer,
    /// Placeholder config schema payload.
    pub config_schema: ConfigSchema,
    /// Keys overridable per region.
    pub overridable_per_region: Vec<String>,
    /// Keys overridable per layer.
    pub overridable_per_layer: Vec<String>,
    /// Effective layer parallel safety used by the runtime.
    pub layer_parallel_safe: bool,
    /// Companion `.wasm` path for this manifest.
    pub wasm_path: PathBuf,
}

/// Minimal placeholder for manifest-defined config schema entries.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConfigSchema {
    /// Raw keyed schema entries.
    pub entries: BTreeMap<String, String>,
}

/// Diagnostic severity emitted during module discovery and ingestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    /// Non-fatal informational message.
    Info,
    /// Non-fatal warning.
    Warning,
    /// Fatal error.
    Error,
}

/// Structured ingestion diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadDiagnostic {
    /// Diagnostic severity.
    pub level: DiagnosticLevel,
    /// File path associated with the diagnostic.
    pub path: PathBuf,
    /// Optional manifest field path associated with the issue.
    pub field: Option<String>,
    /// Human-readable diagnostic message.
    pub message: String,
}

/// Structured manifest ingestion error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadError {
    /// File path associated with the error.
    pub path: PathBuf,
    /// Optional manifest field path associated with the error.
    pub field: Option<String>,
    /// Stable machine-readable error kind.
    pub kind: LoadErrorKind,
    /// Human-readable error message.
    pub message: String,
}

/// Stable error classification for manifest ingestion failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadErrorKind {
    /// Placeholder variant kept for red/green TDD compatibility.
    NotImplemented,
    /// The manifest or companion file could not be read.
    Io,
    /// The TOML document is syntactically invalid.
    TomlParse,
    /// The manifest shape or field types are invalid.
    Schema,
    /// The paired same-stem `.wasm` file is missing.
    MissingWasm,
    /// The manifest violates a semantic ingestion rule.
    Validation,
}

/// Result of scanning one or more module roots.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LoadModulesReport {
    /// Successfully loaded modules.
    pub modules: Vec<LoadedModule>,
    /// Structured diagnostics collected during discovery.
    pub diagnostics: Vec<LoadDiagnostic>,
}

/// Loads a single manifest and its paired `.wasm` path.
pub fn load_module_from_paths(
    manifest_path: &Path,
    wasm_path: &Path,
) -> Result<LoadedModule, LoadError> {
    ingest_manifest(manifest_path, wasm_path).map(|result| result.module)
}

/// Scans search roots and loads all discovered modules.
pub fn load_modules_from_roots(search_roots: &[PathBuf]) -> Result<LoadModulesReport, LoadError> {
    let mut report = LoadModulesReport::default();
    let mut seen_ids = HashSet::new();

    for root in search_roots {
        for manifest_path in discover_manifest_paths(root)? {
            let wasm_path = manifest_path.with_extension("wasm");
            let result = ingest_manifest(&manifest_path, &wasm_path)?;

            if seen_ids.contains(&result.module.id) {
                report.diagnostics.push(LoadDiagnostic {
                    level: DiagnosticLevel::Warning,
                    path: manifest_path,
                    field: Some(String::from("module.id")),
                    message: format!(
                        "duplicate module id '{}' ignored because an earlier search root already provided it",
                        result.module.id
                    ),
                });
                report.diagnostics.extend(result.diagnostics);
                continue;
            }

            seen_ids.insert(result.module.id.clone());
            report.diagnostics.extend(result.diagnostics);
            report.modules.push(result.module);
        }
    }

    Ok(report)
}

#[derive(Debug)]
struct IngestedManifest {
    module: LoadedModule,
    diagnostics: Vec<LoadDiagnostic>,
}

fn ingest_manifest(manifest_path: &Path, wasm_path: &Path) -> Result<IngestedManifest, LoadError> {
    ensure_same_stem_wasm_exists(manifest_path, wasm_path)?;

    let manifest_text = fs::read_to_string(manifest_path).map_err(|error| LoadError {
        path: manifest_path.to_path_buf(),
        field: None,
        kind: LoadErrorKind::Io,
        message: format!("failed to read manifest: {error}"),
    })?;

    let root: Value = manifest_text.parse::<Value>().map_err(|error| LoadError {
        path: manifest_path.to_path_buf(),
        field: None,
        kind: LoadErrorKind::TomlParse,
        message: format!("failed to parse TOML manifest: {error}"),
    })?;

    let module_id = required_string(&root, manifest_path, "module.id")?;
    let version = required_semver(&root, manifest_path, "module.version")?;
    let wit_world = required_string(&root, manifest_path, "module.wit-world")?;
    let stage = required_stage(&root, manifest_path, "stage.id")?;
    let mut diagnostics = Vec::new();
    let layer_parallel_safe = effective_parallel_safety(
        manifest_path,
        &stage,
        required_bool(&root, manifest_path, "hints.layer-parallel-safe")?,
        &mut diagnostics,
    );

    let config_schema = read_config_schema(&root, manifest_path)?;

    Ok(IngestedManifest {
        module: LoadedModule {
            id: module_id,
            version,
            stage,
            wit_world,
            ir_reads: required_string_array(&root, manifest_path, "ir-access.reads")?,
            ir_writes: required_string_array(&root, manifest_path, "ir-access.writes")?,
            claims: required_string_array(&root, manifest_path, "claims.holds")?,
            requires_claims: required_string_array(&root, manifest_path, "claims.requires")?,
            incompatible_with: required_string_array(
                &root,
                manifest_path,
                "compatibility.incompatible-with",
            )?,
            requires_modules: required_string_array(
                &root,
                manifest_path,
                "compatibility.requires",
            )?,
            min_host_version: required_semver(
                &root,
                manifest_path,
                "compatibility.min-host-version",
            )?,
            min_ir_schema: required_semver(&root, manifest_path, "compatibility.min-ir-schema")?,
            max_ir_schema: required_semver(&root, manifest_path, "compatibility.max-ir-schema")?,
            config_schema,
            overridable_per_region: required_string_array(
                &root,
                manifest_path,
                "config.overridable-per-region.keys",
            )?,
            overridable_per_layer: required_string_array(
                &root,
                manifest_path,
                "config.overridable-per-layer.keys",
            )?,
            layer_parallel_safe,
            wasm_path: wasm_path.to_path_buf(),
        },
        diagnostics,
    })
}

fn discover_manifest_paths(root: &Path) -> Result<Vec<PathBuf>, LoadError> {
    let mut manifests = Vec::new();
    let directory = fs::read_dir(root).map_err(|error| LoadError {
        path: root.to_path_buf(),
        field: None,
        kind: LoadErrorKind::Io,
        message: format!("failed to read module root: {error}"),
    })?;

    for entry in directory {
        let entry = entry.map_err(|error| LoadError {
            path: root.to_path_buf(),
            field: None,
            kind: LoadErrorKind::Io,
            message: format!("failed to enumerate module root entries: {error}"),
        })?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) == Some("toml") {
            manifests.push(path);
        }
    }

    manifests.sort();
    Ok(manifests)
}

fn ensure_same_stem_wasm_exists(manifest_path: &Path, wasm_path: &Path) -> Result<(), LoadError> {
    if wasm_path.is_file() {
        return Ok(());
    }

    Err(LoadError {
        path: manifest_path.to_path_buf(),
        field: Some(String::from("wasm_path")),
        kind: LoadErrorKind::MissingWasm,
        message: format!(
            "manifest requires a same-stem '.wasm' file beside it, but '{}' was not found",
            wasm_path.display()
        ),
    })
}

fn required_stage(
    root: &Value,
    manifest_path: &Path,
    field: &'static str,
) -> Result<StageId, LoadError> {
    let stage = required_string(root, manifest_path, field)?;
    if known_stage_ids().contains(&stage.as_str()) {
        Ok(stage)
    } else {
        Err(LoadError {
            path: manifest_path.to_path_buf(),
            field: Some(String::from(field)),
            kind: LoadErrorKind::Validation,
            message: format!("unknown stage id '{stage}'"),
        })
    }
}

fn effective_parallel_safety(
    manifest_path: &Path,
    stage: &str,
    declared: bool,
    diagnostics: &mut Vec<LoadDiagnostic>,
) -> bool {
    if stage == "PostPass::LayerFinalization" {
        if declared {
            diagnostics.push(LoadDiagnostic {
                level: DiagnosticLevel::Warning,
                path: manifest_path.to_path_buf(),
                field: Some(String::from("hints.layer-parallel-safe")),
                message: String::from(
                    "PostPass::LayerFinalization modules are always serialized; normalizing layer-parallel-safe to false",
                ),
            });
        }
        false
    } else {
        declared
    }
}

fn read_config_schema(root: &Value, manifest_path: &Path) -> Result<ConfigSchema, LoadError> {
    let Some(schema) = get_value(root, "config.schema") else {
        return Ok(ConfigSchema::default());
    };

    let table = schema.as_table().ok_or_else(|| LoadError {
        path: manifest_path.to_path_buf(),
        field: Some(String::from("config.schema")),
        kind: LoadErrorKind::Schema,
        message: String::from("manifest field 'config.schema' must be a TOML table"),
    })?;

    let mut entries = BTreeMap::new();
    for (key, value) in table {
        entries.insert(key.clone(), value.type_str().to_string());
    }

    Ok(ConfigSchema { entries })
}

fn required_string(
    root: &Value,
    manifest_path: &Path,
    field: &'static str,
) -> Result<String, LoadError> {
    let value = get_value(root, field).ok_or_else(|| missing_field_error(manifest_path, field))?;
    value
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| type_error(manifest_path, field, "string"))
}

fn required_bool(
    root: &Value,
    manifest_path: &Path,
    field: &'static str,
) -> Result<bool, LoadError> {
    let value = get_value(root, field).ok_or_else(|| missing_field_error(manifest_path, field))?;
    value
        .as_bool()
        .ok_or_else(|| type_error(manifest_path, field, "bool"))
}

fn required_string_array(
    root: &Value,
    manifest_path: &Path,
    field: &'static str,
) -> Result<Vec<String>, LoadError> {
    let value = get_value(root, field).ok_or_else(|| missing_field_error(manifest_path, field))?;
    let items = value
        .as_array()
        .ok_or_else(|| type_error(manifest_path, field, "array of strings"))?;

    let mut values = Vec::with_capacity(items.len());
    for item in items {
        let Some(value) = item.as_str() else {
            return Err(type_error(manifest_path, field, "array of strings"));
        };
        values.push(value.to_owned());
    }

    Ok(values)
}

fn required_semver(
    root: &Value,
    manifest_path: &Path,
    field: &'static str,
) -> Result<SemVer, LoadError> {
    let text = required_string(root, manifest_path, field)?;
    parse_semver(&text).ok_or_else(|| LoadError {
        path: manifest_path.to_path_buf(),
        field: Some(String::from(field)),
        kind: LoadErrorKind::Schema,
        message: format!(
            "manifest field '{field}' must be a semver string like '1.2.3'; got '{text}'"
        ),
    })
}

fn parse_semver(text: &str) -> Option<SemVer> {
    let mut parts = text.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }

    Some(SemVer {
        major,
        minor,
        patch,
    })
}

fn get_value<'a>(root: &'a Value, field: &str) -> Option<&'a Value> {
    let mut current = root;
    for segment in field.split('.') {
        current = current.get(segment)?;
    }
    Some(current)
}

fn missing_field_error(manifest_path: &Path, field: &'static str) -> LoadError {
    LoadError {
        path: manifest_path.to_path_buf(),
        field: Some(String::from(field)),
        kind: LoadErrorKind::Schema,
        message: format!("manifest field '{field}' is required"),
    }
}

fn type_error(manifest_path: &Path, field: &'static str, expected: &str) -> LoadError {
    LoadError {
        path: manifest_path.to_path_buf(),
        field: Some(String::from(field)),
        kind: LoadErrorKind::Schema,
        message: format!("manifest field '{field}' must be {expected}"),
    }
}

fn known_stage_ids() -> &'static [&'static str] {
    &[
        "PrePass::MeshSegmentation",
        "PrePass::MeshAnalysis",
        "PrePass::LayerPlanning",
        "PrePass::PaintSegmentation",
        "PrePass::RegionMapping",
        "Layer::Slice",
        "Layer::SlicePostProcess",
        "Layer::Perimeters",
        "Layer::PerimetersPostProcess",
        "Layer::Infill",
        "Layer::InfillPostProcess",
        "Layer::Support",
        "Layer::SupportPostProcess",
        "Layer::PathOptimization",
        "PostPass::LayerFinalization",
        "PostPass::GCodeEmit",
        "PostPass::GCodePostProcess",
        "PostPass::TextPostProcess",
    ]
}

#[cfg(test)]
mod tests {
    use super::{effective_parallel_safety, parse_semver, DiagnosticLevel};
    use std::path::Path;

    #[test]
    fn parse_semver_accepts_three_part_versions() {
        let version = parse_semver("1.2.3").expect("valid semver should parse");
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
    }

    #[test]
    fn finalization_parallel_hint_is_normalized_and_warned() {
        let mut diagnostics = Vec::new();
        let effective = effective_parallel_safety(
            Path::new("fixture.toml"),
            "PostPass::LayerFinalization",
            true,
            &mut diagnostics,
        );

        assert!(!effective);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].level, DiagnosticLevel::Warning);
    }
}
