//! Controlled numeric preparation fixture, not a production infill module.

use slicer_ir::{units_to_mm, ConfigView, ExtrusionPath3D, ExtrusionRole, Point2, Point3WithWidth};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::preparation::{
    LayerPreparation, PreparedPlanOutput, PreparedPlanView, PrintPreparationInput,
};
use slicer_sdk::slicer_module;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::SliceRegionView;

pub struct PreparationPilotInfill {
    offset: u64,
    max_bytes: u64,
    require_numbers: bool,
    max_offset_probe: bool,
    raw_read_probe: bool,
}

/// One byte represents one nonnegative integer millimeter value. Reserved
/// sentinel bytes are preparation modes, never valid serialized numbers.
fn decode_numbers(bytes: &[u8]) -> Result<Vec<f32>, ModuleError> {
    if bytes.iter().any(|byte| *byte >= 252) {
        return Err(ModuleError::non_fatal(1, "malformed pilot numbers"));
    }
    Ok(bytes.iter().map(|byte| f32::from(*byte)).collect())
}

fn range_config(config: &ConfigView, key: &str, default: i64) -> Result<u64, ModuleError> {
    u64::try_from(config.get_int(key).unwrap_or(default))
        .map_err(|_| ModuleError::non_fatal(1, format!("negative {key}")))
}

impl LayerPreparation for PreparationPilotInfill {
    fn prepare_print(
        input: &PrintPreparationInput,
        output: &mut PreparedPlanOutput,
    ) -> Result<(), ModuleError> {
        let bytes = input.read_fixture(0, 4)?;
        match bytes.first() {
            None => return Ok(()),
            Some(255) => {
                output.put("numbers", &[3, 5])?;
                let duplicate = output
                    .put("numbers", &[11, 17])
                    .expect_err("duplicate fixture mode must reject the repeated name");
                // Preserve the original duplicate detail, but propagate the
                // poisoned completion from the intended-success operation.
                return output.put("after_duplicate", &[11, 17]).map_err(|poison| {
                    ModuleError::fatal(
                        1,
                        format!("layer preparation: {poison}; original put error: {duplicate}"),
                    )
                });
            }
            Some(254) => panic!("pilot preparation trap"),
            Some(253) => {
                output.put("numbers", &[3, 5])?;
                return Err(ModuleError::non_fatal(1, "pilot preparation failed"));
            }
            Some(252) => {
                output.put("numbers", &[3, 5])?;
                let _caught_duplicate = output.put("numbers", &[11, 17]);
                // Genuine module success: generated completion glue, not this
                // fixture, must report the caught error and block publication.
                return Ok(());
            }
            _ => {}
        }
        let numbers = decode_numbers(&bytes)?;
        let encoded: Vec<u8> = numbers.into_iter().map(|number| number as u8).collect();
        output.put("numbers", &encoded)?;
        output.put("π", &[])?;
        Ok(())
    }
}

#[slicer_module(preparation)]
impl LayerModule for PreparationPilotInfill {
    fn from_config(config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self {
            offset: range_config(config, "pilot_range_offset", 0)?,
            max_bytes: range_config(config, "pilot_range_max_bytes", 2)?,
            require_numbers: config.get_bool("pilot_require_numbers").unwrap_or(true),
            max_offset_probe: config.get_bool("pilot_max_offset_probe").unwrap_or(false),
            raw_read_probe: config.get_bool("pilot_raw_read_probe").unwrap_or(false),
        })
    }

    fn run_infill(
        &self,
        _layer_index: u32,
        _regions: &[SliceRegionView],
        _paint: &slicer_sdk::PaintRegionLayerView,
        output: &mut InfillOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        if self.max_offset_probe || self.raw_read_probe {
            // Bypass SDK validation and truncation only in this fixture so
            // the actual guest resource import receives the exact read tuple.
            #[cfg(target_arch = "wasm32")]
            {
                use __slicer_layer_infill_world_export::slicer::layer_preparation::{
                    plan_access, preparation_types,
                };
                let plan =
                    plan_access::current().map_err(|e| ModuleError::fatal(1, format!("{e:?}")))?;
                let piece = plan
                    .open("numbers")
                    .map_err(|e| ModuleError::fatal(1, format!("{e:?}")))?
                    .unwrap();
                let (offset, max) = if self.max_offset_probe {
                    (u64::MAX, 1)
                } else {
                    (self.offset, self.max_bytes)
                };
                return piece.read(offset, max).map(|_| ()).map_err(|e| {
                    if matches!(e, preparation_types::PreparationError::RangeError) {
                        slicer_sdk::preparation::PreparationError::RangeError.into()
                    } else {
                        ModuleError::fatal(1, format!("guest raw-read probe: {e:?}"))
                    }
                });
            }
            #[cfg(not(target_arch = "wasm32"))]
            return Err(ModuleError::fatal(1, "raw-read probe is guest-only"));
        }
        let plan = PreparedPlanView::current()?;
        let Some(piece) = plan.open("numbers")? else {
            return if self.require_numbers {
                Err(ModuleError::non_fatal(1, "required piece numbers missing"))
            } else {
                Ok(())
            };
        };
        let numbers = decode_numbers(&piece.read(self.offset, self.max_bytes)?)?;
        if numbers.len() != 2 {
            return Err(ModuleError::non_fatal(
                1,
                "malformed pilot numbers: expected two endpoints",
            ));
        }
        let points = numbers
            .into_iter()
            .map(|x| {
                let xy = Point2::from_mm(x, 0.0);
                Point3WithWidth {
                    // exhaustive: pilot fixes every output field independently of defaults
                    x: units_to_mm(xy.x),
                    y: units_to_mm(xy.y),
                    z: 0.2,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                    overhang_distance_mm: None,
                }
            })
            .collect();
        output
            .push_sparse_path(ExtrusionPath3D {
                // exhaustive: controlled sparse path has no authored tool or order lock
                points,
                role: ExtrusionRole::SparseInfill,
                speed_factor: 1.0,
                tool_index: None,
                order_lock: None,
            })
            .map_err(|message| ModuleError::fatal(1, message))
    }
}
