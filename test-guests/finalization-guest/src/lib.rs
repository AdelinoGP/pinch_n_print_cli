wit_bindgen::generate!({
    inline: r#"
        package slicer:finalization-world@1.0.0;

        interface geometry {
            record point3 { x: f32, y: f32, z: f32 }
            record bounding-box3 { min: point3, max: point3 }
            record point2 { x: s64, y: s64 }
            record point3-with-width { x: f32, y: f32, z: f32, width: f32, flow-factor: f32 }
            record polygon { points: list<point2> }
            record ex-polygon { contour: polygon, holes: list<polygon> }
            record extrusion-path3d { points: list<point3-with-width>, role: extrusion-role, speed-factor: f32 }
            enum extrusion-role {
                outer-wall, inner-wall, thin-wall,
                top-solid-infill, bottom-solid-infill, sparse-infill,
                support-material, support-interface,
                ironing, bridge-infill, wipe-tower, custom,
            }
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

        world finalization-module {
            import host-services;
            import config-types;
            use config-types.{config-view};
            use geometry.{extrusion-path3d};
            type layer-idx = u32;
            type object-id = string;
            type region-id = string;
            record module-error { code: u32, message: string, fatal: bool }
            record region-key { layer-index: layer-idx, object-id: object-id, region-id: region-id }

            record tool-change-view {
                after-entity-index: u32,
                from-tool: u32,
                to-tool: u32,
            }

            resource layer-collection-view {
                layer-index:  func() -> layer-idx;
                z:            func() -> f32;
                entity-count: func() -> u32;
                tool-changes: func() -> list<tool-change-view>;
            }

            resource finalization-output-builder {
                push-entity-to-layer: func(layer-index: layer-idx, path: extrusion-path3d, region-key: region-key) -> result<_, string>;
                insert-synthetic-layer: func(z: f32, paths: list<extrusion-path3d>) -> result<_, string>;
            }

            export run-finalization: func(
                layers: list<layer-collection-view>,
                output: finalization-output-builder,
                config: config-view,
            ) -> result<_, module-error>;
        }
    "#,
    world: "finalization-module",
});

struct Component;

impl Guest for Component {
    fn run_finalization(_layers: Vec<LayerCollectionView>, _output: FinalizationOutputBuilder, _config: ConfigView) -> Result<(), ModuleError> {
        slicer::finalization_world::host_services::log(
            slicer::finalization_world::host_services::LogLevel::Info,
            "run-finalization: ok",
        );
        Ok(())
    }
}

export!(Component);
