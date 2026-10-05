//! AC-5: the single-range fixture's equivalent typed input reaches both
//! production setup paths with one shared resolution.
//!
//! `layer_height = 0.1` over the half-open world-Z interval `[0.4, 0.8)`, base
//! `layer_height = 0.2`, object height 1.0. Both `run_slice_with_collector`
//! (ordinary slicing) and `prepare_prepass_context` (visual-debug's prefix)
//! must produce the identical schedule `[0.2, 0.4, 0.5, 0.6, 0.7, 0.8, 1.0]`
//! and resolve the in-range `infill_density = 0.35` at layer top `0.6`.
//!
//! The expectations are literals: no assertion derives its schedule, profile,
//! or config from production profile output.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_config::LayerRangeInput;
use slicer_ir::{
    BoundingBox3, ConfigValue, IndexedTriangleSet, MeshIR, ObjectMesh, Point3, Transform3d,
};
use slicer_runtime::run::{prepare_prepass_context, run_slice_with_collector, SliceRunOptions};

const OBJECT_ID: &str = "layer-range-scope-cube";
const RANGE_INFILL_DENSITY: f64 = 0.35;
const BASE_INFILL_DENSITY: f64 = 0.20;

/// The fixture-equivalent schedule, as literal f32 layer tops.
const EXPECTED_TOPS: [f32; 7] = [0.2, 0.4, 0.5, 0.6, 0.7, 0.8, 1.0];
/// The uniform grid the same input produces without the range. A schedule
/// implementation that ignores explicit top-Zs lands here instead.
const UNIFORM_TOPS: [f32; 5] = [0.2, 0.4, 0.6, 0.8, 1.0];
/// Object-local top Z of the layer whose config must carry the range value.
const IN_RANGE_TOP_Z: f32 = 0.6;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root must resolve")
}

fn core_modules_dir() -> PathBuf {
    workspace_root().join("modules").join("core-modules")
}

/// A closed 10 x 10 x 1.0 mm box: `world_z_extent` matches the fixture model,
/// and the footprint is large enough for sparse infill to emit real geometry.
fn cube(origin: f32, extent: f32) -> IndexedTriangleSet {
    let lo = origin;
    let hi = origin + extent;
    IndexedTriangleSet {
        vertices: vec![
            Point3 {
                x: lo,
                y: lo,
                z: lo,
            },
            Point3 {
                x: hi,
                y: lo,
                z: lo,
            },
            Point3 {
                x: hi,
                y: hi,
                z: lo,
            },
            Point3 {
                x: lo,
                y: hi,
                z: lo,
            },
            Point3 {
                x: lo,
                y: lo,
                z: hi,
            },
            Point3 {
                x: hi,
                y: lo,
                z: hi,
            },
            Point3 {
                x: hi,
                y: hi,
                z: hi,
            },
            Point3 {
                x: lo,
                y: hi,
                z: hi,
            },
        ],
        indices: vec![
            0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7,
            6, 3, 0, 4, 3, 4, 7,
        ],
    }
}

fn fixture_mesh() -> Arc<MeshIR> {
    Arc::new(MeshIR {
        // exhaustive: object geometry and its world-Z extent define the fixture.
        objects: vec![ObjectMesh {
            id: OBJECT_ID.to_string(),
            mesh: cube(0.0, 10.0),
            transform: Transform3d {
                matrix: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
            },
            world_z_extent: Some((0.0, 1.0)),
            ..Default::default()
        }],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 200.0,
                y: 200.0,
                z: 200.0,
            },
        },
        ..Default::default()
    })
}

/// The fixture's typed equivalent: global base heights, then one authored
/// `layer_height = 0.1` plus `infill_density = 0.35` over `[0.4, 0.8)`.
fn config_source() -> HashMap<String, ConfigValue> {
    HashMap::from([
        ("layer_height".to_string(), ConfigValue::Float(0.2)),
        ("first_layer_height".to_string(), ConfigValue::Float(0.2)),
        (
            "infill_density".to_string(),
            ConfigValue::Float(BASE_INFILL_DENSITY),
        ),
        ("nozzle_diameter".to_string(), ConfigValue::Float(0.4)),
        ("line_width".to_string(), ConfigValue::Float(0.4)),
        (
            "wall_generator".to_string(),
            ConfigValue::String("classic".to_string()),
        ),
        ("enable_support".to_string(), ConfigValue::Bool(false)),
    ])
}

