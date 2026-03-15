//! Manifest ingestion contracts for the host scheduler.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use slicer_ir::{ModuleId, SemVer, StageId};

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
    /// Placeholder returned until ingestion is implemented.
    NotImplemented,
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
pub fn load_module_from_paths(manifest_path: &Path, _wasm_path: &Path) -> Result<LoadedModule, LoadError> {
    Err(LoadError {
        path: manifest_path.to_path_buf(),
        field: None,
        kind: LoadErrorKind::NotImplemented,
        message: String::from("manifest ingestion not implemented"),
    })
}

/// Scans search roots and loads all discovered modules.
pub fn load_modules_from_roots(search_roots: &[PathBuf]) -> Result<LoadModulesReport, LoadError> {
    let path = search_roots
        .first()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("<no-search-roots>"));

    Err(LoadError {
        path,
        field: None,
        kind: LoadErrorKind::NotImplemented,
        message: String::from("manifest root scanning not implemented"),
    })
}
