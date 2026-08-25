//! Packet 234a AC-5: bundle-primary internal-bridge arbitration evidence.

use pnp_cli_locator::pnp_cli_bin;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root")
}

fn run_capture(model: &Path, tag: &str) -> Value {
    let root = root();
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    let request = target.join(format!("{tag}_request.json"));
    let output = target.join(format!("{tag}_bundle"));
    let config = target.join(format!("{tag}_config.json"));
    let _ = std::fs::remove_dir_all(&output);
    std::fs::write(
        &config,
        br#"{"layer_height":0.2,"first_layer_height":0.25,"nozzle_diameter":0.5,"line_width":0.525,"bridge_flow":0.95,"internal_bridge_flow":0.95,"infill_density":0.25,"sparse_infill_density":25.0,"top_shell_layers":3,"bottom_shell_layers":3,"enable_support":true,"dont_filter_internal_bridges":false,"thick_bridges":false,"thick_internal_bridges":false}"#,
    )
    .expect("write visual-debug config");
    let body = serde_json::json!({
        "schema_version": "1.0.0",
        "source": {
            "kind": "model",
            "model": model,
            "config": config,
            "module_dirs": [root.join("modules/core-modules")],
            "path": null
        },
        "layers": [{"start": 0, "end": 1000}],
        "taps": ["Layer::Slice"],
        "visualizations": [],
        "resolution_scale": 1,
        "frame": "model"
    });
    std::fs::write(
        &request,
        serde_json::to_vec(&body).expect("serialize request"),
    )
    .expect("write visual-debug request");
    let proc = Command::new(pnp_cli_bin())
        .args(["visual-debug", "--request"])
        .arg(&request)
        .args(["--output"])
        .arg(&output)
        .output()
        .expect("pnp_cli visual-debug should execute");
    assert!(
        proc.status.success(),
        "visual-debug failed: {}",
        String::from_utf8_lossy(&proc.stderr)
    );
    serde_json::from_slice(
        &std::fs::read(output.join("manifest.json")).expect("read visual-debug manifest"),
    )
    .expect("manifest JSON")
}

fn polygon_area(poly: &Value) -> f64 {
    let ring_area = |ring: &Value| {
        let points = ring["points"].as_array().expect("polygon points");
        points
            .iter()
            .zip(points.iter().cycle().skip(1))
            .take(points.len())
            .map(|(a, b)| {
                a["x"].as_i64().expect("x") as f64 * b["y"].as_i64().expect("y") as f64
                    - b["x"].as_i64().expect("x") as f64 * a["y"].as_i64().expect("y") as f64
            })
            .sum::<f64>()
            .abs()
            / 2.0
    };
    (ring_area(&poly["contour"])
        - poly["holes"]
            .as_array()
            .expect("polygon holes")
            .iter()
            .map(ring_area)
            .sum::<f64>())
        / 100_000_000.0
}

#[test]
fn calicat_internal_bridge_arbiter_e2e_tdd() {
    let model = root().join("resources/calicat.stl");
    assert!(model.exists(), "calicat fixture missing");
    let manifest = run_capture(&model, "calicat_internal_bridge_arbiter");
    let mut qualified = Vec::new();
    for image in manifest["images"].as_array().expect("images") {
        let payload = &image["typed_capture"]["value"];
        for region in payload["regions"].as_array().expect("SliceIR regions") {
            let area: f64 = region["internal_bridge_areas"]
                .as_array()
                .expect("internal bridge areas")
                .iter()
                .map(polygon_area)
                .sum();
            if area > 0.0 {
                qualified.push((image["layer_z"].as_f64().expect("layer z"), area));
            }
        }
    }
    // Owner ruling 2026-08-25: low-z and cavity-breadth residuals are tracked by
    // DEV-149 and DEV-150 in docs/DEVIATION_LOG.md, not this packet.
    let expected = [(4.45, 23.2), (18.45, 8.4), (29.45, 143.2)];
    assert_eq!(
        qualified.len(),
        expected.len(),
        "qualified site set: {qualified:?}"
    );
    for (expected_z, expected_area) in expected {
        assert!(
            qualified.iter().any(|(z, area)| {
                (z - expected_z).abs() <= 0.11
                    && (area - expected_area).abs() <= expected_area * 0.10
            }),
            "missing baseline site ({expected_z}, {expected_area}): {qualified:?}"
        );
    }
    let cavity = qualified
        .iter()
        .find(|(z, _)| (*z - 29.45).abs() <= 0.11)
        .expect("cavity-ceiling baseline site");
    // The AC-5 bar comes from tmp/calicat_orcaSlicer.gcode. Its 0.525 mm
    // line width at 0.95 bridge flow gives a 0.49875 mm effective width.
    let length = cavity.1 / 0.49875;
    println!("AC-5 qualified sites={qualified:?}, cavity length={length:.3} mm");
    assert!(
        (29.15..=29.75).contains(&cavity.0),
        "qualified z={}",
        cavity.0
    );
}
