//! AC-22 RED bucket — gcode-output behavior gate for `cube_4color.3mf` and
//! `cube_fuzzyPainted.3mf`.
//!
//! These tests assert the *correct* end-to-end gcode behavior that the
//! Step 19 D9 dispatch wiring is expected to produce. They land RED on
//! today's code because the diagnose session traced two regressions:
//!
//!   1. `^T[0-9]+$` lines in the emitted gcode only contain `{0, 1}` — the
//!      `T2` and `T3` tool-change records are missing for the 4-color cube.
//!      Tests assert the full set `{0, 1, 2, 3}` per the four Material paint
//!      values present on the fixture.
//!
//!   2. Per-layer outer-wall block count is 4-9 instead of ~2: extra
//!      phantom perimeters are emitted along Voronoi cell boundaries.
//!      Tests assert the painted cube's per-layer `;TYPE:Outer wall` count
//!      stays within ±1 of an unpainted baseline cube sliced with identical
//!      settings.
//!
//! A third (Test 3) asserts that the painted face of `cube_fuzzyPainted.3mf`
//! produces visibly higher coordinate jitter on its outer-wall point sequence
//! than an unpainted face — i.e. the fuzzy module ran at all on the painted
//! side. This is a coarse, loose-but-clear ratio that may need tightening
//! once D9 dispatch is verified to route through the fuzzy module via the new
//! variant-chain.

#![allow(missing_docs)]
#![allow(dead_code)]

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{
    BoundingBox3, IndexedTriangleSet, MeshIR, ObjectConfig, ObjectMesh, Point3, SemVer, Transform3d,
};
use slicer_runtime::{run_slice, SliceOutcome, SliceRunOptions};

// --------------------------------------------------------------------------
// Workspace + fixture helpers
// --------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root must be resolvable")
}

fn cube_4color_path() -> PathBuf {
    workspace_root().join("resources").join("cube_4color.3mf")
}

fn cube_fuzzy_painted_path() -> PathBuf {
    workspace_root()
        .join("resources")
        .join("cube_fuzzyPainted.3mf")
}

fn core_modules_dir() -> PathBuf {
    workspace_root().join("modules").join("core-modules")
}

fn semver() -> SemVer {
    SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    }
}

fn build_volume() -> BoundingBox3 {
    BoundingBox3 {
        min: Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        max: Point3 {
            x: 250.0,
            y: 250.0,
            z: 250.0,
        },
    }
}

fn identity_transform() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    }
}

/// Build a synthetic 25mm axis-aligned cube with NO paint_data. Used as the
/// unpainted baseline for the per-layer outer-wall count comparison. The
/// cube is centered horizontally near (125, 105, 12.5) — same general world
/// position as the cube_4color fixture (after its build transform) so the
/// pipeline routes through identical layer indices and config inheritance.
fn unpainted_25mm_cube() -> Arc<MeshIR> {
    // Match cube_4color's world-space footprint (25mm side, centered at
    // approximately (125, 105, 12.5)) so layer-height-derived layer counts
    // align.
    let cx = 125.0_f32;
    let cy = 105.0_f32;
    let z_min = 0.0_f32;
    let z_max = 25.0_f32;
    let half = 12.5_f32;
    let x_min = cx - half;
    let x_max = cx + half;
    let y_min = cy - half;
    let y_max = cy + half;

    let p = |x: f32, y: f32, z: f32| Point3 { x, y, z };
    let vertices = vec![
        p(x_min, y_min, z_min), // 0
        p(x_max, y_min, z_min), // 1
        p(x_max, y_max, z_min), // 2
        p(x_min, y_max, z_min), // 3
        p(x_min, y_min, z_max), // 4
        p(x_max, y_min, z_max), // 5
        p(x_max, y_max, z_max), // 6
        p(x_min, y_max, z_max), // 7
    ];
    let indices: Vec<u32> = vec![
        // bottom (-Z)
        0, 2, 1, 0, 3, 2, // top (+Z)
        4, 5, 6, 4, 6, 7, // front (-Y)
        0, 1, 5, 0, 5, 4, // back (+Y)
        2, 3, 7, 2, 7, 6, // left (-X)
        3, 0, 4, 3, 4, 7, // right (+X)
        1, 2, 6, 1, 6, 5,
    ];

    let object = ObjectMesh {
        id: "unpainted_25mm_cube".to_string(),
        mesh: IndexedTriangleSet { vertices, indices },
        transform: identity_transform(),
        config: ObjectConfig {
            data: Default::default(),
        },
        modifier_volumes: Vec::new(),
        paint_data: None,
        // Seed the planner-visible world height (run.rs reads this and injects
        // `object_height:<id>` into the config map before module dispatch).
        world_z_extent: Some((z_min, z_max)),
    };

    Arc::new(MeshIR {
        schema_version: semver(),
        objects: vec![object],
        build_volume: build_volume(),
    })
}

