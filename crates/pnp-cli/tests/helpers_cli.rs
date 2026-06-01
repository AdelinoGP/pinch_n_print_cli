//! Integration tests for the `pnp_cli mesh repair|decimate|import` subcommands.
//!
//! These exercise CLI plumbing only (argument parsing, exit-code mapping,
//! on-disk I/O). Library-level coverage lives in `crates/slicer-helpers/tests/`.

use std::path::PathBuf;

use assert_cmd::Command;
use tempfile::tempdir;

/// Path to the cube.step fixture shipped with `slicer-helpers`.
fn cube_step_fixture() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // crates/
    p.push("slicer-helpers");
    p.push("tests");
    p.push("resources");
    p.push("cube.step");
    p
}

/// Write a tiny binary STL of a single triangle to `path`.
///
/// Used as a deterministic input mesh for `repair` and `decimate` plumbing
/// tests. Triangle is a unit triangle in the XY plane.
fn write_tiny_stl(path: &std::path::Path) {
    let triangles = [stl_io::Triangle {
        normal: stl_io::Normal::new([0.0, 0.0, 1.0]),
        vertices: [
            stl_io::Vertex::new([0.0, 0.0, 0.0]),
            stl_io::Vertex::new([1.0, 0.0, 0.0]),
            stl_io::Vertex::new([0.0, 1.0, 0.0]),
        ],
    }];
    let mut file = std::fs::File::create(path).expect("create tiny STL");
    stl_io::write_stl(&mut file, triangles.iter()).expect("write tiny STL");
}

/// Write a UV sphere STL with ~`lat*lon*2` triangles.
fn write_sphere_stl(path: &std::path::Path, lat: usize, lon: usize, radius_mm: f32) {
    // Generate vertices.
    let mut verts: Vec<[f32; 3]> = Vec::new();
    for la in 0..=lat {
        let theta = std::f32::consts::PI * (la as f32) / (lat as f32);
        let s = theta.sin();
        let c = theta.cos();
        for lo in 0..=lon {
            let phi = 2.0 * std::f32::consts::PI * (lo as f32) / (lon as f32);
            verts.push([
                radius_mm * s * phi.cos(),
                radius_mm * s * phi.sin(),
                radius_mm * c,
            ]);
        }
    }
    let mut triangles: Vec<stl_io::Triangle> = Vec::new();
    for la in 0..lat {
        for lo in 0..lon {
            let first = la * (lon + 1) + lo;
            let second = first + (lon + 1);
            for tri in [[first, second, first + 1], [second, second + 1, first + 1]] {
                let v0 = verts[tri[0]];
                let v1 = verts[tri[1]];
                let v2 = verts[tri[2]];
                triangles.push(stl_io::Triangle {
                    normal: stl_io::Normal::new([0.0, 0.0, 1.0]),
                    vertices: [
                        stl_io::Vertex::new(v0),
                        stl_io::Vertex::new(v1),
                        stl_io::Vertex::new(v2),
                    ],
                });
            }
        }
    }
    let mut file = std::fs::File::create(path).expect("create sphere STL");
    stl_io::write_stl(&mut file, triangles.iter()).expect("write sphere STL");
}

// ────────────────────────────── repair ──────────────────────────────

#[test]
fn repair_clean_cube_exits_zero() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("in.stl");
    let output = dir.path().join("out.stl");
    write_tiny_stl(&input);

    Command::cargo_bin("pnp_cli")
        .expect("binary built")
        .args(["mesh", "repair", "--input"])
        .arg(&input)
        .arg("--output")
        .arg(&output)
        .assert()
        .success();

    let written = std::fs::metadata(&output).expect("output should exist");
    assert!(written.len() > 0, "output STL must be non-empty");
}

#[test]
fn repair_missing_input_exits_two() {
    let dir = tempdir().expect("tempdir");
    let missing = dir.path().join("does_not_exist.stl");
    let output = dir.path().join("out.stl");

    Command::cargo_bin("pnp_cli")
        .expect("binary built")
        .args(["mesh", "repair", "--input"])
        .arg(&missing)
        .arg("--output")
        .arg(&output)
        .assert()
        .code(2);
}

// ────────────────────────────── decimate ──────────────────────────────

