//! Layout-compatibility tests for the unchanged 3.0.0 `RegionMapIR` Postcard
//! shape (reverted packet-10 migration: no fixed typed field, no version
//! dispatch).
//!
//! # Contract this target pins
//!
//! - `CURRENT_REGION_MAP_IR_SCHEMA_VERSION` is 3.0.0, and the frozen
//!   pre-change Postcard fixture (recorded at revision `6ba38c34`, before the
//!   reverted fixed-field addition) decodes through the plain
//!   `postcard`/serde `RegionMapIR` shape unchanged: same field count, same
//!   field order, no remainder bytes, no version upgrade.
//! - The frozen fixture's expected values come from the hand-authored
//!   `region_map_v3_0_0.expected.json` companion — never from this test, the
//!   current encoder, or the production decoder.
//! - The automatic volumetric maximum lives in the
//!   `ResolvedConfig::extensions` carrier. An absent key is the "unavailable"
//!   state: `ResolvedConfig::filament_max_volumetric_speed` returns `0.0` for
//!   every config and every paint override decoded from the frozen fixture.
//! - A current-layout map that carries a non-zero extension value at BOTH
//!   nested sites (`RegionMapIR.configs` and `RegionPlan.paint_overrides`)
//!   round-trips with full map equality, and the accessor returns the literal
//!   magnitudes.
//!
//! The frozen fixture bytes and their expected-values companion are preserved
//! on disk exactly as recorded; this test never regenerates them.

use std::collections::{BTreeMap, HashMap};

use slicer_ir::{
    ConfigId, PaintSemantic, PaintValue, RegionKey, RegionMapIR, RegionPlan, ResolvedConfig,
    SemVer, CURRENT_REGION_MAP_IR_SCHEMA_VERSION,
};

/// The extension-carrier key read by
/// [`ResolvedConfig::filament_max_volumetric_speed`].
const VOLUMETRIC_KEY: &str = "filament_max_volumetric_speed";

/// Frozen pre-change Postcard bytes recorded from the `git archive` extract
/// named in `region_map_v3_0_0.provenance.txt`.
const FROZEN_FIXTURE_BYTES: &[u8] =
    include_bytes!("fixtures/region_map_v3_0_0/region_map_v3_0_0.postcard");

/// Hand-authored expected-values companion; the only oracle used for the
/// frozen payload.
const FROZEN_EXPECTED_JSON: &str =
    include_str!("fixtures/region_map_v3_0_0/region_map_v3_0_0.expected.json");

#[derive(serde::Deserialize)]
struct ExpectedFixture {
    fixture: String,
    schema_version: ExpectedSemVer,
    new_field: ExpectedNewField,
    configs: Vec<ExpectedConfig>,
    entries: Vec<ExpectedEntry>,
    fixture_bytes: usize,
}

#[derive(serde::Deserialize)]
struct ExpectedSemVer {
    major: u32,
    minor: u32,
    patch: u32,
}

impl ExpectedSemVer {
    fn to_sem_ver(&self) -> SemVer {
        SemVer {
            major: self.major,
            minor: self.minor,
            patch: self.patch,
        }
    }
}

#[derive(serde::Deserialize)]
struct ExpectedNewField {
    name: String,
    present_in_fixture: bool,
    expected_default_after_decode: f64,
    expected_default_bits: String,
}

#[derive(serde::Deserialize)]
struct ExpectedConfig {
    index: usize,
    filament_diameter: f64,
    outer_wall_speed: f64,
}

#[derive(serde::Deserialize)]
struct ExpectedEntry {
    region_key: ExpectedRegionKey,
    config_index: u32,
    stage_modules_empty: bool,
    paint_overrides: Vec<ExpectedPaintOverride>,
}

#[derive(serde::Deserialize)]
struct ExpectedRegionKey {
    global_layer_index: u32,
    object_id: String,
    region_id: u64,
    variant_chain: serde_json::Value,
}

impl ExpectedRegionKey {
    fn to_region_key(&self) -> RegionKey {
        RegionKey {
            global_layer_index: self.global_layer_index,
            object_id: self.object_id.clone(),
            region_id: self.region_id,
            variant_chain: serde_json::from_value(self.variant_chain.clone())
                .expect("frozen variant_chain follows the IR serde shape"),
        }
    }
}