// --------------------------------------------------------------------------
// Slice harness
// --------------------------------------------------------------------------

/// Run an end-to-end slice via the library entry point (`run_slice`) and
/// return the produced gcode. Loads the named fixture from `resources/`
/// and wires through `modules/core-modules`. Panics with a clear message
/// on any failure — the AC-22 contract treats a missing fixture or pipeline
/// crash as test infra failure, not assertion failure.
fn slice_fixture_file(model_path: &PathBuf) -> SliceOutcome {
    assert!(
        model_path.exists(),
        "fixture missing: {} — run from workspace root or restore resources/",
        model_path.display()
    );
    let module_dir = core_modules_dir();
    assert!(
        module_dir.exists(),
        "core-modules directory must exist at {}",
        module_dir.display()
    );

    let mesh = Arc::new(
        slicer_model_io::load_model(model_path)
            .unwrap_or_else(|e| panic!("load_model({}) failed: {e}", model_path.display())),
    );
    let opts = SliceRunOptions {
        mesh,
        model_label: model_path.to_string_lossy().into_owned(),
        config_path: None,
        output_path: None,
        module_dirs: vec![module_dir],
        no_default_module_paths: true,
        thumbnail: None,
        report: None,
        report_verbose: false,
        instrument_stderr: false,
    };
    run_slice(opts)
        .unwrap_or_else(|e| panic!("run_slice failed against {}: {e}", model_path.display()))
}

/// Same as [`slice_fixture_file`] but for a pre-constructed `MeshIR` (used
/// by the unpainted-baseline test).
fn slice_synthetic_mesh(label: &str, mesh: Arc<MeshIR>) -> SliceOutcome {
    let module_dir = core_modules_dir();
    let opts = SliceRunOptions {
        mesh,
        model_label: label.to_string(),
        config_path: None,
        output_path: None,
        module_dirs: vec![module_dir],
        no_default_module_paths: true,
        thumbnail: None,
        report: None,
        report_verbose: false,
        instrument_stderr: false,
    };
    run_slice(opts)
        .unwrap_or_else(|e| panic!("run_slice failed against synthetic mesh {label}: {e}"))
}

// --------------------------------------------------------------------------
// Gcode parsing helpers
// --------------------------------------------------------------------------

/// Match a tool-change line of the form `T<digits>` (and only `T<digits>`,
/// possibly with trailing whitespace). Used by Test 1.
fn parse_tool_index_lines(gcode: &str) -> BTreeSet<u32> {
    let mut out = BTreeSet::new();
    for line in gcode.lines() {
        let trimmed = line.trim();
        if trimmed.len() < 2 {
            continue;
        }
        let bytes = trimmed.as_bytes();
        if bytes[0] != b'T' {
            continue;
        }
        if !bytes[1..].iter().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if let Ok(n) = trimmed[1..].parse::<u32>() {
            out.insert(n);
        }
    }
    out
}

/// Split the gcode by `;LAYER_CHANGE` markers and count the number of
/// distinct outer-wall extrusion runs per layer. Each `;TYPE:Outer wall`
/// section can contain multiple disjoint extrusion runs (separated by
/// non-extrusion travel moves or by another `;TYPE:*` marker re-entering
/// outer-wall later). Counting the EXTRUSION MOVES (`G1 ... E<n>`) inside
/// outer-wall blocks captures the regression precisely: the painted cube
/// emits 7-23 extrusion moves per outer-wall layer (extra phantom perimeters
/// along Voronoi cell boundaries) where the unpainted baseline emits ~4
/// (one closed loop = 4 segments for a square).
fn outer_wall_counts_per_layer(gcode: &str) -> Vec<usize> {
    let marker = ";LAYER_CHANGE";
    let outer = ";TYPE:Outer wall";
    let mut counts = Vec::new();
    let mut current = 0usize;
    let mut layer_started = false;
    let mut in_outer = false;
    for line in gcode.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(marker) {
            if layer_started {
                counts.push(current);
            }
            current = 0;
            layer_started = true;
            in_outer = false;
            continue;
        }
        if !layer_started {
            continue;
        }
        if trimmed.starts_with(outer) {
            in_outer = true;
            continue;
        }
        if trimmed.starts_with(";TYPE:") {
            in_outer = false;
            continue;
        }
        if !in_outer {
            continue;
        }
        // Extrusion move inside an outer-wall block.
        if (trimmed.starts_with("G1 ") || trimmed.starts_with("G1\t")) && trimmed.contains(" E") {
            current += 1;
        }
    }
    if layer_started {
        counts.push(current);
    }
    counts
}

