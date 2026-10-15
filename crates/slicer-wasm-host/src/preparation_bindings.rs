//! Composed Layer bindings sharing ordinary and preparation resource identities.

/// Preparation resource identity root and composed perimeters binding.
pub mod perimeters {
    wasmtime::component::bindgen!({
        path: "../slicer-schema/wit",
        world: "slicer:layer-preparation/prepared-perimeters-module",
        imports: { default: trappable, },
        with: {
            "slicer:types/geometry": crate::host::layer_perimeters::slicer::types::geometry,
            "slicer:config/config-types": crate::host::layer_perimeters::slicer::config::config_types,
            "slicer:common/host-services": crate::host::layer_perimeters::slicer::common::host_services,
            "slicer:common/profiling": crate::host::layer_perimeters::slicer::common::profiling,
            "slicer:common/module-errors": crate::host::layer_perimeters::slicer::common::module_errors,
            "slicer:ir-handles/ir-handles@1.0.0": crate::host::layer_perimeters::slicer::ir_handles::ir_handles,
            "slicer:layer-preparation/preparation-types@1.0.0.print-preparation-input": super::super::preparation_host::PreparationInputData,
            "slicer:layer-preparation/preparation-types@1.0.0.prepared-plan-output": super::super::preparation_host::PreparationOutputData,
            "slicer:layer-preparation/preparation-types@1.0.0.prepared-plan-view": super::super::preparation_host::PreparedPlanViewData,
            "slicer:layer-preparation/preparation-types@1.0.0.prepared-piece": super::super::preparation_host::PreparedPieceData,
        },
    });
}

/// Composed slice postprocessing binding.
pub mod slice_postprocess {
    wasmtime::component::bindgen!({
        path: "../slicer-schema/wit",
        world: "slicer:layer-preparation/prepared-slice-postprocess-module",
        imports: { default: trappable, },
        with: {
            "slicer:types/geometry": crate::host::layer_perimeters::slicer::types::geometry,
            "slicer:config/config-types": crate::host::layer_perimeters::slicer::config::config_types,
            "slicer:common/host-services": crate::host::layer_perimeters::slicer::common::host_services,
            "slicer:common/profiling": crate::host::layer_perimeters::slicer::common::profiling,
            "slicer:common/module-errors": crate::host::layer_perimeters::slicer::common::module_errors,
            "slicer:ir-handles/ir-handles@1.0.0": crate::host::layer_perimeters::slicer::ir_handles::ir_handles,
            "slicer:layer-preparation/preparation-types@1.0.0": super::perimeters::slicer::layer_preparation::preparation_types,
            "slicer:layer-preparation/plan-access@1.0.0": super::perimeters::slicer::layer_preparation::plan_access,
        },
    });
}

/// Composed perimeter postprocessing binding.
pub mod perimeters_postprocess {
    wasmtime::component::bindgen!({
        path: "../slicer-schema/wit",
        world: "slicer:layer-preparation/prepared-perimeters-postprocess-module",
        imports: { default: trappable, },
        with: {
            "slicer:types/geometry": crate::host::layer_perimeters::slicer::types::geometry,
            "slicer:config/config-types": crate::host::layer_perimeters::slicer::config::config_types,
            "slicer:common/host-services": crate::host::layer_perimeters::slicer::common::host_services,
            "slicer:common/profiling": crate::host::layer_perimeters::slicer::common::profiling,
            "slicer:common/module-errors": crate::host::layer_perimeters::slicer::common::module_errors,
            "slicer:ir-handles/ir-handles@1.0.0": crate::host::layer_perimeters::slicer::ir_handles::ir_handles,
            "slicer:layer-preparation/preparation-types@1.0.0": super::perimeters::slicer::layer_preparation::preparation_types,
            "slicer:layer-preparation/plan-access@1.0.0": super::perimeters::slicer::layer_preparation::plan_access,
        },
    });
}

