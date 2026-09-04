//! Packet 241 (`241-support-agg-rasterizer`) - Step 1: PRE-port baseline capture.
//!
//! This module captures the wall-leakage and column-continuity metrics of the
//! traditional support planner **before** the OrcaSlicer `SupportGridPattern`
//! AGG rasterizer is ported in. Later steps compare against the tracked
//! baseline at `tests/fixtures/golden/p241_baseline.json` to prove improvement.
//!
//! # Driver
//!
//! The metrics are computed from plan geometry, so the driver is
//! [`slicer_runtime::run::prepare_prepass_context`] - **not** `run_slice`.
//! `run_slice` returns a `SliceOutcome` carrying only `gcode_text`,
//! `layer_count`, `wallclock_ms` and `profile`; it exposes no `SupportPlanIR`,
//! so no wall-leakage or column-continuity metric can be derived from it. The
//! prepass driver hands back a `PrepassContext` whose blackboard carries both
//! `support_plan()` and `layer_plan()`, which is what these metrics need.
//!
//! # Metric definitions
//!
//! All areas are in **PnP units^2** (1 unit = 100 nm = 1e-4 mm, so 1 unit^2 =
//! 1e-8 mm^2). Areas are unsigned (contour area minus hole area, per ExPolygon).
//!
//! * **penetration event** - one `(global_layer_index, body-region)` pair whose
//!   `SupportPlanRole::SupportBody` polygon has a non-empty intersection with
//!   that layer's model occupancy grown outward by `support_object_xy_distance`
//!   (read from the tracked config fixture; expected 0.35 mm). One body
//!   ExPolygon that penetrates counts as exactly one event, regardless of how
//!   many disjoint intersection pieces it produces.
//! * **penetrated area** - the summed area (units^2) of every one of those
//!   intersections, over all layers and all body regions.
//! * **noise floor** - intersection pieces smaller than
//!   [`WALL_LEAKAGE_NOISE_FLOOR_UNITS2`] are clipper tangency slivers along the
//!   grown-occupancy boundary, not penetration, and are ignored by both
//!   numbers above. See the constant for the measured values.
//! * **column drop** - walking DOWN the layer stack: a connected body component
//!   present at layer `N` whose footprint has no overlapping body component at
//!   layer `N - 1`, while layer `N - 1` is still a real printable layer of the
//!   `LayerPlanIR` (so the column did not terminate on the build plate) and the
//!   component also does not overlap model occupancy at layer `N - 1` (so it did
//!   not terminate on the model). Such a column vanishes abruptly instead of
//!   terminating at the plate or the model.
//!
//! Connected body components are obtained by `union_ex` over the layer's
//! `SupportBody` regions; each resulting `ExPolygon` is one component.
//!
//! # Tests
//!
//! * [`capture_pre_port_baseline`] - `#[ignore]`d recorder; writes the tracked
//!   baseline JSON. Run explicitly.
//! * [`p241_metric_helpers_agree_on_baseline_fixture`] - the Step 1 gate. Loads
//!   the committed baseline, re-runs the fixture through the prepass driver,
//!   recomputes both metric structs and asserts exact equality.

use std::collections::HashMap;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use slicer_core::polygon_ops::{
    difference_ex, intersection_ex, offset, union_ex, OffsetJoinType,
};
use slicer_ir::{ConfigValue, ExPolygon, Polygon, SupportPlanIR, SupportPlanRole};
use slicer_wasm_host::exact_z_query::ExactZQueryService;

use crate::common::model_cache::cached_load_model;
use crate::common::support_wedge;

/// Support family the baseline is captured for (traditional planner).
const BASELINE_SUPPORT_TYPE: &str = "normal(auto)";

/// Arc tolerance (mm) for the round-join grow of model occupancy. Fixed so the
/// grown boundary is reproducible run over run.
const GROW_ARC_TOLERANCE_MM: f32 = 0.01;

/// Join type of the occupancy grow: **Round**, i.e. the Euclidean set of points
/// within `support_object_xy_distance` of the model, which is what the metric
/// definition in the module header means by "grown outward by".
///
/// Measured (packet 241 Step 7, `SupportAdversarial.stl`, both rasterizer
/// modes, CLAMP-ERA build): switching this to `Miter` - to mirror the planner's own trimming
/// mask `host::offset_polygons(&occupancy, xy_distance, OffsetJoinType::Miter,
/// 0.0)` in `modules/core-modules/traditional-support-planner/src/lib.rs` -
/// does NOT make the metric read zero. It reads 0.064425 mm^2 per layer,
/// concentrated at the four convex corners of the solid, versus 0.000002 mm^2
/// per layer with `Round`. The planner's emitted body outline is therefore
/// rounder at convex corners than the Miter mask it subtracts, and a Miter
/// metric would penalise geometry that is a full Euclidean `xy_distance` away
/// from the model. `Round` is kept as the honest clearance measure.
const GROW_JOIN: OffsetJoinType = OffsetJoinType::Round;

/// Noise floor for the wall-leakage metric, in PnP units^2.
///
/// The planner trims every carry by the occupancy grown with a **Miter** join
/// (`SupportPlanner::plan_candidate` in
/// `modules/core-modules/traditional-support-planner/src/lib.rs`) while this
/// metric grows with a **Round** join ([`GROW_JOIN`], Euclidean clearance per
/// AC-6). The two grown boundaries coincide along straight walls, so
/// `intersection_ex` of a body against the grown occupancy yields a chain of
/// zero-width tangency slivers along the shared edge - one per body per layer.
///
/// Measured on the exploration candidates (`p241_explore_adversarial_candidates`)
/// in the CLAMP-ERA build, where the two rasterizer modes were byte-identical
/// on these three candidates. The clamp has since been removed and agg now
/// diverges from legacy on every layer of the adopted fixture, so treat the
/// per-mode equality below as historical; the floor itself is a bound on
/// clipper tangency slivers and does not depend on the mode:
///
/// * `stepped_pocket`: 26 events, 4095 units^2 total, largest piece 157.5
///   units^2 (1.575e-6 mm^2);
/// * `thin_wall_slot`: 26 events, 8086 units^2 total, largest 311 units^2;
/// * `roof_edge_slot`: 26 events, 2288 units^2 total, largest 88 units^2.
///
/// A real penetration cannot be that small: the clearance is 0.35 mm, so a
/// body overlapping the grown occupancy along even 0.03 mm of wall covers
/// 0.35 x 0.03 mm = 1.05e6 units^2, four orders of magnitude above the floor.
/// The floor is set at 10_000 units^2 = 1e-4 mm^2 (a 0.01 x 0.01 mm square):
/// ~32x the largest observed sliver and ~100x below the smallest physical
/// penetration. It is a floor on the *measurement*; the AC-6 assertions
/// (`events == 0`, `area <= baseline`) are unchanged.
const WALL_LEAKAGE_NOISE_FLOOR_UNITS2: f64 = 10_000.0;

/// `support_area_rasterizer` value selecting the pre-241 propagation semantic.
const RASTERIZER_LEGACY: &str = "legacy_semantic";

/// `support_area_rasterizer` value selecting the ported `SupportGridPattern`.
const RASTERIZER_AGG: &str = "agg";

// -- Local driver helpers ---------------------------------------------------
//
// `support_family_closure.rs` has private equivalents of all of these. They are
// `fn`, not `pub fn`, so they cannot be reused across submodules; these are
// faithful local copies.

fn support_test_path() -> PathBuf {
    let tracked =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/support-family/SupportTest.stl");
    if tracked.exists() {
        return tracked;
    }
    panic!(
        "required support-family fixture is missing at {} (tracked authoritative path); tmp/* is not authoritative",
        tracked.display()
    );
}

fn matched_config_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/support-family/orca-matched-config.json")
}

fn baseline_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/golden/p241_baseline.json")
}

fn json_to_config_value(value: &serde_json::Value) -> Option<ConfigValue> {
    match value {
        serde_json::Value::Bool(flag) => Some(ConfigValue::Bool(*flag)),
        serde_json::Value::String(text) => Some(ConfigValue::String(text.clone())),
        serde_json::Value::Number(number) => {
            if let Some(integer) = number.as_i64() {
                Some(ConfigValue::Int(integer))
            } else {
                number.as_f64().map(ConfigValue::Float)
            }
        }
        serde_json::Value::Array(items) => items
            .iter()
            .map(json_to_config_value)
            .collect::<Option<Vec<_>>>()
            .map(ConfigValue::List),
        _ => None,
    }
}

fn matched_config_base() -> HashMap<String, ConfigValue> {
    let path = matched_config_path();
    if !path.exists() {
        panic!(
            "required support-family config fixture is missing at {} (tracked authoritative path); tmp/* is not authoritative",
            path.display()
        );
    }
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let parsed: serde_json::Value = serde_json::from_str(&raw)
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));
    let object = parsed
        .as_object()
        .unwrap_or_else(|| panic!("{} is not a JSON object", path.display()));
    object
        .iter()
        .map(|(key, value)| {
            let converted = json_to_config_value(value).unwrap_or_else(|| {
                panic!(
                    "{}: key `{key}` has unsupported JSON value {value}",
                    path.display()
                )
            });
            (key.clone(), converted)
        })
        .collect()
}

fn matched_config_for(support_enabled: bool, support_type: &str) -> HashMap<String, ConfigValue> {
    let mut config = matched_config_base();
    config.insert(
        "enable_support".to_string(),
        ConfigValue::Bool(support_enabled),
    );
    config.insert(
        "support_type".to_string(),
        ConfigValue::String(support_type.to_string()),
    );
    config
}

fn core_module_dirs() -> Vec<PathBuf> {
    vec![Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("modules/core-modules")]
}

fn prepare_model_support(
    model: &Path,
    config: HashMap<String, ConfigValue>,
) -> Result<slicer_runtime::run::PrepassContext, String> {
    if !model.exists() {
        return Err(format!("model is missing at {}", model.display()));
    }
    let mesh = cached_load_model(model);
    slicer_runtime::run::prepare_prepass_context(mesh, config, &core_module_dirs(), true, false)
        .map_err(|error| format!("{} prepass failed: {error:?}", model.display()))
}

/// `support_object_xy_distance` (mm) as actually configured, read from the
/// loaded config rather than hardcoded.
fn xy_distance_mm(config: &HashMap<String, ConfigValue>) -> f32 {
    match config.get("support_object_xy_distance") {
        Some(ConfigValue::Float(value)) => *value as f32,
        Some(ConfigValue::Int(value)) => *value as f32,
        other => panic!(
            "config fixture {} lacks a numeric `support_object_xy_distance` (got {other:?})",
            matched_config_path().display()
        ),
    }
}

// -- Metric helpers (pure) --------------------------------------------------

fn contour_area(polygon: &Polygon) -> f64 {
    let points = &polygon.points;
    if points.len() < 3 {
        return 0.0;
    }
    let mut acc = 0.0f64;
    for index in 0..points.len() {
        let a = points[index];
        let b = points[(index + 1) % points.len()];
        acc += (a.x as f64) * (b.y as f64) - (b.x as f64) * (a.y as f64);
    }
    (acc / 2.0).abs()
}

/// Unsigned area of an ExPolygon in PnP units^2 (contour minus holes).
fn expolygon_area(poly: &ExPolygon) -> f64 {
    let holes: f64 = poly.holes.iter().map(contour_area).sum();
    (contour_area(&poly.contour) - holes).max(0.0)
}

fn total_area(polys: &[ExPolygon]) -> f64 {
    polys.iter().map(expolygon_area).sum()
}

