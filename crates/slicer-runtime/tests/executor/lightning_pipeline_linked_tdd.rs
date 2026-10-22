#![allow(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{
    ConfigValue, ExPolygon, ExtrusionRole, MeshIR, PerimeterIR, PerimeterRegion, Point2, Polygon,
    RegionKey, RegionMapIR, RegionPlan, ResolvedConfig, SemVer, SliceIR, SlicedRegion,
};
use slicer_runtime::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_runtime::manifest::LoadedModuleBuilder;
use slicer_runtime::{Blackboard, CompiledModuleBuilder, LayerArena, WasmRuntimeDispatcher};

use crate::common::{wasm_cache, TestModuleBundle};

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn square() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(10.0, 0.0),
                Point2::from_mm(10.0, 10.0),
                Point2::from_mm(0.0, 10.0),
            ],
        },
        holes: Vec::new(),
    }
}

fn slice_ir() -> SliceIR {
    SliceIR {
        schema_version: semver(4, 1, 0),
        global_layer_index: 1,
        z: 0.2,
        regions: vec![{
            // exhaustive: SlicedRegion lightning fixture specifies the complete region.
            SlicedRegion {
                object_id: "lightning-object".to_string(),
                region_id: 7,
                polygons: vec![square()],
                infill_areas: vec![square()],
                nonplanar_surface: None,
                effective_layer_height: 0.2,
                segment_annotations: HashMap::new(),
                variant_chain: Vec::new(),
                top_shell_index: None,
                bottom_shell_index: None,
                top_solid_fill: Vec::new(),
                bottom_solid_fill: Vec::new(),
                is_bridge: false,
                bridge_areas: Vec::new(),
                bridge_orientation_deg: 0.0,
                sparse_infill_area: vec![square()],
                internal_bridge_areas: Vec::new(),
                internal_bridge_angles_deg: Vec::new(),
                internal_solid_fill: Vec::new(),
                raft_fill: Vec::new(),
            }
        }],
    }
}

fn perimeter_ir() -> PerimeterIR {
    PerimeterIR {
        global_layer_index: 1,
        regions: vec![PerimeterRegion {
            object_id: "lightning-object".to_string(),
            region_id: 7,
            infill_areas: vec![square()],
            ..PerimeterRegion::default()
        }],
        ..PerimeterIR::default()
    }
}

fn lightning_region_map() -> RegionMapIR {
    let mut region_map = RegionMapIR::default();
    let config = region_map.intern_config(ResolvedConfig {
        sparse_fill_holder: "lightning-infill".to_string(),
        line_width: 0.4,
        infill_density: 0.5,
        infill_angle: 45.0,
        layer_height: 0.2,
        // This fixture builds the resolved pool directly, bypassing the live
        // resolver's registry-default seeding for extension-backed linker keys.
        extensions: std::collections::BTreeMap::from([
            (
                "infill_anchor_max".to_string(),
                ConfigValue::FloatOrPercent {
                    value: 20.0,
                    is_percent: false,
                },
            ),
            (
                "prepared_plan_diagnostic".into(),
                ConfigValue::String("none".into()),
            ),
        ]),
        ..ResolvedConfig::default()
    });
    for global_layer_index in [0, 1] {
        region_map.entries.insert(
            RegionKey {
                global_layer_index,
                object_id: "lightning-object".to_string(),
                region_id: 7,
                variant_chain: Vec::new(),
            },
            RegionPlan {
                config,
                ..RegionPlan::default()
            },
        );
    }
    region_map
}

fn module_bundle(module_id: &str, stage: &str, wasm_name: &str) -> TestModuleBundle {
    let wasm_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("modules/core-modules")
        .join(wasm_name)
        .join(format!("{wasm_name}.wasm"));
    assert!(
        wasm_path.exists(),
        "{wasm_name}.wasm not found at {}. Build it with: `cargo xtask build-guests`",
        wasm_path.display()
    );
    let claims = if wasm_name == "lightning-infill" {
        vec!["claim:sparse-fill".to_string()]
    } else {
        vec!["claim:infill-link".to_string()]
    };

    let loaded = LoadedModuleBuilder::new(
        module_id,
        semver(0, 1, 0),
        stage,
        slicer_schema::TIER_LAYER,
        wasm_path.clone(),
    )
    .ir_reads(vec!["SliceIR".to_string(), "InfillIR".to_string()])
    .ir_writes(vec!["InfillIR".to_string()])
    .claims(claims.clone())
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(3, 0, 0))
    .max_ir_schema(semver(5, 0, 0))
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
        .expect("instance pool must build"),
    );
    let config = slicer_ir::ConfigView::from_map(HashMap::from([
        // Required linker reads at infill-linker.toml manifest defaults. The
        // region pool must carry these too; an explicit resolved entry is not
        // a missing-region fallback.
        ("layer_height".to_string(), ConfigValue::Float(0.2)),
        (
            "infill_anchor_max".to_string(),
            ConfigValue::FloatOrPercent {
                value: 20.0,
                is_percent: false,
            },
        ),
        ("infill_density".to_string(), ConfigValue::Float(0.2)),
        ("infill_angle".to_string(), ConfigValue::Float(45.0)),
        ("infill_overlap".to_string(), ConfigValue::Float(0.45)),
        ("infill_speed".to_string(), ConfigValue::Float(50.0)),
        ("line_width".to_string(), ConfigValue::Float(0.4)),
        (
            "prepared_plan_diagnostic".into(),
            ConfigValue::String("none".into()),
        ),
        // Contract-required reads (packet 06): lightning-infill `require_abs_value`s
        // the bound-view widths pair at manifest defaults (0.0), matching
        // `integrated_parity_gyroid_infill_tdd`'s bound-view fixture.
        ("bridge_line_width".to_string(), ConfigValue::Float(0.0)),
        (
            "initial_layer_line_width".to_string(),
            ConfigValue::Float(0.0),
        ),
    ]));
    let module = CompiledModuleBuilder::new(loaded.id().to_string())
        .config_view(Arc::new(config))
        .claims(claims)
        .build();

    TestModuleBundle {
        module,
        pool,
        component: Some(wasm_cache::compiled_component_at(&wasm_path)),
    }
}

