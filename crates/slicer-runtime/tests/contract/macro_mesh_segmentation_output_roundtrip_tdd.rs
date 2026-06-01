//! TDD harness for MeshSegmentation macro-path output round-trip (Packet-43-rev1).
//!
//! Loads sdk-prepass-meshseg-guest.component.wasm (authored in Step 4).
//! fixture_case="marks_basic" â†’ guest marks triangle index 12 on "obj-a" â†’ AC-8 GREEN.
//!
//! Verification: cargo test -p slicer-runtime --test macro_mesh_segmentation_output_roundtrip_tdd

#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    BoundingBox3, ConfigValue, ConfigView, IndexedTriangleSet, MeshIR, ObjectConfig, ObjectMesh,
    Point3, SemVer, Transform3d,
};
use slicer_runtime::dispatch::WasmRuntimeDispatcher;
use slicer_runtime::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_runtime::manifest::{LoadedModule, LoadedModuleBuilder};
use slicer_runtime::{Blackboard, CompiledModule, CompiledModuleBuilder, PrepassStageRunner};

use crate::common::wasm_cache;

// â”€â”€ Path to the sdk-prepass-guest component â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

const SDK_PREPASS_GUEST_NAME: &str = "sdk-prepass-meshseg-guest";

fn sdk_prepass_guest_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test-guests")
        .join(format!("{SDK_PREPASS_GUEST_NAME}.component.wasm"))
}

// â”€â”€ Harness helpers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn identity_transform() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    }
}

fn minimal_object(id: &str) -> ObjectMesh {
    ObjectMesh {
        id: id.to_string(),
        mesh: IndexedTriangleSet {
            vertices: vec![
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3 {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3 {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
            ],
            indices: vec![0, 1, 2],
        },
        transform: identity_transform(),
        config: ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: Vec::new(),
        paint_data: None,
        world_z_extent: None,
    }
}

fn blackboard_with_objects(object_ids: &[&str]) -> Blackboard {
    let objects: Vec<ObjectMesh> = object_ids.iter().map(|id| minimal_object(id)).collect();
    let mesh = Arc::new(MeshIR {
        schema_version: semver(1, 0, 0),
        objects,
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
        },
    });
    Blackboard::new(mesh, 0)
}

fn make_loaded_module(id: &str, stage: &str) -> LoadedModule {
    LoadedModuleBuilder::new(
        id,
        semver(1, 0, 0),
        stage,
        "slicer:world-prepass@1.0.0",
        std::path::PathBuf::from("/dev/null"),
    )
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(2, 0, 0))
    .layer_parallel_safe(true)
    .build()
}

fn make_compiled_module_with_config(
    id: &str,
    stage: &str,
    component: Arc<slicer_runtime::WasmComponent>,
    config: ConfigView,
) -> CompiledModule {
    let loaded = make_loaded_module(id, stage);
    let pool = Arc::new(
        build_wasm_instance_pool(
            &loaded,
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .unwrap(),
    );
    CompiledModuleBuilder::new(id, pool)
        .config_view(Arc::new(config))
        .wasm_component(Some(component))
        .build()
}

fn load_sdk_prepass_guest() -> Option<Arc<slicer_runtime::WasmComponent>> {
    if !sdk_prepass_guest_path().exists() {
        return None;
    }
    Some(wasm_cache::compiled_guest(SDK_PREPASS_GUEST_NAME))
}

// â”€â”€ AC-4: MeshSegmentation marks round-trip â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// AC-8: Dispatch MeshSegmentation with fixture_case="marks_basic".
/// The guest (sdk-prepass-meshseg-guest) emits:
///   mark_triangle_paint(object_id="obj-a", facet_index=12, semantic="material", value="1")
///
/// Config contract: `fixture_case = "marks_basic"` â†’
///   call mark-triangle-paint once with (object_id="obj-a", facet_index=12,
///     semantic="material", value="1") on the MeshSegmentationOutput resource.
///
/// After retargeting, MeshSegmentationIR.marks must contain an entry satisfying:
///   - object_id == "obj-a"
///   - facet_index == 12
///   - semantic string == "material"
///   - value == "1"
#[test]
fn mesh_segmentation_marks_round_trip() {
    use slicer_runtime::PrepassStageOutput;

    let engine = wasm_cache::shared_engine();
    let component = match load_sdk_prepass_guest() {
        Some(c) => c,
        None => {
            eprintln!("SKIP: sdk-prepass-meshseg-guest.component.wasm missing");
            return;
        }
    };
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));

    // Config contract: fixture_case="marks_basic" drives the guest to emit
    // mark_triangle_paint(object_id="obj-a", facet_index=12, semantic="material", value="1")
    let mut config_map = HashMap::new();
    config_map.insert(
        "fixture_case".to_string(),
        ConfigValue::String("marks_basic".to_string()),
    );
    let config = ConfigView::from_map(config_map);

    let module = make_compiled_module_with_config(
        "com.test.mesh-seg-marks",
        "PrePass::MeshSegmentation",
        component,
        config,
    );
    let blackboard = blackboard_with_objects(&["obj-a"]);

    let result = PrepassStageRunner::run_stage(
        &dispatcher,
        &"PrePass::MeshSegmentation".to_string(),
        &module,
        &blackboard,
    );

    let ir = match result {
        Ok((PrepassStageOutput::MeshSegmentation(ir), _)) => ir,
        Ok((PrepassStageOutput::None, _)) => {
            panic!(
                "AC-8 FAIL: got None â€” sdk-prepass-meshseg-guest did not emit marks_basic fixture"
            );
        }
        Ok((other, _)) => panic!(
            "AC-8 FAIL: unexpected variant {:?}",
            std::mem::discriminant(&other)
        ),
        Err(e) => panic!("AC-8 FAIL: dispatch error: {e}"),
    };

    // Find the mark for obj-a.
    let mark = ir
        .marks
        .iter()
        .find(|m| m.object_id == "obj-a")
        .unwrap_or_else(|| {
            panic!(
                "AC-4: harvested MeshSegmentationIR must contain a mark for obj-a; \
                 got {} marks: {:?}",
                ir.marks.len(),
                ir.marks
            )
        });

    assert_eq!(mark.facet_index, 12, "AC-8: facet_index must be 12");
    assert_eq!(
        mark.semantic, "material",
        "AC-8: semantic must be 'material'"
    );
    assert_eq!(mark.value, "1", "AC-8: value must be '1'");
}
