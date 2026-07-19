//! Per-region `ConfigView` delivery contracts for the layer region view.

use crate::common::*;
use slicer_ir::{
    ConfigValue, ConfigView, GlobalLayer, RegionKey, RegionMapIR, RegionPlan, ResolvedConfig,
    SemVer,
};
use slicer_runtime::manifest::LoadedModuleBuilder;
use slicer_runtime::{build_wasm_instance_pool, CompiledModuleBuilder, WasmArtifactMetadata};
use std::collections::HashMap;
use std::sync::Arc;

const EMIT_VIEW_WITNESS: &str = "emit_view_witness";
const INFILL_DENSITY: &str = "infill_density";
const EXTRUDER: &str = "extruder";

fn echo_bundle(config: ConfigView) -> TestModuleBundle {
    let component = wasm_cache::compiled_component_at(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../slicer-wasm-host/test-guests/infill-postprocess-echo-guest.component.wasm"
    )));
    let loaded = LoadedModuleBuilder::new(
        "com.test.infill-echo",
        SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        "Layer::InfillPostProcess",
        slicer_schema::WORLD_LAYER,
        std::path::PathBuf::from("/dev/null"),
    )
    .min_host_version(SemVer {
        major: 0,
        minor: 1,
        patch: 0,
    })
    .min_ir_schema(SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    })
    .max_ir_schema(SemVer {
        major: 2,
        minor: 0,
        patch: 0,
    })
    .layer_parallel_safe(true)
    .build();
    let pool = Arc::new(
        build_wasm_instance_pool(
            loaded.id(),
            loaded.stage(),
            loaded.layer_parallel_safe(),
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .unwrap(),
    );
    let module = CompiledModuleBuilder::new("com.test.infill-echo")
        .config_view(Arc::new(config))
        .build();
    TestModuleBundle {
        module,
        pool,
        component: Some(component),
    }
}

fn witness_config(density: f64, extruder: i64) -> ConfigView {
    ConfigView::from_map(HashMap::from([
        (EMIT_VIEW_WITNESS.into(), ConfigValue::Int(1)),
        (INFILL_DENSITY.into(), ConfigValue::Float(density)),
        (EXTRUDER.into(), ConfigValue::Int(extruder)),
    ]))
}

fn region_map(densities: &[(u64, f64)]) -> RegionMapIR {
    let mut map = RegionMapIR::default();
    for &(region_id, density) in densities {
        let mut resolved = ResolvedConfig::default();
        resolved
            .extensions
            .insert(INFILL_DENSITY.into(), ConfigValue::Float(density));
        resolved
            .extensions
            .insert(EXTRUDER.into(), ConfigValue::Int((density * 100.0) as i64));
        resolved
            .extensions
            .insert(EMIT_VIEW_WITNESS.into(), ConfigValue::Int(1));
        let config = map.intern_config(resolved);
        map.entries.insert(
            RegionKey {
                global_layer_index: 0,
                object_id: "obj-0".into(),
                region_id,
                variant_chain: Vec::new(),
            },
            RegionPlan {
                config,
                ..Default::default()
            },
        );
    }
    map
}

fn run_echo(
    fx: &mut dispatch_fixture::DispatchFixture,
    module_config: ConfigView,
) -> Vec<(u64, f64)> {
    let layer = GlobalLayer {
        index: 0,
        z: 0.0,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let bundle = echo_bundle(module_config);
    run_layer_and_commit_with_bundle(
        &fx.dispatcher,
        "Layer::InfillPostProcess",
        &layer,
        &bundle,
        &fx.blackboard,
        &mut fx.arena,
    )
    .expect("per-region config echo dispatch should succeed");

    fx.arena
        .infill()
        .expect("echo output must be committed")
        .regions
        .iter()
        .map(|region| {
            let point = region.solid_infill[0]
                .points
                .first()
                .expect("echo witness header point");
            (region.region_id, f64::from(point.x) / 100.0)
        })
        .collect()
}

#[test]
fn per_region_config_two_densities() {
    let densities = [(0, 0.15), (1, 0.40)];
    let mut fx = dispatch_fixture::for_stage("Layer::InfillPostProcess")
        .with_slice(ir_builders::slice_ir::with_ids(&[("obj-0", 0), ("obj-0", 1)]).build())
        .with_perimeter(ir_builders::perimeter_ir::with_ids(&[("obj-0", 0), ("obj-0", 1)]).build())
        .build();
    fx.blackboard
        .commit_region_map(Arc::new(region_map(&densities)))
        .expect("commit per-region config map");

    let actual = run_echo(&mut fx, witness_config(0.15, 15));
    assert_eq!(actual, vec![(0, 0.15), (1, 0.40)]);
}

#[test]
fn per_region_config_single_region_unchanged() {
    let module_config = witness_config(0.15, 15);
    let module_density = module_config
        .get_float(INFILL_DENSITY)
        .expect("module ConfigView must expose infill_density");
    let mut fx = dispatch_fixture::for_stage("Layer::InfillPostProcess")
        .with_slice(ir_builders::slice_ir::with_ids(&[("obj-0", 0)]).build())
        .with_perimeter(ir_builders::perimeter_ir::with_ids(&[("obj-0", 0)]).build())
        .build();
    fx.blackboard
        .commit_region_map(Arc::new(region_map(&[(0, module_density)])))
        .expect("commit per-region config map");

    let actual = run_echo(&mut fx, module_config);
    assert_eq!(actual, vec![(0, module_density)]);
}