fn layer_ranges() -> Vec<LayerRangeInput> {
    // exhaustive: this transport fixture pins every LayerRangeInput field explicitly
    vec![LayerRangeInput {
        object_id: OBJECT_ID.to_string(),
        source_index: 0,
        min_z: 0.4,
        max_z: 0.8,
        values: std::collections::BTreeMap::from([
            ("layer_height".to_owned(), "0.1".to_owned()),
            ("infill_density".to_owned(), "0.35".to_owned()),
        ]),
    }]
}

fn gcode_zs(gcode: &str) -> Vec<f32> {
    gcode
        .lines()
        .filter_map(|line| line.strip_prefix(";Z:"))
        .map(|value| value.parse::<f32>().expect(";Z value must be numeric"))
        .collect()
}

fn assert_literal_schedule(actual: &[f32], expected: &[f32], path: &str) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "{path}: schedule length must match the literal fixture schedule; \
         got {actual:?}, expected {expected:?}"
    );
    let observed = actual;
    for (index, (value, expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (value - expected).abs() < 1.0e-5,
            "{path}: layer {index} top must be {expected}, got {value} \
             (full schedule {observed:?})"
        );
    }
}

/// AC-5: both production setup paths share one typed range resolution.
#[test]
fn both_production_entry_points_share_range_resolution() {
    let mesh = fixture_mesh();
    let source = config_source();
    let module_dirs = vec![core_modules_dir()];
    let ranges = layer_ranges();

    // Ordinary slicing: the real pipeline through `run_slice_with_collector`.
    let outcome = run_slice_with_collector(
        SliceRunOptions {
            mesh: Arc::clone(&mesh),
            model_label: "layer-range-scope-fixture".to_string(),
            module_dirs: module_dirs.clone(),
            no_default_module_paths: true,
            config_overrides: source.clone(),
            layer_ranges: ranges.clone(),
            ..SliceRunOptions::default()
        },
        None,
    )
    .expect("run_slice_with_collector must accept the typed layer range");

    let slice_zs = gcode_zs(&outcome.gcode_text);
    assert_literal_schedule(&slice_zs, &EXPECTED_TOPS, "run_slice_with_collector");
    assert_ne!(
        slice_zs, UNIFORM_TOPS,
        "the emitted schedule must be the range-derived mixed sequence, not the \
         uniform grid the same input produces without the range"
    );

    // The ordinary path's own resolved per-region config: the slice returns the
    // `RegionMapIR` its prepass committed, so the in-range value is observed on
    // the path that actually sliced rather than inferred from the other one.
    let slice_region_map = outcome
        .region_map
        .as_ref()
        .expect("the ordinary slice's prepass must commit a region map");
    assert_literal_region_value(
        slice_region_map,
        &slice_zs,
        IN_RANGE_TOP_Z,
        RANGE_INFILL_DENSITY,
        "run_slice_with_collector in-range layer",
    );
    assert_literal_region_value(
        slice_region_map,
        &slice_zs,
        1.0,
        BASE_INFILL_DENSITY,
        "run_slice_with_collector layer above the range",
    );

    // Visual-debug's prefix: the same typed input through `prepare_prepass_context`.
    let context =
        prepare_prepass_context(Arc::clone(&mesh), source, ranges, &module_dirs, true, false)
            .expect("prepare_prepass_context must accept the same typed layer range");

    let prepass_tops: Vec<f32> = context
        .plan
        .global_layers
        .iter()
        .map(|layer| layer.z)
        .collect();
    assert_literal_schedule(&prepass_tops, &EXPECTED_TOPS, "prepare_prepass_context");
    assert_eq!(
        slice_zs, prepass_tops,
        "both production entry points must produce the identical Z sequence"
    );

    // Both paths must resolve the in-range `infill_density` through the same
    // `resolve_scope_stack` target at the layer top `0.6`.
    let region_map = context
        .blackboard
        .region_map()
        .expect("region mapping must commit the resolved region configs");
    assert_literal_region_value(
        region_map,
        &prepass_tops,
        IN_RANGE_TOP_Z,
        RANGE_INFILL_DENSITY,
        "prepare_prepass_context in-range layer",
    );
    assert_literal_region_value(
        region_map,
        &prepass_tops,
        1.0,
        BASE_INFILL_DENSITY,
        "prepare_prepass_context layer above the range",
    );
}

