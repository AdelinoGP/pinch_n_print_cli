//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! machine-gcode-emit matches its manifest's declared postpass world/stage.

#![allow(missing_docs)]

use machine_gcode_emit::MachineGcodeEmit;

#[test]
fn binding_surface_matches_gcode_postprocess_stage() {
    assert_eq!(
        MachineGcodeEmit::__slicer_tier_id(),
        slicer_schema::TIER_POSTPASS
    );
    assert_eq!(MachineGcodeEmit::__slicer_trait_name(), "PostpassModule");
    assert_eq!(
        MachineGcodeEmit::__slicer_stage_name(),
        "PostPass::GCodePostProcess"
    );
    assert_eq!(MachineGcodeEmit::__slicer_stage_export_name(), "run");
    assert_eq!(
        MachineGcodeEmit::__slicer_module_schema().stage_export,
        "slicer:postpass-gcode-postprocess/gcode-postprocess@1.0.0#run"
    );
    let exports = MachineGcodeEmit::__slicer_wit_exports();
    assert!(exports.contains(&"slicer:postpass-gcode-postprocess/gcode-postprocess@1.0.0#run"));
}
