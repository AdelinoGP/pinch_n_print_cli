//! Packet 132 (AC-4) — modifier region split config binding contract.
//!
//! Proves that 131 (`per_region_config`) and 132 (`modifier-region-split`)
//! compose: a modifier's `infill_density` delta is bound to the minted
//! sub-region `RegionKey` (via `stamp_modifier_sub_region_configs`), not
//! stamped object-wide. The packet-131 echo guest then reads `infill_density`
//! per region through the committed `RegionMapIR`:
//!
//! * the base region must report `infill_density = 0.15`
//! * the modifier sub-region must report `infill_density = 0.40`

#![allow(missing_docs)]

use crate::common::*;
use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, GlobalLayer, PerimeterIR, PerimeterRegion, Point2, Polygon,
    RegionKey, RegionMapIR, RegionPlan, ResolvedConfig, SemVer, SliceIR, SlicedRegion,
    CURRENT_PERIMETER_IR_SCHEMA_VERSION, CURRENT_SLICE_IR_SCHEMA_VERSION,
};
use slicer_runtime::blackboard::LayerArena;
use slicer_runtime::manifest::LoadedModuleBuilder;
use slicer_runtime::region_partition::sync_perimeter_infill_areas_into_slice;
use slicer_runtime::{build_wasm_instance_pool, CompiledModuleBuilder, WasmArtifactMetadata};
use std::collections::HashMap;
use std::sync::Arc;

const EMIT_VIEW_WITNESS: &str = "emit_view_witness";
const INFILL_DENSITY: &str = "infill_density";
const EXTRUDER: &str = "extruder";

// ── Packet-131 echo guest plumbing (mirrors per_region_config_tdd) ────────────

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
            let point = region
                .solid_infill
                .first()
                .and_then(|p| p.points.first())
                .expect("echo witness header point");
            (region.region_id, f64::from(point.x) / 100.0)
        })
        .collect()
}

// ── Packet-132 modifier-split geometry (mirrors modifier_region_split_tdd) ─────

fn square(x0: f32, y0: f32, x1: f32, y1: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(x0, y0),
                Point2::from_mm(x1, y0),
                Point2::from_mm(x1, y1),
                Point2::from_mm(x0, y1),
            ],
        },
        holes: vec![],
    }
}

const MODIFIER_FOOTPRINT_REGION_ID: u64 = u64::MAX;

fn base_region(object_id: &str, footprint: ExPolygon) -> SlicedRegion {
    SlicedRegion {
        object_id: object_id.to_string(),
        region_id: 0,
        polygons: vec![footprint.clone()],
        infill_areas: vec![footprint],
        effective_layer_height: 0.5,
        ..Default::default()
    }
}

fn modifier_footprint_region(object_id: &str, footprint: ExPolygon) -> SlicedRegion {
    SlicedRegion {
        object_id: object_id.to_string(),
        region_id: MODIFIER_FOOTPRINT_REGION_ID,
        polygons: vec![footprint.clone()],
        infill_areas: vec![footprint],
        effective_layer_height: 0.5,
        ..Default::default()
    }
}

fn base_perimeter(object_id: &str, wall_inset: ExPolygon) -> PerimeterIR {
    PerimeterIR {
        schema_version: CURRENT_PERIMETER_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        regions: vec![PerimeterRegion {
            object_id: object_id.to_string(),
            region_id: 0,
            walls: vec![],
            infill_areas: vec![wall_inset],
            ..Default::default()
        }],
    }
}

/// Stage a base region + modifier-footprint region, run the partition hook to
/// mint the sub-region, and return the post-hook `SliceIR`.
fn run_split(object_id: &str, base_footprint: ExPolygon, modifier_footprint: ExPolygon) -> SliceIR {
    let mut arena = LayerArena::new();
    let regions = vec![
        base_region(object_id, base_footprint.clone()),
        modifier_footprint_region(object_id, modifier_footprint),
    ];
    let slice = SliceIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        z: 1.0,
        regions,
    };
    arena.set_slice(slice).expect("stage slice must succeed");
    arena
        .set_perimeter(base_perimeter(object_id, base_footprint))
        .expect("stage perimeter must succeed");

    sync_perimeter_infill_areas_into_slice(&mut arena, 0)
        .expect("sync_perimeter_infill_areas_into_slice must succeed");

    arena.slice().expect("slice must be restaged").clone()
}

