//! Explicit SCAD ingestion contracts (AC-2, AC-4, AC-N2, AC-N3 model-I/O half).
//!
//! Every fixture is a real, small, self-contained script evaluated by the pinned
//! adapter through `load_scad_model`. Expected bounds and volumes are computed
//! from the declared dimensions, never from observed output.
//!
//! Numeric tolerances are derived from the adapter's output precision: it writes
//! `f32` vertices, so coordinates are compared with a 1e-3 mm tolerance and
//! volumes with a 1e-2 mm³ tolerance.

use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use slicer_ir::slice_ir::{IndexedTriangleSet, Point3};
use slicer_model_io::{load_scad_model, ModelLoadError, ScadEvalOptions};
use slicer_scad::{ScadDiagnosticClass, ScadError, ScadErrorKind, ScadLimits, ScadParameter};
use tempfile::TempDir;

const COORD_TOL_MM: f32 = 1e-3;
const VOLUME_TOL_MM3: f64 = 1e-2;

/// RFC 4122 NAMESPACE_OID bytes, copied literally so the test computes the id
/// independently of the loader's private constant.
const NAMESPACE_OID_BYTES: [u8; 16] = [
    0x6b, 0xa7, 0xb8, 0x14, 0x9d, 0xad, 0x11, 0xd1, 0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30, 0xc8,
];

fn write_scad(dir: &TempDir, name: &str, text: &str) -> PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, text).expect("write SCAD fixture");
    path
}

/// Save a reproducer under `target/scad-ingest/reproducers/` **before** it is
/// executed, so a failure or abort leaves the exact source behind.
///
/// `target/` is gitignored: this is a scratch record for the run, not a
/// committed fixture. Renderer-stress cases are executed in a supervised child
/// (`supervise_case`); the adapter itself promises no in-process render deadline
/// (see `crates/slicer-scad/tests/scad_process_limits.rs`).
fn save_reproducer(name: &str, text: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/slicer-model-io has a parent")
        .parent()
        .expect("workspace root above crates/")
        .to_path_buf()
        .join("target")
        .join("scad-ingest")
        .join("reproducers");
    std::fs::create_dir_all(&root).expect("create reproducer directory");
    let path = root.join(name);
    std::fs::write(&path, text).expect("save reproducer");
    path
}

/// Record the options a dangerous fixture ran under, next to its source.
fn save_options(name: &str, options: &ScadEvalOptions) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/slicer-model-io has a parent")
        .parent()
        .expect("workspace root above crates/")
        .to_path_buf()
        .join("target")
        .join("scad-ingest")
        .join("reproducers");
    std::fs::create_dir_all(&root).expect("create reproducer directory");
    let text = format!(
        "parameters={:?}\nextrude_height_mm={:?}\nlimits: evaluation_steps={} \
         max_source_bytes={} max_ast_nodes={} max_ast_depth={} max_geometry_nodes={} \
         max_facets={} max_vertices={} max_triangles={} max_abs_coordinate_mm={} \
         worker_stack_bytes={}\n\
         random_seed={:?}\n",
        options.parameters,
        options.extrude_height_mm,
        options.limits.evaluation_steps,
        options.limits.max_source_bytes,
        options.limits.max_ast_nodes,
        options.limits.max_ast_depth,
        options.limits.max_geometry_nodes,
        options.limits.max_facets,
        options.limits.max_vertices,
        options.limits.max_triangles,
        options.limits.max_abs_coordinate_mm,
        options.limits.worker_stack_bytes,
        options.random_seed,
    );
    std::fs::write(root.join(name), text).expect("save options record");
}

/// Environment variable selecting the supervised-child case.
const CHILD_CASE_ENV: &str = "PNP_SCAD_MODEL_INGEST_CASE";

/// Containment deadline for one supervised child.
///
/// The adapter promises no in-process render deadline: `evaluate` spawns a
/// worker and joins it, and a native render cannot be interrupted from inside
/// the library (see `crates/slicer-scad/tests/scad_process_limits.rs` and
/// `docs/scad/adapter.md`). The fixtures whose purpose is to drive the renderer
/// into a degradation/approximation/conversion failure are therefore executed in
/// a killable child, exactly as the prerequisite does. This is containment, not
/// a performance budget.
const CHILD_DEADLINE: Duration = Duration::from_secs(180);