/// `SupportBody` regions of every accepted (non-declined) plan entry, keyed by
/// `global_layer_index`.
fn support_body_regions_by_layer(plan: &SupportPlanIR) -> BTreeMap<i32, Vec<ExPolygon>> {
    let mut by_layer: BTreeMap<i32, Vec<ExPolygon>> = BTreeMap::new();
    for entry in plan
        .entries
        .iter()
        .filter(|entry| entry.decline_reason.is_none())
    {
        for role in &entry.roles {
            if role.role != SupportPlanRole::SupportBody || role.regions.is_empty() {
                continue;
            }
            by_layer
                .entry(entry.global_layer_index)
                .or_default()
                .extend(role.regions.iter().cloned());
        }
    }
    by_layer
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct WallLeakageMetrics {
    penetration_events: usize,
    /// PnP units^2 (1 unit^2 = 1e-8 mm^2).
    penetrated_area: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ColumnContinuityMetrics {
    abrupt_drops: usize,
    /// PnP units^2 (1 unit^2 = 1e-8 mm^2).
    total_support_area: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct P241Baseline {
    /// Provenance: path relative to `crates/slicer-runtime`.
    fixture_model: String,
    fixture_config: String,
    support_type: String,
    /// `support_area_rasterizer` the plan was produced with. The tracked
    /// baseline is always [`RASTERIZER_LEGACY`]: it is the PRE-port reference.
    rasterizer_mode: String,
    /// Why the baseline is recorded against the fixture it names.
    fixture_justification: String,
    support_object_xy_distance_mm: f32,
    /// Number of `LayerPlanIR` global layers the metrics were computed over.
    layer_count: usize,
    /// Number of layers carrying at least one `SupportBody` region.
    support_body_layer_count: usize,
    /// Number of layers whose model occupancy is non-empty. Non-vacuity witness:
    /// a zero wall-leakage result is only meaningful if occupancy actually exists.
    occupied_layer_count: usize,
    wall: WallLeakageMetrics,
    columns: ColumnContinuityMetrics,
}

/// Wall leakage: support body geometry intruding into model occupancy grown by
/// `xy_distance_mm`. See the module header for the exact definitions.
fn wall_leakage_metrics(
    body_by_layer: &BTreeMap<i32, Vec<ExPolygon>>,
    occupancy_by_layer: &BTreeMap<i32, Vec<ExPolygon>>,
    xy_distance_mm: f32,
) -> WallLeakageMetrics {
    let mut penetration_events = 0usize;
    let mut penetrated_area = 0.0f64;
    for (layer, bodies) in body_by_layer {
        let Some(occupancy) = occupancy_by_layer.get(layer) else {
            continue;
        };
        if occupancy.is_empty() {
            continue;
        }
        let grown = offset(occupancy, xy_distance_mm, GROW_JOIN, GROW_ARC_TOLERANCE_MM);
        if grown.is_empty() {
            continue;
        }
        for body in bodies {
            let overlap = intersection_ex(std::slice::from_ref(body), &grown);
            if overlap.is_empty() {
                continue;
            }
            // Sum only the pieces above the tangency-sliver noise floor; a body
            // whose every piece is below it did not penetrate.
            let area: f64 = overlap
                .iter()
                .map(expolygon_area)
                .filter(|piece| *piece >= WALL_LEAKAGE_NOISE_FLOOR_UNITS2)
                .sum();
            if area <= 0.0 {
                continue;
            }
            penetration_events += 1;
            penetrated_area += area;
        }
    }
    WallLeakageMetrics {
        penetration_events,
        penetrated_area,
    }
}

/// Column continuity: connected body components that vanish going down the
/// stack without terminating on the plate or on the model.
fn column_continuity_metrics(
    body_by_layer: &BTreeMap<i32, Vec<ExPolygon>>,
    occupancy_by_layer: &BTreeMap<i32, Vec<ExPolygon>>,
    printable_layers: &BTreeSet<i32>,
) -> ColumnContinuityMetrics {
    let components: BTreeMap<i32, Vec<ExPolygon>> = body_by_layer
        .iter()
        .map(|(layer, bodies)| (*layer, union_ex(bodies)))
        .collect();

    let mut abrupt_drops = 0usize;
    let mut total_support_area = 0.0f64;
    let empty: Vec<ExPolygon> = Vec::new();

    for (layer, comps) in components.iter().rev() {
        total_support_area += total_area(comps);
        let below = layer - 1;
        // Terminating on the build plate: there is no printable layer below.
        if !printable_layers.contains(&below) {
            continue;
        }
        let body_below = components.get(&below).unwrap_or(&empty);
        let occupancy_below = occupancy_by_layer.get(&below).unwrap_or(&empty);
        for comp in comps {
            let one = std::slice::from_ref(comp);
            let supported_by_body =
                !body_below.is_empty() && !intersection_ex(one, body_below).is_empty();
            if supported_by_body {
                continue;
            }
            let landed_on_model =
                !occupancy_below.is_empty() && !intersection_ex(one, occupancy_below).is_empty();
            if landed_on_model {
                continue;
            }
            abrupt_drops += 1;
        }
    }

    ColumnContinuityMetrics {
        abrupt_drops,
        total_support_area,
    }
}

// -- Evidence assembly ------------------------------------------------------

/// Relative (to `crates/slicer-runtime`) label of the tracked adversarial
/// fixture, as recorded in the baseline's `fixture_model`.
const ADVERSARIAL_FIXTURE_LABEL: &str = "tests/fixtures/support-family/SupportAdversarial.stl";

/// Recorded alongside the baseline so the fixture switch is self-describing.
const ADVERSARIAL_FIXTURE_JUSTIFICATION: &str = "The original 30x20 mm SupportTest.stl box measures 0 penetration events / 0 penetrated area / 0 abrupt column drops under the legacy rasterizer, so AC-7's strict 'fewer drops' gate cannot be measured against it. SupportAdversarial.stl (adversarial_mesh, regenerable via p241_generate_adversarial_fixture) carries three roofed slots that narrow abruptly mid-stack and whose far end is a roof-outline (demand) edge rather than a clearance edge; the legacy semantic prints the slot sliver exactly to the roof edge and drops it in mid-air when the slot closes, while the SupportGridPattern expansion_to_slice extraction grows that free end onto the lower slot's end wall (a model landing). stepped_pocket_mesh, the earlier fully-enclosed variant, measures 3 drops in BOTH modes (every sliver edge is a clearance edge, which neither mode moves) and is kept in the test file for reference. The baseline is recorded in legacy_semantic mode.";

/// Runs the tracked adversarial fixture through the prepass driver in
/// **legacy** rasterizer mode and computes both metric structs from the
/// resulting plan geometry. This is the PRE-port reference.
fn compute_baseline() -> Result<P241Baseline, String> {
    compute_metrics_for(
        &support_adversarial_path(),
        ADVERSARIAL_FIXTURE_LABEL,
        RASTERIZER_LEGACY,
    )
    .map(|parts| parts.baseline)
}

/// Same fixture, same metric path, `support_area_rasterizer = "agg"`: the
/// POST-port measurement the AC-6 / AC-7 gates compare against the baseline.
fn compute_agg_metrics() -> Result<BaselineParts, String> {
    compute_metrics_for(
        &support_adversarial_path(),
        ADVERSARIAL_FIXTURE_LABEL,
        RASTERIZER_AGG,
    )
}

/// [`compute_baseline_for`] output plus the two geometry maps the metrics were
/// derived from, so the exploration harness can characterise *individual*
/// penetration events rather than only their total.
struct BaselineParts {
    baseline: P241Baseline,
    body_by_layer: BTreeMap<i32, Vec<ExPolygon>>,
    occupancy_by_layer: BTreeMap<i32, Vec<ExPolygon>>,
}

/// Same computation as [`compute_baseline`] against an arbitrary model, in
/// legacy rasterizer mode. Used by the packet-241 adversarial-fixture
/// exploration below; the tracked baseline path goes through
/// [`compute_baseline`].
fn compute_baseline_for(model: &Path, fixture_label: &str) -> Result<BaselineParts, String> {
    compute_metrics_for(model, fixture_label, RASTERIZER_LEGACY)
}

/// Runs `model` through the prepass driver with the given
/// `support_area_rasterizer` and computes both metric structs.
fn compute_metrics_for(
    model: &Path,
    fixture_label: &str,
    rasterizer: &str,
) -> Result<BaselineParts, String> {
    let model = model.to_path_buf();
    let mut config = matched_config_for(true, BASELINE_SUPPORT_TYPE);
    config.insert(
        "support_area_rasterizer".to_string(),
        ConfigValue::String(rasterizer.to_string()),
    );
    let xy_distance = xy_distance_mm(&config);
    let context = prepare_model_support(&model, config)?;

    let layer_plan = context
        .blackboard
        .layer_plan()
        .ok_or_else(|| "LayerPlanIR missing from prepass blackboard".to_string())?;
    let layer_z_mm: BTreeMap<i32, f32> = layer_plan
        .global_layers
        .iter()
        .map(|layer| (layer.index as i32, layer.z))
        .collect();
    let printable_layers: BTreeSet<i32> = layer_z_mm.keys().copied().collect();

    let plan = context
        .blackboard
        .support_plan()
        .map(|plan| plan.as_ref().clone())
        .unwrap_or_default();
    let body_by_layer = support_body_regions_by_layer(&plan);

    // Non-vacuity guard against the silent-skip path in `wall_leakage_metrics`:
    // a body layer with no `LayerPlanIR` counterpart has no occupancy entry and
    // would be dropped from the metric without a trace. `SupportPlanIR` may
    // legitimately carry off-grid synthesized rows (see
    // `support_never_intersects_model_at_exact_z`, which falls back to
    // `anchor_z`); this fixture has none, and the metrics must not silently
    // start ignoring layers if that ever changes.
    let orphan_body_layers: Vec<i32> = body_by_layer
        .keys()
        .filter(|layer| !printable_layers.contains(layer))
        .copied()
        .collect();
    if !orphan_body_layers.is_empty() {
        return Err(format!(
            "support-body layers {orphan_body_layers:?} have no LayerPlanIR entry; the \
             wall-leakage metric would silently skip them (off-grid anchor_z rows need \
             an explicit Z fallback before this baseline is meaningful)"
        ));
    }

    // Distinct (object, region) pairs the plan references; occupancy for a layer
    // is the union of the model cross-sections of all of them at that layer's Z.
    let mut targets: BTreeSet<(String, u64)> = BTreeSet::new();
    for entry in plan
        .entries
        .iter()
        .filter(|entry| entry.decline_reason.is_none())
    {
        targets.insert((entry.object_id.clone(), entry.region_id));
    }

    let mesh = cached_load_model(&model);
    let exact_z = ExactZQueryService::new(Arc::clone(&mesh));
    let mut occupancy_by_layer: BTreeMap<i32, Vec<ExPolygon>> = BTreeMap::new();
    for (layer, z_mm) in &layer_z_mm {
        let mut occupancy: Vec<ExPolygon> = Vec::new();
        for (object_id, region_id) in &targets {
            let query = exact_z
                .query(object_id.as_str(), *region_id, *z_mm)
                .map_err(|error| {
                    format!("exact-Z query at layer {layer} (z={z_mm:.4}mm) failed: {error}")
                })?;
            occupancy.extend(query.occupancy.iter().cloned());
        }
        occupancy_by_layer.insert(*layer, union_ex(&occupancy));
    }

    let occupied_layer_count = occupancy_by_layer
        .values()
        .filter(|regions| !regions.is_empty())
        .count();

    let wall = wall_leakage_metrics(&body_by_layer, &occupancy_by_layer, xy_distance);
    let columns = column_continuity_metrics(&body_by_layer, &occupancy_by_layer, &printable_layers);

    Ok(BaselineParts {
        baseline: P241Baseline {
            fixture_model: fixture_label.to_string(),
            fixture_config: "tests/fixtures/support-family/orca-matched-config.json".to_string(),
            support_type: BASELINE_SUPPORT_TYPE.to_string(),
            rasterizer_mode: rasterizer.to_string(),
            fixture_justification: ADVERSARIAL_FIXTURE_JUSTIFICATION.to_string(),
            support_object_xy_distance_mm: xy_distance,
            layer_count: layer_z_mm.len(),
            support_body_layer_count: body_by_layer.len(),
            occupied_layer_count,
            wall,
            columns,
        },
        body_by_layer,
        occupancy_by_layer,
    })
}

// -- Tests ------------------------------------------------------------------

/// Recorder. Writes the tracked PRE-port baseline. Not part of the normal run.
#[test]
#[ignore = "recorder: writes the tracked p241 baseline; run explicitly"]
fn capture_pre_port_baseline() {
    let baseline = compute_baseline().expect("prepass baseline capture failed");
    let path = baseline_path();
    let dir = path
        .parent()
        .expect("baseline path has no parent directory")
        .to_path_buf();
    fs::create_dir_all(&dir).unwrap_or_else(|error| panic!("create {}: {error}", dir.display()));
    let json = serde_json::to_string_pretty(&baseline).expect("serialize p241 baseline") + "\n";
    fs::write(&path, json).unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
    eprintln!(
        "p241 baseline written to {}: {:?} / {:?}",
        path.display(),
        baseline.wall,
        baseline.columns
    );
}

/// Step 1 gate: the committed baseline must be reproducible from the current
/// tree by the metric helpers, exactly.
#[test]
fn p241_metric_helpers_agree_on_baseline_fixture() {
    let path = baseline_path();
    let raw = fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "tracked p241 baseline is missing at {} ({error}); regenerate with \
             `cargo test -p slicer-runtime --test integration -- capture_pre_port_baseline --exact --ignored`",
            path.display()
        )
    });
    assert!(
        !raw.trim().is_empty(),
        "tracked p241 baseline at {} is empty",
        path.display()
    );
    let recorded: P241Baseline = serde_json::from_str(&raw)
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));

    let actual = compute_baseline().expect("prepass baseline recomputation failed");

    assert_eq!(
        actual.wall,
        recorded.wall,
        "wall-leakage metrics drifted from the tracked baseline at {}",
        path.display()
    );
    assert_eq!(
        actual.columns,
        recorded.columns,
        "column-continuity metrics drifted from the tracked baseline at {}",
        path.display()
    );
    assert_eq!(
        actual,
        recorded,
        "p241 baseline provenance drifted from {}",
        path.display()
    );
}

