//! Acceptance tests for the explicit SCAD CLI path.
//!
//! This target requires the default-off `scad` feature. It covers `mesh import`
//! (evaluation through the explicit loader, export, reload in source
//! coordinates) and real `slice` runs: source geometry placed on the bed, then
//! sliced through the staged core guest modules with ordinary progress events.
//!
//! Geometry is compared semantically (AABB + signed volume), never by triangle
//! order: the writers are free to emit vertices in any order.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::time::{Duration, Instant};

use tempfile::tempdir;

use slicer_ir::{IndexedTriangleSet, MeshIR};

/// Coordinates round-trip through `f32` (`slicer_ir::Point3`) and through the
/// OBJ/3MF text writers. Every fixture value here is a multiple of 1 mm and the
/// expected volumes (208 mm³) are exactly representable in `f32`, so this
/// tolerance absorbs only text-format rounding, not modelling error.
const EPS_MM: f64 = 1e-4;

/// Backstop for one `pnp_cli` child process.
///
/// The adapter bounds its own evaluation (worker thread, step/geometry budgets,
/// proved in `crates/slicer-scad/tests/scad_process_limits.rs`). This deadline is
/// the frontend's own containment: a fixture that hangs the CLI must be killed
/// and reaped, not left to wedge the harness. It is a containment bound, not a
/// performance budget.
const CHILD_DEADLINE: Duration = Duration::from_secs(180);

/// A finished child process, shaped like `std::process::Output` for the helpers
/// below but produced by [`run_supervised`].
struct Captured {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

/// Run one `pnp_cli` invocation with a deadline, killing and reaping it if the
/// deadline expires.
///
/// `std::process::Command::output()` waits forever; a broken fixture or a
/// regression in the frontend would hang CI instead of failing it, so every
/// child in this target goes through here.
fn run_supervised(argv: &[OsString]) -> Captured {
    let exe = assert_cmd::cargo::cargo_bin("pnp_cli");
    let mut child = std::process::Command::new(&exe)
        .args(argv)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {} {argv:?}: {e}", exe.display()));

    let mut stdout_pipe = child.stdout.take().expect("piped child stdout");
    let mut stderr_pipe = child.stderr.take().expect("piped child stderr");
    let deadline = Instant::now() + CHILD_DEADLINE;

    // Drain both pipes concurrently: a child that fills a pipe would otherwise
    // block before exiting and be killed as if it had hung. Nothing is asserted
    // from these threads; they only prevent back-pressure from becoming a false
    // timeout.
    std::thread::scope(|scope| {
        let stdout_reader = scope.spawn(move || {
            let mut buffer = Vec::new();
            std::io::Read::read_to_end(&mut stdout_pipe, &mut buffer).expect("read child stdout");
            buffer
        });
        let stderr_reader = scope.spawn(move || {
            let mut buffer = Vec::new();
            std::io::Read::read_to_end(&mut stderr_pipe, &mut buffer).expect("read child stderr");
            buffer
        });

        let mut timed_out = false;
        let status = loop {
            match child.try_wait().expect("poll pnp_cli child") {
                Some(status) => break status,
                None if Instant::now() >= deadline => {
                    timed_out = true;
                    let _ = child.kill();
                    break child.wait().expect("reap killed pnp_cli child");
                }
                // The sleep *is* the deadline mechanism, not evidence: every
                // assertion in this target is about the child's exit status, its
                // captured streams or a published artifact. A gated target or an
                // explicit join cannot express "kill this child at a wall-clock
                // bound". Same pattern as
                // `crates/slicer-scad/tests/scad_process_limits.rs`.
                None => std::thread::sleep(Duration::from_millis(20)), // test-quality: deadline enforcement in a process supervisor; no assertion infers correctness from elapsed time
            }
        };

        // The pipes close when the child is gone (killed and reaped above), so
        // both readers finish. On the timeout path this guarantees the streams
        // are persisted before the panic below.
        let stdout = stdout_reader.join().expect("join child stdout reader");
        let stderr = stderr_reader.join().expect("join child stderr reader");

        if timed_out {
            let dir = artifacts_dir();
            std::fs::create_dir_all(&dir).expect("create scad_cli_ingest artifact directory");
            let stamp = std::process::id();
            let stderr_path = dir.join(format!("timeout-{stamp}.stderr.txt"));
            let stdout_path = dir.join(format!("timeout-{stamp}.stdout.txt"));
            std::fs::write(&stderr_path, &stderr).expect("save timed-out child stderr");
            std::fs::write(&stdout_path, &stdout).expect("save timed-out child stdout");
            panic!(
                "pnp_cli exceeded the {CHILD_DEADLINE:?} containment deadline and was killed \
                 ({status:?}) for argv {argv:?}\nartifacts: {} and {}\nstderr:\n{}",
                stderr_path.display(),
                stdout_path.display(),
                String::from_utf8_lossy(&stderr),
            );
        }

        Captured {
            status,
            stdout,
            stderr,
        }
    })
}

fn write_scad(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, text).expect("write SCAD fixture");
    path
}

/// Directory for retained fixture sources, the options they ran under and their
/// captured stderr.
///
/// `target/` is gitignored: these are run artifacts, not committed fixtures.
fn artifacts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/pnp-cli has a parent")
        .parent()
        .expect("workspace root above crates/")
        .join("target")
        .join("scad-ingest")
        .join("scad_cli_ingest")
}

/// Save a fixture and the exact extra arguments it will be run with **before**
/// executing it, so a kill leaves the input and its options recoverable.
fn record(name: &str, source: &str, extra: &[&str]) -> PathBuf {
    let dir = artifacts_dir();
    std::fs::create_dir_all(&dir).expect("create scad_cli_ingest artifact directory");
    let path = dir.join(name);
    std::fs::write(&path, source).expect("save fixture");
    std::fs::write(
        dir.join(format!("{name}.recorded_args.txt")),
        format!("extra_args={extra:?}\n"),
    )
    .expect("save fixture record");
    path
}

/// Record the exact extra arguments of one invocation, immediately before it
/// runs. `record` saves the source; this saves what that source is about to be
/// evaluated with, so two invocations of one fixture cannot share a stale record.
fn record_invocation(label: &str, extra: &[&str]) {
    let dir = artifacts_dir();
    std::fs::create_dir_all(&dir).expect("create scad_cli_ingest artifact directory");
    std::fs::write(
        dir.join(format!("{label}.options.txt")),
        format!("extra_args={extra:?}\n"),
    )
    .expect("save invocation options");
}

/// Retain a run's captured streams next to its fixture.
fn record_capture(name: &str, captured: &Captured) {
    let dir = artifacts_dir();
    std::fs::create_dir_all(&dir).expect("create scad_cli_ingest artifact directory");
    std::fs::write(dir.join(format!("{name}.stderr.txt")), &captured.stderr)
        .expect("save child stderr");
    std::fs::write(dir.join(format!("{name}.stdout.txt")), &captured.stdout)
        .expect("save child stdout");
    std::fs::write(
        dir.join(format!("{name}.exit.txt")),
        format!("{:?}\n", captured.status.code()),
    )
    .expect("save child exit status");
}

fn run_import_owned(argv: &[OsString]) -> Captured {
    let mut full: Vec<OsString> = vec!["mesh".into(), "import".into()];
    full.extend_from_slice(argv);
    run_supervised(&full)
}

fn import_argv(input: &Path, output: &Path, extra: &[&str]) -> Vec<OsString> {
    let mut argv: Vec<OsString> = vec![
        "--input".into(),
        input.as_os_str().to_os_string(),
        "--output".into(),
        output.as_os_str().to_os_string(),
    ];
    argv.extend(extra.iter().map(OsString::from));
    argv
}

fn stderr_text(output: &Captured) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn assert_exit(output: &Captured, expected: i32, context: &str) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{context}: expected exit {expected}, got {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        stderr_text(output),
    );
}

fn assert_stderr_contains(output: &Captured, needle: &str, context: &str) {
    let stderr = stderr_text(output);
    assert!(
        stderr.contains(needle),
        "{context}: stderr must contain {needle:?}\nstderr:\n{stderr}"
    );
}

fn assert_no_done_row(output: &Captured, context: &str) {
    let stderr = stderr_text(output);
    for line in stderr.lines() {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            assert_ne!(
                value["event"], "done",
                "{context}: a failed import must not publish a done row\nstderr:\n{stderr}"
            );
        }
    }
}

/// Reload a written mesh through the same non-evaluating loader every other
/// format uses.
fn reload(path: &Path) -> MeshIR {
    slicer_model_io::load_model(path).unwrap_or_else(|e| panic!("reload {}: {e}", path.display()))
}

fn volume_mm3(its: &IndexedTriangleSet) -> f64 {
    let mut signed = 0.0_f64;
    for triangle in its.indices.chunks_exact(3) {
        let a = &its.vertices[triangle[0] as usize];
        let b = &its.vertices[triangle[1] as usize];
        let c = &its.vertices[triangle[2] as usize];
        let (ax, ay, az) = (a.x as f64, a.y as f64, a.z as f64);
        let (bx, by, bz) = (b.x as f64, b.y as f64, b.z as f64);
        let (cx, cy, cz) = (c.x as f64, c.y as f64, c.z as f64);
        signed += ax * (by * cz - bz * cy) - ay * (bx * cz - bz * cx) + az * (bx * cy - by * cx);
    }
    (signed / 6.0).abs()
}

