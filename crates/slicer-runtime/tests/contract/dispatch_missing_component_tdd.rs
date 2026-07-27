use slicer_wasm_host::PostpassStageRunner;

use crate::common::dispatch_fixture::for_stage;

#[test]
fn missing_component_is_fatal_for_all_five_stages() {
    let fixture = for_stage("PrePass::LayerPlanning").no_wasm().build();
    let result = fixture.run_prepass();
    let Err(slicer_ir::PrepassRunnerError::FatalModule { message, .. }) = result else {
        panic!("missing prepass component was not fatal: {result:?}");
    };
    assert!(
        message.contains("MissingComponent"),
        "prepass error did not report MissingComponent: {message}"
    );
    assert!(
        message.contains("com.test.fixture"),
        "prepass error did not name the module: {message}"
    );

    let mut fixture = for_stage("Layer::Infill").no_wasm().build();
    let layer = slicer_ir::GlobalLayer::default();
    let result = fixture.run_layer(&layer);
    let Err(slicer_ir::LayerStageError::FatalModule { message, .. }) = result else {
        panic!("missing layer component was not fatal: {result:?}");
    };
    assert!(
        message.contains("MissingComponent"),
        "layer error did not report MissingComponent: {message}"
    );
    assert!(
        message.contains("com.test.fixture"),
        "layer error did not name the module: {message}"
    );

    let fixture = for_stage("Finalization::GCode").no_wasm().build();
    let mut layers = Vec::new();
    let result = fixture.run_finalization(&mut layers);
    let Err(slicer_ir::FinalizationError::FatalModule { message, .. }) = result else {
        panic!("missing finalization component was not fatal: {result:?}");
    };
    assert!(
        message.contains("MissingComponent"),
        "finalization error did not report MissingComponent: {message}"
    );
    assert!(
        message.contains("com.test.fixture"),
        "finalization error did not name the module: {message}"
    );

    let fixture = for_stage("PostProcess::GCode").no_wasm().build();
    let mut gcode = slicer_ir::GCodeIR::default();
    let result = fixture.run_postpass(&mut gcode);
    let Err(slicer_ir::PostpassError::FatalModule { message, .. }) = result else {
        panic!("missing G-code postpass component was not fatal: {result:?}");
    };
    assert!(
        message.contains("MissingComponent"),
        "G-code postpass error did not report MissingComponent: {message}"
    );
    assert!(
        message.contains("com.test.fixture"),
        "G-code postpass error did not name the module: {message}"
    );

    let fixture = for_stage("PostProcess::Text").no_wasm().build();
    let live = fixture.bundle.as_live();
    let input = crate::common::postpass_input(&fixture.blackboard);
    let result = PostpassStageRunner::run_text_postprocess(
        &fixture.dispatcher,
        &"PostProcess::Text".to_string(),
        &live,
        input,
        "test".to_string(),
    );
    let Err(slicer_ir::PostpassError::FatalModule { message, .. }) = result else {
        panic!("missing text postpass component was not fatal: {result:?}");
    };
    assert!(
        message.contains("MissingComponent"),
        "text postpass error did not report MissingComponent: {message}"
    );
    assert!(
        message.contains("com.test.fixture"),
        "text postpass error did not name the module: {message}"
    );
}
