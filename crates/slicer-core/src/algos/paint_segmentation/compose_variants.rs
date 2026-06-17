// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/MultiMaterialSegmentation.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Phase 7 — Variant-chain composition for paint-segmentation.
//!
//! Takes per-semantic polygon outputs (from Phase 4 colorize/extract_segments
//! and Phase 6 top_bottom) and composes them into disjoint variant chains per
//! layer. Each variant chain key is a sorted `Vec<(semantic_name: String,
//! PaintValue)>`; the empty Vec represents BASE (unpainted) area.
//!
//! # Key encoding
//! `PaintValue` implements `Ord` (Flag < Scalar < ToolIndex < Custom; within
//! Scalar, `f32::total_cmp` is used). `ChainKey` elements are sorted
//! alphabetically by `semantic_name` for deterministic `BTreeMap` ordering.

use crate::polygon_ops::{difference_ex, intersection_ex, union_ex};
use slicer_ir::{ExPolygon, PaintSemantic, PaintValue};
use std::collections::BTreeMap;

/// Per-semantic polygon output to be composed into variant chains.
pub struct SemanticOutput {
    /// The semantic type for these regions.
    pub semantic: PaintSemantic,
    /// The specific paint value for these regions.
    pub value: PaintValue,
    /// `per_layer[l]` = ExPolygons for layer index `l`.
    pub per_layer: Vec<Vec<ExPolygon>>,
}

/// Returns a stable string name for a `PaintSemantic`.
fn semantic_name(s: &PaintSemantic) -> String {
    match s {
        PaintSemantic::Material => "material".to_owned(),
        PaintSemantic::FuzzySkin => "fuzzy_skin".to_owned(),
        PaintSemantic::SupportEnforcer => "support_enforcer".to_owned(),
        PaintSemantic::SupportBlocker => "support_blocker".to_owned(),
        PaintSemantic::Custom(name) => name.clone(),
    }
}

/// Variant-chain key type.
///
/// Each element is `(semantic_name, PaintValue)`. Elements are sorted
/// alphabetically by `semantic_name` for deterministic BTreeMap ordering.
/// `PaintValue` implements `Ord` (Flag < Scalar < ToolIndex < Custom; within
/// Scalar, `f32::total_cmp` provides the total ordering).
/// Empty Vec ⟹ BASE (unpainted) area.
pub type ChainKey = Vec<(String, PaintValue)>;