#[test]
fn lightning_pipeline_linked() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let layer = slicer_ir::GlobalLayer {
        index: 1,
        z: 0.2,
        ..slicer_ir::GlobalLayer::default()
    };
    let mut blackboard = Blackboard::new(Arc::new(MeshIR::default()), 2);
    blackboard
        .commit_region_map(Arc::new(lightning_region_map()))
        .expect("RegionMapIR must stage");
    let lightning = module_bundle(
        "com.core.lightning-infill",
        "Layer::Infill",
        "lightning-infill",
    );
    // A widening cross-section produces real overhang branches to chain.
    let mut previous = slice_ir();
    previous.global_layer_index = 0;
    previous.z = 0.0;
    for point in &mut previous.regions[0].polygons[0].contour.points {
        point.x = point.x / 5 + Point2::from_mm(4.0, 0.0).x;
        point.y = point.y / 5 + Point2::from_mm(0.0, 4.0).y;
    }
    let (binding, _store, owner) = crate::common::prepare_lightning(
        vec![previous, slice_ir()],
        lightning_region_map(),
        Arc::clone(lightning.module.config_view()),
        true,
    );

    let mut arena = LayerArena::new();
    arena.set_slice(slice_ir()).expect("SliceIR must stage");
    arena
        .set_perimeter(perimeter_ir())
        .expect("PerimeterIR must stage");

    assert_eq!(lightning.module.claims(), &["claim:sparse-fill"]);
    let index = crate::common::selection_for_module(
        "Layer::Infill",
        &layer,
        &lightning.module,
        &blackboard,
        arena.slice(),
        arena.perimeter(),
    )
    .expect("lightning selection");
    let selection = index
        .selection_for(layer.index, lightning.module.module_id())
        .expect("selected lightning region");
    let commit = binding
        .with_native_plan(&owner, |entry| {
            let mut live = lightning.as_live();
            live.native_entry = Some(entry);
            slicer_wasm_host::LayerStageRunner::run_stage(
                &dispatcher,
                &"Layer::Infill".into(),
                &layer,
                &live,
                crate::common::layer_input(&blackboard, &arena, selection),
            )
            .map_err(|error| slicer_sdk::error::ModuleError::fatal(1, format!("{error:?}")))
        })
        .expect("Layer::Infill dispatch must succeed")
        .expect("lightning must emit a commit");
    slicer_runtime::apply_for_test(
        &mut arena,
        commit,
        // exhaustive: pin the lightning dispatch-to-arena commit context, including absent cross-stage inputs; StageApplyContext has no Default.
        &slicer_runtime::StageApplyContext {
            stage_id: "Layer::Infill",
            module_id: lightning.module.module_id(),
            layer_index: layer.index,
            seam_plan: None,
            config_view: Some(lightning.module.config_view().as_ref()),
            committed_slices: None,
        },
    )
    .expect("Layer::Infill commit must succeed");
    assert!(
        arena.infill().is_some(),
        "lightning Layer::Infill must commit raw sparse output"
    );

    let linker = module_bundle(
        "com.core.infill-linker",
        "Layer::InfillPostProcess",
        "infill-linker",
    );
    crate::common::run_layer_and_commit_with_bundle(
        &dispatcher,
        "Layer::InfillPostProcess",
        &layer,
        &linker,
        &blackboard,
        &mut arena,
    )
    .expect("Layer::InfillPostProcess dispatch+commit must succeed");

    let region = arena
        .infill()
        .expect("linked InfillIR must be committed")
        .regions
        .iter()
        .find(|region| region.object_id == "lightning-object" && region.region_id == 7)
        .expect("linker must preserve the lightning region");
    let sparse_paths = &region.sparse_infill;
    assert!(
        !sparse_paths.is_empty(),
        "lightning sparse bucket must be non-empty"
    );
    assert!(
        sparse_paths.iter().all(|path| path.points.len() >= 2),
        "every linked sparse path must have at least two points"
    );
    let mean_points = sparse_paths
        .iter()
        .map(|path| path.points.len() as f32)
        .sum::<f32>()
        / sparse_paths.len() as f32;
    assert!(
        mean_points > 2.0,
        "linker must chain lightning tree segments; mean points per path = {mean_points}"
    );
    assert!(
        sparse_paths
            .iter()
            .all(|path| path.role == ExtrusionRole::SparseInfill),
        "all linked paths must retain the SparseInfill role"
    );
}
