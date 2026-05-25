//! Implementation of the `repair`, `decimate`, and `import` subcommands.
//!
//! These wrap the library entry points in `slicer-helpers` with on-disk mesh
//! I/O, exit-code mapping, and optional JSON-Lines stats events on stderr per
//! `docs/13_slicer_helpers_crate.md` §Integration.

use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use slicer_helpers::{
    decimate, import_step_with_options, merge_step_meshes, repair, DecimateConfigBuilder,
    DecimateError, RepairError, RepairWarning, StepImportError, StepImportOptions, StepWarning,
};
use slicer_ir::MeshIR;

use slicer_host::model_loader::load_model;
use slicer_host::OutputFormat;

mod exit_codes {
    pub const SUCCESS: i32 = 0;
    pub const WARNINGS_OR_PARTIAL: i32 = 1;
    pub const UNREADABLE: i32 = 2;
    pub const EMPTY_OR_TRIVIAL: i32 = 3;
    pub const PARSE_ERROR: i32 = 4;
}

/// Run the `repair` subcommand. Returns the process exit code per
/// `docs/13_slicer_helpers_crate.md` §Repair exit-code table.
pub fn run_repair(input: &Path, output: &Path, format: Option<OutputFormat>, stats: bool) -> i32 {
    if !input.exists() {
        eprintln!("error: input file not found: {}", input.display());
        return exit_codes::UNREADABLE;
    }
    let resolved_format = match resolve_output_format(format, output, Some(input)) {
        Ok(f) => f,
        Err(msg) => {
            eprintln!("error: {msg}");
            return exit_codes::UNREADABLE;
        }
    };

    if stats {
        emit_event(json!({
            "event": "start",
            "operation": "repair",
            "input": input.display().to_string(),
            "output": output.display().to_string(),
        }));
    }

    let mesh = match load_model(input) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: failed to load input mesh: {e}");
            return exit_codes::UNREADABLE;
        }
    };

    let total_tris: usize = mesh.objects.iter().map(|o| o.mesh.indices.len() / 3).sum();
    if total_tris == 0 {
        eprintln!("error: input mesh has zero triangles");
        return exit_codes::EMPTY_OR_TRIVIAL;
    }

    let result = match repair(mesh) {
        Ok(r) => r,
        Err(RepairError::EmptyMesh) => {
            eprintln!("error: input mesh is empty");
            return exit_codes::EMPTY_OR_TRIVIAL;
        }
        Err(e) => {
            eprintln!("error: repair failed: {e}");
            return exit_codes::UNREADABLE;
        }
    };

    if stats {
        for w in &result.stats.warnings {
            emit_event(json!({
                "event": "warning",
                "operation": "repair",
                "kind": repair_warning_kind(w),
                "detail": repair_warning_detail(w),
            }));
        }
    }

    if let Err(e) = write_mesh(&result.mesh, output, resolved_format) {
        eprintln!("error: failed to write output mesh: {e}");
        return exit_codes::UNREADABLE;
    }

    if stats {
        let warning_kinds: Vec<&'static str> = result
            .stats
            .warnings
            .iter()
            .map(repair_warning_kind)
            .collect();
        emit_event(json!({
            "event": "done",
            "operation": "repair",
            "degenerate_removed": result.stats.degenerate_removed,
            "faces_reoriented": result.stats.faces_reoriented,
            "open_edges_closed": result.stats.open_edges_closed,
            "components": result.stats.components,
            "warnings": warning_kinds,
        }));
    }

    if result.stats.warnings.is_empty() {
        exit_codes::SUCCESS
    } else {
        exit_codes::WARNINGS_OR_PARTIAL
    }
}