#[test]
fn decimate_ratio_half_exits_zero_with_target_reached() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("sphere.stl");
    let output = dir.path().join("dec.stl");
    // 16 × 16 × 2 = 512 triangles, comfortably above the 0.5 target.
    write_sphere_stl(&input, 16, 16, 25.0);

    let assert_result = Command::cargo_bin("pnp_cli")
        .expect("binary built")
        .args(["mesh", "decimate", "--input"])
        .arg(&input)
        .arg("--output")
        .arg(&output)
        .args(["--target-ratio", "0.5", "--max-error", "10.0", "--stats"])
        .assert();

    // Exit 0 (target reached) is the contract per docs/13.
    let output_obj = assert_result.success().get_output().clone();

    let stderr = String::from_utf8_lossy(&output_obj.stderr);
    let done_line = stderr
        .lines()
        .find(|l| l.contains("\"event\":\"done\""))
        .expect("expected a done event on stderr");
    let v: serde_json::Value =
        serde_json::from_str(done_line).expect("done event should be valid JSON");
    assert_eq!(v["target_reached"], serde_json::Value::Bool(true));
    let final_count = v["final_triangle_count"].as_u64().expect("count u64");
    let original_count = v["original_triangle_count"].as_u64().expect("orig u64");
    assert!(
        final_count <= original_count / 2 + original_count / 10,
        "expected ≤ 50% + 10% tolerance, got {final_count}/{original_count}"
    );
}

#[test]
fn decimate_conflicting_targets_clap_error() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("in.stl");
    let output = dir.path().join("out.stl");
    write_tiny_stl(&input);

    Command::cargo_bin("pnp_cli")
        .expect("binary built")
        .args(["mesh", "decimate", "--input"])
        .arg(&input)
        .arg("--output")
        .arg(&output)
        .args(["--target-count", "100", "--target-ratio", "0.5"])
        .assert()
        .failure(); // clap rejects before any work runs
}

// ────────────────────────────── import ──────────────────────────────

#[test]
fn import_cube_step_exits_zero() {
    let dir = tempdir().expect("tempdir");
    let output = dir.path().join("cube.stl");
    let fixture = cube_step_fixture();
    assert!(
        fixture.exists(),
        "fixture must exist at {}",
        fixture.display()
    );

    Command::cargo_bin("pnp_cli")
        .expect("binary built")
        .args(["mesh", "import", "--input"])
        .arg(&fixture)
        .arg("--output")
        .arg(&output)
        .assert()
        .success();

    let written = std::fs::metadata(&output).expect("output should exist");
    assert!(written.len() > 0, "output STL must be non-empty");
}

#[test]
fn import_nonexistent_exits_two() {
    let dir = tempdir().expect("tempdir");
    let bogus = dir.path().join("nope.step");
    let output = dir.path().join("out.stl");

    Command::cargo_bin("pnp_cli")
        .expect("binary built")
        .args(["mesh", "import", "--input"])
        .arg(&bogus)
        .arg("--output")
        .arg(&output)
        .assert()
        .code(2);
}

/// Path to the assembly.step fixture shipped with `slicer-helpers`
/// (two distinct STEP solids).
fn assembly_step_fixture() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // crates/
    p.push("slicer-helpers");
    p.push("tests");
    p.push("resources");
    p.push("assembly.step");
    p
}

/// AC-2: Importing a two-solid STEP file with `--output-format 3mf` and
/// WITHOUT `--merge-components` must produce exactly ONE `out.3mf` file
/// containing exactly 2 objects (one per solid).
#[test]
fn import_multi_solid_step_to_single_3mf_two_objects() {
    let dir = tempdir().expect("tempdir");
    let output = dir.path().join("out.3mf");
    let fixture = assembly_step_fixture();
    assert!(
        fixture.exists(),
        "fixture must exist at {}",
        fixture.display()
    );

    // Run import — no --merge-components flag.
    // Exit code 0 (clean) or 1 (warnings) are both success for import.
    let status = Command::cargo_bin("pnp_cli")
        .expect("binary built")
        .args(["mesh", "import", "--input"])
        .arg(&fixture)
        .arg("--output")
        .arg(&output)
        .args(["--output-format", "3mf"])
        .assert()
        .get_output()
        .status
        .code()
        .unwrap_or(99);
    assert!(
        status == 0 || status == 1,
        "expected exit code 0 or 1 (warnings), got {status}"
    );

    // Exactly one output file — no _0.3mf / _1.3mf split.
    assert!(output.exists(), "out.3mf must exist");
    assert!(
        !dir.path().join("out_0.3mf").exists(),
        "out_0.3mf must NOT exist (no per-solid split for 3MF)"
    );
    assert!(
        !dir.path().join("out_1.3mf").exists(),
        "out_1.3mf must NOT exist (no per-solid split for 3MF)"
    );

    // The single 3MF must contain exactly 2 objects (one per solid).
    let mesh = slicer_model_io::load_model(&output).expect("out.3mf must be loadable");
    assert_eq!(
        mesh.objects.len(),
        2,
        "expected 2 objects in combined 3MF, got {}",
        mesh.objects.len()
    );
}

// ────────────────────────────── convert ──────────────────────────────

