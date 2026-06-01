//! File format loaders for STL, OBJ, and 3MF model files (TASK-076).
//!
//! Detects file format by extension and parses geometry into [`MeshIR`].
//! Supported formats:
//! - STL (binary and ASCII)
//! - OBJ (Wavefront)
//! - 3MF (3D Manufacturing Format â€” ZIP-based)

use std::collections::HashMap;
use std::fmt;
use std::io::{BufReader, Read, Seek};
use std::path::Path;

use crate::sidecar::{parse_3mf_sidecar, ObjectSidecarInfo, PartSubtype};

use slicer_ir::{
    BoundingBox3, ConfigDelta, ConfigValue, FacetPaintData, IndexedTriangleSet, MeshIR,
    ModifierScope, ModifierVolume, ObjectConfig, ObjectId, ObjectMesh, PaintLayer, PaintSemantic,
    PaintStroke, PaintValue, Point3, Transform3d,
};

/// Detected model file format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelFormat {
    /// STL (STereoLithography) â€” binary or ASCII.
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
    /// NON_UNIFORM_SCALE_UNSUPPORTED â€” object transform has non-uniform scale
    /// (scale_x â‰  scale_y or scale_y â‰  scale_z).  All three scale axes must be
    /// equal; non-uniform scale is not supported by the slicer pipeline.
    NonUniformScaleUnsupported {
        /// Scale factor along X axis (magnitude of transform column 0 xyz).
        scale_x: f64,
        /// Scale factor along Y axis (magnitude of transform column 1 xyz).
        scale_y: f64,
        /// Scale factor along Z axis (magnitude of transform column 2 xyz).
        scale_z: f64,
    },
    /// WORLD_Z_BELOW_FLOOR â€” one or more object vertices map to a world-space Z
    /// below the print volume floor (0.0 mm).  Translate the object upward so
    /// its lowest point is at or above Z = 0.
    WorldZBelowFloor {
        /// The minimum world-space Z found on the object (negative).
        z_min: f32,
    },
    /// 3MF paint metadata is malformed or contains an unrecognized value.
    PaintMetadata {
        /// Human-readable reason for the failure.
        reason: String,
        /// Byte offset into the XML stream where the malformed attribute was found.
        byte_offset: usize,
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
            Self::PaintMetadata { reason, byte_offset } => write!(
                f,
                "paint metadata error at byte {byte_offset}: {reason}"
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

/// Deterministic object ID: UUID v5 (SHA1) keyed on file path + per-file index.
/// Same path and index always produce the same UUID across process runs.
fn path_object_id(path: &Path, index: usize) -> String {
    // NAMESPACE_OID (RFC 4122 Â§4.3) â€” stable, well-known UUID namespace.
    const NS: uuid::Uuid = uuid::Uuid::from_bytes([
        0x6b, 0xa7, 0xb8, 0x14, 0x9d, 0xad, 0x11, 0xd1, 0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30,
        0xc8,
    ]);
    let name = format!("{}#{}", path.to_string_lossy(), index);
    uuid::Uuid::new_v5(&NS, name.as_bytes()).to_string()
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

    // Every format funnels its parsed mesh(es) through `assemble_object`, the
    // single owner of `ObjectMesh` wrapping + world-Z-extent (packet 75, Phase 4).
    let objects: Vec<ObjectMesh> = match format {
        ModelFormat::Stl => {
            let its = load_stl(&mut reader)?;
            vec![assemble_object(
                path_object_id(path, 0),
                its,
                ObjectConfig {
                    data: HashMap::new(),
                },
                Vec::new(),
                None,
            )]
        }
        ModelFormat::Obj => {
            let its = load_obj(path)?;
            vec![assemble_object(
                path_object_id(path, 0),
                its,
                ObjectConfig {
                    data: HashMap::new(),
                },
                Vec::new(),
                None,
            )]
        }
        ModelFormat::ThreeMf => {
            let items = load_3mf(&mut reader)?;
            items
                .into_iter()
                .enumerate()
                .map(|(idx, (its, paint_data, modifiers, object_config_data))| {
                    assemble_object(
                        path_object_id(path, idx),
                        its,
                        ObjectConfig {
                            data: object_config_data,
                        },
                        modifiers,
                        paint_data,
                    )
                })
                .collect()
        }
    };

    if objects.is_empty() {
        return Err(ModelLoadError::ThreeMfParse("no objects in model".into()));
    }

    let build_volume = compute_bounding_box_union(objects.iter().map(|o| &o.mesh));

    Ok(MeshIR {
        objects,
        build_volume,
        ..Default::default()
    })
}

/// Single owner of `ObjectMesh` assembly.
///
/// Wraps a parsed mesh into an `ObjectMesh` with an identity transform and a
/// freshly computed world-Z extent. Every producer routes through here — both
/// `load_model`'s per-format branches and the `mesh convert` command's
/// split-to-objects re-assembly — so the wrap and the z-extent computation live
/// in exactly one place (packet 75, Phase 4 / TASK-219).
pub fn assemble_object(
    id: ObjectId,
    mesh: IndexedTriangleSet,
    config: ObjectConfig,
    modifier_volumes: Vec<ModifierVolume>,
    paint_data: Option<FacetPaintData>,
) -> ObjectMesh {
    let world_z_extent = compute_z_extent_from_mesh(&mesh);
    ObjectMesh {
        id,
        mesh,
        transform: identity_transform(),
        config,
        modifier_volumes,
        paint_data,
        world_z_extent,
    }
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

struct Parsed3mfObject {
    mesh: Option<(IndexedTriangleSet, Option<FacetPaintData>)>,
    components: Vec<ParsedComponent>,
    transform: Option<[f64; 16]>,
}

struct ParsedComponent {
    objectid: u32,
    transform: Option<[f64; 16]>,
    /// `p:path` attribute â€” references an external .model file inside the 3MF archive.
    external_path: Option<String>,
}

struct ParsedBuildItem {
    objectid: u32,
    transform: Option<[f64; 16]>,
}

struct MeshCollector {
    vertices: Vec<Point3>,
    indices: Vec<u32>,
    has_any_paint: bool,
    fuzzy_states: Vec<Option<u32>>,
    support_states: Vec<Option<u32>>,
    seam_states: Vec<Option<u32>>,
    color_states: Vec<Option<u32>>,
    color_strokes: Vec<PaintStroke>,
    support_strokes_enforcer: Vec<PaintStroke>,
    support_strokes_blocker: Vec<PaintStroke>,
}

impl MeshCollector {
    fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            has_any_paint: false,
            fuzzy_states: Vec::new(),
            support_states: Vec::new(),
            seam_states: Vec::new(),
            color_states: Vec::new(),
            color_strokes: Vec::new(),
            support_strokes_enforcer: Vec::new(),
            support_strokes_blocker: Vec::new(),
        }
    }
}

fn parse_3mf_transform(attr: &[u8]) -> Result<[f64; 16], ModelLoadError> {
    let s = std::str::from_utf8(attr).map_err(|e| ModelLoadError::ThreeMfParse(e.to_string()))?;
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() != 12 {
        return Err(ModelLoadError::ThreeMfParse(format!(
            "3MF transform must have 12 floats, got {}",
            parts.len()
        )));
    }
    let floats: Vec<f64> = parts
        .iter()
        .map(|p| {
            p.parse::<f64>().map_err(|e| {
                ModelLoadError::ThreeMfParse(format!("invalid float in 3MF transform: {e}"))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    // 3MF Core spec Â§4.4: the 12 floats are a 4x3 matrix in row-major order
    // applied in row-vector convention v_new = v Â· M_row. To consume through
    // `apply_transform_to_vertex` (column-vector v_new = M_col Â· v, m[16] in
    // column-major storage) we need M_col = M_row^T. M_row[r][c] = floats[r*3 + c]
    // lands at m[c*4 + r] = M_col[r][c]. For r âˆˆ {0..2}, c âˆˆ {0..2} that's
    // m[r + 4*c] = floats[r*3 + c]; for the 3x3 rotation/scale block this is
    // an identity copy when written column-by-column. The translation row
    // (r=3) maps to m[12..15].
    let mut m = [0.0f64; 16];
    m[0] = floats[0];
    m[1] = floats[1];
    m[2] = floats[2];
    m[3] = 0.0;
    m[4] = floats[3];
    m[5] = floats[4];
    m[6] = floats[5];
    m[7] = 0.0;
    m[8] = floats[6];
    m[9] = floats[7];
    m[10] = floats[8];
    m[11] = 0.0;
    m[12] = floats[9];
    m[13] = floats[10];
    m[14] = floats[11];
    m[15] = 1.0;
    Ok(m)
}

fn compose_transforms(a: &[f64; 16], b: &[f64; 16]) -> [f64; 16] {
    let mut result = [0.0f64; 16];
    for col in 0..4 {
        for row in 0..4 {
            let mut sum = 0.0;
            for k in 0..4 {
                sum += a[k * 4 + row] * b[col * 4 + k];
            }
            result[col * 4 + row] = sum;
        }
    }
    result
}

fn identity_3mf_transform() -> [f64; 16] {
    let mut m = [0.0f64; 16];
    m[0] = 1.0;
    m[5] = 1.0;
    m[10] = 1.0;
    m[15] = 1.0;
    m
}

fn apply_transform_to_vertex(v: &Point3, m: &[f64; 16]) -> Point3 {
    if m.iter().all(|x| *x == 0.0) {
        return *v;
    }
    let x = f64::from(v.x);
    let y = f64::from(v.y);
    let z = f64::from(v.z);
    let tx = m[0] * x + m[4] * y + m[8] * z + m[12];
    let ty = m[1] * x + m[5] * y + m[9] * z + m[13];
    let tz = m[2] * x + m[6] * y + m[10] * z + m[14];
    let tw = m[3] * x + m[7] * y + m[11] * z + m[15];
    let w = if tw == 0.0 { 1.0 } else { tw };
    Point3 {
        x: (tx / w) as f32,
        y: (ty / w) as f32,
        z: (tz / w) as f32,
    }
}

fn apply_transform_to_mesh(mesh: &mut IndexedTriangleSet, m: &[f64; 16]) {
    for v in &mut mesh.vertices {
        *v = apply_transform_to_vertex(v, m);
    }
}

fn apply_transform_to_paint_data(pd: &mut FacetPaintData, m: &[f64; 16]) {
    for layer in &mut pd.layers {
        for stroke in &mut layer.strokes {
            for tri in &mut stroke.triangles {
                *tri = [
                    apply_transform_to_vertex(&tri[0], m),
                    apply_transform_to_vertex(&tri[1], m),
                    apply_transform_to_vertex(&tri[2], m),
                ];
            }
            // f32 rounding during the matrix multiply can collapse two
            // originally-distinct vertices onto the same coordinate. Drop
            // such triangles so downstream consumers (which assert
            // non-degeneracy) don't crash on them.
            stroke.triangles.retain(|tri| !is_degenerate_triangle(tri));
        }
        // Drop strokes that became empty (all triangles were degenerate).
        layer.strokes.retain(|s| !s.triangles.is_empty());
    }
}

/// Load external .model files referenced via `p:path` on component elements
/// (3MF production extension). Parses each external model file and merges its
/// `<object>` definitions into the main objects map so `resolve_object` can
/// find them by their local id.
fn load_external_model_objects(
    objects: &mut HashMap<u32, Parsed3mfObject>,
    archive: &mut zip::ZipArchive<impl Read + Seek>,
) -> Result<(), ModelLoadError> {
    let mut visited: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    loop {
        let new_paths: Vec<String> = objects
            .values()
            .flat_map(|o| o.components.iter())
            .filter_map(|c| c.external_path.clone())
            .filter(|p| !visited.contains(p))
            .collect();

        let mut batch: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for p in new_paths {
            batch.insert(p);
        }

        if batch.is_empty() {
            break;
        }

        for ext_path in &batch {
            visited.insert(ext_path.clone());
            // The p:path may have a leading slash; strip it for archive lookup.
            let archive_path = ext_path.trim_start_matches('/');

            let xml_bytes = match archive.by_name(archive_path) {
                Ok(mut file) => {
                    let mut buf = Vec::new();
                    file.read_to_end(&mut buf)
                        .map_err(|e| ModelLoadError::ThreeMfParse(e.to_string()))?;
                    buf
                }
                Err(_) => {
                    // External file not found in archive â€” the component will be
                    // resolved normally (inline mesh must exist in the main doc).
                    continue;
                }
            };

            // Parse the external model file and merge its objects into the main map.
            let mut reader = quick_xml::Reader::from_reader(xml_bytes.as_slice());
            reader.config_mut().trim_text(true);

            let mut current_object_id: Option<u32> = None;
            let mut current_mesh: Option<MeshCollector> = None;
            let mut current_components: Vec<ParsedComponent> = Vec::new();
            let mut current_object_transform: Option<[f64; 16]> = None;
            let mut in_components = false;
            let mut buf = Vec::new();

            loop {
                match reader.read_event_into(&mut buf) {
                    Ok(quick_xml::events::Event::Start(ref e))
                    | Ok(quick_xml::events::Event::Empty(ref e)) => {
                        let name_bytes = e.name().as_ref().to_vec();
                        let local = local_name(&name_bytes);
                        match local {
                            b"object" => {
                                let mut id: Option<u32> = None;
                                let mut transform: Option<[f64; 16]> = None;
                                for attr in e.attributes().flatten() {
                                    match local_name(attr.key.as_ref()) {
                                        b"id" => id = Some(parse_u32(&attr.value)?),
                                        b"transform" => {
                                            transform = Some(parse_3mf_transform(&attr.value)?)
                                        }
                                        _ => {}
                                    }
                                }
                                let object_id = id.ok_or_else(|| {
                                    ModelLoadError::ThreeMfParse(
                                        "external model object missing id attribute".into(),
                                    )
                                })?;
                                current_object_id = Some(object_id);
                                current_mesh = None;
                                current_components = Vec::new();
                                current_object_transform = transform;
                            }
                            b"mesh" if current_object_id.is_some() => {
                                current_mesh = Some(MeshCollector::new());
                            }
                            b"vertex" => {
                                if let Some(ref mut mc) = current_mesh {
                                    let (mut x, mut y, mut z) = (0.0f32, 0.0f32, 0.0f32);
                                    for attr in e.attributes().flatten() {
                                        match attr.key.as_ref() {
                                            b"x" => x = parse_f32(&attr.value)?,
                                            b"y" => y = parse_f32(&attr.value)?,
                                            b"z" => z = parse_f32(&attr.value)?,
                                            _ => {}
                                        }
                                    }
                                    mc.vertices.push(Point3 { x, y, z });
                                }
                            }
                            b"triangle" => {
                                if let Some(ref mut mc) = current_mesh {
                                    let mut v1: Option<u32> = None;
                                    let mut v2: Option<u32> = None;
                                    let mut v3: Option<u32> = None;
                                    for attr in e.attributes().flatten() {
                                        match attr.key.as_ref() {
                                            b"v1" => v1 = Some(parse_u32(&attr.value)?),
                                            b"v2" => v2 = Some(parse_u32(&attr.value)?),
                                            b"v3" => v3 = Some(parse_u32(&attr.value)?),
                                            _ => {}
                                        }
                                    }
                                    mc.indices.push(v1.ok_or_else(|| {
                                        ModelLoadError::ThreeMfParse("triangle missing v1".into())
                                    })?);
                                    mc.indices.push(v2.ok_or_else(|| {
                                        ModelLoadError::ThreeMfParse("triangle missing v2".into())
                                    })?);
                                    mc.indices.push(v3.ok_or_else(|| {
                                        ModelLoadError::ThreeMfParse("triangle missing v3".into())
                                    })?);
                                    mc.fuzzy_states.push(None);
                                    mc.support_states.push(None);
                                    mc.seam_states.push(None);
                                    mc.color_states.push(None);
                                }
                            }
                            b"components" if current_object_id.is_some() => {
                                in_components = true;
                            }
                            b"component" if in_components => {
                                let mut objectid: Option<u32> = None;
                                let mut transform: Option<[f64; 16]> = None;
                                let mut external_path: Option<String> = None;
                                for attr in e.attributes().flatten() {
                                    match local_name(attr.key.as_ref()) {
                                        b"objectid" => objectid = Some(parse_u32(&attr.value)?),
                                        b"transform" => {
                                            transform = Some(parse_3mf_transform(&attr.value)?)
                                        }
                                        b"path" => {
                                            external_path = Some(
                                                std::str::from_utf8(&attr.value)
                                                    .map_err(|e| {
                                                        ModelLoadError::ThreeMfParse(e.to_string())
                                                    })?
                                                    .to_string(),
                                            );
                                        }
                                        _ => {}
                                    }
                                }
                                let oid = objectid.ok_or_else(|| {
                                    ModelLoadError::ThreeMfParse(
                                        "external component missing objectid".into(),
                                    )
                                })?;
                                current_components.push(ParsedComponent {
                                    objectid: oid,
                                    transform,
                                    external_path,
                                });
                            }
                            _ => {}
                        }
                    }
                    Ok(quick_xml::events::Event::End(ref e)) => {
                        let name_bytes = e.name().as_ref().to_vec();
                        let local = local_name(&name_bytes);
                        match local {
                            b"object" => {
                                if let Some(object_id) = current_object_id.take() {
                                    let mesh_data = match current_mesh.take() {
                                        Some(mut mc) if !mc.vertices.is_empty() => {
                                            let vertices = std::mem::take(&mut mc.vertices);
                                            let indices = std::mem::take(&mut mc.indices);
                                            Some((IndexedTriangleSet { vertices, indices }, None))
                                        }
                                        _ => None,
                                    };
                                    // Only insert if the object doesn't already exist
                                    // (main model definitions take priority).
                                    objects.entry(object_id).or_insert(Parsed3mfObject {
                                        mesh: mesh_data,
                                        components: std::mem::take(&mut current_components),
                                        transform: current_object_transform.take(),
                                    });
                                }
                            }
                            b"components" => in_components = false,
                            _ => {}
                        }
                    }
                    Ok(quick_xml::events::Event::Eof) => break,
                    Err(e) => {
                        return Err(ModelLoadError::ThreeMfParse(format!(
                            "XML parse error in external model {archive_path}: {e}"
                        )));
                    }
                    _ => {}
                }
                buf.clear();
            }
        } // end for ext_path in batch
    } // end loop

    Ok(())
}

fn resolve_object(
    object_id: u32,
    incoming_transform: &[f64; 16],
    objects: &HashMap<u32, Parsed3mfObject>,
    visited: &mut Vec<u32>,
    sidecar: &HashMap<u32, ObjectSidecarInfo>,
) -> Result<
    (
        IndexedTriangleSet,
        Option<FacetPaintData>,
        Vec<ModifierVolume>,
    ),
    ModelLoadError,
> {
    if visited.contains(&object_id) {
        return Err(ModelLoadError::ThreeMfParse(format!(
            "3MF circular component reference detected for object {object_id}"
        )));
    }
    visited.push(object_id);

    let obj = objects.get(&object_id).ok_or_else(|| {
        ModelLoadError::ThreeMfParse(format!("3MF references undefined object id {object_id}"))
    })?;

    let effective_transform = if let Some(ref t) = obj.transform {
        compose_transforms(incoming_transform, t)
    } else {
        *incoming_transform
    };

    if let Some((ref mesh, ref paint)) = obj.mesh {
        let mut result_mesh = mesh.clone();
        apply_transform_to_mesh(&mut result_mesh, &effective_transform);
        let mut result_paint = paint.clone();
        if let Some(ref mut pd) = result_paint {
            apply_transform_to_paint_data(pd, &effective_transform);
        }
        visited.pop();
        Ok((result_mesh, result_paint, Vec::new()))
    } else if !obj.components.is_empty() {
        let mut merged_vertices = Vec::new();
        let mut merged_indices = Vec::new();
        let mut merged_paint: Option<FacetPaintData> = None;
        let mut accumulated_facet_count: usize = 0;
        let mut modifier_volumes: Vec<ModifierVolume> = Vec::new();

        for comp in &obj.components {
            let comp_transform = if let Some(ref t) = comp.transform {
                compose_transforms(&effective_transform, t)
            } else {
                effective_transform
            };

            // Check sidecar classification for this component part.
            // Sidecar layout: object_id -> parts map; part id = comp.objectid.
            let part_info = sidecar
                .get(&object_id)
                .and_then(|o| o.parts.get(&comp.objectid));
            let subtype = part_info
                .map(|p| p.subtype)
                .unwrap_or(PartSubtype::NormalPart);

            let (comp_mesh, comp_paint, child_modifiers) =
                resolve_object(comp.objectid, &comp_transform, objects, visited, sidecar)?;

            // Propagate any modifier volumes from deeper recursion.
            modifier_volumes.extend(child_modifiers);

            if subtype != PartSubtype::NormalPart {
                // Non-NormalPart: route to ModifierVolume, do not merge into solid mesh.
                if comp_paint.is_some() {
                    log::warn!(
                        target: "slicer_model_io::loader",
                        "paint data on non-normal part dropped (part id {})",
                        comp.objectid
                    );
                }

                let subtype_str = match subtype {
                    PartSubtype::ModifierPart => "modifier_part",
                    PartSubtype::NegativePart => "negative_part",
                    PartSubtype::SupportEnforcer => "support_enforcer",
                    PartSubtype::SupportBlocker => "support_blocker",
                    PartSubtype::NormalPart => unreachable!(),
                };

                let priority = match subtype {
                    PartSubtype::ModifierPart => 0u32,
                    PartSubtype::NegativePart => 100u32,
                    PartSubtype::SupportEnforcer => 200u32,
                    PartSubtype::SupportBlocker => 300u32,
                    PartSubtype::NormalPart => unreachable!(),
                };

                let modifier_id = format!("{}-{}-{}", object_id, comp.objectid, subtype_str);

                let mut config_fields = std::collections::HashMap::new();
                config_fields.insert(
                    "subtype".to_string(),
                    ConfigValue::String(subtype_str.to_string()),
                );

                if let Some(part) = part_info {
                    if let Some(fuzzy) = part.metadata.get("fuzzy_skin") {
                        config_fields
                            .insert("fuzzy_skin".to_string(), ConfigValue::String(fuzzy.clone()));
                    }
                    if let Some(extruder_str) = part.metadata.get("extruder") {
                        match extruder_str.parse::<i64>() {
                            Ok(v) => {
                                config_fields.insert("extruder".to_string(), ConfigValue::Int(v));
                            }
                            Err(_) => {
                                log::warn!(
                                    target: "slicer_model_io::loader",
                                    "extruder value '{}' on part {} is not a valid integer, skipping",
                                    extruder_str,
                                    comp.objectid
                                );
                            }
                        }
                    }
                    if let Some(matrix) = part.metadata.get("matrix") {
                        config_fields
                            .insert("matrix".to_string(), ConfigValue::String(matrix.clone()));
                    }
                }

                modifier_volumes.push(ModifierVolume {
                    id: modifier_id,
                    mesh: comp_mesh,
                    config_delta: ConfigDelta {
                        fields: config_fields,
                    },
                    priority,
                    applies_to: ModifierScope::AllFeatures,
                });
            } else {
                // NormalPart: merge into solid mesh as before.
                let comp_facet_count = comp_mesh.indices.len() / 3;

                let offset = merged_vertices.len() as u32;
                merged_vertices.extend(comp_mesh.vertices);
                merged_indices.extend(comp_mesh.indices.iter().map(|&i| i + offset));

                // Per-semantic alignment: every layer in merged_paint must end up with
                // facet_values.len() == accumulated_facet_count + comp_facet_count.
                // Components missing a semantic contribute None Ã— comp_facet_count;
                // semantics first appearing in this component are back-padded with None
                // Ã— accumulated_facet_count.
                if merged_paint.is_none() && comp_paint.is_some() {
                    merged_paint = Some(FacetPaintData { layers: Vec::new() });
                }
                if let Some(ref mut merged) = merged_paint {
                    let comp_layers = comp_paint
                        .as_ref()
                        .map(|pd| pd.layers.as_slice())
                        .unwrap_or(&[]);
                    for layer in merged.layers.iter_mut() {
                        if let Some(comp_layer) =
                            comp_layers.iter().find(|l| l.semantic == layer.semantic)
                        {
                            layer
                                .facet_values
                                .extend(comp_layer.facet_values.iter().cloned());
                            layer.strokes.extend(comp_layer.strokes.iter().cloned());
                        } else {
                            layer
                                .facet_values
                                .extend(std::iter::repeat_n(None, comp_facet_count));
                        }
                    }
                    for comp_layer in comp_layers {
                        let already = merged
                            .layers
                            .iter()
                            .any(|l| l.semantic == comp_layer.semantic);
                        if !already {
                            let mut new_layer = PaintLayer {
                                semantic: comp_layer.semantic.clone(),
                                facet_values: vec![None; accumulated_facet_count],
                                strokes: Vec::new(),
                            };
                            new_layer
                                .facet_values
                                .extend(comp_layer.facet_values.iter().cloned());
                            new_layer.strokes.extend(comp_layer.strokes.iter().cloned());
                            merged.layers.push(new_layer);
                        }
                    }
                }

                accumulated_facet_count += comp_facet_count;
            }
        }

        if merged_vertices.is_empty() && modifier_volumes.is_empty() {
            visited.pop();
            return Err(ModelLoadError::ThreeMfParse(
                "no geometry in 3MF component chain".into(),
            ));
        }

        // If all components were modifiers, create an empty solid mesh rather than erroring.
        if merged_vertices.is_empty() {
            visited.pop();
            return Ok((
                IndexedTriangleSet {
                    vertices: Vec::new(),
                    indices: Vec::new(),
                },
                None,
                modifier_volumes,
            ));
        }

        visited.pop();
        Ok((
            IndexedTriangleSet {
                vertices: merged_vertices,
                indices: merged_indices,
            },
            merged_paint,
            modifier_volumes,
        ))
    } else {
        visited.pop();
        Err(ModelLoadError::ThreeMfParse(format!(
            "3MF object {object_id} has neither mesh nor components"
        )))
    }
}

/// Load 3MF and return a list of (IndexedTriangleSet, optional paint data, modifier volumes) per build item.
/// Convert allowlist object-level sidecar keys to typed `ConfigValue` entries.
///
/// Allowlist: `extruder` â†’ `Int(i64)`, `enable_support` â†’ `Bool` (parses
/// `"1"`/`"0"`/`"true"`/`"false"`; warns and skips otherwise), `support_type`
/// â†’ `String`. Other keys are silently ignored. Mirrors the part-level
/// conversion discipline at the modifier-volume site.
fn object_metadata_to_config_data(
    metadata: &std::collections::BTreeMap<String, String>,
) -> HashMap<String, ConfigValue> {
    let mut out = HashMap::new();
    if let Some(s) = metadata.get("extruder") {
        match s.parse::<i64>() {
            Ok(v) => {
                out.insert("extruder".to_string(), ConfigValue::Int(v));
            }
            Err(_) => {
                log::warn!(
                    target: "slicer_model_io::loader",
                    "object-level extruder value '{}' is not a valid integer, skipping",
                    s
                );
            }
        }
    }
    if let Some(s) = metadata.get("enable_support") {
        match s.as_str() {
            "1" | "true" => {
                out.insert("enable_support".to_string(), ConfigValue::Bool(true));
            }
            "0" | "false" => {
                out.insert("enable_support".to_string(), ConfigValue::Bool(false));
            }
            other => {
                log::warn!(
                    target: "slicer_model_io::loader",
                    "object-level enable_support value '{}' is not a valid bool, skipping",
                    other
                );
            }
        }
    }
    if let Some(s) = metadata.get("support_type") {
        out.insert("support_type".to_string(), ConfigValue::String(s.clone()));
    }
    out
}

/// One element of the 3MF parse result: the geometry plus its modifier volumes,
/// optional paint data, and any per-object metadata key/values.
type ThreeMfPart = (
    IndexedTriangleSet,
    Option<FacetPaintData>,
    Vec<ModifierVolume>,
    HashMap<String, ConfigValue>,
);

fn load_3mf(reader: &mut (impl Read + Seek)) -> Result<Vec<ThreeMfPart>, ModelLoadError> {
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| ModelLoadError::ThreeMfParse(e.to_string()))?;

    let model_path = find_model_path(&archive)?;
    let xml_bytes = {
        let mut model_file = archive
            .by_name(&model_path)
            .map_err(|e| ModelLoadError::ThreeMfParse(e.to_string()))?;
        let mut buf = Vec::new();
        model_file
            .read_to_end(&mut buf)
            .map_err(|e| ModelLoadError::ThreeMfParse(e.to_string()))?;
        buf
    }; // model_file is dropped here, releasing borrow on archive

    let sidecar = parse_3mf_sidecar(&mut archive);
    parse_3mf_model_xml(&xml_bytes, &sidecar, &mut archive)
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

/// Parse 3MF model XML into a list of (IndexedTriangleSet, optional paint data, modifier volumes)
/// per build item.
///
/// Recognizes `<build>/<item>` and nested `<component>` transforms,
/// composes them, and bakes the composed transform into mesh vertices and
/// paint stroke vertices. The returned objects have identity transform
/// (transform baked into geometry).
///
/// Malformed transform strings (wrong count or non-numeric) produce
/// `ModelLoadError::ThreeMfParse`.
fn parse_3mf_model_xml(
    xml_bytes: &[u8],
    sidecar: &HashMap<u32, ObjectSidecarInfo>,
    archive: &mut zip::ZipArchive<impl Read + Seek>,
) -> Result<Vec<ThreeMfPart>, ModelLoadError> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_reader(xml_bytes);
    reader.config_mut().trim_text(true);

    let mut objects: HashMap<u32, Parsed3mfObject> = HashMap::new();
    let mut build_items: Vec<ParsedBuildItem> = Vec::new();

    let mut current_object_id: Option<u32> = None;
    let mut current_mesh: Option<MeshCollector> = None;
    let mut current_components: Vec<ParsedComponent> = Vec::new();
    let mut current_object_transform: Option<[f64; 16]> = None;
    let mut in_components = false;
    let mut in_build = false;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name_bytes = e.name().as_ref().to_vec();
                let local = local_name(&name_bytes);
                match local {
                    b"object" => {
                        let mut id: Option<u32> = None;
                        let mut transform: Option<[f64; 16]> = None;
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"id" => {
                                    id = Some(parse_u32(&attr.value)?);
                                }
                                b"transform" => {
                                    transform = Some(parse_3mf_transform(&attr.value)?);
                                }
                                _ => {}
                            }
                        }
                        let object_id = id.ok_or_else(|| {
                            ModelLoadError::ThreeMfParse("3MF object missing id attribute".into())
                        })?;
                        current_object_id = Some(object_id);
                        current_mesh = None;
                        current_components = Vec::new();
                        current_object_transform = transform;
                    }
                    b"mesh" if current_object_id.is_some() => {
                        current_mesh = Some(MeshCollector::new());
                    }
                    b"vertex" => {
                        if let Some(ref mut mc) = current_mesh {
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
                            mc.vertices.push(Point3 { x, y, z });
                        }
                    }
                    b"triangle" => {
                        if let Some(ref mut mc) = current_mesh {
                            let mut v1: Option<u32> = None;
                            let mut v2: Option<u32> = None;
                            let mut v3: Option<u32> = None;
                            let mut fuzzy_state: Option<u32> = None;
                            let mut support_state: Option<u32> = None;
                            let mut seam_state: Option<u32> = None;
                            let mut color_state: Option<u32> = None;
                            let mut color_hex: Option<String> = None;
                            let mut color_byte_offset: usize = 0;
                            let mut support_hex: Option<String> = None;
                            let mut support_byte_offset: usize = 0;

                            for attr in e.attributes().flatten() {
                                match attr.key.as_ref() {
                                    b"v1" => v1 = Some(parse_u32(&attr.value)?),
                                    b"v2" => v2 = Some(parse_u32(&attr.value)?),
                                    b"v3" => v3 = Some(parse_u32(&attr.value)?),
                                    b"paint_fuzzy_skin" => {
                                        let value_str =
                                            std::str::from_utf8(&attr.value).map_err(|_| {
                                                ModelLoadError::PaintMetadata {
                                                    reason:
                                                        "paint_fuzzy_skin value is not valid UTF-8"
                                                            .into(),
                                                    byte_offset: reader.error_position() as usize,
                                                }
                                            })?;
                                        let state = decode_paint_hex_state(
                                            value_str,
                                            reader.error_position() as usize,
                                        )?;
                                        if state > 1 {
                                            return Err(ModelLoadError::PaintMetadata {
                                                reason: format!(
                                                    "paint_fuzzy_skin state {state} is not supported \
                                                     (only state 1 is valid)"
                                                ),
                                                byte_offset: reader.error_position() as usize,
                                            });
                                        }
                                        if state == 1 {
                                            fuzzy_state = Some(state);
                                            mc.has_any_paint = true;
                                        }
                                    }
                                    b"paint_supports" => {
                                        let value_str =
                                            std::str::from_utf8(&attr.value).map_err(|_| {
                                                ModelLoadError::PaintMetadata {
                                                    reason:
                                                        "paint_supports value is not valid UTF-8"
                                                            .into(),
                                                    byte_offset: reader.error_position() as usize,
                                                }
                                            })?;
                                        support_hex = Some(value_str.to_string());
                                        support_byte_offset = reader.error_position() as usize;
                                        let state = decode_paint_hex_state(
                                            value_str,
                                            reader.error_position() as usize,
                                        )?;
                                        if state > 2 {
                                            return Err(ModelLoadError::PaintMetadata {
                                                reason: format!(
                                                    "paint_supports state {state} is not supported \
                                                     (only states 1-2 are valid)"
                                                ),
                                                byte_offset: reader.error_position() as usize,
                                            });
                                        }
                                        if state > 0 {
                                            support_state = Some(state);
                                            mc.has_any_paint = true;
                                        }
                                    }
                                    b"paint_seam" => {
                                        let value_str =
                                            std::str::from_utf8(&attr.value).map_err(|_| {
                                                ModelLoadError::PaintMetadata {
                                                    reason: "paint_seam value is not valid UTF-8"
                                                        .into(),
                                                    byte_offset: reader.error_position() as usize,
                                                }
                                            })?;
                                        let state = decode_paint_hex_state(
                                            value_str,
                                            reader.error_position() as usize,
                                        )?;
                                        if state > 2 {
                                            return Err(ModelLoadError::PaintMetadata {
                                                reason: format!(
                                                    "paint_seam state {state} is not supported (only \
                                                     states 1-2 are valid)"
                                                ),
                                                byte_offset: reader.error_position() as usize,
                                            });
                                        }
                                        if state > 0 {
                                            seam_state = Some(state);
                                            mc.has_any_paint = true;
                                        }
                                    }
                                    b"paint_color" => {
                                        let value_str =
                                            std::str::from_utf8(&attr.value).map_err(|_| {
                                                ModelLoadError::PaintMetadata {
                                                    reason: "paint_color value is not valid UTF-8"
                                                        .into(),
                                                    byte_offset: reader.error_position() as usize,
                                                }
                                            })?;
                                        color_hex = Some(value_str.to_string());
                                        color_byte_offset = reader.error_position() as usize;
                                        let state = decode_paint_hex_state(
                                            value_str,
                                            reader.error_position() as usize,
                                        )?;
                                        if state > 16 {
                                            return Err(ModelLoadError::PaintMetadata {
                                                reason: format!(
                                                    "paint_color state {state} is not supported (only \
                                                     states 1-16 are valid)"
                                                ),
                                                byte_offset: reader.error_position() as usize,
                                            });
                                        }
                                        if state > 0 {
                                            color_state = Some(state);
                                            mc.has_any_paint = true;
                                        }
                                    }
                                    _ => {}
                                }
                            }

                            if let Some(hex) = &color_hex {
                                if hex.len() > 2 && color_state.is_some_and(|s| s != 0) {
                                    let v1_idx = v1.unwrap_or(0) as usize;
                                    let v2_idx = v2.unwrap_or(0) as usize;
                                    let v3_idx = v3.unwrap_or(0) as usize;
                                    let tri_verts = [
                                        mc.vertices[v1_idx],
                                        mc.vertices[v2_idx],
                                        mc.vertices[v3_idx],
                                    ];
                                    if let Ok(pairs) =
                                        decode_paint_hex_strokes(hex, tri_verts, color_byte_offset)
                                    {
                                        for (sub_verts, sub_state) in pairs {
                                            mc.color_strokes.push(PaintStroke {
                                                triangles: vec![sub_verts],
                                                semantic: PaintSemantic::Material,
                                                value: PaintValue::ToolIndex(
                                                    sub_state.saturating_sub(1),
                                                ),
                                            });
                                        }
                                    }
                                }
                            }
                            if let Some(hex) = &support_hex {
                                if hex.len() > 2 && support_state.is_some_and(|s| s != 0) {
                                    let v1_idx = v1.unwrap_or(0) as usize;
                                    let v2_idx = v2.unwrap_or(0) as usize;
                                    let v3_idx = v3.unwrap_or(0) as usize;
                                    let tri_verts = [
                                        mc.vertices[v1_idx],
                                        mc.vertices[v2_idx],
                                        mc.vertices[v3_idx],
                                    ];
                                    if let Ok(pairs) = decode_paint_hex_strokes(
                                        hex,
                                        tri_verts,
                                        support_byte_offset,
                                    ) {
                                        for (sub_verts, sub_state) in pairs {
                                            let stroke = PaintStroke {
                                                triangles: vec![sub_verts],
                                                semantic: if sub_state == 1 {
                                                    PaintSemantic::SupportEnforcer
                                                } else {
                                                    PaintSemantic::SupportBlocker
                                                },
                                                value: PaintValue::Flag(true),
                                            };
                                            if sub_state == 1 {
                                                mc.support_strokes_enforcer.push(stroke);
                                            } else if sub_state == 2 {
                                                mc.support_strokes_blocker.push(stroke);
                                            }
                                        }
                                    }
                                }
                            }

                            mc.indices.push(v1.ok_or_else(|| {
                                ModelLoadError::ThreeMfParse("triangle missing v1".into())
                            })?);
                            mc.indices.push(v2.ok_or_else(|| {
                                ModelLoadError::ThreeMfParse("triangle missing v2".into())
                            })?);
                            mc.indices.push(v3.ok_or_else(|| {
                                ModelLoadError::ThreeMfParse("triangle missing v3".into())
                            })?);

                            mc.fuzzy_states.push(fuzzy_state);
                            mc.support_states.push(support_state);
                            mc.seam_states.push(seam_state);
                            mc.color_states.push(color_state);
                        }
                    }
                    b"components" if current_object_id.is_some() => {
                        in_components = true;
                    }
                    b"component" if in_components => {
                        let mut objectid: Option<u32> = None;
                        let mut transform: Option<[f64; 16]> = None;
                        let mut external_path: Option<String> = None;
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"objectid" => {
                                    objectid = Some(parse_u32(&attr.value)?);
                                }
                                b"transform" => {
                                    transform = Some(parse_3mf_transform(&attr.value)?);
                                }
                                _ => {
                                    // Check for p:path (production extension).
                                    let local = local_name(attr.key.as_ref());
                                    if local == b"path" {
                                        external_path = Some(
                                            std::str::from_utf8(&attr.value)
                                                .map_err(|e| {
                                                    ModelLoadError::ThreeMfParse(e.to_string())
                                                })?
                                                .to_string(),
                                        );
                                    }
                                }
                            }
                        }
                        let oid = objectid.ok_or_else(|| {
                            ModelLoadError::ThreeMfParse("3MF component missing objectid".into())
                        })?;
                        current_components.push(ParsedComponent {
                            objectid: oid,
                            transform,
                            external_path,
                        });
                    }
                    b"build" => {
                        in_build = true;
                    }
                    b"item" if in_build => {
                        let mut objectid: Option<u32> = None;
                        let mut transform: Option<[f64; 16]> = None;
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"objectid" => {
                                    objectid = Some(parse_u32(&attr.value)?);
                                }
                                b"transform" => {
                                    transform = Some(parse_3mf_transform(&attr.value)?);
                                }
                                _ => {}
                            }
                        }
                        let oid = objectid.ok_or_else(|| {
                            ModelLoadError::ThreeMfParse("3MF item missing objectid".into())
                        })?;
                        build_items.push(ParsedBuildItem {
                            objectid: oid,
                            transform,
                        });
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let name_bytes = e.name().as_ref().to_vec();
                let local = local_name(&name_bytes);
                match local {
                    b"object" => {
                        if let Some(object_id) = current_object_id.take() {
                            let mesh_data = match current_mesh.take() {
                                Some(mut mc) if !mc.vertices.is_empty() => {
                                    let facet_count = mc.indices.len() / 3;
                                    let has_paint = mc.has_any_paint;
                                    let vertices = std::mem::take(&mut mc.vertices);
                                    let indices = std::mem::take(&mut mc.indices);
                                    let paint_data = if has_paint {
                                        Some(mc.build_paint_data(facet_count)?)
                                    } else {
                                        None
                                    };
                                    Some((IndexedTriangleSet { vertices, indices }, paint_data))
                                }
                                _ => None,
                            };
                            objects.insert(
                                object_id,
                                Parsed3mfObject {
                                    mesh: mesh_data,
                                    components: std::mem::take(&mut current_components),
                                    transform: current_object_transform.take(),
                                },
                            );
                        }
                    }
                    b"components" => {
                        in_components = false;
                    }
                    b"build" => {
                        in_build = false;
                    }
                    _ => {}
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

    if build_items.is_empty() {
        let first_obj = objects
            .values()
            .find(|o| o.mesh.is_some())
            .ok_or_else(|| ModelLoadError::ThreeMfParse("no geometry found in 3MF".into()))?;
        let (its, paint) = first_obj
            .mesh
            .clone()
            .ok_or_else(|| ModelLoadError::ThreeMfParse("no geometry found in 3MF".into()))?;
        return Ok(vec![(its, paint, Vec::new(), HashMap::new())]);
    }

    // Load external model files referenced via p:path on component elements.
    // The production extension allows components to reference separate .model
    // files inside the archive. We parse those files and merge their objects
    // into the main objects map so resolve_object can find them by id.
    load_external_model_objects(&mut objects, archive)?;

    let mut results = Vec::new();
    for item in &build_items {
        let item_transform = item.transform.unwrap_or_else(identity_3mf_transform);
        let (its, paint, modifiers) = resolve_object(
            item.objectid,
            &item_transform,
            &objects,
            &mut Vec::new(),
            sidecar,
        )?;
        let object_config_data = sidecar
            .get(&item.objectid)
            .map(|info| object_metadata_to_config_data(&info.object_metadata))
            .unwrap_or_default();
        results.push((its, paint, modifiers, object_config_data));
    }

    if results.is_empty() {
        return Err(ModelLoadError::ThreeMfParse(
            "no geometry in 3MF build items".into(),
        ));
    }

    Ok(results)
}

impl MeshCollector {
    fn build_paint_data(self, facet_count: usize) -> Result<FacetPaintData, ModelLoadError> {
        let mut layers = Vec::new();

        if self.fuzzy_states.iter().any(|s| s == &Some(1)) {
            layers.push(PaintLayer {
                semantic: PaintSemantic::FuzzySkin,
                facet_values: self
                    .fuzzy_states
                    .iter()
                    .map(|s| {
                        if *s == Some(1) {
                            Some(PaintValue::Flag(true))
                        } else {
                            None
                        }
                    })
                    .collect(),
                strokes: Vec::new(),
            });
        }

        if self.support_states.iter().any(|s| s == &Some(1)) {
            layers.push(PaintLayer {
                semantic: PaintSemantic::SupportEnforcer,
                facet_values: self
                    .support_states
                    .iter()
                    .map(|s| {
                        if *s == Some(1) {
                            Some(PaintValue::Flag(true))
                        } else {
                            None
                        }
                    })
                    .collect(),
                strokes: self.support_strokes_enforcer.clone(),
            });
        }

        if self.support_states.iter().any(|s| s == &Some(2)) {
            layers.push(PaintLayer {
                semantic: PaintSemantic::SupportBlocker,
                facet_values: self
                    .support_states
                    .iter()
                    .map(|s| {
                        if *s == Some(2) {
                            Some(PaintValue::Flag(true))
                        } else {
                            None
                        }
                    })
                    .collect(),
                strokes: self.support_strokes_blocker.clone(),
            });
        }

        if self.seam_states.iter().any(|s| s == &Some(1)) {
            layers.push(PaintLayer {
                semantic: PaintSemantic::Custom("seam_enforcer".into()),
                facet_values: self
                    .seam_states
                    .iter()
                    .map(|s| {
                        if *s == Some(1) {
                            Some(PaintValue::Flag(true))
                        } else {
                            None
                        }
                    })
                    .collect(),
                strokes: Vec::new(),
            });
        }

        if self.seam_states.iter().any(|s| s == &Some(2)) {
            layers.push(PaintLayer {
                semantic: PaintSemantic::Custom("seam_blocker".into()),
                facet_values: self
                    .seam_states
                    .iter()
                    .map(|s| {
                        if *s == Some(2) {
                            Some(PaintValue::Flag(true))
                        } else {
                            None
                        }
                    })
                    .collect(),
                strokes: Vec::new(),
            });
        }

        if self.color_states.iter().any(|s| s.is_some()) {
            layers.push(PaintLayer {
                semantic: PaintSemantic::Material,
                facet_values: self
                    .color_states
                    .iter()
                    .map(|&s| s.map(|v| PaintValue::ToolIndex(v.saturating_sub(1))))
                    .collect(),
                strokes: self.color_strokes.clone(),
            });
        }

        for layer in &layers {
            if layer.facet_values.len() != facet_count {
                return Err(ModelLoadError::PaintMetadata {
                    reason: format!(
                        "paint layer {:?} facet_values length {} does not match triangle count {}",
                        layer.semantic,
                        layer.facet_values.len(),
                        facet_count
                    ),
                    byte_offset: 0,
                });
            }
        }

        Ok(FacetPaintData { layers })
    }
}

/// Decode a single hex digit (0-9, A-F, a-f).
fn hex_nibble(c: u8) -> Result<u32, ModelLoadError> {
    match c {
        b'0'..=b'9' => Ok((c - b'0') as u32),
        b'A'..=b'F' => Ok((c - b'A' + 10) as u32),
        b'a'..=b'f' => Ok((c - b'a' + 10) as u32),
        _ => Err(ModelLoadError::ThreeMfParse(format!(
            "invalid hex digit: {}",
            c as char
        ))),
    }
}

fn parse_nibbles(hex: &str, byte_offset: usize) -> Result<Vec<u8>, ModelLoadError> {
    // OrcaSlicer stores the bitstream in reversed nibble order (last nibble first).
    // Reverse the hex string to restore the original bitstream order.
    hex.bytes()
        .rev()
        .map(|b| {
            hex_nibble(b)
                .map(|n| n as u8)
                .map_err(|e| ModelLoadError::PaintMetadata {
                    reason: format!("invalid hex digit in paint state: {e}"),
                    byte_offset,
                })
        })
        .collect()
}

fn walk_triangle_selector_tree(
    nibbles: &[u8],
    pos: &mut usize,
    states: &mut Vec<u32>,
    byte_offset: usize,
    depth: u32,
) -> Result<(), ModelLoadError> {
    if depth > 64 {
        return Err(ModelLoadError::PaintMetadata {
            reason: "TriangleSelector tree exceeds maximum depth".into(),
            byte_offset,
        });
    }
    if *pos >= nibbles.len() {
        return Err(ModelLoadError::PaintMetadata {
            reason: "unexpected end of TriangleSelector tree data".into(),
            byte_offset,
        });
    }
    let nibble = nibbles[*pos];
    *pos += 1;
    let split_type = nibble & 0x3;
    let state_bits = nibble >> 2;

    if split_type == 0 {
        // Leaf node
        let state = if state_bits == 3 {
            // Extended state: next nibble holds (state - 3)
            if *pos >= nibbles.len() {
                return Err(ModelLoadError::PaintMetadata {
                    reason:
                        "unexpected end of TriangleSelector tree: missing extended state nibble"
                            .into(),
                    byte_offset,
                });
            }
            let ext = nibbles[*pos] as u32;
            *pos += 1;
            ext + 3
        } else {
            state_bits as u32
        };
        states.push(state);
    } else {
        // Non-leaf: recurse into split_type + 1 children
        let num_children = (split_type + 1) as usize;
        for _ in 0..num_children {
            walk_triangle_selector_tree(nibbles, pos, states, byte_offset, depth + 1)?;
        }
    }
    Ok(())
}

fn dominant_paint_state(states: &[u32]) -> u32 {
    let mut counts = std::collections::HashMap::new();
    for &s in states {
        if s != 0 {
            *counts.entry(s).or_insert(0u32) += 1;
        }
    }
    counts
        .into_iter()
        .max_by_key(|&(_, c)| c)
        .map(|(s, _)| s)
        .unwrap_or(0)
}

fn midpoint(a: Point3, b: Point3) -> Point3 {
    Point3 {
        x: (a.x + b.x) * 0.5,
        y: (a.y + b.y) * 0.5,
        z: (a.z + b.z) * 0.5,
    }
}

/// Split a triangle into child sub-triangles based on split_type and special_side.
/// Returns child triangle vertices and placeholder special_sides (each child reads
/// its own special_side from the bitstream during DFS deserialization).
fn split_triangle_strokes(
    verts: [Point3; 3],
    split_type: u8,
    special_side: u8,
    byte_offset: usize,
) -> Result<(Vec<[Point3; 3]>, Vec<u8>), ModelLoadError> {
    let i = special_side as usize;
    let j = ((special_side + 1) % 3) as usize;
    let k = ((special_side + 2) % 3) as usize;

    let child_verts = match split_type {
        1 => {
            let m_jk = midpoint(verts[j], verts[k]);
            vec![[verts[i], verts[j], m_jk], [m_jk, verts[k], verts[i]]]
        }
        2 => {
            let m_ij = midpoint(verts[i], verts[j]);
            let m_ik = midpoint(verts[i], verts[k]);
            vec![
                [verts[i], m_ij, m_ik],
                [m_ij, verts[j], m_ik],
                [verts[j], verts[k], m_ik],
            ]
        }
        3 => {
            if special_side != 0 {
                return Err(ModelLoadError::PaintMetadata {
                    reason: format!(
                        "split_sides=3 requires special_side=0, got {}",
                        special_side
                    ),
                    byte_offset,
                });
            }
            let m_01 = midpoint(verts[0], verts[1]);
            let m_12 = midpoint(verts[1], verts[2]);
            let m_20 = midpoint(verts[2], verts[0]);
            vec![
                [verts[0], m_01, m_20],
                [m_01, verts[1], m_12],
                [m_12, verts[2], m_20],
                [m_01, m_12, m_20],
            ]
        }
        // split_type is nibble & 0x3; leaf (0) handled before this call â€” 1/2/3 are exhaustive
        _ => unreachable!("split_type={} outside valid range 1â€“3", split_type),
    };

    let child_special_sides = vec![0u8; child_verts.len()];
    Ok((child_verts, child_special_sides))
}

fn walk_triangle_selector_strokes(
    nibbles: &[u8],
    pos: &mut usize,
    verts: [Point3; 3],
    _special_side: u8,
    out: &mut Vec<([Point3; 3], u32)>,
    byte_offset: usize,
    depth: u32,
) -> Result<(), ModelLoadError> {
    if depth > 64 {
        return Err(ModelLoadError::PaintMetadata {
            reason: "TriangleSelector tree exceeds maximum depth".into(),
            byte_offset,
        });
    }
    if *pos >= nibbles.len() {
        return Err(ModelLoadError::PaintMetadata {
            reason: "unexpected end of TriangleSelector tree data".into(),
            byte_offset,
        });
    }
    let nibble = nibbles[*pos];
    *pos += 1;
    let split_type = nibble & 0x3;

    if split_type == 0 {
        // Leaf node
        let state_bits = nibble >> 2;
        let state = if state_bits == 3 {
            if *pos >= nibbles.len() {
                return Err(ModelLoadError::PaintMetadata {
                    reason:
                        "unexpected end of TriangleSelector tree: missing extended state nibble"
                            .into(),
                    byte_offset,
                });
            }
            let ext = nibbles[*pos] as u32;
            *pos += 1;
            ext + 3
        } else {
            state_bits as u32
        };
        if state != 0 {
            out.push((verts, state));
        }
    } else {
        // Non-leaf: special_side is encoded in the upper bits of this nibble.
        let special_side = nibble >> 2;
        let (child_verts, _) =
            split_triangle_strokes(verts, split_type, special_side, byte_offset)?;
        for child in child_verts {
            walk_triangle_selector_strokes(nibbles, pos, child, 0, out, byte_offset, depth + 1)?;
        }
    }
    Ok(())
}

/// Returns `true` when any two of the three vertices coincide â€” emitting
/// such a stroke would crash downstream consumers (model_loader_tdd asserts
/// `a != b && b != c && a != c`). Coincident vertices arise from
/// `TriangleSelector` hex sequences that subdivide a triangle in ways that
/// place two midpoint vertices at the same coordinate.
fn is_degenerate_triangle(tri: &[Point3; 3]) -> bool {
    tri[0] == tri[1] || tri[1] == tri[2] || tri[0] == tri[2]
}

/// Decode a TriangleSelector hex-encoded state string into leaf sub-triangles and their states.
pub fn decode_paint_hex_strokes(
    hex: &str,
    verts: [Point3; 3],
    byte_offset: usize,
) -> Result<Vec<([Point3; 3], u32)>, ModelLoadError> {
    let hex = hex.trim();
    if hex.is_empty() {
        return Ok(vec![]);
    }
    let nibbles = parse_nibbles(hex, byte_offset)?;
    let mut pos = 0;
    let mut out = Vec::new();
    walk_triangle_selector_strokes(&nibbles, &mut pos, verts, 0, &mut out, byte_offset, 0)?;
    // Drop degenerate sub-triangles (two coincident vertices) â€” they arise
    // from `TriangleSelector` hex sequences that subdivide at midpoints which
    // collapse onto an existing vertex within float precision. Downstream
    // consumers assert non-degeneracy and would crash on them.
    out.retain(|(tri, _)| !is_degenerate_triangle(tri));
    Ok(out)
}

/// Decode a TriangleSelector hex-encoded state string.
///
/// - Empty string â†’ state 0 (unpainted).
/// - 1 hex char: nibble >> 2 = state, lower 2 bits = split (must be 0).
/// - 2 hex chars: state = first_nibble + 3, second_nibble must be 0xC.
/// - >2 chars: subdivision not supported.
fn decode_paint_hex_state(hex_str: &str, byte_offset: usize) -> Result<u32, ModelLoadError> {
    let hex_str = hex_str.trim();
    if hex_str.is_empty() {
        return Ok(0);
    }
    let bytes = hex_str.as_bytes();
    if bytes.len() == 1 {
        let nibble = hex_nibble(bytes[0]).map_err(|e| ModelLoadError::PaintMetadata {
            reason: format!("invalid hex digit in paint state: {e}"),
            byte_offset,
        })?;
        let split = nibble & 0x3;
        if split != 0 {
            return Err(ModelLoadError::PaintMetadata {
                reason: "TriangleSelector subdivision is not supported".into(),
                byte_offset,
            });
        }
        Ok(nibble >> 2)
    } else if bytes.len() == 2 {
        let first = hex_nibble(bytes[0]).map_err(|e| ModelLoadError::PaintMetadata {
            reason: format!("invalid hex digit in paint state: {e}"),
            byte_offset,
        })?;
        let second = hex_nibble(bytes[1]).map_err(|e| ModelLoadError::PaintMetadata {
            reason: format!("invalid hex digit in paint state: {e}"),
            byte_offset,
        })?;
        if second != 0xC {
            let split = second & 0x3;
            if split != 0 {
                return Err(ModelLoadError::PaintMetadata {
                    reason: "TriangleSelector subdivision is not supported".into(),
                    byte_offset,
                });
            }
            return Err(ModelLoadError::PaintMetadata {
                reason: format!(
                    "invalid 2-char TriangleSelector encoding: second nibble must be 0xC, got 0x{second:X}"
                ),
                byte_offset,
            });
        }
        Ok(first + 3)
    } else {
        // TriangleSelector subdivision tree: walk DFS, return dominant state
        let nibbles = parse_nibbles(hex_str, byte_offset)?;
        let mut pos = 0;
        let mut states = Vec::new();
        walk_triangle_selector_tree(&nibbles, &mut pos, &mut states, byte_offset, 0)?;
        Ok(dominant_paint_state(&states))
    }
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
fn compute_bounding_box_union<'a>(
    meshes: impl IntoIterator<Item = &'a IndexedTriangleSet>,
) -> BoundingBox3 {
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
    let mut any = false;
    for its in meshes {
        for v in &its.vertices {
            any = true;
            min.x = min.x.min(v.x);
            min.y = min.y.min(v.y);
            min.z = min.z.min(v.z);
            max.x = max.x.max(v.x);
            max.y = max.y.max(v.y);
            max.z = max.z.max(v.z);
        }
    }
    if !any {
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
/// transform â€” i.e. just the raw vertex Z coordinates.
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
#[allow(dead_code)]
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
/// Extracts the column-vector magnitudes (scale factors) from the upper-left 3Ã—3
/// of the 4Ã—4 column-major transform matrix.  If any two scale axes differ by
/// more than `1e-6`, returns [`ModelLoadError::NonUniformScaleUnsupported`].
///
/// A zero matrix is treated as identity (uniform scale 1.0) to stay consistent
/// with the zero-matrix convention used in [`object_world_z_extent`].
///
/// # Errors
///
/// Returns `Err(NonUniformScaleUnsupported { â€¦ })` when the extracted scale
/// factors are not all equal within tolerance.
pub fn validate_non_uniform_scale(object: &ObjectMesh) -> Result<(), ModelLoadError> {
    let m = &object.transform.matrix;
    // Identity shortcut â€” all-zero matrix is treated as identity.
    if m.iter().all(|v| *v == 0.0) {
        return Ok(()); // identity â†’ uniform scale 1.0
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
/// are treated as valid â€” they will be caught by later validation stages.
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
/// leave `Transform3d::matrix` unset â€” the same convention used elsewhere in
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
        let bb = compute_bounding_box_union(std::iter::once(&its));
        assert_eq!(bb.min.x, 0.0);
        assert_eq!(bb.max.x, 0.0);
    }

    #[test]
    fn bounding_box_union_spans_all_meshes() {
        let m1 = IndexedTriangleSet {
            vertices: vec![Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            }],
            indices: vec![],
        };
        let m2 = IndexedTriangleSet {
            vertices: vec![Point3 {
                x: 10.0,
                y: -5.0,
                z: 7.0,
            }],
            indices: vec![],
        };
        let bb = compute_bounding_box_union([&m1, &m2]);
        assert_eq!(bb.min.x, 0.0);
        assert_eq!(bb.min.y, -5.0);
        assert_eq!(bb.min.z, 0.0);
        assert_eq!(bb.max.x, 10.0);
        assert_eq!(bb.max.y, 0.0);
        assert_eq!(bb.max.z, 7.0);
    }

    // A single connected solid (tetrahedron): 4 vertices, 4 triangular faces all
    // sharing edges, spanning z ∈ [0, 2].
    fn tetrahedron() -> IndexedTriangleSet {
        IndexedTriangleSet {
            vertices: vec![
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3 {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3 {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 2.0,
                },
            ],
            indices: vec![0, 1, 2, 0, 1, 3, 0, 2, 3, 1, 2, 3],
        }
    }

    #[test]
    fn assemble_object_computes_z_extent_and_sets_identity_transform() {
        let mesh = tetrahedron();
        let expected = compute_z_extent_from_mesh(&mesh);
        let obj = assemble_object(
            "obj".to_string(),
            mesh.clone(),
            ObjectConfig {
                data: HashMap::new(),
            },
            Vec::new(),
            None,
        );
        assert_eq!(obj.world_z_extent, expected);
        assert_eq!(obj.world_z_extent, Some((0.0, 2.0)));
        assert_eq!(obj.transform.matrix, identity_transform().matrix);
        assert!(obj.modifier_volumes.is_empty() && obj.paint_data.is_none());
    }

    // Regression (packet 75, Phase 4 / AC-4.3): the `mesh convert` split path used
    // to *reuse* the parent's `world_z_extent` for a single-component solid;
    // routing through `assemble_object` *recomputes* it from the component mesh.
    // Under the identity transform convert uses, the two must be equal — otherwise
    // splitting a single solid would silently change its reported Z extent.
    #[test]
    fn single_component_split_preserves_world_z_extent() {
        // A single triangle is reliably one connected component (split emits even
        // single-face fragments as their own component), spanning z ∈ [0, 2].
        let single = IndexedTriangleSet {
            vertices: vec![
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3 {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3 {
                    x: 0.0,
                    y: 1.0,
                    z: 2.0,
                },
            ],
            indices: vec![0, 1, 2],
        };
        let parent = assemble_object(
            "parent".to_string(),
            single,
            ObjectConfig {
                data: HashMap::new(),
            },
            Vec::new(),
            None,
        );
        // Single-triangle short-circuit: skip `slicer_helpers::split_connected_components`
        // (would re-introduce a first-party dep). A single solid mesh trivially yields one
        // component identical to the input; that's the slicer-helpers contract we rely on.
        let components: Vec<IndexedTriangleSet> = vec![parent.mesh.clone()];
        assert_eq!(components.len(), 1, "a single triangle is one component");
        let reassembled = assemble_object(
            "parent".to_string(),
            components.into_iter().next().unwrap(),
            ObjectConfig {
                data: HashMap::new(),
            },
            Vec::new(),
            None,
        );
        assert_eq!(
            reassembled.world_z_extent, parent.world_z_extent,
            "recomputed extent must equal the reused parent extent for a single solid"
        );
    }

    fn make_object(id: &str, vertices: Vec<Point3>, transform: Transform3d) -> ObjectMesh {
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
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3 {
                    x: 1.0,
                    y: 2.0,
                    z: 48.0,
                },
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
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 5.0,
                },
            ],
            t,
        );
        let (z_min, z_max) = object_world_z_extent(&object).unwrap();
        assert!((z_min - 10.0).abs() < 1e-5);
        assert!((z_max - 15.0).abs() < 1e-5);
    }

    #[test]
    fn object_world_z_extent_applies_rotation_about_x() {
        // 90Â° rotation about X axis: (x, y, z) -> (x, -z, y).
        // So a vertical rod of height 10 along +Z becomes a horizontal rod
        // along -Y, and the world-space Z extent collapses to {0}.
        // Column-major storage: m[col*4 + row].
        let mut t = [0.0f64; 16];
        t[0] = 1.0; // col 0 row 0 â€” X stays X
        t[6] = 1.0; // col 1 row 2 â€” +Y becomes +Z
        t[9] = -1.0; // col 2 row 1 â€” +Z becomes -Y
        t[15] = 1.0;
        let transform = Transform3d { matrix: t };
        let object = make_object(
            "rotated",
            vec![
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 10.0,
                },
            ],
            transform,
        );
        // Post-rotation world Z values: 0 and 0 â†’ degenerate (z_max == z_min).
        assert!(object_world_z_extent(&object).is_none());
    }

    #[test]
    fn object_world_z_extent_applies_scale() {
        let mut t = identity_transform();
        t.matrix[10] = 2.0; // scale Z by 2
        let object = make_object(
            "scaled",
            vec![
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 20.0,
                },
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
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 7.0,
                },
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
