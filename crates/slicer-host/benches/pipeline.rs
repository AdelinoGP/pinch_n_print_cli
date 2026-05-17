//! End-to-end pipeline benchmarks.
//!
//! v1 scope: measures the instrumentation overhead path (Noop vs Collector)
//! by driving the `PipelineInstrumentation` trait directly with a synthetic
//! bracket sequence — no real WASM, no real mesh. This isolates the
//! orchestration cost of the report stack from any pipeline noise.
//!
//! TODO (v2): plumb in a real `run_pipeline` against `resources/benchy.stl`
//! and `modules/core-modules/` (requires `./modules/core-modules/build-core-modules.sh`
//! to have run). The setup mirrors `crates/slicer-host/tests/benchy_end_to_end_tdd.rs`
//! — extract its `noop_runners()`-equivalent into a shared dev module and
//! reuse here. See plan B.A4.

#![allow(missing_docs)]

use std::sync::Arc;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

use slicer_host::instrumentation::{NoopInstrumentation, Phase, PipelineInstrumentation};
use slicer_host::report::Collector;

fn drive_brackets<I: PipelineInstrumentation>(inst: &I, n_layers: u32, n_modules: u32) {
    inst.on_phase_start(Phase::PrePass);
    inst.on_phase_end(Phase::PrePass);
    inst.on_phase_start(Phase::PerLayer);
    for layer in 0..n_layers {
        inst.on_layer_start(layer, 0.2 + layer as f32 * 0.2);
        let stage = "Layer::Perimeters".to_string();
        inst.on_stage_start(&stage, Some(layer));
        for m in 0..n_modules {
            let mid = format!("mod-{m}");
            inst.on_module_start(&stage, Some(layer), &mid);
            inst.on_module_end(&stage, Some(layer), &mid, 0, 0);
        }
        inst.on_stage_end(&stage, Some(layer));
        inst.on_layer_end(layer);
    }
    inst.on_phase_end(Phase::PerLayer);
    inst.on_phase_start(Phase::PostPass);
    inst.on_phase_end(Phase::PostPass);
}

fn bench_noop_overhead(c: &mut Criterion) {
    let mut g = c.benchmark_group("pipeline/instrumentation_noop");
    for layers in &[10u32, 100, 1000] {
        let inst = NoopInstrumentation;
        g.bench_with_input(BenchmarkId::from_parameter(layers), layers, |b, &n| {
            b.iter(|| drive_brackets(black_box(&inst), n, 3))
        });
    }
    g.finish();
}

fn bench_collector_overhead(c: &mut Criterion) {
    let mut g = c.benchmark_group("pipeline/instrumentation_collector");
    for layers in &[10u32, 100, 1000] {
        g.bench_with_input(BenchmarkId::from_parameter(layers), layers, |b, &n| {
            b.iter(|| {
                let coll = Arc::new(Collector::new("bench.stl"));
                drive_brackets(coll.as_ref(), n, 3);
                let _ = black_box(coll.finalize());
            })
        });
    }
    g.finish();
}

fn bench_allocator_disabled(c: &mut Criterion) {
    // Sanity: ensure accounting is OFF for this bench (it's the default,
    // but be defensive — earlier benches in the same process could leave
    // it on).
    slicer_host::report::allocator::disable();

    let mut g = c.benchmark_group("pipeline/allocator_fast_path");
    g.bench_function("vec_push_1k", |b| {
        b.iter(|| {
            let mut v: Vec<u64> = Vec::with_capacity(0);
            for i in 0..1024u64 {
                v.push(black_box(i));
            }
            black_box(v);
        })
    });
    g.bench_function("string_alloc_short", |b| {
        b.iter(|| {
            let s = format!("scope-{}", black_box(42u32));
            black_box(s);
        })
    });
    g.finish();
}

criterion_group!(
    benches,
    bench_noop_overhead,
    bench_collector_overhead,
    bench_allocator_disabled
);
criterion_main!(benches);