#[derive(serde::Deserialize)]
struct ExpectedPaintOverride {
    semantic: String,
    filament_diameter: f64,
    outer_wall_speed: f64,
}

/// The JSON companion's hex spelling of the absent key's decoded default
/// (`0x00000000` here, i.e. `+0.0`), returned as the `f64` the accessor
/// yields. Parsing the oracle's own spelling keeps the expectation driven by
/// the JSON rather than restated in this test.
fn expected_default_from_hex(expected_bits: &str) -> f64 {
    let hex = expected_bits
        .strip_prefix("0x")
        .expect("expected_default_bits is written with a 0x prefix");
    let bits = u32::from_str_radix(hex, 16).expect("expected_default_bits is hexadecimal");
    let zero = f32::from_bits(bits);
    assert_eq!(
        zero, 0.0,
        "the companion's bits spelling must decode to zero"
    );
    assert!(
        zero.is_sign_positive(),
        "the companion pins the positive zero bit pattern"
    );
    f64::from(zero)
}

/// Current-layout control: a 3.0.0 `RegionMapIR` carrying a non-default
/// `filament_max_volumetric_speed` in BOTH nested serialized sites — the
/// interned `RegionMapIR.configs` pool and `RegionPlan.paint_overrides`.
fn build_extension_carrying_region_map() -> RegionMapIR {
    let mut region_map = RegionMapIR::default();
    let mut interned = ResolvedConfig {
        filament_diameter: 1.72,
        outer_wall_speed: 61.5,
        ..ResolvedConfig::default()
    };
    interned.extensions.insert(
        VOLUMETRIC_KEY.to_string(),
        slicer_ir::ConfigValue::Float(9.5),
    );
    let config_id = region_map.intern_config(interned);

    let mut override_config = ResolvedConfig {
        filament_diameter: 2.85,
        outer_wall_speed: 25.0,
        ..ResolvedConfig::default()
    };
    override_config.extensions.insert(
        VOLUMETRIC_KEY.to_string(),
        slicer_ir::ConfigValue::Float(7.25),
    );
    let mut paint_overrides = BTreeMap::new();
    paint_overrides.insert(PaintSemantic::Material, override_config);

    region_map.entries.insert(
        RegionKey {
            global_layer_index: 5,
            object_id: "obj-current-1".to_string(),
            region_id: 7,
            variant_chain: vec![("material".to_string(), PaintValue::ToolIndex(1))],
        },
        RegionPlan {
            config: config_id,
            stage_modules: HashMap::new(),
            paint_overrides,
        },
    );
    region_map
}

