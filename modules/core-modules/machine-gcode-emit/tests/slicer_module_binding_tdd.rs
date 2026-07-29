//! Regression guard: the `#[slicer_module]`-emitted binding surface for
//! machine-gcode-emit matches its manifest's declared postpass world/stage.

#![allow(missing_docs)]

use machine_gcode_emit::MachineGcodeEmit;

#[test]
fn binding_surface_matches_gcode_postprocess_stage() {
    assert_eq!(
        MachineGcodeEmit::__slicer_world_id(),
        slicer_schema::WORLD_POSTPASS
    );
    assert_eq!(MachineGcodeEmit::__slicer_trait_name(), "PostpassModule");
    assert_eq!(
        MachineGcodeEmit::__slicer_stage_name(),
        "PostPass::GCodePostProcess"
    );
    // Packet 163: per-stage package migration. The func is now `run` for
    // every migrated stage; `qualified_export_for_stage_id` is the only
    // lookup that fully identifies the contract.
    assert_eq!(MachineGcodeEmit::__slicer_stage_export_name(), "run");
    let exports = MachineGcodeEmit::__slicer_wit_exports();
    assert!(exports.contains(&"run"));
}
