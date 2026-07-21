/// BuiltinProducer for G-code emission.
pub mod gcode_emit_producer;
/// BuiltinProducer for lightning tree generation.
pub mod lightning_tree_producer;
/// BuiltinProducer for mesh analysis.
pub mod mesh_analysis_producer;
/// BuiltinProducer for overhang quartile-band annotation.
pub mod overhang_annotation_producer;
/// BuiltinProducer for pre-pass slicing.
pub mod prepass_slice_producer;
/// BuiltinProducer for region mapping.
pub mod region_mapping_producer;
/// BuiltinProducer for support geometry.
pub mod support_geometry_producer;

pub use overhang_annotation_producer::{
    commit_overhang_annotation_builtin, OverhangAnnotationBuiltinError,
};
pub use region_mapping_producer::{
    commit_region_mapping_builtin, RegionMappingBuiltinError, REGION_MAPPING_PRODUCER,
};
