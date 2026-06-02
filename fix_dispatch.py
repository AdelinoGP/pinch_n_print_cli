#!/usr/bin/env python3
"""Apply all compile-error fixes to dispatch_tdd.rs."""
import sys

filepath = r'F:\slicerProject\pinch_n_print\crates\slicer-runtime\tests\contract\dispatch_tdd.rs'
with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()
original_len = len(content)

def R(content, old, new, label):
    if old not in content:
        print(f"MISSING: {label}")
        sys.exit(1)
    return content.replace(old, new, 1)

# Fix 1: Add imports
content = R(content,
    'use crate::common::wasm_cache;\nuse witness::{RawInfillWitness, RawInfillWitnessPoint1, RawSupportWitness};',
    'use crate::common::wasm_cache;\nuse crate::common::{commit_hec_for_test, finalization_input, layer_input, postpass_input, prepass_input};\nuse witness::{RawInfillWitness, RawInfillWitnessPoint1, RawSupportWitness};',
    "Fix1 import")
print("Fix 1 OK")

# Fix 2: FinalizationStageRunner (module is just &module, not &module.as_live())
content = R(content,
    '    let result = FinalizationStageRunner::run_stage(\n        &dispatcher,\n        &"PostPass::LayerFinalization".to_string(),\n        &module,\n        &blackboard,\n        &mut layers,\n    );',
    '    let result = FinalizationStageRunner::run_stage(\n        &dispatcher,\n        &"PostPass::LayerFinalization".to_string(),\n        &module.as_live(),\n        finalization_input(&blackboard),\n        &mut layers,\n    );',
    "Fix2 finalization")
print("Fix 2 OK")

# Fix 3: run_gcode_postprocess
content = R(content,
    '    let blackboard = Blackboard::new(empty_mesh_ir(), 0);\n    let mut gcode_ir = minimal_gcode_ir();\n\n    let result = dispatcher.run_gcode_postprocess(\n        &"PostPass::GCodePostProcess".to_string(),\n        &module,\n        &blackboard,\n        &mut gcode_ir,\n    );',
    '    let blackboard = Blackboard::new(empty_mesh_ir(), 0);\n    let mut commands: Vec<slicer_ir::GCodeCommand> = Vec::new();\n\n    let result = dispatcher.run_gcode_postprocess(\n        &"PostPass::GCodePostProcess".to_string(),\n        &module.as_live(),\n        postpass_input(&blackboard),\n        &mut commands,\n    );',
    "Fix3 gcode_postprocess")
print("Fix 3 OK")

# Fix 4: run_text_postprocess
content = R(content,
    '    let blackboard = Blackboard::new(empty_mesh_ir(), 0);\n    let result = dispatcher.run_text_postprocess(\n        &"PostPass::TextPostProcess".to_string(),\n        &module,\n        &blackboard,\n        "; some gcode".to_string(),\n    );',
    '    let blackboard = Blackboard::new(empty_mesh_ir(), 0);\n    let result = dispatcher.run_text_postprocess(\n        &"PostPass::TextPostProcess".to_string(),\n        &module.as_live(),\n        postpass_input(&blackboard),\n        "; some gcode".to_string(),\n    );',
    "Fix4 text_postprocess")
print("Fix 4 OK")

