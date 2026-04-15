//! TASK-109 round-trip guest: authored purely via `#[slicer_module]`.
//!
//! Demonstrates that the macro now emits real typed wit_bindgen export
//! glue for `PostPass::TextPostProcess` — no hand-rolled
//! `wit_bindgen::generate!` + `export!(Component)` block is written
//! here. When compiled to wasm32-unknown-unknown and converted via
//! `wasm-tools component new`, the resulting component implements the
//! documented `postpass-module` WIT world and round-trips the text
//! input through `PostpassModule::run_text_postprocess` with a real
//! typed `ConfigView` on the live path.

use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::PostpassModule;
use slicer_ir::ConfigView;

pub struct SdkPostpassTextModule;

#[slicer_module]
impl PostpassModule for SdkPostpassTextModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_text_postprocess(
        &self,
        gcode_text: &str,
        config: &ConfigView,
    ) -> Result<String, ModuleError> {
        // Round-trip proof points:
        // 1. The input text arrives typed (String from the host, seen
        //    here as &str after the macro's wit_bindgen glue).
        // 2. The ConfigView carries every declared key as typed values.
        //    Read one with a default so the test can assert that the
        //    config pathway really reached this trait body.
        let prefix = config
            .get_string("postpass_text_prefix")
            .map(|s| s.to_string())
            .unwrap_or_else(|| String::from(";; task-109 guest: "));

        Ok(format!("{prefix}{gcode_text}"))
    }
}
