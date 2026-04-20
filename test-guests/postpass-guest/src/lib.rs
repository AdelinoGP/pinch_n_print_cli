wit_bindgen::generate!({
    inline: r#"
        package slicer:world-postpass@1.0.0;

        interface geometry {
            record point3 { x: f32, y: f32, z: f32 }
            record bounding-box3 { min: point3, max: point3 }
            record point2 { x: s64, y: s64 }
            record polygon { points: list<point2> }
            record ex-polygon { contour: polygon, holes: list<polygon> }
            variant extrusion-role {
                outer-wall, inner-wall, thin-wall,
                top-solid-infill, bottom-solid-infill, sparse-infill,
                support-material, support-interface,
                ironing, bridge-infill, wipe-tower, custom(string),
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
            record gcode-retract-cmd { length: f32, speed: f32 }
            record gcode-fan-speed-cmd { value: u8 }
            record gcode-temperature-cmd { tool: u32, celsius: f32, wait: bool }
            record gcode-tool-change-cmd { from-tool: u32, to-tool: u32 }
            resource gcode-output-builder {
                push-move:        func(cmd: gcode-move-cmd) -> result<_, string>;
                push-retract:     func(length: f32, speed: f32) -> result<_, string>;
                push-unretract:   func(length: f32, speed: f32) -> result<_, string>;
                push-fan-speed:   func(value: u8) -> result<_, string>;
                push-temperature: func(tool: u32, celsius: f32, wait: bool) -> result<_, string>;
                push-tool-change: func(from-tool: u32, to-tool: u32) -> result<_, string>;
                push-comment:     func(text: string) -> result<_, string>;
                push-raw:         func(text: string) -> result<_, string>;
                push-z-hop:       func(after-entity-index: u32, hop-height: f32) -> result<_, string>;
            }

            variant gcode-command {
                move(gcode-move-cmd),
                retract(gcode-retract-cmd),
                unretract(gcode-retract-cmd),
                fan-speed(gcode-fan-speed-cmd),
                temperature(gcode-temperature-cmd),
                tool-change(gcode-tool-change-cmd),
                comment(string),
                raw(string),
            }

            export run-gcode-postprocess: func(
                commands: list<gcode-command>,
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

fn echo_command(command: &GcodeCommand, output: &GcodeOutputBuilder) -> Result<(), ModuleError> {
    match command {
        GcodeCommand::Move(cmd) => output.push_move(cmd),
        GcodeCommand::Retract(cmd) => output.push_retract(cmd.length, cmd.speed),
        GcodeCommand::Unretract(cmd) => output.push_unretract(cmd.length, cmd.speed),
        GcodeCommand::FanSpeed(cmd) => output.push_fan_speed(cmd.value),
        GcodeCommand::Temperature(cmd) => output.push_temperature(cmd.tool, cmd.celsius, cmd.wait),
        GcodeCommand::ToolChange(cmd) => output.push_tool_change(cmd.from_tool, cmd.to_tool),
        GcodeCommand::Comment(text) => output.push_comment(text),
        GcodeCommand::Raw(text) => output.push_raw(text),
    }
    .map_err(|message| ModuleError { code: 1, message, fatal: true })
}

impl Guest for Component {
    fn run_gcode_postprocess(commands: Vec<GcodeCommand>, output: GcodeOutputBuilder, config: ConfigView) -> Result<(), ModuleError> {
        slicer::world_postpass::host_services::log(
            slicer::world_postpass::host_services::LogLevel::Info,
            "run-gcode-postprocess: ok",
        );

        match config.get_string("postpass_mode") {
            Some(mode) if mode == "echo" => {
                for command in &commands {
                    echo_command(command, &output)?;
                }
            }
            Some(mode) if mode == "emit-sample" => {
                output.push_move(&GcodeMoveCmd {
                    x: Some(10.0),
                    y: Some(20.0),
                    z: Some(0.3),
                    e: Some(1.25),
                    f: Some(1500.0),
                    role: ExtrusionRole::OuterWall,
                }).map_err(|message| ModuleError { code: 2, message, fatal: true })?;
                output.push_retract(0.8, 35.0).map_err(|message| ModuleError { code: 3, message, fatal: true })?;
                output.push_unretract(0.8, 35.0).map_err(|message| ModuleError { code: 4, message, fatal: true })?;
                output.push_fan_speed(200).map_err(|message| ModuleError { code: 5, message, fatal: true })?;
                output.push_temperature(1, 215.0, false).map_err(|message| ModuleError { code: 6, message, fatal: true })?;
                output.push_tool_change(1, 2).map_err(|message| ModuleError { code: 7, message, fatal: true })?;
                output.push_comment("sample comment").map_err(|message| ModuleError { code: 8, message, fatal: true })?;
                output.push_raw("M117 sample raw").map_err(|message| ModuleError { code: 9, message, fatal: true })?;
            }
            Some(mode) if mode == "emit-z-hop" => {
                output.push_z_hop(0, 0.4).map_err(|message| ModuleError { code: 10, message, fatal: true })?;
            }
            _ => {}
        }

        Ok(())
    }
    fn run_text_postprocess(gcode_text: String, _config: ConfigView) -> Result<String, ModuleError> {
        Ok(gcode_text)
    }
}

export!(Component);