# Fix 5: 6-arg support isolation (module1/module2 no as_live, &bb1/&bb2, &mut arena1/2)
content = R(content,
    '    let mut arena1 = LayerArena::new();\n    arena1.set_slice(make_slice_ir(0, 0.2, 1, 1)).unwrap();\n    LayerStageRunner::run_stage(\n        &dispatcher,\n        &"Layer::Support".to_string(),\n        &layer,\n        &module1,\n        &bb1,\n        &mut arena1,\n    )\n    .unwrap();\n\n    // Second dispatch: 1 enforcer at layer 0\n    let mut bb2 = Blackboard::new(empty_mesh_ir(), 1);\n    bb2.commit_paint_regions(\n        Arc::new(make_paint_region_ir(0, 1, 2)),\n        Arc::new(PaintRegionRTreeIndex {\n            trees: HashMap::default(),\n        }),\n    )\n    .unwrap();\n    let module2 = make_compiled_module_with(\n        "com.test.support2",\n        "Layer::Support",\n        Arc::clone(&component),\n    );\n    let mut arena2 = LayerArena::new();\n    arena2.set_slice(make_slice_ir(0, 0.2, 1, 1)).unwrap();\n    LayerStageRunner::run_stage(\n        &dispatcher,\n        &"Layer::Support".to_string(),\n        &layer,\n        &module2,\n        &bb2,\n        &mut arena2,\n    )\n    .unwrap();\n\n    let sw1 = RawSupportWitness::decode(&arena1.support().unwrap().support_paths[0].points);',
    '    let mut arena1 = LayerArena::new();\n    arena1.set_slice(make_slice_ir(0, 0.2, 1, 1)).unwrap();\n    let commit1 = LayerStageRunner::run_stage(\n        &dispatcher,\n        &"Layer::Support".to_string(),\n        &layer,\n        &module1.as_live(),\n        layer_input(&bb1, &arena1),\n    )\n    .unwrap();\n    slicer_runtime::commit_layer_outputs_for_test("Layer::Support", "com.test.support", layer.index, commit1, &mut arena1, None).unwrap();\n\n    // Second dispatch: 1 enforcer at layer 0\n    let mut bb2 = Blackboard::new(empty_mesh_ir(), 1);\n    bb2.commit_paint_regions(\n        Arc::new(make_paint_region_ir(0, 1, 2)),\n        Arc::new(PaintRegionRTreeIndex {\n            trees: HashMap::default(),\n        }),\n    )\n    .unwrap();\n    let module2 = make_compiled_module_with(\n        "com.test.support2",\n        "Layer::Support",\n        Arc::clone(&component),\n    );\n    let mut arena2 = LayerArena::new();\n    arena2.set_slice(make_slice_ir(0, 0.2, 1, 1)).unwrap();\n    let commit2 = LayerStageRunner::run_stage(\n        &dispatcher,\n        &"Layer::Support".to_string(),\n        &layer,\n        &module2.as_live(),\n        layer_input(&bb2, &arena2),\n    )\n    .unwrap();\n    slicer_runtime::commit_layer_outputs_for_test("Layer::Support", "com.test.support2", layer.index, commit2, &mut arena2, None).unwrap();\n\n    let sw1 = RawSupportWitness::decode(&arena1.support().unwrap().support_paths[0].points);',
    "Fix5 support isolation")
print("Fix 5 OK")

# Fix 6: 6-arg infill no-paint vs paint
content = R(content,
    '    let mut arena1 = LayerArena::new();\n    arena1.set_slice(make_slice_ir(0, 0.2, 1, 1)).unwrap();\n    LayerStageRunner::run_stage(\n        &dispatcher,\n        &"Layer::Infill".to_string(),\n        &layer,\n        &module1,\n        &bb_no_paint,\n        &mut arena1,\n    )\n    .unwrap();\n\n    // Run with paint\n    let mut bb_with_paint = Blackboard::new(empty_mesh_ir(), 1);\n    bb_with_paint\n        .commit_paint_regions(\n            Arc::new(make_paint_region_ir(0, 5, 3)),\n            Arc::new(PaintRegionRTreeIndex {\n                trees: HashMap::default(),\n            }),\n        )\n        .unwrap();\n    let module2 =\n        make_compiled_module_with("com.test.infill2", "Layer::Infill", Arc::clone(&component));\n    let mut arena2 = LayerArena::new();\n    arena2.set_slice(make_slice_ir(0, 0.2, 1, 1)).unwrap();\n    LayerStageRunner::run_stage(\n        &dispatcher,\n        &"Layer::Infill".to_string(),\n        &layer,\n        &module2,\n        &bb_with_paint,\n        &mut arena2,\n    )\n    .unwrap();',
    '    let mut arena1 = LayerArena::new();\n    arena1.set_slice(make_slice_ir(0, 0.2, 1, 1)).unwrap();\n    let commit1 = LayerStageRunner::run_stage(\n        &dispatcher,\n        &"Layer::Infill".to_string(),\n        &layer,\n        &module1.as_live(),\n        layer_input(&bb_no_paint, &arena1),\n    )\n    .unwrap();\n    slicer_runtime::commit_layer_outputs_for_test("Layer::Infill", "com.test.infill", layer.index, commit1, &mut arena1, None).unwrap();\n\n    // Run with paint\n    let mut bb_with_paint = Blackboard::new(empty_mesh_ir(), 1);\n    bb_with_paint\n        .commit_paint_regions(\n            Arc::new(make_paint_region_ir(0, 5, 3)),\n            Arc::new(PaintRegionRTreeIndex {\n                trees: HashMap::default(),\n            }),\n        )\n        .unwrap();\n    let module2 =\n        make_compiled_module_with("com.test.infill2", "Layer::Infill", Arc::clone(&component));\n    let mut arena2 = LayerArena::new();\n    arena2.set_slice(make_slice_ir(0, 0.2, 1, 1)).unwrap();\n    let commit2 = LayerStageRunner::run_stage(\n        &dispatcher,\n        &"Layer::Infill".to_string(),\n        &layer,\n        &module2.as_live(),\n        layer_input(&bb_with_paint, &arena2),\n    )\n    .unwrap();\n    slicer_runtime::commit_layer_outputs_for_test("Layer::Infill", "com.test.infill2", layer.index, commit2, &mut arena2, None).unwrap();',
    "Fix6 infill no-paint vs paint")
