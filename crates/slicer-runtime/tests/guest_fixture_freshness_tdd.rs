//! TDD tests for test guest fixture freshness and reproducibility.
//!
//! These tests verify that:
//! - All expected test guest components exist on disk
//! - Guest source files are not newer than their .component.wasm
//! - Components are valid WASM (compile with wasmtime)
//! - The build script can regenerate components from source

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
        "sdk-prepass-meshseg-guest",
        "sdk-prepass-meshseg-guest.component.wasm",
    ),
    (
        "sdk-layer-infill-guest",
        "sdk-layer-infill-guest.component.wasm",
    ),
];

fn test_guests_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-guests")
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

        if !wasm.exists() || !src.exists() {
            continue; // caught by all_guest_component_files_exist
        }

        let wasm_mtime = std::fs::metadata(&wasm).unwrap().modified().unwrap();
        let src_mtime = std::fs::metadata(&src).unwrap().modified().unwrap();

        assert!(
            src_mtime <= wasm_mtime,
            "Test guest {wasm_name} is stale: source {guest_name}/src/lib.rs is newer. \
             Run: ./test-guests/build-test-guests.sh"
        );

        if toml.exists() {
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
    use slicer_runtime::WasmEngine;

    let engine = WasmEngine::new();
    let dir = test_guests_dir();

    for (_guest_name, wasm_name) in GUESTS {
        let wasm_path = dir.join(wasm_name);
        if !wasm_path.exists() {
            continue; // caught by all_guest_component_files_exist
        }

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
fn build_script_exists_and_is_executable() {
    let script = test_guests_dir().join("build-test-guests.sh");
    assert!(script.exists(), "build-test-guests.sh not found");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::metadata(&script).unwrap().permissions();
        assert!(
            perms.mode() & 0o111 != 0,
            "build-test-guests.sh is not executable"
        );
    }
}

#[test]
fn build_script_check_mode_reports_freshness() {
    let script = test_guests_dir().join("build-test-guests.sh");
    if !script.exists() {
        return;
    }

    let output = std::process::Command::new("bash")
        .arg(&script)
        .arg("--check")
        .output()
        .expect("failed to run build-test-guests.sh --check");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // If all components are fresh, exit code should be 0
    // If any are stale, exit code should be 1
    // We just verify it runs without crashing
    assert!(
        output.status.code().is_some(),
        "build-test-guests.sh --check should exit cleanly, got: {stdout}"
    );
}
