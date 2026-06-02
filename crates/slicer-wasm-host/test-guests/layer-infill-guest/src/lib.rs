//! Minimal test guest module for layer-module world.

wit_bindgen::generate!({
    path: "../../../slicer-schema/wit",
    world: "slicer:world-layer/layer-module@1.0.0",
    generate_all,
});

// First, just try to compile and see what types/traits are generated.
// Use `cargo expand` if needed to inspect the generated code.
struct Component;

impl Guest for Component {
    fn on_print_start(_config: ConfigView) -> Result<(), ModuleError> {
        Ok(())
    }
    fn on_print_end() -> Result<(), ModuleError> {
        Ok(())
    }
    fn run_slice_postprocess(layer_index: LayerIdx, regions: Vec<SliceRegionView>, _paint: PaintRegionLayerView, output: SlicePostprocessBuilder, _config: ConfigView) -> Result<(), ModuleError> {
        // Per-region post-process: replace each input region's polygons with a
        // single triangle, keyed by that region's own identity. This proves
        // per-region commit preserves distinct (object_id, region_id) across
        // the WIT boundary without flattening.
        for r in &regions {
            let obj = r.object_id();
            let rid = r.region_id();
            let key = slicer::ir_handles::ir_handles::RegionKey {
                layer_index,
                object_id: obj,
                region_id: rid,
            };
            let poly = slicer::types::geometry::ExPolygon {
                contour: slicer::types::geometry::Polygon {
                    points: vec![
                        slicer::types::geometry::Point2 { x: 0, y: 0 },
                        slicer::types::geometry::Point2 { x: 1000, y: 0 },
                        slicer::types::geometry::Point2 { x: 1000, y: 1000 },
                    ],
                },
                holes: vec![],
            };
            output.set_polygons(&key, &[poly]).expect("set_polygons failed");
        }
        Ok(())
    }
    fn run_perimeters(_layer_index: LayerIdx, _regions: Vec<SliceRegionView>, _paint: PaintRegionLayerView, _output: PerimeterOutputBuilder, _config: ConfigView) -> Result<(), ModuleError> {
        Ok(())
    }
    fn run_wall_postprocess(_layer_index: LayerIdx, regions: Vec<PerimeterRegionView>, output: PerimeterOutputBuilder, _config: ConfigView) -> Result<(), ModuleError> {
        // Per-region post-process: for each input region, query its identity
        // (which arms the host-side origin tag), then push one wall-loop per
        // region so each output entry is tagged with its source region.
        for r in &regions {
            // Touch wall_loops() to arm origin tag for this region.
            let walls = r.wall_loops();
            let Some(z) = walls
                .first()
                .and_then(|wall| wall.path.points.first())
                .map(|point| point.z)
            else {
                continue;
            };
            let infill_n = r.infill_areas().len();
            let wl = slicer::ir_handles::ir_handles::WallLoopView {
                perimeter_index: walls.len() as u32,
                loop_type: slicer::ir_handles::ir_handles::WallLoopType::Outer,
                path: slicer::types::geometry::ExtrusionPath3d {
                    points: vec![slicer::types::geometry::Point3WithWidth {
                        x: walls.len() as f32,
                        y: infill_n as f32,
                        z,
                        width: 0.4,
                        flow_factor: 1.0,
                        overhang_quartile: None,
                    }],
                    role: slicer::types::geometry::ExtrusionRole::OuterWall,
                    speed_factor: 1.0,
                },
                feature_flags: vec![slicer::ir_handles::ir_handles::WallFeatureFlag {
                    tool_index: None,
                    fuzzy_skin: false,
                    is_bridge: false,
                    is_thin_wall: false,
                    skip_ironing: false,
                    custom: vec![],
                }],
            };
            output.push_wall_loop(&wl).expect("push wall loop failed");
        }
        Ok(())
    }
    fn run_infill(
        layer_index: LayerIdx,
        regions: Vec<SliceRegionView>,
        output: InfillOutputBuilder,
        config: ConfigView,
    ) -> Result<(), ModuleError> {
        // 1. Read config
        let spacing = config.get_float("infill-spacing").unwrap_or(2.0);
        // 2. Log
        slicer::world_layer::host_services::log(
            slicer::world_layer::host_services::LogLevel::Info,
            &format!("run-infill: layer={}, spacing={}, regions={}", layer_index, spacing, regions.len()),
        );
        // 3. Read region data — encode slice region info into output:
        //    point[0].z = region z (or 0 if empty)
        //    point[0].flow_factor = region count as f32
        //    point[0].width = total polygon count across all regions
        let Some(z) = regions.first().map(|region| region.z()) else {
            return Ok(());
        };
        let region_count = regions.len() as f32;
        let total_polys: f32 = regions.iter().map(|r| r.polygons().len() as f32).sum();
        // 4. Push output
        let path = slicer::types::geometry::ExtrusionPath3d {
            points: vec![
                slicer::types::geometry::Point3WithWidth {
                    x: 0.0, y: 0.0, z,
                    width: total_polys,
                    flow_factor: region_count,
                    overhang_quartile: None,
                },
                slicer::types::geometry::Point3WithWidth {
                    x: spacing as f32 * 10.0, y: 0.0, z,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
            ],
            role: slicer::types::geometry::ExtrusionRole::SparseInfill,
            speed_factor: 1.0,
        };
        output.push_sparse_path(&path).expect("push failed");
        Ok(())
    }
    fn run_infill_postprocess(_layer_index: LayerIdx, regions: Vec<PerimeterRegionView>, output: InfillOutputBuilder, _config: ConfigView) -> Result<(), ModuleError> {
        // Per-region post-process: emit one solid-infill path per input region,
        // each tagged with its source region's identity.
        for r in &regions {
            let walls = r.wall_loops();
            let Some(z) = walls
                .first()
                .and_then(|wall| wall.path.points.first())
                .map(|point| point.z)
            else {
                continue;
            };
            let infill_n = r.infill_areas().len();
            let path = slicer::types::geometry::ExtrusionPath3d {
                points: vec![slicer::types::geometry::Point3WithWidth {
                    x: walls.len() as f32,
                    y: infill_n as f32,
                    z,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                }],
                role: slicer::types::geometry::ExtrusionRole::TopSolidInfill,
                speed_factor: 1.0,
            };
            output.push_solid_path(&path).expect("push solid path failed");
        }
        Ok(())
    }
    fn run_support(_layer_index: LayerIdx, regions: Vec<SliceRegionView>, paint: PaintRegionLayerView, output: SupportOutputBuilder, _config: ConfigView) -> Result<(), ModuleError> {
        // Query support-enforcer paint regions.
        use slicer::ir_handles::ir_handles::PaintSemantic;
        let Some(z) = regions.first().map(|region| region.z()) else {
            return Ok(());
        };
        let enforcers = paint.get_regions(&PaintSemantic::SupportEnforcer);
        let blocker_count = paint.get_regions(&PaintSemantic::SupportBlocker).len();
        let paint_layer_idx = paint.layer_index();

        // Encode paint data into observable support output:
        // - first point x = enforcer region count as f32
        // - first point y = blocker region count as f32
        // - first point z = first slice region z (keeps the path inside the host envelope)
        // - first point flow_factor = paint layer index as f32 (proves layer index was threaded)
        let region_count = enforcers.len() as f32;
        let path = slicer::types::geometry::ExtrusionPath3d {
            points: vec![
                slicer::types::geometry::Point3WithWidth {
                    x: region_count,
                    y: blocker_count as f32,
                    z,
                    width: 0.4,
                    flow_factor: paint_layer_idx as f32,
                    overhang_quartile: None,
                },
            ],
            role: slicer::types::geometry::ExtrusionRole::SupportMaterial,
            speed_factor: 1.0,
        };
        output.push_support_path(&path).expect("push support path failed");
        Ok(())
    }
    fn run_support_postprocess(_layer_index: LayerIdx, regions: Vec<SliceRegionView>, output: SupportOutputBuilder, _config: ConfigView) -> Result<(), ModuleError> {
        // Per-region post-process: for each slice region, touch identity
        // (arms host-side origin tag) then push one support path so each
        // output entry is tagged with its source region.
        for r in &regions {
            // Touch identity fields to arm origin tag.
            let _obj = r.object_id();
            let _rid = r.region_id();
            let poly_n = r.polygons().len();
            let path = slicer::types::geometry::ExtrusionPath3d {
                points: vec![slicer::types::geometry::Point3WithWidth {
                    x: poly_n as f32,
                    y: 0.0,
                    z: r.z(),
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                }],
                role: slicer::types::geometry::ExtrusionRole::SupportMaterial,
                speed_factor: 1.0,
            };
            output.push_support_path(&path).expect("push support path failed");
        }
        Ok(())
    }
    fn run_path_optimization(_layer_index: LayerIdx, regions: Vec<PerimeterRegionView>, output: GcodeOutputBuilder, _collection: LayerCollectionBuilder, _config: ConfigView) -> Result<(), ModuleError> {
        // Emit a comment encoding perimeter-region counts (observable through output).
        let region_count = regions.len();
        let total_walls: usize = regions.iter().map(|r| r.wall_loops().len()).sum();
        let total_infill: usize = regions.iter().map(|r| r.infill_areas().len()).sum();
        let comment = format!("regions={} walls={} infill={}", region_count, total_walls, total_infill);
        output.push_comment(&comment).expect("push comment failed");
        // Emit one deterministic tool-change override per active region so the
        // host commit path can fold it into LayerCollectionIR.tool_changes.
        for i in 0..region_count as u32 {
            output.push_tool_change(i, i, i + 1).expect("push tool_change failed");
        }
        // Emit one z-hop per region, all anchored
        // at entity index 0. Using a fixed anchor keeps direct-dispatch tests
        // (which don't pre-stage LayerCollectionIR) within the empty-layer
        // validation rule (after-entity-index must be 0 when entity_count==0)
        // while still proving multi-call ordering through the commit path.
        for _ in 0..region_count as u32 {
            output.push_z_hop(0, 0.5).expect("push z_hop failed");
        }
        Ok(())
    }
}

export!(Component);
