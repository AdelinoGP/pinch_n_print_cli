//! Host-owned exact-Z geometry queries for support family planners.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use slicer_core::algos::mesh_cross_section::cross_section_at_z;
use slicer_ir::{mm_to_units, ExPolygon, MeshIR, ObjectMesh, PaintValue, Point2, Point3, Polygon};

/// Immutable result of one normalized support geometry query.
#[derive(Debug, Clone, PartialEq)]
pub struct ExactZSupportQuery {
    /// Requested physical Z in canonical repository units.
    pub z_units: i64,
    /// Model occupancy at the requested exact Z.
    pub occupancy: Vec<ExPolygon>,
    /// Geometry that blocks support routing at this Z.
    pub blockers: Vec<ExPolygon>,
    /// Model top surfaces and the build-plate termination surface.
    pub termination_surfaces: Vec<ExPolygon>,
    /// Conservative envelope before family-specific tightening.
    pub baseline_envelope: Vec<ExPolygon>,
}

/// Full immutable-cache identity: object, region, ordered source chain, and Z.
/// The chain is part of the identity even though occupancy remains object-wide;
/// an empty chain is the exact identity of a genuinely chain-less source,
/// never a wildcard.
type QueryKey = (String, u64, Vec<(String, PaintValue)>, i64);

/// Thread-safe, immutable-result cache for exact-Z support geometry.
pub struct ExactZQueryService {
    mesh: Arc<MeshIR>,
    cache: Mutex<HashMap<QueryKey, Arc<ExactZSupportQuery>>>,
}

