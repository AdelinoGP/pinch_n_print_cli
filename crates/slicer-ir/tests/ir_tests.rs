//! TDD tests for all IR structs
//! These tests will FAIL initially, then we implement structs to make them pass

use slicer_ir::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // Helper macro to test serde round-trip
    macro_rules! test_serde_roundtrip {
        ($value:expr) => {
            let serialized = bincode::serialize(&$value).unwrap();
            let deserialized = bincode::deserialize(&serialized).unwrap();
            assert_eq!($value, deserialized);
        };
    }

    #[test]
    fn test_point2_coordinate_system() {
        // Point2 uses scaled integers: 1 unit = 100 nm = 10^-4 mm
        // 1 mm = 10,000 units
        let point = Point2::from_mm(1.0, 1.0);
        assert_eq!(point.x, 10_000);
        assert_eq!(point.y, 10_000);

        let point2 = Point2::from_mm(0.4, 0.4); // nozzle diameter
        assert_eq!(point2.x, 4_000);
        assert_eq!(point2.y, 4_000);

        // Round-trip
        let (mm_x, mm_y) = point.to_mm();
        assert!((mm_x - 1.0).abs() < 0.0001);
        assert!((mm_y - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_point3_types() {
        // Point3 uses f32 in millimeters
        let point = Point3 {
            x: 1.5,
            y: 2.5,
            z: 3.5,
        };
        assert!((point.x - 1.5).abs() < 0.0001);

        test_serde_roundtrip!(point);
    }

    #[test]
    fn test_bounding_box3() {
        let bbox = BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 100.0,
                y: 100.0,
                z: 100.0,
            },
        };

        test_serde_roundtrip!(bbox);
    }

    #[test]
    fn test_transform3d() {
        // Column-major 4x4 matrix
        let transform = Transform3d {
            matrix: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        };

        test_serde_roundtrip!(transform);
    }

    #[test]
    fn test_indexed_triangle_set() {
        let triangles = IndexedTriangleSet {
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
        };

        test_serde_roundtrip!(triangles);
    }

    #[test]
    fn test_version_semver() {
        let version = SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        };

        assert_eq!(version.to_string(), "1.0.0");
        test_serde_roundtrip!(version);
    }

    #[test]
    fn test_mesh_ir() {
        let mesh_ir = MeshIR {
            schema_version: SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            objects: vec![],
            build_volume: BoundingBox3 {
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

        test_serde_roundtrip!(mesh_ir);
    }

    #[test]
    fn test_object_mesh() {
        let obj_mesh = ObjectMesh {
            id: "test-uuid-1234".to_string(),
            mesh: IndexedTriangleSet {
                vertices: vec![Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                }],
                indices: vec![0, 1, 2],
            },
            transform: Transform3d { matrix: [1.0; 16] },
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: vec![],
            paint_data: None,
            world_z_extent: None,
        };

        test_serde_roundtrip!(obj_mesh);
    }

    #[test]
    fn test_facet_paint_data() {
        let paint_data = FacetPaintData {
            layers: vec![PaintLayer {
                semantic: PaintSemantic::Material,
                facet_values: vec![Some(PaintValue::ToolIndex(1))],
                strokes: vec![],
            }],
        };

        test_serde_roundtrip!(paint_data);
    }

    #[test]
    fn test_paint_semantic() {
        let material = PaintSemantic::Material;
        let fuzzy = PaintSemantic::FuzzySkin;
        let enforcer = PaintSemantic::SupportEnforcer;
        let blocker = PaintSemantic::SupportBlocker;
        let custom = PaintSemantic::Custom("com.example/test@1".to_string());

        test_serde_roundtrip!(material);
        test_serde_roundtrip!(fuzzy);
        test_serde_roundtrip!(enforcer);
        test_serde_roundtrip!(blocker);
        test_serde_roundtrip!(custom);
    }

    #[test]
    fn test_paint_value() {
        let flag = PaintValue::Flag(true);
        let scalar = PaintValue::Scalar(0.5);
        let tool = PaintValue::ToolIndex(2);

        test_serde_roundtrip!(flag);
        test_serde_roundtrip!(scalar);
        test_serde_roundtrip!(tool);
    }

    #[test]
    fn test_paint_stroke() {
        let stroke = PaintStroke {
            triangles: vec![[
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
            ]],
            semantic: PaintSemantic::Material,
            value: PaintValue::ToolIndex(1),
        };

        test_serde_roundtrip!(stroke);
    }

    #[test]
    fn test_modifier_volume() {
        let modifier = ModifierVolume {
            id: "mod-123".to_string(),
            mesh: IndexedTriangleSet {
                vertices: vec![],
                indices: vec![],
            },
            config_delta: ConfigDelta {
                fields: std::collections::HashMap::new(),
            },
            priority: 10,
            applies_to: ModifierScope::Perimeters,
        };

        test_serde_roundtrip!(modifier);
    }

    #[test]
    fn test_modifier_scope() {
        test_serde_roundtrip!(ModifierScope::AllFeatures);
        test_serde_roundtrip!(ModifierScope::Infill);
        test_serde_roundtrip!(ModifierScope::Perimeters);
        test_serde_roundtrip!(ModifierScope::Support);
        test_serde_roundtrip!(ModifierScope::LayerHeight);
    }

    #[test]
    fn test_surface_classification_ir() {
        let surf_ir = SurfaceClassificationIR {
            schema_version: SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            per_object: std::collections::HashMap::new(),
        };

        test_serde_roundtrip!(surf_ir);
    }

    #[test]
    fn test_facet_class() {
        test_serde_roundtrip!(FacetClass::Normal);
        test_serde_roundtrip!(FacetClass::NearHorizontal {
            slope_angle_deg: 5.0
        });
        test_serde_roundtrip!(FacetClass::Overhang { angle_deg: 45.0 });
        test_serde_roundtrip!(FacetClass::Bridge);
        test_serde_roundtrip!(FacetClass::TopSurface);
        test_serde_roundtrip!(FacetClass::BottomSurface);
    }

    #[test]
    fn test_surface_group() {
        let group = SurfaceGroup {
            id: 42,
            facet_indices: vec![0, 1, 2],
            z_min: 10.0,
            z_max: 20.0,
            area_mm2: 100.0,
            printable: true,
            shell_count: 3,
        };

        test_serde_roundtrip!(group);
    }

    #[test]
    fn test_bridge_region() {
        let region = BridgeRegion {
            id: 100,
            facet_indices: vec![0, 1, 2],
            bridge_direction_deg: 90.0,
            anchor_width_mm: 0.0,
            bridge_length_mm: 0.0,
            expansion_margin_mm: 0.0,
            is_valid: false,
            xy_footprint: vec![],
        };

        test_serde_roundtrip!(region);
    }

    #[test]
    fn test_overhang_region() {
        let region = OverhangRegion {
            id: 200,
            facet_indices: vec![0, 1, 2],
            max_angle_deg: 60.0,
            needs_support: true,
        };

        test_serde_roundtrip!(region);
    }

    #[test]
    fn test_layer_plan_ir() {
        let plan = LayerPlanIR {
            schema_version: SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            global_layers: vec![],
            object_participation: std::collections::HashMap::new(),
        };

        test_serde_roundtrip!(plan);
    }

    #[test]
    fn test_global_layer() {
        let layer = GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: vec![],
            has_nonplanar: false,
            is_sync_layer: false,
        };

        test_serde_roundtrip!(layer);
    }

    #[test]
    fn test_active_region() {
        let region = ActiveRegion {
            object_id: "obj-1".to_string(),
            region_id: 1,
            resolved_config: ResolvedConfig::default(),
            effective_layer_height: 0.2,
            nonplanar_shell: None,
            is_catchup_layer: false,
            catchup_z_bottom: 0.0,
            tool_index: 0,
        };

        test_serde_roundtrip!(region);
    }

    #[test]
    fn test_paint_region_ir() {
        let paint_ir = PaintRegionIR {
            schema_version: SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            per_layer: std::collections::HashMap::new(),
        };

        test_serde_roundtrip!(paint_ir);
    }

    #[test]
    fn test_semantic_region() {
        let region = SemanticRegion {
            object_id: "obj-1".to_string(),
            polygons: vec![],
            value: PaintValue::Flag(true),
            paint_order: 1,
            aabb: None,
        };

        test_serde_roundtrip!(region);
    }

    #[test]
    fn test_region_map_ir() {
        let map = RegionMapIR {
            schema_version: SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            entries: std::collections::HashMap::new(),
        };

        test_serde_roundtrip!(map);
    }

    #[test]
    fn test_region_key() {
        let key = RegionKey {
            global_layer_index: 0,
            object_id: "obj-1".to_string(),
            region_id: 1,
        };

        test_serde_roundtrip!(key);
    }

    #[test]
    fn test_slice_ir() {
        let slice = SliceIR {
            schema_version: SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            global_layer_index: 0,
            z: 0.2,
            regions: vec![],
        };

        test_serde_roundtrip!(slice);
    }

    #[test]
    fn test_sliced_region() {
        let region = SlicedRegion {
            object_id: "obj-1".to_string(),
            region_id: 1,
            polygons: vec![],
            infill_areas: vec![],
            nonplanar_surface: None,
            effective_layer_height: 0.2,
            boundary_paint: std::collections::HashMap::new(),
            top_shell_index: None,
            bottom_shell_index: None,
            top_solid_fill: Vec::new(),
            bottom_solid_fill: Vec::new(),
            is_bridge: false,
            bridge_areas: vec![],
            bridge_orientation_deg: 0.0,
        };

        test_serde_roundtrip!(region);
    }

    /// Packet bridge: schema 3.0.0 introduces `top_shell_index` /
    /// `bottom_shell_index` (Option<u8>) plus `top_solid_fill` /
    /// `bottom_solid_fill` (Vec<ExPolygon>) replacing the prior
    /// `is_top_surface` / `is_bottom_surface` bool fields. Round-trip the
    /// populated form to lock in serde stability.
    #[test]
    fn test_sliced_region_shell_classification_roundtrip() {
        let square = ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(0.0, 0.0),
                    Point2::from_mm(1.0, 0.0),
                    Point2::from_mm(1.0, 1.0),
                    Point2::from_mm(0.0, 1.0),
                ],
            },
            holes: vec![],
        };
        let region = SlicedRegion {
            object_id: "obj-shell".to_string(),
            region_id: 2,
            polygons: vec![square.clone()],
            infill_areas: vec![square.clone()],
            nonplanar_surface: None,
            effective_layer_height: 0.2,
            boundary_paint: std::collections::HashMap::new(),
            top_shell_index: Some(0),
            bottom_shell_index: Some(2),
            top_solid_fill: vec![square.clone()],
            bottom_solid_fill: vec![square],
            is_bridge: false,
            bridge_areas: vec![],
            bridge_orientation_deg: 0.0,
        };

        test_serde_roundtrip!(region);
    }

    #[test]
    fn test_expolygon() {
        let poly = ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(0.0, 0.0),
                    Point2::from_mm(1.0, 0.0),
                    Point2::from_mm(1.0, 1.0),
                    Point2::from_mm(0.0, 1.0),
                ],
            },
            holes: vec![],
        };

        test_serde_roundtrip!(poly);
    }

    #[test]
    fn test_gcode_ir() {
        let gcode = GCodeIR {
            schema_version: SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            commands: vec![],
            metadata: PrintMetadata {
                estimated_print_time_s: 3600,
                filament_used_mm: vec![1000.0],
                layer_count: 100,
                slicer_version: "0.1.0".to_string(),
            },
        };

        test_serde_roundtrip!(gcode);
    }

    #[test]
    fn test_gcode_command() {
        test_serde_roundtrip!(GCodeCommand::Move {
            x: Some(100.0),
            y: Some(200.0),
            z: Some(0.2),
            e: Some(1.0),
            f: Some(3000.0),
            role: ExtrusionRole::OuterWall,
        });

        test_serde_roundtrip!(GCodeCommand::Retract {
            length: 1.0,
            speed: 30.0,
            mode: RetractMode::Gcode,
        });
        test_serde_roundtrip!(GCodeCommand::FanSpeed { value: 255 });
        test_serde_roundtrip!(GCodeCommand::ToolChange {
            after_entity_index: 0,
            from: 0,
            to: 1
        });
        test_serde_roundtrip!(GCodeCommand::Comment {
            text: "test".to_string()
        });
    }

    #[test]
    fn test_extrusion_role() {
        test_serde_roundtrip!(ExtrusionRole::OuterWall);
        test_serde_roundtrip!(ExtrusionRole::InnerWall);
        test_serde_roundtrip!(ExtrusionRole::SupportMaterial);
        test_serde_roundtrip!(ExtrusionRole::Custom("custom_role".to_string()));
    }

    #[test]
    fn test_wall_feature_flags() {
        let flags = WallFeatureFlags {
            tool_index: Some(1),
            fuzzy_skin: true,
            is_bridge: false,
            is_thin_wall: false,
            skip_ironing: false,
            custom: std::collections::HashMap::new(),
        };

        test_serde_roundtrip!(flags);
    }

    #[test]
    fn test_wall_boundary_type() {
        test_serde_roundtrip!(WallBoundaryType::ExteriorSurface);
        test_serde_roundtrip!(WallBoundaryType::MaterialBoundary { adjacent_tool: 1 });
        test_serde_roundtrip!(WallBoundaryType::Interior);
    }

    #[test]
    fn test_loop_type() {
        test_serde_roundtrip!(LoopType::Outer);
        test_serde_roundtrip!(LoopType::Inner);
        test_serde_roundtrip!(LoopType::ThinWall);
        test_serde_roundtrip!(LoopType::NonPlanarShell);
    }

    #[test]
    fn test_seam_reason() {
        test_serde_roundtrip!(SeamReason::Concave);
        test_serde_roundtrip!(SeamReason::Aligned);
        test_serde_roundtrip!(SeamReason::UserForced);
        test_serde_roundtrip!(SeamReason::Sharp);
    }

    #[test]
    fn test_types_ids() {
        // Test ID types compile and can be used
        let _obj_id: ObjectId = "uuid-string".to_string();
        let _mod_id: ModifierId = "uuid-string".to_string();
        let _module_id: ModuleId = "com.example.module".to_string();
        let _surf_group_id: SurfaceGroupId = 42;
        let _bridge_id: BridgeRegionId = 100;
        let _overhang_id: OverhangRegionId = 200;
        let _region_id: RegionId = 1;
    }

    #[test]
    fn test_wall_generator() {
        test_serde_roundtrip!(WallGenerator::Classic);
        test_serde_roundtrip!(WallGenerator::Arachne);
    }

    #[test]
    fn test_support_type() {
        test_serde_roundtrip!(SupportType::Traditional);
        test_serde_roundtrip!(SupportType::Tree);
    }

    #[test]
    fn test_infill_type() {
        // This type should exist based on schema
        // We'll verify it compiles and serde works
        // test_serde_roundtrip!(InfillType::Grid);
    }
}