// ── Per-region config map construction (driven by the 132 binding) ────────────

fn build_region_map(base: &ResolvedConfig, sub: &ResolvedConfig, sub_id: u64) -> RegionMapIR {
    let mut map = RegionMapIR::default();
    for (region_id, cfg) in [(0u64, base), (sub_id, sub)] {
        let density = match cfg.extensions.get(INFILL_DENSITY) {
            Some(ConfigValue::Float(d)) => *d,
            other => panic!("AC-4: region {region_id} must carry infill_density, got {other:?}"),
        };
        let mut resolved = cfg.clone();
        // Proxy `infill_density` into `extruder` so the packet-131 echo guest's
        // view-witness header (x = tool_index) encodes the density for decoding.
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

// ── AC-4 ─────────────────────────────────────────────────────────────────────

#[test]
fn modifier_split_subregion_density() {
    let object_id = "obj-0";

    // Phase 1 — mint the modifier sub-region via the packet-132 split.
    let base = square(0.0, 0.0, 10.0, 10.0);
    let modifier = square(3.0, 3.0, 7.0, 7.0);
    let slice = run_split(object_id, base, modifier);

    let sub = slice
        .regions
        .iter()
        .find(|r| r.region_id != 0 && r.region_id != MODIFIER_FOOTPRINT_REGION_ID)
        .expect("AC-4: modifier split must mint a sub-region");
    let sub_id = sub.region_id;

    // Phase 2 — bind the modifier delta to the sub-region's RegionKey.
    let mut base_config = ResolvedConfig::default();
    base_config
        .extensions
        .insert(INFILL_DENSITY.into(), ConfigValue::Float(0.15));

    let modifier_volume = slicer_ir::ModifierVolume {
        id: "mod-0".into(),
        mesh: slicer_ir::IndexedTriangleSet::default(),
        config_delta: slicer_ir::ConfigDelta {
            fields: HashMap::from([(INFILL_DENSITY.into(), ConfigValue::Float(0.40))]),
        },
        priority: 0,
        applies_to: slicer_ir::ModifierScope::AllFeatures,
    };

    let per_region = slicer_core::algos::region_mapping::stamp_modifier_sub_region_configs(
        base_config.clone(),
        0,
        sub_id,
        &[modifier_volume],
    );

    let base_cfg = per_region
        .get(&0)
        .expect("AC-4: base region config must be present")
        .clone();
    let sub_cfg = per_region
        .get(&sub_id)
        .expect("AC-4: sub-region config must be present")
        .clone();

    let region_map = build_region_map(&base_cfg, &sub_cfg, sub_id);

    // Phase 3 — run the packet-131 echo guest; assert per-region density.
    let mut fx = dispatch_fixture::for_stage("Layer::InfillPostProcess")
        .with_slice(ir_builders::slice_ir::with_ids(&[(object_id, 0), (object_id, sub_id)]).build())
        .with_perimeter(
            ir_builders::perimeter_ir::with_ids(&[(object_id, 0), (object_id, sub_id)]).build(),
        )
        .build();
    fx.blackboard
        .commit_region_map(Arc::new(region_map))
        .expect("commit per-region config map");

    let mut actual = run_echo(&mut fx, witness_config(0.15, 15));
    actual.sort_by_key(|(id, _)| *id);

    assert_eq!(actual.len(), 2, "AC-4: exactly two regions must be echoed");
    assert_eq!(
        actual[0],
        (0, 0.15),
        "AC-4: base region must keep infill_density = 0.15"
    );
    assert_eq!(
        actual[1],
        (sub_id, 0.40),
        "AC-4: sub-region must receive modifier infill_density = 0.40"
    );
}
