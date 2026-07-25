#![cfg(feature = "host-algos")]
#![allow(missing_docs)]

//! DEV-093 reproducer: `medial_axis` must be a pure function of its input.
//!
//! It currently is not. `medial_axis` builds a segment Voronoi diagram through
//! `boostvoronoi 0.12.1`, whose beach line is a `cpp_map::skiplist::SkipList`.
//! `cpp_map 0.2.0` picks skip-list node levels with `rand::prelude::ThreadRng`
//! (`THREAD_RNG` in its `skiplist_impl`), an OS-seeded generator that advances
//! on every insert and exposes no seeding hook. The beach-line comparison
//! predicate is not a strict weak ordering under floating point, so which node
//! the search lands on depends on the random level structure — and near-
//! degenerate inputs resolve differently from one call to the next.
//!
//! The fixture below is a real sub-nozzle-width thin-wall protrusion captured
//! from a 0.2 mm benchy slice (`classic-perimeters`, `Layer::Perimeters`). On
//! this repo's machine it yields **zero** polylines on some calls and **one**
//! on others, within a single process. Downstream that is one `;TYPE:Inner wall`
//! sliver loop appearing or disappearing, which is the whole of DEV-093.
//!
//! `#[ignore]` because the defect is open, not because the assertion is wrong:
//! the test is the acceptance criterion for the fix. Remove the attribute when
//! the RNG is eliminated, and this becomes the standing regression guard.
//!
//! **Must be run in release.** In a debug build every call on this fixture trips
//! `assertion failed: fpv.is_finite()` inside boostvoronoi's `robust_fpt`, which
//! `medial_axis`'s `catch_unwind` converts to an empty result — so debug returns
//! zero polylines *consistently* and the test passes for the wrong reason.
//! Measured: 3/3 release runs fail on call 1 or 2; 1/1 debug run passes.
//!
//! Run it explicitly:
//! `cargo test -p slicer-core --release --features host-algos --test medial_axis_determinism_tdd -- --ignored --nocapture`

use slicer_core::medial_axis::medial_axis;
use slicer_ir::{ExPolygon, Point2, Polygon};

/// Captured DEV-093 fixture, in scaled integer units (1 unit = 100 nm).
/// A thin, near-collinear protrusion — exactly the shape the Voronoi beach-line
/// predicate cannot order robustly.
fn thin_protrusion() -> ExPolygon {
    let pts: &[(i64, i64)] = &[
        (239769, 79714),
        (240934, 80995),
        (242027, 81054),
        (242583, 80561),
        (241910, 81159),
        (241903, 81166),
        (241525, 81505),
        (241462, 81561),
        (241457, 81564),
        (241454, 81565),
        (241447, 81559),
        (240682, 80718),
        (239764, 79709),
    ];
    ExPolygon {
        contour: Polygon {
            points: pts.iter().map(|&(x, y)| Point2 { x, y }).collect(),
        },
        holes: vec![],
    }
}

/// Fingerprint a result so "same output" is a byte-level claim, not a count.
fn fingerprint(polylines: &[slicer_ir::ThickPolyline]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut write = |v: u64| {
        for b in v.to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    };
    for pl in polylines {
        write(pl.points.len() as u64);
        for p in &pl.points {
            write(p.x.to_bits() as u64);
            write(p.y.to_bits() as u64);
            write(p.width.to_bits() as u64);
        }
    }
    h
}

/// The core contract: same input, same output, every call.
///
/// 64 repetitions rather than 2 because the flip is probabilistic — a handful
/// of calls can agree by chance. Measured pre-fix, this fixture produces at
/// least two distinct fingerprints well within 64 calls.
#[test]
#[ignore = "DEV-093 open: cpp_map's skip list uses ThreadRng, so medial_axis is not a pure function"]
fn medial_axis_is_deterministic_across_repeated_calls() {
    let input = thin_protrusion();
    // OrcaSlicer thin-wall band for a 0.4 mm nozzle: min = nozzle/3, max = 2 × line width.
    let (min_width, max_width) = (0.133_333_34_f32, 0.8_f32);

    let first = medial_axis(&input, min_width, max_width).unwrap_or_default();
    let expected = (fingerprint(&first), first.len());

    for call in 1..64 {
        let out = medial_axis(&input, min_width, max_width).unwrap_or_default();
        let actual = (fingerprint(&out), out.len());
        assert_eq!(
            actual, expected,
            "medial_axis returned a different result on call {call} for identical input: \
             first was {:016x} with {} polylines, call {call} was {:016x} with {} polylines. \
             See DEV-093.",
            expected.0, expected.1, actual.0, actual.1
        );
    }
}