/// Run the `decimate` subcommand. Returns the process exit code per
/// `docs/13_slicer_helpers_crate.md` §Decimate exit-code table.
pub fn run_decimate(
    input: &Path,
    output: &Path,
    target_count: Option<usize>,
    target_ratio: Option<f32>,
    max_error: f32,
    aggressive: bool,
    stats: bool,
) -> i32 {
    if !input.exists() {
        eprintln!("error: input file not found: {}", input.display());
        return exit_codes::UNREADABLE;
    }
    let resolved_format = match resolve_output_format(None, output, Some(input)) {
        Ok(f) => f,
        Err(msg) => {
            eprintln!("error: {msg}");
            return exit_codes::UNREADABLE;
        }
    };

    if stats {
        emit_event(json!({
            "event": "start",
            "operation": "decimate",
            "input": input.display().to_string(),
            "output": output.display().to_string(),
        }));
    }

    let mesh = match load_model(input) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: failed to load input mesh: {e}");
            return exit_codes::UNREADABLE;
        }
    };

    let total_tris: usize = mesh.objects.iter().map(|o| o.mesh.indices.len() / 3).sum();
    if total_tris == 0 {
        eprintln!("error: input mesh has zero triangles");
        return exit_codes::EMPTY_OR_TRIVIAL;
    }

    // Effective target — used both as exit-code-3 gate and for "target reached"
    // determination after decimate returns. clap's ArgGroup guarantees exactly
    // one of target_count/target_ratio is Some.
    let effective_target = if let Some(n) = target_count {
        if total_tris <= n {
            eprintln!("error: input has {total_tris} triangles, ≤ requested target {n}");
            return exit_codes::EMPTY_OR_TRIVIAL;
        }
        n
    } else {
        let ratio = target_ratio.expect("clap ArgGroup guarantees a target");
        ((total_tris as f32) * ratio).round().max(1.0) as usize
    };

    let mut builder = DecimateConfigBuilder::new()
        .max_error(max_error)
        .aggressive(aggressive);
    builder = if let Some(n) = target_count {
        builder.target_count(n)
    } else {
        builder.target_ratio(target_ratio.expect("clap ArgGroup guarantees a target"))
    };
    let config = match builder.build() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: invalid decimate configuration: {e}");
            return exit_codes::UNREADABLE;
        }
    };

    let result = match decimate(mesh, config) {
        Ok(r) => r,
        Err(DecimateError::EmptyMesh) => {
            eprintln!("error: input mesh is empty");
            return exit_codes::EMPTY_OR_TRIVIAL;
        }
        Err(e) => {
            eprintln!("error: decimate failed: {e}");
            return exit_codes::UNREADABLE;
        }
    };

    if let Err(e) = write_mesh(&result.mesh, output, resolved_format) {
        eprintln!("error: failed to write output mesh: {e}");
        return exit_codes::UNREADABLE;
    }

    let target_reached = result.final_triangle_count <= effective_target;

    if stats {
        emit_event(json!({
            "event": "done",
            "operation": "decimate",
            "original_triangle_count": result.original_triangle_count,
            "final_triangle_count": result.final_triangle_count,
            "achieved_error": result.achieved_error,
            "target_reached": target_reached,
        }));
    }

    if target_reached {
        exit_codes::SUCCESS
    } else {
        exit_codes::WARNINGS_OR_PARTIAL
    }
}

