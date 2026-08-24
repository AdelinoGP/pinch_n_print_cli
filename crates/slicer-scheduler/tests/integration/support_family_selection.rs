use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    ActiveRegion, ConfigId, ConfigView, GlobalLayer, RegionKey, RegionPlan, ResolvedConfig, SemVer,
};
use slicer_scheduler::{
    build_execution_plan, select_support_family, ExecutionModuleBinding, ExecutionPlanRequest,
    LoadDiagnostic, LoadedModuleBuilder, SortedStageModules,
};
use std::path::PathBuf;

fn version() -> SemVer {
    SemVer {
        major: 0,
        minor: 1,
        patch: 0,
    }
}

pub fn support_family_selection() {
    assert_eq!(
        select_support_family(Some("tree"), Some("normal(auto)")),
        "traditional"
    );
    assert_eq!(
        select_support_family(Some("tree"), Some("classic(manual)")),
        "traditional"
    );
    assert_eq!(
        select_support_family(Some("traditional"), Some("tree(auto)")),
        "tree"
    );
    assert_eq!(
        select_support_family(Some("traditional"), Some("hybrid(auto)")),
        "tree"
    );
    assert_eq!(select_support_family(Some("tree"), None), "tree");
}

pub fn support_family_candidates_are_retained() {
    let make = |id: &str, family: &str| {
        LoadedModuleBuilder::new(
            id,
            version(),
            "Layer::Support",
            slicer_schema::TIER_LAYER,
            PathBuf::from(id),
        )
        .claims(vec![
            "support-generator".into(),
            format!("support-family:{family}"),
        ])
        .min_host_version(version())
        .min_ir_schema(version())
        .max_ir_schema(SemVer {
            major: 2,
            minor: 0,
            patch: 0,
        })
        .layer_parallel_safe(true)
        .build()
    };
    let mut modules = vec![
        make("com.core.traditional-support", "traditional"),
        make("com.core.tree-support", "tree"),
    ];
    let mut diagnostics = Vec::new();
    let kept = slicer_scheduler::dedup_same_claim_modules_for_test(&mut modules, &mut diagnostics);
    assert_eq!(kept.len(), 2);
}

#[test]
fn live_region_dispatch_retains_paired_family_candidates() {
    let make = |id: &str, claims: &[&str]| {
        LoadedModuleBuilder::new(
            id,
            version(),
            "Layer::Support",
            slicer_schema::TIER_LAYER,
            PathBuf::from(id),
        )
        .claims(claims.iter().map(|claim| (*claim).to_string()).collect())
        .min_host_version(version())
        .min_ir_schema(version())
        .max_ir_schema(SemVer {
            major: 2,
            minor: 0,
            patch: 0,
        })
        .layer_parallel_safe(true)
        .build()
    };
    let planner = make(
        "com.core.tree-support-planner",
        &[
            "support-planner",
            "support-family:traditional",
            "support-family:tree",
        ],
    );
    let traditional = make(
        "com.core.traditional-support",
        &["support-generator", "support-family:traditional"],
    );
    let tree = make(
        "com.core.tree-support",
        &["support-generator", "support-family:tree"],
    );
    let bindings = [&planner, &traditional, &tree]
        .into_iter()
        .map(|module| ExecutionModuleBinding {
            config_view: Arc::new(ConfigView::new()),
            module: (*module).clone(),
        })
        .collect();
    let regions = vec![
        ActiveRegion {
            object_id: "object-a".into(),
            region_id: 1,
            resolved_config: ResolvedConfig {
                support_type: slicer_ir::SupportType::NormalAuto,
                ..Default::default()
            },
            ..Default::default()
        },
        ActiveRegion {
            object_id: "object-a".into(),
            region_id: 2,
            resolved_config: ResolvedConfig {
                extensions: std::collections::BTreeMap::from([(
                    "support_family".into(),
                    slicer_ir::ConfigValue::String("tree".into()),
                )]),
                ..Default::default()
            },
            ..Default::default()
        },
    ];
    let global_layers = Arc::new(vec![GlobalLayer {
        index: 0,
        active_regions: regions,
        ..Default::default()
    }]);
    let region_plans = Arc::new(HashMap::from([
        (
            RegionKey {
                global_layer_index: 0,
                object_id: "object-a".into(),
                region_id: 1,
                variant_chain: vec![],
            },
            RegionPlan::default(),
        ),
        (
            RegionKey {
                global_layer_index: 0,
                object_id: "object-a".into(),
                region_id: 2,
                variant_chain: vec![],
            },
            RegionPlan {
                config: ConfigId::default(),
                ..Default::default()
            },
        ),
    ]));
    let plan = build_execution_plan(
        &ExecutionPlanRequest {
            sorted_stages: vec![SortedStageModules {
                stage_id: "Layer::Support".into(),
                module_ids: vec![
                    planner.id().to_string(),
                    traditional.id().to_string(),
                    tree.id().to_string(),
                ],
            }],
            module_bindings: bindings,
            global_layers,
            region_plans,
        },
        &mut Vec::<LoadDiagnostic>::new(),
    )
    .expect("paired support modules must produce a live plan");
    let stage = plan
        .per_layer_stages
        .iter()
        .find(|stage| stage.stage_id == "Layer::Support")
        .unwrap();
    assert_eq!(stage.modules.len(), 3);
    let regions_for = |module_id: &str| {
        let module = stage
            .modules
            .iter()
            .find(|module| module.module_id() == module_id)
            .unwrap();
        plan.resolve_active_regions(&plan.global_layers[0], module)
            .iter()
            .map(|region| region.region_id)
            .collect::<Vec<_>>()
    };
    assert_eq!(regions_for("com.core.tree-support-planner"), vec![1, 2]);
    assert_eq!(regions_for("com.core.traditional-support"), vec![1]);
    assert_eq!(regions_for("com.core.tree-support"), vec![2]);
    assert_eq!(
        select_support_family(Some("traditional"), None),
        "traditional"
    );
    assert_eq!(select_support_family(None, Some("tree(auto)")), "tree");
    assert!(stage
        .modules
        .iter()
        .any(|module| module.module_id() == "com.core.traditional-support"));
    assert!(stage
        .modules
        .iter()
        .any(|module| module.module_id() == "com.core.tree-support"));
}

