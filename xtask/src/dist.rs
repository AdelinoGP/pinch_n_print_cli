use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;

use crate::build_guests::{self, GuestTree};

pub fn dist_command(ws_root: &Path, debug: bool) -> i32 {
    let profile = if debug { "debug" } else { "release" };

    println!("xtask dist: building guest WASMs...");
    let code = build_guests::build_command(ws_root);
    if code != 0 {
        return code;
    }

    println!("xtask dist: building pnp_cli ({profile})...");
    let mut cmd = Command::new("cargo");
    cmd.current_dir(ws_root).args(["build", "-p", "pnp-cli"]);
    if !debug {
        cmd.arg("--release");
    }
    let out = match cmd.output() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("xtask dist: failed to spawn cargo: {e}");
            return 1;
        }
    };
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        eprintln!(
            "xtask dist: cargo build -p pnp-cli failed:\n{}",
            build_guests::tail_lines(&stderr, 20)
        );
        return 1;
    }

    let dist_dir = ws_root.join("target").join("dist");
    match fs::remove_dir_all(&dist_dir) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => {
            eprintln!("xtask dist: failed to clean {}: {e}", dist_dir.display());
            return 1;
        }
    }
    if let Err(e) = fs::create_dir_all(&dist_dir) {
        eprintln!("xtask dist: failed to create {}: {e}", dist_dir.display());
        return 1;
    }

    let bin_name = if cfg!(target_os = "windows") {
        "pnp_cli.exe"
    } else {
        "pnp_cli"
    };
    let bin_src = ws_root.join("target").join(profile).join(bin_name);
    let bin_dest = dist_dir.join(bin_name);
    if let Err(e) = fs::copy(&bin_src, &bin_dest) {
        eprintln!(
            "xtask dist: failed to copy {} -> {}: {e}",
            bin_src.display(),
            bin_dest.display()
        );
        return 1;
    }

    let modules_dir = dist_dir.join("modules");
    if let Err(e) = fs::create_dir_all(&modules_dir) {
        eprintln!(
            "xtask dist: failed to create {}: {e}",
            modules_dir.display()
        );
        return 1;
    }

    let (guests, _skips) = build_guests::discover_guests(ws_root);
    let mut module_count = 0usize;
    for spec in &guests {
        if spec.tree != GuestTree::Core {
            continue;
        }
        let wasm_src = ws_root.join(&spec.artifact_path);
        let toml_src = wasm_src.with_extension("toml");
        let stem = match spec.artifact_path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s,
            None => {
                eprintln!(
                    "xtask dist: artifact_path missing stem: {}",
                    spec.artifact_path.display()
                );
                return 1;
            }
        };
        let dest_dir = modules_dir.join(stem);
        if let Err(e) = fs::create_dir_all(&dest_dir) {
            eprintln!("xtask dist: failed to create {}: {e}", dest_dir.display());
            return 1;
        }
        let wasm_dest = dest_dir.join(format!("{stem}.wasm"));
        let toml_dest = dest_dir.join(format!("{stem}.toml"));
        if let Err(e) = fs::copy(&wasm_src, &wasm_dest) {
            eprintln!(
                "xtask dist: failed to copy {} -> {}: {e}",
                wasm_src.display(),
                wasm_dest.display()
            );
            return 1;
        }
        if let Err(e) = fs::copy(&toml_src, &toml_dest) {
            eprintln!(
                "xtask dist: failed to copy {} -> {}: {e}",
                toml_src.display(),
                toml_dest.display()
            );
            return 1;
        }
        module_count += 1;
    }

    println!(
        "xtask dist: staged 1 binary + {module_count} modules into {}",
        dist_dir.display()
    );
    0
}
