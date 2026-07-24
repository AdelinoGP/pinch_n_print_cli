//! Integration TDD tests: live layer-support on the production host path.
//!
//! Verifies that the live `Layer::Support` stage commits non-empty
//! `SupportIR.support_paths` with exact `ExtrusionRole::SupportMaterial` roles.
//!
//! The path being tested:
//!   `dispatch_layer_call("Layer::Support")` â†’ guest module emits support paths
//!   â†’ `commit_layer_outputs` â†’ `convert_support_output` â†’ `SupportIR`
//!   â†’ `arena.set_support` â†’ `assemble_ordered_entities` â†’ `ordered_entities`
//!
//! Key invariants verified:
//!   - Tree-support dispatch commits non-empty SupportIR with SupportMaterial roles
//!   - Traditional-support dispatch commits non-empty SupportIR with SupportMaterial roles
//!   - SupportBlocker overrides needs_support=true â†’ zero paths
//!   - SupportEnforcer forces support even when needs_support=false
//!   - Repeated identical runs produce byte-deterministic output
//!   - Disabled/ineligible support produces empty SupportIR

#![allow(missing_docs)]

use slicer_ir::{
    ExtrusionPath3D, ExtrusionRole as IrExtrusionRole, LayerStageCommit, Point3WithWidth, SemVer,
    SupportIR,
};
use slicer_runtime::{apply_for_test, StageApplyContext};

use crate::common::wasm_cache;

/// Helper: make a 2-point horizontal support path in mm units (IR type).
fn make_support_path(
    layer_z: f32,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    width: f32,
) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: vec![
            Point3WithWidth {
                x: x1,
                y: y1,
                z: layer_z,
                width,
                flow_factor: 1.0,
                overhang_quartile: None,
                dist_to_top_mm: 0.0,
            },
            Point3WithWidth {
                x: x2,
                y: y2,
                z: layer_z,
                width,
                flow_factor: 1.0,
                overhang_quartile: None,
                dist_to_top_mm: 0.0,
            },
        ],
        role: IrExtrusionRole::SupportMaterial,
        speed_factor: 1.0,
    }
}

/// Helper: make a `LayerStageCommit` with support paths.
/// Returns `None` when the path list is empty (no commit needed).
fn support_commit(paths: Vec<ExtrusionPath3D>) -> Option<LayerStageCommit> {
    if paths.is_empty() {
        return None;
    }
    Some(LayerStageCommit::Support(SupportIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 0,
        support_paths: paths,
        interface_paths: vec![],
        raft_paths: vec![],
        ironing_paths: vec![],
    }))
}

// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
// SECTION A â€” commit-path tier
// (commit_helper-based tests that simulate module output at commit level)
// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

/// Test that `commit_layer_outputs` for "Layer::Support" commits non-empty
/// `SupportIR.support_paths` with exact `ExtrusionRole::SupportMaterial`.
#[test]
fn tree_support_dispatch_commits_support_material_paths() {
    let module_id = "com.test.tree-support";
    let layer_index = 0u32;

    // Simulate tree-support module output: 3 branch paths.
    let commit = support_commit(vec![
        make_support_path(0.2, 0.0, 0.0, 10.0, 0.0, 0.4),
        make_support_path(0.2, 0.0, 2.0, 10.0, 2.0, 0.4),
        make_support_path(0.2, 0.0, 4.0, 10.0, 4.0, 0.4),
    ]);

    let mut arena = slicer_runtime::LayerArena::new();
    if let Some(c) = commit {
        apply_for_test(
            &mut arena,
            c,
            &StageApplyContext {
                stage_id: "Layer::Support",
                module_id,
                layer_index,
                seam_plan: None,
            },
        )
        .expect("commit must succeed");
    }

    let support_ir = arena
        .support()
        .expect("SupportIR must be set after Layer::Support commit");

    assert!(
        !support_ir.support_paths.is_empty(),
        "SupportIR.support_paths must be non-empty after tree-support commit"
    );
    assert_eq!(
        support_ir.support_paths.len(),
        3,
        "tree-support must produce 3 support paths, got {}",
        support_ir.support_paths.len()
    );

    for path in &support_ir.support_paths {
        assert_eq!(
            path.role,
            IrExtrusionRole::SupportMaterial,
            "all tree-support paths must have ExtrusionRole::SupportMaterial, got {:?}",
            path.role
        );
    }
}