fn child_out_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/slicer-model-io has a parent")
        .parent()
        .expect("workspace root above crates/")
        .to_path_buf()
        .join("target")
        .join("scad-ingest")
        .join("scad_model_ingest")
}

/// Source for a supervised case.
fn child_case_source(case: &str) -> &'static str {
    match case {
        "partial" => {
            "function cum(list, k) = (k == 0) ? 0 : cum(list, k - 1) + list[k - 1];\
             cube(3); cube(cum([5], -1)); cube(7);"
        }
        "degraded" => {
            "union(){ cube(10); polyhedron(points=[[0,0,0],[1,0,0],[0,1,0]], faces=[[0,1,2]]); }"
        }
        "approximate" => "roof(){ difference(){ square(10); translate([3,3]) square(4); } }",
        "quantization" => "union(){ square(1); translate([1.00002, 0]) square(1); }",
        other => panic!("unknown supervised case {other:?}"),
    }
}

/// Options for a supervised case. Identical to the defaults on purpose: these
/// fixtures are rejected on their geometry, not by a caller-supplied limit.
fn child_case_options(_case: &str) -> ScadEvalOptions {
    ScadEvalOptions::default()
}

/// Save a case's source and the exact options it will run under before the
/// evaluation, and return the saved source path.
fn record_case(case: &str, source: &str, options: &ScadEvalOptions) -> PathBuf {
    let path = save_reproducer(&format!("{case}.scad"), source);
    save_options(&format!("{case}.options.txt"), options);
    path
}

/// One supervised child run.
struct SupervisedOutcome {
    status: Option<i32>,
    reaped: bool,
    stdout: String,
    stderr: String,
}

/// Spawn this test binary with `--exact scad_model_ingest_child_entry`, supervise
/// it with a hard deadline, and kill it if the deadline expires.
fn supervise_case(case: &str) -> SupervisedOutcome {
    let exe = std::env::current_exe().expect("current test executable");
    let out_dir = child_out_dir();
    std::fs::create_dir_all(&out_dir).expect("create supervised child directory");
    let out_path = out_dir.join(format!("{case}.stdout.txt"));
    let err_path = out_dir.join(format!("{case}.stderr.txt"));
    let stdout = std::fs::File::create(&out_path).expect("create child stdout file");
    let stderr = std::fs::File::create(&err_path).expect("create child stderr file");

    let mut child = Command::new(exe)
        .args([
            "--exact",
            "scad_model_ingest_child_entry",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD_CASE_ENV, case)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .expect("spawn supervised child");

    let started = Instant::now();
    let mut timed_out = false;
    let status = loop {
        match child.try_wait().expect("poll child") {
            Some(status) => break status,
            None if started.elapsed() >= CHILD_DEADLINE => {
                timed_out = true;
                let _ = child.kill();
                break child.wait().expect("reap killed child");
            }
            // The sleep *is* the deadline mechanism, not evidence: every
            // assertion below is about the child's reported outcome, its exit
            // status or whether it was reaped. Same pattern as
            // `crates/slicer-scad/tests/scad_process_limits.rs`.
            None => std::thread::sleep(Duration::from_millis(20)), // test-quality: deadline enforcement in a process supervisor; no assertion infers correctness from elapsed time
        }
    };
    let reaped = child.try_wait().expect("final poll").is_some();

    let mut stdout_text = String::new();
    std::fs::File::open(&out_path)
        .expect("open child stdout")
        .read_to_string(&mut stdout_text)
        .expect("read child stdout");
    let mut stderr_text = String::new();
    std::fs::File::open(&err_path)
        .expect("open child stderr")
        .read_to_string(&mut stderr_text)
        .expect("read child stderr");

    assert!(
        !timed_out,
        "supervised case {case:?} exceeded {CHILD_DEADLINE:?} and was killed; \
         artifacts: {} and {}\nstderr:\n{stderr_text}",
        out_path.display(),
        err_path.display()
    );
    assert!(reaped, "supervised case {case:?} must be reaped");
    SupervisedOutcome {
        status: status.code(),
        reaped,
        stdout: stdout_text,
        stderr: stderr_text,
    }
}

/// Read a `NAME=value` line out of a child's report.
fn field<'a>(report: &'a str, name: &str) -> Option<&'a str> {
    // The child prints through libtest, so fields are prefixed with a sentinel
    // rather than relying on line starts (the harness writes the test name and
    // its "... " separator onto the same line).
    let needle = format!("@PNP@{name}=");
    let at = report.rfind(&needle)? + needle.len();
    let rest = &report[at..];
    let end = rest.find(['\r', '\n']).unwrap_or(rest.len());
    Some(&rest[..end])
}

