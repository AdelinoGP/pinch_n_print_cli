pub mod accumulators;
pub mod in_;
pub mod leaf;
#[cfg(not(target_arch = "wasm32"))]
pub mod native;
/// Origin tracking for WIT output buckets — [`OriginId`], [`OriginBucket`], and [`MarshalError`].
pub mod origin;
pub mod out;

pub use accumulators::{
    AnchoredEventsCollected, GcodeCommandCollected, GcodeOutputCollected, InfillOutputCollected,
    PerimeterOutputCollected, SlicePostprocessCollected, SupportOutputCollected,
};
pub use in_::{
    harvest_seam_plan_ir_from, object_mesh_to_wit_mesh_object_view, perimeter_region_to_data,
    project_layer_plan_view, project_region_segmentation_view, project_support_geometry_view,
    sliced_region_to_data,
};
pub use leaf::{
    convert_extrusion_path, convert_extrusion_role, convert_layer_retract_mode,
    convert_paint_value, convert_point, convert_postpass_retract_mode, convert_wall_feature_flag,
    convert_wall_loop, convert_wall_loop_type, ir_to_wit_expolygon, ir_to_wit_expolygons,
    ir_to_wit_extrusion_path, ir_to_wit_extrusion_role, ir_to_wit_paint_layer_view,
    ir_to_wit_paint_semantic, ir_to_wit_paint_stroke_view, ir_to_wit_paint_value,
    ir_to_wit_paint_value_view, ir_to_wit_wall_feature_flag, ir_to_wit_wall_loop,
    ir_to_wit_wall_loop_type, paint_semantic_to_string, validate_finite, wit_to_ir_expolygon,
    wit_to_ir_expolygons,
};
pub use origin::{MarshalError, OriginBucket, OriginId};
// harvest_*_from functions are pub(crate) in in_.rs and accessed directly by dispatch.rs
// via `use crate::marshal::in_::harvest_*_from` — not re-exported at the marshal:: level.
pub use out::{
    authored_coloring_granted, collect_postpass_output, convert_anchored_events,
    convert_infill_output, convert_perimeter_output, convert_support_output,
    convert_support_output_with_plan, infill_ir_to_prior_regions, merge_slice_postprocess_into,
    validate_anchored_entity_geometry, AuthoredColoringContext, AUTHORED_COLORING_CLAIM,
};

/// Return the effective height for a global layer across all participating objects.
pub fn canonical_effective_layer_height(plan: &slicer_ir::LayerPlanIR, global_index: u32) -> f32 {
    plan.object_participation
        .values()
        .filter_map(|refs| {
            refs.iter()
                .find(|reference| reference.global_layer_index == global_index)
                .map(|reference| reference.effective_layer_height)
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(0.2)
}

/// Convert a native SDK support builder through the same host-side join used
/// for renderer output, preserving origin-based plan identity.
#[cfg(not(target_arch = "wasm32"))]
pub fn convert_native_support_output_with_plan(
    builder: &slicer_sdk::builders::SupportOutputBuilder,
    layer_index: u32,
    plan: &slicer_ir::SupportPlanIR,
) -> Result<slicer_ir::SupportIR, String> {
    let origin = |value: &Option<slicer_sdk::builders::RegionOrigin>| {
        value.as_ref().map(|value| OriginId {
            object_id: value.object_id.clone(),
            region_id: value.region_id,
        })
    };
    let collected = SupportOutputCollected {
        support_paths: builder
            .support_paths()
            .iter()
            .map(ir_to_wit_extrusion_path)
            .collect(),
        interface_paths: builder
            .interface_paths()
            .iter()
            .map(|(path, top)| (ir_to_wit_extrusion_path(path), *top))
            .collect(),
        raft_paths: builder
            .raft_paths()
            .iter()
            .map(ir_to_wit_extrusion_path)
            .collect(),
        support_path_origins: builder.support_path_origins().iter().map(origin).collect(),
        interface_path_origins: builder
            .interface_path_origins()
            .iter()
            .map(origin)
            .collect(),
        raft_path_origins: builder.raft_path_origins().iter().map(origin).collect(),
    };
    convert_support_output_with_plan(&collected, layer_index, Some(plan))
}