pub fn support_family_selection_two_planners_coexist() {
    let make = |id: &str, claims: &[&str]| {
        LoadedModuleBuilder::new(
            id,
            version(),
            "Layer::Support",
            slicer_schema::TIER_LAYER,
            PathBuf::from(id),
        )
        .claims(claims.iter().map(|claim| (*claim).to_string()).collect())
        .min_host_version(version())
        .min_ir_schema(version())
        .max_ir_schema(SemVer {
            major: 2,
            minor: 0,
            patch: 0,
        })
        .layer_parallel_safe(true)
        .build()
    };
    let traditional_planner = make(
        "com.core.traditional-support-planner",
        &["support-planner", "support-family:traditional"],
    );
    let tree_planner = make(
        "com.core.tree-support-planner",
        &["support-planner", "support-family:tree"],
    );
    let traditional = make(
        "com.core.traditional-support",
        &["support-generator", "support-family:traditional"],
    );
    let tree = make(
        "com.core.tree-support",
        &["support-generator", "support-family:tree"],
    );

    // Both family planners must survive claim dedup (no alphabetical collapse).
    let mut dedup_modules = vec![
        traditional_planner.clone(),
        tree_planner.clone(),
        traditional.clone(),
        tree.clone(),
    ];
    let mut diagnostics = Vec::new();
    let kept =
        slicer_scheduler::dedup_same_claim_modules_for_test(&mut dedup_modules, &mut diagnostics);
    assert_eq!(
        kept.len(),
        4,
        "both planners and both renderers survive dedup"
    );

    let bindings = [&traditional_planner, &tree_planner, &traditional, &tree]
        .into_iter()
        .map(|module| ExecutionModuleBinding {
            config_view: Arc::new(ConfigView::new()),
            module: (*module).clone(),
        })
        .collect();
    let regions = vec![
        ActiveRegion {
            object_id: "object-a".into(),
            region_id: 1,
            resolved_config: ResolvedConfig {
                support_type: slicer_ir::SupportType::NormalAuto,
                ..Default::default()
            },
            ..Default::default()
        },
        ActiveRegion {
            object_id: "object-a".into(),
            region_id: 2,
            resolved_config: ResolvedConfig {
                extensions: std::collections::BTreeMap::from([(
                    "support_family".into(),
                    slicer_ir::ConfigValue::String("tree".into()),
                )]),
                ..Default::default()
            },
            ..Default::default()
        },
    ];
    let global_layers = Arc::new(vec![GlobalLayer {
        index: 0,
        active_regions: regions,
        ..Default::default()
    }]);
    let region_plans = Arc::new(HashMap::from([
        (
            RegionKey {
                global_layer_index: 0,
                object_id: "object-a".into(),
                region_id: 1,
                variant_chain: vec![],
            },
            RegionPlan::default(),
        ),
        (
            RegionKey {
                global_layer_index: 0,
                object_id: "object-a".into(),
                region_id: 2,
                variant_chain: vec![],
            },
            RegionPlan {
                config: ConfigId::default(),
                ..Default::default()
            },
        ),
    ]));
    let plan = build_execution_plan(
        &ExecutionPlanRequest {
            sorted_stages: vec![SortedStageModules {
                stage_id: "Layer::Support".into(),
                module_ids: vec![
                    traditional_planner.id().to_string(),
                    tree_planner.id().to_string(),
                    traditional.id().to_string(),
                    tree.id().to_string(),
                ],
            }],
            module_bindings: bindings,
            global_layers,
            region_plans,
        },
        &mut Vec::<LoadDiagnostic>::new(),
    )
    .expect("paired support modules must produce a live plan");
    let stage = plan
        .per_layer_stages
        .iter()
        .find(|stage| stage.stage_id == "Layer::Support")
        .unwrap();
    assert_eq!(stage.modules.len(), 4);
    let regions_for = |module_id: &str| {
        let module = stage
            .modules
            .iter()
            .find(|module| module.module_id() == module_id)
            .unwrap();
        plan.resolve_active_regions(&plan.global_layers[0], module)
            .iter()
            .map(|region| region.region_id)
            .collect::<Vec<_>>()
    };
    assert_eq!(regions_for("com.core.traditional-support-planner"), vec![1]);
    assert_eq!(regions_for("com.core.tree-support-planner"), vec![2]);
    assert_eq!(regions_for("com.core.traditional-support"), vec![1]);
    assert_eq!(regions_for("com.core.tree-support"), vec![2]);
}