/// Assert a case was rejected with the expected typed kind, diagnostic codes and
/// display fragments, as reported by the supervised child.
fn assert_supervised_rejection(case: &str, kind: &str, codes: &[&str], display_fragments: &[&str]) {
    let outcome = supervise_case(case);
    assert!(
        outcome.reaped,
        "supervised case {case:?} must be reaped, not left running"
    );
    assert_eq!(
        outcome.status,
        Some(0),
        "supervised case {case:?} must report its evaluation outcome; stderr:\n{}",
        outcome.stderr
    );
    let report = &outcome.stdout;
    assert_eq!(
        field(report, "OUTCOME"),
        Some("error"),
        "supervised case {case:?} must report a rejection, got:\n{report}"
    );
    assert_eq!(
        field(report, "KIND"),
        Some(kind),
        "supervised case {case:?} must preserve the typed kind, got:\n{report}"
    );
    let reported = field(report, "CODES").unwrap_or("");
    for code in codes {
        assert!(
            reported.split(',').any(|reported| reported == *code),
            "supervised case {case:?} must retain diagnostic code {code}, got {reported:?}"
        );
    }
    let display = field(report, "DISPLAY").unwrap_or("");
    for fragment in display_fragments {
        assert!(
            display.contains(fragment),
            "supervised case {case:?} display must carry {fragment:?}: {display}"
        );
    }
}

/// Child-only dispatcher. Selected with `--exact scad_model_ingest_child_entry`.
///
/// With the case variable unset this is an ordinary passing test that asserts the
/// dispatcher contract; the parent never relies on that path.
#[test]
fn scad_model_ingest_child_entry() {
    let Ok(case) = std::env::var(CHILD_CASE_ENV) else {
        assert!(
            std::env::var(CHILD_CASE_ENV).is_err(),
            "the dispatcher must not run as an ordinary test"
        );
        return;
    };
    let source = child_case_source(&case);
    let options = child_case_options(&case);
    // The reproducer is on disk before the evaluation starts, so a killed child
    // still leaves enough to rerun it.
    let path = record_case(&case, source, &options);
    match load_scad_model(&path, &options) {
        Ok(_) => println!("@PNP@OUTCOME=ok"),
        Err(error) => {
            println!("@PNP@OUTCOME=error");
            println!(
                "@PNP@DISPLAY={}",
                error.to_string().replace(['\r', '\n'], " ")
            );
            if let ModelLoadError::ScadEvaluation(inner) = &error {
                println!("@PNP@KIND={}", inner.kind.as_str());
                let mut codes: Vec<&str> =
                    inner.diagnostics.iter().map(|d| d.code.as_str()).collect();
                codes.sort_unstable();
                codes.dedup();
                println!("@PNP@CODES={}", codes.join(","));
                let mut stages: Vec<String> = inner
                    .diagnostics
                    .iter()
                    .map(|d| format!("{:?}", d.stage).to_ascii_lowercase())
                    .collect();
                stages.sort();
                stages.dedup();
                println!("@PNP@STAGES={}", stages.join(","));
                println!(
                    "@PNP@PARAMETER={}",
                    inner.parameter.clone().unwrap_or_default()
                );
                println!("@PNP@SOURCE_LABEL={}", inner.source_label);
            }
        }
    }
}

/// Axis-aligned bounds of an indexed triangle set.
fn bounds(mesh: &IndexedTriangleSet) -> (Point3, Point3) {
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
    for vertex in &mesh.vertices {
        min.x = min.x.min(vertex.x);
        min.y = min.y.min(vertex.y);
        min.z = min.z.min(vertex.z);
        max.x = max.x.max(vertex.x);
        max.y = max.y.max(vertex.y);
        max.z = max.z.max(vertex.z);
    }
    assert!(min.x.is_finite(), "a loaded mesh must have vertices");
    (min, max)
}

