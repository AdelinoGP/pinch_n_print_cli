//! Host-owned exact-Z geometry queries for support family planners.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use slicer_core::algos::mesh_cross_section::cross_section_at_z;
use slicer_ir::{mm_to_units, ExPolygon, MeshIR, ObjectMesh, Point2, Point3, Polygon};

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

type QueryKey = (String, u64, i64);

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

    /// Query one object and region at an exact physical Z in millimeters.
    /// The region is part of the cache identity even though mesh occupancy is
    /// currently object-wide; future region clipping can therefore be added
    /// without changing the service contract.
    pub fn query(
        &self,
        object_id: &str,
        region_id: u64,
        physical_z_mm: f32,
    ) -> Result<Arc<ExactZSupportQuery>, String> {
        if !physical_z_mm.is_finite() {
            return Err("exact-z-support-query: Z must be finite".into());
        }
        let z_units = mm_to_units(physical_z_mm);
        let key = (object_id.to_owned(), region_id, z_units);
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
