//! Host-owned exact-Z geometry queries for support family planners.

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use slicer_core::algos::mesh_cross_section::cross_section_at_z;
use slicer_core::slice_mesh_ex;
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

/// Geometry terms that depend on the object alone, never on the requested Z.
struct ObjectGeometry {
    /// World-space mesh; also the source of every per-Z cross-section.
    world_mesh: slicer_ir::IndexedTriangleSet,
    /// Build-plate termination rectangle derived from the world bounds.
    plate: ExPolygon,
    /// Cross-section at the object's constant maximum Z.
    top: Vec<ExPolygon>,
    /// Cross-sections already computed, keyed by canonical Z units.
    ///
    /// The mesh occupancy is object-wide (the query's `region_id` is reserved
    /// for future clipping, not part of this computation), so one entry serves
    /// every region queried at the same Z. [`ExactZQueryService::prefetch_cross_sections`]
    /// fills this in a single batched mesh walk.
    cross_sections: Mutex<HashMap<i64, Arc<Vec<ExPolygon>>>>,
}

/// Thread-safe, immutable-result cache for exact-Z support geometry.
pub struct ExactZQueryService {
    mesh: Arc<MeshIR>,
    cache: Mutex<HashMap<QueryKey, Arc<ExactZSupportQuery>>>,
    object_geometry: Mutex<HashMap<String, Arc<ObjectGeometry>>>,
    geometry_builds: AtomicUsize,
}

impl ExactZQueryService {
    /// Create a service over host-owned mesh data.
    pub fn new(mesh: Arc<MeshIR>) -> Self {
        Self {
            mesh,
            cache: Mutex::new(HashMap::new()),
            object_geometry: Mutex::new(HashMap::new()),
            geometry_builds: AtomicUsize::new(0),
        }
    }

    /// Number of per-object geometry entries this service has built.
    ///
    /// Diagnostic surface for the memo's identity contract: the z-independent
    /// terms (world mesh, bounds, plate rectangle, top-Z cross-section) are
    /// built once per object, however many Z values that object is queried at.
    pub fn geometry_build_count(&self) -> usize {
        self.geometry_builds.load(Ordering::Relaxed)
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

        let geometry = self.object_geometry(object_id)?;
        let memoized = geometry
            .cross_sections
            .lock()
            .map_err(|_| "exact-z-support-query: cache poisoned")?
            .get(&z_units)
            .cloned();
        let occupancy = match memoized {
            Some(section) => (*section).clone(),
            None => {
                let section = cross_section_at_z(&geometry.world_mesh, physical_z_mm);
                geometry
                    .cross_sections
                    .lock()
                    .map_err(|_| "exact-z-support-query: cache poisoned")?
                    .entry(z_units)
                    .or_insert_with(|| Arc::new(section.clone()));
                section
            }
        };
        let result = Arc::new(ExactZSupportQuery {
            z_units,
            blockers: occupancy.clone(),
            occupancy,
            termination_surfaces: geometry
                .top
                .iter()
                .cloned()
                .chain(std::iter::once(geometry.plate.clone()))
                .collect(),
            baseline_envelope: vec![geometry.plate.clone()],
        });
        self.cache
            .lock()
            .map_err(|_| "exact-z-support-query: cache poisoned")?
            .insert(key, Arc::clone(&result));
        Ok(result)
    }

    /// Precompute the cross-sections at a set of physical Z values for one
    /// object in a single batched mesh walk.
    ///
    /// `slice_mesh_ex` assigns each triangle to the planes it crosses and
    /// builds every plane independently, so the batched result for a plane is
    /// identical to the single-plane call the per-query path makes; t95 measured
    /// 431 of 431 planes equal, polygon for polygon, on the base supports-on
    /// cells.
    ///
    /// Failures are deliberately not surfaced: an unknown object or a poisoned
    /// lock only means the prefetch filled nothing, and the per-query path then
    /// degrades that entry exactly as it does today
    /// (`body rejected: exact-Z query unavailable`).
    pub fn prefetch_cross_sections(&self, object_id: &str, zs_mm: &[f32]) {
        let Ok(geometry) = self.object_geometry(object_id) else {
            return;
        };
        let mut pending: Vec<(f32, i64)> = Vec::new();
        {
            let Ok(memo) = geometry.cross_sections.lock() else {
                return;
            };
            let mut seen = std::collections::HashSet::new();
            for &z in zs_mm {
                if !z.is_finite() {
                    return;
                }
                let z_units = mm_to_units(z);
                if memo.contains_key(&z_units) || !seen.insert(z_units) {
                    continue;
                }
                pending.push((z, z_units));
            }
        }
        if pending.is_empty() {
            return;
        }
        // `slice_mesh_ex` binary-searches the plane list, so it must be
        // ascending; the returned layers follow the same order.
        pending.sort_by(|left, right| {
            left.0
                .partial_cmp(&right.0)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let planes: Vec<f32> = pending.iter().map(|(z, _)| *z).collect();
        let sections = slice_mesh_ex(&geometry.world_mesh, &planes);
        let Ok(mut memo) = geometry.cross_sections.lock() else {
            return;
        };
        for ((_, z_units), section) in pending.into_iter().zip(sections) {
            memo.entry(z_units).or_insert_with(|| Arc::new(section));
        }
    }

    /// Resolve (building once per object) the terms that do not depend on Z.
    ///
    /// The lookup, the world-space transform, the bounds-derived plate and the
    /// constant top-Z cross-section all read the object alone, so a cache hit
    /// here returns values identical to every earlier query on that object.
    /// Concurrent first queries may build the same terms twice; only the first
    /// inserted entry is retained, and the terms are equal either way.
    fn object_geometry(&self, object_id: &str) -> Result<Arc<ObjectGeometry>, String> {
        if let Some(geometry) = self
            .object_geometry
            .lock()
            .map_err(|_| "exact-z-support-query: cache poisoned")?
            .get(object_id)
        {
            return Ok(Arc::clone(geometry));
        }
        let object = self
            .mesh
            .objects
            .iter()
            .find(|object| object.id == object_id)
            .ok_or_else(|| format!("exact-z-support-query: object '{object_id}' not found"))?;
        let world_mesh = world_mesh(object);
        let bbox = world_bounds(&world_mesh)
            .ok_or_else(|| format!("exact-z-support-query: object '{object_id}' has empty mesh"))?;
        let geometry = Arc::new(ObjectGeometry {
            plate: rectangle(bbox.0, bbox.1, bbox.2, bbox.3),
            top: cross_section_at_z(&world_mesh, bbox.5),
            world_mesh,
            cross_sections: Mutex::new(HashMap::new()),
        });
        let mut guard = self
            .object_geometry
            .lock()
            .map_err(|_| "exact-z-support-query: cache poisoned")?;
        match guard.entry(object_id.to_owned()) {
            Entry::Occupied(entry) => Ok(Arc::clone(entry.get())),
            Entry::Vacant(entry) => {
                self.geometry_builds.fetch_add(1, Ordering::Relaxed);
                Ok(Arc::clone(entry.insert(geometry)))
            }
        }
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