/// Signed mesh volume in mm³: the tetrahedron sum over every triangle.
fn volume_mm3(mesh: &IndexedTriangleSet) -> f64 {
    let mut sum = 0.0f64;
    for triangle in mesh.indices.chunks_exact(3) {
        let a = &mesh.vertices[triangle[0] as usize];
        let b = &mesh.vertices[triangle[1] as usize];
        let c = &mesh.vertices[triangle[2] as usize];
        let (ax, ay, az) = (a.x as f64, a.y as f64, a.z as f64);
        let (bx, by, bz) = (b.x as f64, b.y as f64, b.z as f64);
        let (cx, cy, cz) = (c.x as f64, c.y as f64, c.z as f64);
        sum += ax * (by * cz - bz * cy) - ay * (bx * cz - bz * cx) + az * (bx * cy - by * cx);
    }
    sum / 6.0
}

fn options(parameters: &[(&str, &str)], extrude: Option<f64>) -> ScadEvalOptions {
    ScadEvalOptions {
        parameters: parameters
            .iter()
            .map(|(name, expression)| ScadParameter {
                name: (*name).to_string(),
                expression: (*expression).to_string(),
            })
            .collect(),
        extrude_height_mm: extrude,
        ..ScadEvalOptions::default()
    }
}

fn expected_id(basename: &str) -> String {
    let namespace = uuid::Uuid::from_bytes(NAMESPACE_OID_BYTES);
    let name = format!("{basename}#0");
    uuid::Uuid::new_v5(&namespace, name.as_bytes()).to_string()
}

fn assert_coord(actual: f32, expected: f32, what: &str) {
    assert!(
        (actual - expected).abs() <= COORD_TOL_MM,
        "{what}: expected {expected} mm, got {actual} mm (tolerance {COORD_TOL_MM} mm)"
    );
}

fn assert_volume(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() <= VOLUME_TOL_MM3,
        "{what}: expected {expected} mm3, got {actual} mm3 (tolerance {VOLUME_TOL_MM3} mm3)"
    );
}

fn assert_identity(matrix: &[f64; 16]) {
    let identity = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    assert_eq!(
        *matrix, identity,
        "assemble_object must leave identity transform"
    );
}

fn assert_aabb(
    min: Point3,
    max: Point3,
    expected_min: (f32, f32, f32),
    expected_max: (f32, f32, f32),
) {
    assert_coord(min.x, expected_min.0, "bounds min x");
    assert_coord(min.y, expected_min.1, "bounds min y");
    assert_coord(min.z, expected_min.2, "bounds min z");
    assert_coord(max.x, expected_max.0, "bounds max x");
    assert_coord(max.y, expected_max.1, "bounds max y");
    assert_coord(max.z, expected_max.2, "bounds max z");
}

fn expect_eval(err: &ModelLoadError) -> &ScadError {
    match err {
        ModelLoadError::ScadEvaluation(error) => error,
        other => panic!("expected ModelLoadError::ScadEvaluation, got {other:?}"),
    }
}