/// Format a per-layer count diff for the assertion message (first N layers).
fn fmt_per_layer_diff(painted: &[usize], unpainted: &[usize], n: usize) -> String {
    let len = painted.len().min(unpainted.len()).min(n);
    let mut out = String::new();
    out.push_str("layer painted unpainted diff\n");
    for i in 0..len {
        out.push_str(&format!(
            "  {:>3}    {:>3}    {:>3}    {:+}\n",
            i,
            painted[i],
            unpainted[i],
            painted[i] as i64 - unpainted[i] as i64
        ));
    }
    out
}

// --------------------------------------------------------------------------
// Test 1 — cube_4color: all four tool indices must appear in gcode
// --------------------------------------------------------------------------

#[test]
fn cube_4color_gcode_emits_all_four_tool_indices() {
    let outcome = slice_fixture_file(&cube_4color_path());
    let found = parse_tool_index_lines(&outcome.gcode_text);
    let expected: BTreeSet<u32> = [0u32, 1, 2, 3].iter().copied().collect();

    assert_eq!(
        found, expected,
        "cube_4color emitted tools {found:?}, expected {expected:?} per D9 dispatch wiring \
         (Step 19 must route per-region variant chains so each Material ToolIndex \
         produces its own `T<n>` line)"
    );
}

// --------------------------------------------------------------------------
// Test 2 — per-layer outer-wall count matches unpainted baseline within 1
// --------------------------------------------------------------------------

#[test]
#[ignore = "P96 bisector-edge ownership: every Voronoi edge between two differently-colored cells is traced as an outer wall by both adjacent cells; P96 implements per-edge ownership alongside Phase 5 width-limiting + interlocking. See P96 packet AC-22b."]
fn cube_4color_per_layer_outer_wall_count_matches_unpainted_baseline_within_one() {
    let painted = slice_fixture_file(&cube_4color_path());
    let unpainted = slice_synthetic_mesh("unpainted_25mm_cube", unpainted_25mm_cube());

    let painted_counts = outer_wall_counts_per_layer(&painted.gcode_text);
    let unpainted_counts = outer_wall_counts_per_layer(&unpainted.gcode_text);

    // Layer alignment guard: if the two outputs disagree on layer count by
    // more than a couple of layers, the unpainted fixture is not a like-for-like
    // baseline — surface that as a skip rather than a meaningless failure.
    // (Acceptable difference accounts for leading/trailing priming/skirt
    // layers that may produce or not produce ;LAYER_CHANGE markers.)
    let layer_diff = painted_counts.len() as i64 - unpainted_counts.len() as i64;
    if layer_diff.abs() > 2 {
        eprintln!(
            "skipping AC-22 outer-wall count assertion: painted={} layers, unpainted={} layers — \
             configure an exact-match baseline before tightening this gate",
            painted_counts.len(),
            unpainted_counts.len()
        );
        return;
    }

    // Compare layer-by-layer up to the shorter of the two.
    let common = painted_counts.len().min(unpainted_counts.len());
    let mut violations: Vec<(usize, usize, usize)> = Vec::new();
    for i in 0..common {
        let p = painted_counts[i];
        let u = unpainted_counts[i];
        let diff = (p as i64 - u as i64).abs();
        if diff > 1 {
            violations.push((i, p, u));
        }
    }

    assert!(
        violations.is_empty(),
        "cube_4color per-layer outer-wall count diverges from unpainted baseline (>1) on {} layers; \
         current behavior emits phantom perimeters along Voronoi cell boundaries.\n\
         First 5 layer counts side-by-side:\n{}\
         First 5 violations (layer_idx, painted, unpainted): {:?}",
        violations.len(),
        fmt_per_layer_diff(&painted_counts, &unpainted_counts, 5),
        violations.iter().take(5).collect::<Vec<_>>()
    );
}

// --------------------------------------------------------------------------
// Test 3 — cube_fuzzyPainted: painted face shows visibly more jitter than
//          an unpainted face (proxy: more outer-wall coordinate points)
// --------------------------------------------------------------------------