#[test]
fn frozen_region_map_3_0_0_fixture_decodes_unchanged_with_absent_extension_default() {
    let expected: ExpectedFixture =
        serde_json::from_str(FROZEN_EXPECTED_JSON).expect("expected-values companion parses");

    // Oracle self-checks: the companion describes the bytes actually loaded and
    // covers the frozen payload's required shape (two configs, at least one
    // non-empty paint_overrides payload).
    assert_eq!(expected.fixture, "region_map_v3_0_0.postcard");
    assert_eq!(
        expected.fixture_bytes,
        FROZEN_FIXTURE_BYTES.len(),
        "expected-values companion describes a different fixture byte length"
    );
    assert_eq!(
        expected.configs.len(),
        2,
        "frozen fixture carries two configs"
    );
    assert_eq!(
        expected.entries.len(),
        2,
        "frozen fixture carries two region entries"
    );
    assert!(
        expected
            .entries
            .iter()
            .any(|e| !e.paint_overrides.is_empty()),
        "frozen fixture must exercise a non-empty paint_overrides payload"
    );
    assert_eq!(expected.new_field.name, VOLUMETRIC_KEY);
    assert!(
        !expected.new_field.present_in_fixture,
        "the frozen pre-change payload cannot carry the extension key"
    );
    assert_eq!(expected.new_field.expected_default_after_decode, 0.0);
    let expected_absent_default =
        expected_default_from_hex(&expected.new_field.expected_default_bits);

    // The reverted packet-10 migration leaves the fixture's layout = the
    // current layout: the plain serde shape must consume all bytes.
    let (decoded, remainder) = postcard::take_from_bytes::<RegionMapIR>(FROZEN_FIXTURE_BYTES)
        .expect("frozen 3.0.0 Postcard fixture must decode through the current RegionMapIR shape");
    assert!(
        remainder.is_empty(),
        "the fixture must decode without remainder bytes; {} byte(s) left over",
        remainder.len()
    );

    assert_eq!(
        CURRENT_REGION_MAP_IR_SCHEMA_VERSION,
        SemVer {
            major: 3,
            minor: 0,
            patch: 0,
        },
        "the frozen fixture revision is the unchanged 3.0.0 layout"
    );
    assert_eq!(
        decoded.schema_version,
        expected.schema_version.to_sem_ver(),
        "the decoded map must carry the version the companion records"
    );
    assert_eq!(decoded.schema_version, CURRENT_REGION_MAP_IR_SCHEMA_VERSION);

    assert_eq!(decoded.configs.len(), expected.configs.len());
    for config in &expected.configs {
        let decoded_config = &decoded.configs[config.index];
        assert_eq!(
            decoded_config.filament_diameter.to_bits(),
            (config.filament_diameter as f32).to_bits(),
            "configs[{}].filament_diameter must equal the frozen expected value",
            config.index
        );
        assert_eq!(
            decoded_config.outer_wall_speed.to_bits(),
            (config.outer_wall_speed as f32).to_bits(),
            "configs[{}].outer_wall_speed must equal the frozen expected value",
            config.index
        );
        assert!(
            !decoded_config.extensions.contains_key(VOLUMETRIC_KEY),
            "configs[{}] was recorded before the extension key existed and must not carry it",
            config.index
        );
        let absent = decoded_config
            .filament_max_volumetric_speed()
            .expect("an absent extension key is the unavailable maximum, not an error");
        assert_eq!(
            absent, expected_absent_default,
            "configs[{}].filament_max_volumetric_speed must default to the companion's value",
            config.index
        );
        assert!(
            absent.is_sign_positive(),
            "the absent default must be +0.0, matching the companion's bit pattern"
        );
    }
    assert_eq!(
        decoded.configs[0],
        ResolvedConfig::default(),
        "expected-values companion records configs[0] as the RegionMapIR::default() pre-seed"
    );

    assert_eq!(decoded.entries.len(), expected.entries.len());
    for entry in &expected.entries {
        let key = entry.region_key.to_region_key();
        let plan = decoded
            .entries
            .get(&key)
            .unwrap_or_else(|| panic!("frozen RegionPlan entry missing for key {key:?}"));
        assert_eq!(
            plan.config,
            ConfigId(entry.config_index),
            "plan for {key:?} must reference the frozen interner index"
        );
        if entry.stage_modules_empty {
            assert!(
                plan.stage_modules.is_empty(),
                "stage_modules for {key:?} must be empty"
            );
        }
        assert_eq!(
            plan.paint_overrides.len(),
            entry.paint_overrides.len(),
            "paint_overrides cardinality for {key:?} must match the frozen payload"
        );
        for paint_override in &entry.paint_overrides {
            let semantic: PaintSemantic =
                serde_json::from_value(serde_json::Value::String(paint_override.semantic.clone()))
                    .expect("frozen paint semantic follows the IR serde shape");
            let decoded_override = plan.paint_overrides.get(&semantic).unwrap_or_else(|| {
                panic!("frozen {semantic:?} paint override missing for {key:?}")
            });
            assert_eq!(
                decoded_override.filament_diameter.to_bits(),
                (paint_override.filament_diameter as f32).to_bits(),
                "paint_overrides[{semantic:?}].filament_diameter for {key:?}"
            );
            assert_eq!(
                decoded_override.outer_wall_speed.to_bits(),
                (paint_override.outer_wall_speed as f32).to_bits(),
                "paint_overrides[{semantic:?}].outer_wall_speed for {key:?}"
            );
            assert!(
                !decoded_override.extensions.contains_key(VOLUMETRIC_KEY),
                "paint_overrides[{semantic:?}] for {key:?} predates the extension key"
            );
            assert_eq!(
                decoded_override.filament_max_volumetric_speed(),
                Ok(expected_absent_default),
                "paint_overrides[{semantic:?}].filament_max_volumetric_speed for {key:?} must \
                 default to the companion's value"
            );
        }
    }
}

