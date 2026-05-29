//! Host-fallback tests for `execute_paint_segmentation` (Packet-64).
//!
//! Tests verify the guard-based host fallback path for paint-segmentation
//! without loading any `.wasm` module. Source/doc-level tests from the
//! original Packet-43 roundtrip harness are preserved below.
//!
//! Verification: cargo test -p slicer-runtime --test macro_paint_segmentation_output_roundtrip_tdd

#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    BoundingBox3, FacetPaintData, GlobalLayer, LayerPlanIR, MeshIR, ObjectMesh, ObjectSurfaceData,
    PaintLayer, PaintSemantic, PaintValue, Point3, SurfaceClassificationIR, Transform3d,
};
use slicer_runtime::{execute_paint_segmentation, PaintSegmentationError};

// â”€â”€ Host fallback: empty mesh â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

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

// â”€â”€ Host fallback: objects with paint_data produce Material regions â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

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

// â”€â”€ Host fallback: error on missing surface classification â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

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

// â”€â”€ AC-6: legacy comment block removed â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

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
         slicer-macros/src/lib.rs â€” remove it as part of the drain implementation"
    );
    assert!(
        !src.contains("the SDK PaintSegmentationOutput builder operates on an in-Rust tree"),
        "AC-6 FAIL: legacy comment about 'in-Rust tree disconnect' still present â€” remove it"
    );
}

// â”€â”€ AC-7: docs/07 TASK-130 cluster marked done â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

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

// â”€â”€ AC-8: DEV-025 fully closed â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

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

// â”€â”€ AC-9: audit history DEV-025 row complete â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

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

// â”€â”€ Host fallback: malformed facet values error â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

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

// â”€â”€ Host fallback: unpainted object yields empty per_layer â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

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

// â”€â”€ Host fallback: MMU paint data produces non-empty per_layer â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

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
