//! Minimal test guest module for layer-module world.

wit_bindgen::generate!({
    inline: r#"
        package slicer:world-layer@1.0.0;

        interface geometry {
            record point2 { x: s64, y: s64 }
            record point3 { x: f32, y: f32, z: f32 }
            record point3-with-width { x: f32, y: f32, z: f32, width: f32, flow-factor: f32 }
            record bounding-box2 { min: point2, max: point2 }
            record bounding-box3 { min: point3, max: point3 }
            record polygon       { points: list<point2> }
            record ex-polygon    { contour: polygon, holes: list<polygon> }
            record extrusion-path3d { points: list<point3-with-width>, role: extrusion-role, speed-factor: f32 }
            variant extrusion-role {
                outer-wall, inner-wall, thin-wall,
                top-solid-infill, bottom-solid-infill, sparse-infill,
                support-material, support-interface,
                ironing, bridge-infill, wipe-tower, custom(string),
            }
            record semver { major: u32, minor: u32, patch: u32 }
        }

        interface config-types {
            variant config-value {
                bool-val(bool), int-val(s64), float-val(f64),
                string-val(string), float-list(list<f64>), string-list(list<string>),
            }
            resource config-view {
                get:        func(key: string) -> option<config-value>;
                get-bool:   func(key: string) -> option<bool>;
                get-float:  func(key: string) -> option<f64>;
                get-int:    func(key: string) -> option<s64>;
                get-string: func(key: string) -> option<string>;
                keys:       func() -> list<string>;
            }
        }

        interface host-services {
            use geometry.{point3, bounding-box3, ex-polygon, polygon};
            type object-id = string;
            enum log-level { trace, debug, info, warn, error }
            log: func(level: log-level, message: string);
            raycast-z-down:     func(object-id: object-id, x: f32, y: f32, start-z: f32) -> option<f32>;
            surface-normal-at:  func(object-id: object-id, x: f32, y: f32, z: f32) -> option<point3>;
            object-bounds:      func(object-id: object-id) -> bounding-box3;
            enum clip-operation   { union, intersection, difference, xor }
            enum offset-join-type { miter, round, square }
            clip-polygons:    func(subject: list<ex-polygon>, clip: list<ex-polygon>, op: clip-operation) -> list<ex-polygon>;
            offset-polygons:  func(polygons: list<ex-polygon>, delta-mm: f32, join: offset-join-type) -> list<ex-polygon>;
            simplify-polygon: func(polygon: polygon, tolerance-mm: f32) -> polygon;
            now-us: func() -> u64;
        }

        interface ir-handles {
            use geometry.{ex-polygon, extrusion-path3d, point3, extrusion-role};
            type object-id = string;
            type region-id = string;
            type layer-idx = u32;
            record region-key { layer-index: layer-idx, object-id: object-id, region-id: region-id }
            record wall-feature-flag { tool-index: option<u32>, fuzzy-skin: bool, is-bridge: bool, is-thin-wall: bool, skip-ironing: bool, custom: list<tuple<string, paint-value>> }
            record wall-loop-view { perimeter-index: u32, loop-type: wall-loop-type, path: extrusion-path3d, feature-flags: list<wall-feature-flag> }
            enum wall-loop-type { outer, inner, thin-wall, nonplanar-shell }
            variant paint-semantic { material, fuzzy-skin, support-enforcer, support-blocker, custom(string) }
            variant paint-value { flag(bool), scalar(f32), tool-index(u32) }
            record boundary-paint-polygon { values: list<option<paint-value>> }
            record boundary-paint-entry { semantic: paint-semantic, polygons: list<boundary-paint-polygon> }
            resource slice-region-view {
                object-id: func() -> object-id;
                region-id: func() -> region-id;
                polygons: func() -> list<ex-polygon>;
                infill-areas: func() -> list<ex-polygon>;
                effective-layer-height: func() -> f32;
                z: func() -> f32;
                has-nonplanar: func() -> bool;
                boundary-paint: func() -> list<boundary-paint-entry>;
            }
            resource perimeter-region-view {
                object-id: func() -> object-id;
                region-id: func() -> region-id;
                wall-loops: func() -> list<wall-loop-view>;
                infill-areas: func() -> list<ex-polygon>;
            }
            resource infill-output-builder {
                push-sparse-path:  func(path: extrusion-path3d) -> result<_, string>;
                push-solid-path:   func(path: extrusion-path3d) -> result<_, string>;
                push-ironing-path: func(path: extrusion-path3d) -> result<_, string>;
            }
            resource perimeter-output-builder {
                push-wall-loop:      func(wall-loop: wall-loop-view) -> result<_, string>;
                set-infill-areas:    func(areas: list<ex-polygon>) -> result<_, string>;
                push-seam-candidate: func(pos: point3, score: f32) -> result<_, string>;
            }
            resource slice-postprocess-builder {
                set-polygons: func(region: region-key, polys: list<ex-polygon>) -> result<_, string>;
                set-path-z:   func(region: region-key, path-idx: u32, vertex-idx: u32, z: f32) -> result<_, string>;
            }
            record gcode-move-cmd { x: option<f32>, y: option<f32>, z: option<f32>, e: option<f32>, f: option<f32>, role: extrusion-role }
            resource gcode-output-builder {
                push-move:        func(cmd: gcode-move-cmd) -> result<_, string>;
                push-retract:     func(length: f32, speed: f32) -> result<_, string>;
                push-fan-speed:   func(value: u8) -> result<_, string>;
                push-temperature: func(tool: u32, celsius: f32, wait: bool) -> result<_, string>;
                push-tool-change: func(after-entity-index: u32, from-tool: u32, to-tool: u32) -> result<_, string>;
                push-comment:     func(text: string) -> result<_, string>;
                push-raw:         func(text: string) -> result<_, string>;
                push-z-hop:       func(after-entity-index: u32, hop-height: f32) -> result<_, string>;
            }
            resource layer-collection-builder {
                set-entity-order: func(items: list<tuple<u32, bool>>) -> result<_, string>;
            }
            resource support-output-builder {
                push-support-path:   func(path: extrusion-path3d) -> result<_, string>;
                push-interface-path: func(path: extrusion-path3d, is-top-interface: bool) -> result<_, string>;
                push-raft-path:      func(path: extrusion-path3d) -> result<_, string>;
            }
            record semantic-region { object-id: object-id, polygons: list<ex-polygon>, value: paint-value }
            resource paint-region-layer-view {
                get-regions: func(semantic: paint-semantic) -> list<semantic-region>;
                get-custom-regions: func(module-id: string) -> list<semantic-region>;
                layer-index: func() -> layer-idx;
            }
        }

        world layer-module {
            import host-services;
            import config-types;
            import ir-handles;
            record module-error { code: u32, message: string, fatal: bool }
            use config-types.{config-view};
            use ir-handles.{
                slice-region-view, perimeter-region-view,
                infill-output-builder, perimeter-output-builder,
                slice-postprocess-builder, support-output-builder,
                gcode-output-builder, layer-collection-builder,
                region-key, layer-idx,
                paint-region-layer-view,
            };
            export on-print-start: func(config: config-view) -> result<_, module-error>;
            export on-print-end:   func() -> result<_, module-error>;
            export run-slice-postprocess: func(layer-index: layer-idx, regions: list<slice-region-view>, paint: paint-region-layer-view, output: slice-postprocess-builder, config: config-view) -> result<_, module-error>;
            export run-perimeters: func(layer-index: layer-idx, regions: list<slice-region-view>, paint: paint-region-layer-view, output: perimeter-output-builder, config: config-view) -> result<_, module-error>;
            export run-wall-postprocess: func(layer-index: layer-idx, regions: list<perimeter-region-view>, output: perimeter-output-builder, config: config-view) -> result<_, module-error>;
            export run-infill: func(layer-index: layer-idx, regions: list<slice-region-view>, output: infill-output-builder, config: config-view) -> result<_, module-error>;
            export run-infill-postprocess: func(layer-index: layer-idx, regions: list<perimeter-region-view>, output: infill-output-builder, config: config-view) -> result<_, module-error>;
            export run-support: func(layer-index: layer-idx, regions: list<slice-region-view>, paint: paint-region-layer-view, output: support-output-builder, config: config-view) -> result<_, module-error>;
            export run-support-postprocess: func(layer-index: layer-idx, regions: list<slice-region-view>, output: support-output-builder, config: config-view) -> result<_, module-error>;
            export run-path-optimization: func(layer-index: layer-idx, regions: list<perimeter-region-view>, output: gcode-output-builder, collection: layer-collection-builder, config: config-view) -> result<_, module-error>;
        }
    "#,
    world: "layer-module",
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
            let key = slicer::world_layer::ir_handles::RegionKey {
                layer_index,
                object_id: obj,
                region_id: rid,
            };
            let poly = slicer::world_layer::geometry::ExPolygon {
                contour: slicer::world_layer::geometry::Polygon {
                    points: vec![
                        slicer::world_layer::geometry::Point2 { x: 0, y: 0 },
                        slicer::world_layer::geometry::Point2 { x: 1000, y: 0 },
                        slicer::world_layer::geometry::Point2 { x: 1000, y: 1000 },
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
            let wl = slicer::world_layer::ir_handles::WallLoopView {
                perimeter_index: walls.len() as u32,
                loop_type: slicer::world_layer::ir_handles::WallLoopType::Outer,
                path: slicer::world_layer::geometry::ExtrusionPath3d {
                    points: vec![slicer::world_layer::geometry::Point3WithWidth {
                        x: walls.len() as f32,
                        y: infill_n as f32,
                        z,
                        width: 0.4,
                        flow_factor: 1.0,
                    }],
                    role: slicer::world_layer::geometry::ExtrusionRole::OuterWall,
                    speed_factor: 1.0,
                },
                feature_flags: vec![slicer::world_layer::ir_handles::WallFeatureFlag {
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
        let path = slicer::world_layer::geometry::ExtrusionPath3d {
            points: vec![
                slicer::world_layer::geometry::Point3WithWidth {
                    x: 0.0, y: 0.0, z,
                    width: total_polys,
                    flow_factor: region_count,
                },
                slicer::world_layer::geometry::Point3WithWidth {
                    x: spacing as f32 * 10.0, y: 0.0, z,
                    width: 0.4,
                    flow_factor: 1.0,
                },
            ],
            role: slicer::world_layer::geometry::ExtrusionRole::SparseInfill,
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
            let path = slicer::world_layer::geometry::ExtrusionPath3d {
                points: vec![slicer::world_layer::geometry::Point3WithWidth {
                    x: walls.len() as f32,
                    y: infill_n as f32,
                    z,
                    width: 0.4,
                    flow_factor: 1.0,
                }],
                role: slicer::world_layer::geometry::ExtrusionRole::TopSolidInfill,
                speed_factor: 1.0,
            };
            output.push_solid_path(&path).expect("push solid path failed");
        }
        Ok(())
    }
    fn run_support(_layer_index: LayerIdx, regions: Vec<SliceRegionView>, paint: PaintRegionLayerView, output: SupportOutputBuilder, _config: ConfigView) -> Result<(), ModuleError> {
        // Query support-enforcer paint regions.
        use slicer::world_layer::ir_handles::PaintSemantic;
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
        let path = slicer::world_layer::geometry::ExtrusionPath3d {
            points: vec![
                slicer::world_layer::geometry::Point3WithWidth {
                    x: region_count,
                    y: blocker_count as f32,
                    z,
                    width: 0.4,
                    flow_factor: paint_layer_idx as f32,
                },
            ],
            role: slicer::world_layer::geometry::ExtrusionRole::SupportMaterial,
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
            let path = slicer::world_layer::geometry::ExtrusionPath3d {
                points: vec![slicer::world_layer::geometry::Point3WithWidth {
                    x: poly_n as f32,
                    y: 0.0,
                    z: r.z(),
                    width: 0.4,
                    flow_factor: 1.0,
                }],
                role: slicer::world_layer::geometry::ExtrusionRole::SupportMaterial,
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