/// Composed infill binding.
pub mod infill {
    wasmtime::component::bindgen!({
        path: "../slicer-schema/wit",
        world: "slicer:layer-preparation/prepared-infill-module",
        imports: { default: trappable, },
        with: {
            "slicer:types/geometry": crate::host::layer_perimeters::slicer::types::geometry,
            "slicer:config/config-types": crate::host::layer_perimeters::slicer::config::config_types,
            "slicer:common/host-services": crate::host::layer_perimeters::slicer::common::host_services,
            "slicer:common/profiling": crate::host::layer_perimeters::slicer::common::profiling,
            "slicer:common/module-errors": crate::host::layer_perimeters::slicer::common::module_errors,
            "slicer:ir-handles/ir-handles@1.0.0": crate::host::layer_perimeters::slicer::ir_handles::ir_handles,
            "slicer:support-queries/support-queries@1.0.0": crate::host::layer_infill::slicer::support_queries::support_queries,
            "slicer:lightning-queries/lightning-queries@1.0.0": crate::host::layer_infill::slicer::lightning_queries::lightning_queries,
            "slicer:layer-preparation/preparation-types@1.0.0": super::perimeters::slicer::layer_preparation::preparation_types,
            "slicer:layer-preparation/plan-access@1.0.0": super::perimeters::slicer::layer_preparation::plan_access,
        },
    });
}

/// Composed infill postprocessing binding.
pub mod infill_postprocess {
    wasmtime::component::bindgen!({
        path: "../slicer-schema/wit",
        world: "slicer:layer-preparation/prepared-infill-postprocess-module",
        imports: { default: trappable, },
        with: {
            "slicer:types/geometry": crate::host::layer_perimeters::slicer::types::geometry,
            "slicer:config/config-types": crate::host::layer_perimeters::slicer::config::config_types,
            "slicer:common/host-services": crate::host::layer_perimeters::slicer::common::host_services,
            "slicer:common/profiling": crate::host::layer_perimeters::slicer::common::profiling,
            "slicer:common/module-errors": crate::host::layer_perimeters::slicer::common::module_errors,
            "slicer:ir-handles/ir-handles@1.0.0": crate::host::layer_perimeters::slicer::ir_handles::ir_handles,
            "slicer:layer-preparation/preparation-types@1.0.0": super::perimeters::slicer::layer_preparation::preparation_types,
            "slicer:layer-preparation/plan-access@1.0.0": super::perimeters::slicer::layer_preparation::plan_access,
        },
    });
}

/// Composed support binding.
pub mod support {
    wasmtime::component::bindgen!({
        path: "../slicer-schema/wit",
        world: "slicer:layer-preparation/prepared-support-module",
        imports: { default: trappable, },
        with: {
            "slicer:types/geometry": crate::host::layer_perimeters::slicer::types::geometry,
            "slicer:config/config-types": crate::host::layer_perimeters::slicer::config::config_types,
            "slicer:common/host-services": crate::host::layer_perimeters::slicer::common::host_services,
            "slicer:common/profiling": crate::host::layer_perimeters::slicer::common::profiling,
            "slicer:common/module-errors": crate::host::layer_perimeters::slicer::common::module_errors,
            "slicer:ir-handles/ir-handles@1.0.0": crate::host::layer_perimeters::slicer::ir_handles::ir_handles,
            "slicer:layer-preparation/preparation-types@1.0.0": super::perimeters::slicer::layer_preparation::preparation_types,
            "slicer:layer-preparation/plan-access@1.0.0": super::perimeters::slicer::layer_preparation::plan_access,
        },
    });
}

/// Composed support postprocessing binding.
pub mod support_postprocess {
    wasmtime::component::bindgen!({
        path: "../slicer-schema/wit",
        world: "slicer:layer-preparation/prepared-support-postprocess-module",
        imports: { default: trappable, },
        with: {
            "slicer:types/geometry": crate::host::layer_perimeters::slicer::types::geometry,
            "slicer:config/config-types": crate::host::layer_perimeters::slicer::config::config_types,
            "slicer:common/host-services": crate::host::layer_perimeters::slicer::common::host_services,
            "slicer:common/profiling": crate::host::layer_perimeters::slicer::common::profiling,
            "slicer:common/module-errors": crate::host::layer_perimeters::slicer::common::module_errors,
            "slicer:ir-handles/ir-handles@1.0.0": crate::host::layer_perimeters::slicer::ir_handles::ir_handles,
            "slicer:layer-preparation/preparation-types@1.0.0": super::perimeters::slicer::layer_preparation::preparation_types,
            "slicer:layer-preparation/plan-access@1.0.0": super::perimeters::slicer::layer_preparation::plan_access,
        },
    });
}

