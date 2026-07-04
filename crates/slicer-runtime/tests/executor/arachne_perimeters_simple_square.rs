//! Integration test (AC-9, packet 112 Step 9B): drives the real
//! `arachne-perimeters.wasm` guest through the production `Layer::Perimeters`
//! dispatch path and asserts it produces real variable-width walls via the
//! host-service bridge (`slicer_sdk::host::generate_arachne_walls` ->
//! `slicer_core::arachne::pipeline::run_arachne_pipeline` on the host side).
//!
//! # Honesty note (no OrcaSlicer oracle)
//!
//! This test only asserts that the wired-up pipeline produces *some* walls
//! with variable per-vertex widths for a large-enough square — it does not
//! assert numeric parity with OrcaSlicer's `WallToolPaths`/
//! `PerimeterGenerator`. See `slicer_core::arachne::pipeline`'s own
//! module-level doc comment for the honesty caveats this test inherits.
//!
//! # Geometry note
//!
//! A tiny square (e.g. 0.1mm) legitimately yields zero walls: its medial-axis
//! depth never clears `ArachneParams::default().optimal_width` (0.4mm), so
//! every edge's bead count comes out `0` and `generate_toolpaths` emits
//! nothing (see `crates/slicer-core/src/arachne/pipeline.rs`'s own
//! `square_10mm` test fixture doc comment). This test therefore uses a 10mm
//! square, mirroring that fixture and the passing native
//! `run_arachne_pipeline_square_produces_lines` test.

#![allow(missing_docs)]

use slicer_ir::{mm_to_units, ExPolygon, Point2, Polygon, SemVer, SliceIR, SlicedRegion};
use slicer_runtime::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_runtime::manifest::LoadedModuleBuilder;
use slicer_runtime::{Blackboard, CompiledModuleBuilder, LayerArena, WasmRuntimeDispatcher};
use std::sync::Arc;

use crate::common::wasm_cache;
use crate::common::TestModuleBundle;