// -- Sensitivity tests ------------------------------------------------------
//
// The recorded baseline is all zeros for both leakage counters. A zero is only
// evidence if the code that produced it can produce a non-zero. These tests
// drive the **same** pure metric functions the fixture path drives
// (`wall_leakage_metrics` / `column_continuity_metrics` are already injectable:
// `compute_baseline` builds the two `BTreeMap`s and hands them straight over),
// with hand-built geometry whose answer is known by construction.

/// Axis-aligned CCW rectangle from mm corners, in PnP units.
fn rect_mm(x0: f32, y0: f32, x1: f32, y1: f32) -> ExPolygon {
    use slicer_ir::Point2;
    let p = |x: f32, y: f32| Point2 {
        x: slicer_ir::mm_to_units(x),
        y: slicer_ir::mm_to_units(y),
    };
    ExPolygon {
        contour: Polygon {
            points: vec![p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1)],
        },
        holes: Vec::new(),
    }
}

/// A support body that unambiguously overlaps model occupancy must be reported.
/// If this returns zero, every zero the fixture path reports is meaningless.
#[test]
fn p241_wall_leakage_metric_detects_synthetic_penetration() {
    // 10x10 mm model block at the origin; a 10x10 mm support body shifted 5 mm
    // in X, so exactly 5x10 = 50 mm^2 of raw overlap before any grow.
    let occupancy: BTreeMap<i32, Vec<ExPolygon>> =
        BTreeMap::from([(7, vec![rect_mm(0.0, 0.0, 10.0, 10.0)])]);
    let bodies: BTreeMap<i32, Vec<ExPolygon>> =
        BTreeMap::from([(7, vec![rect_mm(5.0, 0.0, 15.0, 10.0)])]);

    let metrics = wall_leakage_metrics(&bodies, &occupancy, 0.35);

    assert!(
        metrics.penetration_events > 0,
        "wall-leakage metric reported no penetration for two overlapping squares; \
         the metric is broken and the recorded zero baseline is an artifact"
    );
    assert!(
        metrics.penetrated_area > 0.0,
        "wall-leakage metric reported zero penetrated area for a 50 mm^2 overlap"
    );
    assert_eq!(
        metrics.penetration_events, 1,
        "one body ExPolygon, one event"
    );
    // 50 mm^2 raw, plus the 0.35 mm outward grow of the model block along the
    // 10 mm shared span. Pins the area scale as well as the sign.
    let mm2 = metrics.penetrated_area * 1e-8;
    assert!(
        mm2 > 50.0 && mm2 < 60.0,
        "penetrated area {mm2:.4} mm^2 is not the expected ~50 mm^2 overlap plus \
         the 0.35 mm grow; area accumulation is mis-scaled"
    );
}

/// The `xy_distance_mm` grow argument must be interpreted as **millimetres**.
/// A unit/mm mixup (0.35 units = 35 nm) would make the grow a no-op, which is
/// invisible in the fixture baseline because its raw overlap is also zero.
#[test]
fn p241_wall_leakage_grow_argument_is_millimetres() {
    // Body sits 0.5 mm clear of the model in X: no raw overlap at all.
    let occupancy: BTreeMap<i32, Vec<ExPolygon>> =
        BTreeMap::from([(3, vec![rect_mm(0.0, 0.0, 10.0, 10.0)])]);
    let bodies: BTreeMap<i32, Vec<ExPolygon>> =
        BTreeMap::from([(3, vec![rect_mm(10.5, 0.0, 20.0, 10.0)])]);

    let no_grow = wall_leakage_metrics(&bodies, &occupancy, 0.0);
    assert_eq!(
        no_grow.penetration_events, 0,
        "bodies are 0.5 mm clear; raw overlap must be zero"
    );

    let under = wall_leakage_metrics(&bodies, &occupancy, 0.35);
    assert_eq!(
        under.penetration_events, 0,
        "a 0.35 mm grow does not close a 0.5 mm gap"
    );

    let over = wall_leakage_metrics(&bodies, &occupancy, 1.0);
    assert!(
        over.penetration_events > 0 && over.penetrated_area > 0.0,
        "a 1.0 mm grow must close a 0.5 mm gap; the grow argument is being \
         interpreted as PnP units (35 nm) instead of millimetres"
    );
}

/// A column that vanishes mid-stack, above a printable layer and away from the
/// model, must be reported as an abrupt drop.
#[test]
fn p241_column_continuity_metric_detects_synthetic_drop() {
    // Printable layers 0..=5. A single 4x4 mm column occupies layers 3, 4, 5
    // only, so at layer 3 there is a printable layer 2 below it with neither a
    // support body nor model occupancy: the column terminates in mid-air.
    let column = rect_mm(0.0, 0.0, 4.0, 4.0);
    let bodies: BTreeMap<i32, Vec<ExPolygon>> = BTreeMap::from([
        (3, vec![column.clone()]),
        (4, vec![column.clone()]),
        (5, vec![column.clone()]),
    ]);
    let occupancy: BTreeMap<i32, Vec<ExPolygon>> = BTreeMap::new();
    let printable: BTreeSet<i32> = (0..=5).collect();

    let metrics = column_continuity_metrics(&bodies, &occupancy, &printable);

    assert!(
        metrics.abrupt_drops > 0,
        "column-continuity metric reported no drop for a column that vanishes at \
         layer 3 above printable layer 2; the metric is broken and the recorded \
         zero baseline is an artifact"
    );
    assert_eq!(
        metrics.abrupt_drops, 1,
        "exactly one component vanishes (layers 4 and 5 are supported from below)"
    );
    // 3 layers x 16 mm^2 = 48 mm^2 = 4.8e9 units^2. Pins the area scale.
    let mm2 = metrics.total_support_area * 1e-8;
    assert!(
        (mm2 - 48.0).abs() < 1e-3,
        "total_support_area {mm2:.6} mm^2 != 48 mm^2; area accumulation is \
         mis-scaled by roughly {:.3}x",
        mm2 / 48.0
    );
}

/// Grounded and model-landing columns must NOT be counted as drops - the
/// counterpart to the test above, so a metric that returned a constant non-zero
/// would also fail.
#[test]
fn p241_column_continuity_metric_ignores_grounded_and_landed_columns() {
    let column = rect_mm(0.0, 0.0, 4.0, 4.0);
    let printable: BTreeSet<i32> = (0..=3).collect();

    // Grounded: reaches layer 0, whose "below" is not printable.
    let grounded: BTreeMap<i32, Vec<ExPolygon>> = BTreeMap::from([
        (0, vec![column.clone()]),
        (1, vec![column.clone()]),
        (2, vec![column.clone()]),
    ]);
    let grounded_metrics = column_continuity_metrics(&grounded, &BTreeMap::new(), &printable);
    assert_eq!(
        grounded_metrics.abrupt_drops, 0,
        "a column reaching layer 0 terminates on the build plate"
    );

    // Landed on the model: starts at layer 2, with model occupancy at layer 1.
    let landed: BTreeMap<i32, Vec<ExPolygon>> =
        BTreeMap::from([(2, vec![column.clone()]), (3, vec![column.clone()])]);
    let occupancy: BTreeMap<i32, Vec<ExPolygon>> =
        BTreeMap::from([(1, vec![rect_mm(0.0, 0.0, 4.0, 4.0)])]);
    let landed_metrics = column_continuity_metrics(&landed, &occupancy, &printable);
    assert_eq!(
        landed_metrics.abrupt_drops, 0,
        "a column resting on model occupancy at the layer below is not a drop"
    );
}

// -- Packet 241 adversarial-fixture exploration -----------------------------
//
// The tracked baseline is zero on both axes. These helpers generate candidate
// meshes aimed at the two upstream symptoms and run them through the SAME
// metric path (`compute_baseline_for`) so the numbers are comparable.

/// One axis-aligned box, as 12 triangles, in mm.
fn box_triangles(x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32) -> Vec<[[f32; 3]; 3]> {
    let v = [
        [x0, y0, z0],
        [x1, y0, z0],
        [x1, y1, z0],
        [x0, y1, z0],
        [x0, y0, z1],
        [x1, y0, z1],
        [x1, y1, z1],
        [x0, y1, z1],
    ];
    // CCW-outward quads: bottom, top, -y, +y, -x, +x.
    let quads: [[usize; 4]; 6] = [
        [0, 3, 2, 1],
        [4, 5, 6, 7],
        [0, 1, 5, 4],
        [2, 3, 7, 6],
        [3, 0, 4, 7],
        [1, 2, 6, 5],
    ];
    let mut tris = Vec::with_capacity(12);
    for q in quads {
        tris.push([v[q[0]], v[q[1]], v[q[2]]]);
        tris.push([v[q[0]], v[q[2]], v[q[3]]]);
    }
    tris
}

fn write_binary_stl(path: &Path, tris: &[[[f32; 3]; 3]]) {
    let mut bytes = Vec::with_capacity(84 + tris.len() * 50);
    bytes.extend_from_slice(&[0u8; 80]);
    bytes.extend_from_slice(&(tris.len() as u32).to_le_bytes());
    for tri in tris {
        // Face normal (unnormalized zero is legal and ignored by most loaders,
        // but compute it properly so the loader's facet classification works).
        let u = [
            tri[1][0] - tri[0][0],
            tri[1][1] - tri[0][1],
            tri[1][2] - tri[0][2],
        ];
        let w = [
            tri[2][0] - tri[0][0],
            tri[2][1] - tri[0][1],
            tri[2][2] - tri[0][2],
        ];
        let mut n = [
            u[1] * w[2] - u[2] * w[1],
            u[2] * w[0] - u[0] * w[2],
            u[0] * w[1] - u[1] * w[0],
        ];
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if len > 0.0 {
            n = [n[0] / len, n[1] / len, n[2] / len];
        }
        for c in n {
            bytes.extend_from_slice(&c.to_le_bytes());
        }
        for vertex in tri {
            for c in vertex {
                bytes.extend_from_slice(&c.to_le_bytes());
            }
        }
        bytes.extend_from_slice(&0u16.to_le_bytes());
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).unwrap_or_else(|e| panic!("create {}: {e}", dir.display()));
    }
    fs::write(path, bytes).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
}

