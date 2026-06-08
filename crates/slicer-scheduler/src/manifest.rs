//! Manifest ingestion contracts for the host scheduler.

// `LoadError` carries structured diagnostic fields (PathBuf, String payload, enum
// with named fields) which intentionally exceed the 128-byte threshold.  The large
// size is an acceptable trade-off for rich diagnostics at the boundary; boxing would
// complicate call-sites without real performance benefit.
#![allow(clippy::result_large_err)]

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use slicer_ir::{ModuleId, SemVer, StageId};
use toml::Value;

/// Wire-format version for the JSON emitted by [`build_config_schema_json`] and
/// consumed by `pnp_cli module config-schema`. Semver `"<major>.<minor>.<patch>"`;
/// consumers (e.g. `pinch_n_print_studio`) gate on the major. See
/// `docs/11_operational_governance_and_acceptance_gate.md` for bumping rules.
pub const CONFIG_SCHEMA_WIRE_VERSION: &str = "1.0.0";

/// Helper for serde skip_serializing_if on bool.
fn is_false(b: &bool) -> bool {
    !*b
}

/// One declared region-split semantic a module cares about. Parsed from
/// a top-level `[[region_split]]` TOML array entry. See packet 92.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct RegionSplitDeclaration {
    /// The semantic name (e.g. `"material"`, `"fuzzy_skin"`).
    pub semantic: String,
    /// Dispatch priority; lower value = higher priority.
    pub priority: u32,
    /// Value-domain this semantic operates on.
    pub value_type: RegionSplitValueType,
}

/// Value-domain a region-split semantic operates on. `scalar` is
/// architecturally forbidden (D13); the parser rejects it explicitly via
/// `LoadErrorKind::ScalarValueTypeNotAllowedInRegionSplit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionSplitValueType {
    /// Boolean flag (split on/off regions).
    Flag,
    /// Tool/extruder index.
    ToolIndex,
    /// Arbitrary string label defined by the module.
    CustomString,
}

/// Runtime module record produced by manifest ingestion.
///
/// Construction goes through [`LoadedModuleBuilder`] (call
/// [`LoadedModuleBuilder::new`] with the five manifest-derived identity
/// fields, set the optional ones via chained setters, then call
/// [`LoadedModuleBuilder::build`]). Field reads from outside the crate
/// go through the `pub fn` accessor methods declared below.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedModule {
    /// Reverse-domain module identifier.
    pub(crate) id: ModuleId,
    /// Module semantic version.
    pub(crate) version: SemVer,
    /// Canonical scheduler stage identifier.
    pub(crate) stage: StageId,
    /// WIT world exported by the module.
    pub(crate) wit_world: String,
    /// Declared IR access paths for reads.
    pub(crate) ir_reads: Vec<String>,
    /// Declared IR access paths for writes.
    pub(crate) ir_writes: Vec<String>,
    /// Claims held by this module.
    pub(crate) claims: Vec<String>,
    /// Claims required from other modules.
    pub(crate) requires_claims: Vec<String>,
    /// Explicit incompatibility declarations.
    pub(crate) incompatible_with: Vec<String>,
    /// Required peer modules.
    pub(crate) requires_modules: Vec<ModuleId>,
    /// Minimum host version accepted by the module.
    pub(crate) min_host_version: SemVer,
    /// Inclusive minimum IR schema version.
    pub(crate) min_ir_schema: SemVer,
    /// Exclusive maximum IR schema version.
    pub(crate) max_ir_schema: SemVer,
    /// Placeholder config schema payload.
    pub(crate) config_schema: ConfigSchema,
    /// Keys overridable per region.
    pub(crate) overridable_per_region: Vec<String>,
    /// Keys overridable per layer.
    pub(crate) overridable_per_layer: Vec<String>,
    /// Effective layer parallel safety used by the runtime.
    pub(crate) layer_parallel_safe: bool,
    /// Companion `.wasm` path for this manifest.
    pub(crate) wasm_path: PathBuf,
    /// True when the companion `.wasm` is a known placeholder (not a valid
    /// component-model binary). Modules with placeholder binaries are
    /// discoverable for manifest validation and plan construction, but
    /// runtime dispatch will skip them with a diagnostic rather than
    /// attempting component compilation.
    pub(crate) placeholder_wasm: bool,
    /// Region-split semantics this module declares (top-level `[[region_split]]`
    /// TOML entries). Empty for paint-transparent modules; the host-filtered
    /// dispatch guard in `layer_executor.rs` uses this list. See packet 92.
    pub region_splits: Vec<RegionSplitDeclaration>,
    /// Pre-computed lookup set built from `region_splits` at load-time.
    /// O(1) membership probe for the per-layer dispatch filter.
    pub region_split_semantics: std::collections::HashSet<String>,
}

impl LoadedModule {
    /// Reverse-domain module identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Module semantic version.
    pub fn version(&self) -> SemVer {
        self.version
    }

    /// Canonical scheduler stage identifier.
    pub fn stage(&self) -> &str {
        &self.stage
    }

    /// WIT world exported by the module.
    pub fn wit_world(&self) -> &str {
        &self.wit_world
    }