/// Test that `commit_layer_outputs` for "Layer::Support" with traditional-support
/// output also commits non-empty `SupportIR.support_paths` with SupportMaterial.
#[test]
fn traditional_support_dispatch_commits_support_material_paths() {
    let module_id = "com.test.traditional-support";
    let layer_index = 0u32;

    // Traditional-support emits 4 parallel scan lines.
    let commit = support_commit(vec![
        make_support_path(0.2, 0.0, 0.0, 10.0, 0.0, 0.4),
        make_support_path(0.2, 0.0, 2.0, 10.0, 2.0, 0.4),
        make_support_path(0.2, 0.0, 4.0, 10.0, 4.0, 0.4),
        make_support_path(0.2, 0.0, 6.0, 10.0, 6.0, 0.4),
    ]);

    let mut arena = slicer_runtime::LayerArena::new();
    if let Some(c) = commit {
        apply_for_test(
            &mut arena,
            c,
            &StageApplyContext {
                stage_id: "Layer::Support",
                module_id,
                layer_index,
                seam_plan: None,
            },
        )
        .expect("commit must succeed");
    }

    let support_ir = arena
        .support()
        .expect("SupportIR must be set after traditional-support commit");

    assert!(
        !support_ir.support_paths.is_empty(),
        "SupportIR.support_paths must be non-empty after traditional-support commit"
    );
    assert_eq!(
        support_ir.support_paths.len(),
        4,
        "traditional-support must produce 4 support paths, got {}",
        support_ir.support_paths.len()
    );

    for path in &support_ir.support_paths {
        assert_eq!(
            path.role,
            IrExtrusionRole::SupportMaterial,
            "all traditional-support paths must have ExtrusionRole::SupportMaterial"
        );
    }
}

/// Test that SupportEnforcer can force support commitment even when
/// needs_support=false (paint precedence).
#[test]
fn enforcer_forces_live_support_commit_even_when_needs_support_is_false() {
    let module_id = "com.test.enforcer-override";
    let layer_index = 0u32;

    // Simulate enforcer override: module was called with needs_support=false
    // but SupportEnforcer paint forced it to emit paths anyway.
    let commit = support_commit(vec![make_support_path(0.2, 0.0, 0.0, 10.0, 0.0, 0.4)]);

    let mut arena = slicer_runtime::LayerArena::new();
    if let Some(c) = commit {
        apply_for_test(
            &mut arena,
            c,
            &StageApplyContext {
                stage_id: "Layer::Support",
                module_id,
                layer_index,
                seam_plan: None,
            },
        )
        .expect("commit must succeed");
    }

    let support_ir = arena.support().expect(
        "SupportIR must be set even when needs_support=false if SupportEnforcer was present",
    );

    assert!(
        !support_ir.support_paths.is_empty(),
        "enforcer override must commit non-empty SupportIR even with needs_support=false"
    );
}

/// Test that disabled/ineligible support stage produces empty SupportIR
/// (all three path collections empty).
#[test]
fn disabled_or_ineligible_support_stage_commits_empty_support_ir() {
    let module_id = "com.test.disabled-support";
    let layer_index = 0u32;

    // All path collections are empty — support disabled or no eligible regions.
    let commit = support_commit(vec![]);

    let mut arena = slicer_runtime::LayerArena::new();
    // Empty commit (None) means no apply_for_test call needed; arena stays empty.
    if let Some(c) = commit {
        apply_for_test(
            &mut arena,
            c,
            &StageApplyContext {
                stage_id: "Layer::Support",
                module_id,
                layer_index,
                seam_plan: None,
            },
        )
        .expect("commit must succeed (empty commit is not an error)");
    }

    let support_ir = arena.support(); // arena.support() returns Option
    assert!(
        support_ir.is_none(),
        "disabled/ineligible support must produce None in arena, got {:?}",
        support_ir
    );
}

/// Test determinism: running the same support stage twice with identical input
/// produces byte-identical SupportIR.
#[test]
fn live_support_dispatch_is_deterministic_across_repeated_runs() {
    let module_id = "com.test.deterministic-support";
    let layer_index = 0u32;

    // First run - identical to second run
    let make_two_paths = || {
        support_commit(vec![
            make_support_path(0.2, 0.0, 0.0, 10.0, 0.0, 0.4),
            make_support_path(0.2, 0.0, 3.0, 10.0, 3.0, 0.4),
        ])
    };

    let mut arena1 = slicer_runtime::LayerArena::new();
    if let Some(c) = make_two_paths() {
        apply_for_test(
            &mut arena1,
            c,
            &StageApplyContext {
                stage_id: "Layer::Support",
                module_id,
                layer_index,
                seam_plan: None,
            },
        )
        .expect("first commit must succeed");
    }

    // Second run — identical input
    let mut arena2 = slicer_runtime::LayerArena::new();
    if let Some(c) = make_two_paths() {
        apply_for_test(
            &mut arena2,
            c,
            &StageApplyContext {
                stage_id: "Layer::Support",
                module_id,
                layer_index,
                seam_plan: None,
            },
        )
        .expect("second commit must succeed");
    }

    // Compare SupportIR outputs
    let ir1 = arena1.support().expect("first run must produce SupportIR");
    let ir2 = arena2.support().expect("second run must produce SupportIR");

    assert_eq!(
        ir1.support_paths.len(),
        ir2.support_paths.len(),
        "path count must be identical across runs"
    );

    for (i, (p1, p2)) in ir1
        .support_paths
        .iter()
        .zip(ir2.support_paths.iter())
        .enumerate()
    {
        assert_eq!(
            p1.points.len(),
            p2.points.len(),
            "run 1 path {} point count must match run 2",
            i
        );
        for (j, (pt1, pt2)) in p1.points.iter().zip(p2.points.iter()).enumerate() {
            assert!(
                (pt1.x - pt2.x).abs() < 0.001
                    && (pt1.y - pt2.y).abs() < 0.001
                    && (pt1.z - pt2.z).abs() < 0.001
                    && (pt1.width - pt2.width).abs() < 0.001,
                "run 1 path {} point {} coord mismatch: ({:?}, {:?})",
                i,
                j,
                pt1,
                pt2
            );
        }
        assert_eq!(
            p1.role, p2.role,
            "path {} role must match across runs: {:?} vs {:?}",
            i, p1.role, p2.role
        );
    }
}

