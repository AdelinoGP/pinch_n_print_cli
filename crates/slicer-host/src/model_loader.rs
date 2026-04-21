//! File format loaders for STL, OBJ, and 3MF model files (TASK-076).
//!
//! Detects file format by extension and parses geometry into [`MeshIR`].
//! Supported formats:
//! - STL (binary and ASCII)
//! - OBJ (Wavefront)
//! - 3MF (3D Manufacturing Format — ZIP-based)

use std::collections::HashMap;
use std::fmt;
use std::io::{BufReader, Read, Seek};
use std::path::Path;

use slicer_ir::{
    BoundingBox3, IndexedTriangleSet, MeshIR, ObjectConfig, ObjectMesh, Point3, SemVer, Transform3d,
};

/// Detected model file format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelFormat {
    /// STL (STereoLithography) — binary or ASCII.
    Stl,
    /// Wavefront OBJ.
    Obj,
    /// 3D Manufacturing Format (3MF).
    ThreeMf,
}

/// Errors from model loading.
#[derive(Debug)]
pub enum ModelLoadError {
    /// I/O error reading the file.
    Io(std::io::Error),
    /// File extension not recognized.
    UnsupportedFormat(String),
    /// STL parse error.
    StlParse(String),
    /// OBJ parse error.
    ObjParse(String),
    /// 3MF parse error.
    ThreeMfParse(String),
    /// NON_UNIFORM_SCALE_UNSUPPORTED — object transform has non-uniform scale
    /// (scale_x ≠ scale_y or scale_y ≠ scale_z).  All three scale axes must be
    /// equal; non-uniform scale is not supported by the slicer pipeline.
    NonUniformScaleUnsupported {
        /// Scale factor along X axis (magnitude of transform column 0 xyz).
        scale_x: f64,
        /// Scale factor along Y axis (magnitude of transform column 1 xyz).
        scale_y: f64,
        /// Scale factor along Z axis (magnitude of transform column 2 xyz).
        scale_z: f64,
    },
    /// WORLD_Z_BELOW_FLOOR — one or more object vertices map to a world-space Z
    /// below the print volume floor (0.0 mm).  Translate the object upward so
    /// its lowest point is at or above Z = 0.
    WorldZBelowFloor {
        /// The minimum world-space Z found on the object (negative).
        z_min: f32,
    },
}

impl fmt::Display for ModelLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::UnsupportedFormat(ext) => write!(f, "unsupported format: {ext}"),
            Self::StlParse(msg) => write!(f, "STL parse error: {msg}"),
            Self::ObjParse(msg) => write!(f, "OBJ parse error: {msg}"),
            Self::ThreeMfParse(msg) => write!(f, "3MF parse error: {msg}"),
            Self::NonUniformScaleUnsupported { scale_x, scale_y, scale_z } => write!(
                f,
                "NON_UNIFORM_SCALE_UNSUPPORTED: scale ({scale_x:.6}, {scale_y:.6}, {scale_z:.6}) is non-uniform"
            ),
            Self::WorldZBelowFloor { z_min } => write!(
                f,
                "WORLD_Z_BELOW_FLOOR: object world-space Z minimum {z_min} mm is below print floor 0.0 mm"
            ),
        }
    }
}

impl std::error::Error for ModelLoadError {}

impl From<std::io::Error> for ModelLoadError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// Detect model format from file path extension (case-insensitive).
///
/// # Errors
///
/// Returns [`ModelLoadError::UnsupportedFormat`] if the extension is not recognized.
pub fn detect_format(path: impl AsRef<Path>) -> Result<ModelFormat, ModelLoadError> {
    let path = path.as_ref();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "stl" => Ok(ModelFormat::Stl),
        "obj" => Ok(ModelFormat::Obj),
        "3mf" => Ok(ModelFormat::ThreeMf),
        other => Err(ModelLoadError::UnsupportedFormat(other.to_string())),
    }
}

