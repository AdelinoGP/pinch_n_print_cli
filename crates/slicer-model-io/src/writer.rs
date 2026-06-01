//! Geometry-only serializers: `write_3mf` and `write_obj`.
//!
//! Both functions are pure 1:1 serializers — one resource object per
//! `MeshIR.objects[i]`. No connected-component splitting happens here;
//! that is the caller's responsibility.

use std::io::{self, Write};

use slicer_ir::slice_ir::MeshIR;

/// Escape a string for safe inclusion in an XML attribute value.
fn xml_escape_attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// 3MF writer
// ─────────────────────────────────────────────────────────────────────────────

/// Write a `MeshIR` as a geometry-only 3MF package.
///
/// The output is a ZIP archive that contains:
/// - `[Content_Types].xml`
/// - `_rels/.rels`
/// - `3D/3dmodel.model`
/// - `Metadata/model_settings.config`
///
/// One `<object>` resource and one `<build><item>` are emitted per
/// `mesh.objects[i]`. The writer takes `impl Write + Seek` so it can be
/// driven by an in-memory `Cursor<Vec<u8>>` or a `tempfile`.
pub fn write_3mf(mesh: &MeshIR, writer: impl Write + std::io::Seek) -> io::Result<()> {
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let mut zip = zip::ZipWriter::new(writer);

    // ── [Content_Types].xml ──────────────────────────────────────────────────
    zip.start_file("[Content_Types].xml", options)?;
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
 <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
 <Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/>
</Types>"#,
    )?;

    // ── _rels/.rels ──────────────────────────────────────────────────────────
    zip.start_file("_rels/.rels", options)?;
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
 <Relationship Target="/3D/3dmodel.model" Id="rel-1" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/>
</Relationships>"#,
    )?;

    // ── 3D/3dmodel.model ─────────────────────────────────────────────────────
    zip.start_file("3D/3dmodel.model", options)?;
    {
        let mut model = String::new();
        model.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        model.push_str(
            "<model unit=\"millimeter\" xml:lang=\"en-US\" \
             xmlns=\"http://schemas.microsoft.com/3dmanufacturing/core/2015/02\" \
             xmlns:slic3rpe=\"http://schemas.slic3r.org/3mf/2017/06\">\n",
        );
        model.push_str(" <resources>\n");

        for (i, obj) in mesh.objects.iter().enumerate() {
            let resource_id = (i + 1) as u32; // sequential, 1-based
            model.push_str(&format!("  <object id=\"{resource_id}\" type=\"model\">\n"));
            model.push_str("   <mesh>\n");

            // vertices
            model.push_str("    <vertices>\n");
            for v in &obj.mesh.vertices {
                model.push_str(&format!(
                    "     <vertex x=\"{}\" y=\"{}\" z=\"{}\"/>\n",
                    v.x, v.y, v.z
                ));
            }
            model.push_str("    </vertices>\n");

            // triangles
            model.push_str("    <triangles>\n");
            let tri_count = obj.mesh.indices.len() / 3;
            for t in 0..tri_count {
                let v1 = obj.mesh.indices[t * 3];
                let v2 = obj.mesh.indices[t * 3 + 1];
                let v3 = obj.mesh.indices[t * 3 + 2];
                model.push_str(&format!(
                    "     <triangle v1=\"{v1}\" v2=\"{v2}\" v3=\"{v3}\"/>\n"
                ));
            }
            model.push_str("    </triangles>\n");

            model.push_str("   </mesh>\n");
            model.push_str("  </object>\n");
        }

        model.push_str(" </resources>\n");
        model.push_str(" <build>\n");
        for i in 0..mesh.objects.len() {
            let resource_id = (i + 1) as u32;
            model.push_str(&format!(
                "  <item objectid=\"{resource_id}\" \
                 transform=\"1 0 0 0 1 0 0 0 1 0 0 0\"/>\n"
            ));
        }
        model.push_str(" </build>\n");
        model.push_str("</model>\n");

        zip.write_all(model.as_bytes())?;
    }

    // ── Metadata/model_settings.config ───────────────────────────────────────
    zip.start_file("Metadata/model_settings.config", options)?;
    {
        let mut cfg = String::new();
        cfg.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        cfg.push_str("<config>\n");

        for (i, obj) in mesh.objects.iter().enumerate() {
            let resource_id = (i + 1) as u32;
            let name = xml_escape_attr(&obj.id);
            cfg.push_str(&format!(" <object id=\"{resource_id}\">\n"));
            cfg.push_str(&format!("  <metadata key=\"name\" value=\"{name}\"/>\n"));
            cfg.push_str(&format!(
                "  <part id=\"{resource_id}\" subtype=\"normal_part\">\n"
            ));
            cfg.push_str(&format!("   <metadata key=\"name\" value=\"{name}\"/>\n"));
            cfg.push_str(
                "   <metadata key=\"matrix\" value=\"1 0 0 0 0 1 0 0 0 0 1 0 0 0 0 1\"/>\n",
            );
            cfg.push_str(
                "   <mesh_stat edges_fixed=\"0\" degenerate_facets=\"0\" \
                 facets_removed=\"0\" facets_reversed=\"0\" backwards_edges=\"0\"/>\n",
            );
            cfg.push_str("  </part>\n");
            cfg.push_str(" </object>\n");
        }

        cfg.push_str("</config>\n");
        zip.write_all(cfg.as_bytes())?;
    }

    zip.finish()?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// OBJ writer
// ─────────────────────────────────────────────────────────────────────────────

/// Write a `MeshIR` as a Wavefront OBJ file.
///
/// One `o <name>` group line is emitted per `mesh.objects[i]`. Vertices are in
/// millimetres (shortest round-trip `f32` formatting). Face indices are
/// 1-based and global across the file (OBJ convention).
pub fn write_obj(mesh: &MeshIR, writer: &mut impl Write) -> io::Result<()> {
    let mut vertex_offset: u32 = 0;

    for obj in mesh.objects.iter() {
        writeln!(writer, "o {}", obj.id)?;

        // vertices
        for v in &obj.mesh.vertices {
            writeln!(writer, "v {} {} {}", v.x, v.y, v.z)?;
        }

        // faces (1-based, global vertex numbering)
        let tri_count = obj.mesh.indices.len() / 3;
        for t in 0..tri_count {
            let v1 = obj.mesh.indices[t * 3] + vertex_offset + 1;
            let v2 = obj.mesh.indices[t * 3 + 1] + vertex_offset + 1;
            let v3 = obj.mesh.indices[t * 3 + 2] + vertex_offset + 1;
            writeln!(writer, "f {v1} {v2} {v3}")?;
        }

        vertex_offset += obj.mesh.vertices.len() as u32;
    }

    Ok(())
}