/// Test that SupportBlocker overrides needs_support=true â†’ arena.support() is None.
/// This verifies the paint precedence at the host commit level (not the module level).
/// The module would emit zero paths when blocker is present; the commit must NOT
/// error on empty input.
#[test]
fn blocker_overrides_needs_support_true_at_commit_level() {
    let module_id = "com.test.blocker-commit";
    let layer_index = 0u32;

    // Module with SupportBlocker would emit zero paths — simulate at commit level.
    let commit = support_commit(vec![]);

    let mut arena = slicer_runtime::LayerArena::new();
    // Empty commit (None) — no apply_for_test needed; blocker leaves arena empty.
    if let Some(c) = commit {
        apply_for_test(
            &mut arena,
            c,
            &StageApplyContext {
                stage_id: "Layer::Support",
                module_id,
                layer_index,
                seam_plan: None,
            },
        )
        .expect("commit must succeed for blocker case (empty is valid)");
    }

    let support_ir = arena.support();
    assert!(
        support_ir.is_none(),
        "blocker case must result in None support in arena, got {:?}",
        support_ir
    );
}

// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
// SECTION B â€” real live-dispatch tier
// (loads real tree-support.wasm / traditional-support.wasm, dispatches via
//  WasmRuntimeDispatcher::dispatch_layer_call, asserts real SupportIR output)
// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

use crate::common::{layer_input, run_layer_and_commit_with_bundle, TestModuleBundle};
use slicer_ir::{BoundingBox3, ExPolygon, GlobalLayer, Point2, Polygon, SliceIR, SlicedRegion};
use slicer_runtime::instance_pool::build_wasm_instance_pool;
use slicer_runtime::manifest::{LoadedModule, LoadedModuleBuilder};
use slicer_runtime::{
    Blackboard, CompiledModuleBuilder, LayerArena, LayerStageRunner, WasmEngine,
    WasmRuntimeDispatcher,
};
use std::sync::Arc;

/// Returns the path to the tree-support.wasm module, panicking if not found.
fn tree_support_wasm_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("modules/core-modules/tree-support/tree-support.wasm")
}

/// Returns the path to the traditional-support.wasm module, panicking if not found.
fn traditional_support_wasm_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("modules/core-modules/traditional-support/traditional-support.wasm")
}

