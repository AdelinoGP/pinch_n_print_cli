#![allow(missing_docs)]

use std::path::Path;

use crate::common::support_wedge;

fn compare_count_and_endpoints(
    current_count: usize,
    current_endpoints: &[[f32; 3]],
    baseline_count: usize,
    baseline_endpoints: &[[f32; 3]],
    max_drift: f64,
    max_hausdorff: f32,
) -> Result<(), String> {
    if baseline_count == 0 {
        return Err("baseline branch count must be greater than zero".to_string());
    }
    let expected_baseline_endpoints = baseline_count
        .checked_mul(2)
        .ok_or_else(|| "baseline branch count is too large".to_string())?;
    if baseline_endpoints.len() != expected_baseline_endpoints {
        return Err(format!(
            "baseline endpoint count mismatch: expected {} endpoints for {} branches, got {}",
            expected_baseline_endpoints,
            baseline_count,
            baseline_endpoints.len()
        ));
    }
    if current_endpoints.is_empty() {
        return Err("current wedge output must contain branch endpoints".to_string());
    }

    let count_drift = (current_count as f64 - baseline_count as f64).abs() / baseline_count as f64;
    if count_drift > max_drift {
        return Err(format!(
            "branch count drift > {:.0}%: drift={:.4} current={} baseline={}",
            max_drift * 100.0,
            count_drift,
            current_count,
            baseline_count
        ));
    }
    let current_flat: Vec<f32> = current_endpoints
        .iter()
        .flat_map(|p| p.iter().copied())
        .collect();
    let baseline_flat: Vec<f32> = baseline_endpoints
        .iter()
        .flat_map(|p| p.iter().copied())
        .collect();
    let hausdorff = support_wedge::symmetric_hausdorff(&current_flat, &baseline_flat);
    if hausdorff > max_hausdorff {
        return Err(format!(
            "Hausdorff distance {:.4} mm exceeds tolerance {:.1} mm (current {} points vs baseline {} points)",
            hausdorff,
            max_hausdorff,
            current_flat.len() / 3,
            baseline_flat.len() / 3
        ));
    }
    Ok(())
}

#[test]
fn current_wedge_output_stays_within_self_capture_tolerance() {
    let ctx = support_wedge::prepare_wedge_context(true);
    let plan = ctx
        .blackboard
        .support_plan()
        .expect("support_plan must be committed when enable_support=true");
    let current_count = support_wedge::branch_segment_count(plan);
    let current_endpoints = support_wedge::branch_endpoints(plan);

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let golden_dir = manifest_dir
        .join("..")
        .join("..")
        .join("resources")
        .join("golden");
    let count_path = golden_dir.join("support_regression_wedge_branch_count.txt");
    let endpoints_path = golden_dir.join("support_regression_wedge_endpoints.txt");

    let regen = std::env::var("SUPPORT_WEDGE_REGEN_GOLDEN").as_deref() == Ok("1");

    if regen {
        std::fs::create_dir_all(&golden_dir).expect("create golden dir");
        std::fs::write(&count_path, format!("{current_count}\n"))
            .expect("write branch count golden");
        let mut endpoints_text = String::new();
        for [x, y, z] in &current_endpoints {
            endpoints_text.push_str(&format!("{x:.6} {y:.6} {z:.6}\n"));
        }
        std::fs::write(&endpoints_path, endpoints_text).expect("write endpoints golden");
        eprintln!(
            "Regenerated goldens: count={} endpoints={}",
            current_count,
            current_endpoints.len()
        );
        return;
    }

    let count_missing_msg = format!(
        "missing golden file: {}. Run with SUPPORT_WEDGE_REGEN_GOLDEN=1 to capture.",
        count_path.display()
    );
    let count_raw = std::fs::read_to_string(&count_path).expect(&count_missing_msg);
    if count_raw.trim().is_empty() {
        panic!("{count_missing_msg}");
    }

    let endpoints_missing_msg = format!(
        "missing golden file: {}. Run with SUPPORT_WEDGE_REGEN_GOLDEN=1 to capture.",
        endpoints_path.display()
    );
    let endpoints_raw = std::fs::read_to_string(&endpoints_path).expect(&endpoints_missing_msg);
    if endpoints_raw.trim().is_empty() {
        panic!("{endpoints_missing_msg}");
    }

    let baseline_count = support_wedge::parse_branch_count(&count_raw);
    let baseline_endpoints = support_wedge::parse_endpoints(&endpoints_raw);
    assert!(
        baseline_count > 0,
        "baseline branch count must be greater than zero"
    );
    let expected_baseline_endpoints = baseline_count
        .checked_mul(2)
        .expect("baseline branch count is too large");
    assert_eq!(
        baseline_endpoints.len(),
        expected_baseline_endpoints,
        "baseline endpoint count must equal baseline_count * 2"
    );
    assert!(
        !current_endpoints.is_empty(),
        "current wedge output must contain branch endpoints"
    );

    compare_count_and_endpoints(
        current_count,
        &current_endpoints,
        baseline_count,
        &baseline_endpoints,
        0.10,
        0.5,
    )
    .expect("current wedge output must stay within self-capture tolerance");
}

#[test]
fn detects_intentional_branch_count_drift() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let golden_dir = manifest_dir
        .join("..")
        .join("..")
        .join("resources")
        .join("golden");
    let count_path = golden_dir.join("support_regression_wedge_branch_count.txt");
    let endpoints_path = golden_dir.join("support_regression_wedge_endpoints.txt");

    let count_bytes_start =
        std::fs::read(&count_path).expect("read golden branch count at test start");
    let endpoints_bytes_start =
        std::fs::read(&endpoints_path).expect("read golden endpoints at test start");

    let baseline_count_raw =
        std::fs::read_to_string(&count_path).expect("read golden branch count");
    let baseline_count: usize = baseline_count_raw
        .trim()
        .parse()
        .expect("baseline count must be a valid integer");
    let baseline_endpoints_raw =
        std::fs::read_to_string(&endpoints_path).expect("read golden endpoints");
    let baseline_endpoints = support_wedge::parse_endpoints(&baseline_endpoints_raw);

    let mutated_count = baseline_count + (baseline_count * 30 / 100) + 1;
    let mutated_endpoints = baseline_endpoints.clone();

    let err = compare_count_and_endpoints(
        mutated_count,
        &mutated_endpoints,
        baseline_count,
        &baseline_endpoints,
        0.10,
        0.5,
    )
    .expect_err("compare_count_and_endpoints must reject >25% branch count drift");

    assert!(
        err.contains("branch count drift > 10%"),
        "expected error to contain 'branch count drift > 10%', got: {err}"
    );

    let count_bytes_end = std::fs::read(&count_path).expect("read golden branch count at test end");
    let endpoints_bytes_end =
        std::fs::read(&endpoints_path).expect("read golden endpoints at test end");
    assert_eq!(
        count_bytes_start, count_bytes_end,
        "golden branch count file must not be modified by test"
    );
    assert_eq!(
        endpoints_bytes_start, endpoints_bytes_end,
        "golden endpoints file must not be modified by test"
    );
}
