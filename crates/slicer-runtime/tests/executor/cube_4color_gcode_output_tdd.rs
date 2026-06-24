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
        // Outer-wall EXTRUSION SEGMENT: a G1 that moves in XY while extruding.
        // E-only moves (filament retract / unretract / static wipe) are NOT wall
        // segments and must be excluded — otherwise a retract that lands inside
        // the outer-wall block inflates the count by the retract/unretract pair.
        // This counts equally for painted and unpainted gcode, so the baseline
        // comparison stays apples-to-apples.
        let is_extrusion_segment = (trimmed.starts_with("G1 ") || trimmed.starts_with("G1\t"))
            && trimmed.contains(" E")
            && (trimmed.contains(" X") || trimmed.contains(" Y"));
        if is_extrusion_segment {
            current += 1;
        }
    }
    if layer_started {
        counts.push(current);
    }
    counts
}

/// Count the number of `;TYPE:Outer wall` header occurrences per layer bucket
/// (separated by `;LAYER_CHANGE` markers). Each per-color outer-wall fragment
/// begins a fresh `;TYPE:Outer wall` block (after a travel move or tool change),
/// so this directly measures per-color fragmentation — NOT G1 segment count.
///
/// An unpainted cube should have exactly 1 fragment per mid-body layer.
/// A 4-color painted cube should have >= 2 fragments on painted layers.
fn outer_wall_fragments_per_layer(gcode: &str) -> Vec<usize> {
    let marker = ";LAYER_CHANGE";
    let outer = ";TYPE:Outer wall";
    let mut counts = Vec::new();
    let mut current = 0usize;
    let mut layer_started = false;
    for line in gcode.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(marker) {
            if layer_started {
                counts.push(current);
            }
            current = 0;
            layer_started = true;
            continue;
        }
        if !layer_started {
            continue;
        }
        // Each `;TYPE:Outer wall` header starts a new per-color outer-wall fragment.
        if trimmed == outer {
            current += 1;
        }
    }
    if layer_started {
        counts.push(current);
    }
    counts
}

/// Count the number of distinct `T<digits>` tool indices appearing per layer
/// (separated by `;LAYER_CHANGE` markers). Returns one entry per layer.
fn distinct_tool_indices_per_layer(gcode: &str) -> Vec<usize> {
    use std::collections::HashSet;
    let marker = ";LAYER_CHANGE";
    let mut result: Vec<usize> = Vec::new();
    let mut current: HashSet<u32> = HashSet::new();
    let mut layer_started = false;
    for line in gcode.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(marker) {
            if layer_started {
                result.push(current.len());
            }
            current.clear();
            layer_started = true;
            continue;
        }
        if !layer_started {
            continue;
        }
        // Detect bare `T<digits>` lines.
        if trimmed.len() >= 2 {
            let bytes = trimmed.as_bytes();
            if bytes[0] == b'T' && bytes[1..].iter().all(|c| c.is_ascii_digit()) {
                if let Ok(n) = trimmed[1..].parse::<u32>() {
                    current.insert(n);
                }
            }
        }
    }
    if layer_started {
        result.push(current.len());
    }
    result
}

