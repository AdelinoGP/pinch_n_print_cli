//! Host-fallback tests for `execute_paint_segmentation` (Packet-64).
//!
//! Tests verify the guard-based host fallback path for paint-segmentation
//! without loading any `.wasm` module. Source/doc-level tests from the
//! original Packet-43 roundtrip harness are preserved below.
//!
//! Verification: cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd

#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_host::{execute_paint_segmentation, PaintSegmentationError};
use slicer_ir::{
    BoundingBox3, FacetPaintData, GlobalLayer, LayerPlanIR, MeshIR, ObjectMesh, ObjectSurfaceData,
    PaintLayer, PaintSemantic, PaintValue, Point3, SurfaceClassificationIR, Transform3d,
};

// ── AC-1: source-level drain string grep ─────────────────────────────────────

/// AC-1: The PaintSegmentation macro arm body in slicer-macros/src/lib.rs must
/// contain the drain strings `sdk_output.regions()`, `_output.push_paint_region`,
/// and `ModuleError { code: 10, fatal: true }`.
#[test]
fn macro_arm_drains_regions_to_wit() {
    let src = include_str!("../../slicer-macros/src/lib.rs");

    // Use a sentinel that uniquely identifies the arm body (not the WorldGlueKind match).
    // The arm body initialises sdk_output with PaintSegmentationOutput::new().
    let sentinel = "PaintSegmentationOutput::new()";
    let arm_start = src.find(sentinel).expect(
        "slicer-macros must contain PaintSegmentationOutput::new() in PaintSegmentation arm",
    );

    // Bound the arm: take the next 4000 chars as a proxy for the arm body.
    let arm_body = &src[arm_start..arm_start + src[arm_start..].len().min(4000)];

    assert!(
        arm_body.contains("sdk_output.regions()"),
        "PaintSegmentation arm must call sdk_output.regions() to drain; arm snippet:\n{}",
        &arm_body[..arm_body.len().min(500)]
    );
    assert!(
        arm_body.contains("_output.push_paint_region"),
        "PaintSegmentation arm must call _output.push_paint_region; arm snippet:\n{}",
        &arm_body[..arm_body.len().min(500)]
    );
    // The source uses multi-line struct literal, so check each field separately.
    assert!(
        arm_body.contains("code: 10") && arm_body.contains("fatal: true"),
        "PaintSegmentation arm must surface ModuleError with code: 10 and fatal: true on push failure"
    );
}

// ── Host fallback: empty mesh ────────────────────────────────────────────────

/// `execute_paint_segmentation` with no objects produces empty `per_layer`.
#[test]
fn host_fallback_empty_mesh_yields_empty_per_layer() {
    let mesh = Arc::new(MeshIR::default());
    let sc = Arc::new(SurfaceClassificationIR::default());
    let lp = Arc::new(LayerPlanIR::default());

    let ir = execute_paint_segmentation(mesh, sc, lp, true)
        .expect("host fallback must succeed for empty mesh");
    assert!(ir.per_layer.is_empty());
}

// ── Host fallback: objects with paint_data produce Material regions ───────────

/// `execute_paint_segmentation` processes FacetPaintData into PaintRegionIR
/// with the correct PaintSemantic::Material region.
#[test]
fn host_fallback_painted_object_produces_material_region() {
    let object = ObjectMesh {
        id: "obj-a".into(),
        mesh: slicer_ir::IndexedTriangleSet {
            vertices: vec![
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3 {
                    x: 1.0,
                    y: 0.0,
                    z: 0.2,
                },
                Point3 {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
            ],
            indices: vec![0, 1, 2],
        },
        transform: Transform3d {
            matrix: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        },
        config: slicer_ir::ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: Vec::new(),
        paint_data: Some(FacetPaintData {
            layers: vec![PaintLayer {
                semantic: PaintSemantic::Material,
                facet_values: vec![Some(PaintValue::ToolIndex(0))],
                strokes: Vec::new(),
            }],
        }),
        world_z_extent: Some((0.0, 0.2)),
    };

    let mesh = Arc::new(MeshIR {
        objects: vec![object],
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
        ..Default::default()
    });
    let sc = Arc::new(SurfaceClassificationIR {
        per_object: HashMap::from([("obj-a".into(), ObjectSurfaceData::default())]),
        ..Default::default()
    });
    let lp = Arc::new(LayerPlanIR {
        global_layers: vec![GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: Vec::new(),
            has_nonplanar: false,
            is_sync_layer: true,
        }],
        object_participation: HashMap::from([(
            "obj-a".into(),
            vec![slicer_ir::ObjectLayerRef {
                local_layer_index: 0,
                global_layer_index: 0,
                effective_layer_height: 0.2,
            }],
        )]),
        ..Default::default()
    });

    let ir = execute_paint_segmentation(mesh, sc, lp, true).expect("host fallback must succeed");
    assert!(
        !ir.per_layer.is_empty(),
        "per_layer must be non-empty for painted model"
    );
    let has_material = ir
        .per_layer
        .values()
        .any(|lm| lm.semantic_regions.contains_key(&PaintSemantic::Material));
    assert!(has_material, "must contain Material semantic region");
}