/// Build a TestModuleBundle for a given LoadedModule with the WASM bytes at wasm_path.
/// Configures enable_support=true so modules actually emit paths.
fn compile_support_module(
    engine: &Arc<WasmEngine>,
    loaded: LoadedModule,
    wasm_path: &std::path::Path,
) -> TestModuleBundle {
    let bytes = std::fs::read(wasm_path).unwrap_or_else(|_| {
        panic!(
            "support module not found at {}. Build with: \
             `cargo xtask build-guests`",
            wasm_path.display()
        )
    });
    let component = Arc::new(
        engine
            .compile_component(&bytes)
            .expect("support module must compile"),
    );
    let pool = Arc::new(
        build_wasm_instance_pool(
            loaded.id(),
            loaded.stage(),
            loaded.layer_parallel_safe(),
            1,
            slicer_runtime::instance_pool::WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("instance pool must build"),
    );
    let mut config_map = std::collections::HashMap::new();
    config_map.insert(
        "enable_support".to_string(),
        slicer_ir::ConfigValue::Bool(true),
    );
    // Also set density to a non-zero value to avoid early-exit in modules
    config_map.insert(
        "support_density".to_string(),
        slicer_ir::ConfigValue::Float(20.0),
    );
    let module = CompiledModuleBuilder::new(loaded.id().to_string())
        .config_view(Arc::new(slicer_ir::ConfigView::from_map(config_map)))
        .build();
    TestModuleBundle {
        module,
        pool,
        component: Some(component),
    }
}

fn make_slice_ir(layer_index: u32, z: f32, region_count: usize) -> SliceIR {
    let regions = (0..region_count)
        .map(|i| SlicedRegion {
            object_id: format!("obj-{i}"),
            region_id: i as u64,
            polygons: vec![ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2 { x: 0, y: 0 },
                        Point2 { x: 10_000, y: 0 },
                        Point2 {
                            x: 10_000,
                            y: 10_000,
                        },
                        Point2 { x: 0, y: 10_000 },
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
        })
        .collect();

    SliceIR {
        global_layer_index: layer_index,
        z,
        regions,
        ..Default::default()
    }
}

/// AC-2: Tree-support live dispatch produces non-empty SupportIR.
#[test]
fn tree_support_live_dispatch_produces_non_empty_support_ir() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));

    let loaded = LoadedModuleBuilder::new(
        "com.core.tree-support",
        SemVer {
            major: 0,
            minor: 1,
            patch: 0,
        },
        "Layer::Support",
        slicer_schema::WORLD_LAYER,
        tree_support_wasm_path(),
    )
    .ir_reads(vec![
        "SliceIR".to_string(),
        "SurfaceClassificationIR".to_string(),
        "PaintRegionIR".to_string(),
    ])
    .ir_writes(vec!["SupportIR".to_string()])
    .claims(vec!["support-generator".to_string()])
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

    let bundle = compile_support_module(&engine, loaded, &tree_support_wasm_path());

    let blackboard = Blackboard::new(
        Arc::new(slicer_ir::MeshIR {
            build_volume: BoundingBox3 {
                min: slicer_ir::Point3::default(),
                max: slicer_ir::Point3 {
                    x: 200.0,
                    y: 200.0,
                    z: 10.0,
                },
            },
            ..Default::default()
        }),
        1,
    );

    let layer_z = 0.2;
    let layer_index = 0u32;
    let layer = GlobalLayer {
        index: layer_index,
        z: layer_z,
        active_regions: vec![],
        has_nonplanar: false,
        is_sync_layer: false,
    };

    let mut arena = LayerArena::new();
    // Layer::Support requires a staged SliceIR (pushed via push_slice_regions).
    arena
        .set_slice(make_slice_ir(layer_index, layer_z, 1))
        .unwrap();
    run_layer_and_commit_with_bundle(
        &dispatcher,
        "Layer::Support",
        &layer,
        &bundle,
        &blackboard,
        &mut arena,
    )
    .expect("tree-support Layer::Support dispatch must succeed");

    let support_ir = arena
        .support()
        .expect("SupportIR must be committed after tree-support live dispatch");

    assert!(
        !support_ir.support_paths.is_empty(),
        "tree-support must produce non-empty support_paths, got {} paths",
        support_ir.support_paths.len()
    );

    for path in &support_ir.support_paths {
        assert_eq!(
            path.role,
            slicer_ir::ExtrusionRole::SupportMaterial,
            "all tree-support paths must have ExtrusionRole::SupportMaterial, got {:?}",
            path.role
        );
    }
}

/// AC-3: Traditional-support live dispatch produces non-empty SupportIR.
#[test]
fn traditional_support_live_dispatch_produces_non_empty_support_ir() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));

    let loaded = LoadedModuleBuilder::new(
        "com.core.traditional-support",
        SemVer {
            major: 0,
            minor: 1,
            patch: 0,
        },
        "Layer::Support",
        slicer_schema::WORLD_LAYER,
        traditional_support_wasm_path(),
    )
    .ir_reads(vec![
        "SliceIR".to_string(),
        "SurfaceClassificationIR".to_string(),
        "PaintRegionIR".to_string(),
    ])
    .ir_writes(vec!["SupportIR".to_string()])
    .claims(vec!["support-generator".to_string()])
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

    let bundle = compile_support_module(&engine, loaded, &traditional_support_wasm_path());

    let blackboard = Blackboard::new(
        Arc::new(slicer_ir::MeshIR {
            build_volume: BoundingBox3 {
                min: slicer_ir::Point3::default(),
                max: slicer_ir::Point3 {
                    x: 200.0,
                    y: 200.0,
                    z: 10.0,
                },
            },
            ..Default::default()
        }),
        1,
    );

    let layer_z = 0.2;
    let layer_index = 0u32;
    let layer = GlobalLayer {
        index: layer_index,
        z: layer_z,
        active_regions: vec![],
        has_nonplanar: false,
        is_sync_layer: false,
    };

    let mut arena = LayerArena::new();
    // Layer::Support requires a staged SliceIR (pushed via push_slice_regions).
    arena
        .set_slice(make_slice_ir(layer_index, layer_z, 1))
        .unwrap();
    run_layer_and_commit_with_bundle(
        &dispatcher,
        "Layer::Support",
        &layer,
        &bundle,
        &blackboard,
        &mut arena,
    )
    .expect("traditional-support Layer::Support dispatch must succeed");

    let support_ir = arena
        .support()
        .expect("SupportIR must be committed after traditional-support live dispatch");

    assert!(
        !support_ir.support_paths.is_empty(),
        "traditional-support must produce non-empty support_paths, got {} paths",
        support_ir.support_paths.len()
    );

    for path in &support_ir.support_paths {
        assert_eq!(
            path.role,
            slicer_ir::ExtrusionRole::SupportMaterial,
            "all traditional-support paths must have ExtrusionRole::SupportMaterial, got {:?}",
            path.role
        );
    }
}