/// Regression: world-Z height ranges must be translated before the guest adds
/// the shared raft displacement, not displaced along with the object afterwards.
#[test]
fn raft_offset_keeps_height_range_in_authored_world_z() {
    let mesh = fixture_mesh();
    let mut source = config_source();
    source.insert("support_raft_layers".to_owned(), ConfigValue::Int(2));
    let module_dirs = vec![core_modules_dir()];
    let ranges = layer_ranges();
    let expected = [0.2, 0.4, 0.6, 0.7, 0.8, 1.0, 1.2, 1.4];

    // Observe the production layer plan, not emitted G-code: empty raft layers
    // need not emit a ;Z: marker, but still belong to the scheduled Z grid.
    let context = prepare_prepass_context(mesh, source, ranges, &module_dirs, true, false)
        .expect("raft visual-debug prepass must succeed");
    let prepass_tops: Vec<f32> = context
        .plan
        .global_layers
        .iter()
        .map(|layer| layer.z)
        .collect();
    assert_literal_schedule(&prepass_tops, &expected, "raft visual-debug prepass");
    println!("raft world tops: prepass={prepass_tops:?}");
}

/// Assert that the region config at the layer whose top is `layer_top` carries
/// `expected_infill_density`, resolved through the committed `RegionMapIR`.
///
/// Reads the map the slice itself committed; it derives nothing from the module
/// set or from profile output.
fn assert_literal_region_value(
    region_map: &slicer_ir::RegionMapIR,
    schedule: &[f32],
    layer_top: f32,
    expected_infill_density: f64,
    label: &str,
) {
    let layer_index = schedule
        .iter()
        .position(|z| (z - layer_top).abs() < 1.0e-5)
        .unwrap_or_else(|| panic!("{label}: layer top {layer_top} must exist in {schedule:?}"));
    let mut observed = 0usize;
    for (key, plan) in region_map.entries.iter() {
        if key.global_layer_index as usize != layer_index {
            continue;
        }
        observed += 1;
        let resolved = region_map.config_for_raw(plan.config);
        assert!(
            (resolved.infill_density - expected_infill_density as f32).abs() < 1.0e-6,
            "{label}: region {key:?} must resolve infill_density = \
             {expected_infill_density}, got {}",
            resolved.infill_density
        );
    }
    assert!(
        observed > 0,
        "{label}: the committed region map must carry an entry for layer \
         index {layer_index} (top {layer_top})"
    );
}

/// A uniform control: the same slice without the typed range produces the
/// uniform grid, so the mixed schedule above cannot be an artifact of the
/// fixture geometry, the module set, or the emitted `;Z:` parsing.
///
/// This is the differential negative control for the schedule: if the range
/// were silently dropped, `both_production_entry_points_share_range_resolution`
/// would observe this same uniform sequence and fail its length assertion.
#[test]
fn range_less_control_slices_the_uniform_grid() {
    let outcome = run_slice_with_collector(
        SliceRunOptions {
            mesh: fixture_mesh(),
            model_label: "layer-range-scope-control".to_string(),
            module_dirs: vec![core_modules_dir()],
            no_default_module_paths: true,
            config_overrides: config_source(),
            ..SliceRunOptions::default()
        },
        None,
    )
    .expect("ordinary slice without a typed range must succeed");

    let control_zs = gcode_zs(&outcome.gcode_text);
    assert_literal_schedule(&control_zs, &UNIFORM_TOPS, "range-less control");
    assert_ne!(
        control_zs, EXPECTED_TOPS,
        "the range-less control must not reproduce the range-derived schedule"
    );
}