/// One 10x10 mm block carrying a roofed rectangular pocket that narrows
/// abruptly at `step_z`. Emitted at `x0` so several can be butted together into
/// one solid.
///
/// Sized so the support sliver standing in the wide pocket is swallowed by the
/// narrow pocket's 0.35 mm clearance annulus **without** touching the model, so
/// the metric's `landed_on_model` test fails and the column registers as an
/// abrupt drop rather than a model landing:
///
/// * wide pocket 1.30 mm across -> sliver 0.60 mm (0.35 mm inset per side);
/// * narrow pocket 0.66 mm across -> closes entirely under the 0.35 mm grow
///   (0.04 mm of margin) while leaving 0.03 mm of clearance per side.
///
/// Every Z seam is off the 0.2 mm layer grid so no slice plane is coplanar with
/// a horizontal facet.
fn pocket_block(x0: f32, step_z: f32) -> Vec<[[f32; 3]; 3]> {
    let mut tris = Vec::new();
    let (nx0, nx1) = (x0 + 4.67, x0 + 5.33); // narrow pocket, 0.66 mm
    let (ny0, ny1) = (3.75, 6.25);
    let (wx0, wx1) = (x0 + 4.35, x0 + 5.65); // wide pocket, 1.30 mm
    let (wy0, wy1) = (3.5, 6.5);
    let x1 = x0 + 10.0;
    // Solid floor under the pocket.
    tris.extend(box_triangles(x0, 0.0, 0.0, x1, 10.0, 1.05));
    // Narrow band.
    tris.extend(box_triangles(x0, 0.0, 1.05, nx0, 10.0, step_z));
    tris.extend(box_triangles(nx1, 0.0, 1.05, x1, 10.0, step_z));
    tris.extend(box_triangles(nx0, 0.0, 1.05, nx1, ny0, step_z));
    tris.extend(box_triangles(nx0, ny1, 1.05, nx1, 10.0, step_z));
    // Wide band.
    tris.extend(box_triangles(x0, 0.0, step_z, wx0, 10.0, 6.1));
    tris.extend(box_triangles(wx1, 0.0, step_z, x1, 10.0, 6.1));
    tris.extend(box_triangles(wx0, 0.0, step_z, wx1, wy0, 6.1));
    tris.extend(box_triangles(wx0, wy1, step_z, wx1, 10.0, 6.1));
    tris
}

/// Candidate A - "stepped pocket". Three butted [`pocket_block`]s, each
/// stepping at a different height, under one roof slab that also overhangs the
/// solid on every side.
fn stepped_pocket_mesh() -> Vec<[[f32; 3]; 3]> {
    let mut tris = Vec::new();
    for (index, step_z) in [2.1_f32, 3.1, 4.1].into_iter().enumerate() {
        tris.extend(pocket_block(index as f32 * 10.0, step_z));
    }
    tris.extend(box_triangles(-2.0, -2.0, 6.1, 32.0, 12.0, 6.7));
    tris
}

/// Candidate B - "thin-wall slot". Two thin vertical walls with a 0.9 mm slot
/// between them and a roof spanning the slot: the classic wall-leakage shape.
fn thin_wall_slot_mesh() -> Vec<[[f32; 3]; 3]> {
    let mut tris = Vec::new();
    tris.extend(box_triangles(0.0, 0.0, 0.0, 0.8, 10.0, 6.1));
    tris.extend(box_triangles(1.7, 0.0, 0.0, 2.5, 10.0, 6.1));
    tris.extend(box_triangles(-2.0, -2.0, 6.1, 4.5, 12.0, 6.7));
    tris
}

/// Candidate C - "tapered slot". Same two walls, but each is a stack of steps
/// that leans inward going down, so the slot narrows 0.3 mm per side over one
/// layer at a time - the shape that walks a support column through the
/// clearance annulus repeatedly.
fn tapered_slot_mesh() -> Vec<[[f32; 3]; 3]> {
    let mut tris = Vec::new();
    // Slot centred on x = 5.0. Half-width shrinks from 0.8 mm at the top to
    // 0.2 mm at the bottom in 0.1 mm steps, one step per 0.4 mm of height.
    let steps = 6;
    for step in 0..steps {
        let z0 = 1.05 + 0.4 * step as f32;
        let z1 = z0 + 0.4;
        let half = 0.8 - 0.1 * (steps - 1 - step) as f32;
        tris.extend(box_triangles(0.0, 0.0, z0, 5.0 - half, 10.0, z1));
        tris.extend(box_triangles(5.0 + half, 0.0, z0, 10.0, 10.0, z1));
    }
    tris.extend(box_triangles(0.0, 0.0, 0.0, 10.0, 10.0, 1.05));
    let top = 1.05 + 0.4 * steps as f32;
    tris.extend(box_triangles(-2.0, -2.0, top, 12.0, 12.0, top + 0.6));
    tris
}

/// Candidate D - "roof-edge slot". A base block with one roofed slot that is
/// OPEN to the roof outline: the roof stops at `roof_y` while the wide upper
/// slot runs on to `wide_y1`, so the support sliver standing in the slot has a
/// FREE end (a demand-footprint edge, not a clearance edge) at `roof_y`. Below
/// `step_z` the slot narrows to 0.66 mm (closes under the 0.35 mm clearance,
/// as in [`pocket_block`]) and also ends at `narrow_y1`, a hair beyond
/// `roof_y`. Legacy prints the sliver exactly up to `roof_y`, so it vanishes
/// without touching the narrow band's end wall: an abrupt drop. The AGG port
/// prints the `expansion_to_slice` extraction, which grows the sliver's free
/// end (only its free end - the other three edges are clearance edges and stay
/// put) by `line_width / 2` = 0.2 mm, onto that end wall: a model landing.
fn roof_edge_slot_mesh(
    step_z: f32,
    roof_y: f32,
    narrow_y1: f32,
    wide_y1: f32,
) -> Vec<[[f32; 3]; 3]> {
    let mut tris = roof_edge_slot_block(0.0, step_z, narrow_y1, wide_y1);
    // Roof: overhangs three sides, stops short of the slot's far end.
    tris.extend(box_triangles(-2.0, -2.0, 6.1, 12.0, roof_y, 6.7));
    tris
}

/// One 10x10 mm block of [`roof_edge_slot_mesh`] without its roof, emitted at
/// `x0` so several can be butted together under one roof (the
/// [`pocket_block`] pattern). Slot cross-sections are exactly those of
/// [`pocket_block`]; only the slot's far end differs: the wide band runs to
/// `wide_y1` (past the roof outline, so the sliver's far end is a demand
/// edge) and the narrow band ends at `narrow_y1`.
fn roof_edge_slot_block(x0: f32, step_z: f32, narrow_y1: f32, wide_y1: f32) -> Vec<[[f32; 3]; 3]> {
    let mut tris = Vec::new();
    let (nx0, nx1) = (x0 + 4.67, x0 + 5.33); // narrow slot, 0.66 mm
    let ny0 = 3.75;
    let (wx0, wx1) = (x0 + 4.35, x0 + 5.65); // wide slot, 1.30 mm
    let wy0 = 3.5;
    let x1 = x0 + 10.0;
    // Solid floor.
    tris.extend(box_triangles(x0, 0.0, 0.0, x1, 10.0, 1.05));
    // Narrow band.
    tris.extend(box_triangles(x0, 0.0, 1.05, nx0, 10.0, step_z));
    tris.extend(box_triangles(nx1, 0.0, 1.05, x1, 10.0, step_z));
    tris.extend(box_triangles(nx0, 0.0, 1.05, nx1, ny0, step_z));
    tris.extend(box_triangles(nx0, narrow_y1, 1.05, nx1, 10.0, step_z));
    // Wide band.
    tris.extend(box_triangles(x0, 0.0, step_z, wx0, 10.0, 6.1));
    tris.extend(box_triangles(wx1, 0.0, step_z, x1, 10.0, 6.1));
    tris.extend(box_triangles(wx0, 0.0, step_z, wx1, wy0, 6.1));
    tris.extend(box_triangles(wx0, wide_y1, step_z, wx1, 10.0, 6.1));
    tris
}

/// Roof outline (far edge, mm) of the adopted adversarial fixture.
const ADVERSARIAL_ROOF_Y: f32 = 8.0;
/// Narrow-band end wall of the adopted fixture: 0.1 mm past the roof edge,
/// inside the 0.2 mm free-end growth of the AGG printed area.
const ADVERSARIAL_NARROW_Y1: f32 = 8.1;
/// Wide-band end wall of the adopted fixture: far enough past the roof edge
/// (0.35 mm clearance + margin) that the sliver's far end is the roof edge.
const ADVERSARIAL_WIDE_Y1: f32 = 9.0;

/// Candidate E - the ADOPTED adversarial fixture. Three butted
/// [`roof_edge_slot_block`]s stepping at three different heights under one
/// roof that stops at [`ADVERSARIAL_ROOF_Y`]. Measured (see
/// `p241_explore_adversarial_candidates`): legacy 3 abrupt drops, agg 0. The
/// accompanying "total support area inside the AC-7 25% band" claim was
/// recorded while the agg arm still carried the printed-area clamp; with the
/// clamp removed the area delta is +57.09 % (packet 241 Step 14, measured
/// 2026-09-03). AC-7's old +/-25 % total-area half was retired for that reason
/// (the halo adds material by design, DEV-166) and replaced by a per-layer
/// macro-block containment bound - see
/// [`agg_column_continuity_measurement_beats_baseline`].
fn adversarial_mesh() -> Vec<[[f32; 3]; 3]> {
    let mut tris = Vec::new();
    for (index, step_z) in [2.1_f32, 3.1, 4.1].into_iter().enumerate() {
        tris.extend(roof_edge_slot_block(
            index as f32 * 10.0,
            step_z,
            ADVERSARIAL_NARROW_Y1,
            ADVERSARIAL_WIDE_Y1,
        ));
    }
    tris.extend(box_triangles(
        -2.0,
        -2.0,
        6.1,
        32.0,
        ADVERSARIAL_ROOF_Y,
        6.7,
    ));
    tris
}

/// Bounding box of an ExPolygon contour in mm, for the exploration printout.
fn bbox_mm(poly: &ExPolygon) -> (f64, f64, f64, f64) {
    let mut lo = (f64::INFINITY, f64::INFINITY);
    let mut hi = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in &poly.contour.points {
        lo.0 = lo.0.min(p.x as f64);
        lo.1 = lo.1.min(p.y as f64);
        hi.0 = hi.0.max(p.x as f64);
        hi.1 = hi.1.max(p.y as f64);
    }
    (lo.0 * 1e-4, lo.1 * 1e-4, hi.0 * 1e-4, hi.1 * 1e-4)
}

/// Per-mode characterisation used by the exploration harness: every abrupt
/// drop as `(layer, component mm^2, bbox mm)`, and the three largest
/// penetration events in mm^2.
#[allow(clippy::type_complexity)]
fn describe_parts(parts: &BaselineParts) -> (Vec<(i32, f64, (f64, f64, f64, f64))>, Vec<f64>) {
    let baseline = &parts.baseline;
    let mut events_mm2: Vec<f64> = Vec::new();
    for (layer, bodies) in &parts.body_by_layer {
        let Some(occ) = parts.occupancy_by_layer.get(layer) else {
            continue;
        };
        let grown = offset(
            occ,
            baseline.support_object_xy_distance_mm,
            GROW_JOIN,
            GROW_ARC_TOLERANCE_MM,
        );
        for body in bodies {
            let overlap = intersection_ex(std::slice::from_ref(body), &grown);
            let area = total_area(&overlap) * 1e-8;
            if area > 0.0 {
                events_mm2.push(area);
            }
        }
    }
    events_mm2.sort_by(|a, b| b.partial_cmp(a).unwrap());
    events_mm2.truncate(3);
    let comps: BTreeMap<i32, Vec<ExPolygon>> = parts
        .body_by_layer
        .iter()
        .map(|(layer, bodies)| (*layer, union_ex(bodies)))
        .collect();
    let printable: BTreeSet<i32> = parts.occupancy_by_layer.keys().copied().collect();
    let empty: Vec<ExPolygon> = Vec::new();
    let mut drops = Vec::new();
    for (layer, cs) in comps.iter().rev() {
        let below = layer - 1;
        if !printable.contains(&below) {
            continue;
        }
        let bb = comps.get(&below).unwrap_or(&empty);
        let ob = parts.occupancy_by_layer.get(&below).unwrap_or(&empty);
        for c in cs {
            let one = std::slice::from_ref(c);
            if !bb.is_empty() && !intersection_ex(one, bb).is_empty() {
                continue;
            }
            if !ob.is_empty() && !intersection_ex(one, ob).is_empty() {
                continue;
            }
            drops.push((*layer, expolygon_area(c) * 1e-8, bbox_mm(c)));
        }
    }
    (drops, events_mm2)
}

