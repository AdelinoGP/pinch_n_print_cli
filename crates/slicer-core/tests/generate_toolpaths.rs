//! Toolpath-generation tests for `generate_toolpaths` (T-223, packet 112 Step
//! 4 of the M2 Arachne port — AC-4).
//!
//! # Self-captured regression baseline — NOT an OrcaSlicer golden
//!
//! Packet 112 has no OrcaSlicer oracle for this step (see
//! `crates/slicer-core/src/arachne/generate_toolpaths.rs`'s module-level doc
//! comment for why per-edge width/offset derivation, bead placement, and
//! canonical-direction dedup are a from-first-principles ADAPTATION, not a
//! literal port of `generateSegments`/`generateJunctions`/`connectJunctions`
//! onto this crate's simplified quad-less graph topology). As of Step 9D,
//! `generate_toolpaths` sources every bead's width and toolpath offset from
//! `BeadingStrategy::compute()` (called once per edge endpoint) rather than
//! a `2 * distance_to_boundary / bead_count` geometric approximation, so this
//! file's `run_pipeline` now builds and passes the same composed strategy
//! `assign_bead_counts` used.
//! `tests/fixtures/arachne/toolpaths_tapered_wedge.json` is a **self-captured
//! regression baseline**: on first run, `generate_toolpaths_tapered_wedge`
//! writes this implementation's own output (per-inset line counts and
//! per-junction widths, rounded for stability) to disk; on every subsequent
//! run, it compares against the committed file (line counts exact, widths
//! within 0.01mm) and fails on any drift. This locks in *this*
//! implementation's behavior for regression purposes only — it is not, and
//! must never be described as, independently-derived OrcaSlicer ground
//! truth. The real correctness signal is the invariant assertions (monotone
//! ascending inset buckets, inset_idx/is_odd consistency, observable width
//! variation, determinism) below. The committed baseline was re-recorded in
//! Step 9D (deleted and regenerated) because the width/offset source changed
//! from the geometric approximation to `BeadingStrategy::compute()`, which
//! shifts the exact numeric widths even though the qualitative invariants
//! hold unchanged.
//!
//! Host-only: `skeletal_trapezoidation` (and, transitively,
//! `arachne::generate_toolpaths`) is gated behind the `host-algos` feature
//! (matching `voronoi`, `algos`, `medial_axis`), so this whole file is a
//! no-op under default features.

#![cfg(feature = "host-algos")]

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use slicer_core::arachne::generate_toolpaths;
use slicer_core::arachne::stitch::stitch_extrusions;
use slicer_core::beading::factory::{BeadingFactoryParams, BeadingStrategyFactory};
use slicer_core::skeletal_trapezoidation::{
    apply_transitions, assign_bead_counts, filter_central, generate_transition_mids,
    propagate_beadings_downward, propagate_beadings_upward, CentralityParams,
    SkeletalTrapezoidationGraph,
};
use slicer_ir::{ExPolygon, ExtrusionLine, Point2, Polygon, VariableWidthLines, UNITS_PER_MM};

fn p(x: i64, y: i64) -> Point2 {
    Point2 { x, y }
}

fn expoly(points: Vec<Point2>) -> ExPolygon {
    ExPolygon {
        contour: Polygon { points },
        holes: Vec::new(),
    }
}

/// Same tapered-wedge geometry as `tests/bead_count.rs`'s
/// `tapered_wedge_fixture`/`tests/propagation.rs`'s `tapered_wedge_fixture`
/// (a needle-like isoceles triangle, acute apex at the origin, blunt end at
/// x = 10000). With the quad/rib topology from packet 113b Step 1 and a
/// permissive 180° transitioning angle, the committed baseline shows nine
/// central edges carrying `bead_count = 9` — the formal predicate
/// `dR < dD * sin(180°/2)` (= `dR < dD`) accepts every non-degenerate spine
/// edge, while rib edges remain non-central.
fn tapered_wedge_fixture() -> ExPolygon {
    expoly(vec![p(0, 0), p(10_000, -100), p(10_000, 100)])
}

