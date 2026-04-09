//! QEM mesh decimation via meshopt.

use slicer_ir::MeshIR;

/// Configuration for mesh decimation.
#[derive(Debug, Clone)]
pub struct DecimateConfig {
    /// Absolute target triangle count. Mutually exclusive with `target_ratio`.
    pub target_count: Option<usize>,
    /// Fraction of original count to retain (0.0–1.0). Mutually exclusive with `target_count`.
    pub target_ratio: Option<f32>,
    /// Maximum allowed quadric error in internal units. Decimation stops early
    /// if this would be exceeded.
    pub max_error: f32,
    /// Use `simplify_sloppy` instead of `simplify`. Faster but may produce
    /// lower-quality results near boundaries.
    pub aggressive: bool,
}

impl Default for DecimateConfig {
    fn default() -> Self {
        Self {
            target_count: None,
            target_ratio: None,
            max_error: 0.01,
            aggressive: false,
        }
    }
}

/// Result of a mesh decimation operation.
#[derive(Debug, Clone)]
pub struct DecimateResult {
    /// The decimated mesh.
    pub mesh: MeshIR,
    /// Number of triangles in the input mesh.
    pub original_triangle_count: usize,
    /// Number of triangles in the output mesh.
    pub final_triangle_count: usize,
    /// The maximum quadric error achieved during decimation.
    pub achieved_error: f32,
}

/// Errors that can occur during mesh decimation.
#[derive(Debug, thiserror::Error)]
pub enum DecimateError {
    /// The input mesh contains no triangles.
    #[error("input mesh is empty")]
    EmptyMesh,
    /// The decimation configuration is invalid.
    #[error("invalid config: {0}")]
    InvalidConfig(String),
    /// An I/O error occurred.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Reduce triangle count via quadric error metric (QEM) edge collapse.
///
/// Exactly one of `config.target_count` or `config.target_ratio` must be specified.
pub fn decimate(_mesh: MeshIR, _config: DecimateConfig) -> Result<DecimateResult, DecimateError> {
    todo!("TASK-057: implement mesh decimation")
}
