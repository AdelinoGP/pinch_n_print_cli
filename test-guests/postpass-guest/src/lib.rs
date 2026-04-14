wit_bindgen::generate!({
    inline: r#"
        package slicer:postpass-world@1.0.0;

        interface geometry {
            record point3 { x: f32, y: f32, z: f32 }
            record bounding-box3 { min: point3, max: point3 }
            record point2 { x: s64, y: s64 }
            record polygon { points: list<point2> }
            record ex-polygon { contour: polygon, holes: list<polygon> }
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

        world postpass-module {
            import host-services;
            import config-types;
            use config-types.{config-view};
            use geometry.{extrusion-role};
            record module-error { code: u32, message: string, fatal: bool }

            record gcode-move-cmd { x: option<f32>, y: option<f32>, z: option<f32>, e: option<f32>, f: option<f32>, role: extrusion-role }
            resource gcode-output-builder {
                push-move:        func(cmd: gcode-move-cmd) -> result<_, string>;
                push-retract:     func(length: f32, speed: f32) -> result<_, string>;
                push-fan-speed:   func(value: u8) -> result<_, string>;
                push-temperature: func(tool: u32, celsius: f32, wait: bool) -> result<_, string>;
                push-tool-change: func(from-tool: u32, to-tool: u32) -> result<_, string>;
                push-comment:     func(text: string) -> result<_, string>;
                push-raw:         func(text: string) -> result<_, string>;
            }

            enum gcode-command-kind { move-cmd, retract, fan-speed, temperature, tool-change, comment, raw }
            record gcode-command-view { index: u32, kind: gcode-command-kind }

            export run-gcode-postprocess: func(
                commands: list<gcode-command-view>,
                output: gcode-output-builder,
                config: config-view,
            ) -> result<_, module-error>;

            export run-text-postprocess: func(
                gcode-text: string,
                config: config-view,
            ) -> result<string, module-error>;
        }
    "#,
    world: "postpass-module",
});

struct Component;

impl Guest for Component {
    fn run_gcode_postprocess(_commands: Vec<GcodeCommandView>, _output: GcodeOutputBuilder, _config: ConfigView) -> Result<(), ModuleError> {
        slicer::postpass_world::host_services::log(
            slicer::postpass_world::host_services::LogLevel::Info,
            "run-gcode-postprocess: ok",
        );
        Ok(())
    }
    fn run_text_postprocess(gcode_text: String, _config: ConfigView) -> Result<String, ModuleError> {
        Ok(gcode_text)
    }
}

export!(Component);
