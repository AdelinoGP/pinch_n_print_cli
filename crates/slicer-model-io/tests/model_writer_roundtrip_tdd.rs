//! TDD round-trip tests for `write_3mf` and `write_obj`.
//!
//! Tests in this file cover acceptance criteria AC-1, AC-4, and AC-6
//! for packet 71 (paint-ready-3mf-export), Step 2.

use std::io::Cursor;

use slicer_ir::slice_ir::{
    BoundingBox3, IndexedTriangleSet, MeshIR, ObjectConfig, ObjectMesh, Point3, SemVer, Transform3d,
};
use slicer_model_io::{load_model, write_3mf, write_obj};

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn identity_transform() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, // col 0
            0.0, 1.0, 0.0, 0.0, // col 1
            0.0, 0.0, 1.0, 0.0, // col 2
            0.0, 0.0, 0.0, 1.0, // col 3
        ],
    }
}

/// A single tetrahedron: 4 vertices, 4 triangles.
fn single_tetra_mesh() -> MeshIR {
    let vertices = vec![
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
            z: 1.0,
        },
    ];
    // 4 faces of a tetrahedron
    let indices: Vec<u32> = vec![
        0, 1, 2, // base
        0, 1, 3, 0, 2, 3, 1, 2, 3,
    ];
    let its = IndexedTriangleSet { vertices, indices };

    let obj = ObjectMesh {
        id: "obj0".to_string(),
        mesh: its,
        transform: identity_transform(),
        config: ObjectConfig {
            data: Default::default(),
        },
        modifier_volumes: vec![],
        paint_data: None,
        world_z_extent: Some((0.0, 1.0)),
    };

    MeshIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        objects: vec![obj],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 256.0,
                y: 256.0,
                z: 256.0,
            },
        },
    }
}

/// Two cubes (as triangulated boxes) as separate objects.
fn two_cube_mesh() -> MeshIR {
    fn cube_its(offset_x: f32) -> IndexedTriangleSet {
        // 8 vertices of a unit cube, translated by offset_x
        let v = |x: f32, y: f32, z: f32| Point3 {
            x: x + offset_x,
            y,
            z,
        };
        let vertices = vec![
            v(0.0, 0.0, 0.0), // 0
            v(1.0, 0.0, 0.0), // 1
            v(1.0, 1.0, 0.0), // 2
            v(0.0, 1.0, 0.0), // 3
            v(0.0, 0.0, 1.0), // 4
            v(1.0, 0.0, 1.0), // 5
            v(1.0, 1.0, 1.0), // 6
            v(0.0, 1.0, 1.0), // 7
        ];
        // 6 faces × 2 triangles = 12 triangles
        #[rustfmt::skip]
        let indices: Vec<u32> = vec![
            0,1,2, 0,2,3, // bottom
            4,6,5, 4,7,6, // top
            0,1,5, 0,5,4, // front
            2,3,7, 2,7,6, // back
            0,3,7, 0,7,4, // left
            1,2,6, 1,6,5, // right
        ];
        IndexedTriangleSet { vertices, indices }
    }

    let obj0 = ObjectMesh {
        id: "cube0".to_string(),
        mesh: cube_its(0.0),
        transform: identity_transform(),
        config: ObjectConfig {
            data: Default::default(),
        },
        modifier_volumes: vec![],
        paint_data: None,
        world_z_extent: Some((0.0, 1.0)),
    };
    let obj1 = ObjectMesh {
        id: "cube1".to_string(),
        mesh: cube_its(2.0),
        transform: identity_transform(),
        config: ObjectConfig {
            data: Default::default(),
        },
        modifier_volumes: vec![],
        paint_data: None,
        world_z_extent: Some((0.0, 1.0)),
    };

    MeshIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        objects: vec![obj0, obj1],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 256.0,
                y: 256.0,
                z: 256.0,
            },
        },
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// AC-1: round-trip single solid — bit-identical geometry
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn roundtrip_single_solid_exact() {
    let source = single_tetra_mesh();

    // Write to a tempfile (load_model needs a Path)
    let mut tmp = tempfile::Builder::new()
        .suffix(".3mf")
        .tempfile()
        .expect("tempfile creation");

    write_3mf(&source, tmp.as_file_mut()).expect("write_3mf failed");

    // Flush and reload via the crate's public loader
    let path = tmp.path().to_path_buf();
    let reloaded = load_model(&path).expect("load_model failed");

    // Object count
    assert_eq!(
        reloaded.objects.len(),
        source.objects.len(),
        "object count mismatch"
    );

    // Bit-identical vertices and indices
    let src_obj = &source.objects[0];
    let rel_obj = &reloaded.objects[0];
    assert_eq!(
        rel_obj.mesh.vertices.len(),
        src_obj.mesh.vertices.len(),
        "vertex count mismatch"
    );
    assert_eq!(
        rel_obj.mesh.indices.len(),
        src_obj.mesh.indices.len(),
        "index count mismatch"
    );

    for (i, (sv, rv)) in src_obj
        .mesh
        .vertices
        .iter()
        .zip(rel_obj.mesh.vertices.iter())
        .enumerate()
    {
        assert_eq!(sv.x, rv.x, "vertex[{i}].x mismatch: {} vs {}", sv.x, rv.x);
        assert_eq!(sv.y, rv.y, "vertex[{i}].y mismatch: {} vs {}", sv.y, rv.y);
        assert_eq!(sv.z, rv.z, "vertex[{i}].z mismatch: {} vs {}", sv.z, rv.z);
    }

    for (i, (si, ri)) in src_obj
        .mesh
        .indices
        .iter()
        .zip(rel_obj.mesh.indices.iter())
        .enumerate()
    {
        assert_eq!(si, ri, "index[{i}] mismatch: {si} vs {ri}");
    }

    // AABB equality
    let src_aabb = aabb(&src_obj.mesh);
    let rel_aabb = aabb(&rel_obj.mesh);
    assert_eq!(src_aabb.0, rel_aabb.0, "AABB min mismatch");
    assert_eq!(src_aabb.1, rel_aabb.1, "AABB max mismatch");
}