print("Fix 6 OK")

# Fix 7: 6-arg InfillPostProcess isolation ipp1/ipp2 (uses &m1, &m2 no .as_live())
content = R(content,
    '    let mut a1 = LayerArena::new();\n    a1.set_perimeter(make_perimeter_ir(0, 5, 1, 2)).unwrap();\n    LayerStageRunner::run_stage(\n        &dispatcher,\n        &"Layer::InfillPostProcess".to_string(),\n        &layer,\n        &m1,\n        &blackboard,\n        &mut a1,\n    )\n    .unwrap();\n\n    let m2 = make_compiled_module_with(\n        "com.test.ipp2",\n        "Layer::InfillPostProcess",\n        Arc::clone(&component),\n    );\n    let mut a2 = LayerArena::new();\n    a2.set_perimeter(make_perimeter_ir(0, 1, 7, 3)).unwrap();\n    LayerStageRunner::run_stage(\n        &dispatcher,\n        &"Layer::InfillPostProcess".to_string(),\n        &layer,\n        &m2,\n        &blackboard,\n        &mut a2,\n    )\n    .unwrap();\n\n    let i1 = a1.infill().unwrap();',
    '    let mut a1 = LayerArena::new();\n    a1.set_perimeter(make_perimeter_ir(0, 5, 1, 2)).unwrap();\n    let commit_a1 = LayerStageRunner::run_stage(\n        &dispatcher,\n        &"Layer::InfillPostProcess".to_string(),\n        &layer,\n        &m1.as_live(),\n        layer_input(&blackboard, &a1),\n    )\n    .unwrap();\n    slicer_runtime::commit_layer_outputs_for_test("Layer::InfillPostProcess", "com.test.ipp1", layer.index, commit_a1, &mut a1, None).unwrap();\n\n    let m2 = make_compiled_module_with(\n        "com.test.ipp2",\n        "Layer::InfillPostProcess",\n        Arc::clone(&component),\n    );\n    let mut a2 = LayerArena::new();\n    a2.set_perimeter(make_perimeter_ir(0, 1, 7, 3)).unwrap();\n    let commit_a2 = LayerStageRunner::run_stage(\n        &dispatcher,\n        &"Layer::InfillPostProcess".to_string(),\n        &layer,\n        &m2.as_live(),\n        layer_input(&blackboard, &a2),\n    )\n    .unwrap();\n    slicer_runtime::commit_layer_outputs_for_test("Layer::InfillPostProcess", "com.test.ipp2", layer.index, commit_a2, &mut a2, None).unwrap();\n\n    let i1 = a1.infill().unwrap();',
    "Fix7 InfillPostProcess isolation")
print("Fix 7 OK")