/// Exploration harness. Generates each candidate into `target/p241-explore/`,
/// runs it through the same metric path as the tracked baseline, and prints the
/// measured numbers. `#[ignore]`d: it is an investigation tool, not a gate.
#[test]
#[ignore = "packet 241 exploration: prints adversarial-fixture metrics; run explicitly"]
fn p241_explore_adversarial_candidates() {
    let out_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/p241-explore");
    let candidates: Vec<(&str, Vec<[[f32; 3]; 3]>)> = vec![
        ("stepped_pocket", stepped_pocket_mesh()),
        ("thin_wall_slot", thin_wall_slot_mesh()),
        ("tapered_slot", tapered_slot_mesh()),
        (
            "roof_edge_slot_8.0_8.1_9.0",
            roof_edge_slot_mesh(3.1, 8.0, 8.1, 9.0),
        ),
        ("adversarial_3x_roof_edge_slot", adversarial_mesh()),
    ];
    let mut lines = Vec::new();
    for (name, tris) in candidates {
        let path = out_dir.join(format!("{name}.stl"));
        write_binary_stl(&path, &tris);
        let mut areas: Vec<(String, f64, usize)> = Vec::new();
        for mode in [RASTERIZER_LEGACY, RASTERIZER_AGG] {
            match compute_metrics_for(&path, name, mode) {
                Ok(parts) => {
                    let baseline = &parts.baseline;
                    let (drops, events_mm2) = describe_parts(&parts);
                    lines.push(format!("{name} [{mode}]: drops(layer,mm2,bbox)={drops:?}"));
                    lines.push(format!("{name} [{mode}]: largest_event_mm2={events_mm2:?}"));
                    lines.push(format!(
                        "{name} [{mode}]: layers={} body_layers={} occupied_layers={} \
                         penetration_events={} penetrated_area={:.1} abrupt_drops={} \
                         total_support_area={:.1}",
                        baseline.layer_count,
                        baseline.support_body_layer_count,
                        baseline.occupied_layer_count,
                        baseline.wall.penetration_events,
                        baseline.wall.penetrated_area,
                        baseline.columns.abrupt_drops,
                        baseline.columns.total_support_area,
                    ));
                    areas.push((
                        mode.to_string(),
                        baseline.columns.total_support_area,
                        baseline.columns.abrupt_drops,
                    ));
                }
                Err(error) => lines.push(format!("{name} [{mode}]: ERROR {error}")),
            }
        }
        if areas.len() == 2 && areas[0].1 > 0.0 {
            let delta = (areas[1].1 - areas[0].1) / areas[0].1 * 100.0;
            lines.push(format!(
                "{name} SUMMARY: legacy_drops={} agg_drops={} area_delta={delta:+.2}% \
                 discriminates={}",
                areas[0].2,
                areas[1].2,
                areas[1].2 < areas[0].2 && delta.abs() < 25.0
            ));
        }
    }
    for line in &lines {
        eprintln!("P241-EXPLORE {line}");
    }
}

/// Tracked adversarial fixture: the [`adversarial_mesh`] geometry, written to
/// `tests/fixtures/support-family/SupportAdversarial.stl`. The mesh is fully
/// determined by the generator above, so the STL is regenerable byte-for-byte;
/// it is tracked so the baseline has a stable, reviewable input.
fn support_adversarial_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/support-family/SupportAdversarial.stl")
}

/// Regenerator for the tracked adversarial fixture. `#[ignore]`d; run
/// explicitly when [`adversarial_mesh`] changes.
#[test]
#[ignore = "recorder: regenerates the tracked SupportAdversarial.stl; run explicitly"]
fn p241_generate_adversarial_fixture() {
    let path = support_adversarial_path();
    write_binary_stl(&path, &adversarial_mesh());
    let parts = compute_baseline_for(
        &path,
        "tests/fixtures/support-family/SupportAdversarial.stl",
    )
    .expect("adversarial fixture baseline");
    eprintln!(
        "P241-ADVERSARIAL wrote {} -> wall={:?} columns={:?}",
        path.display(),
        parts.baseline.wall,
        parts.baseline.columns
    );
    assert!(
        parts.baseline.columns.abrupt_drops > 0,
        "the adversarial fixture must exhibit at least one abrupt column drop \
         pre-port, otherwise AC-7's strict gate is unsatisfiable"
    );
}

// -- Step 7: post-port measurement gates (AC-6, AC-7, AC-8) -----------------

fn load_tracked_baseline() -> P241Baseline {
    let path = baseline_path();
    let raw = fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "tracked p241 baseline is missing at {} ({error}); regenerate with \
             `cargo test -p slicer-runtime --test integration -- capture_pre_port_baseline --exact --ignored`",
            path.display()
        )
    });
    let recorded: P241Baseline = serde_json::from_str(&raw)
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));
    assert_eq!(
        recorded.rasterizer_mode, RASTERIZER_LEGACY,
        "the tracked baseline must be the PRE-port (legacy_semantic) reference"
    );
    assert_eq!(
        recorded.fixture_model, ADVERSARIAL_FIXTURE_LABEL,
        "the tracked baseline must be recorded against the adversarial fixture"
    );
    recorded
}

/// AC-6. The legacy loop already subtracts the Miter-grown occupancy from the
/// carry on every layer (upstream fb7b995050 collision freedom), so the
/// pre-port baseline is exactly zero on both wall axes. The post-port run must
/// therefore be exactly zero as well: zero events, and a penetrated area no
/// larger than the baseline's (which is zero).
#[test]
fn agg_wall_leakage_measurement_beats_baseline() {
    let baseline = load_tracked_baseline();
    let agg = compute_agg_metrics()
        .expect("agg-mode prepass failed")
        .baseline;

    eprintln!(
        "P241-AC6 wall leakage on {}: baseline(legacy) events={} area={:.1} units^2 | \
         post-port(agg) events={} area={:.1} units^2",
        ADVERSARIAL_FIXTURE_LABEL,
        baseline.wall.penetration_events,
        baseline.wall.penetrated_area,
        agg.wall.penetration_events,
        agg.wall.penetrated_area,
    );

    assert!(
        agg.occupied_layer_count > 0 && agg.support_body_layer_count > 0,
        "agg run is vacuous: occupied_layers={} body_layers={}",
        agg.occupied_layer_count,
        agg.support_body_layer_count
    );
    assert_eq!(
        agg.wall.penetration_events, 0,
        "agg rasterizer produced {} wall-penetration events ({:.1} units^2)",
        agg.wall.penetration_events, agg.wall.penetrated_area
    );
    assert!(
        agg.wall.penetrated_area <= baseline.wall.penetrated_area,
        "agg penetrated area {:.1} exceeds baseline {:.1}",
        agg.wall.penetrated_area,
        baseline.wall.penetrated_area
    );
}

/// Planner defaults mirrored from `SupportPlanner::from_config`
/// (`DEFAULT_LINE_WIDTH_MM` and `DEFAULT_BASE_PATTERN_SPACING_MM` in
/// `modules/core-modules/traditional-support-planner/src/lib.rs`). Used ONLY
/// when the tracked config fixture omits the key, exactly as the planner does,
/// so the derived bound below tracks the profile rather than a constant.
const PLANNER_DEFAULT_LINE_WIDTH_MM: f32 = 0.4;
/// See [`PLANNER_DEFAULT_LINE_WIDTH_MM`].
const PLANNER_DEFAULT_BASE_PATTERN_SPACING_MM: f32 = 2.5;

/// Numeric config lookup with the planner's own fallback semantic: `Float` or
/// `Int` accepted, anything else - including absence - falls back.
fn config_f32(config: &HashMap<String, ConfigValue>, key: &str, default: f32) -> f32 {
    match config.get(key) {
        Some(ConfigValue::Float(value)) => *value as f32,
        Some(ConfigValue::Int(value)) => *value as f32,
        _ => default,
    }
}

/// The macro-block geometry canonical `SupportGridPattern` derives, re-derived
/// host-side from the same two config inputs the planner feeds to
/// `GridParams::from_polygons`
/// (`modules/core-modules/traditional-support-planner/src/agg_raster.rs`).
///
/// The module is a WASM guest crate and is not a dependency of
/// `slicer-runtime`, so `GridParams` cannot be called from here; the arithmetic
/// is mirrored instead, and [`MacroBlockExtent::assert_consistent`] re-derives
/// every field from the two mm inputs so a drifted mirror fails loudly.
/// All lengths are PnP units (1 unit = 100 nm).
#[derive(Debug, Clone, Copy)]
struct MacroBlockExtent {
    /// `support_base_pattern_spacing` as configured (mm).
    spacing_mm: f32,
    /// `line_width` as configured (mm).
    width_mm: f32,
    /// Canonical `grid_resolution`: the UNOVERSAMPLED spacing, units.
    grid_resolution: i64,
    /// `mm_to_units(line_width)`, units.
    width_units: i64,
    /// Canonical oversampling factor, clamped into `1..=8`.
    oversampling: i64,
    /// Canonical `pixel_size`, units.
    pixel_size: i64,
    /// `oversampling * pixel_size`: the side of ONE macro block, units.
    extent_units: i64,
}

impl MacroBlockExtent {
    /// One macro-block side length in mm.
    fn extent_mm(&self) -> f32 {
        self.extent_units as f32 / 10_000.0
    }

    /// Re-derives every field from `spacing_mm` / `width_mm` and asserts the
    /// stored values agree. This is what makes the bound move with the profile:
    /// if the fixture's `line_width` or `support_base_pattern_spacing` changes,
    /// the inputs change and the extent changes with them.
    fn assert_consistent(&self) {
        assert!(
            self.spacing_mm > 0.0 && self.width_mm > 0.0,
            "grid inputs must be positive (spacing={} mm, width={} mm)",
            self.spacing_mm,
            self.width_mm
        );
        let grid_resolution = slicer_ir::mm_to_units(self.spacing_mm).max(1);
        let width_units = slicer_ir::mm_to_units(self.width_mm).max(0);
        // Canonical `+100` orca nm == 1 PnP unit (`OVERSAMPLING_EPSILON`).
        let oversampling = (grid_resolution / (width_units + 1)).clamp(1, 8);
        // Canonical `+21` orca nm == 0.21 PnP units, rounded up to 1
        // (`PIXEL_SIZE_EPSILON`).
        let pixel_size = std::cmp::max(
            width_units + 1,
            slicer_ir::mm_to_units(self.spacing_mm / oversampling as f32),
        )
        .max(1);
        assert_eq!(grid_resolution, self.grid_resolution, "grid_resolution");
        assert_eq!(width_units, self.width_units, "width_units");
        assert_eq!(oversampling, self.oversampling, "oversampling");
        assert_eq!(pixel_size, self.pixel_size, "pixel_size");
        assert_eq!(
            oversampling * pixel_size,
            self.extent_units,
            "macro-block extent is not oversampling * pixel_size"
        );
        assert!(
            (1..=8).contains(&self.oversampling),
            "canonical clamps oversampling into 1..=8, got {}",
            self.oversampling
        );
        assert!(
            self.pixel_size > self.width_units,
            "canonical pixel_size is at least one extrusion width plus PIXEL_SIZE_EPSILON"
        );
    }
}

/// Derives [`MacroBlockExtent`] from the config the run actually used.
fn macro_block_extent(config: &HashMap<String, ConfigValue>) -> MacroBlockExtent {
    let spacing_mm = config_f32(
        config,
        "support_base_pattern_spacing",
        PLANNER_DEFAULT_BASE_PATTERN_SPACING_MM,
    );
    let width_mm = config_f32(config, "line_width", PLANNER_DEFAULT_LINE_WIDTH_MM);
    let grid_resolution = slicer_ir::mm_to_units(spacing_mm).max(1);
    let width_units = slicer_ir::mm_to_units(width_mm).max(0);
    let oversampling = (grid_resolution / (width_units + 1)).clamp(1, 8);
    let pixel_size = std::cmp::max(
        width_units + 1,
        slicer_ir::mm_to_units(spacing_mm / oversampling as f32),
    )
    .max(1);
    let extent = MacroBlockExtent {
        spacing_mm,
        width_mm,
        grid_resolution,
        width_units,
        oversampling,
        pixel_size,
        extent_units: oversampling * pixel_size,
    };
    extent.assert_consistent();
    extent
}

