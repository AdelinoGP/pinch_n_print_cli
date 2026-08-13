pub fn run(gcode_text : String, config : @config.config_view) -> Result[String, @common.module_error] {
  Ok(";; foreign-language-probe\n; probe input\n")
}
