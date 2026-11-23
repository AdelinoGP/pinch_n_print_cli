//! Canonical `resolution` floor: `DynamicPrintConfig::normalize_fdm` raises an
//! explicitly-set `resolution` below 1 µm to 1 µm ("Resolution will be above
//! 1um"), so canonical cannot express "disabled".
//!
//! The clamp lives in the resolver rather than at the consumer, because the
//! G-code config appendix discloses the resolved config: clamping at the point
//! of use only would disclose the authored number while applying a different
//! one — the disclosure defect perf-vs-orca ticket 80 measured.

use std::collections::BTreeMap;

use slicer_config::{
    assemble_registry, resolve_scope_stack, ConfigSchemaRegistry, ConfigScope, ExpansionContext,
    HostChannels, ModuleDeclaration, ResolutionTarget, ScopeDelta, ScopedConfig,
};
use slicer_ir::config_schema::{ConfigFieldEntry, ConfigSchema};
use slicer_ir::ConfigValue;

const FLOOR_MM: f32 = 0.001;
const DEFAULT_MM: f32 = 0.01;

fn registry() -> ConfigSchemaRegistry {
    assemble_registry(
        &[ModuleDeclaration {
            module_id: "dev.pinch.test.resolution-floor".to_owned(),
            schema: ConfigSchema {
                entries: BTreeMap::from([(
                    "resolution".to_owned(),
                    ConfigFieldEntry {
                        field_type: "float".to_owned(),
                        default: Some("0.01".to_owned()),
                        min: Some(0.0),
                        ..ConfigFieldEntry::default()
                    },
                )]),
            },
            ..ModuleDeclaration::default()
        }],
        &HostChannels::from_live(),
    )
    .expect("fixture registry must assemble")
    .registry
}

fn resolve_authored(authored: f64) -> f32 {
    resolve_scope_stack(
        &registry(),
        &ScopedConfig {
            deltas: [(
                ConfigScope::Global,
                ScopeDelta {
                    values: BTreeMap::from([(
                        "resolution".to_owned(),
                        ConfigValue::Float(authored),
                    )]),
                },
            )]
            .into_iter()
            .collect(),
            layer_ranges: Default::default(),
        },
        &ResolutionTarget::default(),
        &ExpansionContext {
            nozzle_diameter_mm: 0.4,
            ..ExpansionContext::default()
        },
    )
    .expect("resolution is an admissible print-level key")
    .resolution()
}

#[test]
fn authored_below_floor_is_raised_to_the_canonical_floor() {
    for authored in [0.0_f64, 0.0004, 0.000_999] {
        let resolved = resolve_authored(authored);
        assert!(
            (resolved - FLOOR_MM).abs() < 1e-9,
            "authored {authored} mm resolved to {resolved}, expected the {FLOOR_MM} mm floor"
        );
    }
}

#[test]
fn authored_at_or_above_floor_is_untouched() {
    for authored in [0.001_f64, DEFAULT_MM as f64, 0.05] {
        let resolved = resolve_authored(authored);
        assert!(
            (resolved - authored as f32).abs() < 1e-9,
            "authored {authored} mm resolved to {resolved} and must not be rewritten"
        );
    }
}

#[test]
fn the_default_resolves_to_the_canonical_default_not_the_floor() {
    // The floor is a clamp, not the default: an unauthored config must still
    // disclose OrcaSlicer's 0.01 mm.
    let resolved = resolve_authored(DEFAULT_MM as f64);
    assert!((resolved - DEFAULT_MM).abs() < 1e-9);
}
