//! Modifier-identity and painted-id rejection controls for `H-REGIONMAP`.
//!
//! These are the caller-level obligations the packet lists separately from the
//! plain error-field checks in `controls::rejection_controls`: an unsupported
//! base parent, identical-footprint grouping, a real distinct-footprint
//! `modifier_sub_region_id` collision, and the painted-id contract on the real
//! region-map and segmentation callers.

use std::collections::{BTreeMap, HashMap};

use super::model::{run_with_cap, Case, ObjectAxis, ObjectCase, AXIS_POOL};
use slicer_core::algos::paint_segmentation::{execute_paint_segmentation, PaintSegmentationError};
use slicer_core::algos::region_mapping::{
    execute_region_mapping_with_cap, RegionMappingError, RegionMappingPlanProjection,
};
use slicer_ir::{
    modifier_sub_region_id, ActiveRegion, ConfigDelta, ExPolygon, FacetPaintData, GlobalLayer,
    IndexedTriangleSet, LayerPlanIR, MeshIR, ModifierKind, ModifierVolume, ModuleInvocation,
    ObjectMesh, PaintLayer, PaintSemantic, PaintValue, Point2, Point3, Polygon, RegionMapIR,
    ResolvedConfig, SemVer, SliceIR, SlicedRegion, StageId, Transform3d,
};

const OBJECT_ID: &str = "obj_mod";
const BASE_REGION_ID: u64 = 7;

fn unit_matrix() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    }
}

/// An axis-aligned cube `[x0,x1] x [y0,y1] x [z0,z1]`.
fn cube_mesh(x0: f32, y0: f32, x1: f32, y1: f32, z0: f32, z1: f32) -> IndexedTriangleSet {
    let v = |x: f32, y: f32, z: f32| Point3 { x, y, z };
    IndexedTriangleSet {
        vertices: vec![
            v(x0, y0, z0),
            v(x1, y0, z0),
            v(x1, y1, z0),
            v(x0, y1, z0),
            v(x0, y0, z1),
            v(x1, y0, z1),
            v(x1, y1, z1),
            v(x0, y1, z1),
        ],
        indices: vec![
            0, 2, 1, 0, 3, 2, // bottom
            4, 5, 6, 4, 6, 7, // top
            0, 1, 5, 0, 5, 4, // front
            1, 2, 6, 1, 6, 5, // right
            2, 3, 7, 2, 7, 6, // back
            3, 0, 4, 3, 4, 7, // left
        ],
    }
}

fn parameter_modifier(id: &str, mesh: IndexedTriangleSet, priority: u32) -> ModifierVolume {
    ModifierVolume::new(
        id.to_owned(),
        mesh,
        ConfigDelta {
            fields: HashMap::new(),
        },
        priority,
        ModifierKind::ParameterModifier,
    )
}

/// One layer, one object, one base region, plus the supplied modifier volumes.
fn run_modifier_case(
    region_id: u64,
    modifiers: Vec<ModifierVolume>,
    cap: usize,
) -> Result<RegionMapIR, RegionMappingError> {
    let object = ObjectMesh {
        id: OBJECT_ID.to_owned(),
        mesh: IndexedTriangleSet::default(),
        transform: unit_matrix(),
        modifier_volumes: modifiers,
        ..ObjectMesh::default()
    };
    let plan = LayerPlanIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layers: vec![GlobalLayer {
            index: 0,
            z: 0.5,
            active_regions: vec![ActiveRegion {
                object_id: OBJECT_ID.to_owned(),
                region_id,
                resolved_config: ResolvedConfig::default(),
                effective_layer_height: 0.2,
                ..ActiveRegion::default()
            }],
            ..GlobalLayer::default()
        }],
        object_participation: HashMap::new(),
    };
    let stage_invocations: Vec<(StageId, Vec<ModuleInvocation>)> = Vec::new();
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    execute_region_mapping_with_cap(
        &plan,
        &projection,
        &BTreeMap::new(),
        &BTreeMap::new(),
        &[object],
        cap,
    )
}