/// Load a model file and produce a [`MeshIR`].
///
/// Detects format by extension, parses geometry, deduplicates vertices,
/// computes a bounding box, and returns a single-object MeshIR.
///
/// # Errors
///
/// Returns [`ModelLoadError`] on I/O failure, unsupported format, or parse error.
pub fn load_model(path: &Path) -> Result<MeshIR, ModelLoadError> {
    let format = detect_format(path)?;
    let file = std::fs::File::open(path)?;
    let mut reader = BufReader::new(file);

    let its = match format {
        ModelFormat::Stl => load_stl(&mut reader)?,
        ModelFormat::Obj => load_obj(path)?,
        ModelFormat::ThreeMf => load_3mf(&mut reader)?,
    };

    let build_volume = compute_bounding_box(&its);

    let transform = identity_transform();
    let world_z_extent = {
        // Apply identity transform (zero-matrix fallback) to extract Z range.
        // Safe because transform is identity — z_min/z_max of mesh == world z range.
        let identity = transform.matrix.iter().all(|v| *v == 0.0);
        if identity {
            compute_z_extent_from_mesh(&its)
        } else {
            object_world_z_extent_from_mesh_and_transform(&its, &transform)
        }
    };
    let object = ObjectMesh {
        id: uuid::Uuid::new_v4().to_string(),
        mesh: its,
        transform,
        config: ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: Vec::new(),
        paint_data: None,
        world_z_extent,
    };

    Ok(MeshIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        objects: vec![object],
        build_volume,
    })
}

// ---------------------------------------------------------------------------
// STL loader
// ---------------------------------------------------------------------------

/// Load STL (binary or ASCII) and return deduplicated IndexedTriangleSet.
fn load_stl(reader: &mut (impl Read + Seek)) -> Result<IndexedTriangleSet, ModelLoadError> {
    let stl = stl_io::read_stl(reader).map_err(|e| ModelLoadError::StlParse(e.to_string()))?;

    // stl_io gives us Vec<Vertex> and Vec<Triangle> where Triangle has
    // normal + vertices[3] as indices into the vertex list.
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut vertex_map: HashMap<[u32; 3], u32> = HashMap::new();

    for v in &stl.vertices {
        let key = [v[0].to_bits(), v[1].to_bits(), v[2].to_bits()];
        let idx = vertices.len() as u32;
        vertex_map.entry(key).or_insert_with(|| {
            vertices.push(Point3 {
                x: v[0],
                y: v[1],
                z: v[2],
            });
            idx
        });
    }

    for tri in &stl.faces {
        for &vi in &tri.vertices {
            let v = &stl.vertices[vi];
            let key = [v[0].to_bits(), v[1].to_bits(), v[2].to_bits()];
            indices.push(vertex_map[&key]);
        }
    }

    Ok(IndexedTriangleSet { vertices, indices })
}

// ---------------------------------------------------------------------------
// OBJ loader
// ---------------------------------------------------------------------------

/// Load OBJ and return IndexedTriangleSet.
fn load_obj(path: &Path) -> Result<IndexedTriangleSet, ModelLoadError> {
    let (models, _materials) = tobj::load_obj(path, &tobj::GPU_LOAD_OPTIONS)
        .map_err(|e| ModelLoadError::ObjParse(e.to_string()))?;

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Merge all meshes into a single ITS (like OrcaSlicer does)
    for model in &models {
        let m = &model.mesh;
        let vertex_offset = vertices.len() as u32;

        // tobj stores positions as flat [x0, y0, z0, x1, y1, z1, ...]
        for chunk in m.positions.chunks_exact(3) {
            vertices.push(Point3 {
                x: chunk[0],
                y: chunk[1],
                z: chunk[2],
            });
        }

        for &idx in &m.indices {
            indices.push(vertex_offset + idx);
        }
    }

    if vertices.is_empty() {
        return Err(ModelLoadError::ObjParse("no geometry found".into()));
    }

    Ok(IndexedTriangleSet { vertices, indices })
}

