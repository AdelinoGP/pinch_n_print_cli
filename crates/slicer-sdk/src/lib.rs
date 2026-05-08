//! ModularSlicer SDK foundations.
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
pub mod coords;
pub mod error;
pub mod host;
pub mod layer_collection_builder;
pub mod postpass_builders;
pub mod postpass_types;
pub mod prelude;
pub mod prepass_builders;
pub mod prepass_types;
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
