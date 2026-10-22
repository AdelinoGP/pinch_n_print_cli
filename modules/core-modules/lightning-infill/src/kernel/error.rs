//! Error type for the module-private lightning kernel.

use std::fmt;

/// Errors produced by [`super::generate_lightning_trees`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum KernelError {
    /// 139 wiring point: replace this variant with the real error set the
    /// distance-field / tree-node generator can produce.
    Unimplemented,
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unimplemented => f.write_str(
                "lightning tree generation not yet implemented (packet 139 wiring point)",
            ),
        }
    }
}

impl std::error::Error for KernelError {}
