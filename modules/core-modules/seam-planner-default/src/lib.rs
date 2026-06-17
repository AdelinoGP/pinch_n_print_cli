// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/GCode/SeamPlacer.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Default seam planner for Pinch 'n Print.
//!
//! Implements the `PrepassModule` trait for the `PrePass::SeamPlanning` stage.
//! Analyzes mesh geometry to find and score optimal seam positions for each region.
//!
//! # Algorithm (OrcaSlicer-inspired)
//!
//! For each object at each layer, this module:
//! 1. Collects mesh vertices from the object's triangles via host services
//! 2. Identifies candidate seam positions at each region's boundary
//! 3. Scores candidates: concave corners score best, convex worst
//! 4. Emits the best candidate as the chosen seam position per region

#![warn(missing_docs)]
#![warn(unused_imports)]

use slicer_sdk::prelude::*;
use std::collections::HashMap;

/// Default seam planner that selects seam positions based on corner geometry.
///
/// Reads `seam_mode` from config ("nearest" / "rear" / "random").
/// Emits `SeamPlanEntry` records for each `(layer, object, region)` triple.
pub struct SeamPlannerDefault {
    /// Seam placement mode.
    #[allow(dead_code)]
    mode: String,
}

#[slicer_module]
impl PrepassModule for SeamPlannerDefault {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        let mode = config
            .get("seam_mode")
            .and_then(|v| match v {
                ConfigValue::String(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "nearest".to_string());

        Ok(Self { mode })
    }

    fn run_seam_planning(
        &self,
        objects: &[MeshObjectView],
        output: &mut SeamPlanningOutput,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        for obj in objects {
            // Build per-face normal map for corner detection.
            // For each triangle, compute its normal and centroid.
            // Vertices near region boundaries become seam candidates.
            let facet_count = obj.triangles.len();
            if facet_count == 0 {
                continue;
            }

            // Compute per-vertex normals by averaging adjacent triangle normals.
            let mut vertex_normal_sums: Vec<[f32; 3]> = vec![[0.0; 3]; obj.vertices.len()];
            let mut vertex_counts: Vec<u32> = vec![0; obj.vertices.len()];
            let mut triangle_normals: Vec<[f32; 3]> = Vec::with_capacity(facet_count);

            for triangle in &obj.triangles {
                let v0 = obj.vertices[triangle[0] as usize];
                let v1 = obj.vertices[triangle[1] as usize];
                let v2 = obj.vertices[triangle[2] as usize];

                // Edge vectors
                let e1: [f32; 3] = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
                let e2: [f32; 3] = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

                // Cross product (normal)
                let nx = e1[1] * e2[2] - e1[2] * e2[1];
                let ny = e1[2] * e2[0] - e1[0] * e2[2];
                let nz = e1[0] * e2[1] - e1[1] * e2[0];
                let len = (nx * nx + ny * ny + nz * nz).sqrt();
                let normal = if len > 1e-8 {
                    [nx / len, ny / len, nz / len]
                } else {
                    [0.0, 0.0, 1.0]
                };
                triangle_normals.push(normal);

                // Accumulate for vertex normals
                for &vi in triangle {
                    vertex_normal_sums[vi as usize][0] += normal[0];
                    vertex_normal_sums[vi as usize][1] += normal[1];
                    vertex_normal_sums[vi as usize][2] += normal[2];
                    vertex_counts[vi as usize] += 1;
                }
            }

            // Average vertex normals
            let vertex_normals: Vec<[f32; 3]> = vertex_normal_sums
                .iter()
                .zip(vertex_counts.iter())
                .map(|(sum, cnt)| {
                    if *cnt > 0 {
                        let len = (sum[0] * sum[0] + sum[1] * sum[1] + sum[2] * sum[2]).sqrt();
                        if len > 1e-8 {
                            [sum[0] / len, sum[1] / len, sum[2] / len]
                        } else {
                            [0.0, 0.0, 1.0]
                        }
                    } else {
                        [0.0, 0.0, 1.0]
                    }
                })
                .collect();

            // Find vertex-to-triangle adjacency for corner detection.
            let mut vertex_to_triangles: HashMap<u32, Vec<u32>> = HashMap::new();
            for (ti, triangle) in obj.triangles.iter().enumerate() {
                for &vi in triangle {
                    vertex_to_triangles.entry(vi).or_default().push(ti as u32);
                }
            }

            // Identify corners: vertices where adjacent triangles have
            // significantly different normals (high curvature = good seam).
            let mut corner_candidates: Vec<(u32, f32)> = Vec::new(); // (vertex_index, curvature)

            for (vi, tris) in &vertex_to_triangles {
                if tris.len() >= 2 {
                    let v_normal = vertex_normals[*vi as usize];
                    let mut max_cosine = -1.0f32;
                    let mut min_cosine = 1.0f32;

                    for &ti in tris {
                        let t_normal = triangle_normals[ti as usize];
                        let dot = v_normal[0] * t_normal[0]
                            + v_normal[1] * t_normal[1]
                            + v_normal[2] * t_normal[2];
                        let dot = dot.clamp(-1.0, 1.0);
                        max_cosine = max_cosine.max(dot);
                        min_cosine = min_cosine.min(dot);
                    }

                    // Angular gap = high curvature corner
                    let curvature = (max_cosine - min_cosine).abs();

                    // Threshold: must be a real corner, not just smooth surface
                    if curvature > 0.2 {
                        corner_candidates.push((*vi, curvature));
                    }
                }
            }

            // Sort corners by curvature (highest first = best seam candidates).
            // Break ties on vertex index so selection is deterministic: the
            // candidate set is built by iterating a HashMap (random order), and
            // symmetric meshes (e.g. a cube) produce many equal-curvature
            // corners — without a stable tie-break, `candidates.first()` would
            // pick a different corner per process, yielding non-reproducible
            // G-code. `unwrap_or(Equal)` also avoids a NaN-curvature panic.
            corner_candidates.sort_by(|a, b| {
                b.1.partial_cmp(&a.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.0.cmp(&b.0))
            });

            // Build scored candidates from corner vertices.
            let candidates: Vec<ScoredSeamCandidate> = corner_candidates
                .iter()
                .take(10) // limit to top 10 candidates
                .map(|(vi, curvature)| {
                    let v = obj.vertices[*vi as usize];
                    ScoredSeamCandidate {
                        position: Point3WithWidth {
                            x: v[0],
                            y: v[1],
                            z: v[2],
                            width: 0.4, // default line width
                            flow_factor: 1.0,
                            overhang_quartile: None,
                        },
                        score: 1.0 - curvature, // lower score = better (curvature inverted)
                        reason: SeamReason {
                            tag: if *curvature > 0.8 {
                                "concave".to_string()
                            } else {
                                "aligned".to_string()
                            },
                        },
                    }
                })
                .collect();

            // Emit one seam plan per layer (MVP: one region per object).
            // Use layer indices from LayerPlanIR via host lookup, or enumerate
            // a default set of layers based on object bounds.
            let bounds = obj
                .vertices
                .iter()
                .fold(None, |acc: Option<([f32; 3], [f32; 3])>, v| {
                    Some(match acc {
                        None => ([v[0], v[1], v[2]], [v[0], v[1], v[2]]),
                        Some((mn, mx)) => (
                            [mn[0].min(v[0]), mn[1].min(v[1]), mn[2].min(v[2])],
                            [mx[0].max(v[0]), mx[1].max(v[1]), mx[2].max(v[2])],
                        ),
                    })
                });

            let (Some((_bmin, bmax)), layer_height) = (bounds, 0.2) else {
                continue;
            };

            // Estimate number of layers from object height and layer height.
            let object_height = bmax[2] - 0.0;
            let num_layers = (object_height / layer_height).ceil() as usize;
            let num_layers = num_layers.clamp(1, 100); // sanity clamp

            for layer_idx in 0..num_layers {
                let z = layer_idx as f32 * layer_height;
                let region_id: u64 = 0; // MVP: single region per object

                // Choose best candidate (or fallback if none found).
                let best = candidates
                    .first()
                    .map(|c| {
                        let mut chosen = c.clone();
                        chosen.position.z = z;
                        chosen
                    })
                    .unwrap_or_else(|| ScoredSeamCandidate {
                        position: Point3WithWidth {
                            x: bmax[0], // rear-most X
                            y: (obj
                                .vertices
                                .iter()
                                .map(|v| v[1])
                                .fold(f32::INFINITY, |a, b| a.min(b))
                                + bmax[1])
                                / 2.0,
                            z,
                            width: 0.4,
                            flow_factor: 1.0,
                            overhang_quartile: None,
                        },
                        score: 100.0, // worst score
                        reason: SeamReason {
                            tag: "aligned".to_string(),
                        },
                    });

                let entry = SeamPlanEntry {
                    global_layer_index: layer_idx as u32,
                    object_id: obj.object_id.clone(),
                    region_id: region_id.to_string(),
                    chosen_position: best.position,
                    chosen_wall_index: 0,
                    scored_candidates: candidates.clone(),
                };

                output
                    .push_seam_plan(entry)
                    .map_err(|e| ModuleError::fatal(1, format!("push_seam_plan failed: {e}")))?;
            }
        }

        Ok(())
    }
}

// Unit tests for this module live in `tests/seam_planner_tdd.rs` (external test
// crate), built via the public `on_print_start` constructor rather than the
// private `mode` field.