impl ExactZQueryService {
    /// Create a service over host-owned mesh data.
    pub fn new(mesh: Arc<MeshIR>) -> Self {
        Self {
            mesh,
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Query one object and source region at an exact physical Z in millimeters.
    /// The region and ordered `variant_chain` are part of the cache identity
    /// even though mesh occupancy is currently object-wide, so two queries that
    /// differ only by chain (including chain order) never share a cached
    /// result; an empty slice is the exact identity of a chain-less source.
    pub fn query(
        &self,
        object_id: &str,
        region_id: u64,
        variant_chain: &[(String, PaintValue)],
        physical_z_mm: f32,
    ) -> Result<Arc<ExactZSupportQuery>, String> {
        if !physical_z_mm.is_finite() {
            return Err("exact-z-support-query: Z must be finite".into());
        }
        let z_units = mm_to_units(physical_z_mm);
        let key = (
            object_id.to_owned(),
            region_id,
            variant_chain.to_vec(),
            z_units,
        );
        if let Some(result) = self
            .cache
            .lock()
            .map_err(|_| "exact-z-support-query: cache poisoned")?
            .get(&key)
        {
            return Ok(Arc::clone(result));
        }

        let object = self
            .mesh
            .objects
            .iter()
            .find(|object| object.id == object_id)
            .ok_or_else(|| format!("exact-z-support-query: object '{object_id}' not found"))?;
        let world_mesh = world_mesh(object);
        let occupancy = cross_section_at_z(&world_mesh, physical_z_mm);
        let bbox = world_bounds(&world_mesh)
            .ok_or_else(|| format!("exact-z-support-query: object '{object_id}' has empty mesh"))?;
        let plate = rectangle(bbox.0, bbox.1, bbox.2, bbox.3);
        let top = cross_section_at_z(&world_mesh, bbox.5);
        let result = Arc::new(ExactZSupportQuery {
            z_units,
            blockers: occupancy.clone(),
            occupancy,
            termination_surfaces: top
                .into_iter()
                .chain(std::iter::once(plate.clone()))
                .collect(),
            baseline_envelope: vec![plate],
        });
        self.cache
            .lock()
            .map_err(|_| "exact-z-support-query: cache poisoned")?
            .insert(key, Arc::clone(&result));
        Ok(result)
    }
}

fn world_mesh(object: &ObjectMesh) -> slicer_ir::IndexedTriangleSet {
    slicer_ir::IndexedTriangleSet {
        vertices: object
            .mesh
            .vertices
            .iter()
            .map(|p| transform(&object.transform.matrix, p))
            .collect(),
        indices: object.mesh.indices.clone(),
    }
}

fn transform(matrix: &[f64; 16], p: &Point3) -> Point3 {
    Point3 {
        x: (matrix[0] * p.x as f64 + matrix[4] * p.y as f64 + matrix[8] * p.z as f64 + matrix[12])
            as f32,
        y: (matrix[1] * p.x as f64 + matrix[5] * p.y as f64 + matrix[9] * p.z as f64 + matrix[13])
            as f32,
        z: (matrix[2] * p.x as f64 + matrix[6] * p.y as f64 + matrix[10] * p.z as f64 + matrix[14])
            as f32,
    }
}

fn world_bounds(mesh: &slicer_ir::IndexedTriangleSet) -> Option<(i64, i64, i64, i64, f32, f32)> {
    let first = mesh.vertices.first()?;
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (first.x, first.x, first.y, first.y);
    let (mut min_z, mut max_z) = (first.z, first.z);
    for p in &mesh.vertices[1..] {
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
        min_z = min_z.min(p.z);
        max_z = max_z.max(p.z);
    }
    Some((
        mm_to_units(min_x),
        mm_to_units(max_x),
        mm_to_units(min_y),
        mm_to_units(max_y),
        min_z,
        max_z,
    ))
}

fn rectangle(min_x: i64, max_x: i64, min_y: i64, max_y: i64) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: min_x, y: min_y },
                Point2 { x: max_x, y: min_y },
                Point2 { x: max_x, y: max_y },
                Point2 { x: min_x, y: max_y },
            ],
        },
        holes: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::{ExactZQueryService, QueryKey};
    use slicer_ir::{IndexedTriangleSet, MeshIR, ObjectMesh, PaintValue, Point3, Transform3d};
    use std::collections::BTreeSet;
    use std::sync::Arc;

    /// One nonempty physical object (a 10 mm cube) shared by every query in
    /// `query_cache_identity_includes_ordered_source_chain`; only the chain
    /// varies across calls.
    fn cube_mesh() -> MeshIR {
        MeshIR {
            objects: vec![ObjectMesh {
                id: "object-a".into(),
                mesh: IndexedTriangleSet {
                    vertices: vec![
                        Point3 {
                            x: 0.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        Point3 {
                            x: 10.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        Point3 {
                            x: 10.0,
                            y: 10.0,
                            z: 0.0,
                        },
                        Point3 {
                            x: 0.0,
                            y: 10.0,
                            z: 0.0,
                        },
                        Point3 {
                            x: 0.0,
                            y: 0.0,
                            z: 10.0,
                        },
                        Point3 {
                            x: 10.0,
                            y: 0.0,
                            z: 10.0,
                        },
                        Point3 {
                            x: 10.0,
                            y: 10.0,
                            z: 10.0,
                        },
                        Point3 {
                            x: 0.0,
                            y: 10.0,
                            z: 10.0,
                        },
                    ],
                    indices: vec![
                        0, 1, 2, 0, 2, 3, 4, 6, 5, 4, 7, 6, 0, 4, 5, 0, 5, 1, 1, 5, 6, 1, 6, 2, 2,
                        6, 7, 2, 7, 3, 3, 7, 4, 3, 4, 0,
                    ],
                },
                transform: Transform3d {
                    matrix: [
                        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0,
                        1.0,
                    ],
                },
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn query_cache_identity_includes_ordered_source_chain() {
        let service = ExactZQueryService::new(Arc::new(cube_mesh()));
        let object_id = "object-a";
        let region_id = 7u64;
        // Intermediate Z through the 10 mm cube: nonempty physical occupancy,
        // shared by every call below.
        let physical_z_mm = 4.321f32;

        let empty: Vec<(String, PaintValue)> = Vec::new();
        let chain_a: Vec<(String, PaintValue)> = vec![
            ("wall".to_string(), PaintValue::ToolIndex(1)),
            ("top".to_string(), PaintValue::Flag(true)),
        ];
        let reversed: Vec<(String, PaintValue)> = chain_a.iter().cloned().rev().collect();
        let sibling: Vec<(String, PaintValue)> = vec![
            ("wall".to_string(), PaintValue::ToolIndex(2)),
            ("top".to_string(), PaintValue::Flag(true)),
        ];

        let empty_result = service
            .query(object_id, region_id, &empty, physical_z_mm)
            .expect("empty-chain query");
        let chain_a_result = service
            .query(object_id, region_id, &chain_a, physical_z_mm)
            .expect("chain query");
        let chain_a_cached = service
            .query(object_id, region_id, &chain_a, physical_z_mm)
            .expect("cached chain query");
        let reversed_result = service
            .query(object_id, region_id, &reversed, physical_z_mm)
            .expect("reversed-chain query");
        let sibling_result = service
            .query(object_id, region_id, &sibling, physical_z_mm)
            .expect("sibling-chain query");

        // Same full key (object, region, ordered chain, Z) reuses the immutable
        // cached Arc; differing chains never do.
        assert!(
            Arc::ptr_eq(&chain_a_result, &chain_a_cached),
            "same full key must reuse one cached Arc"
        );
        assert!(
            !Arc::ptr_eq(&chain_a_result, &reversed_result),
            "reordered chain must not reuse another chain's Arc"
        );
        assert!(
            !Arc::ptr_eq(&chain_a_result, &sibling_result),
            "sibling chain must not reuse another chain's Arc"
        );
        assert!(
            !Arc::ptr_eq(&empty_result, &chain_a_result),
            "empty chain must not reuse a nonempty chain's Arc"
        );

        // Actual cached entries: one per distinct full identity. The expected
        // keys are built independently from the literal unit conversion
        // (4.321 mm = 43_210 canonical units).
        let cached: BTreeSet<QueryKey> = service
            .cache
            .lock()
            .expect("exact-z cache lock")
            .keys()
            .cloned()
            .collect();
        let expected: BTreeSet<QueryKey> = [
            (object_id.to_string(), region_id, empty.clone(), 43_210i64),
            (object_id.to_string(), region_id, chain_a.clone(), 43_210i64),
            (
                object_id.to_string(),
                region_id,
                reversed.clone(),
                43_210i64,
            ),
            (object_id.to_string(), region_id, sibling.clone(), 43_210i64),
        ]
        .into_iter()
        .collect();
        assert_eq!(
            cached.len(),
            4,
            "each distinct full source identity has its own cache entry"
        );
        assert_eq!(
            cached, expected,
            "cached keys are exactly the independently expected full identities"
        );

        // The query geometry stays object-wide: every result is the identical
        // physical result, regardless of which source chain asked.
        assert_eq!(empty_result.z_units, 43_210);
        assert!(
            !empty_result.occupancy.is_empty(),
            "the cube intersects the intermediate Z with nonempty occupancy"
        );
        for (label, result) in [
            ("nonempty chain", &chain_a_result),
            ("reversed chain", &reversed_result),
            ("sibling chain", &sibling_result),
        ] {
            assert_eq!(result.z_units, empty_result.z_units, "{label}: z_units");
            assert_eq!(
                result.occupancy, empty_result.occupancy,
                "{label}: occupancy remains object-wide"
            );
            assert_eq!(
                result.blockers, empty_result.blockers,
                "{label}: blockers remain object-wide"
            );
            assert_eq!(
                result.termination_surfaces, empty_result.termination_surfaces,
                "{label}: termination surfaces remain object-wide"
            );
            assert_eq!(
                result.baseline_envelope, empty_result.baseline_envelope,
                "{label}: baseline envelope remains object-wide"
            );
        }
    }
}