#[test]
fn scad_model_one_object_source_identity() {
    let boolean_source = "difference() { cube([12,10,6]); translate([4,3,-1]) cube([4,4,8]); }";
    let dir = tempfile::tempdir().expect("temp dir");
    let path = write_scad(&dir, "boolean.scad", boolean_source);

    let loaded =
        load_scad_model(&path, &ScadEvalOptions::default()).expect("the Boolean fixture must load");
    assert_eq!(
        loaded.mesh.objects.len(),
        1,
        "SCAD ingestion must produce exactly one object"
    );

    let object = &loaded.mesh.objects[0];
    assert_eq!(
        object.id,
        expected_id("boolean.scad"),
        "identity is uuid5(NAMESPACE_OID, '<basename>#0') computed independently"
    );
    assert_identity(&object.transform.matrix);
    assert!(
        object.config.data.is_empty(),
        "SCAD carries no per-object config: {:?}",
        object.config.data
    );
    assert!(
        object.modifier_volumes.is_empty(),
        "SCAD ingestion must not invent modifier volumes"
    );
    assert!(
        object.paint_data.is_none(),
        "SCAD colors/groups must not become paint data"
    );

    let its = &object.mesh;
    assert!(!its.indices.is_empty(), "the Boolean must have geometry");
    assert_eq!(its.indices.len() % 3, 0, "indices must be whole triangles");
    assert!(
        its.indices
            .iter()
            .all(|&i| (i as usize) < its.vertices.len()),
        "every index must be in range"
    );

    let (min, max) = bounds(its);
    assert_aabb(min, max, (0.0, 0.0, 0.0), (12.0, 10.0, 6.0));
    assert_volume(
        volume_mm3(its),
        624.0,
        "(12*10 - 4*4) * 6 through-hole volume",
    );

    match object.world_z_extent {
        Some((lo, hi)) => {
            assert_coord(lo, 0.0, "world_z_extent min");
            assert_coord(hi, 6.0, "world_z_extent max");
        }
        None => panic!("the Boolean fixture must have a Z extent"),
    }
    let build_volume = &loaded.mesh.build_volume;
    assert_aabb(
        build_volume.min,
        build_volume.max,
        (0.0, 0.0, 0.0),
        (12.0, 10.0, 6.0),
    );

    // Repeated loads of the same file are stable.
    let again =
        load_scad_model(&path, &ScadEvalOptions::default()).expect("repeated load must succeed");
    let again_object = &again.mesh.objects[0];
    assert_eq!(object.id, again_object.id, "repeated loads keep the id");
    assert_eq!(
        object.mesh.vertices.len(),
        again_object.mesh.vertices.len(),
        "repeated loads keep the vertex count"
    );
    assert_eq!(
        object.mesh.indices.len(),
        again_object.mesh.indices.len(),
        "repeated loads keep the index count"
    );
    assert_eq!(
        bounds(&object.mesh),
        bounds(&again_object.mesh),
        "repeated loads keep the AABB"
    );

    // A different directory with the same basename keeps id and geometry:
    // identity is the basename, not the absolute path.
    let other_dir = tempfile::tempdir().expect("second temp dir");
    let other_path = write_scad(&other_dir, "boolean.scad", boolean_source);
    let other =
        load_scad_model(&other_path, &ScadEvalOptions::default()).expect("second directory load");
    assert_eq!(
        object.id, other.mesh.objects[0].id,
        "same basename in a different directory must keep the id"
    );
    assert_eq!(
        object.mesh, other.mesh.objects[0].mesh,
        "same source must keep the geometry regardless of directory"
    );

    // Two separated translated boxes remain one unplaced object.
    let separated_source = "translate([-30,5,-4]) cube([4,6,8]); \
                            translate([-20,7,-2]) cube([2,2,4]);";
    let separated_path = write_scad(&dir, "separated.scad", separated_source);
    let separated = load_scad_model(&separated_path, &ScadEvalOptions::default())
        .expect("the disconnected fixture must load");
    assert_eq!(
        separated.mesh.objects.len(),
        1,
        "disconnected bodies stay in one object"
    );
    let separated_object = &separated.mesh.objects[0];
    let (min, max) = bounds(&separated_object.mesh);
    assert_aabb(min, max, (-30.0, 5.0, -4.0), (-18.0, 11.0, 4.0));
    assert_volume(
        volume_mm3(&separated_object.mesh),
        208.0,
        "4*6*8 + 2*2*4 separated-box volume",
    );
    match separated_object.world_z_extent {
        Some((lo, hi)) => {
            assert_coord(lo, -4.0, "unplaced Z minimum");
            assert_coord(hi, 4.0, "world_z_extent max");
        }
        None => panic!("the disconnected fixture must have a Z extent"),
    }
    let build_volume = &separated.mesh.build_volume;
    assert_aabb(
        build_volume.min,
        build_volume.max,
        (-30.0, 5.0, -4.0),
        (-18.0, 11.0, 4.0),
    );
}

#[test]
fn scad_model_explicit_holed_extrusion() {
    let dir = tempfile::tempdir().expect("temp dir");
    let source = "difference() { square([12,10]); translate([4,3]) square([4,4]); }";
    let path = write_scad(&dir, "holed-2d.scad", source);

    let loaded = load_scad_model(&path, &options(&[], Some(2.0)))
        .expect("an explicit height must extrude the holed profile");
    assert_eq!(loaded.mesh.objects.len(), 1);

    let its = &loaded.mesh.objects[0].mesh;
    let (min, max) = bounds(its);
    assert_aabb(min, max, (0.0, 0.0, 0.0), (12.0, 10.0, 2.0));
    // The hole must survive: the volume is (outer - hole) * height, not the
    // solid-profile volume 120 * 2.
    assert_volume(
        volume_mm3(its),
        208.0,
        "(12*10 - 4*4) * 2 extruded holed volume",
    );

    match loaded.mesh.objects[0].world_z_extent {
        Some((lo, hi)) => {
            assert_coord(lo, 0.0, "extrusion Z minimum");
            assert_coord(hi, 2.0, "extrusion Z maximum");
        }
        None => panic!("the extruded fixture must have a Z extent"),
    }
}