// ---------------------------------------------------------------------------
// 3MF loader
// ---------------------------------------------------------------------------

/// Load 3MF and return IndexedTriangleSet from the first object.
fn load_3mf(reader: &mut (impl Read + Seek)) -> Result<IndexedTriangleSet, ModelLoadError> {
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| ModelLoadError::ThreeMfParse(e.to_string()))?;

    // Find the 3D model file — standard path is "3D/3dmodel.model"
    let model_path = find_model_path(&archive)?;
    let mut model_file = archive
        .by_name(&model_path)
        .map_err(|e| ModelLoadError::ThreeMfParse(e.to_string()))?;

    let mut xml_bytes = Vec::new();
    model_file
        .read_to_end(&mut xml_bytes)
        .map_err(|e| ModelLoadError::ThreeMfParse(e.to_string()))?;

    parse_3mf_model_xml(&xml_bytes)
}

/// Find the 3D model XML path inside a 3MF ZIP archive.
fn find_model_path<R: Read + Seek>(archive: &zip::ZipArchive<R>) -> Result<String, ModelLoadError> {
    for i in 0..archive.len() {
        if let Some(name) = archive.name_for_index(i) {
            let lower = name.to_lowercase();
            if lower.ends_with("3dmodel.model") {
                return Ok(name.to_string());
            }
        }
    }
    Err(ModelLoadError::ThreeMfParse(
        "no 3dmodel.model found in archive".into(),
    ))
}

/// Parse 3MF model XML into IndexedTriangleSet.
fn parse_3mf_model_xml(xml_bytes: &[u8]) -> Result<IndexedTriangleSet, ModelLoadError> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_reader(xml_bytes);
    reader.config_mut().trim_text(true);

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut buf = Vec::new();
    let mut in_mesh = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name_bytes = e.name().as_ref().to_vec();
                let local = local_name(&name_bytes);
                match local {
                    b"mesh" => in_mesh = true,
                    b"vertex" if in_mesh => {
                        let (mut x, mut y, mut z) = (0.0f32, 0.0f32, 0.0f32);
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"x" => {
                                    x = parse_f32(&attr.value)?;
                                }
                                b"y" => {
                                    y = parse_f32(&attr.value)?;
                                }
                                b"z" => {
                                    z = parse_f32(&attr.value)?;
                                }
                                _ => {}
                            }
                        }
                        vertices.push(Point3 { x, y, z });
                    }
                    b"triangle" if in_mesh => {
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"v1" => indices.push(parse_u32(&attr.value)?),
                                b"v2" => indices.push(parse_u32(&attr.value)?),
                                b"v3" => indices.push(parse_u32(&attr.value)?),
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let name_bytes = e.name().as_ref().to_vec();
                if local_name(&name_bytes) == b"mesh" {
                    in_mesh = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(ModelLoadError::ThreeMfParse(format!(
                    "XML parse error: {e}"
                )));
            }
            _ => {}
        }
        buf.clear();
    }

    if vertices.is_empty() {
        return Err(ModelLoadError::ThreeMfParse("no geometry found".into()));
    }

    Ok(IndexedTriangleSet { vertices, indices })
}

/// Extract local name from a possibly-namespaced XML tag.
fn local_name(name: &[u8]) -> &[u8] {
    match name.iter().rposition(|&b| b == b':') {
        Some(pos) => &name[pos + 1..],
        None => name,
    }
}

/// Parse a UTF-8 byte slice as f32.
fn parse_f32(bytes: &[u8]) -> Result<f32, ModelLoadError> {
    std::str::from_utf8(bytes)
        .map_err(|e| ModelLoadError::ThreeMfParse(e.to_string()))?
        .parse::<f32>()
        .map_err(|e| ModelLoadError::ThreeMfParse(e.to_string()))
}

