//! Elementary mesh checks, independent of production mesh/geometry helpers.

use std::collections::BTreeMap;

use slicer_ir::{IndexedTriangleSet, Point3};

use super::DomainError;

fn coords(p: Point3) -> [f64; 3] {
    [f64::from(p.x), f64::from(p.y), f64::from(p.z)]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn input(message: &str) -> DomainError {
    DomainError::InvalidInput(message.into())
}

pub(super) fn validate_indexed_mesh(
    mesh: &IndexedTriangleSet,
    require_closed: bool,
) -> Result<(), DomainError> {
    if mesh
        .vertices
        .iter()
        .any(|p| !p.x.is_finite() || !p.y.is_finite() || !p.z.is_finite())
    {
        return Err(input("non-finite vertex coordinate"));
    }
    if !mesh.indices.len().is_multiple_of(3) {
        return Err(input("triangle stream is incomplete"));
    }
    let mut edges = BTreeMap::<(u32, u32), (usize, i64)>::new();
    for triangle in mesh.indices.chunks_exact(3) {
        let [a, b, c] = [triangle[0], triangle[1], triangle[2]];
        if [a, b, c]
            .into_iter()
            .any(|index| usize::try_from(index).map_or(true, |i| i >= mesh.vertices.len()))
        {
            return Err(input("index out of range"));
        }
        if a == b || b == c || c == a {
            return Err(input("repeated index in triangle"));
        }
        let [p, q, r] = [a, b, c].map(|i| coords(mesh.vertices[i as usize]));
        let ab = std::array::from_fn(|i| q[i] - p[i]);
        let ac = std::array::from_fn(|i| r[i] - p[i]);
        if cross(ab, ac) == [0.0; 3] {
            return Err(input("degenerate triangle"));
        }
        if require_closed {
            for (start, end) in [(a, b), (b, c), (c, a)] {
                let forward = start < end;
                let entry = edges.entry((start.min(end), start.max(end))).or_default();
                entry.0 += 1;
                entry.1 += if forward { 1 } else { -1 };
            }
        }
    }
    if require_closed
        && edges
            .values()
            .any(|&(count, direction)| count != 2 || direction != 0)
    {
        return Err(input("closed edge requires two opposite occurrences"));
    }
    Ok(())
}

fn finite(value: f64) -> Result<f64, DomainError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(DomainError::ArithmeticOverflow)
    }
}

pub(super) fn mesh_signed_volume_mm3(mesh: &IndexedTriangleSet) -> Result<f64, DomainError> {
    validate_indexed_mesh(mesh, false)?;
    let mut sum = 0.0;
    for triangle in mesh.indices.chunks_exact(3) {
        let [a, b, c] =
            [triangle[0], triangle[1], triangle[2]].map(|i| coords(mesh.vertices[i as usize]));
        let mut cross = [0.0; 3];
        for i in 0..3 {
            let j = (i + 1) % 3;
            let k = (i + 2) % 3;
            cross[i] = finite(finite(b[j] * c[k])? - finite(b[k] * c[j])?)?;
        }
        let mut dot = 0.0;
        for i in 0..3 {
            dot = finite(dot + finite(a[i] * cross[i])?)?;
        }
        sum = finite(sum + dot)?;
    }
    finite(sum / 6.0)
}
