//! Build script for slicer-host: checks test guest component freshness.
///
/// If any test guest source (src/lib.rs or Cargo.toml) is newer than the
/// corresponding .component.wasm, emits a cargo:warning so the developer
/// knows to rebuild. Also sets cargo:rerun-if-changed on all guest sources
/// so that cargo re-checks when they change.
use std::path::Path;

fn main() {
    let guests = [
        "layer-infill-guest",
        "prepass-guest",
        "finalization-guest",
        "postpass-guest",
    ];

    let test_guests_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("test-guests");

    for guest in &guests {
        let src = test_guests_dir.join(guest).join("src").join("lib.rs");
        let toml = test_guests_dir.join(guest).join("Cargo.toml");
        let wasm = test_guests_dir.join(format!("{guest}.component.wasm"));

        // Tell cargo to rerun this build script when guest sources change.
        if src.exists() {
            println!("cargo:rerun-if-changed={}", src.display());
        }
        if toml.exists() {
            println!("cargo:rerun-if-changed={}", toml.display());
        }
        if wasm.exists() {
            println!("cargo:rerun-if-changed={}", wasm.display());
        }

        // Check freshness.
        if !wasm.exists() {
            println!(
                "cargo:warning=Test guest {guest}.component.wasm is missing. \
                 Run: ./test-guests/build-test-guests.sh"
            );
            continue;
        }

        if !src.exists() {
            continue;
        }

        let src_mtime = std::fs::metadata(&src).and_then(|m| m.modified()).ok();
        let toml_mtime = std::fs::metadata(&toml).and_then(|m| m.modified()).ok();
        let wasm_mtime = std::fs::metadata(&wasm).and_then(|m| m.modified()).ok();

        if let (Some(wasm_t), Some(src_t)) = (wasm_mtime, src_mtime) {
            if src_t > wasm_t {
                println!(
                    "cargo:warning=Test guest {guest}.component.wasm is stale \
                     (source is newer). Run: ./test-guests/build-test-guests.sh"
                );
            }
        }
        if let (Some(wasm_t), Some(toml_t)) = (wasm_mtime, toml_mtime) {
            if toml_t > wasm_t {
                println!(
                    "cargo:warning=Test guest {guest}.component.wasm is stale \
                     (Cargo.toml is newer). Run: ./test-guests/build-test-guests.sh"
                );
            }
        }
    }
}
