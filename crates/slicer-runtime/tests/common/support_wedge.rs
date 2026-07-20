#![allow(dead_code)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{ConfigValue, SupportPlanIR};
use slicer_runtime::run::PrepassContext;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root must be resolvable")
}

fn wedge_path() -> PathBuf {
    workspace_root()
        .join("resources")
        .join("regression_wedge.stl")
}

fn core_modules_dir() -> PathBuf {
    workspace_root().join("modules").join("core-modules")
}

pub fn prepare_wedge_context(support_enabled: bool) -> PrepassContext {
    prepare_wedge_context_with_overrides(support_enabled, &[])
}

pub fn prepare_wedge_context_with_overrides(
    support_enabled: bool,
    overrides: &[(&str, ConfigValue)],
) -> PrepassContext {
    let model = wedge_path();
    assert!(
        model.exists(),
        "regression_wedge.stl must exist at {}",
        model.display()
    );

    let mesh = Arc::new(
        slicer_model_io::load_model(&model).expect("load regression_wedge.stl must succeed"),
    );

    let mut config: HashMap<String, ConfigValue> = HashMap::new();
    config.insert(
        "support_enabled".to_string(),
        ConfigValue::Bool(support_enabled),
    );
    for (key, value) in overrides {
        config.insert((*key).to_string(), value.clone());
    }

    let module_dirs = vec![core_modules_dir()];

    let ctx = slicer_runtime::run::prepare_prepass_context(mesh, config, &module_dirs, true)
        .expect("prepare_prepass_context must succeed");

    if support_enabled {
        let plan = ctx
            .blackboard
            .support_plan()
            .expect("support_plan must be committed when support_enabled=true");
        assert!(
            !plan.entries.is_empty(),
            "support_enabled=true but SupportPlanIR.entries is empty (len={}) for fixture {}",
            plan.entries.len(),
            model.display()
        );
    }

    ctx
}

pub fn branch_segment_count(plan: &SupportPlanIR) -> usize {
    plan.entries.iter().map(|e| e.branch_segments.len()).sum()
}

pub fn branch_endpoints(plan: &SupportPlanIR) -> Vec<[f32; 3]> {
    let mut endpoints: Vec<[f32; 3]> = plan
        .entries
        .iter()
        .flat_map(|entry| {
            entry.branch_segments.iter().flat_map(|seg| {
                let first = seg
                    .points
                    .first()
                    .expect("branch segment must have at least one point");
                let last = seg
                    .points
                    .last()
                    .expect("branch segment must have at least one point");
                [[first.x, first.y, first.z], [last.x, last.y, last.z]]
            })
        })
        .collect();
    endpoints.sort_by(|a, b| {
        a[0].partial_cmp(&b[0])
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a[1].partial_cmp(&b[1]).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| a[2].partial_cmp(&b[2]).unwrap_or(std::cmp::Ordering::Equal))
    });
    endpoints
}

pub fn parse_branch_count(s: &str) -> usize {
    s.trim()
        .parse()
        .expect("branch count must be a valid integer")
}

pub fn parse_endpoints(s: &str) -> Vec<[f32; 3]> {
    s.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let parts: Vec<f32> = line
                .split_whitespace()
                .map(|v| v.parse().expect("endpoint value must be a valid f32"))
                .collect();
            assert_eq!(
                parts.len(),
                3,
                "each endpoint line must have exactly 3 values, got {}",
                parts.len()
            );
            [parts[0], parts[1], parts[2]]
        })
        .collect()
}

pub fn symmetric_hausdorff(a: &[f32], b: &[f32]) -> f32 {
    fn one_sided(from: &[f32], to: &[f32]) -> f32 {
        if from.is_empty() || to.is_empty() {
            return 0.0;
        }
        let mut max_dist = 0.0f32;
        for i in (0..from.len()).step_by(3) {
            let px = from[i];
            let py = from[i + 1];
            let pz = from[i + 2];
            let mut min_dist_sq = f32::MAX;
            for j in (0..to.len()).step_by(3) {
                let dx = px - to[j];
                let dy = py - to[j + 1];
                let dz = pz - to[j + 2];
                let d = dx * dx + dy * dy + dz * dz;
                if d < min_dist_sq {
                    min_dist_sq = d;
                }
            }
            let d = min_dist_sq.sqrt();
            if d > max_dist {
                max_dist = d;
            }
        }
        max_dist
    }
    one_sided(a, b).max(one_sided(b, a))
}
