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
//! is the module's job (it knows its own `on_print_start` default). Keeping
//! the resolution here at the per-region level is what makes the per-region
//! partition contract (packet 131) testable in isolation.

use crate::views::SliceRegionView;
use slicer_ir::ConfigValue;

/// Resolve a `f32` config value for a region.
///
/// Returns the value at `key` from the region's per-region `ConfigView` if
/// present and a `Float`/`FloatOrPercent(literal)`, otherwise the supplied
/// `fallback`. The fallback is what a module passes from its module-global
/// `on_print_start`-derived default.
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
        assert_eq!(resolve_float(&region, "infill_density", 0.2), 0.2);
    }

    #[test]
    fn resolve_float_uses_per_region_when_present() {
        let mut region = empty_region();
        let mut fields = std::collections::HashMap::new();
        fields.insert(
            slicer_ir::ConfigKey::from("infill_density"),
            ConfigValue::Float(0.5),
        );
        region.set_config(slicer_ir::ConfigView::from_map(fields));
        assert_eq!(resolve_float(&region, "infill_density", 0.2), 0.5);
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
        assert_eq!(resolve_float(&region, "infill_density", 0.2), 0.2);
    }
}