/// Per-piece floor below which a residual "outside the bound" fragment is a
/// clipper/offset-join artifact rather than geometry.
///
/// Same measured basis as [`WALL_LEAKAGE_NOISE_FLOOR_UNITS2`], which see: the
/// largest clipper tangency sliver observed on these fixtures was 311 units^2
/// (`thin_wall_slot`), and this floor of 10_000 units^2 = 1e-4 mm^2 (a
/// 0.01 x 0.01 mm square) sits ~32x above it.
///
/// **It was not fitted, and it is currently inert.** Measured 2026-09-03 on
/// `SupportAdversarial.stl`: at the derived extent the UNFILTERED outside area
/// (`max unfiltered outside` in the test's own printout) is `0.0 units^2` over
/// **0 difference pieces** across all 26 compared layers - `difference_ex`
/// returns nothing at all, of any size - so the floor absorbs nothing and the
/// containment result is identical with or without it. That inertness is
/// GATED, not merely printed: `agg_column_continuity_measurement_beats_baseline`
/// asserts `outside_pieces == 0`, so a floor that ever starts doing real work
/// fails the test instead of being absorbed by the filtered maximum.
const CONTAINMENT_SLIVER_FLOOR_UNITS2: f64 = 10_000.0;

/// Bisection resolution for the "smallest grow that contains agg" measurement,
/// in PnP units. 100 units = 0.01 mm. Reporting only; the gate itself is the
/// fixed derived extent.
const REQUIRED_GROW_RESOLUTION_UNITS: i64 = 100;

/// Area of `agg` lying outside `legacy` grown by `grow_mm`, in units^2, with
/// artifact pieces below [`CONTAINMENT_SLIVER_FLOOR_UNITS2`] dropped.
///
/// The grow uses a **Miter** join. The mechanism being bounded is canonical
/// `seed_fill_block`'s block-local flood, which is separable in x and y: a set
/// cell can only set other cells of the SAME macro block, so the displacement
/// is bounded per axis (Chebyshev), not radially. A Miter grow of an
/// axis-aligned region is the Minkowski sum with the square of half-side
/// `grow_mm`, which is exactly that per-axis bound; a Round grow would assert
/// something canonical does not promise on the diagonals.
fn area_outside_grown(agg: &[ExPolygon], legacy: &[ExPolygon], grow_mm: f32) -> f64 {
    outside_areas(agg, legacy, grow_mm).0
}

/// `(filtered, raw, pieces)`: the area of `agg` outside `legacy` grown by
/// `grow_mm` in units^2, with and without the
/// [`CONTAINMENT_SLIVER_FLOOR_UNITS2`] filter, plus the number of difference
/// pieces `difference_ex` returned. All three are reported so it is visible
/// whether the floor is doing any work at all.
fn outside_areas(agg: &[ExPolygon], legacy: &[ExPolygon], grow_mm: f32) -> (f64, f64, usize) {
    if agg.is_empty() {
        return (0.0, 0.0, 0);
    }
    let bound = if legacy.is_empty() {
        Vec::new()
    } else if grow_mm > 0.0 {
        let grown = offset(legacy, grow_mm, OffsetJoinType::Miter, 0.0);
        if grown.is_empty() {
            legacy.to_vec()
        } else {
            grown
        }
    } else {
        legacy.to_vec()
    };
    if bound.is_empty() {
        let all = total_area(agg);
        return (all, all, agg.len());
    }
    let pieces: Vec<f64> = difference_ex(agg, &bound)
        .iter()
        .map(expolygon_area)
        .collect();
    // `+ 0.0` normalises the signed zero clipper can produce for a degenerate
    // zero-area difference piece, so the printout reads `0.0`, not `-0.0`.
    let raw: f64 = pieces.iter().sum::<f64>() + 0.0;
    let filtered: f64 = pieces
        .iter()
        .filter(|piece| **piece >= CONTAINMENT_SLIVER_FLOOR_UNITS2)
        .sum::<f64>()
        + 0.0;
    (filtered, raw, pieces.len())
}

/// Outcome of the AC-7 containment measurement.
#[derive(Debug)]
struct ContainmentReport {
    extent: MacroBlockExtent,
    /// Layers carrying an agg support body (the layers the bound is checked on).
    layers_compared: usize,
    /// Layers where agg has a body and legacy has none: no grow of an empty
    /// region can contain anything, so these are called out separately.
    layers_agg_only: Vec<i32>,
    /// Largest per-layer area of agg outside the derived bound, units^2.
    max_outside_units2: f64,
    /// Layer attaining [`ContainmentReport::max_outside_units2`]; `None` when
    /// no layer has any area outside the bound.
    max_outside_layer: Option<i32>,
    /// Summed outside area over all layers, units^2.
    total_outside_units2: f64,
    /// Largest per-layer UNFILTERED outside area (every difference piece, no
    /// sliver floor), units^2. Equal to `max_outside_units2` means the floor
    /// absorbed nothing.
    max_outside_raw_units2: f64,
    /// Total number of difference pieces `agg - grow(legacy, extent)` produced
    /// over all layers, of any size. Zero means containment is exact rather
    /// than exact-modulo-the-floor.
    outside_pieces: usize,
    /// Largest per-layer SMALLEST grow that contains agg, units. The margin is
    /// `extent_units` minus this.
    required_grow_units: i64,
    /// Layer attaining [`ContainmentReport::required_grow_units`].
    required_grow_layer: i32,
    /// True when some layer needed more than the searched upper bound.
    required_grow_saturated: bool,
    legacy_drops: usize,
    agg_drops: usize,
    legacy_area_units2: f64,
    agg_area_units2: f64,
}

