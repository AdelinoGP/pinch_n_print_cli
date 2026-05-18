//! QEM mesh decimation via meshopt.

use meshopt::{DecodePosition, SimplifyOptions};
use slicer_ir::{IndexedTriangleSet, MeshIR, Point3};

/// Configuration for mesh decimation.
///
/// Construct via [`DecimateConfigBuilder`]. Fields are crate-private;
/// invariants (exactly one of `target_count`/`target_ratio` set,
/// `max_error > 0.0`) are enforced at [`DecimateConfigBuilder::build`] time.
#[derive(Debug, Clone)]
pub struct DecimateConfig {
    pub(crate) target_count: Option<usize>,
    pub(crate) target_ratio: Option<f32>,
    pub(crate) max_error: f32,
    pub(crate) aggressive: bool,
}

/// Builder for [`DecimateConfig`]. Consuming-style setters; terminal
/// [`build`](Self::build) validates the configuration.
#[derive(Debug, Clone)]
#[must_use]
pub struct DecimateConfigBuilder {
    target_count: Option<usize>,
    target_ratio: Option<f32>,
    max_error: f32,
    aggressive: bool,
}

impl Default for DecimateConfigBuilder {
    fn default() -> Self {
        Self {
            target_count: None,
            target_ratio: None,
            max_error: 0.01,
            aggressive: false,
        }
    }
}

impl DecimateConfigBuilder {
    /// Start a new builder with the project default `max_error` (0.01) and
    /// no target set. A target must be supplied via [`target_count`](Self::target_count)
    /// or [`target_ratio`](Self::target_ratio) before [`build`](Self::build).
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the absolute target triangle count. Mutually exclusive with
    /// [`target_ratio`](Self::target_ratio).
    pub fn target_count(mut self, n: usize) -> Self {
        self.target_count = Some(n);
        self
    }

    /// Set the fraction of the original count to retain (0.0–1.0). Mutually
    /// exclusive with [`target_count`](Self::target_count).
    pub fn target_ratio(mut self, ratio: f32) -> Self {
        self.target_ratio = Some(ratio);
        self
    }

    /// Set the maximum allowed quadric error. Must be `> 0.0`.
    pub fn max_error(mut self, e: f32) -> Self {
        self.max_error = e;
        self
    }

    /// Toggle the sloppy/aggressive simplification path.
    pub fn aggressive(mut self, b: bool) -> Self {
        self.aggressive = b;
        self
    }

    /// Validate and produce a [`DecimateConfig`].
    ///
    /// Returns [`DecimateError::InvalidConfig`] when:
    /// - neither `target_count` nor `target_ratio` is set,
    /// - both are set, or
    /// - `max_error <= 0.0`.
    pub fn build(self) -> Result<DecimateConfig, DecimateError> {
        match (self.target_count, self.target_ratio) {
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
        if self.max_error <= 0.0 {
            return Err(DecimateError::InvalidConfig(
                "max_error must be > 0.0".to_string(),
            ));
        }
        Ok(DecimateConfig {
            target_count: self.target_count,
            target_ratio: self.target_ratio,
            max_error: self.max_error,
            aggressive: self.aggressive,
        })
    }
}

/// Result of a mesh decimation operation.
#[derive(Debug, Clone, Default)]
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
/// Build `config` with [`DecimateConfigBuilder`]; construction guarantees
/// exactly one of `target_count`/`target_ratio` is set and `max_error > 0.0`.
/// Each `ObjectMesh` in the input `MeshIR` is decimated independently.
pub fn decimate(mut mesh: MeshIR, config: DecimateConfig) -> Result<DecimateResult, DecimateError> {
    // Check for empty mesh.
    let total_tris: usize = mesh.objects.iter().map(|o| o.mesh.indices.len() / 3).sum();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_errors_when_no_target_set() {
        let err = DecimateConfigBuilder::new().build().unwrap_err();
        match err {
            DecimateError::InvalidConfig(msg) => assert!(msg.contains("neither")),
            other => panic!("expected InvalidConfig, got {other:?}"),
        }
    }

    #[test]
    fn build_errors_when_both_targets_set() {
        let err = DecimateConfigBuilder::new()
            .target_count(400)
            .target_ratio(0.5)
            .build()
            .unwrap_err();
        match err {
            DecimateError::InvalidConfig(msg) => assert!(msg.contains("both")),
            other => panic!("expected InvalidConfig, got {other:?}"),
        }
    }

    #[test]
    fn build_errors_when_max_error_non_positive() {
        let err = DecimateConfigBuilder::new()
            .target_ratio(0.5)
            .max_error(0.0)
            .build()
            .unwrap_err();
        match err {
            DecimateError::InvalidConfig(msg) => assert!(msg.contains("max_error")),
            other => panic!("expected InvalidConfig, got {other:?}"),
        }
    }

    #[test]
    fn build_ok_with_target_count() {
        let cfg = DecimateConfigBuilder::new()
            .target_count(400)
            .build()
            .unwrap();
        assert_eq!(cfg.target_count, Some(400));
        assert_eq!(cfg.target_ratio, None);
        assert!((cfg.max_error - 0.01).abs() < f32::EPSILON);
        assert!(!cfg.aggressive);
    }

    #[test]
    fn build_ok_with_target_ratio_and_overrides() {
        let cfg = DecimateConfigBuilder::new()
            .target_ratio(0.25)
            .max_error(0.5)
            .aggressive(true)
            .build()
            .unwrap();
        assert_eq!(cfg.target_count, None);
        assert_eq!(cfg.target_ratio, Some(0.25));
        assert!((cfg.max_error - 0.5).abs() < f32::EPSILON);
        assert!(cfg.aggressive);
    }
}