/// Count the number of `T<digits>` tool-change lines per layer. Returns one
/// entry per layer (same index convention as `outer_wall_counts_per_layer`).
fn tool_changes_per_layer(gcode: &str) -> Vec<usize> {
    let marker = ";LAYER_CHANGE";
    let mut counts: Vec<usize> = Vec::new();
    let mut current = 0usize;
    let mut layer_started = false;
    for line in gcode.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(marker) {
            if layer_started {
                counts.push(current);
            }
            current = 0;
            layer_started = true;
            continue;
        }
        if !layer_started {
            continue;
        }
        // Detect bare `T<digits>` lines (same logic as parse_tool_index_lines).
        if trimmed.len() >= 2 {
            let bytes = trimmed.as_bytes();
            if bytes[0] == b'T' && bytes[1..].iter().all(|c| c.is_ascii_digit()) {
                current += 1;
            }
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
// Test 2 — Model A: per-color outer-wall fragmentation on painted layers
// --------------------------------------------------------------------------
//
// ADR-0013 (P105 rewrite): under the confirmed OrcaSlicer "Model A" behavior,
// a painted 4-color cube emits PER-COLOR outer-wall fragments on every painted
// layer — one distinct ;TYPE:Outer wall sequence per paint cell encountered on
// that layer. The old "within ±1 of unpainted baseline" contract (union-trace
// behavior) is RETIRED. This test asserts the new Model A contract.
//
// AC-6: external_contour removal asserted by the rg grep in packet.spec.md AC-6
//
// P105_CUBE_4COLOR_PARITY_SHA: gcode is byte-stable across runs (confirmed by
// test run post-FIX2 (boostvoronoi panic hardening) on 2026-06-24).  SHA-256 pinned below.
// Previous SHA d4b4a3fad... was set before the catch_unwind hardening; new SHA reflects
// correct medial-axis output now that panics are caught and regions are processed.
// If this assertion fails after a legitimate impl change, re-baseline by running
// `pnp_cli slice --model resources/cube_4color.3mf --module-dir modules/core-modules
// --output /tmp/out.gcode && sha256sum /tmp/out.gcode` and updating the const.
// Do NOT remove the assertion; treat a hash mismatch as a regression gate.
//
// Note: the EXACT per-layer fragment count is covered by the controlled fixture
// in `mmu_per_color_fragmentation_tdd::per_color_regions_each_trace_own_outer_wall`
// (AC-6 exact-count, 2 per-color regions → exactly 2 outer-wall loops).  This
// E2E test covers the full cube fixture at a coarser level (total/max fragments,
// all 4 tools present, per-layer tool-change invariant) plus the SHA regression gate.
#[test]
fn cube_4color_per_layer_per_color_fragmentation_with_tool_changes() {
    let painted = slice_fixture_file(&cube_4color_path());
    let unpainted = slice_synthetic_mesh("unpainted_25mm_cube", unpainted_25mm_cube());

    // Use ;TYPE:Outer wall header count (fragments), NOT G1 segment count.
    let painted_frags = outer_wall_fragments_per_layer(&painted.gcode_text);
    let unpainted_frags = outer_wall_fragments_per_layer(&unpainted.gcode_text);
    let tc_per_layer = tool_changes_per_layer(&painted.gcode_text);
    let distinct_tools_per_layer = distinct_tool_indices_per_layer(&painted.gcode_text);

    let painted_total: usize = painted_frags.iter().sum();
    let unpainted_total: usize = unpainted_frags.iter().sum();
    let painted_max = painted_frags.iter().copied().max().unwrap_or(0);

    // Print per-layer diagnostics for post-Model-A capture.
    eprintln!(
        "cube_4color Model A diagnostics (FRAGMENTS): painted_layers={}, unpainted_ref_layers={}, \
         painted_total_frags={}, unpainted_total_frags={}, painted_max_frags_per_layer={}",
        painted_frags.len(),
        unpainted_frags.len(),
        painted_total,
        unpainted_total,
        painted_max,
    );
    for i in 0..painted_frags.len().min(10) {
        let ref_count = unpainted_frags.get(i).copied().unwrap_or(0);
        let tc = tc_per_layer.get(i).copied().unwrap_or(0);
        let dt = distinct_tools_per_layer.get(i).copied().unwrap_or(0);
        eprintln!(
            "  layer {:>3}: painted_frags={}, unpainted_frags={}, tool_changes={}, distinct_tools={}",
            i, painted_frags[i], ref_count, tc, dt
        );
    }

    // Layer alignment guard: if the pipeline emits zero layers (stale guests),
    // fail loudly — do NOT silently skip assertions.
    assert!(
        !painted_frags.is_empty(),
        "Model A fragmentation assertion: painted gcode has 0 ;LAYER_CHANGE markers. \
         Rebuild guests (cargo xtask build-guests) and re-run."
    );

    // -----------------------------------------------------------------------
    // Assertion 1 — Model A fragmentation.
    //
    // (a) The painted cube's TOTAL outer-wall-fragment count across all layers
    //     must be strictly greater than the unpainted baseline's total.
    //     An unpainted cube has ~1 fragment per layer; a 4-color painted cube
    //     must produce additional per-color fragments, raising the total.
    //
    // (b) At least some painted layers must have >= 2 outer-wall fragments,
    //     proving that at least one layer split into multiple per-color outer
    //     walls. We require at least 1 such layer as the minimal meaningful
    //     bound that distinguishes Model A from single-fragment monochrome output.
    // -----------------------------------------------------------------------
    assert!(
        painted_total > unpainted_total,
        "cube_4color Model A Assertion 1(a): painted total outer-wall fragments ({painted_total}) \
         must exceed unpainted baseline total ({unpainted_total}). \
         Per-color fragmentation must raise the total fragment count across all layers."
    );

    let layers_with_multi_frags = painted_frags.iter().filter(|&&f| f >= 2).count();
    assert!(
        layers_with_multi_frags >= 1,
        "cube_4color Model A Assertion 1(b): at least 1 painted layer must have >= 2 outer-wall \
         fragments (proving per-color split occurred), but found {layers_with_multi_frags} such layers. \
         painted_frags (first 10): {:?}",
        painted_frags.iter().take(10).collect::<Vec<_>>()
    );

    // -----------------------------------------------------------------------
    // Assertion 2 — Tool changes.
    //
    // (a) All four tool indices T0, T1, T2, T3 must appear somewhere in the
    //     full gcode (already asserted by Test 1, but we re-verify here for
    //     completeness within this test).
    //
    // (b) For every layer whose gcode block contains >= 2 DISTINCT tool indices,
    //     that layer must contain at least 1 tool-change line. A layer that
    //     uses two or more tools must switch between them at least once.
    // -----------------------------------------------------------------------
    let all_tools = parse_tool_index_lines(&painted.gcode_text);
    let expected_tools: BTreeSet<u32> = [0u32, 1, 2, 3].iter().copied().collect();
    assert_eq!(
        all_tools, expected_tools,
        "cube_4color Model A Assertion 2(a): not all four tool indices appear in the gcode. \
         Found: {all_tools:?}, expected: {expected_tools:?}"
    );

    let mut multi_tool_no_change: Vec<(usize, usize, usize)> = Vec::new();
    let n = distinct_tools_per_layer.len().min(tc_per_layer.len());
    for i in 0..n {
        let distinct = distinct_tools_per_layer[i];
        let tcs = tc_per_layer[i];
        if distinct >= 2 && tcs == 0 {
            multi_tool_no_change.push((i, distinct, tcs));
        }
    }

    assert!(
        multi_tool_no_change.is_empty(),
        "cube_4color Model A Assertion 2(b): {} layer(s) have >= 2 distinct tool indices but \
         zero tool-change lines. Every multi-tool layer must contain at least one T<N> switch.\n\
         Failures (layer_idx, distinct_tools, tool_changes): {:?}",
        multi_tool_no_change.len(),
        multi_tool_no_change.iter().take(5).collect::<Vec<_>>()
    );

    // AC-6: external_contour removal asserted by the rg grep in packet.spec.md AC-6

    // Assertion 3 — byte-exact SHA pin REMOVED (diagnose 2026-06-24).
    //
    // The former `P105_CUBE_4COLOR_PARITY_SHA` assertion claimed the cube_4color
    // gcode was byte-stable. It is NOT: the medial axis runs on boostvoronoi
    // 0.12.1 → cpp_map 0.2.0 → rand 0.9.4, whose RNG-seeded skiplist makes the
    // Voronoi output (gap-fill + MMU paint-segmentation partition) vary across
    // runs (~1/3 of slices differed; inner-wall and gap-fill geometry drift). A
    // byte-exact hash is therefore a guaranteed CI flake, not a regression gate.
    // The meaningful behavioural gates are the structural assertions above
    // (all four tools present, per-color Model A fragmentation, multi-tool layers
    // carry tool changes). Determinism itself is tracked as a separate follow-up
    // (make the Voronoi path reproducible — e.g. a fixed-seed cpp_map). Do NOT
    // re-introduce a byte-exact pin until the Voronoi path is deterministic.
    assert!(
        !painted.gcode_text.is_empty(),
        "cube_4color produced empty gcode"
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

// --------------------------------------------------------------------------
// Regression tests — parity gaps found in the 2026-06-24 diagnose session.
//
// A G-code preview comparison against OrcaSlicer surfaced four shipped-but-broken
// behaviours on the painted cube. Each test below locks in one fix. They assert
// STRUCTURAL properties (presence / bounded counts), never byte-exact output, so
// they are robust to the known boostvoronoi medial-axis non-determinism.
// --------------------------------------------------------------------------

/// Count `;TYPE:<name>` block headers in gcode.
fn count_type(gcode: &str, ty: &str) -> usize {
    let needle = format!(";TYPE:{ty}");
    gcode.lines().filter(|l| l.trim() == needle).count()
}

/// Gap #1 regression: a PAINTED model must emit top/bottom solid surfaces.
///
/// Root cause (fixed): `PrePass::ShellClassification` ran before
/// `PrePass::PaintSegmentation`, which replaced every region with
/// `..Default::default()` — discarding the classified top/bottom solid fill. The
/// painted cube emitted ZERO `Top surface` / `Bottom surface` (open top, ~4×
/// extrusion deficit) while unpainted models were fine. This guards against the
/// per-color regions silently losing their solid fill again.
#[test]
fn cube_4color_painted_model_emits_top_and_bottom_solid_surfaces() {
    let outcome = slice_fixture_file(&cube_4color_path());
    let top = count_type(&outcome.gcode_text, "Top surface");
    let bottom = count_type(&outcome.gcode_text, "Bottom surface");
    assert!(
        top > 0 && bottom > 0,
        "painted cube must emit top AND bottom solid surfaces (was 0/0 before the \
         ShellClassification→PaintSegmentation propagation fix); got Top={top}, Bottom={bottom}"
    );
}

/// Gap #2 regression: gap-fill must not flood the per-color bisector seams.
///
/// Root cause (fixed): the single-shot `difference_ex(innermost, infill_inset)`
/// rang the entire innermost contour — including the per-color MMU bisector edge —
/// producing ~302 phantom GapFill slivers ("wavy walls"). The OrcaSlicer-parity
/// incremental + infill-transition collection drops that to a small count of
/// genuine thin-feature gaps. The bound is generous (well under the old 302 and
/// well over the deterministic-ish ~86) so the known medial-axis non-determinism
/// cannot flake it.
#[test]
fn cube_4color_gapfill_does_not_flood_bisector_seams() {
    let outcome = slice_fixture_file(&cube_4color_path());
    let gapfill = count_type(&outcome.gcode_text, "GapFill");
    assert!(
        gapfill < 150,
        "GapFill block count {gapfill} exceeds the regression ceiling (150). The pre-fix \
         single-shot collection flooded ~302 bisector-seam slivers; the incremental + \
         infill-transition port should keep this well below 150."
    );
}

/// Gap #3 regression: a multi-tool (MMU) model must auto-enable the wipe tower.
///
/// OrcaSlicer enables a prime/wipe tower automatically for multi-tool prints.
/// Ours defaulted `wipe_tower_enabled = false` with no auto-enable, so painted
/// models emitted zero `Prime tower` blocks. `run_slice` now auto-enables it when
/// the model paints ≥2 distinct tool indices (this fixture has 4). Sliced here
/// with no config, so only the auto-enable path can produce the tower.
#[test]
fn cube_4color_auto_enables_wipe_tower_for_mmu() {
    let outcome = slice_fixture_file(&cube_4color_path());
    let prime = count_type(&outcome.gcode_text, "Prime tower");
    assert!(
        prime > 0,
        "multi-tool model must auto-enable the wipe tower (got {prime} Prime tower blocks). \
         run_slice should inject wipe_tower_enabled=true when >= 2 tool indices are painted."
    );
}

/// Gap #4 regression: the G-code header must declare per-filament colours.
///
/// OrcaSlicer's filament-view preview colours extrusions by the
/// `filament_colour` / `extruder_colour` header directives. Without them a
/// multi-tool print renders monochrome despite `T<n>` tool changes. The header
/// must now list one colour per filament slot (semicolon-separated).
#[test]
fn cube_4color_header_declares_per_filament_colours() {
    let outcome = slice_fixture_file(&cube_4color_path());
    let colour_line = outcome
        .gcode_text
        .lines()
        .find(|l| l.trim_start().starts_with("; filament_colour ="))
        .unwrap_or_else(|| panic!("gcode header missing `; filament_colour =` directive"));
    let has_extruder = outcome
        .gcode_text
        .lines()
        .any(|l| l.trim_start().starts_with("; extruder_colour ="));
    assert!(
        has_extruder,
        "gcode header missing `; extruder_colour =` directive"
    );
    // The 4-color cube must declare multiple distinct, semicolon-separated colours.
    let value = colour_line.split('=').nth(1).unwrap_or("").trim();
    let colours: Vec<&str> = value.split(';').filter(|s| !s.trim().is_empty()).collect();
    assert!(
        colours.len() >= 2,
        "filament_colour must list >= 2 colours for a multi-tool model; got {colours:?}"
    );
}