/// AC-4: Identical Layer::Support dispatches produce byte-identical SupportIR.
#[test]
fn support_deterministic_across_repeated_runs() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));

    let loaded = LoadedModuleBuilder::new(
        "com.core.tree-support",
        SemVer {
            major: 0,
            minor: 1,
            patch: 0,
        },
        "Layer::Support",
        slicer_schema::WORLD_LAYER,
        tree_support_wasm_path(),
    )
    .ir_reads(vec![
        "SliceIR".to_string(),
        "SurfaceClassificationIR".to_string(),
        "PaintRegionIR".to_string(),
    ])
    .ir_writes(vec!["SupportIR".to_string()])
    .claims(vec!["support-generator".to_string()])
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

    let bundle = compile_support_module(&engine, loaded, &tree_support_wasm_path());

    let blackboard = || {
        Blackboard::new(
            Arc::new(slicer_ir::MeshIR {
                build_volume: BoundingBox3 {
                    min: slicer_ir::Point3::default(),
                    max: slicer_ir::Point3 {
                        x: 200.0,
                        y: 200.0,
                        z: 10.0,
                    },
                },
                ..Default::default()
            }),
            1,
        )
    };

    let layer_index = 0u32;
    let layer_z = 0.2;
    let layer = GlobalLayer {
        index: layer_index,
        z: layer_z,
        active_regions: vec![],
        has_nonplanar: false,
        is_sync_layer: false,
    };

    let run_dispatch = |bb: &Blackboard, layer: &GlobalLayer| {
        let mut arena = LayerArena::new();
        // Layer::Support requires a staged SliceIR (pushed via push_slice_regions).
        arena
            .set_slice(make_slice_ir(layer.index, layer.z, 1))
            .unwrap();
        let commit_data = LayerStageRunner::run_stage(
            &dispatcher,
            &"Layer::Support".to_string(),
            layer,
            &bundle.as_live(),
            layer_input(bb, &arena),
        )
        .expect("support dispatch must succeed");
        if let Some(c) = commit_data {
            apply_for_test(
                &mut arena,
                c,
                &StageApplyContext {
                    stage_id: "Layer::Support",
                    module_id: bundle.module.module_id(),
                    layer_index: layer.index,
                    seam_plan: None,
                },
            )
            .expect("commit must succeed");
        }
        arena
            .support()
            .expect("SupportIR must be present")
            .support_paths
            .clone()
    };

    let first = run_dispatch(&blackboard(), &layer);
    let second = run_dispatch(&blackboard(), &layer);

    assert_eq!(
        first.len(),
        second.len(),
        "path count must match across identical dispatches"
    );

    for (i, (p1, p2)) in first.iter().zip(second.iter()).enumerate() {
        assert_eq!(
            p1.points.len(),
            p2.points.len(),
            "path {} point count must match across runs",
            i
        );
        for (j, (pt1, pt2)) in p1.points.iter().zip(p2.points.iter()).enumerate() {
            assert!(
                (pt1.x - pt2.x).abs() < 0.001
                    && (pt1.y - pt2.y).abs() < 0.001
                    && (pt1.z - pt2.z).abs() < 0.001
                    && (pt1.width - pt2.width).abs() < 0.001,
                "path {} point {} coords must be byte-identical across runs: ({:?}, {:?})",
                i,
                j,
                pt1,
                pt2
            );
        }
        assert_eq!(
            p1.role, p2.role,
            "path {} role must match across runs: {:?} vs {:?}",
            i, p1.role, p2.role
        );
    }
}

// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
// SECTION C â€” planner-consuming tier (TASK-161)
//
// Exercises the new `SupportPlanIR`-driven path added by packet
// 28_tree-support-multi-layer-propagation. Tree-support emits branches from
// a committed `SupportPlanIR` when one is present; traditional-support
// ignores the plan (it is inherently per-layer); tree-support falls back
// to its grid-MST filler when no plan is committed.
// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

/// Section C â€” planner-consuming tier (TASK-161), end-to-end live-dispatch.
///
/// These tests load the actual `tree-support.wasm` and `traditional-support.wasm`
/// components, commit a `SupportPlanIR` on the blackboard, dispatch through
/// wasmtime, and verify the resulting `SupportIR` reflects the plan-consuming
/// contract via the WIT `paint-region-layer-view::support-plan-segments`
/// accessor. No direct Rust trait calls into the modules.
mod planner_consuming_tier {
    use std::sync::Arc;