/// AC-9: a real 10mm square region, run through the real
/// `arachne-perimeters.wasm` guest via `Layer::Perimeters`, must produce at
/// least one `WallLoop`, ordered by ascending `perimeter_index`, each with a
/// populated `ExtrusionPath3D` (>= 2 points), and must exhibit real per-vertex
/// width variation (not every wall carries an identical constant width) —
/// proving `run_perimeters` actually calls the Arachne beading-strategy
/// pipeline rather than the pre-P112 skeleton stub.
#[test]
fn arachne_perimeters_simple_square_produces_walls() {
    let layer_index = 0u32;
    let layer_z = 0.2_f32;
    let object_id = "obj-a";
    let region_id: u64 = 0;

    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));

    // Load the real arachne-perimeters.wasm module (not a hand-rolled test
    // guest) so the actual `generate_arachne_walls` host-service bridge call
    // is exercised at the WIT boundary.
    let wasm_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap() // crates
        .parent()
        .unwrap() // pinch_n_print
        .join("modules/core-modules/arachne-perimeters/arachne-perimeters.wasm");
    assert!(
        wasm_path.exists(),
        "arachne-perimeters.wasm not found at {}. Build it with: `cargo xtask build-guests`",
        wasm_path.display()
    );
    let component = wasm_cache::compiled_component_at(&wasm_path);

    let loaded = LoadedModuleBuilder::new(
        "com.core.arachne-perimeters",
        SemVer {
            major: 0,
            minor: 1,
            patch: 0,
        },
        "Layer::Perimeters",
        "slicer:world-layer@1.0.0",
        wasm_path.clone(),
    )
    .ir_reads(vec!["SliceIR".to_string(), "PaintRegionIR".to_string()])
    .ir_writes(vec!["PerimeterIR".to_string()])
    .claims(vec!["perimeter-generator".to_string()])
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
        major: 5,
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
        .expect("instance pool must build"),
    );

    let module = CompiledModuleBuilder::new(loaded.id().to_string())
        .config_view(Arc::new(slicer_ir::ConfigView::from_map(
            std::collections::HashMap::new(),
        )))
        .build();

    let bundle = TestModuleBundle {
        module,
        pool,
        component: Some(component),
    };

    // Stage a real 10mm square region — large enough for the medial-axis
    // depth to clear ArachneParams::default().optimal_width (0.4mm); see this
    // test file's own module doc comment for why a smaller square would
    // legitimately produce zero walls.
    let side = mm_to_units(10.0);
    let mut arena = LayerArena::new();
    arena
        .set_slice(SliceIR {
            global_layer_index: layer_index,
            z: layer_z,
            regions: vec![SlicedRegion {
                object_id: object_id.to_string(),
                region_id,
                polygons: vec![ExPolygon {
                    contour: Polygon {
                        points: vec![
                            Point2 { x: 0, y: 0 },
                            Point2 { x: side, y: 0 },
                            Point2 { x: side, y: side },
                            Point2 { x: 0, y: side },
                        ],
                    },
                    holes: Vec::new(),
                }],
                infill_areas: Vec::new(),
                nonplanar_surface: None,
                effective_layer_height: 0.2,
                segment_annotations: std::collections::HashMap::new(),
                variant_chain: Vec::new(),
                top_shell_index: None,
                bottom_shell_index: None,
                top_solid_fill: Vec::new(),
                bottom_solid_fill: Vec::new(),
                is_bridge: false,
                bridge_areas: vec![],
                bridge_orientation_deg: 0.0,
                sparse_infill_area: Vec::new(),
            }],
            ..Default::default()
        })
        .expect("set_slice must succeed");

    let blackboard = Blackboard::new(Arc::new(slicer_ir::MeshIR::default()), 1);

    let layer = slicer_ir::GlobalLayer {
        index: layer_index,
        z: layer_z,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    crate::common::run_layer_and_commit_with_bundle(
        &dispatcher,
        "Layer::Perimeters",
        &layer,
        &bundle,
        &blackboard,
        &mut arena,
    )
    .expect("Layer::Perimeters dispatch+commit must succeed");

    let perimeter_ir = arena
        .perimeter()
        .expect("PerimeterIR must be committed after Layer::Perimeters dispatch");
    let region = perimeter_ir
        .regions
        .iter()
        .find(|r| r.object_id == object_id && r.region_id == region_id)
        .expect("staged region must be present in committed PerimeterIR");

    // (a) At least one wall loop produced — the guest must have actually
    // called generate_arachne_walls and gotten a non-empty pipeline result,
    // not the pre-P112 skeleton's silent Ok(()) with no walls.
    assert!(
        !region.walls.is_empty(),
        "expected at least one WallLoop for a 10mm square region, got 0 — either the guest \
         still runs the skeleton stub, or the pipeline legitimately produced nothing for this \
         geometry (unexpected for 10mm; see module doc comment)"
    );

    // (b) Walls sorted by perimeter_index ascending.
    let indices: Vec<u32> = region.walls.iter().map(|w| w.perimeter_index).collect();
    let mut sorted_indices = indices.clone();
    sorted_indices.sort();
    assert_eq!(
        indices, sorted_indices,
        "WallLoop.perimeter_index must be ascending across region.walls, got {indices:?}"
    );

    // (c) Each wall's path is a populated ExtrusionPath3D (>= 2 points).
    for (i, wall) in region.walls.iter().enumerate() {
        assert!(
            wall.path.points.len() >= 2,
            "wall[{i}] (perimeter_index={}) path.points.len() must be >= 2, got {}",
            wall.perimeter_index,
            wall.path.points.len()
        );
        assert_eq!(
            wall.width_profile.widths.len(),
            wall.path.points.len(),
            "wall[{i}] width_profile.widths must be parallel to path.points"
        );
    }

    // (d) Variable widths observable: not every wall's width_profile.widths
    // is identical to every other's — the real Arachne pipeline emits both
    // the outer wall (a 3-junction line closing back on itself per spoke) and
    // multiple deeper insets (2-junction lines) with distinct width vectors
    // for a 10mm square (confirmed empirically: 26 lines across 9 insets,
    // width vectors including both `[0.0, 1.11, 0.0]` and `[1.11, 0.0]`
    // shapes — never a single constant-width vector repeated for every wall).
    assert!(
        region.walls.len() > 1,
        "expected more than one WallLoop to compare widths across, got {}",
        region.walls.len()
    );
    let first_widths = &region.walls[0].width_profile.widths;
    let all_identical = region
        .walls
        .iter()
        .all(|w| &w.width_profile.widths == first_widths);
    assert!(
        !all_identical,
        "expected variable widths across walls (not all width_profile.widths identical), \
         got identical widths across all {} walls: {:?}",
        region.walls.len(),
        region
            .walls
            .iter()
            .map(|w| &w.width_profile.widths)
            .collect::<Vec<_>>()
    );
}
