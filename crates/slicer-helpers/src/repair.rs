//! Mesh manifold repair — degenerate removal, orientation normalization, open-edge closure.

use slicer_ir::MeshIR;

/// Maximum number of vertices in a boundary loop that can be fan-capped.
/// Loops larger than this are skipped with a warning.
pub const MAX_REPAIR_CAP_VERTICES: usize = 256;

/// Result of a mesh repair operation.
#[derive(Debug, Clone)]
pub struct RepairResult {
    /// The repaired mesh.
    pub mesh: MeshIR,
    /// Statistics about what the repair operation changed.
    pub stats: RepairStats,
}

/// Statistics about a mesh repair operation.
#[derive(Debug, Clone, Default)]
pub struct RepairStats {
    /// Number of degenerate (zero-area) triangles removed.
    pub degenerate_removed: usize,
    /// Number of faces whose winding was corrected.
    pub faces_reoriented: usize,
    /// Number of open edges that were closed by fan-capping.
    pub open_edges_closed: usize,
    /// Number of disconnected mesh components found.
    pub components: usize,
    /// Non-fatal warnings encountered during repair.
    pub warnings: Vec<RepairWarning>,
}

/// Non-fatal warnings from the repair process.
#[derive(Debug, Clone, PartialEq)]
pub enum RepairWarning {
    /// A boundary loop was too large to fan-cap reliably.
    LargeCapLoop {
        /// Number of vertices in the skipped boundary loop.
        vertex_count: usize,
    },
    /// The mesh has multiple disconnected components.
    MultipleComponents {
        /// Number of components found.
        count: usize,
    },
}

/// Errors that can occur during mesh repair.
#[derive(Debug, thiserror::Error)]
pub enum RepairError {
    /// The input mesh contains no triangles.
    #[error("input mesh is empty")]
    EmptyMesh,
    /// An I/O error occurred.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Repair a mesh in place. Returns a [`RepairResult`].
///
/// Input mesh may be non-manifold. Output mesh is manifold unless warnings
/// indicate skipped loops.
pub fn repair(_mesh: MeshIR) -> Result<RepairResult, RepairError> {
    todo!("TASK-056: implement mesh repair")
}