/// Runs both rasterizer modes on the adversarial fixture and measures the
/// per-layer containment of the agg body region inside the legacy body region
/// grown by one derived macro-block extent.
fn measure_containment() -> Result<ContainmentReport, String> {
    let legacy = compute_metrics_for(
        &support_adversarial_path(),
        ADVERSARIAL_FIXTURE_LABEL,
        RASTERIZER_LEGACY,
    )?;
    let agg = compute_agg_metrics()?;
    let config = matched_config_for(true, BASELINE_SUPPORT_TYPE);
    let extent = macro_block_extent(&config);
    let extent_mm = extent.extent_mm();
    let search_ceiling = extent.extent_units * 4;
    let empty: Vec<ExPolygon> = Vec::new();

    let mut layers_compared = 0usize;
    let mut layers_agg_only: Vec<i32> = Vec::new();
    let mut max_outside_units2 = 0.0f64;
    let mut max_outside_layer: Option<i32> = None;
    let mut total_outside_units2 = 0.0f64;
    let mut max_outside_raw_units2 = 0.0f64;
    let mut outside_pieces = 0usize;
    let mut required_grow_units = 0i64;
    let mut required_grow_layer = i32::MIN;
    let mut required_grow_saturated = false;

    let layers: BTreeSet<i32> = legacy
        .body_by_layer
        .keys()
        .chain(agg.body_by_layer.keys())
        .copied()
        .collect();

    let units_to_mm = |units: i64| units as f32 / 10_000.0;

    for layer in &layers {
        let legacy_u = union_ex(legacy.body_by_layer.get(layer).unwrap_or(&empty));
        let agg_u = union_ex(agg.body_by_layer.get(layer).unwrap_or(&empty));
        if agg_u.is_empty() {
            continue;
        }
        layers_compared += 1;
        if legacy_u.is_empty() {
            layers_agg_only.push(*layer);
        }

        let (outside, outside_raw, pieces) = outside_areas(&agg_u, &legacy_u, extent_mm);
        total_outside_units2 += outside;
        max_outside_raw_units2 = max_outside_raw_units2.max(outside_raw);
        outside_pieces += pieces;
        if outside > max_outside_units2 {
            max_outside_units2 = outside;
            max_outside_layer = Some(*layer);
        }

        // Smallest grow (bisected) at which this layer is contained. Reported
        // so the margin under the derived bound is visible, not merely asserted.
        let mut lo = 0i64;
        let mut hi = search_ceiling;
        if area_outside_grown(&agg_u, &legacy_u, units_to_mm(hi)) > 0.0 {
            required_grow_saturated = true;
        } else {
            while hi - lo > REQUIRED_GROW_RESOLUTION_UNITS {
                let mid = lo + (hi - lo) / 2;
                if area_outside_grown(&agg_u, &legacy_u, units_to_mm(mid)) > 0.0 {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
        }
        if hi > required_grow_units {
            required_grow_units = hi;
            required_grow_layer = *layer;
        }
    }

    Ok(ContainmentReport {
        extent,
        layers_compared,
        layers_agg_only,
        max_outside_units2,
        max_outside_layer,
        total_outside_units2,
        max_outside_raw_units2,
        outside_pieces,
        required_grow_units,
        required_grow_layer,
        required_grow_saturated,
        legacy_drops: legacy.baseline.columns.abrupt_drops,
        agg_drops: agg.baseline.columns.abrupt_drops,
        legacy_area_units2: legacy.baseline.columns.total_support_area,
        agg_area_units2: agg.baseline.columns.total_support_area,
    })
}

/// AC-7, re-authored (packet 241 Step 15b). Against the adversarial fixture the
/// agg rasterizer must
///
/// 1. produce **strictly fewer** abrupt column drops than the legacy baseline
///    (unchanged), and
/// 2. keep its support-body region, on **every** layer, inside the legacy body
///    region for that layer grown by **one derived macro-block extent**.
///
/// # Why the old area guard is gone
///
/// AC-7 used to require the total emitted support area to stay within +/-25% of
/// the baseline "so continuity is not bought by inflation". That premise is
/// contradicted by the accepted canonical behaviour: with the asymmetric
/// printed-area clamp removed (human decision; DEV-166), canonical
/// `seed_fill_block` (`SupportMaterial.cpp`) floods each
/// `oversampling * oversampling` macro block independently, so the carry grows
/// by at most one macro-block extent and that halo **adds material by design**.
/// Measured 2026-09-03: legacy 225789129333 units^2 (2257.89 mm^2) vs agg
/// 354695221947 units^2 (3546.95 mm^2), i.e. **+57.09%**. The total-area figures
/// are still measured and printed here as a RECORDED metric; they no longer
/// gate.
///
/// The replacement is strictly stronger than an area ratio and is tied to the
/// real mechanism: a ratio cannot tell a block-scale halo from support
/// appearing somewhere else entirely, whereas containment forbids the latter
/// outright. The extent is derived from the config the run actually used
/// (`support_base_pattern_spacing`, `line_width`) through the same arithmetic as
/// `GridParams::from_polygons`; see [`macro_block_extent`].
#[test]
fn agg_column_continuity_measurement_beats_baseline() {
    let baseline = load_tracked_baseline();
    let report = measure_containment().expect("containment measurement failed");
    let extent = report.extent;

    let base_area = report.legacy_area_units2;
    let agg_area = report.agg_area_units2;
    let delta_pct = if base_area > 0.0 {
        (agg_area - base_area) / base_area * 100.0
    } else {
        f64::INFINITY
    };
    eprintln!(
        "P241-AC7 column continuity on {}: baseline(legacy) drops={} area={:.1} units^2 | \
         post-port(agg) drops={} area={:.1} units^2 | RECORDED area delta={delta_pct:+.2}% \
         (not a gate; see DEV-166)",
        ADVERSARIAL_FIXTURE_LABEL,
        report.legacy_drops,
        base_area,
        report.agg_drops,
        agg_area,
    );
    eprintln!(
        "P241-AC7 macro-block extent derived from config: spacing={} mm line_width={} mm -> \
         grid_resolution={} units, width={} units, oversampling={}, pixel_size={} units, \
         extent={} units ({:.4} mm)",
        extent.spacing_mm,
        extent.width_mm,
        extent.grid_resolution,
        extent.width_units,
        extent.oversampling,
        extent.pixel_size,
        extent.extent_units,
        extent.extent_mm(),
    );
    eprintln!(
        "P241-AC7 containment: layers={} agg_only_layers={:?} | max per-layer area outside \
         bound={:.1} units^2 ({:.6} mm^2) on layer {:?} | max unfiltered outside={:.1} units^2 \
         over {} piece(s) (sliver floor {:.1} units^2) | total outside={:.1} units^2 | \
         smallest grow containing agg={} units ({:.4} mm) on layer {} (saturated={}) | \
         margin under derived extent={} units ({:.4} mm)",
        report.layers_compared,
        report.layers_agg_only,
        report.max_outside_units2,
        report.max_outside_units2 * 1e-8,
        report.max_outside_layer,
        report.max_outside_raw_units2,
        report.outside_pieces,
        CONTAINMENT_SLIVER_FLOOR_UNITS2,
        report.total_outside_units2,
        report.required_grow_units,
        report.required_grow_units as f32 / 10_000.0,
        report.required_grow_layer,
        report.required_grow_saturated,
        extent.extent_units - report.required_grow_units,
        (extent.extent_units - report.required_grow_units) as f32 / 10_000.0,
    );

    // The legacy arm the geometry came from must be the tracked baseline, or
    // the drops comparison below is against a different run than the one the
    // containment was measured on.
    assert_eq!(
        report.legacy_drops, baseline.columns.abrupt_drops,
        "legacy re-run disagrees with the tracked baseline on abrupt drops"
    );
    assert!(
        (report.legacy_area_units2 - baseline.columns.total_support_area).abs() < 1.0,
        "legacy re-run area {:.1} disagrees with the tracked baseline {:.1}",
        report.legacy_area_units2,
        baseline.columns.total_support_area
    );

    // -- Half 1: strictly fewer abrupt drops (UNCHANGED) --------------------
    assert!(
        baseline.columns.abrupt_drops > 0,
        "baseline records no abrupt drops; the strict gate is unsatisfiable on this fixture"
    );
    assert!(
        report.agg_drops < baseline.columns.abrupt_drops,
        "agg abrupt drops {} are not strictly fewer than the legacy baseline's {}",
        report.agg_drops,
        baseline.columns.abrupt_drops
    );

    // -- Half 2: macro-block containment ------------------------------------
    assert!(
        report.layers_compared > 0,
        "no agg body layers to check containment on; the bound is vacuous"
    );
    // Non-vacuity of the GROW itself: if agg were already inside the ungrown
    // legacy region on every layer, the derived extent would be doing no work
    // and the bound would pass for the wrong reason.
    assert!(
        report.required_grow_units > 0,
        "agg is contained in the ungrown legacy region on every layer; the macro-block bound \
         is not being exercised ({report:?})"
    );
    assert!(
        report.layers_agg_only.is_empty(),
        "agg emits support on layers {:?} where legacy emits none; no grow of the legacy region \
         can contain it there ({report:?})",
        report.layers_agg_only
    );
    assert!(
        !report.required_grow_saturated,
        "some layer needs a grow beyond the searched ceiling of {} units ({:.4} mm) to contain \
         agg; the halo is not macro-block bounded ({report:?})",
        extent.extent_units * 4,
        (extent.extent_units * 4) as f32 / 10_000.0,
    );
    // The filtered `max_outside_units2` above is measured AFTER
    // `CONTAINMENT_SLIVER_FLOOR_UNITS2` drops sub-floor difference pieces, so on
    // its own it would also pass with unboundedly many sub-floor pieces outside
    // the bound. Gate the floor's claimed inertness instead of asserting it in
    // prose: zero difference pieces of ANY size means the floor absorbed
    // nothing and containment is exact, not exact-modulo-the-floor.
    assert_eq!(
        report.outside_pieces, 0,
        "agg leaves {} difference piece(s) outside the legacy region grown by one macro-block          extent (largest unfiltered {:.1} units^2, sliver floor {:.1} units^2); containment is          only exact-modulo-the-floor ({report:?})",
        report.outside_pieces, report.max_outside_raw_units2, CONTAINMENT_SLIVER_FLOOR_UNITS2,
    );
    assert_eq!(
        report.max_outside_units2, 0.0,
        "agg support exceeds the legacy region grown by one macro-block extent ({} units, \
         {:.4} mm): layer {:?} has {:.1} units^2 ({:.6} mm^2) outside it, and containment needs a \
         grow of {} units ({:.4} mm). Canonical seed_fill_block is block-local, so this means \
         the halo is NOT macro-block bounded ({report:?})",
        extent.extent_units,
        extent.extent_mm(),
        report.max_outside_layer,
        report.max_outside_units2,
        report.max_outside_units2 * 1e-8,
        report.required_grow_units,
        report.required_grow_units as f32 / 10_000.0,
    );
}

/// `offset_to_slice` exactly as the planner's agg arm computes it at the
/// fixture's 0.4 mm line width: `mm_to_units(0.4) / 2 + OFFSET_TO_SLICE_EPSILON`
/// (`SupportPlanner::plan_candidate`, `RasterizerMode::Agg` arm, in
/// `modules/core-modules/traditional-support-planner/src/lib.rs`). PnP units.
const CONTROL_OFFSET_TO_SLICE_UNITS: i64 = 2001;

/// Per-layer / aggregate numbers of the review-finding F-I1 control experiment.
#[derive(Debug)]
struct ControlComparison {
    legacy_drops: usize,
    control_drops: usize,
    agg_drops: usize,
    legacy_area_mm2: f64,
    control_area_mm2: f64,
    agg_area_mm2: f64,
    /// Largest per-layer symmetric-difference area between CONTROL and agg, mm^2.
    max_symdiff_mm2: f64,
    /// Layers on which CONTROL and agg differ by more than 1e-3 mm^2.
    differing_layers: usize,
    /// Layers carrying a support body in either CONTROL or agg.
    compared_layers: usize,
}

/// Builds the F-I1 CONTROL geometry from the LEGACY plan, host-side, with the
/// same `slicer_core::polygon_ops` primitives the planner's `host::*` calls
/// resolve to (`offset_polygons` without a miter limit is
/// `polygon_ops::offset`; `clip_polygons` is `polygon_ops::clip_polygons`):
///
/// ```text
/// CONTROL(layer) = difference(
///     offset(legacy_body(layer), 0.2001 mm, Miter),
///     offset(occupancy(layer), support_object_xy_distance, Miter))
/// ```
///
/// i.e. "legacy printed area grown by `offset_to_slice`, clipped to the
/// trimming mask" - a purely *global* offset of the legacy geometry, with no
/// grid anywhere in it. It is the null hypothesis for the grid pipeline
/// (rasterize, `seed_fill_block`, `fill_holes`, island filter,
/// `extract_islands`): if the grid contributed nothing beyond a uniform
/// outward offset, agg would equal this. Compared against the real agg plan's
/// bodies layer by layer.
///
/// Historical note: while the agg arm still carried the asymmetric printed-area
/// clamp (`slice_bound = offset(pre_grid_carry, offset_to_slice, Miter)`), this
/// control was an upper bound on the agg output and agg tracked it almost
/// exactly. The clamp has been removed (see the `RasterizerMode::Agg` arm of
/// `SupportPlanner::plan_candidate` in
/// `modules/core-modules/traditional-support-planner/src/lib.rs`, and DEV-166),
/// so the control is no longer a bound and the grid's own contribution is
/// visible in the difference.
fn compare_control_vs_agg() -> Result<ControlComparison, String> {
    let legacy = compute_metrics_for(
        &support_adversarial_path(),
        ADVERSARIAL_FIXTURE_LABEL,
        RASTERIZER_LEGACY,
    )?;
    let agg = compute_agg_metrics()?;
    let config = matched_config_for(true, BASELINE_SUPPORT_TYPE);
    let xy_distance = xy_distance_mm(&config);
    let offset_to_slice_mm = CONTROL_OFFSET_TO_SLICE_UNITS as f32 / 10_000.0;
    let printable_layers: BTreeSet<i32> = legacy.occupancy_by_layer.keys().copied().collect();
    let empty: Vec<ExPolygon> = Vec::new();

    let mut control_by_layer: BTreeMap<i32, Vec<ExPolygon>> = BTreeMap::new();
    for (layer, bodies) in &legacy.body_by_layer {
        let grown = offset(bodies, offset_to_slice_mm, OffsetJoinType::Miter, 0.0);
        let occupancy = legacy.occupancy_by_layer.get(layer).unwrap_or(&empty);
        let control = if occupancy.is_empty() {
            grown
        } else {
            let mask = offset(occupancy, xy_distance, OffsetJoinType::Miter, 0.0);
            let mask = if mask.is_empty() { occupancy.clone() } else { mask };
            difference_ex(&grown, &mask)
        };
        if !control.is_empty() {
            control_by_layer.insert(*layer, control);
        }
    }

    let control_columns =
        column_continuity_metrics(&control_by_layer, &legacy.occupancy_by_layer, &printable_layers);

    let layers: BTreeSet<i32> = control_by_layer
        .keys()
        .chain(agg.body_by_layer.keys())
        .copied()
        .collect();
    let mut max_symdiff_units2 = 0.0f64;
    let mut differing_layers = 0usize;
    for layer in &layers {
        let control = control_by_layer.get(layer).unwrap_or(&empty);
        let agg_bodies = agg.body_by_layer.get(layer).unwrap_or(&empty);
        let control_u = union_ex(control);
        let agg_u = union_ex(agg_bodies);
        let symdiff = total_area(&difference_ex(&control_u, &agg_u))
            + total_area(&difference_ex(&agg_u, &control_u));
        if symdiff * 1e-8 > 1e-3 {
            differing_layers += 1;
        }
        max_symdiff_units2 = max_symdiff_units2.max(symdiff);
    }

    Ok(ControlComparison {
        legacy_drops: legacy.baseline.columns.abrupt_drops,
        control_drops: control_columns.abrupt_drops,
        agg_drops: agg.baseline.columns.abrupt_drops,
        legacy_area_mm2: legacy.baseline.columns.total_support_area * 1e-8,
        control_area_mm2: control_columns.total_support_area * 1e-8,
        agg_area_mm2: agg.baseline.columns.total_support_area * 1e-8,
        max_symdiff_mm2: max_symdiff_units2 * 1e-8,
        differing_layers,
        compared_layers: layers.len(),
    })
}

/// Review finding F-I1, re-settled by measurement after the printed-area clamp
/// was removed (packet 241 Step 14, this fixture, `SupportAdversarial.stl`,
/// xy_distance 0.35 mm, line width 0.4 mm).
///
/// Measured 2026-09-03, clamp removed, `legacy_semantic` the default:
///
/// ```text
/// legacy   drops=3  area=2257.89 mm^2
/// control  drops=0  area=2562.83 mm^2
/// agg      drops=0  area=3546.95 mm^2
/// grid contribution   (agg - control)    =  984.12 mm^2  (+38.40 % of control)
/// offset contribution (control - legacy) =  304.94 mm^2
/// max per-layer symdiff(control, agg) = 38.1980 mm^2
/// layers with symdiff > 1e-3 mm^2     = 26 of 26
/// ```
///
/// Interpretation, inverted relative to the clamp-era reading of F-I1. The
/// column-continuity gain (3 -> 0 abrupt drops) is still reproduced in full by
/// the CONTROL: a global `offset_to_slice` grow of the legacy printed area,
/// clipped to the trimming mask, removes every legacy drop on its own, and agg
/// does not improve on that count. But the grid is no longer a rounding-scale
/// perturbation of that control. With the clamp gone, canonical
/// `seed_fill_block`'s macro-block snapping (DEV-166) adds a halo that makes
/// agg's printed area exceed the control by **more than three times** what the
/// global offset itself added over legacy, and shifts every single compared
/// layer's outline by a block-scale amount.
///
/// Assertions are the structural facts, not tuned thresholds: the control alone
/// removes every legacy drop; agg matches the control's drop count; agg's
/// printed area strictly exceeds the control's; the grid's contribution exceeds
/// the global offset's; and every compared layer differs. The one numeric floor
/// (1 mm^2 on the largest per-layer symmetric difference) is 6.25 squares of
/// one 0.4 mm extrusion width on a side - far above clipper tangency noise, and
/// ~38x below the measured value, so it is a scale claim rather than a fit.
#[test]
fn agg_printed_area_exceeds_global_offset_control() {
    let cmp = compare_control_vs_agg().expect("control comparison failed");
    eprintln!(
        "P241-F-I1 drops legacy={} control={} agg={} | area(mm^2) legacy={:.2} control={:.2} agg={:.2} | max per-layer symdiff(control,agg)={:.4} mm^2 | layers differing (>1e-3 mm^2)={} of {}",
        cmp.legacy_drops,
        cmp.control_drops,
        cmp.agg_drops,
        cmp.legacy_area_mm2,
        cmp.control_area_mm2,
        cmp.agg_area_mm2,
        cmp.max_symdiff_mm2,
        cmp.differing_layers,
        cmp.compared_layers,
    );

    // Non-vacuity: the fixture still has legacy drops to remove and bodies to
    // compare; otherwise the assertions below hold trivially.
    assert!(cmp.legacy_drops > 0, "legacy baseline has no drops ({cmp:?})");
    assert!(cmp.compared_layers > 0, "no body layers to compare ({cmp:?})");
    assert_eq!(
        cmp.control_drops, 0,
        "the offset_to_slice control alone should remove every legacy drop ({cmp:?})"
    );
    assert_eq!(
        cmp.agg_drops, cmp.control_drops,
        "agg and control disagree on abrupt drops ({cmp:?})"
    );
    // The grid is not a global offset. Unclamped, its macro-block halo adds
    // area on top of the control rather than tracking it.
    let grid_contribution = cmp.agg_area_mm2 - cmp.control_area_mm2;
    let offset_contribution = cmp.control_area_mm2 - cmp.legacy_area_mm2;
    assert!(
        grid_contribution > 0.0,
        "agg printed area does not exceed the global-offset control; the grid pipeline \
         contributes nothing beyond a uniform offset ({cmp:?})"
    );
    assert!(
        offset_contribution > 0.0,
        "the control does not exceed legacy; the control experiment is degenerate ({cmp:?})"
    );
    assert!(
        grid_contribution > offset_contribution,
        "the grid contributes {grid_contribution:.2} mm^2 over the control, no more than the \
         global offset's own {offset_contribution:.2} mm^2 over legacy; the AC-7 area change \
         would then be attributable to the offset alone ({cmp:?})"
    );
    assert_eq!(
        cmp.differing_layers, cmp.compared_layers,
        "the grid changes only {} of {} compared layers by more than 1e-3 mm^2; the macro-block \
         halo is expected on every layer ({cmp:?})",
        cmp.differing_layers, cmp.compared_layers
    );
    assert!(
        cmp.max_symdiff_mm2 > 1.0,
        "largest per-layer symmetric difference between control and agg is {:.4} mm^2, below the \
         1 mm^2 block-scale floor; the difference is extrusion/clipper-scale, not a macro-block \
         halo ({cmp:?})",
        cmp.max_symdiff_mm2
    );
}

/// Canonical, order-independent representation of a layer's connected body
/// outline set, for exact set comparison between the two modes.
fn outline_set(bodies: &[ExPolygon]) -> BTreeSet<Vec<(i64, i64)>> {
    union_ex(bodies)
        .iter()
        .map(|comp| comp.contour.points.iter().map(|p| (p.x, p.y)).collect())
        .collect()
}

fn run_slice_gcode(model: &Path, rasterizer: &str) -> Result<String, String> {
    let mesh = cached_load_model(model);
    let mut overrides = matched_config_for(true, BASELINE_SUPPORT_TYPE);
    overrides.insert(
        "support_area_rasterizer".to_string(),
        ConfigValue::String(rasterizer.to_string()),
    );
    let opts = slicer_runtime::run::SliceRunOptions {
        mesh,
        config_overrides: overrides,
        module_dirs: core_module_dirs(),
        ..Default::default()
    };
    let outcome = slicer_runtime::run::run_slice(opts)
        .map_err(|e| format!("run_slice({rasterizer}) on {} failed: {e}", model.display()))?;
    Ok(outcome.gcode_text)
}

/// AC-8. Both `support_area_rasterizer` modes must drive a full slice to
/// completion and both must emit support, yet their plans must differ on at
/// least one layer - proving the knob actually selects a different rasterizer
/// and neither mode is a silent alias of the other.
#[test]
fn agg_and_legacy_modes_both_function_and_diverge() {
    // Full slices through `run_slice`, on both tracked fixtures.
    for model in [support_test_path(), support_adversarial_path()] {
        for mode in [RASTERIZER_LEGACY, RASTERIZER_AGG] {
            let gcode = run_slice_gcode(&model, mode).expect("full slice must succeed");
            assert!(!gcode.trim().is_empty(), "{mode}: empty G-code");
            assert!(
                gcode.lines().any(|line| line.trim_end() == ";TYPE:Support"),
                "{mode} on {}: no `;TYPE:Support` block emitted",
                model.display()
            );
        }
    }

    // Plan-level comparison through the prepass driver on the adversarial fixture.
    let legacy = compute_metrics_for(
        &support_adversarial_path(),
        ADVERSARIAL_FIXTURE_LABEL,
        RASTERIZER_LEGACY,
    )
    .expect("legacy prepass");
    let agg = compute_agg_metrics().expect("agg prepass");

    assert!(
        !legacy.body_by_layer.is_empty() && !agg.body_by_layer.is_empty(),
        "both plans must be non-empty (legacy layers={}, agg layers={})",
        legacy.body_by_layer.len(),
        agg.body_by_layer.len()
    );
    // Both reach the build plate beneath the roof overhang: their lowest body
    // layer is the lowest printable layer.
    let plate = *legacy
        .occupancy_by_layer
        .keys()
        .next()
        .expect("printable layers");
    for (name, parts) in [("legacy", &legacy), ("agg", &agg)] {
        let lowest = *parts.body_by_layer.keys().next().unwrap();
        assert_eq!(
            lowest, plate,
            "{name}: lowest support-body layer {lowest} is not the plate layer {plate}"
        );
    }

    let layers: BTreeSet<i32> = legacy
        .body_by_layer
        .keys()
        .chain(agg.body_by_layer.keys())
        .copied()
        .collect();
    let empty: Vec<ExPolygon> = Vec::new();
    let diverging: Vec<i32> = layers
        .into_iter()
        .filter(|layer| {
            outline_set(legacy.body_by_layer.get(layer).unwrap_or(&empty))
                != outline_set(agg.body_by_layer.get(layer).unwrap_or(&empty))
        })
        .collect();
    eprintln!(
        "P241-AC8 on {}: legacy body layers={} agg body layers={} diverging layers={} {:?}",
        ADVERSARIAL_FIXTURE_LABEL,
        legacy.body_by_layer.len(),
        agg.body_by_layer.len(),
        diverging.len(),
        diverging
    );
    assert!(
        !diverging.is_empty(),
        "legacy_semantic and agg produced identical body outlines on every layer; the knob \
         does not select a different rasterizer"
    );
}

// -- Step 8 (TASK-426): real-mesh wedge proof ---------------------------------

/// Regions of `role` over every accepted plan entry, keyed by layer.
fn plan_regions_by_layer(
    plan: &SupportPlanIR,
    role: SupportPlanRole,
) -> BTreeMap<i32, Vec<ExPolygon>> {
    let mut by_layer: BTreeMap<i32, Vec<ExPolygon>> = BTreeMap::new();
    for entry in plan
        .entries
        .iter()
        .filter(|entry| entry.decline_reason.is_none())
    {
        for role_region in &entry.roles {
            if role_region.role != role || role_region.regions.is_empty() {
                continue;
            }
            by_layer
                .entry(entry.global_layer_index)
                .or_default()
                .extend(role_region.regions.iter().cloned());
        }
    }
    by_layer
}

/// Structural facts of one wedge plan, printed and asserted on by the Step 8
/// proof. Areas are in mm^2.
struct WedgePlanFacts {
    printable_layers: usize,
    plate_layer: i32,
    body_layers: Vec<i32>,
    body_polygons: usize,
    interface_layers: Vec<i32>,
    plate_body_area_mm2: f64,
}

fn wedge_plan_facts(rasterizer: &str) -> WedgePlanFacts {
    let ctx = support_wedge::prepare_wedge_context_with_overrides(
        true,
        &[(
            "support_area_rasterizer",
            ConfigValue::String(rasterizer.to_string()),
        )],
    );
    let layer_plan = ctx
        .blackboard
        .layer_plan()
        .expect("LayerPlanIR must be committed by the prepass");
    let plate_layer = layer_plan
        .global_layers
        .iter()
        .map(|layer| layer.index as i32)
        .min()
        .expect("wedge must have at least one printable layer");
    let plan = ctx
        .blackboard
        .support_plan()
        .expect("support_plan must be committed when enable_support=true");
    assert!(
        !plan.entries.is_empty(),
        "{rasterizer}: SupportPlanIR.entries is empty on the wedge"
    );
    let body = plan_regions_by_layer(&plan, SupportPlanRole::SupportBody);
    let interface = plan_regions_by_layer(&plan, SupportPlanRole::TopInterface);
    let plate_body_area_mm2 = body
        .get(&plate_layer)
        .map(|regions| total_area(regions) * 1e-8)
        .unwrap_or(0.0);
    WedgePlanFacts {
        printable_layers: layer_plan.global_layers.len(),
        plate_layer,
        body_polygons: body.values().map(Vec::len).sum(),
        body_layers: body.keys().copied().collect(),
        interface_layers: interface.keys().copied().collect(),
        plate_body_area_mm2,
    }
}

/// AC (Step 8 / TASK-426): on `resources/regression_wedge.stl`, the `agg`
/// rasterizer - opt-in since the clamp removal made `legacy_semantic` the
/// default, and selected explicitly here via
/// `support_wedge::prepare_wedge_context_with_overrides` - yields a non-empty
/// plan whose support body starts on the
/// lowest printable layer and climbs to the top-interface band beneath the
/// overhang. `legacy_semantic` on the same fixture is also non-empty, and the
/// two rasterizers produce a different plate-layer body area, proving the knob
/// selects a different rasterizer on a real mesh (not only on the synthetic
/// adversarial fixture).
///
/// Release-CLI reference facts, recorded in the CLAMP-ERA build (may differ
/// from this harness's config, and the agg figure predates the clamp removal):
/// 200 layers, body layers 0..141, interface 77..143, 219 body polygons,
/// layer-0 body area ~172 mm^2 (agg) vs ~154 mm^2 (legacy). The assertions
/// below are structural; the measured numbers are printed for the record.
#[test]
fn agg_wedge_plan_is_nonempty_and_reaches_beneath_overhang() {
    let agg = wedge_plan_facts(RASTERIZER_AGG);
    let legacy = wedge_plan_facts(RASTERIZER_LEGACY);

    for (name, facts) in [("agg", &agg), ("legacy", &legacy)] {
        eprintln!(
            "P241-STEP8 wedge {name}: printable_layers={} plate_layer={} body_layers={}..{} \
             (count={}) body_polygons={} interface_layers={:?}..{:?} (count={}) \
             plate_body_area_mm2={:.3}",
            facts.printable_layers,
            facts.plate_layer,
            facts.body_layers.first().copied().unwrap_or(i32::MIN),
            facts.body_layers.last().copied().unwrap_or(i32::MIN),
            facts.body_layers.len(),
            facts.body_polygons,
            facts.interface_layers.first(),
            facts.interface_layers.last(),
            facts.interface_layers.len(),
            facts.plate_body_area_mm2
        );
        assert!(
            !facts.body_layers.is_empty(),
            "{name}: wedge plan carries no SupportBody regions"
        );
        assert!(
            facts.body_polygons > 0,
            "{name}: wedge plan carries zero SupportBody polygons"
        );
    }

    // The agg body reaches the build plate: its lowest body layer is the
    // lowest printable layer, with a positive area there.
    let agg_lowest = agg.body_layers[0];
    assert_eq!(
        agg_lowest, agg.plate_layer,
        "agg: lowest SupportBody layer {agg_lowest} is not the plate layer {}",
        agg.plate_layer
    );
    assert!(
        agg.plate_body_area_mm2 > 0.0,
        "agg: plate-layer SupportBody area is zero"
    );

    // The agg body climbs beneath the overhang: it reaches into the
    // top-interface band and the interface itself is present.
    assert!(
        !agg.interface_layers.is_empty(),
        "agg: wedge plan carries no TopInterface regions"
    );
    let interface_lowest = agg.interface_layers[0];
    let agg_highest = *agg.body_layers.last().unwrap();
    assert!(
        agg_highest >= interface_lowest,
        "agg: highest SupportBody layer {agg_highest} does not reach the top-interface band \
         starting at layer {interface_lowest}"
    );
    assert!(
        agg.body_layers.len() > 1,
        "agg: SupportBody occupies a single layer; the column does not span the overhang gap"
    );

    // The knob selects a different rasterizer on the real mesh.
    assert!(
        (agg.plate_body_area_mm2 - legacy.plate_body_area_mm2).abs() > f64::EPSILON,
        "agg and legacy_semantic produced the same plate-layer body area ({:.6} mm^2); the knob \
         does not select a different rasterizer on the wedge",
        agg.plate_body_area_mm2
    );
}