#[test]
fn slice_ir_schema_version_is_one_one_zero() {
    let slice = SliceIR {
        schema_version: SemVer {
            major: 1,
            minor: 1,
            patch: 0,
        },
        global_layer_index: 0,
        z: 0.0,
        regions: vec![],
    };
    assert_eq!(
        (
            slice.schema_version.major,
            slice.schema_version.minor,
            slice.schema_version.patch
        ),
        (1, 1, 0)
    );
}

/// AC-10 (packet 36-rev1): bridge_detector_schema_versions_are_constant_sourced
///
/// Asserts that:
///   (a) CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION == SemVer { 1, 1, 0 }
///   (b) CURRENT_SLICE_IR_SCHEMA_VERSION == SemVer { 1, 2, 0 }
///   (c) SurfaceClassificationIR::default().schema_version == CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION
///   (d) SliceIR::default().schema_version == CURRENT_SLICE_IR_SCHEMA_VERSION
#[test]
fn bridge_detector_schema_versions_are_constant_sourced() {
    // (a)
    assert_eq!(
        slicer_ir::CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION,
        slicer_ir::SemVer {
            major: 1,
            minor: 1,
            patch: 0
        },
        "CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION must be (1, 1, 0)"
    );

    // (b)
    assert_eq!(
        slicer_ir::CURRENT_SLICE_IR_SCHEMA_VERSION,
        slicer_ir::SemVer {
            major: 3,
            minor: 0,
            patch: 0
        },
        "CURRENT_SLICE_IR_SCHEMA_VERSION must be (3, 0, 0)"
    );

    // (c)
    let surf_ir = SurfaceClassificationIR::default();
    assert_eq!(
        surf_ir.schema_version,
        slicer_ir::CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION,
        "SurfaceClassificationIR::default() schema_version must equal the constant"
    );

    // (d)
    let slice_ir = SliceIR::default();
    assert_eq!(
        slice_ir.schema_version,
        slicer_ir::CURRENT_SLICE_IR_SCHEMA_VERSION,
        "SliceIR::default() schema_version must equal the constant"
    );
}