# Fix 8: 6-arg PerimetersPostProcess isolation iso1/iso2 (uses &m1, &m2)
content = R(content,
    '    LayerStageRunner::run_stage(\n        &dispatcher,\n        &"Layer::PerimetersPostProcess".to_string(),\n        &layer,\n        &m1,\n        &blackboard,\n        &mut a1,\n    )\n    .unwrap();\n\n    let m2 = make_compiled_module_with(\n        "com.test.iso2",\n        "Layer::PerimetersPostProcess",\n        Arc::clone(&component),\n    );\n    let mut a2 = LayerArena::new();\n    a2.set_perimeter(make_perimeter_ir_with_ids(0, &[("alt", 999)], 1, 0))\n        .unwrap();\n    LayerStageRunner::run_stage(\n        &dispatcher,\n        &"Layer::PerimetersPostProcess".to_string(),\n        &layer,\n        &m2,\n        &blackboard,\n        &mut a2,\n    )\n    .unwrap();\n\n    let p1 = a1.perimeter().unwrap();',
    '    let commit_a1 = LayerStageRunner::run_stage(\n        &dispatcher,\n        &"Layer::PerimetersPostProcess".to_string(),\n        &layer,\n        &m1.as_live(),\n        layer_input(&blackboard, &a1),\n    )\n    .unwrap();\n    slicer_runtime::commit_layer_outputs_for_test("Layer::PerimetersPostProcess", "com.test.iso1", layer.index, commit_a1, &mut a1, None).unwrap();\n\n    let m2 = make_compiled_module_with(\n        "com.test.iso2",\n        "Layer::PerimetersPostProcess",\n        Arc::clone(&component),\n    );\n    let mut a2 = LayerArena::new();\n    a2.set_perimeter(make_perimeter_ir_with_ids(0, &[("alt", 999)], 1, 0))\n        .unwrap();\n    let commit_a2 = LayerStageRunner::run_stage(\n        &dispatcher,\n        &"Layer::PerimetersPostProcess".to_string(),\n        &layer,\n        &m2.as_live(),\n        layer_input(&blackboard, &a2),\n    )\n    .unwrap();\n    slicer_runtime::commit_layer_outputs_for_test("Layer::PerimetersPostProcess", "com.test.iso2", layer.index, commit_a2, &mut a2, None).unwrap();\n\n    let p1 = a1.perimeter().unwrap();',
    "Fix8 PerimetersPostProcess isolation")
print("Fix 8 OK")

# Fix 9: 6-arg SupportPostProcess isolation spp-iso1/spp-iso2
content = R(content,
    '    LayerStageRunner::run_stage(\n        &dispatcher,\n        &"Layer::SupportPostProcess".to_string(),\n        &layer,\n        &m1,\n        &blackboard,\n        &mut a1,\n    )\n    .unwrap();\n\n    let m2 = make_compiled_module_with(\n        "com.test.spp-iso2",\n        "Layer::SupportPostProcess",\n        Arc::clone(&component),\n    );\n    let mut a2 = LayerArena::new();\n    a2.set_slice(make_slice_ir(0, 0.2, 1, 1)).unwrap();\n    LayerStageRunner::run_stage(\n        &dispatcher,\n        &"Layer::SupportPostProcess".to_string(),\n        &layer,\n        &m2,\n        &blackboard,\n        &mut a2,\n    )\n    .unwrap();\n\n    assert_eq!(\n        a1.support().unwrap().support_paths.len(),\n        3,\n        "dispatch 1 kept its 3 regions"\n    );',
    '    let commit_a1 = LayerStageRunner::run_stage(\n        &dispatcher,\n        &"Layer::SupportPostProcess".to_string(),\n        &layer,\n        &m1.as_live(),\n        layer_input(&blackboard, &a1),\n    )\n    .unwrap();\n    slicer_runtime::commit_layer_outputs_for_test("Layer::SupportPostProcess", "com.test.spp-iso1", layer.index, commit_a1, &mut a1, None).unwrap();\n\n    let m2 = make_compiled_module_with(\n        "com.test.spp-iso2",\n        "Layer::SupportPostProcess",\n        Arc::clone(&component),\n    );\n    let mut a2 = LayerArena::new();\n    a2.set_slice(make_slice_ir(0, 0.2, 1, 1)).unwrap();\n    let commit_a2 = LayerStageRunner::run_stage(\n        &dispatcher,\n        &"Layer::SupportPostProcess".to_string(),\n        &layer,\n        &m2.as_live(),\n        layer_input(&blackboard, &a2),\n    )\n    .unwrap();\n    slicer_runtime::commit_layer_outputs_for_test("Layer::SupportPostProcess", "com.test.spp-iso2", layer.index, commit_a2, &mut a2, None).unwrap();\n\n    assert_eq!(\n        a1.support().unwrap().support_paths.len(),\n        3,\n        "dispatch 1 kept its 3 regions"\n    );',
    "Fix9 SupportPostProcess isolation")
print("Fix 9 OK")