// ── Host fallback: error on missing surface classification ─────────────────────

/// Missing `SurfaceClassificationIR` data for a painted object surfaces
/// as `PaintSegmentationError::MissingSurfaceObject`.
#[test]
fn host_fallback_missing_surface_errors() {
    let object = ObjectMesh {
        id: "obj-a".into(),
        mesh: slicer_ir::IndexedTriangleSet {
            vertices: vec![
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3 {
                    x: 1.0,
                    y: 0.0,
                    z: 0.2,
                },
                Point3 {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
            ],
            indices: vec![0, 1, 2],
        },
        transform: Transform3d {
            matrix: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        },
        config: slicer_ir::ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: Vec::new(),
        paint_data: Some(FacetPaintData {
            layers: vec![PaintLayer {
                semantic: PaintSemantic::Material,
                facet_values: vec![Some(PaintValue::ToolIndex(2))],
                strokes: Vec::new(),
            }],
        }),
        world_z_extent: Some((0.0, 0.2)),
    };

    let mesh = Arc::new(MeshIR {
        objects: vec![object],
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
        ..Default::default()
    });
    let sc = Arc::new(SurfaceClassificationIR::default());
    let lp = Arc::new(LayerPlanIR::default());

    let err =
        execute_paint_segmentation(mesh, sc, lp, true).expect_err("missing surface must error");
    match err {
        PaintSegmentationError::MissingSurfaceObject { object_id } => {
            assert_eq!(object_id, "obj-a");
        }
        other => panic!("expected MissingSurfaceObject, got: {other:?}"),
    }
}

// ── AC-6: legacy comment block removed ───────────────────────────────────────

/// AC-6: The legacy disconnect comment block must be removed from
/// crates/slicer-macros/src/lib.rs before packet closure.
///
/// RED today: the legacy comment is still present.
#[test]
fn legacy_comment_block_removed() {
    let src = include_str!("../../slicer-macros/src/lib.rs");
    assert!(
        !src.contains("Same disconnect as MeshSegmentation"),
        "AC-6 FAIL: legacy comment 'Same disconnect as MeshSegmentation' still present in \
         slicer-macros/src/lib.rs — remove it as part of the drain implementation"
    );
    assert!(
        !src.contains("the SDK PaintSegmentationOutput builder operates on an in-Rust tree"),
        "AC-6 FAIL: legacy comment about 'in-Rust tree disconnect' still present — remove it"
    );
}

// ── AC-7: docs/07 TASK-130 cluster marked done ───────────────────────────────

/// AC-7: docs/07_implementation_status.md must show TASK-130, TASK-130a, TASK-130b as [x].
/// The blocker list section must NOT reference TASK-130a or TASK-130b.
///
/// RED today: TASK-130 cluster not yet done.
#[test]
fn docs_07_marks_130_cluster_done() {
    let src = include_str!("../../../docs/07_implementation_status.md");

    // Each TASK-130 row must be [x].
    for task in &["TASK-130", "TASK-130a", "TASK-130b"] {
        // Find the line containing this task ID.
        let line = src
            .lines()
            .find(|l| l.contains(task))
            .unwrap_or_else(|| panic!("AC-7: {task} line not found in docs/07"));
        assert!(
            line.contains("[x]"),
            "AC-7 FAIL: {task} line not marked [x]: '{line}'"
        );
    }

    // Blocker section must not list TASK-130a or TASK-130b.
    let blocker_section_start = src
        .find("blocker")
        .or_else(|| src.find("Blocker"))
        .or_else(|| src.find("BLOCKER"));
    if let Some(start) = blocker_section_start {
        let blocker_slice = &src[start..start + src[start..].len().min(2000)];
        assert!(
            !blocker_slice.contains("TASK-130a"),
            "AC-7 FAIL: TASK-130a still referenced in blocker section"
        );
        assert!(
            !blocker_slice.contains("TASK-130b"),
            "AC-7 FAIL: TASK-130b still referenced in blocker section"
        );
    }
}

// ── AC-8: DEV-025 fully closed ───────────────────────────────────────────────

/// AC-8: docs/DEVIATION_LOG.md must show DEV-025 mismatch-3 closed-by-Packet-43
/// and the DEV-025 overall status must be `closed`.
///
/// RED today: not yet updated.
#[test]
fn dev_025_fully_closed() {
    let src = include_str!("../../../docs/DEVIATION_LOG.md");

    // Find DEV-025 section.
    let dev025_start = src
        .find("DEV-025")
        .expect("AC-8: DEV-025 must exist in DEVIATION_LOG.md");
    let dev025_slice = &src[dev025_start..dev025_start + src[dev025_start..].len().min(3000)];

    assert!(
        dev025_slice.contains("closed-by-Packet-43"),
        "AC-8 FAIL: DEV-025 mismatch-3 must show 'closed-by-Packet-43'"
    );
    assert!(
        dev025_slice.contains("status") && dev025_slice.contains("closed"),
        "AC-8 FAIL: DEV-025 status line must show 'closed'"
    );
}

// ── AC-9: audit history DEV-025 row complete ─────────────────────────────────

/// AC-9: docs/14_deviation_audit_history.md DEV-025 row must reference
/// TASK-128a, TASK-128b, TASK-130, TASK-130a, TASK-130b, TASK-130c.
///
/// RED today: not yet updated.
#[test]
fn dev_025_audit_history_complete() {
    let src = include_str!("../../../docs/14_deviation_audit_history.md");

    let dev025_start = src
        .find("DEV-025")
        .expect("AC-9: DEV-025 must exist in 14_deviation_audit_history.md");
    let dev025_slice = &src[dev025_start..dev025_start + src[dev025_start..].len().min(2000)];

    for task in &[
        "TASK-128a",
        "TASK-128b",
        "TASK-130",
        "TASK-130a",
        "TASK-130b",
        "TASK-130c",
    ] {
        assert!(
            dev025_slice.contains(task),
            "AC-9 FAIL: DEV-025 audit row missing {task}"
        );
    }
}

// ── Host fallback: malformed facet values error ───────────────────────────────

/// Mismatched facet value count vs triangle count surfaces as
/// `PaintSegmentationError::MalformedFacetValues`.
#[test]
fn host_fallback_malformed_facet_values_errors() {
    let object = ObjectMesh {
        id: "obj".into(),
        mesh: slicer_ir::IndexedTriangleSet {
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
        transform: Transform3d {
            matrix: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        },
        config: slicer_ir::ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: Vec::new(),
        paint_data: Some(FacetPaintData {
            layers: vec![PaintLayer {
                semantic: PaintSemantic::Material,
                // 1 triangle but 2 facet_values -> mismatch
                facet_values: vec![
                    Some(PaintValue::ToolIndex(1)),
                    Some(PaintValue::ToolIndex(2)),
                ],
                strokes: Vec::new(),
            }],
        }),
        world_z_extent: Some((0.0, 0.2)),
    };

    let mesh = Arc::new(MeshIR {
        objects: vec![object],
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
        ..Default::default()
    });
    let sc = Arc::new(SurfaceClassificationIR {
        per_object: HashMap::from([("obj".into(), ObjectSurfaceData::default())]),
        ..Default::default()
    });
    let lp = Arc::new(LayerPlanIR {
        global_layers: vec![GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: Vec::new(),
            has_nonplanar: false,
            is_sync_layer: true,
        }],
        object_participation: HashMap::from([(
            "obj".into(),
            vec![slicer_ir::ObjectLayerRef {
                local_layer_index: 0,
                global_layer_index: 0,
                effective_layer_height: 0.2,
            }],
        )]),
        ..Default::default()
    });

    let err = execute_paint_segmentation(mesh, sc, lp, true)
        .expect_err("malformed facet values must error");
    match err {
        PaintSegmentationError::MalformedFacetValues {
            object_id,
            expected_facets,
            actual_facet_values,
            ..
        } => {
            assert_eq!(object_id, "obj");
            assert_eq!(expected_facets, 1, "1 triangle = 1 facet");
            assert_eq!(actual_facet_values, 2, "2 values provided");
        }
        other => panic!("expected MalformedFacetValues, got: {other:?}"),
    }
}

// ── Host fallback: unpainted object yields empty per_layer ─────────────────────

/// Object with no paint_data produces empty per_layer (no error).
#[test]
fn host_fallback_unpainted_object_yields_empty_per_layer() {
    let object = ObjectMesh {
        id: "obj".into(),
        mesh: slicer_ir::IndexedTriangleSet {
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
        transform: Transform3d {
            matrix: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        },
        config: slicer_ir::ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: Vec::new(),
        paint_data: None,
        world_z_extent: None,
    };

    let mesh = Arc::new(MeshIR {
        objects: vec![object],
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
        ..Default::default()
    });
    let sc = Arc::new(SurfaceClassificationIR {
        per_object: HashMap::from([("obj".into(), ObjectSurfaceData::default())]),
        ..Default::default()
    });
    let lp = Arc::new(LayerPlanIR::default());

    let ir =
        execute_paint_segmentation(mesh, sc, lp, true).expect("unpainted object must not error");
    assert!(ir.per_layer.is_empty());
}

// ── Negative-2: no early return bypasses drain ───────────────────────────────

/// Negative-2: Within the PaintSegmentation arm body in slicer-macros/src/lib.rs,
/// there must be zero occurrences of `return Ok(())` that appear BEFORE the
/// `for` loop over `sdk_output.regions()`.
///
/// RED today: there is no drain loop yet, so any `return Ok(())` before it counts as a bypass.
#[test]
fn no_early_return_bypasses_drain() {
    let src = include_str!("../../slicer-macros/src/lib.rs");

    let sentinel = "PrePass::PaintSegmentation";
    let arm_start = src
        .find(sentinel)
        .expect("must contain PrePass::PaintSegmentation arm sentinel");

    // Extract a bounded arm body (4000 chars max).
    let arm_end = arm_start + src[arm_start..].len().min(4000);
    let arm_body = &src[arm_start..arm_end];

    // Find position of the drain loop.
    let loop_pos = arm_body.find("for");

    // Count `return Ok(())` occurrences before the loop.
    let early_returns: usize = if let Some(loop_at) = loop_pos {
        let pre_loop = &arm_body[..loop_at];
        pre_loop.matches("return Ok(())").count()
    } else {
        // No loop found yet — any return Ok(()) in arm is a potential bypass.
        arm_body.matches("return Ok(())").count()
    };

    assert_eq!(
        early_returns, 0,
        "Neg-2 FAIL: found {early_returns} early `return Ok(())` before drain loop in \
         PaintSegmentation arm — these would bypass the drain"
    );
}

// ── Host fallback: MMU paint data produces non-empty per_layer ────────────────

/// `execute_paint_segmentation` with MMU paint data produces non-empty
/// per_layer containing Material regions.
#[test]
fn host_fallback_mmu_paint_data_yields_material_regions() {
    let object = ObjectMesh {
        id: "obj-a".into(),
        mesh: slicer_ir::IndexedTriangleSet {
            vertices: vec![
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3 {
                    x: 1.0,
                    y: 0.0,
                    z: 0.2,
                },
                Point3 {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
            ],
            indices: vec![0, 1, 2],
        },
        transform: Transform3d {
            matrix: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        },
        config: slicer_ir::ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: Vec::new(),
        paint_data: Some(FacetPaintData {
            layers: vec![PaintLayer {
                semantic: PaintSemantic::Material,
                facet_values: vec![Some(PaintValue::ToolIndex(0))],
                strokes: Vec::new(),
            }],
        }),
        world_z_extent: Some((0.0, 0.2)),
    };

    let mesh = Arc::new(MeshIR {
        objects: vec![object],
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
        ..Default::default()
    });
    let sc = Arc::new(SurfaceClassificationIR {
        per_object: HashMap::from([("obj-a".into(), ObjectSurfaceData::default())]),
        ..Default::default()
    });
    let lp = Arc::new(LayerPlanIR {
        global_layers: vec![GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: Vec::new(),
            has_nonplanar: false,
            is_sync_layer: true,
        }],
        object_participation: HashMap::from([(
            "obj-a".into(),
            vec![slicer_ir::ObjectLayerRef {
                local_layer_index: 0,
                global_layer_index: 0,
                effective_layer_height: 0.2,
            }],
        )]),
        ..Default::default()
    });

    let ir = execute_paint_segmentation(mesh, sc, lp, true).expect("host fallback must succeed");
    assert!(
        !ir.per_layer.is_empty(),
        "MMU paint data must produce non-empty per_layer"
    );
    let has_material = ir
        .per_layer
        .values()
        .any(|lm| lm.semantic_regions.contains_key(&PaintSemantic::Material));
    assert!(
        has_material,
        "per_layer must contain Material semantic regions"
    );
}