/// Write a binary STL containing two disjoint unit cubes far apart.
///
/// Cube A is centred at (0,0,0), Cube B is centred at (100,0,0).
/// Each cube is represented as 12 triangles (2 per face × 6 faces).
fn write_two_cube_stl(path: &std::path::Path) {
    fn cube_triangles(ox: f32, oy: f32, oz: f32, s: f32) -> Vec<stl_io::Triangle> {
        // 8 corners of the cube offset by (ox, oy, oz), side length s.
        let v = |dx: f32, dy: f32, dz: f32| -> stl_io::Vertex {
            stl_io::Vertex::new([ox + dx * s, oy + dy * s, oz + dz * s])
        };
        let tri = |a: stl_io::Vertex,
                   b: stl_io::Vertex,
                   c: stl_io::Vertex,
                   nx: f32,
                   ny: f32,
                   nz: f32|
         -> stl_io::Triangle {
            stl_io::Triangle {
                normal: stl_io::Normal::new([nx, ny, nz]),
                vertices: [a, b, c],
            }
        };
        vec![
            // -Z face
            tri(v(0., 0., 0.), v(1., 0., 0.), v(1., 1., 0.), 0., 0., -1.),
            tri(v(0., 0., 0.), v(1., 1., 0.), v(0., 1., 0.), 0., 0., -1.),
            // +Z face
            tri(v(0., 0., 1.), v(1., 1., 1.), v(1., 0., 1.), 0., 0., 1.),
            tri(v(0., 0., 1.), v(0., 1., 1.), v(1., 1., 1.), 0., 0., 1.),
            // -X face
            tri(v(0., 0., 0.), v(0., 1., 0.), v(0., 1., 1.), -1., 0., 0.),
            tri(v(0., 0., 0.), v(0., 1., 1.), v(0., 0., 1.), -1., 0., 0.),
            // +X face
            tri(v(1., 0., 0.), v(1., 1., 1.), v(1., 1., 0.), 1., 0., 0.),
            tri(v(1., 0., 0.), v(1., 0., 1.), v(1., 1., 1.), 1., 0., 0.),
            // -Y face
            tri(v(0., 0., 0.), v(1., 0., 1.), v(1., 0., 0.), 0., -1., 0.),
            tri(v(0., 0., 0.), v(0., 0., 1.), v(1., 0., 1.), 0., -1., 0.),
            // +Y face
            tri(v(0., 1., 0.), v(1., 1., 0.), v(1., 1., 1.), 0., 1., 0.),
            tri(v(0., 1., 0.), v(1., 1., 1.), v(0., 1., 1.), 0., 1., 0.),
        ]
    }
    // Cube A at origin, Cube B 100 units away on X — fully disjoint.
    let mut tris = cube_triangles(0.0, 0.0, 0.0, 1.0);
    tris.extend(cube_triangles(100.0, 0.0, 0.0, 1.0));
    let mut file = std::fs::File::create(path).expect("create two-cube STL");
    stl_io::write_stl(&mut file, tris.iter()).expect("write two-cube STL");
}

/// AC-3: splitting two disjoint cubes yields 2 objects; merging yields 1.
#[test]
fn convert_split_vs_merge_object_count() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("two_cubes.stl");
    write_two_cube_stl(&input);

    // --- split (default: no --merge-components) ---
    let output_split = dir.path().join("split.3mf");
    Command::cargo_bin("pnp_cli")
        .expect("binary built")
        .args(["mesh", "convert", "--input"])
        .arg(&input)
        .arg("--output")
        .arg(&output_split)
        .assert()
        .success();

    let split_mesh =
        slicer_model_io::load_model(&output_split).expect("split.3mf must be loadable");
    assert_eq!(
        split_mesh.objects.len(),
        2,
        "expected 2 objects after split, got {}",
        split_mesh.objects.len()
    );

    // --- merge (--merge-components keeps input object count = 1) ---
    let output_merge = dir.path().join("merge.3mf");
    Command::cargo_bin("pnp_cli")
        .expect("binary built")
        .args(["mesh", "convert", "--input"])
        .arg(&input)
        .arg("--output")
        .arg(&output_merge)
        .arg("--merge-components")
        .assert()
        .success();

    let merge_mesh =
        slicer_model_io::load_model(&output_merge).expect("merge.3mf must be loadable");
    assert_eq!(
        merge_mesh.objects.len(),
        1,
        "expected 1 object after merge, got {}",
        merge_mesh.objects.len()
    );
}

/// AC-N1: STEP/STP input is rejected with exit code 2 and mentions `import`.
#[test]
fn convert_rejects_step_input() {
    let dir = tempdir().expect("tempdir");
    // The file does NOT need to exist — rejection happens on extension.
    let fake_step = dir.path().join("something.step");
    let output = dir.path().join("out.3mf");

    let output_obj = Command::cargo_bin("pnp_cli")
        .expect("binary built")
        .args(["mesh", "convert", "--input"])
        .arg(&fake_step)
        .arg("--output")
        .arg(&output)
        .assert()
        .code(2)
        .get_output()
        .clone();

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output_obj.stdout),
        String::from_utf8_lossy(&output_obj.stderr),
    );
    assert!(
        combined.to_ascii_lowercase().contains("import"),
        "expected output to mention `import`, got: {combined}"
    );
}
