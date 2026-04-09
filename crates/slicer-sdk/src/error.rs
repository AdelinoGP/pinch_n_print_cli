//! Module error type for SDK.
//!
//! `ModuleError` represents errors returned from module functions.
//! Per docs/03_wit_and_manifest.md, it has code, message, and fatal fields.

use serde::{Deserialize, Serialize};

/// Error returned from module functions.
///
/// Matches WIT `record module-error { code: u32, message: string, fatal: bool }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleError {
    /// Error code (module-specific).
    pub code: u32,
    /// Human-readable error message.
    pub message: String,
    /// If true, the host aborts the current slice. If false, host logs and continues.
    pub fatal: bool,
}

impl ModuleError {
    /// Create a fatal error that will abort the slice.
    pub fn fatal(_code: u32, _message: impl Into<String>) -> Self {
        todo!("TASK-042: implement ModuleError::fatal constructor")
    }

    /// Create a non-fatal error that allows the host to continue.
    pub fn non_fatal(_code: u32, _message: impl Into<String>) -> Self {
        todo!("TASK-042: implement ModuleError::non_fatal constructor")
    }

    /// Convenience constructor from a string error (non-fatal).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(_message: impl Into<String>) -> Self {
        todo!("TASK-042: implement ModuleError::from_str constructor")
    }
}

impl std::fmt::Display for ModuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ModuleError(code={}, fatal={}, message={})",
            self.code, self.fatal, self.message
        )
    }
}

impl std::error::Error for ModuleError {}