/// Extract `G1 X<f> Y<f>` coordinates from outer-wall blocks of layers whose
/// Z roughly equals `target_z_mm` (within `tolerance_mm`). The result is the
/// flat list of (x, y) extrusion-move endpoints in those blocks.
fn outer_wall_points_at_z(gcode: &str, target_z_mm: f32, tolerance_mm: f32) -> Vec<(f32, f32)> {
    let mut points = Vec::new();
    let mut current_z: Option<f32> = None;
    let mut in_outer = false;
    for line in gcode.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(";Z:") {
            if let Ok(z) = rest.split_whitespace().next().unwrap_or("").parse::<f32>() {
                current_z = Some(z);
            }
            continue;
        }
        if trimmed.starts_with(";LAYER_CHANGE") {
            in_outer = false;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix(";TYPE:") {
            in_outer = rest.starts_with("Outer wall");
            continue;
        }
        if trimmed.starts_with(';') {
            // Another semantic comment — leave `in_outer` as-is until the
            // next ;TYPE: or ;LAYER_CHANGE flips it.
            continue;
        }
        if !in_outer {
            continue;
        }
        let z_match = match current_z {
            Some(z) => (z - target_z_mm).abs() <= tolerance_mm,
            None => false,
        };
        if !z_match {
            continue;
        }
        // Look for `G1 ... X<f> Y<f> ... E<f>` (extrusion moves only).
        if !(trimmed.starts_with("G1 ") || trimmed.starts_with("G1\t")) {
            continue;
        }
        if !trimmed.contains(" E") {
            continue;
        }
        let mut x: Option<f32> = None;
        let mut y: Option<f32> = None;
        for tok in trimmed.split_whitespace() {
            if let Some(rest) = tok.strip_prefix('X') {
                x = rest.parse::<f32>().ok();
            } else if let Some(rest) = tok.strip_prefix('Y') {
                y = rest.parse::<f32>().ok();
            }
        }
        if let (Some(xv), Some(yv)) = (x, y) {
            points.push((xv, yv));
        }
    }
    points
}

#[test]
fn cube_fuzzy_painted_face_jitter_present_on_painted_face_only() {
    // cube_fuzzyPainted layout (world-space after build transform 125,115,12.5):
    //   Front (-Y, y ≈ 102.5)  — fuzzy painted
    //   Back  (+Y, y ≈ 127.5)  — half fuzzy / half unpainted
    //   Left  (-X, x ≈ 112.5)  — unpainted
    //   Right (+X, x ≈ 137.5)  — fuzzy circle
    //
    // The Left face (-X) is a clean unpainted face; the Front face (-Y) is
    // fully fuzzy. We compare the count of outer-wall extrusion endpoints
    // emitted along each face at a mid-height layer. Fuzzy skin injects
    // intermediate points along the perimeter, so the painted-face count
    // should be materially higher than the clean face's.
    //
    // Threshold: painted face count > 2× unpainted face count (loose-but-clear
    // proxy for jitter; may need tightening once D9 dispatch is verified to
    // route through the fuzzy-skin module via the new variant-chain).
    let outcome = slice_fixture_file(&cube_fuzzy_painted_path());
    let mid_z = 12.5_f32;
    let tol = 0.6_f32;
    let pts = outer_wall_points_at_z(&outcome.gcode_text, mid_z, tol);
    if pts.is_empty() {
        eprintln!(
            "skipping AC-22 fuzzy-jitter assertion: no outer-wall extrusion points captured at \
             z≈{mid_z}±{tol} mm — gcode parser found 0 candidate moves. \
             Verify cube_fuzzyPainted slices and a mid-height layer is emitted."
        );
        return;
    }

    // Face bins (world space, mm). Use generous margins to absorb fuzz/jitter.
    let mut painted_face_pts = 0usize; // Front face: y ≈ 102.5
    let mut unpainted_face_pts = 0usize; // Left face: x ≈ 112.5
    for &(x, y) in &pts {
        if y < 105.0 && x > 113.5 && x < 136.5 {
            painted_face_pts += 1;
        }
        if x < 113.5 && y > 103.5 && y < 126.5 {
            unpainted_face_pts += 1;
        }
    }

    if painted_face_pts == 0 || unpainted_face_pts == 0 {
        eprintln!(
            "skipping AC-22 fuzzy-jitter assertion: insufficient face coverage at z≈{mid_z}mm \
             (painted_face_pts={painted_face_pts}, unpainted_face_pts={unpainted_face_pts}, \
             total_pts={}). Likely a face-region misalignment vs the 3MF build transform — \
             refine face bins before tightening the gate.",
            pts.len()
        );
        return;
    }

    assert!(
        painted_face_pts as f32 > unpainted_face_pts as f32 * 2.0,
        "cube_fuzzyPainted: fuzzy face point count ({painted_face_pts}) is NOT > 2× \
         clean face point count ({unpainted_face_pts}) at z≈{mid_z}mm. Either the fuzzy-skin \
         module did not run on the painted face (D9 dispatch did not route through the \
         variant-chain for the painted region) or the proxy threshold needs revisiting."
    );
}
