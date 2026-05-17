//! Per-WASM-module benchmarks.
//!
//! v1 scope: stub. The benchmark harness for invoking each core module
//! through `WasmRuntimeDispatcher` in isolation requires:
//! 1. `./modules/core-modules/build-core-modules.sh` to have been run
//!    (the wasm32 component artifacts must exist on disk; the host loader
//!    fails loud on the documented 8-byte placeholder).
//! 2. A pre-populated `Blackboard` + `LayerArena` matching each module's
//!    declared `ir_reads` so the dispatch contract is satisfied.
//! 3. A shared `WasmEngine` (wasmtime ties compiled `Component`s to the
//!    engine that produced them — see `main.rs:265` for the production
//!    constraint).
//!
//! Until that fixture layer is extracted into a shared dev helper (see
//! plan B.A6), this file only exercises a sanity-check that the
//! dispatcher's stage→export mapping is fast (it is — a `match`).
//!
//! TODO (v2): mirror `tests/dispatch_tdd.rs` setup for each module in
//! `modules/core-modules/` and bench `dispatcher.run_stage(...)` with
//! `iter_batched` cloning `LayerArena` per iteration.

#![allow(missing_docs)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use slicer_host::dispatch::export_name_for_stage;

fn bench_export_name_for_stage(c: &mut Criterion) {
    let stages: &[&str] = &[
        "PrePass::MeshSegmentation",
        "PrePass::MeshAnalysis",
        "PrePass::LayerPlanning",
        "PrePass::SeamPlanning",
        "Layer::Slice",
        "Layer::SlicePostProcess",
        "Layer::Perimeters",
        "Layer::PerimetersPostProcess",
        "Layer::Infill",
        "Layer::Support",
        "Layer::PathOptimization",
        "PostPass::LayerFinalization",
        "PostPass::GCodeEmit",
        "Unknown::Made::Up",
    ];
    c.bench_function("wasm_modules/export_name_for_stage", |b| {
        b.iter(|| {
            for s in stages {
                let _ = export_name_for_stage(black_box(s));
            }
        })
    });
}

criterion_group!(benches, bench_export_name_for_stage);
criterion_main!(benches);
