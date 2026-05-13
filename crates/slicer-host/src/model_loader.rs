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
    BoundingBox3, FacetPaintData, IndexedTriangleSet, MeshIR, ObjectConfig, ObjectMesh, PaintLayer,
    PaintSemantic, PaintStroke, PaintValue, Point3, SemVer, Transform3d,
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

    let (its, paint_data) = match format {
        ModelFormat::Stl => (load_stl(&mut reader)?, None),
        ModelFormat::Obj => (load_obj(path)?, None),
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
        paint_data,
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

/// Load 3MF and return IndexedTriangleSet and optional FacetPaintData from the first object.
fn load_3mf(
    reader: &mut (impl Read + Seek),
) -> Result<(IndexedTriangleSet, Option<FacetPaintData>), ModelLoadError> {
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

/// Parse 3MF model XML into IndexedTriangleSet and optional FacetPaintData.
///
/// Recognizes the four per-triangle paint attributes emitted by
/// OrcaSlicer/BambuStudio: `paint_fuzzy_skin`, `paint_supports`,
/// `paint_seam`, and `paint_color`.  Painted triangles carry hex-encoded
/// state strings; unpainted triangles omit the attribute.  Subdivision
/// (strings longer than two hex characters or split bits ≠ 0) raises
/// `ModelLoadError::PaintMetadata`.
fn parse_3mf_model_xml(
    xml_bytes: &[u8],
) -> Result<(IndexedTriangleSet, Option<FacetPaintData>), ModelLoadError> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_reader(xml_bytes);
    reader.config_mut().trim_text(true);

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut buf = Vec::new();
    let mut in_mesh = false;
    let mut has_any_paint = false;
    let mut fuzzy_states: Vec<Option<u32>> = Vec::new();
    let mut support_states: Vec<Option<u32>> = Vec::new();
    let mut seam_states: Vec<Option<u32>> = Vec::new();
    let mut color_states: Vec<Option<u32>> = Vec::new();
    let mut color_strokes: Vec<PaintStroke> = Vec::new();
    let mut support_strokes_enforcer: Vec<PaintStroke> = Vec::new();
    let mut support_strokes_blocker: Vec<PaintStroke> = Vec::new();

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
                                                reason: "paint_fuzzy_skin value is not valid UTF-8"
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
                                        has_any_paint = true;
                                    }
                                }
                                b"paint_supports" => {
                                    let value_str =
                                        std::str::from_utf8(&attr.value).map_err(|_| {
                                            ModelLoadError::PaintMetadata {
                                                reason: "paint_supports value is not valid UTF-8"
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
                                        has_any_paint = true;
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
                                        has_any_paint = true;
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
                                        has_any_paint = true;
                                    }
                                }
                                _ => {}
                            }
                        }

                        // Decode strokes for subdivided paint channels (hex len > 2 only).
                        if let Some(hex) = &color_hex {
                            if hex.len() > 2 && color_state.map_or(false, |s| s != 0) {
                                let v1_idx = v1.unwrap_or(0) as usize;
                                let v2_idx = v2.unwrap_or(0) as usize;
                                let v3_idx = v3.unwrap_or(0) as usize;
                                let tri_verts =
                                    [vertices[v1_idx], vertices[v2_idx], vertices[v3_idx]];
                                if let Ok(pairs) =
                                    decode_paint_hex_strokes(hex, tri_verts, color_byte_offset)
                                {
                                    for (sub_verts, sub_state) in pairs {
                                        color_strokes.push(PaintStroke {
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
                            if hex.len() > 2 && support_state.map_or(false, |s| s != 0) {
                                let v1_idx = v1.unwrap_or(0) as usize;
                                let v2_idx = v2.unwrap_or(0) as usize;
                                let v3_idx = v3.unwrap_or(0) as usize;
                                let tri_verts =
                                    [vertices[v1_idx], vertices[v2_idx], vertices[v3_idx]];
                                if let Ok(pairs) =
                                    decode_paint_hex_strokes(hex, tri_verts, support_byte_offset)
                                {
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
                                            support_strokes_enforcer.push(stroke);
                                        } else if sub_state == 2 {
                                            support_strokes_blocker.push(stroke);
                                        }
                                    }
                                }
                            }
                        }

                        indices.push(v1.ok_or_else(|| {
                            ModelLoadError::ThreeMfParse("triangle missing v1".into())
                        })?);
                        indices.push(v2.ok_or_else(|| {
                            ModelLoadError::ThreeMfParse("triangle missing v2".into())
                        })?);
                        indices.push(v3.ok_or_else(|| {
                            ModelLoadError::ThreeMfParse("triangle missing v3".into())
                        })?);

                        fuzzy_states.push(fuzzy_state);
                        support_states.push(support_state);
                        seam_states.push(seam_state);
                        color_states.push(color_state);
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

    let its = IndexedTriangleSet { vertices, indices };

    let paint_data = if has_any_paint {
        let facet_count = its.indices.len() / 3;
        let mut layers = Vec::new();

        if fuzzy_states.iter().any(|s| s == &Some(1)) {
            layers.push(PaintLayer {
                semantic: PaintSemantic::FuzzySkin,
                facet_values: fuzzy_states
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

        if support_states.iter().any(|s| s == &Some(1)) {
            layers.push(PaintLayer {
                semantic: PaintSemantic::SupportEnforcer,
                facet_values: support_states
                    .iter()
                    .map(|s| {
                        if *s == Some(1) {
                            Some(PaintValue::Flag(true))
                        } else {
                            None
                        }
                    })
                    .collect(),
                strokes: support_strokes_enforcer,
            });
        }

        if support_states.iter().any(|s| s == &Some(2)) {
            layers.push(PaintLayer {
                semantic: PaintSemantic::SupportBlocker,
                facet_values: support_states
                    .iter()
                    .map(|s| {
                        if *s == Some(2) {
                            Some(PaintValue::Flag(true))
                        } else {
                            None
                        }
                    })
                    .collect(),
                strokes: support_strokes_blocker,
            });
        }

        if seam_states.iter().any(|s| s == &Some(1)) {
            layers.push(PaintLayer {
                semantic: PaintSemantic::Custom("seam_enforcer".into()),
                facet_values: seam_states
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

        if seam_states.iter().any(|s| s == &Some(2)) {
            layers.push(PaintLayer {
                semantic: PaintSemantic::Custom("seam_blocker".into()),
                facet_values: seam_states
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

        if color_states.iter().any(|s| s.is_some()) {
            layers.push(PaintLayer {
                semantic: PaintSemantic::Material,
                facet_values: color_states
                    .iter()
                    .map(|&s| s.map(|v| PaintValue::ToolIndex(v.saturating_sub(1))))
                    .collect(),
                strokes: color_strokes,
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

        Some(FacetPaintData { layers })
    } else {
        None
    };

    Ok((its, paint_data))
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
        // split_type is nibble & 0x3; leaf (0) handled before this call — 1/2/3 are exhaustive
        _ => unreachable!("split_type={} outside valid range 1–3", split_type),
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
    Ok(out)
}

/// Decode a TriangleSelector hex-encoded state string.
///
/// - Empty string → state 0 (unpainted).
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
