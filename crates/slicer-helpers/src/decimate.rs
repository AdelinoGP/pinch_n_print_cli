//! QEM mesh decimation via meshopt.

use meshopt::{DecodePosition, SimplifyOptions};
use slicer_ir::{IndexedTriangleSet, MeshIR, Point3};

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

/// Wrapper for f32 vertex positions to implement meshopt's `DecodePosition`.
#[derive(Debug, Clone, Copy)]
struct Vertex([f32; 3]);

impl DecodePosition for Vertex {
    fn decode_position(&self) -> [f32; 3] {
        self.0
    }
}

/// Reduce triangle count via quadric error metric (QEM) edge collapse.
///
/// Exactly one of `config.target_count` or `config.target_ratio` must be specified.
/// Each `ObjectMesh` in the input `MeshIR` is decimated independently.
pub fn decimate(mut mesh: MeshIR, config: DecimateConfig) -> Result<DecimateResult, DecimateError> {
    // Validate config: exactly one target must be set.
    match (config.target_count, config.target_ratio) {
        (Some(_), Some(_)) => {
            return Err(DecimateError::InvalidConfig(
                "both target_count and target_ratio are set; specify exactly one".to_string(),
            ));
        }
        (None, None) => {
            return Err(DecimateError::InvalidConfig(
                "neither target_count nor target_ratio is set; specify exactly one".to_string(),
            ));
        }
        _ => {}
    }

    // Check for empty mesh.
    let total_tris: usize = mesh
        .objects
        .iter()
        .map(|o| o.mesh.indices.len() / 3)
        .sum();
    if total_tris == 0 {
        return Err(DecimateError::EmptyMesh);
    }

    let original_triangle_count = total_tris;
    let mut final_triangle_count = 0usize;
    let mut max_achieved_error = 0.0f32;

    for obj in &mut mesh.objects {
        let tri_count = obj.mesh.indices.len() / 3;
        if tri_count == 0 {
            continue;
        }

        // Compute per-object target count.
        let obj_target = if let Some(count) = config.target_count {
            // Distribute proportionally across objects.
            let fraction = tri_count as f64 / original_triangle_count as f64;
            ((count as f64) * fraction).round().max(1.0) as usize
        } else {
            let ratio = config.target_ratio.unwrap();
            ((tri_count as f32) * ratio).round().max(1.0) as usize
        };

        // Convert vertices to meshopt format.
        let vertices: Vec<Vertex> = obj
            .mesh
            .vertices
            .iter()
            .map(|p| Vertex([p.x, p.y, p.z]))
            .collect();

        // Run decimation.
        let mut result_error = 0.0f32;
        let new_indices = if config.aggressive {
            meshopt::simplify_sloppy_decoder(
                &obj.mesh.indices,
                &vertices,
                obj_target * 3, // meshopt target_count is in indices, not triangles
                config.max_error,
                Some(&mut result_error),
            )
        } else {
            meshopt::simplify_decoder(
                &obj.mesh.indices,
                &vertices,
                obj_target * 3,
                config.max_error,
                SimplifyOptions::empty(),
                Some(&mut result_error),
            )
        };

        // Compact unused vertices.
        let (compacted_its, _) = compact_mesh(&obj.mesh.vertices, &new_indices);
        obj.mesh = compacted_its;

        final_triangle_count += obj.mesh.indices.len() / 3;
        if result_error > max_achieved_error {
            max_achieved_error = result_error;
        }
    }

    Ok(DecimateResult {
        mesh,
        original_triangle_count,
        final_triangle_count,
        achieved_error: max_achieved_error,
    })
}

/// Remove unreferenced vertices and remap indices.
/// Returns the compacted IndexedTriangleSet and a map from old to new vertex indices.
fn compact_mesh(
    original_vertices: &[Point3],
    indices: &[u32],
) -> (IndexedTriangleSet, Vec<Option<u32>>) {
    let mut old_to_new: Vec<Option<u32>> = vec![None; original_vertices.len()];
    let mut new_vertices = Vec::new();
    let mut new_indices = Vec::with_capacity(indices.len());

    for &idx in indices {
        let new_idx = if let Some(mapped) = old_to_new[idx as usize] {
            mapped
        } else {
            let n = new_vertices.len() as u32;
            new_vertices.push(original_vertices[idx as usize]);
            old_to_new[idx as usize] = Some(n);
            n
        };
        new_indices.push(new_idx);
    }

    (
        IndexedTriangleSet {
            vertices: new_vertices,
            indices: new_indices,
        },
        old_to_new,
    )
}
