//! Host-side scheduler and manifest ingestion APIs.

#![warn(missing_docs)]
#![warn(unused_imports)]
#![warn(unused_must_use)]

pub mod manifest;
pub mod dag;

pub use dag::{ModuleNode, SchedulerError, build_intra_stage_dag};
pub use manifest::{
    ConfigSchema, DiagnosticLevel, LoadDiagnostic, LoadError, LoadErrorKind, LoadModulesReport,
    LoadedModule, load_module_from_paths, load_modules_from_roots,
};