fn aabb(its: &IndexedTriangleSet) -> ([f64; 3], [f64; 3]) {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for vertex in &its.vertices {
        min[0] = min[0].min(vertex.x as f64);
        min[1] = min[1].min(vertex.y as f64);
        min[2] = min[2].min(vertex.z as f64);
        max[0] = max[0].max(vertex.x as f64);
        max[1] = max[1].max(vertex.y as f64);
        max[2] = max[2].max(vertex.z as f64);
    }
    (min, max)
}

fn assert_close(actual: f64, expected: f64, context: &str) {
    assert!(
        (actual - expected).abs() <= EPS_MM,
        "{context}: expected {expected}, got {actual} (tolerance {EPS_MM})"
    );
}

/// Assert a single-object mesh's AABB against analytic bounds.
fn assert_mesh_bounds(
    mesh: &MeshIR,
    expected_min: [f64; 3],
    expected_max: [f64; 3],
    context: &str,
) {
    let (min, max) = aabb(single_object(mesh, context));
    for axis in 0..3 {
        assert_close(
            min[axis],
            expected_min[axis],
            &format!("{context} min[{axis}]"),
        );
        assert_close(
            max[axis],
            expected_max[axis],
            &format!("{context} max[{axis}]"),
        );
    }
}

/// Assert the only file written for this output is the requested one: no
/// `out_0.stl` / `out_1.3mf` style split siblings.
fn assert_no_split_siblings(dir: &Path, stem: &str, extension: &str) {
    let split_prefix = format!("{stem}_");
    let expected_suffix = format!(".{extension}");
    for entry in std::fs::read_dir(dir).expect("read tempdir") {
        let path = entry.expect("dir entry").path();
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        assert!(
            !(name.starts_with(&split_prefix) && name.ends_with(&expected_suffix)),
            "unexpected split sibling written: {}",
            path.display()
        );
    }
}

fn single_object<'a>(mesh: &'a MeshIR, context: &str) -> &'a IndexedTriangleSet {
    assert_eq!(
        mesh.objects.len(),
        1,
        "{context}: SCAD import must produce one fused object"
    );
    &mesh.objects[0].mesh
}

// ---------------------------------------------------------------------------
// Real slice helpers
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // crates/
    path.pop(); // workspace root
    path
}

fn core_modules_dir() -> PathBuf {
    workspace_root().join("modules").join("core-modules")
}

/// True when at least one subdirectory of `dir` holds a staged `*.wasm` guest.
fn has_staged_guest(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let guest_dir = entry.path();
        if !guest_dir.is_dir() {
            continue;
        }
        let Ok(files) = std::fs::read_dir(&guest_dir) else {
            continue;
        };
        if files.flatten().any(|file| {
            file.path()
                .extension()
                .and_then(|extension| extension.to_str())
                == Some("wasm")
        }) {
            return true;
        }
    }
    false
}

/// Run the real `slice` verb against the staged core guest tree.
///
/// The module arguments are the current guest search convention
/// (`--module-dir <ws>/modules/core-modules --no-default-module-paths
/// --no-integrated-modules`), so the run never depends on platform default
/// search paths. The directory and at least one staged `*.wasm` guest must
/// exist: with a missing tree a module-loading failure would be
/// indistinguishable from a broken checkout. Artifact *freshness* is
/// `cargo xtask build-guests --check`'s job, not this test's.
fn run_slice(model: &Path, output: &Path, extra: &[&str]) -> Captured {
    let core = core_modules_dir();
    assert!(
        core.is_dir(),
        "core guest module directory must exist at {}",
        core.display()
    );
    assert!(
        has_staged_guest(&core),
        "at least one staged *.wasm guest must exist under {}",
        core.display()
    );

    let mut argv: Vec<OsString> = vec![
        "slice".into(),
        "--model".into(),
        model.as_os_str().to_os_string(),
        "--output".into(),
        output.as_os_str().to_os_string(),
        "--module-dir".into(),
        core.as_os_str().to_os_string(),
        "--no-default-module-paths".into(),
        "--no-integrated-modules".into(),
    ];
    argv.extend(extra.iter().map(OsString::from));
    run_supervised(&argv)
}

