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
use slicer_core::beading::factory::{BeadingFactoryParams, BeadingStrategyFactory};
use slicer_core::skeletal_trapezoidation::{
    assign_bead_counts, filter_central, propagate_beadings_downward, propagate_beadings_upward,
    CentralityParams, SkeletalTrapezoidationGraph,
};
use slicer_ir::{ExPolygon, Point2, Polygon, VariableWidthLines};

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
/// x = 10000). Under `factory_params()`/`centrality_params()` below (the
/// identical values used by those two files), the committed baseline
/// `tests/fixtures/arachne/bead_count_tapered_wedge.json` shows six central
/// edges all landing on `bead_count = 5` — a uniform bead *count* with a
/// smoothly-varying `r_avg` along the taper, which is exactly the case this
/// module's doc comment calls out as the cheapest way to observe width
/// variation without needing per-bead radius interpolation.
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
    }
}

/// Same tightened `CentralityParams` as `tests/centrality.rs`'s/
/// `tests/bead_count.rs`'s/`tests/propagation.rs`'s wedge fixture.
fn centrality_params() -> CentralityParams {
    CentralityParams::new(200.0, 50.0)
}

/// Builds a fresh graph for `poly`, runs the full Step 1-3 pipeline
/// (`filter_central` -> `assign_bead_counts` -> `propagate_beadings_upward` ->
/// `propagate_beadings_downward`), then `generate_toolpaths` (Step 4).
fn run_pipeline(poly: &ExPolygon) -> Vec<VariableWidthLines> {
    let mut graph = SkeletalTrapezoidationGraph::from_polygons(std::slice::from_ref(poly))
        .expect("fixture polygon must build a valid SKT graph");

    filter_central(&mut graph, &centrality_params());

    let strategy = BeadingStrategyFactory::create_stack(&factory_params());
    assign_bead_counts(&mut graph, strategy.as_ref())
        .expect("centrality was run, so assign_bead_counts must succeed");

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

        // --- (b) Every ExtrusionLine's inset_idx matches its bucket;
        // is_odd == (inset_idx % 2 == 1).
        for (line_idx, line) in bucket.iter().enumerate() {
            assert_eq!(
                line.inset_idx, bucket_inset,
                "bucket at outer-Vec position {bucket_pos} (inset {bucket_inset}): line \
                 {line_idx} has inset_idx {} which does not match its bucket",
                line.inset_idx
            );
            assert_eq!(
                line.is_odd,
                bucket_inset % 2 == 1,
                "bucket at outer-Vec position {bucket_pos} (inset {bucket_inset}): line \
                 {line_idx} has is_odd={} but inset_idx {bucket_inset} implies is_odd={}",
                line.is_odd,
                bucket_inset % 2 == 1
            );
        }
    }

    // --- (c) Variable widths observable: not every junction width across
    // the whole output is identical. Per generate_toolpaths.rs's module doc
    // comment, an earlier per-edge-uniform r_avg width formula collapsed to
    // a constant across every central edge on *this exact* wedge fixture
    // (its three surviving central spokes all share nearly identical
    // r_avg), so width is instead derived per-junction from
    // `strategy.compute()` called on each endpoint's own local
    // distance_to_boundary (r_start != r_end along the same tapering spoke
    // edge, e.g. the apex spoke's r=0 at the tip vs r~99 at the incenter) —
    // genuine, non-fabricated variation within a single line, not merely
    // line-to-line.
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
    let first_width = all_widths[0];
    assert!(
        all_widths.iter().any(|&w| (w - first_width).abs() > 1e-6),
        "tapered wedge: expected observable width variation across junctions, but every \
         junction width equals {first_width}mm"
    );

    write_or_compare_baseline(&build_fixture(&output_a));
}