/// Composed anchored events binding.
pub mod anchored_events {
    wasmtime::component::bindgen!({
        path: "../slicer-schema/wit",
        world: "slicer:layer-preparation/prepared-anchored-events-module",
        imports: { default: trappable, },
        with: {
            "slicer:types/geometry": crate::host::layer_perimeters::slicer::types::geometry,
            "slicer:config/config-types": crate::host::layer_perimeters::slicer::config::config_types,
            "slicer:common/host-services": crate::host::layer_perimeters::slicer::common::host_services,
            "slicer:common/profiling": crate::host::layer_perimeters::slicer::common::profiling,
            "slicer:common/module-errors": crate::host::layer_perimeters::slicer::common::module_errors,
            "slicer:ir-handles/ir-handles@1.0.0": crate::host::layer_perimeters::slicer::ir_handles::ir_handles,
            "slicer:layer-preparation/preparation-types@1.0.0": super::perimeters::slicer::layer_preparation::preparation_types,
            "slicer:layer-preparation/plan-access@1.0.0": super::perimeters::slicer::layer_preparation::plan_access,
        },
    });
}

/// Composed path optimization binding.
pub mod path_optimization {
    wasmtime::component::bindgen!({
        path: "../slicer-schema/wit",
        world: "slicer:layer-preparation/prepared-path-optimization-module",
        imports: { default: trappable, },
        with: {
            "slicer:types/geometry": crate::host::layer_perimeters::slicer::types::geometry,
            "slicer:config/config-types": crate::host::layer_perimeters::slicer::config::config_types,
            "slicer:common/host-services": crate::host::layer_perimeters::slicer::common::host_services,
            "slicer:common/profiling": crate::host::layer_perimeters::slicer::common::profiling,
            "slicer:common/module-errors": crate::host::layer_perimeters::slicer::common::module_errors,
            "slicer:ir-handles/ir-handles@1.0.0": crate::host::layer_perimeters::slicer::ir_handles::ir_handles,
            "slicer:layer-preparation/preparation-types@1.0.0": super::perimeters::slicer::layer_preparation::preparation_types,
            "slicer:layer-preparation/plan-access@1.0.0": super::perimeters::slicer::layer_preparation::plan_access,
        },
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_layer_stage_with_a_package_has_a_prepared_binding() {
        let expected: Vec<_> = slicer_schema::STAGES
            .iter()
            .filter(|stage| {
                stage.tier_id == slicer_schema::TIER_LAYER && !stage.wit_package.is_empty()
            })
            .collect();
        assert!(
            !expected.is_empty(),
            "schema contains no packaged Layer stages"
        );
        for stage in &expected {
            // Each arm names a generated concrete type; removing its binding breaks compilation.
            let compiled = match stage.wit_world {
                "slice-postprocess-module" => {
                    std::any::type_name::<slice_postprocess::PreparedSlicePostprocessModule>()
                }
                "perimeters-module" => {
                    std::any::type_name::<perimeters::PreparedPerimetersModule>()
                }
                "perimeters-postprocess-module" => std::any::type_name::<
                    perimeters_postprocess::PreparedPerimetersPostprocessModule,
                >(),
                "infill-module" => std::any::type_name::<infill::PreparedInfillModule>(),
                "infill-postprocess-module" => {
                    std::any::type_name::<infill_postprocess::PreparedInfillPostprocessModule>()
                }
                "support-module" => std::any::type_name::<support::PreparedSupportModule>(),
                "support-postprocess-module" => {
                    std::any::type_name::<support_postprocess::PreparedSupportPostprocessModule>()
                }
                "anchored-events-module" => {
                    std::any::type_name::<anchored_events::PreparedAnchoredEventsModule>()
                }
                "path-optimization-module" => {
                    std::any::type_name::<path_optimization::PreparedPathOptimizationModule>()
                }
                _ => panic!(
                    "no compiled prepared binding for {} ({})",
                    stage.stage_id, stage.wit_world
                ),
            };
            let expected_type = format!(
                "Prepared{}",
                stage
                    .wit_world
                    .split('-')
                    .map(|part| {
                        let mut chars = part.chars();
                        chars
                            .next()
                            .expect("nonempty world segment")
                            .to_uppercase()
                            .collect::<String>()
                            + chars.as_str()
                    })
                    .collect::<String>()
            );
            assert_eq!(
                compiled.rsplit("::").next(),
                Some(expected_type.as_str()),
                "compiled prepared binding does not match {}",
                stage.stage_id
            );
        }
        println!("DERIVED_LAYER_STAGE_COUNT={}", expected.len());
    }
}