fn aabb(its: &IndexedTriangleSet) -> (Point3, Point3) {
    let mut min = Point3 {
        x: f32::MAX,
        y: f32::MAX,
        z: f32::MAX,
    };
    let mut max = Point3 {
        x: f32::MIN,
        y: f32::MIN,
        z: f32::MIN,
    };
    for v in &its.vertices {
        min.x = min.x.min(v.x);
        min.y = min.y.min(v.y);
        min.z = min.z.min(v.z);
        max.x = max.x.max(v.x);
        max.y = max.y.max(v.y);
        max.z = max.z.max(v.z);
    }
    (min, max)
}

// ─────────────────────────────────────────────────────────────────────────────
// AC-4: OPC package structure and namespaces
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn threemf_opc_package_and_namespaces() {
    let mesh = single_tetra_mesh();

    let mut buf: Cursor<Vec<u8>> = Cursor::new(Vec::new());
    write_3mf(&mesh, &mut buf).expect("write_3mf failed");

    let data = buf.into_inner();
    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).expect("zip open failed");

    // Required entries present
    let names: Vec<String> = (0..archive.len())
        .map(|i| archive.name_for_index(i).unwrap().to_string())
        .collect();

    assert!(
        names.iter().any(|n| n == "[Content_Types].xml"),
        "missing [Content_Types].xml; entries: {names:?}"
    );
    assert!(
        names.iter().any(|n| n == "_rels/.rels"),
        "missing _rels/.rels; entries: {names:?}"
    );
    assert!(
        names.iter().any(|n| n.ends_with("3dmodel.model")),
        "missing 3dmodel.model; entries: {names:?}"
    );
    assert!(
        names.iter().any(|n| n == "Metadata/model_settings.config"),
        "missing Metadata/model_settings.config; entries: {names:?}"
    );

    // Read 3dmodel.model and check root element attributes
    let model_path = names
        .iter()
        .find(|n| n.ends_with("3dmodel.model"))
        .unwrap()
        .clone();
    let model_xml = {
        let mut f = archive.by_name(&model_path).unwrap();
        let mut v = Vec::new();
        std::io::Read::read_to_end(&mut f, &mut v).unwrap();
        String::from_utf8(v).expect("model XML must be UTF-8")
    };

    assert!(
        model_xml.contains("unit=\"millimeter\""),
        "missing unit=\"millimeter\" in model XML"
    );
    assert!(
        model_xml.contains("xmlns=\"http://schemas.microsoft.com/3dmanufacturing/core/2015/02\""),
        "missing 3MF core namespace in model XML"
    );

    // Read sidecar and check subtype="normal_part"
    let sidecar_xml = {
        let mut f = archive.by_name("Metadata/model_settings.config").unwrap();
        let mut v = Vec::new();
        std::io::Read::read_to_end(&mut f, &mut v).unwrap();
        String::from_utf8(v).expect("sidecar XML must be UTF-8")
    };

    assert!(
        sidecar_xml.contains("subtype=\"normal_part\""),
        "missing subtype=\"normal_part\" in sidecar XML"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// AC-6: OBJ geometry and object groups
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn obj_geometry_and_object_groups() {
    let mesh = two_cube_mesh();

    let total_src_vertices: usize = mesh.objects.iter().map(|o| o.mesh.vertices.len()).sum();
    let total_src_triangles: usize = mesh.objects.iter().map(|o| o.mesh.indices.len() / 3).sum();
    let object_count = mesh.objects.len();

    let mut buf: Vec<u8> = Vec::new();
    write_obj(&mesh, &mut buf).expect("write_obj failed");

    let obj_text = String::from_utf8(buf.clone()).expect("OBJ must be UTF-8");

    // Count `o ` lines
    let group_lines = obj_text.lines().filter(|l| l.starts_with("o ")).count();
    assert_eq!(
        group_lines, object_count,
        "expected {object_count} 'o ' lines, got {group_lines}"
    );

    // Parse via tobj for geometry counts
    let cursor = Cursor::new(buf);
    let (models, _materials) = tobj::load_obj_buf(
        &mut std::io::BufReader::new(cursor),
        &tobj::LoadOptions {
            single_index: true,
            triangulate: false,
            ignore_points: true,
            ignore_lines: true,
        },
        // no MTL resolver needed
        |_| Err(tobj::LoadError::OpenFileFailed),
    )
    .expect("tobj parse failed");

    let total_tobj_vertices: usize = models.iter().map(|m| m.mesh.positions.len() / 3).sum();
    let total_tobj_triangles: usize = models.iter().map(|m| m.mesh.indices.len() / 3).sum();

    assert_eq!(
        total_tobj_vertices, total_src_vertices,
        "tobj vertex count {total_tobj_vertices} != source {total_src_vertices}"
    );
    assert_eq!(
        total_tobj_triangles, total_src_triangles,
        "tobj triangle count {total_tobj_triangles} != source {total_src_triangles}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Regression: object names come from ObjectMesh.id (<stem>/<stem>_<i>),
// not a hardcoded "object_<i>". Guards requirements.md:65 naming refinement,
// which had no AC coverage and so silently regressed.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn object_names_use_object_id() {
    let mesh = two_cube_mesh(); // ids "cube0", "cube1"

    // ── 3MF: sidecar name metadata reflects obj.id ───────────────────────────
    let mut buf: Cursor<Vec<u8>> = Cursor::new(Vec::new());
    write_3mf(&mesh, &mut buf).expect("write_3mf failed");
    let data = buf.into_inner();
    let mut archive = zip::ZipArchive::new(Cursor::new(data)).expect("zip open failed");
    let sidecar_xml = {
        let mut f = archive.by_name("Metadata/model_settings.config").unwrap();
        let mut v = Vec::new();
        std::io::Read::read_to_end(&mut f, &mut v).unwrap();
        String::from_utf8(v).expect("sidecar XML must be UTF-8")
    };
    assert!(
        sidecar_xml.contains("value=\"cube0\""),
        "sidecar missing name from obj.id 'cube0'; got:\n{sidecar_xml}"
    );
    assert!(
        sidecar_xml.contains("value=\"cube1\""),
        "sidecar missing name from obj.id 'cube1'; got:\n{sidecar_xml}"
    );
    assert!(
        !sidecar_xml.contains("value=\"object_0\""),
        "sidecar still emits hardcoded 'object_0' instead of obj.id"
    );

    // ── OBJ: `o <id>` group lines reflect obj.id ─────────────────────────────
    let mut obuf: Vec<u8> = Vec::new();
    write_obj(&mesh, &mut obuf).expect("write_obj failed");
    let obj_text = String::from_utf8(obuf).expect("OBJ must be UTF-8");
    assert!(
        obj_text.lines().any(|l| l == "o cube0"),
        "OBJ missing 'o cube0' group line; got:\n{obj_text}"
    );
    assert!(
        obj_text.lines().any(|l| l == "o cube1"),
        "OBJ missing 'o cube1' group line; got:\n{obj_text}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Regression: 3MF object names are XML-escaped, since names now flow from
// user-supplied file stems and may contain XML-special characters.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn threemf_object_name_is_xml_escaped() {
    let mut mesh = single_tetra_mesh();
    mesh.objects[0].id = "a&b<c>\"d".to_string();

    let mut buf: Cursor<Vec<u8>> = Cursor::new(Vec::new());
    write_3mf(&mesh, &mut buf).expect("write_3mf failed");
    let data = buf.into_inner();
    let mut archive = zip::ZipArchive::new(Cursor::new(data)).expect("zip open failed");
    let sidecar_xml = {
        let mut f = archive.by_name("Metadata/model_settings.config").unwrap();
        let mut v = Vec::new();
        std::io::Read::read_to_end(&mut f, &mut v).unwrap();
        String::from_utf8(v).expect("sidecar XML must be UTF-8")
    };

    assert!(
        sidecar_xml.contains("value=\"a&amp;b&lt;c&gt;&quot;d\""),
        "object name not XML-escaped; got:\n{sidecar_xml}"
    );
    assert!(
        !sidecar_xml.contains("a&b<c>"),
        "raw unescaped name leaked into sidecar XML"
    );
}