# Fix 10: commit_layer_outputs_for_test(&ctx) -> commit_hec_for_test (fan-speed)
content = R(content,
    '    let mut arena = LayerArena::new();\n    let err = slicer_runtime::commit_layer_outputs_for_test(\n        "Layer::PathOptimization",\n        "com.test.pathopt-bad",\n        0,\n        &ctx,\n        &mut arena,\n        None,\n    )\n    .expect_err("fan-speed override must be rejected");',
    '    let mut arena = LayerArena::new();\n    let err = commit_hec_for_test(\n        "Layer::PathOptimization",\n        "com.test.pathopt-bad",\n        0,\n        &ctx,\n        &mut arena,\n        None,\n    )\n    .expect_err("fan-speed override must be rejected");',
    "Fix10 hec fan-speed")
print("Fix 10 OK")

# Fix 11: commit_layer_outputs_for_test(&ctx) -> commit_hec_for_test (comment/raw)
content = R(content,
    '    let mut arena = LayerArena::new();\n    slicer_runtime::commit_layer_outputs_for_test(\n        "Layer::PathOptimization",\n        "com.test.pathopt-ann",\n        0,\n        &ctx,\n        &mut arena,\n        None,\n    )\n    .expect("comment/raw must commit successfully");',
    '    let mut arena = LayerArena::new();\n    commit_hec_for_test(\n        "Layer::PathOptimization",\n        "com.test.pathopt-ann",\n        0,\n        &ctx,\n        &mut arena,\n        None,\n    )\n    .expect("comment/raw must commit successfully");',
    "Fix11 hec comment/raw")
print("Fix 11 OK")

# Fix 12: commit_layer_outputs_for_test(&ctx) -> commit_hec_for_test (det2 loop)
content = R(content,
    '    let mut snapshots = Vec::new();\n    for _ in 0..3 {\n        let mut arena = LayerArena::new();\n        let ctx = mk_ctx();\n        slicer_runtime::commit_layer_outputs_for_test(\n            "Layer::PathOptimization",\n            "com.test.pathopt-det2",\n            0,\n            &ctx,\n            &mut arena,\n            None,\n        )\n        .unwrap();',
    '    let mut snapshots = Vec::new();\n    for _ in 0..3 {\n        let mut arena = LayerArena::new();\n        let ctx = mk_ctx();\n        commit_hec_for_test(\n            "Layer::PathOptimization",\n            "com.test.pathopt-det2",\n            0,\n            &ctx,\n            &mut arena,\n            None,\n        )\n        .unwrap();',
    "Fix12 hec det2")
print("Fix 12 OK")

# Fix 13: commit_layer_outputs_for_test(&ctx) -> commit_hec_for_test (z-hop)
content = R(content,
    '    let mut arena = LayerArena::new();\n    slicer_runtime::commit_layer_outputs_for_test(\n        "Layer::PathOptimization",\n        "com.test.pathopt-zhop",\n        0,\n        &ctx,\n        &mut arena,\n        None,\n    )\n    .expect("z-hop must commit");',
    '    let mut arena = LayerArena::new();\n    commit_hec_for_test(\n        "Layer::PathOptimization",\n        "com.test.pathopt-zhop",\n        0,\n        &ctx,\n        &mut arena,\n        None,\n    )\n    .expect("z-hop must commit");',
    "Fix13 hec z-hop")
print("Fix 13 OK")

# Fix 14: commit_layer_outputs_for_test(&ctx) -> commit_hec_for_test (zhop-norm)
content = R(content,
    '    slicer_runtime::commit_layer_outputs_for_test(\n        "Layer::PathOptimization",\n        "com.test.pathopt-zhop-norm",\n        0,\n        &ctx,\n        &mut arena,\n        None,\n    )\n    .expect("ZHop with arbitrary entity index must be accepted and normalized to anchor");',
    '    commit_hec_for_test(\n        "Layer::PathOptimization",\n        "com.test.pathopt-zhop-norm",\n        0,\n        &ctx,\n        &mut arena,\n        None,\n    )\n    .expect("ZHop with arbitrary entity index must be accepted and normalized to anchor");',
    "Fix14 hec zhop-norm")
print("Fix 14 OK")

# Fix 15: commit_layer_outputs_for_test(&ctx) -> commit_hec_for_test (bad hop)
content = R(content,
    '        let mut arena = LayerArena::new();\n        let err = slicer_runtime::commit_layer_outputs_for_test(\n            "Layer::PathOptimization",\n            "com.test.pathopt-zhop-bad",\n            0,\n            &ctx,\n            &mut arena,\n            None,\n        )\n        .expect_err("bad hop_height must fail");',
    '        let mut arena = LayerArena::new();\n        let err = commit_hec_for_test(\n            "Layer::PathOptimization",\n            "com.test.pathopt-zhop-bad",\n            0,\n            &ctx,\n            &mut arena,\n            None,\n        )\n        .expect_err("bad hop_height must fail");',
    "Fix15 hec bad-hop")
