//! Per-WASM-module benchmarks.
//!
//! Iterates every component under `modules/core-modules/*/` and benches:
//! - manifest ingestion (`load_module_from_paths`)
//! - wasm component compilation (`WasmEngine::compile_component`)
//!
//! The full `dispatcher.run_stage(...)` path requires a tier-specific
//! `Blackboard` + `LayerArena` matching each module's `ir_reads`; bench-side
//! IR fixture construction for every stage is left as v3 work â€” the
//! comparable cost is captured implicitly by the `pipeline` bench's
//! end-to-end driver.
//!
//! **Requires** `./modules/core-modules/build-core-modules.sh` to have been
//! run; bench panics loudly with the rebuild hint if a component is missing
//! or matches the documented 8-byte placeholder.

#![allow(missing_docs)]

use std::path::PathBuf;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

use slicer_runtime::manifest::load_module_from_paths;
use slicer_runtime::{export_name_for_stage, WasmEngine};

/// Discovered (module_id, manifest_path, wasm_path) tuple.
struct DiscoveredModule {
    id: String,
    manifest_path: PathBuf,
    wasm_path: PathBuf,
}

/// Walks `modules/core-modules/*/module.toml`, returning one entry per
/// directory that has both a manifest and a non-placeholder wasm artifact.
/// Panics with a rebuild hint if a manifest is found but the corresponding
/// `*.wasm` is missing or matches the 8-byte placeholder.
fn discover_core_modules() -> Vec<DiscoveredModule> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("modules").join("core-modules"))
        .expect("workspace root layout");
    assert!(
        root.is_dir(),
        "expected core-modules directory at {root:?}; \
         run ./modules/core-modules/build-core-modules.sh"
    );

    let mut out = Vec::new();
    for entry in std::fs::read_dir(&root).expect("read core-modules dir") {
        let dir = entry.expect("dir entry").path();
        if !dir.is_dir() {
            continue;
        }
        // Convention: <dir-name>.toml and <dir-name>.wasm sit alongside.
        let leaf = dir
            .file_name()
            .and_then(|s| s.to_str())
            .expect("utf-8 dir name");
        let manifest_path = dir.join(format!("{leaf}.toml"));
        if !manifest_path.is_file() {
            continue;
        }
        let wasm_path = dir.join(format!("{leaf}.wasm"));
        assert!(
            wasm_path.is_file(),
            "missing WASM artifact at {wasm_path:?}; \
             run ./modules/core-modules/build-core-modules.sh"
        );
        let bytes = std::fs::read(&wasm_path).expect("read wasm");
        assert!(
            bytes.len() > 8,
            "{wasm_path:?} looks like the placeholder stub; \
             run ./modules/core-modules/build-core-modules.sh"
        );
        out.push(DiscoveredModule {
            id: leaf.to_string(),
            manifest_path,
            wasm_path,
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

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

fn bench_manifest_load(c: &mut Criterion) {
    let modules = discover_core_modules();
    let mut g = c.benchmark_group("wasm_modules/manifest_load");
    for m in &modules {
        g.bench_with_input(BenchmarkId::from_parameter(&m.id), m, |b, m| {
            b.iter(|| {
                let loaded =
                    load_module_from_paths(black_box(&m.manifest_path), black_box(&m.wasm_path))
                        .expect("manifest should load");
                black_box(loaded);
            })
        });
    }
    g.finish();
}

fn bench_wasm_compile(c: &mut Criterion) {
    let modules = discover_core_modules();
    let engine = WasmEngine::new();
    let preloaded: Vec<(String, Vec<u8>)> = modules
        .iter()
        .map(|m| {
            (
                m.id.clone(),
                std::fs::read(&m.wasm_path).expect("read wasm"),
            )
        })
        .collect();
    let mut g = c.benchmark_group("wasm_modules/compile_component");
    for (id, bytes) in &preloaded {
        g.bench_with_input(BenchmarkId::from_parameter(id), bytes, |b, bytes| {
            b.iter(|| {
                let c = engine
                    .compile_component(black_box(bytes))
                    .expect("component should compile");
                black_box(c);
            })
        });
    }
    g.finish();
}

criterion_group!(
    benches,
    bench_export_name_for_stage,
    bench_manifest_load,
    bench_wasm_compile,
);
criterion_main!(benches);
