//! Gate-evidence benchmark for the DEV-026 gap (3) full-slice time budget
//! (`docs/12_architecture_gate_metrics.md`: "Time budget: full slice <= 10
//! seconds on 50-layer benchy reference fixture").
//!
//! Downgraded per 2026-07-03 maintainer decision: measures wall-clock time
//! only (no peak-RSS — `AccountingAllocator` cannot see WASM linear memory,
//! see `docs/DEVIATION_LOG.md` DEV-026), against the real WASM pipeline via a
//! real `pnp_cli slice` subprocess against `resources/regression_wedge.stl`
//! together with `resources/test_config/gate_evidence_50l.json` (a
//! `layer_height` / `first_layer_height` override that coerces the
//! 40mm-tall fixture to exactly 50 layers, verified empirically this
//! session — `;LAYER_CHANGE` count in the emitted gcode — rather than
//! building a new purpose-made fixture).
//!
//! Measure-and-report, not hard-fail: like `pipeline`/`per_stage`/
//! `wasm_modules`, this is explicitly "slow; not in CI" per `CLAUDE.md`.
//! Compare the reported time against the docs/12 10-second bound manually.
//!
//! **Requires** `cargo xtask build-guests` to have been run (same
//! precondition as `wasm_modules.rs`) and `cargo build --workspace` (or
//! `--release`) so `pnp_cli` exists in the matching profile directory.

#![allow(missing_docs)]

use std::path::{Path, PathBuf};
use std::process::Command;

use criterion::{criterion_group, criterion_main, Criterion};
use pnp_cli_locator::pnp_cli_bin;

/// Workspace root, uncanonicalized (unlike
/// `pnp_cli_locator::workspace_root`), because the paths derived from it
/// are passed straight to the `pnp_cli` subprocess as arguments.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root layout")
        .to_path_buf()
}

/// Precondition guard mirroring `wasm_modules.rs::discover_core_modules`:
/// panics with a rebuild hint if `modules/core-modules` has no real
/// (non-placeholder) wasm artifact.
fn assert_core_modules_built(root: &Path) {
    let core_modules = root.join("modules").join("core-modules");
    assert!(
        core_modules.is_dir(),
        "expected core-modules directory at {core_modules:?}"
    );
    let mut found_any = false;
    for entry in std::fs::read_dir(&core_modules).expect("read core-modules dir") {
        let dir = entry.expect("dir entry").path();
        if !dir.is_dir() {
            continue;
        }
        let leaf = dir
            .file_name()
            .and_then(|s| s.to_str())
            .expect("utf-8 dir name");
        let wasm_path = dir.join(format!("{leaf}.wasm"));
        if wasm_path.is_file() {
            let bytes = std::fs::read(&wasm_path).expect("read wasm");
            assert!(
                bytes.len() > 8,
                "{wasm_path:?} looks like the placeholder stub; run `cargo xtask build-guests`"
            );
            found_any = true;
        }
    }
    assert!(
        found_any,
        "no real wasm artifacts found under {core_modules:?}; run `cargo xtask build-guests`"
    );
}

fn bench_full_slice_50_layers(c: &mut Criterion) {
    let root = repo_root();
    assert_core_modules_built(&root);

    let bin = pnp_cli_bin();
    let model = root.join("resources").join("regression_wedge.stl");
    let module_dir = root.join("modules").join("core-modules");
    let config = root
        .join("resources")
        .join("test_config")
        .join("gate_evidence_50l.json");
    let out_dir = root.join("target").join("gate-evidence-bench");
    std::fs::create_dir_all(&out_dir).expect("create bench output dir");
    let output = out_dir.join("gate_evidence_50l.gcode");

    let mut g = c.benchmark_group("gate_evidence");
    // Criterion's minimum: each sample is a real subprocess slice, so the
    // default 100 samples would make this bench take an unreasonably long
    // time for something explicitly "slow; not in CI" already.
    g.sample_size(10);
    g.bench_function("full_slice_50_layers", |b| {
        b.iter(|| {
            let status = Command::new(&bin)
                .args(["slice", "--model"])
                .arg(&model)
                .arg("--module-dir")
                .arg(&module_dir)
                .arg("--output")
                .arg(&output)
                .arg("--config")
                .arg(&config)
                .status()
                .expect("pnp_cli should execute");
            assert!(
                status.success(),
                "pnp_cli slice must succeed for gate evidence to be meaningful"
            );
        })
    });
    g.finish();
}

criterion_group!(benches, bench_full_slice_50_layers);
criterion_main!(benches);
