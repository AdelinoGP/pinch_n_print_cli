#![allow(missing_docs)]

use std::sync::Arc;

use crate::common::{
    integrated_parity_harness::{run_integrated_parity, IntegratedParitySpec},
    parity_invariants::ParityTolerance,
};
use lightning_infill::LightningInfill;
use slicer_ir::{
    ConfigView, ExPolygon, GlobalLayer, Point2, Polygon, RegionKey, RegionMapIR, RegionPlan,
    ResolvedConfig, SemVer, SliceIR, SlicedRegion, StageId,
};
use slicer_runtime::{Blackboard, LayerArena};

fn non_empty_slice() -> SliceIR {
    let polygon = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(12.0, 0.0),
                Point2::from_mm(12.0, 12.0),
                Point2::from_mm(0.0, 12.0),
            ],
        },
        holes: Vec::new(),
    };
    SliceIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 5,
        z: 1.0,
        regions: vec![SlicedRegion {
            object_id: "parity-object".to_string(),
            region_id: 0,
            polygons: vec![polygon.clone()],
            infill_areas: vec![polygon.clone()],
            sparse_infill_area: vec![polygon],
            ..Default::default()
        }],
    }
}

#[test]
fn integrated_parity_lightning_infill() {
    let mut bb = Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 1);
    let mut region_map = RegionMapIR::default();
    let resolved = ResolvedConfig {
        sparse_fill_holder: "com.core.lightning-infill".to_string(),
        line_width: 0.45,
        infill_density: 0.2,
        layer_height: 0.2,
        extensions: std::collections::BTreeMap::from([(
            "prepared_plan_diagnostic".into(),
            slicer_ir::ConfigValue::String("none".into()),
        )]),
        ..Default::default()
    };
    let config = region_map.intern_config(resolved);
    region_map.entries.insert(
        RegionKey {
            global_layer_index: 5,
            object_id: "parity-object".to_string(),
            region_id: 0,
            variant_chain: Vec::new(),
        },
        RegionPlan {
            config,
            ..Default::default()
        },
    );
    bb.commit_region_map(Arc::new(region_map.clone()))
        .expect("commit region map");
    let mut wasm_arena = LayerArena::new();
    let mut native_arena = LayerArena::new();
    wasm_arena
        .set_slice(non_empty_slice())
        .expect("set wasm slice");
    native_arena
        .set_slice(non_empty_slice())
        .expect("set native slice");
    let layer = GlobalLayer {
        index: 5,
        z: 1.0,
        ..Default::default()
    };
    let stage: StageId = "Layer::Infill".to_string();
    let spec = IntegratedParitySpec {
        module_id: "com.core.lightning-infill".into(),
        wasm_path: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../modules/core-modules/lightning-infill/lightning-infill.wasm"),
        stage: stage.clone(),
        version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        min_ir_schema: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        max_ir_schema: SemVer {
            major: 2,
            minor: 0,
            patch: 0,
        },
        tier: String::new(),
        claims: vec!["claim:sparse-fill".into()],
        // Bound-view shape (packet 06 5c-prime): `bridge_line_width` /
        // `initial_layer_line_width` are contract-required reads
        // (`require_abs_value` over the fixed 0.4 nozzle base); a bound
        // view holds them at their resolved defaults (lightning_infill_tdd.rs
        // fixtures) and carries the host-expanded `line_width`.
        // `infill_density` / `infill_speed` are declared required reads
        // since the guest's config-literal fallbacks migrated to
        // `require_*`; seed the manifest defaults (lightning-infill.toml).
        config: Arc::new(ConfigView::from_map(std::collections::HashMap::from([
            ("layer_height".into(), slicer_ir::ConfigValue::Float(0.2)),
            ("infill_angle".into(), slicer_ir::ConfigValue::Float(45.0)),
            (
                "prepared_plan_diagnostic".into(),
                slicer_ir::ConfigValue::String("none".into()),
            ),
            (
                "infill_density".to_string(),
                slicer_ir::ConfigValue::Float(0.2),
            ),
            (
                "infill_speed".to_string(),
                slicer_ir::ConfigValue::Float(60.0),
            ),
            (
                "bridge_line_width".to_string(),
                slicer_ir::ConfigValue::Float(0.4),
            ),
            (
                "initial_layer_line_width".to_string(),
                slicer_ir::ConfigValue::Float(0.4),
            ),
            (
                "line_width".to_string(),
                slicer_ir::ConfigValue::Float(0.45),
            ),
        ]))),
        native_entry: LightningInfill::__slicer_native_entry(),
    };
    let selection_module = slicer_runtime::CompiledModuleBuilder::new(spec.module_id.clone())
        .claims(spec.claims.clone())
        .config_view(Arc::clone(&spec.config))
        .build();
    let selection_index = crate::common::selection_for_module(
        &stage,
        &layer,
        &selection_module,
        &bb,
        wasm_arena.slice(),
        None,
    )
    .expect("build fixture selection index");
    let selection = selection_index
        .selection_for(layer.index, selection_module.module_id())
        .expect("fixture layer selection");
    let mut wasm_input = crate::common::layer_input(&bb, &wasm_arena, selection);
    let mut native_input = crate::common::layer_input(&bb, &native_arena, selection);
    wasm_input.paint_regions = Some(());
    native_input.paint_regions = Some(());
    let mut previous = non_empty_slice();
    previous.global_layer_index = 4;
    previous.z = 0.8;
    for point in &mut previous.regions[0].polygons[0].contour.points {
        point.x = point.x * 10 / 12;
        point.y = point.y * 10 / 12;
    }
    let slices = vec![previous, non_empty_slice()];
    let previous_key = RegionKey {
        global_layer_index: 4,
        object_id: "parity-object".into(),
        region_id: 0,
        variant_chain: Vec::new(),
    };
    region_map.entries.insert(
        previous_key,
        RegionPlan {
            config,
            ..Default::default()
        },
    );
    let (native_binding, _native_store, native_owner) = crate::common::prepare_lightning(
        slices.clone(),
        region_map.clone(),
        Arc::clone(&spec.config),
        true,
    );
    let (wasm_binding, _wasm_store, wasm_owner) =
        crate::common::prepare_lightning(slices, region_map, Arc::clone(&spec.config), false);
    let (native, wasm) = run_integrated_parity(
        spec,
        |dispatcher, native_live, wasm_live, _native_module, _wasm_module| {
            let wasm = dispatcher
                .run_prepared_stage(
                    &stage,
                    &layer,
                    wasm_live,
                    wasm_input,
                    &wasm_binding,
                    &wasm_owner,
                )
                .expect("wasm dispatch")
                .expect("wasm commit");
            let native = dispatcher
                .run_prepared_stage(
                    &stage,
                    &layer,
                    native_live,
                    native_input,
                    &native_binding,
                    &native_owner,
                )
                .expect("native dispatch")
                .expect("native commit");
            (native, wasm)
        },
    );
    for commit in [&native, &wasm] {
        let slicer_ir::LayerStageCommit::Infill(ir) = commit else {
            panic!("expected infill");
        };
        assert_eq!(ir.regions.len(), 1);
        let region = &ir.regions[0];
        assert_eq!(
            (
                region.object_id.as_str(),
                region.region_id,
                region.variant_chain.as_slice()
            ),
            ("parity-object", 0, &[][..])
        );
        assert!(
            !region.sparse_infill.is_empty(),
            "real preparation must emit geometry"
        );
    }
    crate::common::parity_invariants::assert_parity_structural(
        &native,
        &wasm,
        ParityTolerance::default(),
        0.4,
    )
    .expect("lightning native/wasm parity");
}