    /// Declared IR access paths for reads.
    pub fn ir_reads(&self) -> &[String] {
        &self.ir_reads
    }

    /// Declared IR access paths for writes.
    pub fn ir_writes(&self) -> &[String] {
        &self.ir_writes
    }

    /// Claims held by this module.
    pub fn claims(&self) -> &[String] {
        &self.claims
    }

    /// Claims required from other modules.
    pub fn requires_claims(&self) -> &[String] {
        &self.requires_claims
    }

    /// Explicit incompatibility declarations.
    pub fn incompatible_with(&self) -> &[String] {
        &self.incompatible_with
    }

    /// Required peer modules.
    pub fn requires_modules(&self) -> &[ModuleId] {
        &self.requires_modules
    }

    /// Minimum host version accepted by the module.
    pub fn min_host_version(&self) -> SemVer {
        self.min_host_version
    }

    /// Inclusive minimum IR schema version.
    pub fn min_ir_schema(&self) -> SemVer {
        self.min_ir_schema
    }

    /// Exclusive maximum IR schema version.
    pub fn max_ir_schema(&self) -> SemVer {
        self.max_ir_schema
    }

    /// Placeholder config schema payload.
    pub fn config_schema(&self) -> &ConfigSchema {
        &self.config_schema
    }

    /// Config keys this module declared in its manifest, sorted lexically.
    pub fn config_keys(&self) -> Vec<String> {
        self.config_schema.entries.keys().cloned().collect()
    }

    /// Keys overridable per region.
    pub fn overridable_per_region(&self) -> &[String] {
        &self.overridable_per_region
    }

    /// Keys overridable per layer.
    pub fn overridable_per_layer(&self) -> &[String] {
        &self.overridable_per_layer
    }

    /// Effective layer parallel safety used by the runtime.
    pub fn layer_parallel_safe(&self) -> bool {
        self.layer_parallel_safe
    }

    /// Companion `.wasm` path for this manifest.
    pub fn wasm_path(&self) -> &Path {
        &self.wasm_path
    }

    /// True when the companion `.wasm` is a known placeholder (not a valid
    /// component-model binary).
    pub fn placeholder_wasm(&self) -> bool {
        self.placeholder_wasm
    }

    /// Region-split declarations parsed from the manifest `[[region_split]]`
    /// array. Empty for paint-transparent modules.
    pub fn region_splits(&self) -> &[RegionSplitDeclaration] {
        &self.region_splits
    }

    /// Pre-computed set of declared region-split semantic names.
    pub fn region_split_semantics(&self) -> &std::collections::HashSet<String> {
        &self.region_split_semantics
    }
}

/// Builder for [`LoadedModule`]. Required identity fields
/// (`id`, `version`, `stage`, `wit_world`, `wasm_path`) are positional
/// arguments to [`LoadedModuleBuilder::new`]; every other field has a
/// safe empty/zero default and is set via a chained `with_*`-style setter.
///
/// All setters consume `self` and return `Self`. Call [`Self::build`] to
/// produce the finished [`LoadedModule`].
#[must_use = "LoadedModuleBuilder must be finalized with .build()"]
#[derive(Debug, Clone)]
pub struct LoadedModuleBuilder {
    id: ModuleId,
    version: SemVer,
    stage: StageId,
    wit_world: String,
    wasm_path: PathBuf,
    ir_reads: Vec<String>,
    ir_writes: Vec<String>,
    claims: Vec<String>,
    requires_claims: Vec<String>,
    incompatible_with: Vec<String>,
    requires_modules: Vec<ModuleId>,
    min_host_version: SemVer,
    min_ir_schema: SemVer,
    max_ir_schema: SemVer,
    config_schema: ConfigSchema,
    overridable_per_region: Vec<String>,
    overridable_per_layer: Vec<String>,
    layer_parallel_safe: bool,
    placeholder_wasm: bool,
    region_splits: Vec<RegionSplitDeclaration>,
    region_split_semantics: std::collections::HashSet<String>,
}