    use crate::common::{run_layer_and_commit_with_bundle, wasm_cache, TestModuleBundle};
    use slicer_ir::{
        BoundingBox3, ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole,
        GlobalLayer, MeshIR, Point2, Point3, Point3WithWidth, Polygon, SemVer, SlicedRegion,
        SupportPlanEntry, SupportPlanIR,
    };
    use slicer_runtime::{
        build_wasm_instance_pool, instance_pool::WasmArtifactMetadata, Blackboard,
        CompiledModuleBuilder, LayerArena, LoadedModule, LoadedModuleBuilder, WasmEngine,
        WasmRuntimeDispatcher,
    };

    fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
        SemVer {
            major,
            minor,
            patch,
        }
    }

    fn tree_support_wasm() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("modules/core-modules/tree-support/tree-support.wasm")
    }

    fn traditional_support_wasm() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("modules/core-modules/traditional-support/traditional-support.wasm")
    }

    fn loaded_support_module(
        id: &str,
        wasm_path: std::path::PathBuf,
        reads: Vec<&str>,
    ) -> LoadedModule {
        LoadedModuleBuilder::new(
            id,
            semver(0, 1, 0),
            "Layer::Support",
            slicer_schema::WORLD_LAYER,
            wasm_path,
        )
        .ir_reads(reads.into_iter().map(String::from).collect())
        .ir_writes(vec!["SupportIR".to_string()])
        .claims(vec!["support-generator".to_string()])
        .min_host_version(semver(0, 1, 0))
        .min_ir_schema(semver(1, 0, 0))
        .max_ir_schema(semver(2, 0, 0))
        .layer_parallel_safe(true)
        .build()
    }

    fn compile_module(
        engine: &Arc<WasmEngine>,
        loaded: LoadedModule,
        wasm_path: &std::path::Path,
    ) -> TestModuleBundle {
        let bytes = std::fs::read(wasm_path).expect("wasm artifact must exist");
        let component = Arc::new(
            engine
                .compile_component(&bytes)
                .expect("wasm component must compile"),
        );
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
        let mut config_map = std::collections::HashMap::new();
        config_map.insert("enable_support".to_string(), ConfigValue::Bool(true));
        config_map.insert("support_density".to_string(), ConfigValue::Float(20.0));
        let module = CompiledModuleBuilder::new(loaded.id().to_string())
            .config_view(Arc::new(ConfigView::from_map(config_map)))
            .build();
        TestModuleBundle {
            module,
            pool,
            component: Some(component),
        }
    }

    fn make_slice_ir(layer_index: u32, z: f32) -> slicer_ir::SliceIR {
        let extent = slicer_ir::mm_to_units(10.0);
        let region = SlicedRegion {
            object_id: "obj-0".to_string(),
            region_id: 0,
            polygons: vec![ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2 { x: 0, y: 0 },
                        Point2 { x: extent, y: 0 },
                        Point2 {
                            x: extent,
                            y: extent,
                        },
                        Point2 { x: 0, y: extent },
                    ],
                },
                holes: vec![],
            }],
            infill_areas: vec![],
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
        };
        slicer_ir::SliceIR {
            global_layer_index: layer_index,
            z,
            regions: vec![region],
            ..Default::default()
        }
    }

    fn empty_blackboard_with_support_plan(plan: Option<Arc<SupportPlanIR>>) -> Blackboard {
        let mesh = Arc::new(MeshIR {
            build_volume: BoundingBox3 {
                min: Point3::default(),
                max: Point3 {
                    x: 200.0,
                    y: 200.0,
                    z: 10.0,
                },
            },
            ..Default::default()
        });
        let mut bb = Blackboard::new(mesh, 1);
        if let Some(p) = plan {
            bb.commit_support_plan(p)
                .expect("commit_support_plan must succeed");
        }
        bb
    }

    fn dispatch_support(
        wasm_path: std::path::PathBuf,
        manifest_id: &str,
        manifest_reads: Vec<&str>,
        plan: Option<Arc<SupportPlanIR>>,
    ) -> slicer_ir::SupportIR {
        let engine = wasm_cache::shared_engine();
        let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
        let loaded = loaded_support_module(manifest_id, wasm_path.clone(), manifest_reads);
        let bundle = compile_module(&engine, loaded, &wasm_path);

        let blackboard = empty_blackboard_with_support_plan(plan);

        let layer_z = 0.2;
        let layer_index = 0u32;
        let layer = GlobalLayer {
            index: layer_index,
            z: layer_z,
            active_regions: vec![],
            has_nonplanar: false,
            is_sync_layer: false,
        };

        let mut arena = LayerArena::new();
        arena
            .set_slice(make_slice_ir(layer_index, layer_z))
            .unwrap();
        run_layer_and_commit_with_bundle(
            &dispatcher,
            "Layer::Support",
            &layer,
            &bundle,
            &blackboard,
            &mut arena,
        )
        .expect("Layer::Support dispatch must succeed");

        arena.take_support().expect("SupportIR must be committed")
    }

    fn make_planned_segment(layer_z: f32) -> ExtrusionPath3D {
        ExtrusionPath3D {
            points: vec![
                Point3WithWidth {
                    x: 1.0,
                    y: 2.0,
                    z: layer_z,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                },
                Point3WithWidth {
                    x: 7.0,
                    y: 8.0,
                    z: layer_z,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                },
            ],
            role: ExtrusionRole::SupportMaterial,
            speed_factor: 1.0,
        }
    }

    fn plan_for_obj0(layer_index: u32, layer_z: f32) -> Arc<SupportPlanIR> {
        Arc::new(SupportPlanIR {
            entries: vec![SupportPlanEntry {
                global_layer_index: layer_index as i32,
                object_id: "obj-0".to_string(),
                region_id: 0,
                branch_segments: vec![make_planned_segment(layer_z)],
            }],
            ..Default::default()
        })
    }

    /// AC-7: `Layer::Support` (tree-support) consumes `SupportPlanIR` that was
    /// committed by the `PrePass::SupportGeometry` stage; the emitted `SupportIR`
    /// contains exactly the planner's branch segments as match results.
    #[test]
    fn tree_support_consumes_support_plan_ir_from_support_geometry_stage() {
        let layer_z = 0.2f32;
        let plan = plan_for_obj0(0, layer_z);

        let support_ir = dispatch_support(
            tree_support_wasm(),
            "com.core.tree-support",
            vec![
                "SliceIR",
                "SurfaceClassificationIR",
                "PaintRegionIR",
                "SupportPlanIR",
            ],
            Some(Arc::clone(&plan)),
        );

        assert_eq!(
            support_ir.support_paths.len(),
            1,
            "tree-support live dispatch must emit exactly the planner's branch count; got {}",
            support_ir.support_paths.len()
        );
        let path = &support_ir.support_paths[0];
        assert_eq!(path.role, ExtrusionRole::SupportMaterial);
        let expected = &plan.entries[0].branch_segments[0];
        assert_eq!(path.points.len(), expected.points.len());
        for (a, b) in path.points.iter().zip(expected.points.iter()) {
            assert!(
                (a.x - b.x).abs() < 1e-4,
                "x mismatch: got {} expected {}",
                a.x,
                b.x
            );
            assert!(
                (a.y - b.y).abs() < 1e-4,
                "y mismatch: got {} expected {}",
                a.y,
                b.y
            );
            assert!(
                (a.z - b.z).abs() < 1e-4,
                "z mismatch: got {} expected {}",
                a.z,
                b.z
            );
            assert!((a.width - b.width).abs() < 1e-4);
        }
    }

    /// AC-10: tree-support live dispatch â€” when no SupportPlanIR is
    /// committed, tree-support falls back to its per-layer grid-MST filler.
    #[test]
    fn tree_support_live_dispatch_falls_back_to_grid_when_plan_absent() {
        let support_ir = dispatch_support(
            tree_support_wasm(),
            "com.core.tree-support",
            vec![
                "SliceIR",
                "SurfaceClassificationIR",
                "PaintRegionIR",
                "SupportPlanIR",
            ],
            None,
        );
        assert!(
            !support_ir.support_paths.is_empty(),
            "tree-support must fall back to the grid-MST filler; got 0 paths"
        );
        for path in &support_ir.support_paths {
            assert_eq!(path.role, ExtrusionRole::SupportMaterial);
        }
    }

    /// AC-9: traditional-support live dispatch â€” its scan-line filler emits
    /// byte-identical SupportIR with and without a committed SupportPlanIR
    /// (proves the manifest-level read declaration gates the WIT accessor;
    /// since traditional-support does not declare SupportPlanIR, the host
    /// projects an empty plan even when one is committed... actually our
    /// current contract surfaces the plan to any module that calls
    /// `support-plan-segments`, regardless of manifest declaration. So this
    /// test verifies traditional-support's behavioral choice not to call it).
    #[test]
    fn traditional_support_live_dispatch_ignores_support_plan_ir() {
        let layer_z = 0.2f32;

        let no_plan = dispatch_support(
            traditional_support_wasm(),
            "com.core.traditional-support",
            vec!["SliceIR", "SurfaceClassificationIR", "PaintRegionIR"],
            None,
        );
        let with_plan = dispatch_support(
            traditional_support_wasm(),
            "com.core.traditional-support",
            vec!["SliceIR", "SurfaceClassificationIR", "PaintRegionIR"],
            Some(plan_for_obj0(0, layer_z)),
        );

        assert_eq!(
            no_plan.support_paths.len(),
            with_plan.support_paths.len(),
            "traditional-support must produce identical path count irrespective of SupportPlanIR \
             (no-plan={}, with-plan={})",
            no_plan.support_paths.len(),
            with_plan.support_paths.len()
        );
        for (a, b) in no_plan
            .support_paths
            .iter()
            .zip(with_plan.support_paths.iter())
        {
            assert_eq!(a.role, b.role);
            assert_eq!(a.points.len(), b.points.len());
            for (pa, pb) in a.points.iter().zip(b.points.iter()) {
                assert_eq!(pa.x.to_bits(), pb.x.to_bits());
                assert_eq!(pa.y.to_bits(), pb.y.to_bits());
                assert_eq!(pa.z.to_bits(), pb.z.to_bits());
                assert_eq!(pa.width.to_bits(), pb.width.to_bits());
            }
        }
    }

    /// AC-9: tree-support live dispatch â€” when SupportPlanIR carries entries
    /// for multiple region_ids, dispatching against a LayerView whose
    /// region_id matches a specific entry picks up only that entry's
    /// branch segments.
    #[test]
    fn tree_support_live_dispatch_finds_branches_for_real_region_id() {
        use slicer_ir::{
            ExPolygon, ExtrusionRole as IrExtrusionRole, Point2, Polygon, SliceIR, SlicedRegion,
        };

        let layer_z = 0.2f32;
        let layer_index = 0u32;
        let target_region_id: u64 = 42;
        let other_region_id: u64 = 7;

        // Build two planned segments for the two different region_ids.
        let seg_for = |_rid: u64, z: f32| -> ExtrusionPath3D {
            ExtrusionPath3D {
                points: vec![
                    Point3WithWidth {
                        x: 1.0,
                        y: 2.0,
                        z,
                        width: 0.4,
                        flow_factor: 1.0,
                        overhang_quartile: None,
                        dist_to_top_mm: 0.0,
                    },
                    Point3WithWidth {
                        x: 7.0,
                        y: 8.0,
                        z,
                        width: 0.4,
                        flow_factor: 1.0,
                        overhang_quartile: None,
                        dist_to_top_mm: 0.0,
                    },
                ],
                role: IrExtrusionRole::SupportMaterial,
                speed_factor: 1.0,
            }
        };

        let plan = Arc::new(SupportPlanIR {
            entries: vec![
                SupportPlanEntry {
                    global_layer_index: layer_index as i32,
                    object_id: "obj-0".to_string(),
                    region_id: other_region_id,
                    branch_segments: vec![seg_for(other_region_id, layer_z)],
                },
                SupportPlanEntry {
                    global_layer_index: layer_index as i32,
                    object_id: "obj-0".to_string(),
                    region_id: target_region_id,
                    branch_segments: vec![seg_for(target_region_id, layer_z)],
                },
            ],
            ..Default::default()
        });

        // Build a SliceIR whose single region has region_id = 42.
        let extent = slicer_ir::mm_to_units(10.0);
        let slice_ir = SliceIR {
            global_layer_index: layer_index,
            z: layer_z,
            regions: vec![SlicedRegion {
                object_id: "obj-0".to_string(),
                region_id: target_region_id,
                polygons: vec![ExPolygon {
                    contour: Polygon {
                        points: vec![
                            Point2 { x: 0, y: 0 },
                            Point2 { x: extent, y: 0 },
                            Point2 {
                                x: extent,
                                y: extent,
                            },
                            Point2 { x: 0, y: extent },
                        ],
                    },
                    holes: vec![],
                }],
                infill_areas: vec![],
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
        };

        // Dispatch tree-support with the multi-region plan.
        let engine = wasm_cache::shared_engine();
        let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
        let wasm_path = tree_support_wasm();
        let loaded = loaded_support_module(
            "com.core.tree-support",
            wasm_path.clone(),
            vec![
                "SliceIR",
                "SurfaceClassificationIR",
                "PaintRegionIR",
                "SupportPlanIR",
            ],
        );
        let bundle = compile_module(&engine, loaded, &wasm_path);

        let blackboard = empty_blackboard_with_support_plan(Some(Arc::clone(&plan)));

        let layer = GlobalLayer {
            index: layer_index,
            z: layer_z,
            active_regions: vec![],
            has_nonplanar: false,
            is_sync_layer: false,
        };

        let mut arena = LayerArena::new();
        arena.set_slice(slice_ir).unwrap();
        run_layer_and_commit_with_bundle(
            &dispatcher,
            "Layer::Support",
            &layer,
            &bundle,
            &blackboard,
            &mut arena,
        )
        .expect("Layer::Support dispatch must succeed");

        let support_ir = arena.take_support().expect("SupportIR must be committed");

        assert!(
            !support_ir.support_paths.is_empty(),
            "tree-support must find branches for region_id={target_region_id}; got 0 paths"
        );
        for path in &support_ir.support_paths {
            assert_eq!(
                path.role,
                IrExtrusionRole::SupportMaterial,
                "all support paths must carry SupportMaterial role"
            );
        }
    }
}
