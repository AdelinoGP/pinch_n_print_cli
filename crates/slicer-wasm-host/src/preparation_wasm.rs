//! Projected ordinary-world imports with preparation-only additions.

use super::preparation_bindings::perimeters::slicer::layer_preparation::{
    plan_access, preparation_types,
};
use super::PreparedHostState;
use crate::host::HostExecutionContext;
use wasmtime::component::{HasSelf, Linker};

pub(super) struct FrozenWasmArtifact {
    pub(super) engine: wasmtime::Engine,
    pub(super) component: wasmtime::component::Component,
    pub(super) stage_id: slicer_ir::StageId,
    pub(super) indices: FrozenIndices,
}

macro_rules! indices {
    ($( $variant:ident, $module:ident, $world:ident, $plain:ident, $ordinary:ident, $name:literal; )*) => {
        pub(super) enum FrozenIndices {
            $( $variant(super::preparation_bindings::$module::$world),
               $plain, )*
        }
        pub(super) fn validate_indices(component: &wasmtime::component::Component, world: &str, prepared: bool) -> wasmtime::Result<FrozenIndices> {
            let stage = slicer_schema::STAGES.iter().find(|stage| stage.wit_world == world).ok_or_else(|| wasmtime::Error::msg("unknown ordinary world"))?;
            let pre = linker(component.engine(), stage.stage_id)?.instantiate_pre(component)?;
            if prepared {
                validate_prepare_signature(&pre)?;
            }
            match (world, prepared) {
                $( ($name, true) => Ok(FrozenIndices::$variant(super::preparation_bindings::$module::$world::new(&pre)?)),
                   ($name, false) => {
                       crate::host::$ordinary::$plain::new(&pre)?;
                       Ok(FrozenIndices::$plain)
                   }, )*
                _ => wasmtime::bail!("unsupported preparation world"),
            }
        }
        impl FrozenIndices {
            pub(super) fn prepare(&self, store: &mut wasmtime::Store<PreparedHostState>, instance: &wasmtime::component::Instance, input: wasmtime::component::Resource<super::PreparationInputData>, output: wasmtime::component::Resource<super::PreparationOutputData>) -> wasmtime::Result<Result<(), crate::host::layer_perimeters::slicer::common::module_errors::ModuleError>> {
                match self {
                    $( Self::$variant(indices) => indices.load(&mut *store, instance)?.slicer_layer_preparation_prepare().call_prepare_print(store, input, output), )*
                    _ => wasmtime::bail!("artifact has no preparation export"),
                }
            }
        }
    };
}

/// Interface indices only resolve names; check the actual function before any
/// store, owner, instance, or preparation invocation exists.
fn validate_prepare_signature(
    pre: &wasmtime::component::InstancePre<PreparedHostState>,
) -> wasmtime::Result<()> {
    use wasmtime::component::{types::ComponentItem, Resource};

    let component = pre.component();
    let (_, interface) = component
        .get_export(None, "slicer:layer-preparation/prepare@1.0.0")
        .ok_or_else(|| wasmtime::Error::msg("missing preparation interface"))?;
    let (ComponentItem::ComponentFunc(function), _) = component
        .get_export(Some(&interface), "prepare-print")
        .ok_or_else(|| wasmtime::Error::msg("missing prepare-print"))?
    else {
        wasmtime::bail!("prepare-print is not a function");
    };
    function.typecheck::<(
        Resource<super::PreparationInputData>,
        Resource<super::PreparationOutputData>,
    ), (Result<(), crate::host::layer_perimeters::slicer::common::module_errors::ModuleError>,)>(
        &pre.instance_type(),
    )
}

indices! {
    Perimeters, perimeters, PreparedPerimetersModuleIndices, PerimetersModuleIndices, layer_perimeters, "perimeters-module";
    SlicePostprocess, slice_postprocess, PreparedSlicePostprocessModuleIndices, SlicePostprocessModuleIndices, layer_slice_postprocess, "slice-postprocess-module";
    PerimetersPostprocess, perimeters_postprocess, PreparedPerimetersPostprocessModuleIndices, PerimetersPostprocessModuleIndices, layer_perimeters_postprocess, "perimeters-postprocess-module";
    Infill, infill, PreparedInfillModuleIndices, InfillModuleIndices, layer_infill, "infill-module";
    InfillPostprocess, infill_postprocess, PreparedInfillPostprocessModuleIndices, InfillPostprocessModuleIndices, layer_infill_postprocess, "infill-postprocess-module";
    Support, support, PreparedSupportModuleIndices, SupportModuleIndices, layer_support, "support-module";
    SupportPostprocess, support_postprocess, PreparedSupportPostprocessModuleIndices, SupportPostprocessModuleIndices, layer_support_postprocess, "support-postprocess-module";
    AnchoredEvents, anchored_events, PreparedAnchoredEventsModuleIndices, AnchoredEventsModuleIndices, layer_anchored_events, "anchored-events-module";
    PathOptimization, path_optimization, PreparedPathOptimizationModuleIndices, PathOptimizationModuleIndices, layer_path_optimization, "path-optimization-module";
}

pub(super) fn linker(
    engine: &wasmtime::Engine,
    stage_id: &str,
) -> wasmtime::Result<Linker<PreparedHostState>> {
    let mut linker = Linker::new(engine);
    macro_rules! ordinary {
        ($module:ident, $world:ident) => {
            crate::host::$module::$world::add_to_linker::<_, HasSelf<HostExecutionContext>>(
                &mut linker,
                |state: &mut PreparedHostState| state.ordinary_mut(),
            )?
        };
    }
    let stage = slicer_schema::STAGES
        .iter()
        .find(|stage| stage.stage_id == stage_id && stage.tier_id == slicer_schema::TIER_LAYER)
        .ok_or_else(|| {
            wasmtime::Error::msg(format!("unsupported preparation stage: {stage_id}"))
        })?;
    match stage.wit_world {
        "slice-postprocess-module" => ordinary!(layer_slice_postprocess, SlicePostprocessModule),
        "perimeters-module" => ordinary!(layer_perimeters, PerimetersModule),
        "perimeters-postprocess-module" => {
            ordinary!(layer_perimeters_postprocess, PerimetersPostprocessModule)
        }
        "infill-module" => ordinary!(layer_infill, InfillModule),
        "infill-postprocess-module" => ordinary!(layer_infill_postprocess, InfillPostprocessModule),
        "support-module" => ordinary!(layer_support, SupportModule),
        "support-postprocess-module" => {
            ordinary!(layer_support_postprocess, SupportPostprocessModule)
        }
        "anchored-events-module" => ordinary!(layer_anchored_events, AnchoredEventsModule),
        "path-optimization-module" => ordinary!(layer_path_optimization, PathOptimizationModule),
        _ => wasmtime::bail!("unsupported preparation stage: {stage_id}"),
    }
    preparation_types::add_to_linker::<_, HasSelf<PreparedHostState>>(&mut linker, |state| state)?;
    plan_access::add_to_linker::<_, HasSelf<PreparedHostState>>(&mut linker, |state| state)?;
    Ok(linker)
}