#[test]
fn scad_model_rejects_dimensions_and_empty() {
    let dir = tempfile::tempdir().expect("temp dir");
    let holed_2d = "difference() { square([12,10]); translate([4,3]) square([4,4]); }";

    // 2D without an explicit height is never given implicit thickness.
    let path = write_scad(&dir, "holed-2d.scad", holed_2d);
    let err = load_scad_model(&path, &ScadEvalOptions::default())
        .expect_err("2D contours without a height must be rejected");
    match &err {
        ModelLoadError::ScadNeedsExtrusion { source_label } => {
            assert!(
                source_label.contains("holed-2d.scad"),
                "the source label must name the file: {source_label}"
            );
        }
        other => panic!("expected ScadNeedsExtrusion, got {other:?}"),
    }
    let text = err.to_string();
    assert!(
        text.contains("SCAD_NEEDS_EXTRUSION"),
        "display must carry SCAD_NEEDS_EXTRUSION: {text}"
    );
    assert!(
        text.contains("--scad-extrude-height"),
        "display must name the extrusion flag: {text}"
    );

    // A valid height on a 3D result is an adapter error, not an ignored option.
    let cube_path = write_scad(&dir, "cube3d.scad", "cube([2,2,2]);");
    let err = load_scad_model(&cube_path, &options(&[], Some(1.0)))
        .expect_err("a height on a 3D result must be rejected");
    let scad_error = expect_eval(&err);
    assert_eq!(
        scad_error.kind,
        ScadErrorKind::ExtrusionHeightOn3d,
        "the typed adapter kind must be preserved: {scad_error}"
    );
    let nested = std::error::Error::source(&err)
        .expect("source() must expose the nested ScadError")
        .downcast_ref::<ScadError>()
        .expect("the nested error must be the adapter's ScadError");
    assert_eq!(nested.kind, ScadErrorKind::ExtrusionHeightOn3d);
    let text = err.to_string();
    assert!(
        text.contains("SCAD_EVALUATION"),
        "display must carry SCAD_EVALUATION: {text}"
    );
    assert!(
        text.contains("extrusion_height_on_3d"),
        "display must carry the adapter kind name: {text}"
    );

    // Mixed 2D/3D content uses the pin-verified public reproducer.
    let mixed_path = write_scad(&dir, "mixed.scad", "union(){ cube(1); square(1); }");
    let err = load_scad_model(&mixed_path, &ScadEvalOptions::default())
        .expect_err("a mixed-dimension union must be rejected");
    assert_eq!(
        expect_eval(&err).kind,
        ScadErrorKind::MixedDimensions,
        "the adapter's mixed-dimension classification must survive ingestion"
    );

    // Empty geometry is empty even with a valid height (pin-verified reproducer).
    let empty_path = write_scad(&dir, "empty.scad", "%cube(1);");
    let err = load_scad_model(&empty_path, &ScadEvalOptions::default())
        .expect_err("empty geometry must be rejected");
    assert!(
        matches!(err, ModelLoadError::ScadEmpty { .. }),
        "expected ScadEmpty, got {err:?}"
    );
    assert!(
        err.to_string().contains("SCAD_EMPTY_MODEL"),
        "display must carry SCAD_EMPTY_MODEL: {err}"
    );
    let err = load_scad_model(&empty_path, &options(&[], Some(2.0)))
        .expect_err("a valid height must not turn empty geometry into a solid");
    assert!(
        matches!(err, ModelLoadError::ScadEmpty { .. }),
        "expected ScadEmpty with a height, got {err:?}"
    );

    // A parse error keeps the adapter's kind.
    let parse_path = write_scad(&dir, "parse.scad", "cube(");
    let err = load_scad_model(&parse_path, &ScadEvalOptions::default())
        .expect_err("a syntax error must be rejected");
    assert_eq!(
        expect_eval(&err).kind,
        ScadErrorKind::Parse,
        "the adapter parse classification must survive ingestion"
    );

    // An invalid override keeps the offending parameter name.
    let parameter_path = write_scad(&dir, "parameter.scad", "cube(1);");
    let err = load_scad_model(&parameter_path, &options(&[("nope", "(")], None))
        .expect_err("an invalid override expression must be rejected");
    let scad_error = expect_eval(&err);
    assert_eq!(scad_error.kind, ScadErrorKind::InvalidParameter);
    assert_eq!(
        scad_error.parameter.as_deref(),
        Some("nope"),
        "the offending parameter must be retained"
    );
}

