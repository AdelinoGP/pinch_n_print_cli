pub mod accumulators;
pub mod in_;
pub mod leaf;
/// Origin tracking for WIT output buckets — [`OriginId`], [`OriginBucket`], and [`MarshalError`].
pub mod origin;
pub mod out;

pub use accumulators::{
    GcodeCommandCollected, GcodeOutputCollected, InfillOutputCollected, PerimeterOutputCollected,
    SlicePostprocessCollected, SupportOutputCollected,
};
pub use in_::{
    object_mesh_to_wit_mesh_object_view, perimeter_region_to_data, project_layer_plan_view,
    project_region_segmentation_view, project_support_geometry_view, sliced_region_to_data,
};
pub use leaf::{
    convert_extrusion_path, convert_extrusion_role, convert_layer_retract_mode,
    convert_paint_value, convert_point, convert_postpass_retract_mode, convert_postpass_role,
    convert_wall_feature_flag, convert_wall_loop, convert_wall_loop_type,
    finalization_role_wit_to_ir, ir_to_wit_expolygon, ir_to_wit_expolygons,
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
    collect_postpass_output, convert_infill_output, convert_perimeter_output,
    convert_support_output, merge_slice_postprocess_into,
};
