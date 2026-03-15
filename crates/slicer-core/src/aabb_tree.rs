//! Minimal TASK-014 mesh-query API scaffolding.

use slicer_ir::{BoundingBox3, IndexedTriangleSet, Point3};

/// Acceleration structure for mesh raycasts, bounds queries, and closest-point lookups.
#[derive(Debug, Clone)]
pub struct AabbTree {
    mesh: IndexedTriangleSet,
}

impl AabbTree {
    /// Creates a query tree over the provided indexed triangle mesh.
    pub fn new(mesh: IndexedTriangleSet) -> Self {
        Self { mesh }
    }

    /// Returns whether the tree contains no queryable triangles.
    pub fn is_empty(&self) -> bool {
        let _ = &self.mesh;
        todo!("TASK-014: implement AABB tree emptiness checks")
    }

    /// Returns the mesh-space axis-aligned bounding box, if any triangles exist.
    pub fn bounds(&self) -> Option<BoundingBox3> {
        let _ = &self.mesh;
        todo!("TASK-014: implement AABB tree bounds queries")
    }

    /// Returns the nearest hit along a ray, if the mesh is intersected.
    pub fn raycast_first_hit(&self, origin: Point3, dir: Point3) -> Option<RayHit> {
        let _ = (&self.mesh, origin, dir);
        todo!("TASK-014: implement first-hit raycasts")
    }

    /// Returns every hit along a ray, sorted by increasing distance.
    pub fn raycast_all_hits(&self, origin: Point3, dir: Point3) -> Vec<RayHit> {
        let _ = (&self.mesh, origin, dir);
        todo!("TASK-014: implement all-hit raycasts")
    }

    /// Returns the closest point on the mesh to the query point, if any triangles exist.
    pub fn closest_point(&self, point: Point3) -> Option<ClosestPointHit> {
        let _ = (&self.mesh, point);
        todo!("TASK-014: implement closest-point queries")
    }
}

/// A ray intersection result in mesh space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RayHit {
    /// Distance from the ray origin in millimeters along the supplied direction.
    pub distance: f32,
    /// Intersection point in mesh coordinates.
    pub point: Point3,
}

/// The nearest point on the mesh to a query point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClosestPointHit {
    /// Closest point in mesh coordinates.
    pub point: Point3,
    /// Squared distance from the query point in square millimeters.
    pub squared_distance: f32,
}