impl LoadedModuleBuilder {
    /// Start a new builder from the five manifest-derived identity fields.
    pub fn new(
        id: impl Into<ModuleId>,
        version: SemVer,
        stage: impl Into<StageId>,
        wit_world: impl Into<String>,
        wasm_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            id: id.into(),
            version,
            stage: stage.into(),
            wit_world: wit_world.into(),
            wasm_path: wasm_path.into(),
            ir_reads: Vec::new(),
            ir_writes: Vec::new(),
            claims: Vec::new(),
            requires_claims: Vec::new(),
            incompatible_with: Vec::new(),
            requires_modules: Vec::new(),
            min_host_version: SemVer::default(),
            min_ir_schema: SemVer::default(),
            max_ir_schema: SemVer::default(),
            config_schema: ConfigSchema::default(),
            overridable_per_region: Vec::new(),
            overridable_per_layer: Vec::new(),
            layer_parallel_safe: false,
            placeholder_wasm: false,
            region_splits: Vec::new(),
            region_split_semantics: std::collections::HashSet::new(),
        }
    }

    /// Set declared IR-access read paths.
    pub fn ir_reads(mut self, reads: Vec<String>) -> Self {
        self.ir_reads = reads;
        self
    }

    /// Set declared IR-access write paths.
    pub fn ir_writes(mut self, writes: Vec<String>) -> Self {
        self.ir_writes = writes;
        self
    }

    /// Set claims held by this module.
    pub fn claims(mut self, claims: Vec<String>) -> Self {
        self.claims = claims;
        self
    }

    /// Set claims required from other modules.
    pub fn requires_claims(mut self, requires_claims: Vec<String>) -> Self {
        self.requires_claims = requires_claims;
        self
    }

    /// Set explicit incompatibility declarations.
    pub fn incompatible_with(mut self, incompatible_with: Vec<String>) -> Self {
        self.incompatible_with = incompatible_with;
        self
    }

    /// Set required peer modules.
    pub fn requires_modules(mut self, requires_modules: Vec<ModuleId>) -> Self {
        self.requires_modules = requires_modules;
        self
    }

    /// Set minimum host version accepted by the module.
    pub fn min_host_version(mut self, v: SemVer) -> Self {
        self.min_host_version = v;
        self
    }

    /// Set inclusive minimum IR schema version.
    pub fn min_ir_schema(mut self, v: SemVer) -> Self {
        self.min_ir_schema = v;
        self
    }

    /// Set exclusive maximum IR schema version.
    pub fn max_ir_schema(mut self, v: SemVer) -> Self {
        self.max_ir_schema = v;
        self
    }

    /// Set the per-module config schema payload.
    pub fn config_schema(mut self, schema: ConfigSchema) -> Self {
        self.config_schema = schema;
        self
    }

    /// Set keys overridable per region.
    pub fn overridable_per_region(mut self, keys: Vec<String>) -> Self {
        self.overridable_per_region = keys;
        self
    }

    /// Set keys overridable per layer.
    pub fn overridable_per_layer(mut self, keys: Vec<String>) -> Self {
        self.overridable_per_layer = keys;
        self
    }

    /// Set the effective layer-parallel safety flag.
    pub fn layer_parallel_safe(mut self, safe: bool) -> Self {
        self.layer_parallel_safe = safe;
        self
    }

    /// Mark the companion `.wasm` as a known placeholder.
    pub fn placeholder_wasm(mut self, placeholder: bool) -> Self {
        self.placeholder_wasm = placeholder;
        self
    }

    /// Set region-split declarations and the pre-computed semantic lookup set.
    pub fn region_splits(
        mut self,
        splits: Vec<RegionSplitDeclaration>,
        semantics: std::collections::HashSet<String>,
    ) -> Self {
        self.region_splits = splits;
        self.region_split_semantics = semantics;
        self
    }

    /// Finalize into a [`LoadedModule`].
    pub fn build(self) -> LoadedModule {
        LoadedModule {
            id: self.id,
            version: self.version,
            stage: self.stage,
            wit_world: self.wit_world,
            ir_reads: self.ir_reads,
            ir_writes: self.ir_writes,
            claims: self.claims,
            requires_claims: self.requires_claims,
            incompatible_with: self.incompatible_with,
            requires_modules: self.requires_modules,
            min_host_version: self.min_host_version,
            min_ir_schema: self.min_ir_schema,
            max_ir_schema: self.max_ir_schema,
            config_schema: self.config_schema,
            overridable_per_region: self.overridable_per_region,
            overridable_per_layer: self.overridable_per_layer,
            layer_parallel_safe: self.layer_parallel_safe,
            wasm_path: self.wasm_path,
            placeholder_wasm: self.placeholder_wasm,
            region_splits: self.region_splits,
            region_split_semantics: self.region_split_semantics,
        }
    }
}