/// Encode/decode pair: `postcard::to_allocvec` / `postcard::take_from_bytes`.
///
/// The extension magnitudes are asserted literally before and after the
/// round-trip, and the full-map equality check (which compares
/// `ResolvedConfig::extensions`) pins both nested sites.
// test-quality: encode/decode roundtrip pair — postcard::to_allocvec / postcard::take_from_bytes preserve RegionMapIR and both extension sites.
#[test]
fn region_map_roundtrip_preserves_extension_magnitudes_at_both_sites() {
    let control = build_extension_carrying_region_map();
    assert_eq!(control.schema_version, CURRENT_REGION_MAP_IR_SCHEMA_VERSION);
    assert_eq!(control.configs.len(), 2, "the control interns one config");
    assert_eq!(control.configs[1].filament_max_volumetric_speed(), Ok(9.5));
    let control_key = RegionKey {
        global_layer_index: 5,
        object_id: "obj-current-1".to_string(),
        region_id: 7,
        variant_chain: vec![("material".to_string(), PaintValue::ToolIndex(1))],
    };
    assert_eq!(
        control.entries[&control_key].paint_overrides[&PaintSemantic::Material]
            .filament_max_volumetric_speed(),
        Ok(7.25)
    );

    let bytes = postcard::to_allocvec(&control)
        .expect("the extension-carrying RegionMapIR is Postcard-serializable");
    let (decoded, remainder) = postcard::take_from_bytes::<RegionMapIR>(&bytes)
        .expect("the current-layout round-trip bytes must decode");
    assert!(
        remainder.is_empty(),
        "the round-trip payload must decode without remainder bytes; {} byte(s) left over",
        remainder.len()
    );
    assert_eq!(
        decoded, control,
        "the round-trip must preserve full map equality, including both extension sites"
    );
    assert_eq!(
        decoded.schema_version, CURRENT_REGION_MAP_IR_SCHEMA_VERSION,
        "the round-trip map keeps the current layout version"
    );

    assert_eq!(
        decoded.configs[1].filament_max_volumetric_speed(),
        Ok(9.5),
        "the interned configs pool must carry the literal non-zero extension magnitude"
    );
    assert_eq!(
        decoded.entries[&control_key].paint_overrides[&PaintSemantic::Material]
            .filament_max_volumetric_speed(),
        Ok(7.25),
        "paint_overrides must carry the literal non-zero extension magnitude"
    );
    assert_eq!(
        decoded
            .config_for(&control_key)
            .to_config_map()
            .get(VOLUMETRIC_KEY),
        Some(&slicer_ir::ConfigValue::Float(9.5)),
        "the decoded interned config's flattened map must expose the extension value"
    );
}

/// Distinct extension values must participate in interner identity, and equal
/// extension values must dedupe: the accessor's values are part of
/// `ResolvedConfig`'s map equality/hash, so the config pool cannot collapse
/// two configs that differ only in the declared extension.
#[test]
fn distinct_extension_values_are_interned_separately_and_equal_ones_dedupe() {
    let with = |magnitude: f64| {
        let mut config = ResolvedConfig {
            filament_diameter: 1.75,
            ..ResolvedConfig::default()
        };
        config.extensions.insert(
            VOLUMETRIC_KEY.to_string(),
            slicer_ir::ConfigValue::Float(magnitude),
        );
        config
    };

    let mut region_map = RegionMapIR::default();
    let seeded = region_map.configs.len();
    let eight = region_map.intern_config(with(8.0));
    let twelve = region_map.intern_config(with(12.0));
    assert_ne!(
        eight, twelve,
        "configs differing only in the extension value must not share an interner id"
    );
    assert_eq!(
        region_map.configs.len(),
        seeded + 2,
        "both distinct extension-carrying configs must be retained in the pool"
    );

    let eight_again = region_map.intern_config(with(8.0));
    assert_eq!(
        eight, eight_again,
        "an equal extension-carrying config must dedupe onto its existing id"
    );
    assert_eq!(
        region_map.configs.len(),
        seeded + 2,
        "deduping a repeated config must not grow the pool"
    );

    assert_eq!(
        region_map
            .config_for_raw(eight)
            .filament_max_volumetric_speed(),
        Ok(8.0)
    );
    assert_eq!(
        region_map
            .config_for_raw(twelve)
            .filament_max_volumetric_speed(),
        Ok(12.0),
        "each interner id must retain its own literal extension magnitude"
    );
}
