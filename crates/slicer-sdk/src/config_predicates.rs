//! Cross-module config predicates.
//!
//! A predicate that decides whether one module's output is consumed by another
//! module must be evaluated identically on both sides. These helpers are the
//! single definition for such shared predicates, so a producer cannot skip
//! work that a consumer still reads (or keep doing work nobody reads).

use slicer_ir::slice_ir::ConfigReadError;
use slicer_ir::ConfigView;

/// Whether this slice's config makes any consumer read the per-vertex
/// overhang annotations (`overhang_quartile` / `overhang_distance_mm`).
///
/// This is exactly `OverhangClassifierDefault::run_finalization`'s liveness
/// predicate: the classifier returns before reading a single stamp when
/// `enable_overhang_speed` is false, or when all four
/// `overhang_{1..4}_4_speed` bands are zero. The classifier calls this same
/// helper, so the producer (`classic-perimeters` / `arachne-perimeters`) and
/// the consumer cannot disagree.
///
/// The bands are `float_or_percent` and are host-expanded to absolute values
/// before a module's `ConfigView` is bound, so `require_float` is the same
/// read the classifier has always performed.
pub fn overhang_annotations_consumed(config: &ConfigView) -> Result<bool, ConfigReadError> {
    if !config.require_bool("enable_overhang_speed")? {
        return Ok(false);
    }
    for key in [
        "overhang_1_4_speed",
        "overhang_2_4_speed",
        "overhang_3_4_speed",
        "overhang_4_4_speed",
    ] {
        if config.require_float(key)? != 0.0 {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Producer-side variant of [`overhang_annotations_consumed`]: a missing or
/// mistyped key reports "inactive" instead of failing the perimeter call.
///
/// The producer modules declare all five keys, so on the live path the host
/// seeds them into the resolved view and this fallback is unreachable. If it
/// is ever reached, skipping the stamps is safe: the consumer performs its
/// own required read of the same keys and aborts the slice on that same
/// config defect, so no output can be emitted with a reader that expected
/// values the producer did not stamp.
#[must_use]
pub fn overhang_annotations_consumed_or_inactive(config: &ConfigView) -> bool {
    overhang_annotations_consumed(config).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::{ConfigKey, ConfigValue};
    use std::collections::HashMap;

    fn view(enable: bool, bands: [f64; 4]) -> ConfigView {
        let mut fields = HashMap::new();
        fields.insert(
            ConfigKey::from("enable_overhang_speed"),
            ConfigValue::Bool(enable),
        );
        for (index, value) in bands.into_iter().enumerate() {
            fields.insert(
                ConfigKey::from(format!("overhang_{}_4_speed", index + 1).as_str()),
                ConfigValue::Float(value),
            );
        }
        ConfigView::from_map(fields)
    }

    #[test]
    fn all_zero_bands_are_unconsumed_even_when_enabled() {
        assert!(!overhang_annotations_consumed(&view(true, [0.0; 4])).unwrap());
    }

    #[test]
    fn disabled_switch_is_unconsumed_even_with_nonzero_bands() {
        assert!(!overhang_annotations_consumed(&view(false, [0.0, 0.0, 30.0, 0.0])).unwrap());
    }

    #[test]
    fn any_nonzero_band_with_the_switch_enabled_is_consumed() {
        assert!(overhang_annotations_consumed(&view(true, [0.0, 0.0, 0.0, 30.0])).unwrap());
        assert!(overhang_annotations_consumed(&view(true, [30.0, 0.0, 0.0, 0.0])).unwrap());
    }

    #[test]
    fn negative_zero_counts_as_zero_like_the_classifier_comparison() {
        assert!(!overhang_annotations_consumed(&view(true, [-0.0, 0.0, 0.0, 0.0])).unwrap());
    }

    #[test]
    fn missing_key_is_a_config_error_not_a_silent_skip() {
        assert!(overhang_annotations_consumed(&ConfigView::new()).is_err());
    }

    #[test]
    fn producer_variant_reports_inactive_for_a_missing_key() {
        assert!(!overhang_annotations_consumed_or_inactive(&ConfigView::new()));
        assert!(overhang_annotations_consumed_or_inactive(&view(
            true,
            [0.0, 0.0, 30.0, 0.0]
        )));
    }
}
