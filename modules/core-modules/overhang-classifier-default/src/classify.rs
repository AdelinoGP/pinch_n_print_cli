// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/SupportMaterial.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the ModularSlicer architecture.
// -----------------------------------------------------------------------------
//! Overhang-quartile classifier.
//!
//! [`classify_layers`] walks a slice of [`LayerCollectionView`] in order and
//! returns the worst-case overhang quartile (Q1–Q4) for every wall-family
//! entity on layers ≥ 1, derived from signed distance to the previous layer's
//! wall geometry.

use crate::lines_distancer::LinesDistancer2D;
use slicer_ir::ExtrusionRole;
use slicer_sdk::traits::LayerCollectionView;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Classifies overhang quartiles for every wall-family entity across `layers`.
///
/// Returns a map from `(layer_index, entity_id)` to the worst-case (minimum)
/// quartile across all points in that entity.  Layer 0 is never classified
/// (no previous layer exists).  Non-wall roles are omitted.
pub fn classify_layers(layers: &[LayerCollectionView]) -> HashMap<(u32, u64), u8> {
    let mut results = HashMap::new();

    for i in 1..layers.len() {
        let prev_layer = &layers[i - 1];
        let cur_layer = &layers[i];

        let (segments, polygons) = build_prev_geometry(prev_layer);

        if segments.is_empty() {
            continue;
        }

        let distancer = LinesDistancer2D::new(segments);

        classify_layer(cur_layer, &distancer, &polygons, &mut results);
    }

    results
}

// ---------------------------------------------------------------------------
// Geometry helpers
// ---------------------------------------------------------------------------

/// Returns true when `role` belongs to the wall family.
#[inline]
fn is_wall_role(role: &ExtrusionRole) -> bool {
    matches!(
        role,
        ExtrusionRole::OuterWall | ExtrusionRole::InnerWall | ExtrusionRole::ThinWall
    )
}

/// Builds the segment list and polygon list from a layer's wall-family paths.
fn build_prev_geometry(
    layer: &LayerCollectionView,
) -> (Vec<([f32; 2], [f32; 2])>, Vec<Vec<[f32; 2]>>) {
    let mut segments: Vec<([f32; 2], [f32; 2])> = Vec::new();
    let mut polygons: Vec<Vec<[f32; 2]>> = Vec::new();

    for entity in layer.ordered_entities() {
        if !is_wall_role(&entity.role) {
            continue;
        }

        let pts = &entity.path.points;
        if pts.len() < 2 {
            continue;
        }

        let poly: Vec<[f32; 2]> = pts.iter().map(|p| [p.x, p.y]).collect();

        for w in pts.windows(2) {
            segments.push(([w[0].x, w[0].y], [w[1].x, w[1].y]));
        }
        let last = pts.last().unwrap();
        let first = &pts[0];
        segments.push(([last.x, last.y], [first.x, first.y]));

        polygons.push(poly);
    }

    (segments, polygons)
}

/// Classifies every wall-family entity in `layer` using `distancer` and
/// `polygons`, recording the worst-case quartile per entity.
fn classify_layer(
    layer: &LayerCollectionView,
    distancer: &LinesDistancer2D,
    polygons: &[Vec<[f32; 2]>],
    results: &mut HashMap<(u32, u64), u8>,
) {
    let layer_idx = layer.layer_index();

    for entity in layer.ordered_entities() {
        if !is_wall_role(&entity.role) {
            continue;
        }

        let mut worst_q: u8 = 0;

        for pt in &entity.path.points {
            let sd = distancer.signed_distance([pt.x, pt.y], polygons);
            let w = pt.width;

            let q: u8 = if sd > 0.0 {
                4
            } else if sd > -0.25 * w {
                3
            } else if sd > -0.5 * w {
                2
            } else {
                1
            };

            debug_assert!((1..=4).contains(&q), "quartile out of range: {q}");

            if worst_q == 0 || q < worst_q {
                worst_q = q;
            }
        }

        if worst_q > 0 {
            results.insert((layer_idx, entity.entity_id), worst_q);
        }
    }
}
