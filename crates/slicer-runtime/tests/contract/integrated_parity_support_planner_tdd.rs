#![allow(missing_docs)]

//! AC-5 (ADR-0056): support-planner must produce structurally equivalent
//! SupportPlanIR through native and real wasm dispatch paths.

use std::path::PathBuf;
use std::sync::Arc;

use slicer_core::PrepassStageOutput;
use slicer_ir::{ConfigView, SemVer, StageId};
use slicer_runtime::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_runtime::run::PrepassContext;
use slicer_runtime::{
    CompiledModuleBuilder, CompiledModuleLive, LoadedModuleBuilder, PrepassStageInput,
    PrepassStageRunner, WasmInstancePool, WasmRuntimeDispatcher,
};
use support_planner::SupportPlanner;

use crate::common::parity_invariants::{assert_prepass_parity_structural, ParityTolerance};
use crate::common::{support_wedge, wasm_cache};

fn module_id() -> slicer_ir::ModuleId {
    "com.core.support-planner".to_string()
}

fn input(ctx: &PrepassContext) -> PrepassStageInput<'static> {
    // exhaustive: parity comparison pins every field explicitly
    PrepassStageInput {
        mesh: Arc::clone(ctx.blackboard.mesh()),
        layer_plan: ctx.blackboard.layer_plan().map(Arc::clone),
        slice_ir: ctx.blackboard.slice_ir().map(Arc::clone),
        region_map: ctx.blackboard.region_map().map(Arc::clone),
        support_geometry: ctx.blackboard.support_geometry().map(Arc::clone),
        _phantom: std::marker::PhantomData,
    }
}

fn support_plan(output: &PrepassStageOutput) -> &slicer_ir::SupportPlanIR {
    match output {
        PrepassStageOutput::SupportPlan(plan) => plan,
        other => panic!("expected SupportPlan output, got {other:?}"),
    }
}

#[test]
fn integrated_parity_support_planner_native_matches_wasm() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let config = Arc::new(ConfigView::new());
    let wasm_module = CompiledModuleBuilder::new(module_id())
        .config_view(Arc::clone(&config))
        .build();
    let native_module = CompiledModuleBuilder::new(module_id())
        .config_view(Arc::clone(&config))
        .build();

    let loaded = LoadedModuleBuilder::new(
        module_id().as_str(),
        SemVer {
            major: 0,
            minor: 1,
            patch: 0,
        },
        "PrePass::SupportGeometry",
        String::new(),
        PathBuf::from("/dev/null"),
    )
    .min_host_version(SemVer {
        major: 0,
        minor: 1,
        patch: 0,
    })
    .min_ir_schema(SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    })
    .max_ir_schema(SemVer {
        major: 2,
        minor: 0,
        patch: 0,
    })
    .layer_parallel_safe(false)
    .build();
    let pool = Arc::new(
        build_wasm_instance_pool(
            loaded.id(),
            loaded.stage(),
            loaded.layer_parallel_safe(),
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("build instance pool"),
    );
    let wasm_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("modules/core-modules/support-planner/support-planner.wasm");
    assert!(
        wasm_path.exists(),
        "real support-planner guest is missing: {} (run `cargo xtask build-guests`)",
        wasm_path.display()
    );
    let component = wasm_cache::compiled_component_at(&wasm_path);
    let wasm_live = CompiledModuleLive::new(
        wasm_module.module_id(),
        pool,
        Some(component),
        wasm_module.claims(),
        Arc::clone(wasm_module.config_view()),
    );
    let native_live = CompiledModuleLive::new(
        native_module.module_id(),
        WasmInstancePool::placeholder(),
        None,
        native_module.claims(),
        Arc::clone(native_module.config_view()),
    )
    .with_native_entry(SupportPlanner::__slicer_native_entry());

    let ctx = support_wedge::prepare_wedge_context(true);
    let native_output = PrepassStageRunner::run_stage(
        &dispatcher,
        &StageId::from("PrePass::SupportGeometry"),
        &native_live,
        input(&ctx),
    )
    .expect("native dispatch");
    let wasm_output = PrepassStageRunner::run_stage(
        &dispatcher,
        &StageId::from("PrePass::SupportGeometry"),
        &wasm_live,
        input(&ctx),
    )
    .expect("wasm dispatch");

    for (name, output) in [("native", &native_output), ("wasm", &wasm_output)] {
        let plan = support_plan(output);
        assert!(
            plan.entries.iter().any(|entry| {
                entry
                    .branch_segments
                    .iter()
                    .any(|segment| !segment.points.is_empty())
            }),
            "{name} support plan must contain an entry, branch segment, and point"
        );
        eprintln!(
            "{name} support plan: entries={}, segments={}, points={}",
            plan.entries.len(),
            plan.entries
                .iter()
                .map(|entry| entry.branch_segments.len())
                .sum::<usize>(),
            plan.entries
                .iter()
                .flat_map(|entry| entry.branch_segments.iter())
                .map(|segment| segment.points.len())
                .sum::<usize>()
        );
    }
    assert_prepass_parity_structural(&native_output, &wasm_output, ParityTolerance::default())
        .expect("AC-5 structural parity native vs wasm for support-planner");
}
