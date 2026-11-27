//! Per-region config resolution helpers.
//!
//! Modules that consume the per-region `ConfigView` (populated by the host's
//! partition step per packet 131 / TASK-256) need a uniform way to read a
//! scalar config key with a fallback to a module-global value when the
//! per-region view is absent or the key is not declared. This module owns
//! that resolution rule so the per-region vs. global semantics are not
//! reinvented in each infill/perimeter module.
//!
//! Resolution order (first match wins):
//! 1. The region's `ConfigView`, if `Some` and the key is present.
//! 2. The supplied fallback value.
//!
//! The helper does **not** fall back to a module-global `ConfigView` — that
//! is the module's job (it knows its own defaults from the `from_config`
//! constructor). Keeping
//! the resolution here at the per-region level is what makes the per-region
//! partition contract (packet 131) testable in isolation.

use crate::views::SliceRegionView;
use slicer_ir::ConfigValue;

/// Resolve a `f32` config value for a region.
///
/// Returns the value at `key` from the region's per-region `ConfigView` if
/// present and a `Float`/`FloatOrPercent(literal)`, otherwise the supplied
/// `fallback`. The fallback is supplied by the module's `from_config`
/// constructor.
#[must_use]
pub fn resolve_float(region: &SliceRegionView, key: &str, fallback: f32) -> f32 {
    let Some(view) = region.config() else {
        return fallback;
    };
    match view.get(key) {
        Some(ConfigValue::Float(f)) => *f as f32,
        Some(ConfigValue::FloatOrPercent {
            value,
            is_percent: false,
        }) => *value as f32,
        _ => fallback,
    }
}

/// Resolve a canonical-percent config value for a region, returning it as a
/// fraction of `1.0` (percent ÷ 100).
///
/// Canonical percent keys (`sparse_infill_density`, ...) surface in the
/// per-region view as `Float` percent values (typed/CLI config), percent-form
/// strings (`"20%"`, the form 3MF metadata preserves), and
/// `Percent`/`FloatOrPercent` variants. `ConfigView::get_abs_value` resolves
/// all of those against `base = 100.0`, yielding the literal percent; this
/// helper divides by 100 so modules consume the density fraction. Unresolvable
/// or absent keys return `fallback`.
///
/// Wayfinder ticket 107 introduced this so the percent-everywhere
/// standardisation of `sparse_infill_density` reaches per-region reads.
#[must_use]
pub fn resolve_percent_float(region: &SliceRegionView, key: &str, fallback: f32) -> f32 {
    let Some(view) = region.config() else {
        return fallback;
    };
    view.get_abs_value(key, 100.0)
        .map(|value| (value as f32) / 100.0)
        .unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_prelude::SliceRegionViewBuilder;

    fn empty_region() -> SliceRegionView {
        SliceRegionViewBuilder::new()
            .object_id("obj1")
            .region_id(1)
            .z(0.3)
            .build()
    }

    #[test]
    fn resolve_float_falls_back_when_no_config_view() {
        let region = empty_region();
        assert_eq!(resolve_float(&region, "example_float_key", 0.2), 0.2);
    }

    #[test]
    fn resolve_float_uses_per_region_when_present() {
        let mut region = empty_region();
        let mut fields = std::collections::HashMap::new();
        fields.insert(
            slicer_ir::ConfigKey::from("example_float_key"),
            ConfigValue::Float(0.5),
        );
        region.set_config(slicer_ir::ConfigView::from_map(fields));
        assert_eq!(resolve_float(&region, "example_float_key", 0.2), 0.5);
    }

    #[test]
    fn resolve_float_falls_back_when_key_absent() {
        let mut region = empty_region();
        let mut fields = std::collections::HashMap::new();
        fields.insert(
            slicer_ir::ConfigKey::from("line_width"),
            ConfigValue::Float(0.4),
        );
        region.set_config(slicer_ir::ConfigView::from_map(fields));
        assert_eq!(resolve_float(&region, "example_float_key", 0.2), 0.2);
    }

    // ── resolve_percent_float: canonical-percent keys (wayfinder ticket 107)

    fn percent_region(key_value: (slicer_ir::ConfigKey, ConfigValue)) -> SliceRegionView {
        let mut region = empty_region();
        let mut fields = std::collections::HashMap::new();
        fields.insert(key_value.0, key_value.1);
        region.set_config(slicer_ir::ConfigView::from_map(fields));
        region
    }

    #[test]
    fn resolve_percent_float_returns_fallback_when_no_config_view() {
        let region = empty_region();
        assert_eq!(
            resolve_percent_float(&region, "sparse_infill_density", 0.2),
            0.2
        );
    }

    #[test]
    fn resolve_percent_float_divides_float_percent_by_100() {
        let region = percent_region((
            slicer_ir::ConfigKey::from("sparse_infill_density"),
            ConfigValue::Float(20.0),
        ));
        assert_eq!(
            resolve_percent_float(&region, "sparse_infill_density", 0.2),
            0.2
        );
    }

    #[test]
    fn resolve_percent_float_parses_percent_string() {
        let region = percent_region((
            slicer_ir::ConfigKey::from("sparse_infill_density"),
            ConfigValue::String("40%".to_string()),
        ));
        assert_eq!(
            resolve_percent_float(&region, "sparse_infill_density", 0.2),
            0.4
        );
    }

    #[test]
    fn resolve_percent_float_resolves_percent_variant() {
        let region = percent_region((
            slicer_ir::ConfigKey::from("sparse_infill_density"),
            ConfigValue::Percent(25.0),
        ));
        assert_eq!(
            resolve_percent_float(&region, "sparse_infill_density", 0.2),
            0.25
        );
    }

    #[test]
    fn resolve_percent_float_falls_back_on_unresolvable() {
        let region = percent_region((
            slicer_ir::ConfigKey::from("sparse_infill_density"),
            ConfigValue::String("not-a-percent".to_string()),
        ));
        assert_eq!(
            resolve_percent_float(&region, "sparse_infill_density", 0.2),
            0.2
        );
    }
}