/// Identical footprints group; an unsupported base parent and a real
/// distinct-footprint id collision are rejected.
pub fn modifier_identity_controls() {
    // --- Identical footprints group (one minted sub-region, no error) ------
    let twin = cube_mesh(2.0, 2.0, 3.0, 3.0, 0.0, 1.0);
    let grouped = run_modifier_case(
        BASE_REGION_ID,
        vec![
            parameter_modifier("mod_a", twin.clone(), 0),
            parameter_modifier("mod_b", twin, 1),
        ],
        64,
    )
    .expect("identical footprints must group, not collide");
    assert_eq!(
        grouped.entries.len(),
        2,
        "one base entry plus one grouped sub-region entry"
    );
    assert_eq!(
        grouped
            .entries
            .keys()
            .filter(|key| slicer_ir::is_modifier_namespace_id(key.region_id))
            .count(),
        1,
        "identical footprints share exactly one minted sub-region id"
    );

    // --- Unsupported base parent ------------------------------------------
    // `modifier_sub_region_id_fits` rejects any parent above ~9.2e12; this one
    // is far outside it, so the kernel must refuse before minting.
    let too_large_parent = 1u64 << 50;
    assert!(!slicer_ir::modifier_sub_region_id_fits(too_large_parent));
    let error = run_modifier_case(
        too_large_parent,
        vec![parameter_modifier(
            "mod_a",
            cube_mesh(2.0, 2.0, 3.0, 3.0, 0.0, 1.0),
            0,
        )],
        64,
    )
    .expect_err("an unsupported base parent must be rejected before minting");
    match error {
        RegionMappingError::ModifierParentRegionIdOutOfRange { key } => {
            assert_eq!(key.region_id, too_large_parent);
            assert!(key.variant_chain.is_empty());
            assert_eq!(key.object_id, OBJECT_ID);
        }
        other => panic!("expected ModifierParentRegionIdOutOfRange, got {other:?}"),
    }

    // --- A painted parent whose minted id no longer fits ------------------
    // `paint_variant_region_id` succeeds for this base (its payload stays below
    // 2^63) while `modifier_sub_region_id_fits` rejects the minted parent, so
    // the painted-parent branch must report the OUT-OF-RANGE error rather than
    // panicking or silently minting.
    // The base must still fit (so the plain-parent guard does not fire) while
    // the minted painted parent — one paint stride higher — does not. Any base
    // in `(9.2e6, 9.2e12)` is such a witness: its own id fits, and
    // `base * 1_000_000` lands past the modifier namespace ceiling.
    let painted_edge_base: u64 = 10_000_000;
    let painted_chain = vec![("material".to_owned(), PaintValue::ToolIndex(0))];
    let painted_edge_parent = slicer_core::algos::paint_segmentation::paint_variant_region_id(
        painted_edge_base,
        &painted_chain,
    );
    assert!(
        slicer_ir::modifier_sub_region_id_fits(painted_edge_base),
        "the base parent itself must fit, or the plain-parent guard fires first"
    );
    assert!(
        !slicer_ir::modifier_sub_region_id_fits(painted_edge_parent),
        "the minted painted parent must not fit"
    );
    let plan = LayerPlanIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layers: vec![GlobalLayer {
            index: 0,
            z: 0.5,
            active_regions: vec![ActiveRegion {
                object_id: OBJECT_ID.to_owned(),
                region_id: painted_edge_base,
                resolved_config: ResolvedConfig::default(),
                effective_layer_height: 0.2,
                ..ActiveRegion::default()
            }],
            ..GlobalLayer::default()
        }],
        object_participation: HashMap::new(),
    };
    let painted_object = ObjectMesh {
        id: OBJECT_ID.to_owned(),
        mesh: IndexedTriangleSet::default(),
        transform: unit_matrix(),
        modifier_volumes: vec![parameter_modifier(
            "mod_a",
            cube_mesh(2.0, 2.0, 3.0, 3.0, 0.0, 1.0),
            0,
        )],
        paint_data: Some(FacetPaintData {
            layers: vec![PaintLayer {
                semantic: PaintSemantic::Material,
                facet_values: vec![Some(PaintValue::ToolIndex(0))],
                strokes: Vec::new(),
            }],
        }),
        ..ObjectMesh::default()
    };
    let aggregated: BTreeMap<String, slicer_ir::slice_ir::AggregatedRegionSplitEntry> =
        BTreeMap::from([(
            "material".to_owned(),
            slicer_ir::slice_ir::AggregatedRegionSplitEntry {
                priority: 100,
                value_type: slicer_ir::RegionSplitValueType::ToolIndex,
                declaring_modules: Vec::new(),
            },
        )]);
    let stage_invocations: Vec<(StageId, Vec<ModuleInvocation>)> = Vec::new();
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    let error = execute_region_mapping_with_cap(
        &plan,
        &projection,
        &BTreeMap::new(),
        &aggregated,
        &[painted_object],
        64,
    )
    .expect_err("a painted parent whose minted id does not fit must be rejected");
    match error {
        RegionMappingError::ModifierParentRegionIdOutOfRange { key } => {
            assert_eq!(key.object_id, OBJECT_ID);
            assert!(
                !key.variant_chain.is_empty(),
                "the reported parent is the PAINTED parent, so it carries the chain"
            );
            assert_eq!(
                key.region_id, painted_edge_parent,
                "the reported parent is the minted painted id"
            );
        }
        other => panic!("expected ModifierParentRegionIdOutOfRange, got {other:?}"),
    }

    // --- A real distinct-footprint minted-id collision --------------------
    // Retained concrete coordinates (found once by a birthday search over real
    // sliced cube footprints; the payload space is `MODIFIER_VARIANT_REGION_ID_STRIDE - 1`
    // = 1_000_002). These two cubes are DIFFERENT footprints whose minted ids
    // are equal, so the kernel must refuse to overwrite one with the other.
    // The mint helper is used only to *state* the stimulus; the collision
    // contract asserted below is the kernel's own.
    let first_cube = cube_mesh(20.87, 2.0, 21.37, 2.5, 0.0, 1.0);
    let second_cube = cube_mesh(24.32, 2.0, 24.82, 2.5, 0.0, 1.0);
    let first_footprint = slicer_core::slice_mesh_ex(&first_cube, &[0.5])
        .into_iter()
        .next()
        .unwrap_or_default();
    let second_footprint = slicer_core::slice_mesh_ex(&second_cube, &[0.5])
        .into_iter()
        .next()
        .unwrap_or_default();
    assert_eq!(
        first_footprint,
        vec![ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 {
                        x: 213_700,
                        y: 25_000
                    },
                    Point2 {
                        x: 208_700,
                        y: 25_000
                    },
                    Point2 {
                        x: 208_700,
                        y: 20_000
                    },
                    Point2 {
                        x: 213_700,
                        y: 20_000
                    },
                ],
            },
            holes: Vec::new(),
        }],
        "the retained first footprint must not drift"
    );
    assert_eq!(
        second_footprint,
        vec![ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 {
                        x: 248_200,
                        y: 25_000
                    },
                    Point2 {
                        x: 243_200,
                        y: 25_000
                    },
                    Point2 {
                        x: 243_200,
                        y: 20_000
                    },
                    Point2 {
                        x: 248_200,
                        y: 20_000
                    },
                ],
            },
            holes: Vec::new(),
        }],
        "the retained second footprint must not drift"
    );
    assert_ne!(first_footprint, second_footprint, "distinct footprints");
    assert_eq!(
        modifier_sub_region_id(BASE_REGION_ID, OBJECT_ID, &first_footprint),
        modifier_sub_region_id(BASE_REGION_ID, OBJECT_ID, &second_footprint),
        "the retained pair must still collide"
    );
    let error = run_modifier_case(
        BASE_REGION_ID,
        vec![
            parameter_modifier("mod_first", first_cube, 0),
            parameter_modifier("mod_second", second_cube, 1),
        ],
        4096,
    )
    .expect_err("distinct footprints with one minted id must not silently overwrite");
    match error {
        RegionMappingError::DuplicateRegionKey { key } => {
            assert!(slicer_ir::is_modifier_namespace_id(key.region_id));
            assert_eq!(key.object_id, OBJECT_ID);
        }
        other => panic!("expected DuplicateRegionKey, got {other:?}"),
    }
}

