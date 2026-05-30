wit_bindgen::generate!({
    path: "../../../slicer-schema/wit",
    world: "slicer:world-postpass/postpass-module@1.0.0",
    generate_all,
});

struct Component;

fn echo_command(command: &GcodeCommand, output: &GcodeOutputBuilder) -> Result<(), ModuleError> {
    match command {
        GcodeCommand::Move(cmd) => output.push_move(cmd),
        GcodeCommand::Retract(cmd) => output.push_retract(cmd.length, cmd.speed, cmd.mode),
        GcodeCommand::Unretract(cmd) => output.push_unretract(cmd.length, cmd.speed, cmd.mode),
        GcodeCommand::FanSpeed(cmd) => output.push_fan_speed(cmd.value),
        GcodeCommand::Temperature(cmd) => output.push_temperature(cmd.tool, cmd.celsius, cmd.wait),
        GcodeCommand::ToolChange(cmd) => output.push_tool_change(cmd.after_entity_index, cmd.from_tool, cmd.to_tool),
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
                output.push_retract(0.8, 35.0, RetractMode::Gcode).map_err(|message| ModuleError { code: 3, message, fatal: true })?;
                output.push_unretract(0.8, 35.0, RetractMode::Gcode).map_err(|message| ModuleError { code: 4, message, fatal: true })?;
                output.push_fan_speed(200).map_err(|message| ModuleError { code: 5, message, fatal: true })?;
                output.push_temperature(1, 215.0, false).map_err(|message| ModuleError { code: 6, message, fatal: true })?;
                output.push_tool_change(0, 1, 2).map_err(|message| ModuleError { code: 7, message, fatal: true })?;
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