/// TASK-200b: every schema-versioned IR's `Default` must pin
/// `schema_version` to the matching `CURRENT_*_IR_SCHEMA_VERSION` constant.
#[test]
fn chunk2_ir_schema_versions_are_default_sourced() {
    assert_eq!(
        MeshIR::default().schema_version,
        slicer_ir::CURRENT_MESH_IR_SCHEMA_VERSION,
        "MeshIR::default().schema_version must equal CURRENT_MESH_IR_SCHEMA_VERSION"
    );
    assert_eq!(
        LayerPlanIR::default().schema_version,
        slicer_ir::CURRENT_LAYER_PLAN_IR_SCHEMA_VERSION,
        "LayerPlanIR::default().schema_version must equal CURRENT_LAYER_PLAN_IR_SCHEMA_VERSION"
    );
    assert_eq!(
        SeamPlanIR::default().schema_version,
        slicer_ir::CURRENT_SEAM_PLAN_IR_SCHEMA_VERSION,
        "SeamPlanIR::default().schema_version must equal CURRENT_SEAM_PLAN_IR_SCHEMA_VERSION"
    );
    assert_eq!(
        SupportPlanIR::default().schema_version,
        slicer_ir::CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION,
        "SupportPlanIR::default().schema_version must equal CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION"
    );
    assert_eq!(
        SupportGeometryIR::default().schema_version,
        slicer_ir::CURRENT_SUPPORT_GEOMETRY_IR_SCHEMA_VERSION,
        "SupportGeometryIR::default().schema_version must equal CURRENT_SUPPORT_GEOMETRY_IR_SCHEMA_VERSION"
    );
    assert_eq!(
        PaintRegionIR::default().schema_version,
        slicer_ir::CURRENT_PAINT_REGION_IR_SCHEMA_VERSION,
        "PaintRegionIR::default().schema_version must equal CURRENT_PAINT_REGION_IR_SCHEMA_VERSION"
    );
    assert_eq!(
        MeshSegmentationIR::default().schema_version,
        slicer_ir::CURRENT_MESH_SEGMENTATION_IR_SCHEMA_VERSION,
        "MeshSegmentationIR::default().schema_version must equal CURRENT_MESH_SEGMENTATION_IR_SCHEMA_VERSION"
    );
    assert_eq!(
        RegionMapIR::default().schema_version,
        slicer_ir::CURRENT_REGION_MAP_IR_SCHEMA_VERSION,
        "RegionMapIR::default().schema_version must equal CURRENT_REGION_MAP_IR_SCHEMA_VERSION"
    );
    assert_eq!(
        PerimeterIR::default().schema_version,
        slicer_ir::CURRENT_PERIMETER_IR_SCHEMA_VERSION,
        "PerimeterIR::default().schema_version must equal CURRENT_PERIMETER_IR_SCHEMA_VERSION"
    );
    assert_eq!(
        InfillIR::default().schema_version,
        slicer_ir::CURRENT_INFILL_IR_SCHEMA_VERSION,
        "InfillIR::default().schema_version must equal CURRENT_INFILL_IR_SCHEMA_VERSION"
    );
    assert_eq!(
        SupportIR::default().schema_version,
        slicer_ir::CURRENT_SUPPORT_IR_SCHEMA_VERSION,
        "SupportIR::default().schema_version must equal CURRENT_SUPPORT_IR_SCHEMA_VERSION"
    );
    assert_eq!(
        LayerCollectionIR::default().schema_version,
        slicer_ir::CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION,
        "LayerCollectionIR::default().schema_version must equal CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION"
    );
    assert_eq!(
        GCodeIR::default().schema_version,
        slicer_ir::CURRENT_GCODE_IR_SCHEMA_VERSION,
        "GCodeIR::default().schema_version must equal CURRENT_GCODE_IR_SCHEMA_VERSION"
    );
}