/// Run the `import` subcommand. Returns the process exit code per
/// `docs/13_slicer_helpers_crate.md` §Import exit-code table.
pub fn run_import(
    input: &Path,
    output: &Path,
    output_format: OutputFormat,
    merge_components: bool,
    no_repair: bool,
    stats: bool,
) -> i32 {
    if !input.exists() {
        eprintln!("error: input file not found: {}", input.display());
        return exit_codes::UNREADABLE;
    }

    if stats {
        emit_event(json!({
            "event": "start",
            "operation": "import",
            "input": input.display().to_string(),
            "output": output.display().to_string(),
        }));
    }

    let result = match import_step_with_options(
        input,
        StepImportOptions {
            skip_repair: no_repair,
        },
    ) {
        Ok(r) => r,
        Err(StepImportError::FileNotFound(_)) | Err(StepImportError::IoError(_)) => {
            eprintln!("error: input file not readable: {}", input.display());
            return exit_codes::UNREADABLE;
        }
        Err(StepImportError::NoGeometry) => {
            eprintln!("error: STEP file contains no recognisable geometry");
            return exit_codes::EMPTY_OR_TRIVIAL;
        }
        Err(StepImportError::ParseError(msg)) => {
            eprintln!("error: STEP parse error: {msg}");
            return exit_codes::PARSE_ERROR;
        }
    };

    let final_result = if merge_components {
        merge_step_meshes(result)
    } else {
        result
    };

    if final_result.meshes.is_empty() {
        eprintln!("error: STEP file produced zero meshes");
        return exit_codes::EMPTY_OR_TRIVIAL;
    }

    if stats {
        for w in &final_result.warnings {
            emit_event(json!({
                "event": "warning",
                "operation": "import",
                "kind": step_warning_kind(w),
                "detail": step_warning_detail(w),
            }));
        }
    }

    let mesh_count = final_result.meshes.len();
    if mesh_count == 1 {
        if let Err(e) = write_mesh(&final_result.meshes[0].mesh, output, output_format) {
            eprintln!("error: failed to write output mesh: {e}");
            return exit_codes::UNREADABLE;
        }
    } else {
        for (i, named) in final_result.meshes.iter().enumerate() {
            let path = derive_indexed_output(output, i);
            if let Err(e) = write_mesh(&named.mesh, &path, output_format) {
                eprintln!("error: failed to write output mesh {i}: {e}");
                return exit_codes::UNREADABLE;
            }
        }
    }

    if stats {
        let total_triangles: usize = final_result
            .meshes
            .iter()
            .flat_map(|n| n.mesh.objects.iter())
            .map(|o| o.mesh.indices.len() / 3)
            .sum();
        emit_event(json!({
            "event": "done",
            "operation": "import",
            "source_unit": format!("{:?}", final_result.source_unit),
            "mesh_count": mesh_count,
            "total_triangles": total_triangles,
            "warnings": final_result.warnings.len(),
        }));
    }

    if final_result.warnings.is_empty() {
        exit_codes::SUCCESS
    } else {
        exit_codes::WARNINGS_OR_PARTIAL
    }
}

// ─────────────────────────── helpers ───────────────────────────

fn emit_event(value: Value) {
    let stderr = io::stderr();
    let mut lock = stderr.lock();
    let _ = writeln!(lock, "{value}");
}

fn repair_warning_kind(w: &RepairWarning) -> &'static str {
    match w {
        RepairWarning::LargeCapLoop { .. } => "large_cap_loop",
        RepairWarning::MultipleComponents { .. } => "multiple_components",
    }
}

fn repair_warning_detail(w: &RepairWarning) -> Value {
    match w {
        RepairWarning::LargeCapLoop { vertex_count } => json!({ "vertex_count": vertex_count }),
        RepairWarning::MultipleComponents { count } => json!({ "count": count }),
    }
}

fn step_warning_kind(w: &StepWarning) -> &'static str {
    match w {
        StepWarning::UnsupportedSchema { .. } => "unsupported_schema",
        StepWarning::UnknownUnit => "unknown_unit",
        StepWarning::RepairApplied { .. } => "repair_applied",
        StepWarning::MultipleComponents { .. } => "multiple_components",
    }
}

fn step_warning_detail(w: &StepWarning) -> Value {
    match w {
        StepWarning::UnsupportedSchema { schema } => json!({ "schema": schema }),
        StepWarning::UnknownUnit => Value::Null,
        StepWarning::RepairApplied {
            component_index,
            stats,
        } => json!({
            "component_index": component_index,
            "degenerate_removed": stats.degenerate_removed,
            "faces_reoriented": stats.faces_reoriented,
            "open_edges_closed": stats.open_edges_closed,
        }),
        StepWarning::MultipleComponents { count } => json!({ "count": count }),
    }
}

/// Resolve the output format using (in priority order):
/// 1. Explicit `--format` flag, if present.
/// 2. The output path's extension.
/// 3. The input path's extension (fallback when output has no extension).
fn resolve_output_format(
    explicit: Option<OutputFormat>,
    output: &Path,
    input: Option<&Path>,
) -> Result<OutputFormat, String> {
    if let Some(f) = explicit {
        return Ok(f);
    }
    if let Some(f) = format_from_extension(output) {
        return Ok(f);
    }
    if let Some(input) = input {
        if let Some(f) = format_from_extension(input) {
            return Ok(f);
        }
    }
    Err(format!(
        "could not determine output format for {} — pass --format <stl|obj|3mf>",
        output.display()
    ))
}