/// Parse the JSONL progress events on stderr. Plain `note:` / `error:` lines are
/// expected (accepted SCAD diagnostics and placement notes go there) and are
/// ignored; only well-formed JSON objects count as events.
fn slice_events(stderr: &str) -> Vec<serde_json::Value> {
    stderr
        .lines()
        .filter(|line| line.starts_with('{'))
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

fn run_slice_raw(model: &Path, output: &Path, extra: &[&str]) -> Captured {
    let mut argv: Vec<OsString> = vec![
        "slice".into(),
        "--model".into(),
        model.as_os_str().to_os_string(),
        "--output".into(),
        output.as_os_str().to_os_string(),
    ];
    argv.extend(extra.iter().map(OsString::from));
    run_supervised(&argv)
}

/// `slice` without `--output`: G-code would go to stdout, so this is the path
/// that must stay empty when ingestion fails.
fn run_slice_stdout(model: &Path, extra: &[&str]) -> Captured {
    let mut argv: Vec<OsString> = vec![
        "slice".into(),
        "--model".into(),
        model.as_os_str().to_os_string(),
    ];
    argv.extend(extra.iter().map(OsString::from));
    run_supervised(&argv)
}

fn assert_no_slice_complete(stderr: &str, context: &str) {
    for event in slice_events(stderr) {
        assert_ne!(
            event["event"], "slice_complete",
            "{context}: a failed ingestion must not publish slice_complete\nstderr:\n{stderr}"
        );
    }
}

/// Parse extrusion moves out of the G-code.
///
/// Only `G1` lines carrying a **positive** `E` token extrude; `X`/`Y`/`Z` are
/// absolute (`M83` makes only extrusion relative), so the move's position is the
/// last-seen value of each axis. Section filtering lets the caller skip
/// decoration (`;TYPE:Skirt`, `;TYPE:Brim`).
fn parse_extrusions(gcode: &str, include_section: impl Fn(&str) -> bool) -> Vec<(f64, f64, f64)> {
    let mut section = "";
    let mut last = [0.0_f64; 3];
    let mut moves = Vec::new();
    for line in gcode.lines() {
        let line = line.trim();
        if let Some(name) = line.strip_prefix(";TYPE:") {
            section = name.trim();
            continue;
        }
        if !line.starts_with("G1 ") || !include_section(section) {
            continue;
        }
        let mut extrudes = false;
        for token in line.split_whitespace().skip(1) {
            let Some(axis) = token.chars().next() else {
                continue;
            };
            let Ok(value) = token[axis.len_utf8()..].parse::<f64>() else {
                continue;
            };
            match axis {
                'X' => last[0] = value,
                'Y' => last[1] = value,
                'Z' => last[2] = value,
                'E' if value > 0.0 => extrudes = true,
                _ => {}
            }
        }
        if extrudes {
            moves.push((last[0], last[1], last[2]));
        }
    }
    moves
}

/// Every extruding move in the file, decoration included.
fn extrusions(gcode: &str) -> Vec<(f64, f64, f64)> {
    parse_extrusions(gcode, |_| true)
}

/// The part's own extruding moves: skirt and brim are generated around the first
/// region's footprint, not the whole model, so they are excluded from the
/// centring assertion.
fn part_extrusions(gcode: &str) -> Vec<(f64, f64, f64)> {
    parse_extrusions(gcode, |section| section != "Skirt" && section != "Brim")
}

fn bounds3(points: &[(f64, f64, f64)]) -> ([f64; 3], [f64; 3]) {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for (x, y, z) in points {
        min[0] = min[0].min(*x);
        min[1] = min[1].min(*y);
        min[2] = min[2].min(*z);
        max[0] = max[0].max(*x);
        max[1] = max[1].max(*y);
        max[2] = max[2].max(*z);
    }
    (min, max)
}

/// Parse `; max_z_height: <mm>` from the G-code header.
fn max_z_height(gcode: &str) -> f64 {
    gcode
        .lines()
        .find_map(|line| line.trim().strip_prefix("; max_z_height:"))
        .map(|value| {
            value
                .trim()
                .parse::<f64>()
                .unwrap_or_else(|e| panic!("parse max_z_height {value:?}: {e}"))
        })
        .expect("G-code header must carry `; max_z_height:`")
}

/// Parse `; total layer number: N` from the G-code header.
fn total_layer_number(gcode: &str) -> usize {
    gcode
        .lines()
        .find_map(|line| line.trim().strip_prefix("; total layer number:"))
        .map(|value| {
            value
                .trim()
                .parse::<usize>()
                .unwrap_or_else(|e| panic!("parse total layer number {value:?}: {e}"))
        })
        .expect("G-code header must carry `; total layer number:`")
}

/// Analytic whole-model placement: bed centre from `bed_shape`, then one rigid
/// translation of the unplaced AABB. Expectations are never read back from a
/// placed result.
fn placed_aabb(
    unplaced_min: [f64; 3],
    unplaced_max: [f64; 3],
    bed_shape: &[f64],
) -> ([f64; 3], [f64; 3]) {
    let (centre_x, centre_y) =
        slicer_model_io::bed_center_mm(bed_shape).expect("bed_shape describes a plate");
    let dx = f64::from(centre_x) - (unplaced_min[0] + unplaced_max[0]) / 2.0;
    let dy = f64::from(centre_y) - (unplaced_min[1] + unplaced_max[1]) / 2.0;
    let dz = -unplaced_min[2];
    (
        [
            unplaced_min[0] + dx,
            unplaced_min[1] + dy,
            unplaced_min[2] + dz,
        ],
        [
            unplaced_max[0] + dx,
            unplaced_max[1] + dy,
            unplaced_max[2] + dz,
        ],
    )
}

#[test]
fn scad_cli_import_preserves_source_coordinates() {
    let dir = tempdir().expect("tempdir");
    let scad = write_scad(
        dir.path(),
        "two_cubes.scad",
        "translate([-30,5,-4]) cube([4,6,8]);\ntranslate([-20,7,-2]) cube([2,2,4]);\n",
    );

    // Oracle for the triangle count only; geometry assertions below are
    // semantic. A repair/decimate/split pass would change this count.
    let evaluated =
        slicer_model_io::load_scad_model(&scad, &slicer_model_io::ScadEvalOptions::default())
            .expect("explicit SCAD evaluation of the fixture");
    let evaluated_indices: usize = evaluated
        .mesh
        .objects
        .iter()
        .map(|object| object.mesh.indices.len())
        .sum();

    for (extension, format) in [("stl", "stl"), ("obj", "obj"), ("3mf", "3mf")] {
        let output = dir.path().join(format!("out.{extension}"));
        let argv = import_argv(&scad, &output, &["--output-format", format]);
        let result = run_import_owned(&argv);
        assert_exit(&result, 0, &format!("SCAD import to {format}"));
        assert!(
            output.exists(),
            "SCAD import to {format} must write {}",
            output.display()
        );
        assert_no_split_siblings(dir.path(), "out", extension);

        let mesh = reload(&output);
        let its = single_object(&mesh, &format!("SCAD import to {format}"));
        let (min, max) = aabb(its);

        // Unplaced source coordinates: the whole point of this assertion is
        // that X stays negative. A bed-placed model would sit near the plate
        // centre instead.
        assert_close(min[0], -30.0, &format!("{format} min X"));
        assert_close(min[1], 5.0, &format!("{format} min Y"));
        assert_close(min[2], -4.0, &format!("{format} min Z"));
        assert_close(max[0], -18.0, &format!("{format} max X"));
        assert_close(max[1], 11.0, &format!("{format} max Y"));
        assert_close(max[2], 4.0, &format!("{format} max Z"));
        assert_close(
            volume_mm3(its),
            208.0,
            &format!("{format} analytic volume 4*6*8 + 2*2*4"),
        );

        // No repair happened: `run_import_command` routes `.scad` away from
        // `import_step_with_options`, and `run_scad_import` writes the
        // evaluated mesh directly, so the triangle count must match the
        // explicit loader's output exactly.
        assert_eq!(
            its.indices.len(),
            evaluated_indices,
            "{format} must preserve the evaluated triangle count (no repair, \
             decimation or component splitting)"
        );
    }
}

#[test]
fn scad_cli_slice_places_and_emits() {
    let dir = tempdir().expect("tempdir");
    let scad = write_scad(
        dir.path(),
        "placed_two_body.scad",
        "translate([-30,5,-4]) cube([4,6,8]);\ntranslate([-20,7,-2]) cube([2,2,4]);\n",
    );

    // Unplaced comparison through real `mesh import`: the same script is written
    // in source coordinates, so any placement observed below belongs to the
    // slice path and not to ingestion.
    let imported = dir.path().join("placed_two_body.stl");
    let import_result = run_import_owned(&import_argv(&scad, &imported, &[]));
    assert_exit(&import_result, 0, "unplaced import comparison");
    let imported_mesh = reload(&imported);
    let (import_min, import_max) =
        aabb(single_object(&imported_mesh, "unplaced import comparison"));
    for (axis, (got, want)) in [
        (import_min[0], -30.0),
        (import_min[1], 5.0),
        (import_min[2], -4.0),
        (import_max[0], -18.0),
        (import_max[1], 11.0),
        (import_max[2], 4.0),
    ]
    .into_iter()
    .enumerate()
    {
        assert_close(
            got,
            want,
            &format!("unplaced import comparison component {axis}"),
        );
    }

    let output = dir.path().join("placed_two_body.gcode");
    let result = run_slice(&scad, &output, &[]);
    assert_exit(&result, 0, "placed slice");
    assert!(
        result.stdout.is_empty(),
        "slice with --output must not write G-code to stdout:\n{}",
        String::from_utf8_lossy(&result.stdout)
    );
    let gcode = std::fs::read_to_string(&output)
        .unwrap_or_else(|e| panic!("read {}: {e}", output.display()));
    assert!(
        !gcode.is_empty(),
        "a successful slice must write non-empty G-code"
    );

    let stderr = stderr_text(&result);
    assert!(
        !stderr.contains("SCAD_OPTIONS_REQUIRED"),
        "the slice path must evaluate explicitly:\n{stderr}"
    );
    let events = slice_events(&stderr);
    let complete: Vec<&serde_json::Value> = events
        .iter()
        .filter(|event| event["event"] == "slice_complete")
        .collect();
    assert_eq!(
        complete.len(),
        1,
        "exactly one slice_complete expected\nstderr:\n{stderr}"
    );
    assert_eq!(
        complete[0]["status"].as_str(),
        Some("ok"),
        "slice_complete must report ok: {}",
        complete[0]
    );
    assert_eq!(
        complete[0]["fatal_error_count"].as_u64(),
        Some(0),
        "slice_complete must report no fatal errors: {}",
        complete[0]
    );

    // Analytic placement: unplaced x[-30,-18] y[5,11] z[-4,4] and the default
    // bed centre (125, 125) give dx=149 dy=117 dz=4, i.e. x[119,131]
    // y[122,128] z[0,8].
    let bed_shape = slicer_ir::ResolvedConfig::default().bed_shape;
    let (placed_min, placed_max) = placed_aabb([-30.0, 5.0, -4.0], [-18.0, 11.0, 4.0], &bed_shape);
    let analytic = [
        placed_min[0],
        placed_min[1],
        placed_min[2],
        placed_max[0],
        placed_max[1],
        placed_max[2],
    ];
    for (index, (got, want)) in analytic
        .iter()
        .zip([119.0, 122.0, 0.0, 131.0, 128.0, 8.0])
        .enumerate()
    {
        assert!(
            (got - want).abs() <= 1e-3,
            "analytic placement component {index}: expected {want}, got {got}"
        );
    }

    // Every extruding move, decoration included: the default skirt/brim offset
    // plus a line width stays well under 8 mm, so the placed footprint inflated
    // by that margin bounds the whole print.
    const DECORATION_MARGIN_MM: f64 = 8.0;
    let moves = extrusions(&gcode);
    assert!(
        !moves.is_empty(),
        "a successful slice must emit extrusion moves"
    );
    for (x, y, z) in &moves {
        assert!(
            *x >= placed_min[0] - DECORATION_MARGIN_MM
                && *x <= placed_max[0] + DECORATION_MARGIN_MM,
            "extruded X {x} outside the placed footprint x[{}, {}] plus {DECORATION_MARGIN_MM} mm",
            placed_min[0],
            placed_max[0]
        );
        assert!(
            *y >= placed_min[1] - DECORATION_MARGIN_MM
                && *y <= placed_max[1] + DECORATION_MARGIN_MM,
            "extruded Y {y} outside the placed footprint y[{}, {}] plus {DECORATION_MARGIN_MM} mm",
            placed_min[1],
            placed_max[1]
        );
        assert!(*z >= 0.0, "extruded Z {z} must never go below the plate");
        assert!(
            *z <= placed_max[2] + 0.4,
            "extruded Z {z} exceeds the placed top {} by more than 0.4 mm",
            placed_max[2]
        );
    }

    // Centring is asserted on the part's own extrusion types. `Skirt`/`Brim` are
    // generated around the first region's footprint rather than the whole model,
    // so their envelope is legitimately off-centre on a multi-region part
    // (measured X midpoint 121.0 for this fixture). A part that was never
    // placed, or was moved, still fails the span and midpoint checks.
    let part_moves = part_extrusions(&gcode);
    assert!(
        !part_moves.is_empty(),
        "the part itself must emit extrusion moves"
    );
    let (part_min, part_max) = bounds3(&part_moves);
    assert!(
        part_max[0] - part_min[0] >= 10.0,
        "the 12 mm-wide placed part must span at least 10 mm in X, got {}",
        part_max[0] - part_min[0]
    );
    for (axis, label) in [(0_usize, "X"), (1, "Y")] {
        let midpoint = (part_min[axis] + part_max[axis]) / 2.0;
        assert!(
            (midpoint - 125.0).abs() <= 2.0,
            "part extrusion midpoint {label} must be within 2 mm of the bed centre 125, \
             got {midpoint} from [{}, {}]",
            part_min[axis],
            part_max[axis]
        );
    }

    let max_z = max_z_height(&gcode);
    assert!(
        (max_z - 8.0).abs() <= 0.05,
        "max_z_height must be the placed top 8.0, got {max_z}"
    );
    let layers = total_layer_number(&gcode);
    assert!(
        (35..=45).contains(&layers),
        "8 mm at the default 0.2 mm layer height is about 40 layers, got {layers}"
    );
}

#[test]
fn scad_cli_holed_extrusion_slices() {
    let dir = tempdir().expect("tempdir");
    let scad = write_scad(
        dir.path(),
        "holed_slice.scad",
        "difference() { square([12,10]); translate([4,3]) square([4,4]); }\n",
    );
    let output = dir.path().join("holed_slice.gcode");
    let result = run_slice(&scad, &output, &["--scad-extrude-height", "2"]);
    assert_exit(&result, 0, "holed extrusion slice");
    assert!(
        result.stdout.is_empty(),
        "slice with --output must not write G-code to stdout"
    );
    let gcode = std::fs::read_to_string(&output)
        .unwrap_or_else(|e| panic!("read {}: {e}", output.display()));
    assert!(!gcode.is_empty(), "holed slice must write G-code");

    let stderr = stderr_text(&result);
    let events = slice_events(&stderr);
    let complete: Vec<&serde_json::Value> = events
        .iter()
        .filter(|event| event["event"] == "slice_complete")
        .collect();
    assert_eq!(
        complete.len(),
        1,
        "holed slice must emit exactly one slice_complete\nstderr:\n{stderr}"
    );
    assert_eq!(
        complete[0]["status"].as_str(),
        Some("ok"),
        "{}",
        complete[0]
    );

    // Analytic placement: the unplaced extrusion is x[0,12] y[0,10] z[0,2], so
    // the bed centre (125, 125) shifts it by dx=119 dy=120 dz=0.
    let bed_shape = slicer_ir::ResolvedConfig::default().bed_shape;
    let (placed_min, placed_max) = placed_aabb([0.0, 0.0, 0.0], [12.0, 10.0, 2.0], &bed_shape);
    for (index, (got, want)) in [
        (placed_min[0], 119.0),
        (placed_min[1], 120.0),
        (placed_min[2], 0.0),
        (placed_max[0], 131.0),
        (placed_max[1], 130.0),
        (placed_max[2], 2.0),
    ]
    .into_iter()
    .enumerate()
    {
        assert!(
            (got - want).abs() <= 1e-3,
            "holed analytic placement component {index}: expected {want}, got {got}"
        );
    }

    // The part's own walls must lie inside the placed footprint: a 0.2 mm inset
    // for the 0.4 mm line width stays strictly inside.
    let part_moves = part_extrusions(&gcode);
    assert!(!part_moves.is_empty(), "holed part must emit moves");
    for (x, y, z) in &part_moves {
        assert!(
            *x >= placed_min[0] && *x <= placed_max[0],
            "holed part X {x} outside the placed footprint [{}, {}]",
            placed_min[0],
            placed_max[0]
        );
        assert!(
            *y >= placed_min[1] && *y <= placed_max[1],
            "holed part Y {y} outside the placed footprint [{}, {}]",
            placed_min[1],
            placed_max[1]
        );
        assert!(
            *z >= 0.0 && *z <= placed_max[2],
            "holed part Z {z} outside the extruded height [0, {}]",
            placed_max[2]
        );
    }

    // The Boolean difference survives slicing: the source hole is
    // x[4,8] y[3,7], i.e. x[123,127] y[123,127] after the dx=119 dy=120 shift.
    // Shrink it by 0.4 mm (one line width) so wall geometry on the hole boundary
    // is not counted, and require the interior to stay empty.
    let hole_min = (placed_min[0] + 4.0 + 0.4, placed_min[1] + 3.0 + 0.4);
    let hole_max = (placed_min[0] + 8.0 - 0.4, placed_min[1] + 7.0 - 0.4);
    for (x, y, _) in &part_moves {
        assert!(
            !(*x > hole_min.0 && *x < hole_max.0 && *y > hole_min.1 && *y < hole_max.1),
            "extrusion at ({x}, {y}) falls inside the 4x4 hole x[{},{}] y[{},{}]",
            hole_min.0,
            hole_max.0,
            hole_min.1,
            hole_max.1
        );
    }

    let max_z = max_z_height(&gcode);
    assert!(
        (max_z - 2.0).abs() <= 0.05,
        "2 mm extrusion must report max_z_height 2.0, got {max_z}"
    );
    let layers = total_layer_number(&gcode);
    assert!(
        (8..=12).contains(&layers),
        "2 mm at the default 0.2 mm layer height is about 10 layers, got {layers}"
    );
}

#[test]
fn scad_cli_holed_extrusion() {
    let dir = tempdir().expect("tempdir");
    let scad = write_scad(
        dir.path(),
        "holed.scad",
        "difference() { square([12,10]); translate([4,3]) square([4,4]); }\n",
    );

    for (extension, format) in [("stl", "stl"), ("obj", "obj"), ("3mf", "3mf")] {
        let output = dir.path().join(format!("holed.{extension}"));
        let argv = import_argv(
            &scad,
            &output,
            &["--output-format", format, "--scad-extrude-height", "2"],
        );
        let result = run_import_owned(&argv);
        assert_exit(&result, 0, &format!("holed extrusion to {format}"));
        assert!(
            output.exists(),
            "holed extrusion to {format} must write {}",
            output.display()
        );

        // Compare semantically (AABB + volume), never by triangle order.
        let mesh = reload(&output);
        let its = single_object(&mesh, &format!("holed extrusion to {format}"));
        let (min, max) = aabb(its);
        assert_close(min[0], 0.0, &format!("{format} holed min X"));
        assert_close(min[1], 0.0, &format!("{format} holed min Y"));
        assert_close(min[2], 0.0, &format!("{format} holed min Z"));
        assert_close(max[0], 12.0, &format!("{format} holed max X"));
        assert_close(max[1], 10.0, &format!("{format} holed max Y"));
        assert_close(max[2], 2.0, &format!("{format} holed max Z"));
        assert_close(
            volume_mm3(its),
            208.0,
            &format!("{format} holed volume (12*10 - 4*4) * 2"),
        );
    }
}

#[test]
fn scad_cli_information_and_stats() {
    let dir = tempdir().expect("tempdir");
    let scad = write_scad(
        dir.path(),
        "echo.scad",
        "echo(\"scad information\");\ncube([2,2,2]);\n",
    );

    // With `--stats`: one JSONL diagnostic row and one done row on stderr,
    // nothing on stdout.
    let output_path = dir.path().join("echo.stl");
    let result = run_import_owned(&import_argv(&scad, &output_path, &["--stats"]));
    assert_exit(&result, 0, "echo import with --stats");
    assert!(
        result.stdout.is_empty(),
        "import must not write mesh bytes to stdout:\n{}",
        String::from_utf8_lossy(&result.stdout)
    );
    let stderr = stderr_text(&result);
    let rows: Vec<serde_json::Value> = stderr
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    let diagnostics: Vec<&serde_json::Value> = rows
        .iter()
        .filter(|row| row["event"] == "diagnostic")
        .collect();
    assert_eq!(
        diagnostics.len(),
        1,
        "exactly one accepted information diagnostic expected\nstderr:\n{stderr}"
    );
    let diagnostic = diagnostics[0];
    assert_eq!(diagnostic["kind"], "scad_information");
    assert_eq!(diagnostic["code"], "scad_echo");
    assert_eq!(diagnostic["operation"], "import");
    assert!(
        diagnostic["stage"].is_string(),
        "diagnostic row must carry a serialized stage: {diagnostic}"
    );
    assert_eq!(diagnostic["stage"], "evaluation");
    let message = diagnostic["message"]
        .as_str()
        .expect("diagnostic row message must be a string");
    assert!(
        message.contains("scad information"),
        "the diagnostic row must carry the adapter's message, got {message:?}"
    );
    let source_label = diagnostic["source_label"]
        .as_str()
        .expect("diagnostic row source_label must be a string");
    assert!(
        source_label.contains("echo.scad"),
        "source_label must name the input, got {source_label:?}"
    );

    // Delivery is once: with `--stats` the information appears exactly once on
    // stderr (the JSON row), never as a plain note as well.
    assert_eq!(
        stderr.matches("scad information").count(),
        1,
        "accepted information must be delivered exactly once with --stats
stderr:
{stderr}"
    );

    let done: Vec<&serde_json::Value> = rows.iter().filter(|row| row["event"] == "done").collect();
    assert_eq!(
        done.len(),
        1,
        "exactly one done row expected\nstderr:\n{stderr}"
    );
    let done = done[0];
    assert_eq!(done["operation"], "import");
    assert_eq!(done["source_unit"], "mm");
    assert_eq!(done["mesh_count"].as_u64(), Some(1));
    assert_eq!(done["warnings"].as_u64(), Some(0));
    assert_eq!(done["total_triangles"].as_u64(), Some(12));

    // Without `--stats`: the same information reaches stderr as a plain line,
    // and no `scad_information` JSON row is emitted.
    let quiet_path = dir.path().join("echo_quiet.stl");
    let result = run_import_owned(&import_argv(&scad, &quiet_path, &[]));
    assert_exit(&result, 0, "echo import without --stats");
    assert!(result.stdout.is_empty());
    let stderr = stderr_text(&result);
    assert!(
        stderr.contains("scad information"),
        "accepted information must reach stderr without --stats:\n{stderr}"
    );
    assert!(
        !stderr.contains("\"kind\":\"scad_information\""),
        "no diagnostic JSON row may be emitted without --stats:\n{stderr}"
    );

    // The same accepted information reaches stderr on the slice path, where
    // there is no `--stats` / JSONL diagnostic event: the plain `note:` line
    // must name the code and carry the echoed message, before the G-code exists.
    let sliced = dir.path().join("echo.gcode");
    let result = run_slice(&scad, &sliced, &[]);
    assert_exit(&result, 0, "echo slice");
    assert!(
        result.stdout.is_empty(),
        "slice with --output must not write G-code to stdout:\n{}",
        String::from_utf8_lossy(&result.stdout)
    );
    let stderr = stderr_text(&result);
    assert!(
        stderr
            .lines()
            .any(|line| line.contains("scad_echo") && line.contains("scad information")),
        "slice must report the accepted information on stderr with its code and message:\n{stderr}"
    );
    let complete: Vec<serde_json::Value> = slice_events(&stderr)
        .into_iter()
        .filter(|event| event["event"] == "slice_complete")
        .collect();
    assert_eq!(
        complete.len(),
        1,
        "echo slice must emit exactly one slice_complete\nstderr:\n{stderr}"
    );
    assert_eq!(
        complete[0]["status"].as_str(),
        Some("ok"),
        "echo slice must succeed: {}",
        complete[0]
    );

    // The stdout G-code path (no `--output`) must succeed too, with the G-code
    // alone on stdout and the diagnostic on stderr only.
    let result = run_slice_stdout(&scad, &[]);
    assert_exit(&result, 0, "echo slice to stdout");
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(
        !stdout.trim().is_empty(),
        "a successful stdout slice must emit G-code"
    );
    assert!(
        stdout.contains(";LAYER_CHANGE") || stdout.contains("G1 "),
        "stdout must carry G-code, not a diagnostic:\\n{stdout}"
    );
    assert!(
        !stdout.contains("scad information") && !stdout.contains("note: scad"),
        "no diagnostic may contaminate stdout G-code:\\n{stdout}"
    );
    let stderr = stderr_text(&result);
    assert!(
        stderr
            .lines()
            .any(|line| line.contains("scad_echo") && line.contains("scad information")),
        "the stdout slice must still report the information on stderr:\\n{stderr}"
    );
    assert_eq!(
        slice_events(&stderr)
            .into_iter()
            .filter(|event| event["event"] == "slice_complete")
            .count(),
        1,
        "the stdout slice must emit exactly one slice_complete\\nstderr:\\n{stderr}"
    );
}

#[test]
fn scad_cli_parameters_and_config_separation() {
    let dir = tempdir().expect("tempdir");

    // (a) Repeated `--scad-param` for one name: the last expression wins, and
    // the expression is SCAD arithmetic evaluated by the adapter, not a JSON
    // literal parsed by the CLI.
    let size = write_scad(dir.path(), "size.scad", "Size=[6,8,2]; cube(Size);\n");
    let size_first = dir.path().join("size_first.stl");
    let result = run_import_owned(&import_argv(
        &size,
        &size_first,
        &["--scad-param", "Size=[5*2,12,3]"],
    ));
    assert_exit(&result, 0, "parameter arithmetic");
    assert_mesh_bounds(
        &reload(&size_first),
        [0.0, 0.0, 0.0],
        [10.0, 12.0, 3.0],
        "parameter arithmetic 5*2 x 12 x 3",
    );
    let size_last = dir.path().join("size_last_wins.stl");
    let result = run_import_owned(&import_argv(
        &size,
        &size_last,
        &[
            "--scad-param",
            "Size=[5*2,12,3]",
            "--scad-param",
            "Size=[8,6,4]",
        ],
    ));
    assert_exit(&result, 0, "duplicate parameter last wins");
    assert_mesh_bounds(
        &reload(&size_last),
        [0.0, 0.0, 0.0],
        [8.0, 6.0, 4.0],
        "duplicate parameter last wins",
    );

    // (b) The raw token splits at its **first** `=`, so a comparison operator
    // survives intact in the expression bytes. The source must declare the
    // name: an override rebinds an existing binding, and an override of an
    // undeclared name leaves it `undef`, which renders as empty geometry
    // (verified against this pin; it is not a frontend concern).
    let ternary = write_scad(dir.path(), "ternary.scad", "e=1; cube([2,2,e]);\n");
    let ternary_out = dir.path().join("ternary.stl");
    let result = run_import_owned(&import_argv(
        &ternary,
        &ternary_out,
        &["--scad-param", "e=1==1 ? 4 : 9"],
    ));
    assert_exit(&result, 0, "first-= split");
    assert_mesh_bounds(
        &reload(&ternary_out),
        [0.0, 0.0, 0.0],
        [2.0, 2.0, 4.0],
        "first-= split keeps `1==1 ? 4 : 9`",
    );

    // (b2) A **nested** vector expression: the override is a list of lists and
    // the source indexes it twice, so a frontend that flattened or JSON-parsed
    // the value could not produce these bounds.
    let nested = write_scad(
        dir.path(),
        "nested.scad",
        "v=[[2,3],[4,5]]; cube([v[0][1], v[1][0], 1]);\n",
    );
    let nested_default = dir.path().join("nested_default.stl");
    let result = run_import_owned(&import_argv(&nested, &nested_default, &[]));
    assert_exit(&result, 0, "nested vector source default");
    assert_mesh_bounds(
        &reload(&nested_default),
        [0.0, 0.0, 0.0],
        [3.0, 4.0, 1.0],
        "nested vector source default (v[0][1] x v[1][0])",
    );
    let nested_override = dir.path().join("nested_override.stl");
    let result = run_import_owned(&import_argv(
        &nested,
        &nested_override,
        &["--scad-param", "v=[[6,7],[8,9]]"],
    ));
    assert_exit(&result, 0, "nested vector override");
    assert_mesh_bounds(
        &reload(&nested_override),
        [0.0, 0.0, 0.0],
        [7.0, 8.0, 1.0],
        "nested vector override (v[0][1]=7 x v[1][0]=8)",
    );

    // (b3) An invalid expression that a later duplicate overwrites still fails:
    // every supplied override is parsed, and the frontend preserves the
    // adapter's offending-parameter context on stderr.
    let overwritten = write_scad(dir.path(), "overwritten.scad", "cube([2,2,2]);\n");
    let overwritten_out = dir.path().join("overwritten.stl");
    let result = run_import_owned(&import_argv(
        &overwritten,
        &overwritten_out,
        &["--scad-param", "nope=(", "--scad-param", "nope=2"],
    ));
    assert_exit(&result, 4, "invalid overwritten expression");
    assert_stderr_contains(
        &result,
        "invalid_parameter",
        "invalid overwritten expression",
    );
    assert_stderr_contains(
        &result,
        "'nope'",
        "invalid overwritten expression: parameter name",
    );
    assert!(
        !overwritten_out.exists(),
        "an invalid overwritten override must publish nothing"
    );

    // (c) Override names are case-sensitive: `A=3` must not touch `a=5`.
    let case = write_scad(dir.path(), "case.scad", "A=2; a=5; cube([A,a,a]);\n");
    let case_out = dir.path().join("case.stl");
    let result = run_import_owned(&import_argv(&case, &case_out, &["--scad-param", "A=3"]));
    assert_exit(&result, 0, "case-sensitive names");
    assert_mesh_bounds(
        &reload(&case_out),
        [0.0, 0.0, 0.0],
        [3.0, 5.0, 5.0],
        "case-sensitive names (3 x 5 x 5, not 3 x 3 x 3)",
    );

    // (d) `$preview` is an ordinary dynamic variable: the source default is
    // used unless an override names it explicitly.
    let preview = write_scad(
        dir.path(),
        "preview.scad",
        "$preview = 5; cube([$preview, 2, 2]);\n",
    );
    let preview_default = dir.path().join("preview_default.stl");
    let result = run_import_owned(&import_argv(&preview, &preview_default, &[]));
    assert_exit(&result, 0, "$preview source default");
    assert_mesh_bounds(
        &reload(&preview_default),
        [0.0, 0.0, 0.0],
        [5.0, 2.0, 2.0],
        "$preview source default",
    );
    let preview_override = dir.path().join("preview_override.stl");
    let result = run_import_owned(&import_argv(
        &preview,
        &preview_override,
        &["--scad-param", "$preview=9"],
    ));
    assert_exit(&result, 0, "$preview override");
    assert_mesh_bounds(
        &reload(&preview_override),
        [0.0, 0.0, 0.0],
        [9.0, 2.0, 2.0],
        "$preview override",
    );

    // (e) Slicer config and SCAD variables share a *name* but not a namespace.
    // The script's `layer_height` sets the model height (5 mm through the
    // override); the JSON config's `layer_height` sets the print layer height
    // (0.4 mm). A slice driven by the script variable would produce one layer;
    // the default 0.2 mm would produce about 25.
    let shared = write_scad(
        dir.path(),
        "shared_name.scad",
        "layer_height=3; cube([6,6,layer_height]);\n",
    );
    let shared_import = dir.path().join("shared_name.stl");
    let result = run_import_owned(&import_argv(
        &shared,
        &shared_import,
        &["--scad-param", "layer_height=5"],
    ));
    assert_exit(&result, 0, "config separation import");
    assert_mesh_bounds(
        &reload(&shared_import),
        [0.0, 0.0, 0.0],
        [6.0, 6.0, 5.0],
        "config separation import height",
    );

    let config = dir.path().join("layer_height.json");
    std::fs::write(&config, r#"{"layer_height":0.4}"#).expect("write JSON config");
    let sliced = dir.path().join("shared_name.gcode");
    let result = run_slice(
        &shared,
        &sliced,
        &[
            "--scad-param",
            "layer_height=5",
            "--config",
            config.to_str().expect("temp config path is valid UTF-8"),
        ],
    );
    assert_exit(&result, 0, "config separation slice");
    let gcode = std::fs::read_to_string(&sliced)
        .unwrap_or_else(|e| panic!("read {}: {e}", sliced.display()));
    let max_z = max_z_height(&gcode);
    assert!(
        (max_z - 5.0).abs() <= 0.05,
        "the script's layer_height=5 override must set the model height, got max_z_height {max_z}"
    );
    let layers = total_layer_number(&gcode);
    assert!(
        (12..=14).contains(&layers),
        "5 mm at the config's 0.4 mm layer height is 12.5 layers, got {layers} \
         (1 layer would mean the script variable drove slicing; 25 would mean the 0.2 mm default)"
    );

    // (f) Neither flag name nor a `NAME=EXPR` token may leak into G-code. The
    // model label legitimately contains the `.scad` file name.
    for forbidden in [
        "--scad-param",
        "scad_param",
        "scad-extrude-height",
        "scad_extrude_height",
        "layer_height=5",
    ] {
        assert!(
            !gcode.contains(forbidden),
            "G-code must not carry the CLI flag/name {forbidden:?}"
        );
    }
}

#[test]
fn scad_cli_failures_do_not_publish() {
    let dir = tempdir().expect("tempdir");
    let cube3d = write_scad(dir.path(), "cube3d.scad", "cube([2,2,2]);\n");
    let square2d = write_scad(dir.path(), "square2d.scad", "square([4,4]);\n");
    let empty = write_scad(dir.path(), "empty.scad", "%cube(1);\n");
    let broken = write_scad(dir.path(), "broken.scad", "cube(\n");

    // Case A: absent destination. 3D geometry + explicit height is an adapter
    // rejection (`ExtrusionHeightOn3d`), not an ignored flag.
    let absent = dir.path().join("case_a.stl");
    let result = run_import_owned(&import_argv(
        &cube3d,
        &absent,
        &["--scad-extrude-height", "1", "--stats"],
    ));
    assert_exit(&result, 4, "case A: height on 3D");
    assert_stderr_contains(&result, "SCAD_EVALUATION", "case A: height on 3D");
    assert_stderr_contains(&result, "extrusion_height_on_3d", "case A: height on 3D");
    assert!(!absent.exists(), "case A must leave the destination absent");
    assert_no_done_row(&result, "case A");
    assert!(result.stdout.is_empty(), "case A must not write stdout");

    // Case B: pre-existing destination bytes are untouched on failure.
    let existing = dir.path().join("case_b.stl");
    std::fs::write(&existing, b"keep me").expect("seed pre-existing destination");
    let result = run_import_owned(&import_argv(
        &cube3d,
        &existing,
        &["--scad-extrude-height", "1", "--stats"],
    ));
    assert_exit(&result, 4, "case B: height on 3D with existing destination");
    assert_eq!(
        std::fs::read(&existing).expect("read pre-existing destination"),
        b"keep me",
        "case B must not change a pre-existing destination"
    );
    assert_no_done_row(&result, "case B");
    assert!(result.stdout.is_empty(), "case B must not write stdout");

    // Case C: 2D without an explicit height is never given an implicit one.
    let needs_extrusion = dir.path().join("case_c.stl");
    let result = run_import_owned(&import_argv(&square2d, &needs_extrusion, &["--stats"]));
    assert_exit(&result, 4, "case C: 2D without height");
    assert_stderr_contains(&result, "SCAD_NEEDS_EXTRUSION", "case C: 2D without height");
    assert!(
        !needs_extrusion.exists(),
        "case C must leave the destination absent"
    );
    assert_no_done_row(&result, "case C");
    assert!(result.stdout.is_empty(), "case C must not write stdout");

    // Case D: empty geometry is an empty model, exit 3.
    let empty_out = dir.path().join("case_d.stl");
    let result = run_import_owned(&import_argv(&empty, &empty_out, &["--stats"]));
    assert_exit(&result, 3, "case D: empty model");
    assert_stderr_contains(&result, "SCAD_EMPTY_MODEL", "case D: empty model");
    assert!(
        !empty_out.exists(),
        "case D must leave the destination absent"
    );
    assert_no_done_row(&result, "case D");
    assert!(result.stdout.is_empty(), "case D must not write stdout");

    // Case E: a parse error is an adapter evaluation error, exit 4.
    let broken_out = dir.path().join("case_e.stl");
    let result = run_import_owned(&import_argv(&broken, &broken_out, &["--stats"]));
    assert_exit(&result, 4, "case E: parse error");
    assert_stderr_contains(&result, "SCAD_EVALUATION", "case E: parse error");
    assert!(
        !broken_out.exists(),
        "case E must leave the destination absent"
    );
    assert_no_done_row(&result, "case E");
    assert!(result.stdout.is_empty(), "case E must not write stdout");

    // Case F: mixed 2D/3D content is an adapter rejection on the frontend too.
    let mixed = write_scad(dir.path(), "mixed.scad", "union(){ cube(1); square(1); }\n");
    let mixed_out = dir.path().join("case_f.stl");
    let result = run_import_owned(&import_argv(&mixed, &mixed_out, &["--stats"]));
    assert_exit(&result, 4, "case F: mixed dimensions");
    assert_stderr_contains(&result, "mixed_dimensions", "case F: mixed dimensions");
    assert!(
        !mixed_out.exists(),
        "case F must leave the destination absent"
    );
    assert_no_done_row(&result, "case F");
    assert!(result.stdout.is_empty(), "case F must not write stdout");

    // Exit-table rows that need their own inputs: unreadable source, and a
    // destination that cannot be written (an existing directory).
    let missing = dir.path().join("does_not_exist.scad");
    let missing_out = dir.path().join("case_missing.stl");
    let result = run_import_owned(&import_argv(&missing, &missing_out, &["--stats"]));
    assert_exit(&result, 2, "case missing: import unreadable source");
    assert_stderr_contains(&result, "input file not found", "case missing: import");
    assert_no_done_row(&result, "case missing: import");

    let unwritable_import = dir.path().to_path_buf();
    let result = run_import_owned(&import_argv(&cube3d, &unwritable_import, &["--stats"]));
    assert_exit(&result, 2, "case unwritable: import write failure");
    assert_stderr_contains(
        &result,
        "failed to write output mesh",
        "case unwritable: import",
    );
    assert!(
        result.stdout.is_empty(),
        "case unwritable: import must not write stdout"
    );

    // Slice rows: missing source exits 1 before any ingestion, empty geometry is
    // exit 1, and a destination that cannot be written is exit 1 *after* a
    // successful slice (the slice itself is done; only the write failed, so a
    // `slice_complete` event is legitimate there and is not asserted absent).
    let result = run_slice_raw(
        &dir.path().join("does_not_exist.scad"),
        &dir.path().join("case_missing.gcode"),
        &[],
    );
    assert_exit(&result, 1, "slice: missing source");
    assert_stderr_contains(&result, "model file not found", "slice: missing source");

    let result = run_slice_raw(&empty, &dir.path().join("case_empty.gcode"), &[]);
    assert_exit(&result, 1, "slice: empty model");
    assert_stderr_contains(&result, "SCAD_EMPTY_MODEL", "slice: empty model");
    assert_no_slice_complete(&stderr_text(&result), "slice: empty model");

    let result = run_slice(&cube3d, dir.path(), &[]);
    assert_exit(&result, 1, "slice: write failure");
    assert_stderr_contains(&result, "failed to write output", "slice: write failure");
    assert!(
        result.stdout.is_empty(),
        "slice: write failure must not write stdout"
    );

    // Slice half: an ingestion failure exits 1, publishes no G-code and never
    // emits `slice_complete`. No `--module-dir` is passed: ingestion fails
    // before any module loads, so a missing guest tree must never be what makes
    // these cases pass or fail.
    let slice_cases: [(&str, &Path, &[&str], &str); 3] = [
        (
            "slice: height on 3D",
            &cube3d,
            &["--scad-extrude-height", "1"],
            "extrusion_height_on_3d",
        ),
        (
            "slice: 2D without height",
            &square2d,
            &[],
            "SCAD_NEEDS_EXTRUSION",
        ),
        ("slice: parse error", &broken, &[], "SCAD_EVALUATION"),
    ];
    for (index, (label, model, extra, fragment)) in slice_cases.into_iter().enumerate() {
        let out = dir.path().join(format!("slice_case_{index}.gcode"));
        let result = run_slice_raw(model, &out, extra);
        assert_exit(&result, 1, label);
        assert_stderr_contains(&result, fragment, label);
        assert_stderr_contains(&result, "error: failed to load model", label);
        assert_no_slice_complete(&stderr_text(&result), label);
        assert!(
            result.stdout.is_empty(),
            "{label} must not write G-code to stdout"
        );
        assert!(!out.exists(), "{label} must leave the destination absent");
    }

    // A pre-existing G-code destination survives an ingestion failure byte for
    // byte: the output is not created or truncated until evaluation succeeds.
    let seeded = dir.path().join("slice_existing.gcode");
    std::fs::write(&seeded, b"keep me").expect("seed pre-existing G-code");
    let result = run_slice_raw(&cube3d, &seeded, &["--scad-extrude-height", "1"]);
    assert_exit(&result, 1, "slice: pre-existing destination");
    assert_eq!(
        std::fs::read(&seeded).expect("read pre-existing G-code"),
        b"keep me",
        "slice must not change a pre-existing destination on ingestion failure"
    );
    assert_no_slice_complete(&stderr_text(&result), "slice: pre-existing destination");
    assert!(
        result.stdout.is_empty(),
        "slice must not write G-code to stdout"
    );

    // The stdout path (no `--output`) must not claim success either: nothing on
    // stdout, and no success event on stderr.
    let result = run_slice_stdout(&cube3d, &["--scad-extrude-height", "1"]);
    assert_exit(&result, 1, "slice: stdout path");
    assert!(
        result.stdout.is_empty(),
        "a failed ingestion must not write G-code to stdout:\n{}",
        String::from_utf8_lossy(&result.stdout)
    );
    assert_no_slice_complete(&stderr_text(&result), "slice: stdout path");
}

#[test]
fn scad_cli_strict_errors_and_resources() {
    let dir = tempdir().expect("tempdir");

    // AC-N3: a readable sibling primitive next to the source must not soften the
    // adapter's deny-all resource policy, and no partial geometry may be
    // published. The sibling is a real `cube(99)`, so accepting the `include`
    // would produce visibly different geometry rather than merely a warning.
    // Both inputs are saved under `target/scad-ingest/` before execution.
    let sibling = record("tempting_sibling.scad", "cube(99);\n", &[]);
    let sibling_label = sibling.to_string_lossy().replace('\\', "/");
    let resource_source = format!("include <{sibling_label}>; cube(1);\n");
    let resource = record("resource.scad", &resource_source, &["--stats"]);

    let resource_out = dir.path().join("resource.stl");
    record_invocation("resource.import", &["--stats"]);
    let result = run_import_owned(&import_argv(&resource, &resource_out, &["--stats"]));
    record_capture("resource.import", &result);
    assert_exit(&result, 4, "external resource with a valid sibling");
    assert_stderr_contains(&result, "SCAD_EVALUATION", "external resource");
    assert_stderr_contains(&result, "external_resource_denied", "external resource");
    assert_stderr_contains(&result, "resource.scad", "external resource: source label");
    assert!(
        !resource_out.exists(),
        "an external-resource request must publish no mesh, not even the sibling's geometry"
    );
    assert_no_done_row(&result, "external resource");
    assert!(
        result.stdout.is_empty(),
        "external resource must not write stdout"
    );

    // The same rejection must hold on the slice path: the evaluator is shared,
    // and a denied request is a failure there too.
    let resource_gcode = dir.path().join("resource.gcode");
    record_invocation("resource.slice", &[]);
    let result = run_slice_raw(&resource, &resource_gcode, &[]);
    record_capture("resource.slice", &result);
    assert_exit(&result, 1, "external resource on the slice path");
    assert_stderr_contains(
        &result,
        "external_resource_denied",
        "external resource (slice)",
    );
    assert_no_slice_complete(&stderr_text(&result), "external resource (slice)");
    assert!(
        !resource_gcode.exists(),
        "slice must publish no G-code for a denied resource request"
    );

    // The pinned engine's public-path rejections keep their typed kind, source
    // label and message on stderr. These are #3's own reproducers, not new
    // fixture interpretations.
    let rejected: [(&str, &str, &str); 3] = [
        (
            "partial_output.scad",
            "function cum(list, k) = (k == 0) ? 0 : cum(list, k - 1) + list[k - 1];\
             cube(3); cube(cum([5], -1)); cube(7);",
            "partial_output",
        ),
        (
            "approximate_output.scad",
            "roof(){ difference(){ square(10); translate([3,3]) square(4); } }",
            "approximate_output",
        ),
        (
            "degraded_output.scad",
            "union(){ cube(10); polyhedron(points=[[0,0,0],[1,0,0],[0,1,0]], faces=[[0,1,2]]); }",
            "degraded_output",
        ),
    ];
    for (name, source, kind) in rejected {
        let path = record(name, source, &["--stats"]);
        let out = dir.path().join(format!("{name}.stl"));
        record_invocation(&format!("{kind}.import"), &["--stats"]);
        let result = run_import_owned(&import_argv(&path, &out, &["--stats"]));
        record_capture(&format!("{kind}.import"), &result);
        let context = format!("{kind} rejection");
        assert_exit(&result, 4, &context);
        assert_stderr_contains(&result, "SCAD_EVALUATION", &context);
        assert_stderr_contains(&result, kind, &context);
        assert_stderr_contains(&result, name, &format!("{context}: source label"));
        assert!(
            !out.exists(),
            "{context}: partial, degraded or approximate geometry must never be published"
        );
        assert_no_done_row(&result, &context);
        assert!(result.stdout.is_empty(), "{context}: must not write stdout");
    }

    // Over the **default** source bound: no flag is needed, because the adapter's
    // default `max_source_bytes` is 1_048_576. The fixture is generated here and
    // retained, not committed.
    let default_cap = slicer_model_io::ScadEvalOptions::default()
        .limits
        .max_source_bytes;
    let mut huge = String::from("cube([2,2,2]);\n//");
    while huge.len() <= default_cap + 1 {
        huge.push_str(" padding");
    }
    let huge_path = record("oversize_default.scad", &huge, &["--stats"]);
    let huge_out = dir.path().join("oversize_default.stl");
    record_invocation("oversize_default.import", &["--stats"]);
    let result = run_import_owned(&import_argv(&huge_path, &huge_out, &["--stats"]));
    record_capture("oversize_default.import", &result);
    assert_exit(&result, 4, "default source bound");
    assert_stderr_contains(&result, "complexity_limit", "default source bound");
    assert_stderr_contains(
        &result,
        &default_cap.to_string(),
        "default source bound names the cap",
    );
    assert!(
        !huge_out.exists(),
        "an over-limit source must publish nothing"
    );
    assert_no_done_row(&result, "default source bound");

    // Out-of-range coordinates, reachable on the public path: the adapter's
    // default `max_abs_coordinate_mm` is 10_000.
    let range = record(
        "coordinate_range.scad",
        "translate([10001,0,0]) cube(1);\n",
        &["--stats"],
    );
    let range_out = dir.path().join("coordinate_range.stl");
    record_invocation("coordinate_range.import", &["--stats"]);
    let result = run_import_owned(&import_argv(&range, &range_out, &["--stats"]));
    record_capture("coordinate_range.import", &result);
    assert_exit(&result, 4, "coordinate out of range");
    assert_stderr_contains(
        &result,
        "coordinate_out_of_range",
        "coordinate out of range",
    );
    assert_stderr_contains(&result, "10001", "coordinate out of range names the value");
    assert!(
        !range_out.exists(),
        "out-of-range geometry must publish nothing"
    );
    assert_no_done_row(&result, "coordinate out of range");

    // Post-quantization conversion damage, reachable on the public path: two unit
    // squares 2e-5 mm apart are disjoint in raw f64, but the 1e-4 mm output grid
    // closes the gap. This is #3's `scad_public_path_rejects_quantization_damage`
    // reproducer, so the frontend rejection is proved end to end, not constructed.
    let quant = record(
        "quantization.scad",
        "union(){ square(1); translate([1.00002, 0]) square(1); }\n",
        &["--stats"],
    );
    let quant_out = dir.path().join("quantization.stl");
    let result = run_import_owned(&import_argv(&quant, &quant_out, &["--stats"]));
    record_capture("quantization", &result);
    assert_exit(&result, 4, "post-quantization conversion damage");
    assert_stderr_contains(
        &result,
        "invalid_contours",
        "post-quantization conversion damage",
    );
    assert_stderr_contains(
        &result,
        "quantized grid",
        "post-quantization conversion damage names the cause",
    );
    assert!(
        !quant_out.exists(),
        "conversion damage must never be repaired into an output"
    );
    assert_no_done_row(&result, "post-quantization conversion damage");

    // Deliberate coverage statement for the one kind no frontend can drive:
    // `non_finite_coordinate` has no reproducer reachable through the public
    // path at this pin — `cube([0/0,1,1])`, `cube([1/0,1,1])` and
    // `cube([1e400,1,1])` all evaluate to *empty* geometry, not to non-finite
    // coordinates. It is therefore error-envelope evidence at the model
    // boundary (`slicer-model-io`'s
    // `scad_model_preserves_strict_adapter_errors`, constructed `ScadError`
    // through `ModelLoadError::ScadEvaluation`, `source()` and Display), and
    // #3's private conversion tests own the executed rejection. This frontend
    // deliberately adds no second injection seam; every other kind above is
    // driven through real public input.
}

/// AC-N1 (enabled state): every option-misuse path is a usage error, exit 2,
/// with the fragment the packet names.
///
/// This lives in the enabled target because the flags only exist there. The
/// feature-off behaviour — the same tokens being unknown arguments — is checked
/// by `scad_cli_feature_routing` under `CF`, per AC-N1.
#[test]
fn scad_cli_rejects_option_misuse() {
    let dir = tempdir().expect("tempdir");
    let scad = record("misuse.scad", "cube([2,2,2]);\n", &[]);
    let real_stl = workspace_root()
        .join("resources")
        .join("regression_wedge.stl");
    assert!(
        real_stl.exists(),
        "real STL fixture must exist at {}",
        real_stl.display()
    );
    let step = record("empty.step", "", &[]);

    // Malformed NAME=EXPR tokens are CLI usage errors, not adapter errors.
    for raw in ["NoEquals", "=1", "a="] {
        let import_out = dir.path().join("invalid_import.stl");
        let slice_out = dir.path().join("invalid_slice.gcode");
        for argv in [
            import_argv(&scad, &import_out, &["--scad-param", raw]),
            import_argv(&step, &import_out, &["--scad-param", raw]),
        ] {
            let result = run_import_owned(&argv);
            assert_exit(&result, 2, &format!("malformed parameter {raw:?}"));
            assert_stderr_contains(
                &result,
                "SCAD_INVALID_PARAMETER_ARGUMENT",
                &format!("malformed parameter {raw:?}"),
            );
        }
        for (model, out) in [(&scad, &slice_out), (&step, &slice_out)] {
            let result = run_slice_raw(model, out, &["--scad-param", raw]);
            assert_exit(&result, 2, &format!("malformed parameter {raw:?} (slice)"));
            assert_stderr_contains(
                &result,
                "SCAD_INVALID_PARAMETER_ARGUMENT",
                &format!("malformed parameter {raw:?} (slice)"),
            );
        }
        assert!(!import_out.exists());
        assert!(!slice_out.exists());
    }

    // Non-finite / non-positive heights are rejected by the same CLI validator.
    // `allow_hyphen_values` means `-1` reaches it instead of clap's unknown-flag
    // error.
    for raw in ["0", "-1", "NaN", "inf"] {
        let import_out = dir.path().join("height_import.stl");
        let slice_out = dir.path().join("height_slice.gcode");
        let result = run_import_owned(&import_argv(
            &scad,
            &import_out,
            &["--scad-extrude-height", raw],
        ));
        assert_exit(&result, 2, &format!("invalid height {raw:?}"));
        assert_stderr_contains(
            &result,
            "SCAD_INVALID_EXTRUSION_HEIGHT",
            &format!("invalid height {raw:?}"),
        );
        let result = run_slice_raw(&scad, &slice_out, &["--scad-extrude-height", raw]);
        assert_exit(&result, 2, &format!("invalid height {raw:?} (slice)"));
        assert_stderr_contains(
            &result,
            "SCAD_INVALID_EXTRUSION_HEIGHT",
            &format!("invalid height {raw:?} (slice)"),
        );
        assert!(!import_out.exists());
        assert!(!slice_out.exists());
    }

    // Both SCAD flags require a detected `.scad` input, before any load. An STL
    // and a STEP file are both non-SCAD input; the conflict is decided from the
    // extension, so the empty STEP file is never parsed.
    for (label, path) in [("STL", &real_stl), ("STEP", &step)] {
        for extra in [
            &["--scad-param", "a=1"][..],
            &["--scad-extrude-height", "1"][..],
        ] {
            let import_out = dir.path().join("require_scad_import.stl");
            let slice_out = dir.path().join("require_scad_slice.gcode");
            let result = run_import_owned(&import_argv(path, &import_out, extra));
            assert_exit(&result, 2, &format!("SCAD flag on {label} input"));
            assert_stderr_contains(
                &result,
                "SCAD_OPTIONS_REQUIRE_SCAD_INPUT",
                &format!("SCAD flag on {label} input"),
            );
            let result = run_slice_raw(path, &slice_out, extra);
            assert_exit(&result, 2, &format!("SCAD flag on {label} input (slice)"));
            assert_stderr_contains(
                &result,
                "SCAD_OPTIONS_REQUIRE_SCAD_INPUT",
                &format!("SCAD flag on {label} input (slice)"),
            );
            assert!(!import_out.exists());
            assert!(!slice_out.exists());
        }
    }

    // STEP-only flags conflict with SCAD import (which fuses and never repairs).
    // The message must name the offending flag.
    for flag in ["--merge-components", "--no-repair"] {
        let conflict_out = dir.path().join("conflict.stl");
        let argv = import_argv(&scad, &conflict_out, &[flag]);
        let result = run_import_owned(&argv);
        assert_exit(&result, 2, &format!("{flag} on SCAD input"));
        assert_stderr_contains(
            &result,
            "SCAD_STEP_OPTION_CONFLICT",
            &format!("{flag} on SCAD input"),
        );
        assert_stderr_contains(&result, flag, &format!("{flag} on SCAD input"));
        assert!(!conflict_out.exists());
    }

    // Without a SCAD flag, the empty `.step` still reaches the STEP importer and
    // reports a STEP parse error, not a SCAD one: the branch is decided by the
    // detected format, not by the presence of this test.
    let step_out = dir.path().join("step_plain.stl");
    let result = run_import_owned(&import_argv(&step, &step_out, &[]));
    assert_exit(&result, 4, "plain STEP import still reaches STEP");
    let stderr = stderr_text(&result);
    assert!(
        !stderr.contains("SCAD_"),
        "a STEP input must not take the SCAD branch:\n{stderr}"
    );
}

/// Regression probe for the harness itself (review finding: pipe back-pressure).
///
/// `run_supervised` drains both pipes concurrently. Before that, a child writing
/// more than a pipe buffer could block before exiting and be killed as if it had
/// hung. This slices an ordinary STL **to stdout** — roughly 800 KB of G-code,
/// far above the pipe buffer — and requires a clean exit plus the full stream, so
/// re-introducing the sequential-drain bug fails here rather than as a mysterious
/// timeout in an unrelated test.
#[test]
fn scad_cli_supervisor_drains_large_stdout() {
    let wedge = workspace_root()
        .join("resources")
        .join("regression_wedge.stl");
    assert!(
        wedge.exists(),
        "the large-output probe needs {}",
        wedge.display()
    );
    let core = core_modules_dir();
    assert!(core.is_dir(), "core guest module directory must exist");

    let mut argv: Vec<OsString> = vec![
        "slice".into(),
        "--model".into(),
        wedge.as_os_str().to_os_string(),
        "--module-dir".into(),
        core.as_os_str().to_os_string(),
        "--no-default-module-paths".into(),
        "--no-integrated-modules".into(),
    ];
    record_invocation("large_stdout.slice", &["<stdout>"]);
    let captured = run_supervised(&argv);
    record_capture("large_stdout.slice", &captured);
    assert_exit(&captured, 0, "large stdout slice");
    assert!(
        captured.stdout.len() > 500_000,
        "the probe must really exceed a pipe buffer; got {} bytes of stdout",
        captured.stdout.len()
    );
    let stdout = String::from_utf8_lossy(&captured.stdout);
    assert!(
        stdout.contains(";LAYER_CHANGE"),
        "the captured stdout must be G-code, not an empty stream"
    );
    // The tail is the point: a truncated prefix of this stream satisfies the
    // length and mid-stream markers above, so only a terminal marker proves the
    // whole stream was drained. Every emitted G-code ends with the config block.
    assert!(
        stdout.trim_end().ends_with("; CONFIG_BLOCK_END"),
        "the captured stdout must be the complete stream, ending with the config \
         block; last 200 bytes were: {:?}",
        stdout
            .char_indices()
            .rev()
            .nth(199)
            .map(|(index, _)| &stdout[index..])
            .unwrap_or(&stdout)
    );
    // `argv` is only extended above; keep the binding used for clarity.
    argv.clear();
}
