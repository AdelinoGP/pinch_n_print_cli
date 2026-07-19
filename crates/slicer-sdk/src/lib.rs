//! Pinch 'n Print SDK foundations.
//!
//! This crate currently provides:
//! - stable re-exports for shared IR types,
//! - host service wrapper functions,
//! - coordinate conversion helpers,
//! - module traits (LayerModule, PrepassModule, FinalizationModule, PostpassModule),
//! - error types (ModuleError),
//! - view types (SliceRegionView, PerimeterRegionView),
//! - output builders (InfillOutputBuilder, PerimeterOutputBuilder, etc.),
//! - prepass types (FacetAnnotation, FacetClass, LayerProposal, etc.),
//! - prepass builders (MeshAnalysisOutput, LayerPlanOutput),
//! - postpass types (GcodeCommand, GcodeOutputCommand),
//! - postpass builders (GcodeOutputBuilder, GcodeMoveCmd).

#![warn(missing_docs)]
#![warn(unused_imports)]
#![warn(unused_must_use)]

pub mod builders;
pub mod config_resolution;
pub mod coords;
pub mod error;
pub mod host;
pub mod layer_collection_builder;
pub mod postpass_builders;
pub mod postpass_types;
pub mod prelude;
pub mod prepass_builders;
pub mod prepass_types;
#[rustfmt::skip]
#[cfg(any(test, feature = "test"))] pub mod test_prelude;
#[rustfmt::skip]
#[cfg(any(test, feature = "test"))] pub mod test_support;
pub mod traits;
pub mod views;

pub use layer_collection_builder::LayerCollectionBuilder;
pub use traits::{EntityMutation, SortKey, SyntheticLayerData};
pub use views::OrderedEntityView;

/// Re-export of the shared IR crate used by host and modules.
pub use slicer_ir as ir;

/// Re-export the authoring macros so modules can write
/// `use slicer_sdk::{slicer_module, module_test};` directly.
pub use slicer_macros::{module_test, slicer_module};

/// Re-export of the shared binding-schema crate. The `#[slicer_module]`
/// macro emits `::slicer_schema::SlicerModuleSchema` values, so any
/// crate that uses the macro transitively needs this name resolvable.
pub use slicer_schema;

/// Append a closing repeat to a Vec by cloning the first element to the end.
///
/// No-op on empty input. Encodes the OrcaSlicer "closed loop" convention used
/// by wall/skirt/brim paths: an N-vertex polygon contour is stored as N+1
/// vertices with `items[N] == items[0]`, so segment iterators (fuzzy-skin,
/// G-code emit) process the closing edge as a first-class segment.
///
/// See `slicer_ir::ExtrusionPath3D::is_closed()` for the contract and
/// `modules/core-modules/{classic,arachne}-perimeters` for wall construction.
pub fn close_loop<T: Clone>(items: &mut Vec<T>) {
    if let Some(first) = items.first().cloned() {
        items.push(first);
    }
}

/// Copy the first element onto the last position in-place.
///
/// Used by wall construction to keep parallel arrays (feature_flags,
/// width_profile.widths) consistent with the closing-repeat invariant on
/// `WallLoop.path.points`: the closing-repeat vertex must carry the same
/// per-vertex paint flags and width as the first vertex, since they refer
/// to the same physical point.
///
/// No-op when `items.len() < 2`.
pub fn mirror_first_to_last<T: Clone>(items: &mut [T]) {
    if items.len() < 2 {
        return;
    }
    let first = items[0].clone();
    if let Some(last) = items.last_mut() {
        *last = first;
    }
}