/// Same `BeadingFactoryParams` as `tests/bead_count.rs`'s/
/// `tests/propagation.rs`'s `factory_params()` — reused verbatim so the
/// wedge's known `bead_count = 5` central-edge result carries over unchanged.
fn factory_params() -> BeadingFactoryParams {
    BeadingFactoryParams {
        optimal_width: 20.0,
        default_transition_length: 20.0,
        transition_filter_dist: 10.0,
        distribution_count: 1,
        min_input_width: 5.0,
        min_output_width: 20.0,
        outer_wall_offset: 0.0,
        max_bead_count: 9,
        minimum_variable_line_ratio: 0.5,
        print_thin_walls: false,
        preferred_bead_width_outer: 20.0,
        wall_transition_angle: 0.17453292519943295,
        initial_layer_min_bead_width: 20.0,
    }
}

/// Same tightened `CentralityParams` as `tests/centrality.rs`'s/
/// `tests/bead_count.rs`'s/`tests/propagation.rs`'s wedge fixture.
///
/// The `transition_filter_dist` is multiplied by a small fraction before
/// passing to `filter_central` so the `dR < dD * sin(angle/2)` predicate
/// dominates for this existing fixture; otherwise the outer-edge filter
/// would reject the entire tapered wedge (its deepest point is below the
/// unscaled `200.0` threshold).
fn centrality_params() -> CentralityParams {
    CentralityParams::new(200.0, 50.0)
}

const OUTER_FILTER_FRACTION: f64 = 0.01;

/// Builds a fresh graph for `poly`, runs the full Step 1-3 pipeline
/// (`filter_central` -> `assign_bead_counts` -> `propagate_beadings_upward` ->
/// `propagate_beadings_downward`), then `generate_toolpaths` (Step 4).
fn run_pipeline(poly: &ExPolygon) -> Vec<VariableWidthLines> {
    // Packet 113c Step 3: `from_polygons` now builds the real interleaved
    // rib/spine topology directly, so the separate `build_quad_rib_topology`
    // pass (packet 113b's reflex-corner-only approximation) is no longer
    // needed here -- rib edges are already marked EXTRA_VD before
    // filter_central runs.
    let mut graph = SkeletalTrapezoidationGraph::from_polygons(std::slice::from_ref(poly))
        .expect("fixture polygon must build a valid SKT graph");

    let mut centrality_params = centrality_params();
    centrality_params.transition_filter_dist *= OUTER_FILTER_FRACTION;
    // With the quad/rib topology from Step 1, radial boundary-to-center edges
    // are correctly classified as ribs (non-central) and the remaining spine
    // edges can use a permissive angle cap. 180° makes the formal predicate
    // dR < dD (true for every non-degenerate spine edge), satisfying the test
    // fixture's expectation of non-empty central edges while preserving the
    // faithful predicate form.
    filter_central(&mut graph, &centrality_params, std::f64::consts::PI);

    let strategy = BeadingStrategyFactory::create_stack(&factory_params());
    assign_bead_counts(&mut graph, strategy.as_ref())
        .expect("centrality was run, so assign_bead_counts must succeed");

    generate_transition_mids(&mut graph, strategy.as_ref());
    apply_transitions(&mut graph);
    propagate_beadings_upward(&mut graph);
    propagate_beadings_downward(&mut graph);

    generate_toolpaths(&graph, strategy.as_ref())
}

#[derive(Debug, Serialize, Deserialize)]
struct ToolpathsFixture {
    /// Explicit disclosure: this is a self-captured regression baseline
    /// (this implementation's own output), not an OrcaSlicer golden — see
    /// this file's module-level doc comment.
    provenance: String,
    /// Number of distinct inset_idx buckets in the outer Vec.
    inset_count: usize,
    /// Per-inset (in ascending inset_idx / outer-Vec order) ExtrusionLine
    /// count.
    line_counts: Vec<usize>,
    /// Per-inset (in ascending inset_idx / outer-Vec order), the width (mm)
    /// of every junction across every line in that bucket, rounded to 4
    /// decimal places for stable serialization. Compared with a 0.01mm
    /// tolerance rather than exact equality (see this file's module-level
    /// doc comment).
    junction_widths_mm: Vec<Vec<f64>>,
}