print("Fix 15 OK")

# Fix 16: prepass layer_planning - result.unwrap().0 -> result.unwrap()
# This is in 'prepass_layer_planning_returns_typed_ir_on_success'
content = R(content,
    '    // Must return LayerPlan(...) variant, not None.\n    match result.unwrap().0 {\n        PrepassStageOutput::LayerPlan(ir) => {',
    '    // Must return LayerPlan(...) variant, not None.\n    match result.unwrap() {\n        PrepassStageOutput::LayerPlan(ir) => {',
    "Fix16 result.0 on prepass layer plan")
print("Fix 16 OK")

# Fix 17: deterministic layer plan - Ok((PrepassStageOutput::LayerPlan(ir), _)) -> Ok(...)
content = R(content,
    '        match PrepassStageRunner::run_stage(\n            &dispatcher,\n            &"PrePass::LayerPlanning".to_string(),\n            &module.as_live(),\n            prepass_input(&blackboard),\n        ) {\n            Ok((PrepassStageOutput::LayerPlan(ir), _)) => ir,\n            Ok((other, _)) => panic!(\n                "expected LayerPlan variant, got discriminant {:?}",\n                std::mem::discriminant(&other)\n            ),\n            Err(e) => panic!("dispatch failed: {e}"),\n        }',
    '        match PrepassStageRunner::run_stage(\n            &dispatcher,\n            &"PrePass::LayerPlanning".to_string(),\n            &module.as_live(),\n            prepass_input(&blackboard),\n        ) {\n            Ok(PrepassStageOutput::LayerPlan(ir)) => ir,\n            Ok(other) => panic!(\n                "expected LayerPlan variant, got discriminant {:?}",\n                std::mem::discriminant(&other)\n            ),\n            Err(e) => panic!("dispatch failed: {e}"),\n        }',
    "Fix17 deterministic layer plan tuple")
print("Fix 17 OK")

# Fix 18: PrepassExecutionError -> PrepassRunnerError import
content = R(content,
    '    use slicer_runtime::PrepassExecutionError;\n\n    let engine = wasm_cache::shared_engine();\n    let component = match load_layer_planner_default(&engine) {',
    '    use slicer_ir::PrepassRunnerError;\n\n    let engine = wasm_cache::shared_engine();\n    let component = match load_layer_planner_default(&engine) {',
    "Fix18 PrepassExecutionError import")
print("Fix 18 OK")

# Fix 18b: PrepassExecutionError match arm -> PrepassRunnerError
content = R(content,
    '    match result.unwrap_err() {\n        PrepassExecutionError::FatalModule {\n            stage_id,\n            module_id,\n            message,\n        } => {',
    '    match result.unwrap_err() {\n        PrepassRunnerError::FatalModule {\n            stage_id,\n            module_id,\n            message,\n        } => {',
    "Fix18b PrepassExecutionError match arm")
print("Fix 18b OK")

# Fix 19: mesh_seg result.0 -> result (empty marks)
# The two occurrences have different preceding text - find first one
idx1 = content.find('    match result.0 {\n        PrepassStageOutput::MeshSegmentation(ir) => {\n            assert_eq!(\n                ir.schema_version,')
if idx1 < 0:
    print("MISSING: Fix19")
    sys.exit(1)
content = content[:idx1] + '    match result {\n        PrepassStageOutput::MeshSegmentation(ir) => {\n            assert_eq!(\n                ir.schema_version,' + content[idx1+len('    match result.0 {\n        PrepassStageOutput::MeshSegmentation(ir) => {\n            assert_eq!(\n                ir.schema_version,'):]
print("Fix 19 OK")

# Fix 20: mesh_seg result.0 -> result (config marks - "let ir = match result.0")
content = R(content,
    '    let ir = match result.0 {\n        PrepassStageOutput::MeshSegmentation(ir) => ir,',
    '    let ir = match result {\n        PrepassStageOutput::MeshSegmentation(ir) => ir,',
    "Fix20 mesh_seg marks result.0")
print("Fix 20 OK")