/// Parse a UTF-8 byte slice as u32.
fn parse_u32(bytes: &[u8]) -> Result<u32, ModelLoadError> {
    std::str::from_utf8(bytes)
        .map_err(|e| ModelLoadError::ThreeMfParse(e.to_string()))?
        .parse::<u32>()
        .map_err(|e| ModelLoadError::ThreeMfParse(e.to_string()))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Compute the axis-aligned bounding box of an IndexedTriangleSet.
fn compute_bounding_box(its: &IndexedTriangleSet) -> BoundingBox3 {
    if its.vertices.is_empty() {
        return BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        };
    }

    let mut min = Point3 {
        x: f32::INFINITY,
        y: f32::INFINITY,
        z: f32::INFINITY,
    };
    let mut max = Point3 {
        x: f32::NEG_INFINITY,
        y: f32::NEG_INFINITY,
        z: f32::NEG_INFINITY,
    };

    for v in &its.vertices {
        min.x = min.x.min(v.x);
        min.y = min.y.min(v.y);
        min.z = min.z.min(v.z);
        max.x = max.x.max(v.x);
        max.y = max.y.max(v.y);
        max.z = max.z.max(v.z);
    }

    BoundingBox3 { min, max }
}

/// Create a 4x4 identity transform.
fn identity_transform() -> Transform3d {
    let mut matrix = [0.0f64; 16];
    matrix[0] = 1.0;
    matrix[5] = 1.0;
    matrix[10] = 1.0;
    matrix[15] = 1.0;
    Transform3d { matrix }
}

/// Compute the Z extent `(z_min, z_max)` of a mesh assuming an identity
/// transform — i.e. just the raw vertex Z coordinates.
fn compute_z_extent_from_mesh(mesh: &IndexedTriangleSet) -> Option<(f32, f32)> {
    let mut z_min = f32::INFINITY;
    let mut z_max = f32::NEG_INFINITY;
    for v in &mesh.vertices {
        if v.z < z_min {
            z_min = v.z;
        }
        if v.z > z_max {
            z_max = v.z;
        }
    }
    if z_min.is_finite() && z_max.is_finite() && z_max > z_min {
        Some((z_min, z_max))
    } else {
        None
    }
}

/// Compute the world-space Z extent of a mesh given an explicit transform.
fn object_world_z_extent_from_mesh_and_transform(
    mesh: &IndexedTriangleSet,
    transform: &Transform3d,
) -> Option<(f32, f32)> {
    let m = &transform.matrix;
    let identity = m.iter().all(|v| *v == 0.0);
    let mut z_min = f32::INFINITY;
    let mut z_max = f32::NEG_INFINITY;
    for v in &mesh.vertices {
        let z = if identity {
            v.z
        } else {
            let x = v.x as f64;
            let y = v.y as f64;
            let z_val = v.z as f64;
            (m[2] * x + m[6] * y + m[10] * z_val + m[14]) as f32
        };
        if z < z_min {
            z_min = z;
        }
        if z > z_max {
            z_max = z;
        }
    }
    if z_min.is_finite() && z_max.is_finite() && z_max > z_min {
        Some((z_min, z_max))
    } else {
        None
    }
}

/// Validate that an [`ObjectMesh`] transform does not have non-uniform scale.
///
/// Extracts the column-vector magnitudes (scale factors) from the upper-left 3×3
/// of the 4×4 column-major transform matrix.  If any two scale axes differ by
/// more than `1e-6`, returns [`ModelLoadError::NonUniformScaleUnsupported`].
///
/// A zero matrix is treated as identity (uniform scale 1.0) to stay consistent
/// with the zero-matrix convention used in [`object_world_z_extent`].
///
/// # Errors
///
/// Returns `Err(NonUniformScaleUnsupported { … })` when the extracted scale
/// factors are not all equal within tolerance.
pub fn validate_non_uniform_scale(object: &ObjectMesh) -> Result<(), ModelLoadError> {
    let m = &object.transform.matrix;
    // Identity shortcut — all-zero matrix is treated as identity.
    if m.iter().all(|v| *v == 0.0) {
        return Ok(()); // identity → uniform scale 1.0
    }
    // Column-major layout: column k starts at index k*4.
    // Scale factor = Euclidean length of the 3-element (xyz) part of each column.
    let scale_x = (m[0] * m[0] + m[1] * m[1] + m[2] * m[2]).sqrt();
    let scale_y = (m[4] * m[4] + m[5] * m[5] + m[6] * m[6]).sqrt();
    let scale_z = (m[8] * m[8] + m[9] * m[9] + m[10] * m[10]).sqrt();
    const TOLERANCE: f64 = 1e-6;
    if (scale_x - scale_y).abs() > TOLERANCE || (scale_y - scale_z).abs() > TOLERANCE {
        return Err(ModelLoadError::NonUniformScaleUnsupported {
            scale_x,
            scale_y,
            scale_z,
        });
    }
    Ok(())
}

