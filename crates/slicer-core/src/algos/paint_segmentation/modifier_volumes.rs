// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/MultiMaterialSegmentation.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the ModularSlicer architecture.
// -----------------------------------------------------------------------------
//! Modifier-volume slicing for paint segmentation (Step 10 / AC-13 / D14).
//!
//! Slices each modifier volume's mesh per layer and produces per-layer
//! per-semantic polygon lists that the v2 driver routes into the BASE variant
//! chain's `segment_annotations` only.
//!
//! # D14 Invariant
//! Caller **MUST** route the output onto the BASE variant chain only
//! (`variant_chain.is_empty()`). Painted variant chains must never receive
//! modifier-volume annotations.

use slicer_ir::{ExPolygon, PaintSemantic};

/// Per-layer modifier-volume polygons keyed by PaintSemantic.
#[derive(Debug, Clone, PartialEq)]
pub struct ModifierVolumeLayer {
    /// Paint semantic carried by this modifier volume (`SupportEnforcer` or `SupportBlocker` only).
    pub semantic: PaintSemantic,
    /// Polygons produced by slicing this modifier volume at the corresponding layer Z.
    pub polygons: Vec<ExPolygon>,
}

/// Slice every modifier-volume mesh per layer and produce per-layer per-semantic polygon lists.
///
/// # D14 invariant
/// Caller MUST route the output onto the BASE variant chain ONLY in
/// `SlicedRegion.segment_annotations`, never on painted chains.
///
/// # Arguments
/// * `mesh`     – The full MeshIR (iterates `object.modifier_volumes`).
/// * `layer_zs` – Layer center Z values in millimetres, indexed by layer_idx.
///
/// # Returns
/// `Vec<Vec<ModifierVolumeLayer>>` — outer index is `layer_idx`, inner has one
/// entry per (semantic) that has non-empty polygons on that layer.
pub fn slice_modifier_volumes(
    mesh: &slicer_ir::MeshIR,
    layer_zs: &[f32],
) -> Vec<Vec<ModifierVolumeLayer>> {
    use std::collections::HashMap;

    // Pre-allocate one bucket per layer.
    let n = layer_zs.len();
    // per_layer[layer_idx] -> HashMap<semantic_key, Vec<ExPolygon>>
    let mut per_layer: Vec<HashMap<PaintSemanticKey, Vec<ExPolygon>>> =
        (0..n).map(|_| HashMap::new()).collect();

    for object in &mesh.objects {
        for mv in &object.modifier_volumes {
            let subtype = match mv.config_delta.fields.get("subtype") {
                Some(slicer_ir::ConfigValue::String(s)) => s.as_str(),
                _ => continue,
            };
            let semantic = match subtype {
                "support_enforcer" => PaintSemantic::SupportEnforcer,
                "support_blocker" => PaintSemantic::SupportBlocker,
                _ => continue, // Material etc. are not modifier semantics
            };
            if mv.mesh.vertices.is_empty() || mv.mesh.indices.is_empty() {
                continue;
            }

            let projections = crate::slice_mesh_ex(&mv.mesh, layer_zs);
            for (layer_idx, polys) in projections.into_iter().enumerate() {
                if polys.is_empty() {
                    continue;
                }
                per_layer[layer_idx]
                    .entry(PaintSemanticKey::from(&semantic))
                    .or_default()
                    .extend(polys);
            }
        }
    }

    // Convert into the public shape.
    per_layer
        .into_iter()
        .map(|bucket| {
            bucket
                .into_iter()
                .map(|(key, polygons)| ModifierVolumeLayer {
                    semantic: key.into(),
                    polygons,
                })
                .collect()
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// A cheap hashable key for `PaintSemantic` (avoids Clone-then-hash on the full enum).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum PaintSemanticKey {
    SupportEnforcer,
    SupportBlocker,
}

impl From<&PaintSemantic> for PaintSemanticKey {
    fn from(s: &PaintSemantic) -> Self {
        match s {
            PaintSemantic::SupportEnforcer => PaintSemanticKey::SupportEnforcer,
            PaintSemantic::SupportBlocker => PaintSemanticKey::SupportBlocker,
            _ => unreachable!("only enforcer/blocker reach this path"),
        }
    }
}

impl From<PaintSemanticKey> for PaintSemantic {
    fn from(k: PaintSemanticKey) -> Self {
        match k {
            PaintSemanticKey::SupportEnforcer => PaintSemantic::SupportEnforcer,
            PaintSemanticKey::SupportBlocker => PaintSemantic::SupportBlocker,
        }
    }
}

// ---------------------------------------------------------------------------
// Point-in-polygon helpers (delegates to slicer_ir::point_in_polygon_winding)
// ---------------------------------------------------------------------------

const POINT_IN_POLY_EPS_MM: f64 = 0.001;

/// Returns `true` if `point` (integer units, 1 unit = 100 nm) is inside `expolygon`.
///
/// Uses `slicer_ir::point_in_polygon_winding` against the outer contour, then
/// excludes points that fall inside any hole.
pub(crate) fn expolygon_contains_point(exp: &ExPolygon, point: slicer_ir::Point2) -> bool {
    let px_mm = point.x as f64 / 10_000.0;
    let py_mm = point.y as f64 / 10_000.0;
    if !slicer_ir::point_in_polygon_winding(exp, px_mm, py_mm, POINT_IN_POLY_EPS_MM) {
        return false;
    }
    // Subtract holes: if the point is inside any hole it is NOT inside the expolygon.
    for hole in &exp.holes {
        let hole_poly = ExPolygon {
            contour: hole.clone(),
            holes: Vec::new(),
        };
        if slicer_ir::point_in_polygon_winding(&hole_poly, px_mm, py_mm, POINT_IN_POLY_EPS_MM) {
            return false;
        }
    }
    true
}

/// Returns `true` if `point` is inside any ExPolygon in `polys`.
pub(crate) fn any_expolygon_contains_point(polys: &[ExPolygon], point: slicer_ir::Point2) -> bool {
    polys.iter().any(|ep| expolygon_contains_point(ep, point))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::{
        BoundingBox3, ConfigDelta, ConfigValue, ExPolygon, IndexedTriangleSet, ModifierScope,
        ModifierVolume, ObjectConfig, ObjectMesh, PaintSemantic, Point2, Point3, Polygon,
        Transform3d, CURRENT_MESH_IR_SCHEMA_VERSION,
    };
    use std::collections::HashMap;

    fn identity_transform() -> Transform3d {
        Transform3d {
            matrix: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    fn default_build_volume() -> BoundingBox3 {
        BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 250.0,
                y: 210.0,
                z: 220.0,
            },
        }
    }

    fn empty_mesh() -> slicer_ir::MeshIR {
        slicer_ir::MeshIR {
            schema_version: CURRENT_MESH_IR_SCHEMA_VERSION,
            objects: Vec::new(),
            build_volume: default_build_volume(),
        }
    }

    /// Build a minimal cube IndexedTriangleSet from 0,0,0 to (size,size,size).
    fn cube_mesh(size: f32) -> IndexedTriangleSet {
        // 8 vertices
        let v = |x, y, z| Point3 { x, y, z };
        let vertices = vec![
            v(0.0, 0.0, 0.0),    // 0
            v(size, 0.0, 0.0),   // 1
            v(size, size, 0.0),  // 2
            v(0.0, size, 0.0),   // 3
            v(0.0, 0.0, size),   // 4
            v(size, 0.0, size),  // 5
            v(size, size, size), // 6
            v(0.0, size, size),  // 7
        ];
        #[rustfmt::skip]
        let indices = vec![
            // bottom
            0, 2, 1,  0, 3, 2,
            // top
            4, 5, 6,  4, 6, 7,
            // front
            0, 1, 5,  0, 5, 4,
            // back
            2, 3, 7,  2, 7, 6,
            // left
            0, 4, 7,  0, 7, 3,
            // right
            1, 2, 6,  1, 6, 5,
        ];
        IndexedTriangleSet { vertices, indices }
    }

    fn make_modifier_volume(subtype: &str, mesh: IndexedTriangleSet) -> ModifierVolume {
        let mut fields = HashMap::new();
        fields.insert(
            "subtype".to_string(),
            ConfigValue::String(subtype.to_string()),
        );
        ModifierVolume {
            id: "mv1".to_string(),
            mesh,
            config_delta: ConfigDelta { fields },
            priority: 0,
            applies_to: ModifierScope::AllFeatures,
        }
    }

    fn mesh_with_modifier(subtype: &str, mv_mesh: IndexedTriangleSet) -> slicer_ir::MeshIR {
        slicer_ir::MeshIR {
            schema_version: CURRENT_MESH_IR_SCHEMA_VERSION,
            objects: vec![ObjectMesh {
                id: "obj1".to_string(),
                mesh: cube_mesh(10.0),
                transform: identity_transform(),
                config: ObjectConfig {
                    data: HashMap::new(),
                },
                modifier_volumes: vec![make_modifier_volume(subtype, mv_mesh)],
                paint_data: None,
                world_z_extent: None,
            }],
            build_volume: default_build_volume(),
        }
    }

    // ---- slice_modifier_volumes_empty_mesh_returns_empty ------------------

    #[test]
    fn slice_modifier_volumes_empty_mesh_returns_empty() {
        let mesh = empty_mesh();
        let layer_zs = vec![0.1, 0.3, 0.5];
        let result = slice_modifier_volumes(&mesh, &layer_zs);
        assert_eq!(result.len(), 3, "one bucket per layer");
        for (i, bucket) in result.iter().enumerate() {
            assert!(
                bucket.is_empty(),
                "layer {i}: no modifier volumes in empty mesh"
            );
        }
    }

    // ---- slice_modifier_volumes_support_enforcer_routes_correctly ----------

    #[test]
    fn slice_modifier_volumes_support_enforcer_routes_correctly() {
        // 1×1×1 mm cube, slice at z=0.5 (midpoint) — should produce polygons.
        let mv_mesh = cube_mesh(1.0);
        let mesh = mesh_with_modifier("support_enforcer", mv_mesh);
        let layer_zs = vec![0.1, 0.5, 0.9];
        let result = slice_modifier_volumes(&mesh, &layer_zs);

        assert_eq!(result.len(), 3);

        // At least one layer should have non-empty SupportEnforcer polygons.
        let any_enforcer = result.iter().any(|bucket| {
            bucket.iter().any(|mvl| {
                mvl.semantic == PaintSemantic::SupportEnforcer && !mvl.polygons.is_empty()
            })
        });
        assert!(
            any_enforcer,
            "expected SupportEnforcer polygons from cube modifier volume"
        );

        // All returned entries must be tagged SupportEnforcer.
        for bucket in &result {
            for mvl in bucket {
                assert_eq!(
                    mvl.semantic,
                    PaintSemantic::SupportEnforcer,
                    "expected SupportEnforcer semantic"
                );
            }
        }
    }

    // ---- slice_modifier_volumes_skips_non_modifier_semantics ---------------

    #[test]
    fn slice_modifier_volumes_skips_non_modifier_semantics() {
        // Use subtype "material" — not SupportEnforcer/SupportBlocker, must be skipped.
        let mv_mesh = cube_mesh(1.0);
        let mesh = mesh_with_modifier("material", mv_mesh);
        let layer_zs = vec![0.1, 0.5, 0.9];
        let result = slice_modifier_volumes(&mesh, &layer_zs);

        assert_eq!(result.len(), 3);
        for bucket in &result {
            assert!(bucket.is_empty(), "material subtype must be skipped");
        }
    }

    // ---- point-in-polygon helpers -----------------------------------------

    #[test]
    fn expolygon_contains_point_basic_inside_outside() {
        let u = |mm: f64| -> i64 { (mm * 10_000.0).round() as i64 };
        let outer = Polygon {
            points: vec![
                Point2 {
                    x: u(0.0),
                    y: u(0.0),
                },
                Point2 {
                    x: u(2.0),
                    y: u(0.0),
                },
                Point2 {
                    x: u(2.0),
                    y: u(2.0),
                },
                Point2 {
                    x: u(0.0),
                    y: u(2.0),
                },
            ],
        };
        let exp = ExPolygon {
            contour: outer,
            holes: Vec::new(),
        };
        assert!(
            expolygon_contains_point(
                &exp,
                Point2 {
                    x: u(1.0),
                    y: u(1.0)
                }
            ),
            "centre must be inside"
        );
        assert!(
            !expolygon_contains_point(
                &exp,
                Point2 {
                    x: u(3.0),
                    y: u(3.0)
                }
            ),
            "far outside"
        );
    }

    #[test]
    fn expolygon_contains_point_excludes_hole() {
        let u = |mm: f64| -> i64 { (mm * 10_000.0).round() as i64 };
        let outer = Polygon {
            points: vec![
                Point2 {
                    x: u(0.0),
                    y: u(0.0),
                },
                Point2 {
                    x: u(4.0),
                    y: u(0.0),
                },
                Point2 {
                    x: u(4.0),
                    y: u(4.0),
                },
                Point2 {
                    x: u(0.0),
                    y: u(4.0),
                },
            ],
        };
        let hole = Polygon {
            points: vec![
                Point2 {
                    x: u(1.0),
                    y: u(1.0),
                },
                Point2 {
                    x: u(3.0),
                    y: u(1.0),
                },
                Point2 {
                    x: u(3.0),
                    y: u(3.0),
                },
                Point2 {
                    x: u(1.0),
                    y: u(3.0),
                },
            ],
        };
        let exp = ExPolygon {
            contour: outer,
            holes: vec![hole],
        };
        // In contour but inside hole → NOT inside expolygon.
        assert!(
            !expolygon_contains_point(
                &exp,
                Point2 {
                    x: u(2.0),
                    y: u(2.0)
                }
            ),
            "hole centre must be outside"
        );
        // In contour, outside hole → inside expolygon.
        assert!(
            expolygon_contains_point(
                &exp,
                Point2 {
                    x: u(0.2),
                    y: u(0.2)
                }
            ),
            "corner outside hole must be inside"
        );
    }
}