# Fix 21: mesh_seg deterministic - &blackboard -> prepass_input + result.0 -> result
content = R(content,
    '        let result = PrepassStageRunner::run_stage(\n            &dispatcher,\n            &"PrePass::MeshSegmentation".to_string(),\n            &module.as_live(),\n            &blackboard,\n        )\n        .expect("dispatch succeeds");\n        match result.0 {\n            PrepassStageOutput::MeshSegmentation(ir) => ir.marks.clone(),\n            _ => panic!("wrong variant"),\n        }',
    '        let result = PrepassStageRunner::run_stage(\n            &dispatcher,\n            &"PrePass::MeshSegmentation".to_string(),\n            &module.as_live(),\n            prepass_input(&blackboard),\n        )\n        .expect("dispatch succeeds");\n        match result {\n            PrepassStageOutput::MeshSegmentation(ir) => ir.marks.clone(),\n            _ => panic!("wrong variant"),\n        }',
    "Fix21 mesh_seg deterministic")
print("Fix 21 OK")

# Fix 22: result.0 on LayerStageCommitData (path-optimization dispatch emits marker)
content = R(content,
    '    let result = LayerStageRunner::run_stage(\n        &dispatcher,\n        &"Layer::PathOptimization".to_string(),\n        &layer,\n        &module,\n        &blackboard,\n        &mut arena,\n    )\n    .expect("path-optimization dispatch must succeed");\n    assert_eq!(result.0, LayerStageOutput::Success);',
    '    let commit_data = LayerStageRunner::run_stage(\n        &dispatcher,\n        &"Layer::PathOptimization".to_string(),\n        &layer,\n        &module.as_live(),\n        layer_input(&blackboard, &arena),\n    )\n    .expect("path-optimization dispatch must succeed");\n    slicer_runtime::commit_layer_outputs_for_test(\n        "Layer::PathOptimization",\n        "com.test.path-opt-dispatch",\n        layer.index,\n        commit_data,\n        &mut arena,\n        None,\n    )\n    .expect("commit must succeed");',
    "Fix22 result.0 LayerStageCommitData")
print("Fix 22 OK")

# Also fix the subsequent comment
content = R(content,
    '    // Dispatch already ran commit_layer_outputs internally; the comment\n    // is now in the arena as a deferred annotation. Verify it.\n    let annotations = arena.take_deferred_annotations();',
    '    // The commit path wrote annotations to the arena. Verify them.\n    let annotations = arena.take_deferred_annotations();',
    "Fix22b comment update")
print("Fix 22b OK")

# Fix 23: layer planner macro Ok((..LayerPlan(ir), _)) -> Ok(..LayerPlan(ir))
content = R(content,
    '    let ir = match result {\n        Ok((PrepassStageOutput::LayerPlan(ir), _)) => ir,\n        Ok((other, _)) => panic!(\n            "expected PrepassStageOutput::LayerPlan, got {:?}",\n            std::mem::discriminant(&other)\n        ),\n        Err(e) => panic!("dispatch failed: {e}"),\n    };\n\n    // 2mm object / 0.2mm layer height = 10 layers.',
    '    let ir = match result {\n        Ok(PrepassStageOutput::LayerPlan(ir)) => ir,\n        Ok(other) => panic!(\n            "expected PrepassStageOutput::LayerPlan, got {:?}",\n            std::mem::discriminant(&other)\n        ),\n        Err(e) => panic!("dispatch failed: {e}"),\n    };\n\n    // 2mm object / 0.2mm layer height = 10 layers.',
    "Fix23 macro layer planner 10 layers")
print("Fix 23 OK")

# Fix 24: layer planner deterministic Ok((.., _)) -> Ok(..)
content = R(content,
    '        match PrepassStageRunner::run_stage(\n            &dispatcher,\n            &"PrePass::LayerPlanning".to_string(),\n            &module.as_live(),\n            prepass_input(&blackboard),\n        ) {\n            Ok((PrepassStageOutput::LayerPlan(ir), _)) => ir,\n            Ok((other, _)) => panic!(\n                "expected LayerPlan, got {:?}",\n                std::mem::discriminant(&other)\n            ),\n            Err(e) => panic!("dispatch failed: {e}"),\n        }',
    '        match PrepassStageRunner::run_stage(\n            &dispatcher,\n            &"PrePass::LayerPlanning".to_string(),\n            &module.as_live(),\n            prepass_input(&blackboard),\n        ) {\n            Ok(PrepassStageOutput::LayerPlan(ir)) => ir,\n            Ok(other) => panic!(\n                "expected LayerPlan, got {:?}",\n                std::mem::discriminant(&other)\n            ),\n            Err(e) => panic!("dispatch failed: {e}"),\n        }',
    "Fix24 layer planner deterministic")
