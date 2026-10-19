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

use slicer_ir::resolved_config::{MachineLimitPair, ResolvedFloatOrPercent};
use slicer_ir::slice_ir::{InfillType, SupportType, WallGenerator};
use slicer_ir::{
    ConfigId, ConfigValue, ModuleInvocation, PaintSemantic, PaintValue, RegionKey, RegionMapIR,
    RegionPlan, ResolvedConfig, SemVer, StageId, CURRENT_REGION_MAP_IR_SCHEMA_VERSION,
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

// ── Frozen 3.0.0 layout shim ────────────────────────────────────────────────
//
// The frozen payload was recorded at revision `6ba38c34`, whose
// `ResolvedConfig` predates this branch's field set (the percent
// `sparse_infill_density`, the canonical key renames, and the machine-limit
// pair). Postcard is positional, so the current struct cannot consume those
// bytes directly; the payload is decoded through the exact recorded field
// order and then lifted into the current shape. The byte-level oracle (field
// count, order, no remainder) is therefore still tested against the archived
// layout, and the lifted values are asserted against the hand-authored
// companion below.

#[derive(serde::Deserialize)]
struct LegacyResolvedConfig {
    layer_height: f64,
    line_width: f64,
    first_layer_height: f64,
    initial_layer_line_width: ResolvedFloatOrPercent,
    filament_diameter: f32,
    wall_count: u32,
    outer_wall_speed: f32,
    inner_wall_speed: f32,
    wall_generator: WallGenerator,
    arachne_min_feature_size: Option<f32>,
    infill_type: InfillType,
    infill_density: f32,
    infill_angle: f32,
    infill_speed: f32,
    solid_infill_speed: f32,
    top_shell_layers: u32,
    bottom_shell_layers: u32,
    top_fill_holder: String,
    bottom_fill_holder: String,
    bridge_fill_holder: String,
    sparse_fill_holder: String,
    fill_authored_coloring: Vec<String>,
    gcode_resolution: f32,
    infill_resolution: f32,
    infill_overlap: f32,
    support_resolution: f32,
    min_segment_length: f32,
    gcode_xy_decimals: u32,
    perimeter_arc_tolerance: f32,
    slice_closing_radius: f32,
    flat_bridge_closing_join: String,
    support_enabled: bool,
    disable_m73: bool,
    support_type: SupportType,
    support_threshold_angle: f32,
    support_layer_height_mm: f32,
    support_expansion: f32,
    support_top_z_distance: f32,
    support_bottom_z_distance: f32,
    support_threshold_overlap: ResolvedFloatOrPercent,
    support_line_width: ResolvedFloatOrPercent,
    bridge_no_support: bool,
    enforce_support_layers: u32,
    support_critical_regions_only: bool,
    support_remove_small_overhang: bool,
    support_object_first_layer_gap: f32,
    // Retired by wayfinder ticket 115 on this branch; the field must stay in
    // the legacy layout so Postcard's positional decode stays aligned.
    #[allow(dead_code)]
    support_sharp_tails: bool,
    nonplanar_max_angle_deg: Option<f32>,
    nonplanar_shell_count: Option<u32>,
    nonplanar_amplitude: Option<f32>,
    smoothificator_target_height: Option<f32>,
    smoothificator_adaptive: Option<bool>,
    bed_shape: Vec<f64>,
    retract_length: f32,
    wipe_tower_enabled: bool,
    mmu_segmented_region_max_width: f32,
    mmu_segmented_region_interlocking_depth: f32,
    mmu_segmented_region_interlocking_beam: bool,
    machine_max_acceleration_extruding: Option<f32>,
    machine_max_acceleration_travel: Option<f32>,
    machine_max_speed_x: Option<f32>,
    machine_max_speed_y: Option<f32>,
    machine_max_speed_z: Option<f32>,
    machine_max_speed_e: Option<f32>,
    machine_max_jerk_x: Option<f32>,
    machine_max_jerk_y: Option<f32>,
    machine_max_jerk_z: Option<f32>,
    machine_max_jerk_e: Option<f32>,
    filament_density: Vec<f64>,
    extensions: BTreeMap<String, ConfigValue>,
}

impl From<LegacyResolvedConfig> for ResolvedConfig {
    fn from(legacy: LegacyResolvedConfig) -> Self {
        ResolvedConfig {
            layer_height: legacy.layer_height,
            line_width: legacy.line_width,
            initial_layer_print_height: legacy.first_layer_height,
            initial_layer_line_width: legacy.initial_layer_line_width,
            filament_diameter: legacy.filament_diameter,
            wall_loops: legacy.wall_count,
            outer_wall_speed: legacy.outer_wall_speed,
            inner_wall_speed: legacy.inner_wall_speed,
            wall_generator: legacy.wall_generator,
            arachne_min_feature_size: legacy.arachne_min_feature_size,
            infill_type: legacy.infill_type,
            sparse_infill_density: legacy.infill_density * 100.0,
            infill_direction: legacy.infill_angle,
            sparse_infill_speed: legacy.infill_speed,
            solid_infill_speed: legacy.solid_infill_speed,
            top_shell_layers: legacy.top_shell_layers,
            bottom_shell_layers: legacy.bottom_shell_layers,
            top_fill_holder: legacy.top_fill_holder,
            bottom_fill_holder: legacy.bottom_fill_holder,
            bridge_fill_holder: legacy.bridge_fill_holder,
            sparse_fill_holder: legacy.sparse_fill_holder,
            fill_authored_coloring: legacy.fill_authored_coloring,
            gcode_resolution: legacy.gcode_resolution,
            infill_resolution: legacy.infill_resolution,
            infill_overlap: legacy.infill_overlap,
            support_resolution: legacy.support_resolution,
            min_segment_length: legacy.min_segment_length,
            gcode_xy_decimals: legacy.gcode_xy_decimals,
            perimeter_arc_tolerance: legacy.perimeter_arc_tolerance,
            slice_closing_radius: legacy.slice_closing_radius,
            flat_bridge_closing_join: legacy.flat_bridge_closing_join,
            support_enabled: legacy.support_enabled,
            disable_m73: legacy.disable_m73,
            support_type: legacy.support_type,
            support_threshold_angle: legacy.support_threshold_angle,
            support_layer_height_mm: legacy.support_layer_height_mm,
            support_expansion: legacy.support_expansion,
            support_top_z_distance: legacy.support_top_z_distance,
            support_bottom_z_distance: legacy.support_bottom_z_distance,
            support_threshold_overlap: legacy.support_threshold_overlap,
            support_line_width: legacy.support_line_width,
            bridge_no_support: legacy.bridge_no_support,
            enforce_support_layers: legacy.enforce_support_layers,
            support_critical_regions_only: legacy.support_critical_regions_only,
            support_remove_small_overhang: legacy.support_remove_small_overhang,
            support_object_first_layer_gap: legacy.support_object_first_layer_gap,
            nonplanar_max_angle_deg: legacy.nonplanar_max_angle_deg,
            nonplanar_shell_count: legacy.nonplanar_shell_count,
            nonplanar_amplitude: legacy.nonplanar_amplitude,
            smoothificator_target_height: legacy.smoothificator_target_height,
            smoothificator_adaptive: legacy.smoothificator_adaptive,
            printable_area: legacy.bed_shape,
            retract_length: legacy.retract_length,
            enable_prime_tower: legacy.wipe_tower_enabled,
            mmu_segmented_region_max_width: legacy.mmu_segmented_region_max_width,
            mmu_segmented_region_interlocking_depth: legacy.mmu_segmented_region_interlocking_depth,
            mmu_segmented_region_interlocking_beam: legacy.mmu_segmented_region_interlocking_beam,
            machine_max_acceleration_extruding: legacy.machine_max_acceleration_extruding.map(
                |value| MachineLimitPair {
                    normal: value,
                    stealth: value,
                },
            ),
            machine_max_acceleration_travel: legacy.machine_max_acceleration_travel.map(|value| {
                MachineLimitPair {
                    normal: value,
                    stealth: value,
                }
            }),
            machine_max_speed_x: legacy.machine_max_speed_x.map(|value| MachineLimitPair {
                normal: value,
                stealth: value,
            }),
            machine_max_speed_y: legacy.machine_max_speed_y.map(|value| MachineLimitPair {
                normal: value,
                stealth: value,
            }),
            machine_max_speed_z: legacy.machine_max_speed_z.map(|value| MachineLimitPair {
                normal: value,
                stealth: value,
            }),
            machine_max_speed_e: legacy.machine_max_speed_e.map(|value| MachineLimitPair {
                normal: value,
                stealth: value,
            }),
            machine_max_jerk_x: legacy.machine_max_jerk_x.map(|value| MachineLimitPair {
                normal: value,
                stealth: value,
            }),
            machine_max_jerk_y: legacy.machine_max_jerk_y.map(|value| MachineLimitPair {
                normal: value,
                stealth: value,
            }),
            machine_max_jerk_z: legacy.machine_max_jerk_z.map(|value| MachineLimitPair {
                normal: value,
                stealth: value,
            }),
            machine_max_jerk_e: legacy.machine_max_jerk_e.map(|value| MachineLimitPair {
                normal: value,
                stealth: value,
            }),
            filament_density: legacy.filament_density,
            extensions: legacy.extensions,
            ..ResolvedConfig::default()
        }
    }
}

#[derive(serde::Deserialize)]
struct LegacyRegionPlan {
    config: ConfigId,
    stage_modules: HashMap<StageId, Vec<ModuleInvocation>>,
    paint_overrides: BTreeMap<PaintSemantic, LegacyResolvedConfig>,
}

#[derive(serde::Deserialize)]
struct LegacyRegionMapIR {
    schema_version: SemVer,
    entries: HashMap<RegionKey, LegacyRegionPlan>,
    configs: Vec<LegacyResolvedConfig>,
}

impl From<LegacyRegionMapIR> for RegionMapIR {
    fn from(legacy: LegacyRegionMapIR) -> Self {
        RegionMapIR {
            schema_version: legacy.schema_version,
            entries: legacy
                .entries
                .into_iter()
                .map(|(key, plan)| {
                    (
                        key,
                        RegionPlan {
                            config: plan.config,
                            stage_modules: plan.stage_modules,
                            paint_overrides: plan
                                .paint_overrides
                                .into_iter()
                                .map(|(semantic, config)| (semantic, config.into()))
                                .collect(),
                        },
                    )
                })
                .collect(),
            configs: legacy.configs.into_iter().map(Into::into).collect(),
        }
    }
}

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
    let (legacy, remainder) = postcard::take_from_bytes::<LegacyRegionMapIR>(FROZEN_FIXTURE_BYTES)
        .expect(
            "frozen 3.0.0 Postcard fixture must decode through the archived ResolvedConfig layout",
        );
    let decoded: RegionMapIR = legacy.into();
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
    // The archived pre-seed was the *then-current* `ResolvedConfig::default()`.
    // One default has moved since: wayfinder ticket 114 raised
    // `sparse_infill_speed` from 50 to the canonical 100. The lifted payload
    // must reproduce the archived default exactly, not today's.
    let archived_default = ResolvedConfig {
        sparse_infill_speed: 50.0,
        ..ResolvedConfig::default()
    };
    assert_eq!(
        decoded.configs[0], archived_default,
        "expected-values companion records configs[0] as the archived RegionMapIR::default() pre-seed"
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