fn format_from_extension(path: &Path) -> Option<OutputFormat> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase());
    match ext.as_deref() {
        Some("stl") => Some(OutputFormat::Stl),
        Some("obj") => Some(OutputFormat::Obj),
        Some("3mf") => Some(OutputFormat::ThreeMf),
        _ => None,
    }
}

/// `<stem>_<index>.<ext>` per `docs/13_slicer_helpers_crate.md` §Import multi-solid rule.
fn derive_indexed_output(base: &Path, index: usize) -> PathBuf {
    let ext = base.extension().and_then(|s| s.to_str());
    let stem = base.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
    let new_name = match ext {
        Some(e) => format!("{stem}_{index}.{e}"),
        None => format!("{stem}_{index}"),
    };
    match base.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.join(new_name),
        _ => PathBuf::from(new_name),
    }
}

fn write_mesh(mesh: &MeshIR, path: &Path, format: OutputFormat) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    match format {
        OutputFormat::Stl => {
            let file = std::fs::File::create(path)?;
            let mut w = BufWriter::new(file);
            write_stl_binary(mesh, &mut w)
        }
        OutputFormat::Obj | OutputFormat::ThreeMf => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "OBJ and 3MF output writers are not yet implemented; use --format stl",
        )),
    }
}

fn write_stl_binary(mesh: &MeshIR, w: &mut impl Write) -> io::Result<()> {
    let mut triangles: Vec<stl_io::Triangle> = Vec::new();
    for obj in &mesh.objects {
        let its = &obj.mesh;
        for t in 0..(its.indices.len() / 3) {
            let i0 = its.indices[t * 3] as usize;
            let i1 = its.indices[t * 3 + 1] as usize;
            let i2 = its.indices[t * 3 + 2] as usize;
            let v0 = &its.vertices[i0];
            let v1 = &its.vertices[i1];
            let v2 = &its.vertices[i2];

            let e1 = [v1.x - v0.x, v1.y - v0.y, v1.z - v0.z];
            let e2 = [v2.x - v0.x, v2.y - v0.y, v2.z - v0.z];
            let n = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];
            let mag = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            let normal = if mag > 0.0 {
                stl_io::Normal::new([n[0] / mag, n[1] / mag, n[2] / mag])
            } else {
                stl_io::Normal::new([0.0, 0.0, 1.0])
            };

            triangles.push(stl_io::Triangle {
                normal,
                vertices: [
                    stl_io::Vertex::new([v0.x, v0.y, v0.z]),
                    stl_io::Vertex::new([v1.x, v1.y, v1.z]),
                    stl_io::Vertex::new([v2.x, v2.y, v2.z]),
                ],
            });
        }
    }
    stl_io::write_stl(w, triangles.iter())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_indexed_output_with_extension() {
        let p = PathBuf::from("/tmp/foo/cube.stl");
        let r = derive_indexed_output(&p, 3);
        assert_eq!(r, PathBuf::from("/tmp/foo/cube_3.stl"));
    }

    #[test]
    fn derive_indexed_output_without_extension() {
        let p = PathBuf::from("cube");
        let r = derive_indexed_output(&p, 0);
        assert_eq!(r, PathBuf::from("cube_0"));
    }

    #[test]
    fn format_from_extension_recognises_lowercase_and_uppercase() {
        assert_eq!(
            format_from_extension(Path::new("a.STL")),
            Some(OutputFormat::Stl)
        );
        assert_eq!(
            format_from_extension(Path::new("a.obj")),
            Some(OutputFormat::Obj)
        );
        assert_eq!(
            format_from_extension(Path::new("a.3mf")),
            Some(OutputFormat::ThreeMf)
        );
        assert_eq!(format_from_extension(Path::new("a.step")), None);
    }

    #[test]
    fn resolve_output_format_priority_explicit_first() {
        let r = resolve_output_format(
            Some(OutputFormat::Stl),
            Path::new("out.obj"),
            Some(Path::new("in.3mf")),
        )
        .unwrap();
        assert_eq!(r, OutputFormat::Stl);
    }

    #[test]
    fn resolve_output_format_falls_back_to_input_extension() {
        let r = resolve_output_format(None, Path::new("out_no_ext"), Some(Path::new("in.stl")))
            .unwrap();
        assert_eq!(r, OutputFormat::Stl);
    }
}