print("Fix 24 OK")

# Fix 25: mesh_analysis macro Ok((.., _)) -> Ok(..)
content = R(content,
    '    let aux = match result {\n        Ok((PrepassStageOutput::MeshAnalysisAuxiliary(a), _)) => a,\n        Ok((other, _)) => panic!(\n            "expected PrepassStageOutput::MeshAnalysisAuxiliary, got {:?}",\n            std::mem::discriminant(&other)\n        ),\n        Err(e) => panic!("dispatch failed: {e}"),\n    };',
    '    let aux = match result {\n        Ok(PrepassStageOutput::MeshAnalysisAuxiliary(a)) => a,\n        Ok(other) => panic!(\n            "expected PrepassStageOutput::MeshAnalysisAuxiliary, got {:?}",\n            std::mem::discriminant(&other)\n        ),\n        Err(e) => panic!("dispatch failed: {e}"),\n    };',
    "Fix25 mesh_analysis macro aux")
print("Fix 25 OK")

# Fix 26: mesh_analysis deterministic Ok((.., _)) -> Ok(..)
content = R(content,
    '        match PrepassStageRunner::run_stage(\n            &dispatcher,\n            &"PrePass::MeshAnalysis".to_string(),\n            &module.as_live(),\n            prepass_input(&blackboard),\n        ) {\n            Ok((PrepassStageOutput::MeshAnalysisAuxiliary(a), _)) => a,\n            Ok((other, _)) => panic!(\n                "expected MeshAnalysisAuxiliary, got {:?}",\n                std::mem::discriminant(&other)\n            ),\n            Err(e) => panic!("dispatch failed: {e}"),\n        }',
    '        match PrepassStageRunner::run_stage(\n            &dispatcher,\n            &"PrePass::MeshAnalysis".to_string(),\n            &module.as_live(),\n            prepass_input(&blackboard),\n        ) {\n            Ok(PrepassStageOutput::MeshAnalysisAuxiliary(a)) => a,\n            Ok(other) => panic!(\n                "expected MeshAnalysisAuxiliary, got {:?}",\n                std::mem::discriminant(&other)\n            ),\n            Err(e) => panic!("dispatch failed: {e}"),\n        }',
    "Fix26 mesh_analysis deterministic")
print("Fix 26 OK")

# Fix 27: out.0 -> out (empty drain None check)
content = R(content,
    '    .expect("empty-config path must succeed");\n    assert!(matches!(out.0, PrepassStageOutput::None));',
    '    .expect("empty-config path must succeed");\n    assert!(matches!(out, PrepassStageOutput::None));',
    "Fix27 out.0 empty None")
print("Fix 27 OK")

# Fix 28: out.0 -> out (mesh_seg ordering)
content = R(content,
    '    .expect("mesh-segmentation dispatch must succeed");\n\n    let ir = match out.0 {\n        PrepassStageOutput::MeshSegmentation(ir) => ir,',
    '    .expect("mesh-segmentation dispatch must succeed");\n\n    let ir = match out {\n        PrepassStageOutput::MeshSegmentation(ir) => ir,',
    "Fix28 out.0 ordering")
print("Fix 28 OK")

# Fix 29: SeamPlan Ok((..SeamPlan(ir), _)) -> Ok(..SeamPlan(ir))
content = R(content,
    '        Ok((PrepassStageOutput::SeamPlan(ir), _)) => {',
    '        Ok(PrepassStageOutput::SeamPlan(ir)) => {',
    "Fix29a SeamPlan Ok tuple")
print("Fix 29a OK")

# Fix 29b: Ok((other, _)) after SeamPlan
content = R(content,
    '        Ok((other, _)) => panic!(\n            "expected PrepassStageOutput::SeamPlan, got {:?}",\n            std::mem::discriminant(&other)\n        ),',
    '        Ok(other) => panic!(\n            "expected PrepassStageOutput::SeamPlan, got {:?}",\n            std::mem::discriminant(&other)\n        ),',
    "Fix29b SeamPlan Ok other")
print("Fix 29b OK")

print(f"\nAll fixes done. Content length: {len(content)} (was {original_len})")

with open(filepath, 'w', encoding='utf-8') as f:
    f.write(content)
print("File saved.")