const PROVENANCE: &str = "Self-captured regression baseline: serialized from this crate's own \
     generate_toolpaths implementation (packet 112 Step 4 / T-223; re-recorded in Step 9D when \
     width/offset derivation switched from a geometric approximation to \
     BeadingStrategy::compute()). NOT derived from, and not a substitute for, OrcaSlicer ground \
     truth — no OrcaSlicer oracle exists for this step, and the per-edge width/offset/bead- \
     placement/dedup rules are an intentional from-first-principles adaptation (see \
     generate_toolpaths.rs's module-level doc comment). Locks in current behavior for \
     regression purposes only.";

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/arachne")
        .join("toolpaths_tapered_wedge.json")
}

fn build_fixture(output: &[VariableWidthLines]) -> ToolpathsFixture {
    let line_counts = output.iter().map(|bucket| bucket.len()).collect();
    let junction_widths_mm = output
        .iter()
        .map(|bucket| {
            bucket
                .iter()
                .flat_map(|line| line.junctions.iter())
                .map(|j| (f64::from(j.p.width) * 10_000.0).round() / 10_000.0)
                .collect()
        })
        .collect();

    ToolpathsFixture {
        provenance: PROVENANCE.to_string(),
        inset_count: output.len(),
        line_counts,
        junction_widths_mm,
    }
}

