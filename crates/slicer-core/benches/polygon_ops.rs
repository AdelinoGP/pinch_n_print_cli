//! Microbenchmarks for slicer-core 2D polygon ops.
//!
//! Each group constructs fixtures outside `b.iter` so the iteration body
//! measures only the operation. Polygons are synthetic squares laid out
//! on a grid; sizes parameterized by `bench_with_input`.

#![allow(missing_docs)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

use slicer_core::polygon_ops::{difference, intersection, offset, union, OffsetJoinType};
use slicer_ir::{ExPolygon, Point2, Polygon};

/// Build a CCW square ExPolygon centered at `(cx, cy)` with side length
/// `side_units` (in scaled IR units: 1 unit = 100 nm, so 10000 = 1 mm).
fn square(cx: i64, cy: i64, side_units: i64) -> ExPolygon {
    let half = side_units / 2;
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 {
                    x: cx - half,
                    y: cy - half,
                },
                Point2 {
                    x: cx + half,
                    y: cy - half,
                },
                Point2 {
                    x: cx + half,
                    y: cy + half,
                },
                Point2 {
                    x: cx - half,
                    y: cy + half,
                },
            ],
        },
        holes: Vec::new(),
    }
}

/// Build N squares on an MxM grid sized to overlap their neighbours.
fn square_grid(n_per_side: i64) -> Vec<ExPolygon> {
    // Side = 12 mm (120_000 units), pitch = 10 mm (100_000 units) — squares
    // overlap their neighbours by 2 mm so set operations have real work.
    let side = 120_000;
    let pitch = 100_000;
    let mut out = Vec::with_capacity((n_per_side * n_per_side) as usize);
    for ix in 0..n_per_side {
        for iy in 0..n_per_side {
            out.push(square(ix * pitch, iy * pitch, side));
        }
    }
    out
}

fn bench_union(c: &mut Criterion) {
    let mut g = c.benchmark_group("polygon_ops/union");
    for n in &[4i64, 8, 16] {
        let subject = square_grid(*n);
        let clip: Vec<ExPolygon> = vec![square(50_000, 50_000, 1_000_000)]; // one big square
        g.bench_with_input(BenchmarkId::from_parameter(n * n), n, |b, _| {
            b.iter(|| {
                let r = union(black_box(&subject), black_box(&clip));
                black_box(r);
            })
        });
    }
    g.finish();
}

fn bench_intersection(c: &mut Criterion) {
    let mut g = c.benchmark_group("polygon_ops/intersection");
    for n in &[4i64, 8, 16] {
        let subject = square_grid(*n);
        let clip: Vec<ExPolygon> = vec![square(50_000, 50_000, 1_500_000)];
        g.bench_with_input(BenchmarkId::from_parameter(n * n), n, |b, _| {
            b.iter(|| {
                let r = intersection(black_box(&subject), black_box(&clip));
                black_box(r);
            })
        });
    }
    g.finish();
}

fn bench_difference(c: &mut Criterion) {
    let mut g = c.benchmark_group("polygon_ops/difference");
    for n in &[4i64, 8, 16] {
        let subject = square_grid(*n);
        let clip: Vec<ExPolygon> = vec![square(50_000, 50_000, 600_000)];
        g.bench_with_input(BenchmarkId::from_parameter(n * n), n, |b, _| {
            b.iter(|| {
                let r = difference(black_box(&subject), black_box(&clip));
                black_box(r);
            })
        });
    }
    g.finish();
}

fn bench_offset(c: &mut Criterion) {
    let mut g = c.benchmark_group("polygon_ops/offset");
    for n in &[4i64, 8, 16] {
        let subject = square_grid(*n);
        g.bench_with_input(BenchmarkId::from_parameter(n * n), n, |b, _| {
            b.iter(|| {
                let r = offset(black_box(&subject), 0.4, OffsetJoinType::Miter);
                black_box(r);
            })
        });
    }
    g.finish();
}

criterion_group!(
    benches,
    bench_union,
    bench_intersection,
    bench_difference,
    bench_offset
);
criterion_main!(benches);
