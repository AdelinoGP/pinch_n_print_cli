//! AC-4 follow-up: prove that the `infill_overlap` CLI value reaches the
//! pipeline and produces measurably different gcode output.
//!
//! The CLI-binding test (`crates/slicer-ir/tests/infill_overlap_cli_binding_tdd.rs`)
//! proves the key reaches `ResolvedConfig.infill_overlap` and appears in
//! `to_config_map`. This test proves the VALUE flows into the linker and
//! changes the emitted gcode. We run the wedge slice twice, once with
//! `infill_overlap=0.30` and once with `infill_overlap=0.45`, and assert
//! the gcode outputs differ.
//!
//! Authoritative pipe command:
//!   `cargo test -p slicer-runtime --test e2e -- infill_overlap_changes_gcode`

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root canonicalize")
}

fn pnp_cli_bin() -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let profile = if std::env::var("PROFILE").as_deref() == Ok("release") {
        "release"
    } else {
        "debug"
    };
    PathBuf::from(manifest)
        .join("..")
        .join("..")
        .join("target")
        .join(profile)
        .join(if cfg!(windows) {
            "pnp_cli.exe"
        } else {
            "pnp_cli"
        })
}

fn core_modules_dir() -> PathBuf {
    repo_root().join("modules").join("core-modules")
}

fn wedge_stl() -> PathBuf {
    repo_root().join("resources").join("regression_wedge.stl")
}

fn config_path_with(infill_overlap: f32) -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let path = PathBuf::from(manifest)
        .join("target")
        .join(format!("infill_overlap_{infill_overlap}.json"));
    let json = format!(
        r#"{{
  "infill_overlap": {}
}}
"#,
        infill_overlap
    );
    std::fs::write(&path, json).expect("write config");
    path
}

fn gcode_path_for(label: &str) -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest)
        .join("target")
        .join(format!("infill_overlap_{label}.gcode"))
}

fn run_slice_with_config(config: &PathBuf, output: &PathBuf) -> std::process::Output {
    let bin = pnp_cli_bin();
    let model = wedge_stl();
    let modules = core_modules_dir();
    Command::new(&bin)
        .args(["slice", "--model"])
        .arg(&model)
        .args(["--output"])
        .arg(output)
        .args(["--config"])
        .arg(config)
        .args(["--module-dir"])
        .arg(&modules)
        .output()
        .expect("pnp_cli binary should execute")
}

fn count_g1_moves(gcode: &str) -> usize {
    gcode
        .lines()
        .filter(|l| l.trim().starts_with("G1 "))
        .count()
}

#[test]
fn infill_overlap_changes_gcode() {
    let bin = pnp_cli_bin();
    assert!(bin.exists(), "pnp_cli not built at {}", bin.display());

    let config_30 = config_path_with(0.30);
    let config_45 = config_path_with(0.45);
    let gcode_30 = gcode_path_for("0_30");
    let gcode_45 = gcode_path_for("0_45");
    let _ = std::fs::remove_file(&gcode_30);
    let _ = std::fs::remove_file(&gcode_45);

    let proc_30 = run_slice_with_config(&config_30, &gcode_30);
    let proc_45 = run_slice_with_config(&config_45, &gcode_45);
    assert!(
        proc_30.status.success() && proc_45.status.success(),
        "pnp_cli must succeed for both runs"
    );
    assert!(
        gcode_30.exists() && gcode_45.exists(),
        "both gcodes must be written"
    );

    let text_30 = std::fs::read_to_string(&gcode_30).expect("read gcode 0.30");
    let text_45 = std::fs::read_to_string(&gcode_45).expect("read gcode 0.45");

    // The two outputs must differ. The cleanest signal: the gcode's
    // CONFIG_BLOCK must carry the per-run value (`0.30` vs `0.45`). A
    // weaker but still meaningful signal: the total G1 move count must
    // differ (different overlap = different boundary positions = different
    // total move count).
    let moves_30 = count_g1_moves(&text_30);
    let moves_45 = count_g1_moves(&text_45);
    let delta = (moves_30 as isize - moves_45 as isize).unsigned_abs();
    let max_moves = moves_30.max(moves_45) as usize;
    let pct = if max_moves == 0 {
        0.0
    } else {
        100.0 * delta as f64 / max_moves as f64
    };
    assert!(
        text_30 != text_45,
        "AC-4: the two gcode outputs are byte-identical; `infill_overlap` did not change the slice."
    );
    assert!(
        pct > 0.1,
        "AC-4: G1-move count delta is {delta} ({pct:.3}%), expected > 0.1%. \
         0.30 produced {moves_30} moves, 0.45 produced {moves_45}."
    );
}