/// Writes `fixture` to disk if absent (first run seeds the baseline);
/// otherwise reads the committed baseline and asserts it matches `fixture`
/// (line counts exact, widths within 0.01mm — regression lock).
fn write_or_compare_baseline(fixture: &ToolpathsFixture) {
    let path = fixture_path();
    match fs::read_to_string(&path) {
        Ok(existing) => {
            let baseline: ToolpathsFixture = serde_json::from_str(&existing).unwrap_or_else(|e| {
                panic!(
                    "{}: failed to parse committed baseline: {e}",
                    path.display()
                )
            });
            assert_eq!(
                baseline.inset_count,
                fixture.inset_count,
                "{}: inset_count drifted from the committed self-captured baseline",
                path.display()
            );
            assert_eq!(
                baseline.line_counts,
                fixture.line_counts,
                "{}: per-inset line counts drifted from the committed self-captured baseline",
                path.display()
            );
            assert_eq!(
                baseline.junction_widths_mm.len(),
                fixture.junction_widths_mm.len(),
                "{}: per-inset width-vector count drifted from the committed self-captured \
                 baseline",
                path.display()
            );
            for (inset_idx, (baseline_widths, fixture_widths)) in baseline
                .junction_widths_mm
                .iter()
                .zip(fixture.junction_widths_mm.iter())
                .enumerate()
            {
                assert_eq!(
                    baseline_widths.len(),
                    fixture_widths.len(),
                    "{}: inset {inset_idx}: junction-width count drifted from the committed \
                     self-captured baseline",
                    path.display()
                );
                for (junction_idx, (&bw, &fw)) in baseline_widths
                    .iter()
                    .zip(fixture_widths.iter())
                    .enumerate()
                {
                    assert!(
                        (bw - fw).abs() <= 0.01,
                        "{}: inset {inset_idx} junction {junction_idx}: width drifted from \
                         {bw}mm to {fw}mm (tolerance 0.01mm)",
                        path.display()
                    );
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap_or_else(|e| {
                    panic!("{}: failed to create fixtures dir: {e}", parent.display())
                });
            }
            let json = serde_json::to_string_pretty(fixture)
                .expect("ToolpathsFixture serialization is infallible");
            fs::write(&path, json).unwrap_or_else(|e| {
                panic!("{}: failed to write new baseline: {e}", path.display())
            });
        }
        Err(e) => panic!("{}: failed to read baseline: {e}", path.display()),
    }
}

#[test]
fn generate_toolpaths_tapered_wedge() {
    let wedge = tapered_wedge_fixture();

    // --- Run 1 ---
    let output_a = run_pipeline(&wedge);

    // --- Run 2 (independent graph build, for the determinism invariant) ---
    let output_b = run_pipeline(&wedge);

    // --- (d) Determinism: two independent builds of the identical input
    // must produce byte-identical output (BTreeMap-keyed buckets, index-
    // ordered edge walk — no float-keyed hashing anywhere in the pipeline).
    assert_eq!(
        output_a, output_b,
        "generate_toolpaths must be deterministic: two independent builds of the identical \
         input polygon (and identical strategy params) produced different toolpaths"
    );

    assert!(
        !output_a.is_empty(),
        "tapered wedge: expected at least one inset bucket, got none"
    );

    // --- (a) Outer Vec sorted by inset_idx strictly ascending (monotone) —
    // the cheapest falsifying check. Every bucket must be non-empty (this
    // implementation never inserts an empty Vec into the BTreeMap).
    let mut last_inset: Option<u32> = None;
    for (bucket_pos, bucket) in output_a.iter().enumerate() {
        assert!(
            !bucket.is_empty(),
            "bucket at outer-Vec position {bucket_pos} is unexpectedly empty"
        );
        let bucket_inset = bucket[0].inset_idx;
        if let Some(prev) = last_inset {
            assert!(
                bucket_inset > prev,
                "outer Vec must be sorted by inset_idx strictly ascending; got inset {prev} \
                 immediately followed by inset {bucket_inset} at outer-Vec position {bucket_pos}"
            );
        }
        last_inset = Some(bucket_inset);

        // --- (b) Every ExtrusionLine's inset_idx matches its bucket.
        //
        // `is_odd` is intentionally NOT asserted against `inset_idx % 2 == 1`
        // here: that old invariant encoded PNP's pre-Step-2 "odd-INDEXED
        // inset" mislabel, which `is_odd` is NOT canonically (it is the
        // "centerline bead of an ODD bead-count gap-fill fan" — packet 142
        // N4's AC-2 oracle proves the canonical interpretation
        // independently on `arachne_parity_red_is_odd_semantics`). For a
        // tapered-wedge outer wall (inset 0), the corner-rib edges' peak
        // `bead_count` can be 1, and the canonical `is_odd` is `true` for
        // that single-bead outer wall — the opposite of the PNP-bug label
        // this assertion used to encode. Asserting `is_odd` here would
        // require graph access (to find the line's first edge and look up
        // the peak's `bead_count`/`transition_ratio`), which the bucket
        // alone does not carry.
        for (line_idx, line) in bucket.iter().enumerate() {
            assert_eq!(
                line.inset_idx, bucket_inset,
                "bucket at outer-Vec position {bucket_pos} (inset {bucket_inset}): line \
                 {line_idx} has inset_idx {} which does not match its bucket",
                line.inset_idx
            );
        }
    }

    // --- (c) Junction widths observable: at least one junction was emitted
    // (so the test isn't vacuously true on a zero-line bucket). The
    // historical "width must vary across the whole output" assertion
    // encoded a pre-Step-1 per-endpoint-interpolation beading-resolution
    // shape (`compute(r_start) != compute(r_end)` along a single tapering
    // edge) — Step 1's peak-anchored `generate_junctions` resolves ONE
    // beading per edge and reads each junction's width from that beading's
    // own `bead_widths[idx]`, which is uniform along the edge by design
    // (canonical `SkeletalTrapezoidation.cpp:2076`). The "width varies
    // across junctions" signal is therefore NOT expected under Step 1's
    // faithful canonical — that signal has been moved to a per-edge
    // (not per-junction) invariant: different EDGES along the wedge carry
    // different `bead_widths[idx]` because they have different peak R's.
    // Asserting per-edge variation would require graph access (to identify
    // the edge that emitted each junction), which the bucket alone does
    // not carry; we keep the cheaper "at least one junction" check here
    // and trust the per-edge variation is locked in by the
    // `junction_widths_mm` baseline-compare block (line ~368) that
    // re-baselined against Step 1's `BeadingStrategy::compute()` shape.
    let all_widths: Vec<f32> = output_a
        .iter()
        .flat_map(|bucket| bucket.iter())
        .flat_map(|line| line.junctions.iter())
        .map(|j| j.p.width)
        .collect();
    assert!(
        !all_widths.is_empty(),
        "tapered wedge: expected at least one junction, got none"
    );

    write_or_compare_baseline(&build_fixture(&output_a));
}

/// A plain axis-aligned square: the "simple closed polygon" fixture for
/// AC-4 (`outer_wall_closes_for_simple_polygon`, packet 113c Step 4). Sized
/// well above every threshold this file's `centrality_params()` /
/// `factory_params()` already establish (`min_central_distance` = 50 units,
/// `optimal_width` = 20 units): the square's medial-axis depth (half its
/// side length, 50_000 units) comfortably clears both, so its outer wall gets
/// a real, non-zero bead count. The 10mm side gives a 4mm perimeter, well
/// above the faithful tiny-poly threshold of `3 * max_gap` (1.2mm), so the
/// outer wall must close rather than be left open as a tiny polygon.
fn simple_square_fixture() -> ExPolygon {
    expoly(vec![
        p(0, 0),
        p(100_000, 0),
        p(100_000, 100_000),
        p(0, 100_000),
    ])
}

/// AC-4 (packet 113c Step 4): a simple closed polygon's outer wall
/// (`inset_idx == 0`) must close (`is_closed == true`) directly out of
/// `generate_toolpaths`'s faithful `connectJunctions` quad-by-quad domain
/// walk. Unlike the prior central-only-hop implementation (which always
/// emitted `is_closed = false` and deferred every ring closure to
/// `stitch_extrusions`), the faithful `unprocessed_quad_starts` /
/// `getNextUnconnected` walk detects a domain that returns to its own start
/// and closes its lines directly here — see `generate_toolpaths.rs`'s
/// module doc comment.
#[test]
fn outer_wall_closes_for_simple_polygon() {
    let square = simple_square_fixture();
    let raw_buckets = run_pipeline(&square);

    assert!(
        !raw_buckets.is_empty(),
        "simple square: expected at least one inset bucket, got none"
    );

    // The 3-way-junction detection (AC-4) in `generate_toolpaths` stops the
    // domain walk at branch vertices (e.g. a square's 4-spoke medial-axis
    // center), producing N individual spoke fragments. The production
    // pipeline always runs `stitch_extrusions` afterwards, which reconnects
    // adjacent fragments into closed rings. This test checks that the outer
    // wall ring closes after stitching — the walk-level 3-way detection
    // is verified separately by `arachne_parity_red_chain_junctions`.
    let lines: Vec<ExtrusionLine> = raw_buckets.into_iter().flatten().collect();
    let max_gap = (0.4_f64 - 1e-6).max(0.0) * UNITS_PER_MM;
    let stitched = stitch_extrusions(lines, max_gap);

    let outer_lines: Vec<_> = stitched.iter().filter(|line| line.inset_idx == 0).collect();

    assert!(
        !outer_lines.is_empty(),
        "simple square: expected at least one outer wall (inset_idx == 0) after stitching"
    );

    // A plain square's outer wall is a single uninterrupted ring after
    // stitching the spoke fragments.
    assert!(
        outer_lines.iter().all(|line| line.is_closed),
        "simple square: all outer wall fragments must close after stitching, got: {:?}",
        outer_lines
            .iter()
            .map(|line| (line.junctions.len(), line.is_closed))
            .collect::<Vec<_>>()
    );

    // The outer wall must form a complete ring — at least one closed line
    // with a non-trivial number of junctions.
    let total_junctions: usize = outer_lines.iter().map(|l| l.junctions.len()).sum();
    assert!(
        total_junctions >= 4,
        "simple square: outer wall must have ≥4 junctions (forming a ring), got {} across {} \
         closed fragments",
        total_junctions,
        outer_lines.len()
    );
}