#[test]
fn scad_model_preserves_strict_adapter_errors() {
    let dir = tempfile::tempdir().expect("temp dir");

    // A readable sibling primitive must not change the deny-all outcome.
    // Both files are saved under `target/scad-ingest/reproducers/` before
    // execution so the exact request is recoverable from a failure.
    save_options(
        "scad_model_preserves_strict_adapter_errors.default.options.txt",
        &ScadEvalOptions::default(),
    );
    let sibling = save_reproducer("tempting.scad", "cube(99);\n");
    let sibling_path = sibling.to_string_lossy().replace('\\', "/");
    let resource_source = format!("include <{sibling_path}>; cube(1);");
    let resource_path = save_reproducer("resource.scad", &resource_source);
    let err = load_scad_model(&resource_path, &ScadEvalOptions::default())
        .expect_err("an external resource request must be rejected");
    let scad_error = expect_eval(&err);
    assert_eq!(
        scad_error.kind,
        ScadErrorKind::ExternalResourceDenied,
        "the deny-all classification must survive ingestion"
    );
    assert!(
        scad_error
            .diagnostics
            .iter()
            .any(|d| d.code == "external_resource_denied"),
        "the denial must be retained as a diagnostic"
    );
    assert!(
        err.to_string().contains("SCAD_EVALUATION"),
        "display must carry SCAD_EVALUATION: {err}"
    );

    // Partial / degraded / approximate outcomes use the pin-verified public
    // reproducers and keep both the kind and its diagnostic code.
    // Each case runs in a supervised, killable child, because these fixtures
    // deliberately drive the native renderer to a partial/degraded/approximate
    // outcome and evaluate() cannot be interrupted from inside the library. The
    // child writes its reproducer and options before evaluating and reports the
    // typed outcome; the parent asserts on that report and on the reaping.
    for (case, kind, code) in [
        ("partial", "partial_output", "recursion_aborted"),
        ("degraded", "degraded_output", "boolean_fallback"),
        ("approximate", "approximate_output", "roof_omitted"),
    ] {
        assert_supervised_rejection(case, kind, &[code], &[case]);
    }

    // Over-limit source, custom bound: the loader refuses at the read bound,
    // before evaluation, and honours the caller's cap. The source stays valid
    // SCAD; only its byte count exceeds the cap.
    let long_source = format!("cube([2,2,2]);\n// {}\n", "padding ".repeat(16));
    assert!(long_source.len() > 64, "the fixture must exceed the cap");
    let over_limit = ScadEvalOptions {
        limits: ScadLimits {
            max_source_bytes: 64,
            ..ScadLimits::default()
        },
        ..ScadEvalOptions::default()
    };
    let long_path = save_reproducer("oversize.scad", &long_source);
    save_options("oversize.options.txt", &over_limit);
    let err = load_scad_model(&long_path, &over_limit)
        .expect_err("a source above max_source_bytes must be rejected");
    let scad_error = expect_eval(&err);
    assert_eq!(scad_error.kind, ScadErrorKind::ComplexityLimit);
    assert!(
        scad_error.message.contains("max_source_bytes"),
        "the failure must name the limit: {}",
        scad_error.message
    );
    assert!(
        scad_error.message.contains("64"),
        "the failure must name the cap: {}",
        scad_error.message
    );
    assert!(
        scad_error.message.contains("source is at least"),
        "the loader must fail at the bounded read, before evaluation: {}",
        scad_error.message
    );
    assert!(
        scad_error.diagnostics.is_empty(),
        "a pre-evaluation read refusal must not carry evaluator diagnostics"
    );

    // Over-limit source, **default** bound: this is the case a user reaches with
    // no flag at all, because the adapter's default `max_source_bytes` is
    // 1_048_576. The fixture is generated here rather than committed.
    let default_cap = ScadLimits::default().max_source_bytes;
    assert!(default_cap > 0 && default_cap != usize::MAX);
    let mut huge = String::from("cube([2,2,2]);\n");
    huge.push_str("//");
    while huge.len() <= default_cap + 1 {
        huge.push_str(" padding");
    }
    let huge_path = save_reproducer("oversize_default.scad", &huge);
    let err = load_scad_model(&huge_path, &ScadEvalOptions::default())
        .expect_err("a source above the default max_source_bytes must be rejected");
    let scad_error = expect_eval(&err);
    assert_eq!(scad_error.kind, ScadErrorKind::ComplexityLimit);
    assert!(
        scad_error.message.contains(&default_cap.to_string()),
        "the default-bound failure must name the default cap {default_cap}: {}",
        scad_error.message
    );

    // Post-quantization conversion damage, reachable on the public path. Two unit
    // squares 2e-5 mm apart are disjoint in raw f64 but the 1e-4 mm output grid
    // closes the gap; the adapter rejects rather than repairs. This is #3's own
    // reproducer (`scad_public_path_rejects_quantization_damage`), exercised here
    // through the ingestion boundary so the typed kind and retained diagnostic
    // survive it.
    // Post-quantization conversion damage, in a supervised child for the same
    // reason: it is a renderer/converter stress case.
    assert_supervised_rejection("quantization", "invalid_contours", &[], &["quantized grid"]);
    let quant = supervise_case("quantization");
    assert!(
        field(&quant.stdout, "STAGES")
            .unwrap_or("")
            .split(',')
            .any(|stage| stage == "conversion"),
        "the conversion rejection must be reported at the Conversion stage: {}",
        quant.stdout
    );

    // Out-of-range coordinates, reachable on the public path: the adapter's
    // default `max_abs_coordinate_mm` is 10_000, so a translated cube crosses it.
    let range_path = save_reproducer("coordinate_range.scad", "translate([10001,0,0]) cube(1);");
    let err = load_scad_model(&range_path, &ScadEvalOptions::default())
        .expect_err("a coordinate beyond max_abs_coordinate_mm must be rejected");
    let scad_error = expect_eval(&err);
    assert_eq!(scad_error.kind, ScadErrorKind::CoordinateOutOfRange);
    assert!(
        scad_error.message.contains("10001"),
        "the rejection must name the offending coordinate: {}",
        scad_error.message
    );

    // error-envelope evidence: `NonFiniteCoordinate` has no reproducer that is
    // reachable through the public path at this pin. Probed and rejected as
    // *empty* rather than non-finite: `cube([0/0,1,1])`, `cube([1/0,1,1])`,
    // `cube([1e400,1,1])` all evaluate to no geometry. So this pins
    // ModelLoadError wrapping, source() and Display; the adapter-side rejection
    // is proved in slicer-scad's private conversion tests.
    // exhaustive: every ScadError field is under test; a FRU default would import evaluator state.
    let constructed = ScadError {
        kind: ScadErrorKind::NonFiniteCoordinate,
        source_label: "env.scad".into(),
        parameter: Some("sz".into()),
        message: "constructed envelope check".into(),
        diagnostics: vec![],
    };
    let err = ModelLoadError::ScadEvaluation(constructed);
    let text = err.to_string();
    for fragment in ["SCAD_EVALUATION", "non_finite_coordinate", "env.scad", "sz"] {
        assert!(
            text.contains(fragment),
            "the envelope display must carry {fragment}: {text}"
        );
    }
    let nested = std::error::Error::source(&err)
        .expect("source() must expose a constructed adapter error")
        .downcast_ref::<ScadError>()
        .expect("the nested error must be the adapter's ScadError");
    assert_eq!(nested.kind, ScadErrorKind::NonFiniteCoordinate);

    // A successful load passes accepted information and provenance through
    // unmodified.
    let info_source = "echo(\"scad information\"); cube([2,2,2]);";
    let info_path = write_scad(&dir, "info.scad", info_source);
    let loaded = load_scad_model(&info_path, &ScadEvalOptions::default())
        .expect("an informational source must load");
    assert!(!loaded.diagnostics.is_empty(), "the echo must be retained");
    assert!(
        loaded
            .diagnostics
            .iter()
            .all(|d| d.class == ScadDiagnosticClass::Information),
        "a successful load may only carry information diagnostics: {:?}",
        loaded.diagnostics
    );
    assert!(
        loaded.diagnostics.iter().any(|d| d.code == "scad_echo"),
        "the recognized scad_echo code must be preserved"
    );
    assert_eq!(
        loaded.provenance.source_label,
        info_path.display().to_string(),
        "provenance must carry the original path label"
    );
    assert!(
        loaded.provenance.parameters.is_empty(),
        "no overrides were supplied: {:?}",
        loaded.provenance.parameters
    );
}