/// Validate that the world-space Z minimum of an [`ObjectMesh`] is at or above
/// the print volume floor (0.0 mm).
///
/// Uses [`object_world_z_extent`] to compute the world-space Z range.  If the
/// minimum Z is negative, returns [`ModelLoadError::WorldZBelowFloor`].
///
/// Objects with no geometry (empty mesh) or a degenerate extent (single vertex)
/// are treated as valid — they will be caught by later validation stages.
///
/// # Errors
///
/// Returns `Err(WorldZBelowFloor { z_min })` when the object extends below Z = 0.
pub fn validate_world_z_floor(object: &ObjectMesh) -> Result<(), ModelLoadError> {
    if let Some((z_min, _z_max)) = object_world_z_extent(object) {
        if z_min < 0.0 {
            return Err(ModelLoadError::WorldZBelowFloor { z_min });
        }
    }
    Ok(())
}

/// Compute the world-space Z extent `(z_min, z_max)` of an [`ObjectMesh`] by
/// applying its `transform` to each vertex and reducing.
///
/// Returns `None` when the mesh has no vertices or when the resulting extent
/// is non-finite or degenerate (`z_max <= z_min`).
///
/// A zero matrix is treated as identity to stay robust against fixtures that
/// leave `Transform3d::matrix` unset — the same convention used elsewhere in
/// the host when applying transforms (see `mesh_analysis::apply_transform`).
#[must_use]
pub fn object_world_z_extent(object: &ObjectMesh) -> Option<(f32, f32)> {
    let m = &object.transform.matrix;
    let identity = m.iter().all(|v| *v == 0.0);
    let mut z_min = f32::INFINITY;
    let mut z_max = f32::NEG_INFINITY;
    for v in &object.mesh.vertices {
        let z = if identity {
            v.z
        } else {
            let x = v.x as f64;
            let y = v.y as f64;
            let z = v.z as f64;
            (m[2] * x + m[6] * y + m[10] * z + m[14]) as f32
        };
        if z < z_min {
            z_min = z;
        }
        if z > z_max {
            z_max = z;
        }
    }
    if z_min.is_finite() && z_max.is_finite() && z_max > z_min {
        Some((z_min, z_max))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_transform_diagonal() {
        let t = identity_transform();
        assert_eq!(t.matrix[0], 1.0);
        assert_eq!(t.matrix[5], 1.0);
        assert_eq!(t.matrix[10], 1.0);
        assert_eq!(t.matrix[15], 1.0);
    }

    #[test]
    fn bounding_box_empty_mesh() {
        let its = IndexedTriangleSet {
            vertices: vec![],
            indices: vec![],
        };
        let bb = compute_bounding_box(&its);
        assert_eq!(bb.min.x, 0.0);
        assert_eq!(bb.max.x, 0.0);
    }

    fn make_object(
        id: &str,
        vertices: Vec<Point3>,
        transform: Transform3d,
    ) -> ObjectMesh {
        let mesh = IndexedTriangleSet {
            vertices,
            indices: vec![],
        };
        let world_z_extent = object_world_z_extent_from_mesh_and_transform(&mesh, &transform);
        ObjectMesh {
            id: id.to_string(),
            mesh,
            transform,
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: Vec::new(),
            paint_data: None,
            world_z_extent,
        }
    }

    #[test]
    fn object_world_z_extent_identity_matches_raw_vertices() {
        let object = make_object(
            "benchy",
            vec![
                Point3 { x: 0.0, y: 0.0, z: 0.0 },
                Point3 { x: 1.0, y: 2.0, z: 48.0 },
            ],
            identity_transform(),
        );
        let extent = object_world_z_extent(&object).expect("extent should exist");
        assert!((extent.0 - 0.0).abs() < 1e-5);
        assert!((extent.1 - 48.0).abs() < 1e-5);
    }

    #[test]
    fn object_world_z_extent_applies_translation() {
        let mut t = identity_transform();
        // Column-major: translation is in column 3 (index 12,13,14).
        t.matrix[14] = 10.0; // +10 on Z
        let object = make_object(
            "translated",
            vec![
                Point3 { x: 0.0, y: 0.0, z: 0.0 },
                Point3 { x: 0.0, y: 0.0, z: 5.0 },
            ],
            t,
        );
        let (z_min, z_max) = object_world_z_extent(&object).unwrap();
        assert!((z_min - 10.0).abs() < 1e-5);
        assert!((z_max - 15.0).abs() < 1e-5);
    }

    #[test]
    fn object_world_z_extent_applies_rotation_about_x() {
        // 90° rotation about X axis: (x, y, z) -> (x, -z, y).
        // So a vertical rod of height 10 along +Z becomes a horizontal rod
        // along -Y, and the world-space Z extent collapses to {0}.
        // Column-major storage: m[col*4 + row].
        let mut t = [0.0f64; 16];
        t[0] = 1.0; // col 0 row 0 — X stays X
        t[6] = 1.0; // col 1 row 2 — +Y becomes +Z
        t[9] = -1.0; // col 2 row 1 — +Z becomes -Y
        t[15] = 1.0;
        let transform = Transform3d { matrix: t };
        let object = make_object(
            "rotated",
            vec![
                Point3 { x: 0.0, y: 0.0, z: 0.0 },
                Point3 { x: 0.0, y: 0.0, z: 10.0 },
            ],
            transform,
        );
        // Post-rotation world Z values: 0 and 0 → degenerate (z_max == z_min).
        assert!(object_world_z_extent(&object).is_none());
    }

    #[test]
    fn object_world_z_extent_applies_scale() {
        let mut t = identity_transform();
        t.matrix[10] = 2.0; // scale Z by 2
        let object = make_object(
            "scaled",
            vec![
                Point3 { x: 0.0, y: 0.0, z: 0.0 },
                Point3 { x: 0.0, y: 0.0, z: 20.0 },
            ],
            t,
        );
        let (z_min, z_max) = object_world_z_extent(&object).unwrap();
        assert!((z_min - 0.0).abs() < 1e-5);
        assert!((z_max - 40.0).abs() < 1e-5);
    }

    #[test]
    fn object_world_z_extent_zero_matrix_treated_as_identity() {
        // Fixtures that leave `Transform3d::matrix` all-zero must not
        // collapse the mesh to a degenerate point.
        let object = make_object(
            "zero-matrix",
            vec![
                Point3 { x: 0.0, y: 0.0, z: 0.0 },
                Point3 { x: 0.0, y: 0.0, z: 7.0 },
            ],
            Transform3d { matrix: [0.0; 16] },
        );
        let (z_min, z_max) = object_world_z_extent(&object).unwrap();
        assert!((z_min - 0.0).abs() < 1e-5);
        assert!((z_max - 7.0).abs() < 1e-5);
    }

    #[test]
    fn object_world_z_extent_empty_mesh_is_none() {
        let object = make_object("empty", vec![], identity_transform());
        assert!(object_world_z_extent(&object).is_none());
    }
}