/// The painted-id contract, established on the real callers rather than
/// asserted as hash injectivity.
///
/// `paint_variant_region_id`'s material fast path maps `ToolIndex(0)` and
/// `ToolIndex(999_999)` to the same payload for the same base. The **region
/// map** keeps both entries because its keys include the full chain; the
/// **segmentation** caller rejects the second mint for the same object.
pub fn painted_id_contract_controls() {
    // --- The region map keeps both colliding chains ------------------------
    let case = Case {
        layers: 1,
        regions_per_object: 1,
        axes: vec![AXIS_POOL[0]],
        objects: vec![ObjectCase {
            id: "obj_0".to_owned(),
            axes: vec![ObjectAxis {
                axis: 0,
                values: vec![PaintValue::ToolIndex(0), PaintValue::ToolIndex(999_999)],
            }],
        }],
    };
    let map = run_with_cap(&case, 64)
        .expect("the region map must keep both colliding painted chains as distinct keys");
    let chains: Vec<Vec<(String, PaintValue)>> = map
        .entries
        .keys()
        .map(|key| key.variant_chain.clone())
        .collect();
    for expected in [
        vec![("material".to_owned(), PaintValue::ToolIndex(0))],
        vec![("material".to_owned(), PaintValue::ToolIndex(999_999))],
    ] {
        assert!(
            chains.contains(&expected),
            "the map must retain {expected:?}; got {chains:?}"
        );
    }

    // --- Segmentation rejects the same-object collision --------------------
    let mut indices = Vec::new();
    for corner in 0..4u32 {
        let a = corner;
        let b = (corner + 1) % 4;
        indices.extend([a, b, b + 4, a, b + 4, a + 4]);
    }
    // Two wall halves carry the two colliding tool indices, so both painted
    // chains have real geometry and both mints are attempted.
    let facet_values: Vec<Option<PaintValue>> = (0..8)
        .map(|facet| {
            Some(if facet < 4 {
                PaintValue::ToolIndex(0)
            } else {
                PaintValue::ToolIndex(999_999)
            })
        })
        .collect();
    let mesh = MeshIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        objects: vec![ObjectMesh {
            id: "obj_0".to_owned(),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    Point3 {
                        x: 2.0,
                        y: 2.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 4.0,
                        y: 2.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 4.0,
                        y: 4.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 2.0,
                        y: 4.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 2.0,
                        y: 2.0,
                        z: 1.0,
                    },
                    Point3 {
                        x: 4.0,
                        y: 2.0,
                        z: 1.0,
                    },
                    Point3 {
                        x: 4.0,
                        y: 4.0,
                        z: 1.0,
                    },
                    Point3 {
                        x: 2.0,
                        y: 4.0,
                        z: 1.0,
                    },
                ],
                indices,
            },
            transform: unit_matrix(),
            paint_data: Some(FacetPaintData {
                layers: vec![PaintLayer {
                    semantic: PaintSemantic::Material,
                    facet_values,
                    strokes: Vec::new(),
                }],
            }),
            ..ObjectMesh::default()
        }],
        build_volume: slicer_ir::BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 200.0,
                y: 200.0,
                z: 200.0,
            },
        },
    };
    let slice = SliceIR {
        global_layer_index: 0,
        z: 0.5,
        regions: vec![SlicedRegion {
            object_id: "obj_0".to_owned(),
            region_id: 0,
            polygons: vec![ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2::from_mm(2.0, 2.0),
                        Point2::from_mm(4.0, 2.0),
                        Point2::from_mm(4.0, 4.0),
                        Point2::from_mm(2.0, 4.0),
                    ],
                },
                holes: Vec::new(),
            }],
            ..SlicedRegion::default()
        }],
        ..SliceIR::default()
    };
    let error = execute_paint_segmentation(
        std::sync::Arc::new(mesh),
        std::sync::Arc::new(vec![slice]),
        std::sync::Arc::new(map),
    )
    .expect_err("the segmentation caller must reject the colliding painted ids");
    match error {
        PaintSegmentationError::PaintVariantRegionIdCollision {
            object_id,
            region_id,
            first_chain,
            second_chain,
        } => {
            assert_eq!(object_id, "obj_0");
            assert_eq!(region_id, 1, "both material tool values mint payload 1");
            let pair = [first_chain, second_chain];
            assert!(
                pair.contains(&vec![("material".to_owned(), PaintValue::ToolIndex(0))])
                    && pair.contains(&vec![(
                        "material".to_owned(),
                        PaintValue::ToolIndex(999_999)
                    )]),
                "the error must name both colliding chains, got {pair:?}"
            );
        }
        other => panic!("expected PaintVariantRegionIdCollision, got {other:?}"),
    }
}