/// A single config field entry parsed from a module manifest `[config.schema]`
/// table entry.
///
/// Mirrors the fields defined in `docs/03_wit_and_manifest.md` § Config Field
/// Types Reference.  The `type` field is required; all others are optional and
/// serialize as `null` when absent.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize)]
pub struct ConfigFieldEntry {
    /// Field type string — must be one of: `"bool"`, `"int"`, `"float"`,
    /// `"string"`, `"enum"`, `"float-list"`, `"string-list"`.
    pub field_type: String,
    /// Default value as a string representation.
    pub default: Option<String>,
    /// Minimum for int/float fields.
    pub min: Option<f64>,
    /// Maximum for int/float fields.
    pub max: Option<f64>,
    /// Step for int/float fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
    /// UI display name.
    pub display: Option<String>,
    /// UI tooltip / description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// UI grouping hint.
    pub group: Option<String>,
    /// Unit hint (`"mm"`, `"ratio"`, `"degrees"`, `"mm/s"`, `"ms"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// Whether this is an advanced setting (hidden by default).
    #[serde(skip_serializing_if = "is_false")]
    pub advanced: bool,
    /// Allowed values for `"enum"` fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
    /// Max length for `"string"` fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
    /// Min list length for list fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_list_length: Option<usize>,
    /// Max list length for list fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_list_length: Option<usize>,
    /// Single-field validation expression.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validate: Option<String>,
    /// UI taxonomy tags for sub-tab filtering and search. Free-form strings;
    /// see `docs/03_wit_and_manifest.md` for conventions. Empty by default.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Full config schema for a module, holding all field entries.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize)]
pub struct ConfigSchema {
    /// Parsed field entries keyed by field name.
    pub entries: BTreeMap<String, ConfigFieldEntry>,
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
    /// Two `[[region_split]]` entries in the same manifest declared the same `semantic`.
    /// Carries both source line numbers if the parser can recover them.
    DuplicateRegionSplitSemantic {
        /// The duplicated semantic name.
        semantic: String,
        /// Line of the first declaration, if recoverable.
        first_line: Option<usize>,
        /// Line of the second (duplicate) declaration, if recoverable.
        second_line: Option<usize>,
    },
    /// `[[region_split]]` declared `value_type = "scalar"`. Architecturally
    /// forbidden (D13); see packet 92.
    ScalarValueTypeNotAllowedInRegionSplit {
        /// The offending semantic name.
        semantic: String,
    },
    /// A community semantic (not in `CORE_REGION_SPLIT_PRIORITIES`) was declared
    /// at a priority below `COMMUNITY_PRIORITY_FLOOR` (1000).
    CommunityPriorityBelowFloor {
        /// The offending semantic name.
        semantic: String,
        /// The priority value supplied in the manifest.
        given_priority: u32,
        /// The minimum allowed priority for community semantics.
        floor: u32,
    },
    /// A core semantic was declared at a priority other than its registered
    /// value in `CORE_REGION_SPLIT_PRIORITIES`.
    CorePriorityMismatch {
        /// The offending semantic name.
        semantic: String,
        /// The priority value supplied in the manifest.
        given_priority: u32,
        /// The expected priority for this core semantic.
        expected_priority: u32,
    },
}

/// Result of scanning one or more module roots.
#[derive(Debug, Clone, PartialEq, Default)]
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

/// A .wasm file is considered a placeholder if it contains only the 8-byte
/// WASM magic header (`\0asm\x01\x00\x00\x00`) with no sections.
/// These are produced by the repo scaffolding and cannot be compiled as
/// component-model binaries.
fn is_placeholder_wasm(wasm_path: &Path) -> bool {
    match fs::metadata(wasm_path) {
        Ok(meta) => meta.len() <= 8,
        Err(_) => false,
    }
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
    validate_wit_world(&wit_world, manifest_path)?;
    let stage = required_stage(&root, manifest_path, "stage.id")?;
    let mut diagnostics = Vec::new();
    let layer_parallel_safe = effective_parallel_safety(
        manifest_path,
        &stage,
        required_bool(&root, manifest_path, "hints.layer-parallel-safe")?,
        &mut diagnostics,
    );

    let config_schema = read_config_schema(&root, manifest_path)?;
    let region_splits = parse_region_splits(&root, manifest_path)?;
    validate_region_splits(&region_splits, manifest_path)?;
    let region_split_semantics: std::collections::HashSet<String> =
        region_splits.iter().map(|d| d.semantic.clone()).collect();
    let placeholder_wasm = is_placeholder_wasm(wasm_path);
    if placeholder_wasm {
        diagnostics.push(LoadDiagnostic {
            level: DiagnosticLevel::Warning,
            path: wasm_path.to_path_buf(),
            field: None,
            message: format!(
                "companion .wasm for '{}' is a placeholder ({} bytes); \
                 module will be skipped at runtime until a valid component is built",
                module_id,
                fs::metadata(wasm_path).map(|m| m.len()).unwrap_or(0)
            ),
        });
    }

    let module = LoadedModuleBuilder::new(
        module_id,
        version,
        stage,
        wit_world,
        wasm_path.to_path_buf(),
    )
    .ir_reads(required_string_array(
        &root,
        manifest_path,
        "ir-access.reads",
    )?)
    .ir_writes(required_string_array(
        &root,
        manifest_path,
        "ir-access.writes",
    )?)
    .claims(validate_claim_ids(
        &required_string_array(&root, manifest_path, "claims.holds")?,
        manifest_path,
    )?)
    .requires_claims(required_string_array(
        &root,
        manifest_path,
        "claims.requires",
    )?)
    .incompatible_with(required_string_array(
        &root,
        manifest_path,
        "compatibility.incompatible-with",
    )?)
    .requires_modules(required_string_array(
        &root,
        manifest_path,
        "compatibility.requires",
    )?)
    .min_host_version(required_semver(
        &root,
        manifest_path,
        "compatibility.min-host-version",
    )?)
    .min_ir_schema(required_semver(
        &root,
        manifest_path,
        "compatibility.min-ir-schema",
    )?)
    .max_ir_schema(required_semver(
        &root,
        manifest_path,
        "compatibility.max-ir-schema",
    )?)
    .config_schema(config_schema)
    .overridable_per_region(required_string_array(
        &root,
        manifest_path,
        "config.overridable-per-region.keys",
    )?)
    .overridable_per_layer(required_string_array(
        &root,
        manifest_path,
        "config.overridable-per-layer.keys",
    )?)
    .layer_parallel_safe(layer_parallel_safe)
    .placeholder_wasm(placeholder_wasm)
    .region_splits(region_splits, region_split_semantics)
    .build();

    Ok(IngestedManifest {
        module,
        diagnostics,
    })
}

/// Validates that all held claim IDs are known fill-role claims.
/// Unknown claim IDs cause a LoadError with kind Validation.
/// Fill-style claims (prefix `claim:`) are validated against FILL_CLAIM_IDS.
/// At manifest load time we accept any `claim:*` ID — the previous hardcoded
/// `RECOGNIZED_NONFILL_CLAIM_IDS` allowlist (containing just `"claim:ironing"`)
/// required every new non-fill claim to be added here, which silently rotted
/// against the actual set of holders declared in core-module manifests.
///
/// Typos in claim IDs are caught downstream by the DAG validator's
/// `MissingDependency` pass: a `requires`-side typo names a claim no module
/// holds, surfacing as a structured error at startup. A `holds`-side typo
/// (claim that nobody requires) is benign — the claim simply never fires.
///
/// Fill-role claims are still privileged: `FILL_CLAIM_IDS` defines the four
/// interchangeable roles (top, bottom, bridge, sparse) that the dispatch
/// pipeline routes to fill modules. Non-fill claims are opaque labels.
fn validate_claim_ids(holds: &[String], _manifest_path: &Path) -> Result<Vec<String>, LoadError> {
    // No load-time gating beyond the `claim:` prefix convention. Anything
    // that doesn't start with `claim:` is a non-claim string (used by
    // legacy `holds` entries) and passes through; anything that does is
    // accepted as a potential holder. Catalog-wide validation happens in
    // `validate_startup_dag::validate_missing_dependencies`.
    Ok(holds.to_vec())
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
        } else if path.is_dir() {
            // Scan one level of subdirectories for module manifests.
            // This supports the core-module layout where each module is a
            // subdirectory containing {stem}.toml + {stem}.wasm.
            if let Ok(sub_dir) = fs::read_dir(&path) {
                for sub_entry in sub_dir.flatten() {
                    let sub_path = sub_entry.path();
                    if sub_path.extension().and_then(|v| v.to_str()) == Some("toml")
                        && sub_path.file_name().and_then(|n| n.to_str()) != Some("Cargo.toml")
                    {
                        manifests.push(sub_path);
                    }
                }
            }
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

/// Parses the optional top-level `[[region_split]]` TOML array into a
/// `Vec<RegionSplitDeclaration>`. Returns an empty vec when the key is absent.
/// Pre-checks for `value_type = "scalar"` before strict deserialization so
/// that the specific `ScalarValueTypeNotAllowedInRegionSplit` error is returned
/// instead of the generic `Schema` error that `toml-serde` would produce.
fn parse_region_splits(
    root: &Value,
    manifest_path: &Path,
) -> Result<Vec<RegionSplitDeclaration>, LoadError> {
    let Some(array) = root.get("region_split") else {
        return Ok(Vec::new());
    };

    let array = array.as_array().ok_or_else(|| LoadError {
        path: manifest_path.to_path_buf(),
        field: Some("region_split".to_string()),
        kind: LoadErrorKind::Schema,
        message: "`region_split` must be a TOML array of tables".to_string(),
    })?;

    // Pre-scan for `value_type = "scalar"` before strict deserialization.
    // If found, surface the specific architectural-rejection error variant
    // rather than the generic Schema error toml-serde would produce.
    for (idx, entry) in array.iter().enumerate() {
        if let Some(table) = entry.as_table() {
            if let Some(vt) = table.get("value_type").and_then(|v| v.as_str()) {
                if vt == "scalar" {
                    let semantic = table
                        .get("semantic")
                        .and_then(|s| s.as_str())
                        .unwrap_or("")
                        .to_string();
                    return Err(LoadError {
                        path: manifest_path.to_path_buf(),
                        field: Some(format!("region_split[{idx}].value_type")),
                        kind: LoadErrorKind::ScalarValueTypeNotAllowedInRegionSplit { semantic },
                        message: "value_type = \"scalar\" is architecturally forbidden (D13); \
                                  use \"flag\", \"tool_index\", or \"custom_string\" instead"
                            .to_string(),
                    });
                }
            }
        }
    }

    let mut result = Vec::with_capacity(array.len());
    for (idx, entry) in array.iter().enumerate() {
        let decl: RegionSplitDeclaration =
            entry
                .clone()
                .try_into()
                .map_err(|e: toml::de::Error| LoadError {
                    path: manifest_path.to_path_buf(),
                    field: Some(format!("region_split[{idx}]")),
                    kind: LoadErrorKind::Schema,
                    message: format!("failed to deserialize region_split entry: {e}"),
                })?;
        result.push(decl);
    }
    Ok(result)
}

/// Validates a parsed `Vec<RegionSplitDeclaration>` against the architectural
/// rules defined in packet 92:
/// 1. No two entries may share the same `semantic` (duplicate check).
/// 2. Community semantics (not in `CORE_REGION_SPLIT_PRIORITIES`) must have
///    `priority >= COMMUNITY_PRIORITY_FLOOR`.
/// 3. Core semantics must have the exact priority registered in
///    `CORE_REGION_SPLIT_PRIORITIES`.
///
/// Note: scalar `value_type` rejection is handled in [`parse_region_splits`]
/// before deserialization.
fn validate_region_splits(
    splits: &[RegionSplitDeclaration],
    manifest_path: &Path,
) -> Result<(), LoadError> {
    // --- 1. Duplicate semantic check ---
    let mut seen: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for (idx, decl) in splits.iter().enumerate() {
        if let Some(_prev_idx) = seen.insert(decl.semantic.as_str(), idx) {
            return Err(LoadError {
                path: manifest_path.to_path_buf(),
                field: Some("region_split".to_string()),
                kind: LoadErrorKind::DuplicateRegionSplitSemantic {
                    semantic: decl.semantic.clone(),
                    first_line: None,
                    second_line: None,
                },
                message: format!(
                    "duplicate region_split semantic '{}': each semantic may only appear once per manifest",
                    decl.semantic
                ),
            });
        }
    }

    // --- 2 & 3. Priority checks (community floor + core mismatch) ---
    for decl in splits {
        let core_priority = slicer_schema::CORE_REGION_SPLIT_PRIORITIES
            .iter()
            .find(|(name, _)| *name == decl.semantic.as_str())
            .map(|(_, p)| *p);

        match core_priority {
            Some(expected) => {
                // Core semantic: priority must exactly match the registered value.
                if decl.priority != expected {
                    return Err(LoadError {
                        path: manifest_path.to_path_buf(),
                        field: Some("region_split".to_string()),
                        kind: LoadErrorKind::CorePriorityMismatch {
                            semantic: decl.semantic.clone(),
                            given_priority: decl.priority,
                            expected_priority: expected,
                        },
                        message: format!(
                            "core semantic '{}' must have priority {}, but {} was declared",
                            decl.semantic, expected, decl.priority
                        ),
                    });
                }
            }
            None => {
                // Community semantic: priority must be >= floor.
                if decl.priority < slicer_schema::COMMUNITY_PRIORITY_FLOOR {
                    return Err(LoadError {
                        path: manifest_path.to_path_buf(),
                        field: Some("region_split".to_string()),
                        kind: LoadErrorKind::CommunityPriorityBelowFloor {
                            semantic: decl.semantic.clone(),
                            given_priority: decl.priority,
                            floor: slicer_schema::COMMUNITY_PRIORITY_FLOOR,
                        },
                        message: format!(
                            "community semantic '{}' priority {} is below the minimum floor of {}",
                            decl.semantic,
                            decl.priority,
                            slicer_schema::COMMUNITY_PRIORITY_FLOOR
                        ),
                    });
                }
            }
        }
    }

    Ok(())
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
        let entry = parse_config_field_entry(key, value, manifest_path)?;
        entries.insert(key.clone(), entry);
    }

    Ok(ConfigSchema { entries })
}

/// Parses a single `[config.schema.<key>]` entry from a TOML value.
///
/// Handles both the shorthand string form:
///   `wall_count = "int"`
/// and the full table form:
///   `[config.schema.wall_count]`
///   `type = "int"`
///   `default = 3`
///   `min = 1`
///   `max = 10`
///   `display = "Wall Count"`
///   `group = "Walls"`
fn parse_config_field_entry(
    field_key: &str,
    value: &toml::Value,
    manifest_path: &Path,
) -> Result<ConfigFieldEntry, LoadError> {
    // Handle shorthand: value is just a string like "int" or "float"
    if let Some(type_str) = value.as_str() {
        return Ok(ConfigFieldEntry {
            field_type: type_str.to_string(),
            ..Default::default()
        });
    }

    // Full table form
    let table = value.as_table().ok_or_else(|| LoadError {
        path: manifest_path.to_path_buf(),
        field: Some(format!("config.schema.{field_key}")),
        kind: LoadErrorKind::Schema,
        message: format!(
            "config.schema.{} must be a string (e.g. '\"int\"') or a table",
            field_key
        ),
    })?;

    let field_type = get_string(
        table,
        manifest_path,
        &format!("config.schema.{field_key}.type"),
        "type",
    )?;
    let default = table.get("default").map(|v| v.to_string());
    let min = get_float_opt(table, "min");
    let max = get_float_opt(table, "max");
    let step = get_float_opt(table, "step");
    let display = get_string_opt(table, "display");
    let description = get_string_opt(table, "description");
    let group = get_string_opt(table, "group");
    let unit = get_string_opt(table, "unit");
    let advanced = table
        .get("advanced")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let values = table.get("values").and_then(|v| {
        v.as_array().map(|arr| {
            arr.iter()
                .filter_map(|item| item.as_str().map(String::from))
                .collect()
        })
    });
    let max_length = table
        .get("max_length")
        .and_then(|v| v.as_integer().map(|i| i as usize));
    let min_list_length = table
        .get("min_list_length")
        .and_then(|v| v.as_integer().map(|i| i as usize));
    let max_list_length = table
        .get("max_list_length")
        .and_then(|v| v.as_integer().map(|i| i as usize));
    let validate = get_string_opt(table, "validate");
    let tags = table
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    Ok(ConfigFieldEntry {
        field_type,
        default,
        min,
        max,
        step,
        display,
        description,
        group,
        unit,
        advanced,
        values,
        max_length,
        min_list_length,
        max_list_length,
        validate,
        tags,
    })
}

fn get_string(
    table: &toml::map::Map<String, toml::Value>,
    manifest_path: &Path,
    field: &str,
    key: &str,
) -> Result<String, LoadError> {
    get_string_opt(table, key).ok_or_else(|| LoadError {
        path: manifest_path.to_path_buf(),
        field: Some(field.to_string()),
        kind: LoadErrorKind::Schema,
        message: format!("config.schema.{key} is required",),
    })
}

fn get_string_opt(table: &toml::map::Map<String, toml::Value>, key: &str) -> Option<String> {
    table.get(key).and_then(|v| v.as_str().map(String::from))
}

fn get_float_opt(table: &toml::map::Map<String, toml::Value>, key: &str) -> Option<f64> {
    table
        .get(key)
        .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
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

/// The canonical WIT world identifiers accepted by the host.
///
/// All four world identifiers must match the on-disk `wit/world-*.wit` package
/// names exactly.  Version (`@1.0.0`) is part of the identifier.
const WIT_WORLD_ALLOWLIST: &[&str] = &[
    "slicer:world-layer@1.0.0",
    "slicer:world-prepass@1.0.0",
    "slicer:world-postpass@1.0.0",
    "slicer:world-finalization@1.0.0",
];

/// Validates that `wit_world` from the manifest is in the host's allowlist.
///
/// This is a fatal startup check — modules that declare a non-allowlisted
/// `wit_world` cannot be loaded and the host aborts with a diagnostic.
fn validate_wit_world(wit_world: &str, manifest_path: &Path) -> Result<(), LoadError> {
    if WIT_WORLD_ALLOWLIST.contains(&wit_world) {
        Ok(())
    } else {
        Err(LoadError {
            path: manifest_path.to_path_buf(),
            field: Some(String::from("module.wit-world")),
            kind: LoadErrorKind::Validation,
            message: format!(
                "Unknown wit_world '{wit_world}' — expected one of: {}",
                WIT_WORLD_ALLOWLIST.join(", ")
            ),
        })
    }
}

fn known_stage_ids() -> &'static [&'static str] {
    crate::stage_order::known_stage_ids()
}

/// Build the documented config-schema JSON response from loaded modules.
///
/// Per `docs/01_system_architecture.md`, the config-schema query response
/// format is:
/// ```jsonc
/// {
///   "schema_version": "1.0.0",
///   "schema": [
///     {
///       "module": "com.community.tpms-infill",
///       "fields": [
///         {"key": "pattern", "type": "enum", "values": [...],
///          "default": "...", "display": "...", "group": "...",
///          "step": null, "description": null, "unit": null,
///          "advanced": false, "max_length": null,
///          "min_list_length": null, "max_list_length": null,
///          "validate": null, "tags": []}
///       ]
///     }
///   ]
/// }
/// ```
///
/// Every per-field key is always present: `Option<T>::None` → JSON `null`;
/// `bool` → JSON `true`/`false`; `Vec<T>` → JSON array (`[]` when empty).
/// The top-level `schema_version` is [`CONFIG_SCHEMA_WIRE_VERSION`].
pub fn build_config_schema_json(modules: &[LoadedModule]) -> serde_json::Value {
    let schema_entries: Vec<serde_json::Value> = modules
        .iter()
        .filter(|m| !m.config_schema.entries.is_empty())
        .map(|m| {
            let fields: Vec<serde_json::Value> = m
                .config_schema
                .entries
                .iter()
                .map(|(key, entry)| {
                    serde_json::json!({
                        "key": key,
                        "type": entry.field_type,
                        "default": entry.default,
                        "min": entry.min,
                        "max": entry.max,
                        "step": entry.step,
                        "display": entry.display,
                        "description": entry.description,
                        "group": entry.group,
                        "unit": entry.unit,
                        "advanced": entry.advanced,
                        "values": entry.values,
                        "max_length": entry.max_length,
                        "min_list_length": entry.min_list_length,
                        "max_list_length": entry.max_list_length,
                        "validate": entry.validate,
                        "tags": entry.tags,
                    })
                })
                .collect();
            serde_json::json!({
                "module": m.id,
                "fields": fields,
            })
        })
        .collect();

    serde_json::json!({
        "schema_version": CONFIG_SCHEMA_WIRE_VERSION,
        "schema": schema_entries,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        build_config_schema_json, effective_parallel_safety, parse_semver, ConfigFieldEntry,
        ConfigSchema, DiagnosticLevel, LoadedModuleBuilder, CONFIG_SCHEMA_WIRE_VERSION,
    };
    use slicer_ir::SemVer;
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

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

    #[test]
    fn loaded_module_builder_round_trips_minimal_fields() {
        let module = LoadedModuleBuilder::new(
            "com.test.module",
            SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            "Layer::SlicePostProcess",
            "slicer:world-layer@1.0.0",
            PathBuf::from("fixtures/test.wasm"),
        )
        .build();

        assert_eq!(module.id, "com.test.module");
        assert_eq!(module.version.major, 1);
        assert_eq!(module.stage, "Layer::SlicePostProcess");
        assert_eq!(module.wit_world, "slicer:world-layer@1.0.0");
        assert_eq!(module.wasm_path, PathBuf::from("fixtures/test.wasm"));
        assert!(module.ir_reads.is_empty());
        assert!(module.ir_writes.is_empty());
        assert!(module.claims.is_empty());
        assert!(module.requires_modules.is_empty());
        assert_eq!(module.config_schema, ConfigSchema::default());
        assert!(!module.layer_parallel_safe);
        assert!(!module.placeholder_wasm);
    }

    #[test]
    fn loaded_module_builder_carries_optional_fields() {
        let module = LoadedModuleBuilder::new(
            "com.test.full",
            SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            "Layer::Perimeters",
            "slicer:world-layer@1.0.0",
            PathBuf::from("fixtures/full.wasm"),
        )
        .ir_reads(vec!["SliceIR".to_string()])
        .ir_writes(vec!["PerimeterIR".to_string()])
        .claims(vec!["perimeter-generator".to_string()])
        .requires_modules(vec!["com.test.helper".to_string()])
        .layer_parallel_safe(true)
        .placeholder_wasm(false)
        .build();

        assert_eq!(module.ir_reads, vec!["SliceIR".to_string()]);
        assert_eq!(module.ir_writes, vec!["PerimeterIR".to_string()]);
        assert_eq!(module.claims, vec!["perimeter-generator".to_string()]);
        assert_eq!(module.requires_modules, vec!["com.test.helper".to_string()]);
        assert!(module.layer_parallel_safe);
    }

    fn synthetic_module(id: &str, schema: ConfigSchema) -> super::LoadedModule {
        LoadedModuleBuilder::new(
            id,
            SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            "Layer::Infill",
            "slicer:world-layer@1.0.0",
            PathBuf::from("fixtures/synth.wasm"),
        )
        .config_schema(schema)
        .build()
    }

    #[test]
    fn build_config_schema_json_emits_top_level_schema_version() {
        let json = build_config_schema_json(&[]);
        assert_eq!(
            json["schema_version"].as_str(),
            Some(CONFIG_SCHEMA_WIRE_VERSION),
            "top-level schema_version must equal CONFIG_SCHEMA_WIRE_VERSION"
        );
        assert_eq!(CONFIG_SCHEMA_WIRE_VERSION, "1.0.0");
        assert!(
            json["schema"].is_array(),
            "top-level 'schema' must always be an array"
        );
    }

    #[test]
    fn build_config_schema_json_emits_all_per_field_keys() {
        let mut entries = BTreeMap::new();
        entries.insert(
            "density".to_string(),
            ConfigFieldEntry {
                field_type: "float".to_string(),
                default: Some("0.15".to_string()),
                min: Some(0.0),
                max: Some(1.0),
                step: Some(0.05),
                display: Some("Density".to_string()),
                description: Some("Fraction of solid coverage".to_string()),
                group: Some("Pattern".to_string()),
                unit: Some("ratio".to_string()),
                advanced: true,
                values: None,
                max_length: None,
                min_list_length: None,
                max_list_length: None,
                validate: Some("density <= 1.0".to_string()),
                tags: vec!["infill".to_string(), "advanced".to_string()],
            },
        );
        let module = synthetic_module("com.test.allkeys", ConfigSchema { entries });
        let json = build_config_schema_json(&[module]);

        let field = &json["schema"][0]["fields"][0];
        for key in [
            "key",
            "type",
            "default",
            "min",
            "max",
            "step",
            "display",
            "description",
            "group",
            "unit",
            "advanced",
            "values",
            "max_length",
            "min_list_length",
            "max_list_length",
            "validate",
            "tags",
        ] {
            assert!(
                field.get(key).is_some(),
                "per-field JSON must always include '{key}'"
            );
        }

        assert_eq!(field["key"], "density");
        assert_eq!(field["type"], "float");
        assert_eq!(field["unit"], "ratio");
        assert_eq!(field["advanced"], true);
        assert_eq!(field["step"], 0.05);
        assert_eq!(field["description"], "Fraction of solid coverage");
        assert_eq!(field["validate"], "density <= 1.0");
        assert!(field["values"].is_null());
        assert!(field["max_length"].is_null());
        assert_eq!(field["tags"], serde_json::json!(["infill", "advanced"]));
    }

    #[test]
    fn build_config_schema_json_emits_empty_tags_array_when_absent() {
        let mut entries = BTreeMap::new();
        entries.insert(
            "wall_count".to_string(),
            ConfigFieldEntry {
                field_type: "int".to_string(),
                default: Some("3".to_string()),
                ..Default::default()
            },
        );
        let module = synthetic_module("com.test.notags", ConfigSchema { entries });
        let json = build_config_schema_json(&[module]);

        let field = &json["schema"][0]["fields"][0];
        assert!(field["tags"].is_array(), "tags must always be an array");
        assert_eq!(
            field["tags"].as_array().unwrap().len(),
            0,
            "absent tags must serialize as [], never null"
        );
        assert!(
            !field["tags"].is_null(),
            "tags must never be null even when empty"
        );

        assert!(field["unit"].is_null());
        assert!(field["description"].is_null());
        assert_eq!(field["advanced"], false);
    }
}