/// Compose per-semantic polygon outputs into disjoint variant chains per layer.
///
/// Returns one `BTreeMap` per layer indexed by layer index. Each map key is a
/// `ChainKey`:
/// - Empty `Vec` → BASE (unpainted) area.
/// - Non-empty `Vec` → painted by that exact combination of
///   `(semantic_name, value_repr)` pairs, ordered alphabetically by
///   semantic name.
///
/// Chains are assigned priority by length (longer chains win): the combined
/// intersection of multiple semantics is claimed before any individual
/// semantic's exclusive area.
///
/// # Panics (debug only)
/// In debug builds, asserts that all chains in the resulting map are
/// pairwise disjoint.
pub fn compose_variants(
    layer_total_contours: &[Vec<ExPolygon>],
    semantic_outputs: &[SemanticOutput],
) -> Vec<BTreeMap<ChainKey, Vec<ExPolygon>>> {
    let n_layers = layer_total_contours.len();
    if n_layers == 0 {
        return Vec::new();
    }

    let n_sem = semantic_outputs.len();

    // Pre-compute all non-empty subsets of the semantic index set. Each
    // subset's indices are sorted by semantic name for deterministic key
    // emission. The full list is then sorted descending by length so that
    // longer chains (higher specificity) are processed first.
    let subsets: Vec<Vec<usize>> = if n_sem == 0 {
        Vec::new()
    } else {
        let total = 1usize << n_sem;
        let mut subs: Vec<Vec<usize>> = Vec::with_capacity(total - 1);
        for mask in 1..total {
            let mut indices: Vec<usize> = (0..n_sem).filter(|&i| mask & (1 << i) != 0).collect();
            indices.sort_by_key(|&i| semantic_name(&semantic_outputs[i].semantic));
            subs.push(indices);
        }
        subs.sort_by_key(|b| std::cmp::Reverse(b.len()));
        subs
    };

    let mut result: Vec<BTreeMap<ChainKey, Vec<ExPolygon>>> = Vec::with_capacity(n_layers);

    for layer_idx in 0..n_layers {
        let total_contour = &layer_total_contours[layer_idx];
        let mut layer_map: BTreeMap<ChainKey, Vec<ExPolygon>> = BTreeMap::new();

        // Accumulates all already-claimed polygons so shorter chains can
        // subtract the area taken by longer chains.
        let mut claimed: Vec<ExPolygon> = Vec::new();

        for subset_indices in &subsets {
            // Intersect all member semantics for this layer.
            let mut chain_poly: Vec<ExPolygon> = {
                let first_idx = subset_indices[0];
                let base = semantic_outputs[first_idx]
                    .per_layer
                    .get(layer_idx)
                    .map_or(&[] as &[ExPolygon], |v| v.as_slice());
                let mut acc: Vec<ExPolygon> = base.to_vec();
                for &idx in &subset_indices[1..] {
                    let other = semantic_outputs[idx]
                        .per_layer
                        .get(layer_idx)
                        .map_or(&[] as &[ExPolygon], |v| v.as_slice());
                    acc = intersection_ex(&acc, other);
                    if acc.is_empty() {
                        break;
                    }
                }
                acc
            };

            // Subtract already-claimed area (from longer chains processed first).
            if !claimed.is_empty() && !chain_poly.is_empty() {
                chain_poly = difference_ex(&chain_poly, &claimed);
            }

            // Build chain key: sorted (semantic_name, PaintValue) pairs.
            let chain_key: ChainKey = subset_indices
                .iter()
                .map(|&i| {
                    (
                        semantic_name(&semantic_outputs[i].semantic),
                        semantic_outputs[i].value.clone(),
                    )
                })
                .collect();

            if !chain_poly.is_empty() {
                claimed = union_ex(&[claimed.as_slice(), chain_poly.as_slice()].concat());
            }

            layer_map.insert(chain_key, chain_poly);
        }

        // BASE chain: total contour minus all painted chains.
        let base_poly = if claimed.is_empty() {
            total_contour.to_vec()
        } else {
            difference_ex(total_contour, &claimed)
        };
        layer_map.insert(vec![], base_poly);

        // Debug disjointness invariant (AC-11 c).
        #[cfg(debug_assertions)]
        {
            let entries: Vec<(&ChainKey, &Vec<ExPolygon>)> = layer_map.iter().collect();
            for i in 0..entries.len() {
                for j in (i + 1)..entries.len() {
                    let overlap = intersection_ex(entries[i].1, entries[j].1);
                    debug_assert!(
                        overlap.is_empty(),
                        "compose_variants: disjointness violated between chains {:?} and {:?} \
                         on layer {}",
                        entries[i].0,
                        entries[j].0,
                        layer_idx
                    );
                }
            }
        }

        result.push(layer_map);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::{ExPolygon, PaintSemantic, PaintValue, Point2, Polygon};

    fn rect(x0_mm: f64, y0_mm: f64, x1_mm: f64, y1_mm: f64) -> ExPolygon {
        let u = |mm: f64| -> i64 { (mm * 10_000.0).round() as i64 };
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 {
                        x: u(x0_mm),
                        y: u(y0_mm),
                    },
                    Point2 {
                        x: u(x1_mm),
                        y: u(y0_mm),
                    },
                    Point2 {
                        x: u(x1_mm),
                        y: u(y1_mm),
                    },
                    Point2 {
                        x: u(x0_mm),
                        y: u(y1_mm),
                    },
                ],
            },
            holes: vec![],
        }
    }

    fn shoelace_area(exp: &ExPolygon) -> f64 {
        let pts = &exp.contour.points;
        let n = pts.len();
        if n < 3 {
            return 0.0;
        }
        let mut a = 0i64;
        for i in 0..n {
            let j = (i + 1) % n;
            a += pts[i].x * pts[j].y - pts[j].x * pts[i].y;
        }
        (a.abs() as f64) / 2.0
    }

    fn total_area(polys: &[ExPolygon]) -> f64 {
        polys.iter().map(shoelace_area).sum()
    }

    fn chain_key(pairs: &[(&str, PaintValue)]) -> ChainKey {
        pairs
            .iter()
            .map(|(s, v)| (s.to_string(), v.clone()))
            .collect()
    }

    #[test]
    fn compose_empty_inputs_returns_empty_layer_map() {
        let result = compose_variants(&[], &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn compose_base_chain_when_no_painted_semantics() {
        let contour = rect(0.0, 0.0, 1.0, 1.0);
        let expected_area = shoelace_area(&contour);
        let result = compose_variants(&[vec![contour]], &[]);
        assert_eq!(result.len(), 1);
        let map = &result[0];
        assert_eq!(map.len(), 1, "only BASE chain expected");
        let base = map.get(&vec![]).expect("BASE chain missing");
        let got = total_area(base);
        assert!(
            (got - expected_area).abs() < expected_area * 0.01,
            "BASE area {got} ≠ expected {expected_area}"
        );
    }

    #[test]
    fn compose_single_semantic_splits_base_and_painted() {
        let total = rect(0.0, 0.0, 1.0, 1.0);
        let painted_rect = rect(0.0, 0.0, 0.5, 1.0);
        let sem_out = SemanticOutput {
            semantic: PaintSemantic::Material,
            value: PaintValue::ToolIndex(1),
            per_layer: vec![vec![painted_rect]],
        };
        let result = compose_variants(&[vec![total.clone()]], &[sem_out]);
        let map = &result[0];
        assert_eq!(map.len(), 2, "expected BASE + 1 painted chain");

        let painted_key = chain_key(&[("material", PaintValue::ToolIndex(1))]);
        let painted_chain = map.get(&painted_key).expect("painted chain missing");
        let base_chain = map.get(&vec![]).expect("BASE chain missing");

        let total_area_val = shoelace_area(&total);
        let union_area = total_area(painted_chain) + total_area(base_chain);
        assert!(
            (union_area - total_area_val).abs() < total_area_val * 0.01,
            "union area {union_area} ≠ total {total_area_val}"
        );

        let overlap = intersection_ex(painted_chain, base_chain);
        assert!(
            overlap.is_empty(),
            "painted and base chains must not overlap"
        );
    }

    #[test]
    fn compose_two_semantics_produces_4_variants_at_full_overlap() {
        let full = rect(0.0, 0.0, 1.0, 1.0);
        let full_area = shoelace_area(&full);
        let sem_a = SemanticOutput {
            semantic: PaintSemantic::Material,
            value: PaintValue::ToolIndex(1),
            per_layer: vec![vec![full.clone()]],
        };
        let sem_b = SemanticOutput {
            semantic: PaintSemantic::SupportEnforcer,
            value: PaintValue::Flag(true),
            per_layer: vec![vec![full.clone()]],
        };
        let result = compose_variants(&[vec![full.clone()]], &[sem_a, sem_b]);
        let map = &result[0];
        assert_eq!(map.len(), 4, "expected 4 chains (3 painted + BASE)");

        // Combined chain ("material" < "support_enforcer") covers full area.
        let combined_key = chain_key(&[
            ("material", PaintValue::ToolIndex(1)),
            ("support_enforcer", PaintValue::Flag(true)),
        ]);
        let combined = map.get(&combined_key).expect("combined chain missing");
        let combined_area = total_area(combined);
        assert!(
            (combined_area - full_area).abs() < full_area * 0.01,
            "combined chain area {combined_area} ≠ full {full_area}"
        );

        // Individual-only chains are empty (all area claimed by combined).
        let mat_key = chain_key(&[("material", PaintValue::ToolIndex(1))]);
        let sup_key = chain_key(&[("support_enforcer", PaintValue::Flag(true))]);
        let mat_only = map.get(&mat_key).expect("material chain missing");
        let sup_only = map.get(&sup_key).expect("support_enforcer chain missing");
        assert!(
            total_area(mat_only) < full_area * 0.01,
            "material-only should be empty; area={}",
            total_area(mat_only)
        );
        assert!(
            total_area(sup_only) < full_area * 0.01,
            "support_enforcer-only should be empty; area={}",
            total_area(sup_only)
        );

        // BASE is empty.
        let base = map.get(&vec![]).expect("BASE missing");
        assert!(
            total_area(base) < full_area * 0.01,
            "BASE should be empty; area={}",
            total_area(base)
        );
    }

    #[test]
    fn compose_disjointness_invariant_holds() {
        // A=left half, B=right half, C=middle quarter — creates partial overlaps.
        let total = rect(0.0, 0.0, 1.0, 1.0);
        let sem_a = SemanticOutput {
            semantic: PaintSemantic::Material,
            value: PaintValue::ToolIndex(1),
            per_layer: vec![vec![rect(0.0, 0.0, 0.5, 1.0)]],
        };
        let sem_b = SemanticOutput {
            semantic: PaintSemantic::SupportEnforcer,
            value: PaintValue::Flag(true),
            per_layer: vec![vec![rect(0.5, 0.0, 1.0, 1.0)]],
        };
        let sem_c = SemanticOutput {
            semantic: PaintSemantic::FuzzySkin,
            value: PaintValue::Flag(true),
            per_layer: vec![vec![rect(0.25, 0.0, 0.75, 1.0)]],
        };
        let result = compose_variants(&[vec![total]], &[sem_a, sem_b, sem_c]);
        let map = &result[0];

        let entries: Vec<&Vec<ExPolygon>> = map.values().collect();
        for i in 0..entries.len() {
            for j in (i + 1)..entries.len() {
                let overlap = intersection_ex(entries[i], entries[j]);
                assert!(overlap.is_empty(), "chains {i} and {j} are not disjoint");
            }
        }
    }

    #[test]
    fn compose_variant_keys_are_deterministic_via_btreemap() {
        let total = rect(0.0, 0.0, 1.0, 1.0);
        let make_inputs = || {
            (
                vec![vec![total.clone()]],
                vec![
                    SemanticOutput {
                        semantic: PaintSemantic::Material,
                        value: PaintValue::ToolIndex(1),
                        per_layer: vec![vec![rect(0.0, 0.0, 0.5, 1.0)]],
                    },
                    SemanticOutput {
                        semantic: PaintSemantic::FuzzySkin,
                        value: PaintValue::Flag(false),
                        per_layer: vec![vec![rect(0.25, 0.0, 0.75, 1.0)]],
                    },
                ],
            )
        };

        let (c1, s1) = make_inputs();
        let (c2, s2) = make_inputs();
        let result1 = compose_variants(&c1, &s1);
        let result2 = compose_variants(&c2, &s2);

        let keys1: Vec<&ChainKey> = result1[0].keys().collect();
        let keys2: Vec<&ChainKey> = result2[0].keys().collect();
        assert_eq!(keys1, keys2, "key iteration order must be deterministic");
    }
}
