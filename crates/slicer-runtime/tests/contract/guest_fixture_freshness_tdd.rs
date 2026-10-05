//! TDD tests for test guest fixture freshness and reproducibility.
//!
//! These tests verify that:
//! - All expected test guest components exist on disk
//! - Guest source files are not newer than their .component.wasm
//! - Components are valid WASM (compile with wasmtime)
//! - The `cargo xtask build-guests` command can regenerate components from source

use std::path::PathBuf;

const GUESTS: &[(&str, &str)] = &[
    ("layer-infill-guest", "layer-infill-guest.component.wasm"),
    ("prepass-guest", "prepass-guest.component.wasm"),
    ("finalization-guest", "finalization-guest.component.wasm"),
    ("postpass-guest", "postpass-guest.component.wasm"),
    // TASK-109 round-trip witnesses â€” guests authored purely via the
    // macro-emitted wit_bindgen glue (no hand-rolled `wit_bindgen::generate!`).
    (
        "sdk-postpass-text-guest",
        "sdk-postpass-text-guest.component.wasm",
    ),
    (
        "sdk-finalization-guest",
        "sdk-finalization-guest.component.wasm",
    ),
    ("sdk-prepass-guest", "sdk-prepass-guest.component.wasm"),
    (
        "sdk-layer-infill-guest",
        "sdk-layer-infill-guest.component.wasm",
    ),
];

fn test_guests_dir() -> PathBuf {
    // Test-guests moved to slicer-wasm-host in P83.1.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("slicer-wasm-host")
        .join("test-guests")
}

#[test]
fn all_guest_component_files_exist() {
    let dir = test_guests_dir();
    for (guest_name, wasm_name) in GUESTS {
        let wasm_path = dir.join(wasm_name);
        assert!(
            wasm_path.exists(),
            "Missing test guest component: {wasm_name}. \
             Run: ./test-guests/build-test-guests.sh"
        );

        // Verify it's not empty
        let meta = std::fs::metadata(&wasm_path).unwrap();
        assert!(
            meta.len() > 100,
            "Test guest {wasm_name} is suspiciously small ({} bytes)",
            meta.len()
        );

        // Verify source directory exists
        let src = dir.join(guest_name).join("src").join("lib.rs");
        assert!(
            src.exists(),
            "Missing source for {guest_name}: expected {}",
            src.display()
        );
    }
}

#[test]
fn guest_components_are_not_stale() {
    let dir = test_guests_dir();
    for (guest_name, wasm_name) in GUESTS {
        let src = dir.join(guest_name).join("src").join("lib.rs");
        let toml = dir.join(guest_name).join("Cargo.toml");
        let wasm = dir.join(wasm_name);

        let wasm_mtime = std::fs::metadata(&wasm).unwrap().modified().unwrap();
        let src_mtime = std::fs::metadata(&src).unwrap().modified().unwrap();

        assert!(
            src_mtime <= wasm_mtime,
            "Test guest {wasm_name} is stale: source {guest_name}/src/lib.rs is newer. \
             Run: ./test-guests/build-test-guests.sh"
        );

        {
            let toml_mtime = std::fs::metadata(&toml).unwrap().modified().unwrap();
            assert!(
                toml_mtime <= wasm_mtime,
                "Test guest {wasm_name} is stale: {guest_name}/Cargo.toml is newer. \
                 Run: ./test-guests/build-test-guests.sh"
            );
        }
    }
}

#[test]
fn guest_components_are_valid_wasm_components() {
    // Engine is shared (cheap) but `compile_component` stays raw — this test
    // is the canonical compile-freshness check for every guest. Routing it
    // through `wasm_cache::compiled_guest` would mean a cache hit no longer
    // exercises a real compile, masking future regressions.
    let engine = crate::common::wasm_cache::shared_engine();
    let dir = test_guests_dir();

    for (_guest_name, wasm_name) in GUESTS {
        let wasm_path = dir.join(wasm_name);
        let bytes = std::fs::read(&wasm_path).unwrap();
        let result = engine.compile_component(&bytes);
        assert!(
            result.is_ok(),
            "Test guest {wasm_name} failed to compile as a WASM component: {:?}",
            result.err()
        );
    }
}

#[test]
fn xtask_check_mode_reports_freshness() {
    // cargo xtask test builds this executable before checking guests. Locate
    // it beside the test binary's deps directory, respecting Cargo's target
    // directory and debug/release profile rather than pinning target/debug.
    let test_exe = std::env::current_exe().expect("test executable path");
    let profile_dir = test_exe.parent().unwrap().parent().unwrap();
    let xtask = profile_dir.join(format!("xtask{}", std::env::consts::EXE_SUFFIX));
    let output = std::process::Command::new(&xtask)
        .arg("build-guests")
        .arg("--check")
        .output()
        .unwrap_or_else(|err| {
            panic!(
                "cannot run {}: {err}; run cargo xtask test",
                xtask.display()
            )
        });

    // EXIT_FRESH is 0; stale (1) and infrastructure failure (3) must not pass.
    assert_eq!(
        output.status.code(),
        Some(0),
        "xtask build-guests --check must confirm fresh artifacts; stdout: {}; stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
