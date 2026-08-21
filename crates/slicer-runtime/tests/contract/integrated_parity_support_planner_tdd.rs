#![allow(missing_docs)]

//! AC-5 (ADR-0056): support-planner must produce structurally equivalent
//! SupportPlanIR through native and real wasm dispatch paths.

use std::path::PathBuf;
use std::sync::Arc;

use slicer_core::PrepassStageOutput;
use slicer_ir::{ConfigView, SemVer, StageId};
use slicer_runtime::run::PrepassContext;
use slicer_runtime::{PrepassStageInput, PrepassStageRunner};
use tree_support_planner::SupportPlanner;

use crate::common::support_wedge;
use crate::common::{
    integrated_parity_harness::{run_integrated_parity, IntegratedParitySpec},
    parity_invariants::{assert_prepass_parity_structural, ParityTolerance},
};

fn module_id() -> slicer_ir::ModuleId {
    "com.core.tree-support-planner".to_string()
}

fn input(ctx: &PrepassContext) -> PrepassStageInput<'static> {
    // exhaustive: parity comparison pins every field explicitly
    PrepassStageInput {
        mesh: Arc::clone(ctx.blackboard.mesh()),
        layer_plan: ctx.blackboard.layer_plan().map(Arc::clone),
        slice_ir: ctx.blackboard.slice_ir().map(Arc::clone),
        region_map: ctx.blackboard.region_map().map(Arc::clone),
        support_analysis: ctx.blackboard.support_analysis().map(Arc::clone),
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
    let config = Arc::new(ConfigView::new());
    let stage = StageId::from("PrePass::SupportGeometry");
    // `com.core.tree-support-planner` skips every candidate whose resolved
    // family is not "tree". The default wedge sets no `support_type`, so its
    // `family_assignments` are all "traditional" and the planner emitted an
    // empty plan on both sides — the parity comparison was vacuous.
    let ctx = support_wedge::prepare_wedge_context_tree(true);
    let (native_output, wasm_output) = run_integrated_parity(
        IntegratedParitySpec {
            module_id: module_id(),
            wasm_path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../modules/core-modules/tree-support-planner/tree-support-planner.wasm"),
            stage: stage.to_string(),
            version: SemVer {
                major: 0,
                minor: 1,
                patch: 0,
            },
            min_ir_schema: SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            max_ir_schema: SemVer {
                major: 2,
                minor: 0,
                patch: 0,
            },
            tier: String::new(),
            claims: Vec::new(),
            config,
            native_entry: SupportPlanner::__slicer_native_entry(),
        },
        |dispatcher, native_live, wasm_live| {
            let native_output =
                PrepassStageRunner::run_stage(dispatcher, &stage, native_live, input(&ctx))
                    .expect("native dispatch");
            let wasm_output =
                PrepassStageRunner::run_stage(dispatcher, &stage, wasm_live, input(&ctx))
                    .expect("wasm dispatch");
            (native_output, wasm_output)
        },
    );

    for (name, output) in [("native", &native_output), ("wasm", &wasm_output)] {
        let plan = support_plan(output);
        assert!(
            plan.entries.iter().any(|entry| {
                entry.roles.iter().any(|role| !role.regions.is_empty())
                    || entry
                        .skeleton
                        .as_ref()
                        .is_some_and(|s| !s.points.is_empty())
            }),
            "{name} support plan must contain structural support geometry"
        );
        eprintln!(
            "{name} support plan: entries={}, segments={}, points={}",
            plan.entries.len(),
            plan.entries
                .iter()
                .map(|entry| entry.roles.len())
                .sum::<usize>(),
            plan.entries
                .iter()
                .flat_map(|entry| entry.roles.iter())
                .map(|role| role.regions.len())
                .sum::<usize>()
        );
    }
    assert_prepass_parity_structural(&native_output, &wasm_output, ParityTolerance::default())
        .expect("AC-5 structural parity native vs wasm for support-planner");
}
