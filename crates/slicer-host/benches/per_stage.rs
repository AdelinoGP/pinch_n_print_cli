//! Per-stage benchmarks.
//!
//! v1 scope: measures `compute_serial_edges_for_stage` and
//! `compute_serial_edges_from_compiled` — the pure-Rust helpers consulted
//! at plan freeze for the serial-edge explainer. These are called once per
//! stage on plan startup; understanding their cost relative to stage count
//! / module count helps size the upper bound for module-heavy pipelines.
//!
//! TODO (v2): bench each prepass / per-layer / postpass stage in isolation
//! by snapshotting the `Blackboard` after PrePass completes and driving
//! the stage executors directly with `LayerExecutionPlan` filtered to one
//! `STAGE_ORDER` entry. See plan B.A5.

#![allow(missing_docs)]

use std::path::PathBuf;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

use slicer_host::instrumentation::compute_serial_edges_for_stage;
use slicer_host::manifest::{ConfigSchema, LoadedModule};
use slicer_ir::SemVer;

fn loaded_module(id: &str, ir_reads: &[&str], ir_writes: &[&str]) -> LoadedModule {
    LoadedModule {
        id: id.to_string(),
        version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        stage: "Layer::Perimeters".to_string(),
        wit_world: "slicer:world-layer@1.0.0".to_string(),
        ir_reads: ir_reads.iter().map(|s| s.to_string()).collect(),
        ir_writes: ir_writes.iter().map(|s| s.to_string()).collect(),
        claims: Vec::new(),
        requires_claims: Vec::new(),
        incompatible_with: Vec::new(),
        requires_modules: Vec::new(),
        min_host_version: SemVer {
            major: 0,
            minor: 1,
            patch: 0,
        },
        min_ir_schema: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        max_ir_schema: SemVer {
            major: 2,
            minor: 0,
            patch: 0,
        },
        config_schema: ConfigSchema::default(),
        overridable_per_region: Vec::new(),
        overridable_per_layer: Vec::new(),
        layer_parallel_safe: true,
        wasm_path: PathBuf::from(format!("fixtures/{id}.wasm")),
        placeholder_wasm: false,
    }
}

/// Build N modules where module i writes path Pi and module i+1 reads Pi,
/// producing an N-1 length chain of IrWriteRead edges.
fn chain_modules(n: usize) -> Vec<LoadedModule> {
    (0..n)
        .map(|i| {
            let writes_path: String = format!("PerimeterIR.region.{i}");
            let reads_path: String = if i == 0 {
                String::new()
            } else {
                format!("PerimeterIR.region.{}", i - 1)
            };
            let mut m = loaded_module(
                &format!("m{i}"),
                &[reads_path.as_str()],
                &[writes_path.as_str()],
            );
            if i == 0 {
                m.ir_reads.clear();
            }
            m
        })
        .collect()
}

fn bench_compute_edges(c: &mut Criterion) {
    let mut g = c.benchmark_group("per_stage/compute_serial_edges");
    for n in &[8usize, 32, 128] {
        let modules = chain_modules(*n);
        g.bench_with_input(BenchmarkId::from_parameter(n), n, |b, _| {
            b.iter(|| {
                let edges = compute_serial_edges_for_stage(black_box(&modules));
                black_box(edges);
            })
        });
    }
    g.finish();
}

criterion_group!(benches, bench_compute_edges);
criterion_main!(benches);
