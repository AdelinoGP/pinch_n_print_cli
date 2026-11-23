//! Independent schedule and source model for `H-SUPPORT-SCHEDULE`.
//!
//! `execute_support_geometry` (`crates/slicer-core/src/algos/support_geometry.rs`)
//! publishes, per `(support layer, object, region, chain)`, the source polygons
//! that the support renderers consume. The model here re-derives the emission
//! schedule from the declared per-object target height, re-derives each
//! object's column top, and copies the declared source polygons; it never calls
//! `build_emit_schedule` or the geometry collector to build an expectation.

use std::collections::{BTreeMap, BTreeSet};

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use slicer_core::algos::support_geometry::{build_emit_schedule, execute_support_geometry};
use slicer_ir::{
    ActiveRegion, ExPolygon, GlobalLayer, LayerPlanIR, PaintValue, Point2, Polygon, ResolvedConfig,
    SemVer, SliceIR, SlicedRegion,
};
use slicer_sdk::test_support::property_testing::{CaseGuard, PropertyRun};

/// The sentinel support layer index the intermediate-model pass uses.
const INTERMEDIATE_SENTINEL: u32 = u32::MAX;
/// Source geometry side length in canonical units (2 mm).
const SQUARE: i64 = 20_000;

/// One region declared on one plan layer.
#[derive(Debug, Clone)]
pub struct ScheduleRegion {
    /// Index into the case's object list.
    pub object: usize,
    /// Region id.
    pub region_id: u64,
    /// Whether the committed slice carries geometry for this region.
    pub has_source: bool,
    /// Chain carried by the committed slice region, if any.
    pub chain: usize,
}

/// One plan layer.
#[derive(Debug, Clone)]
pub struct ScheduleLayer {
    /// Global layer index (unique, never the reserved sentinel).
    pub index: u32,
    /// Regions active on this layer.
    pub regions: Vec<ScheduleRegion>,
}

/// One generated scheduling scene.
#[derive(Debug, Clone)]
pub struct ScheduleCase {
    /// Object ids, at most three.
    pub objects: Vec<String>,
    /// Plan layers in increasing `z`, strictly unique indices.
    pub layers: Vec<ScheduleLayer>,
    /// Per-object support target height in millimetres; `0` means "every layer".
    pub targets_mm: Vec<f32>,
    /// Effective layer height in millimetres (same for every region).
    pub height_mm: f32,
    /// Support top-Z distance in millimetres.
    pub top_distance_mm: f32,
}

/// The two chain spellings the fixture commits.
#[must_use]
pub fn chain(spelling: usize) -> Vec<(String, PaintValue)> {
    match spelling % 2 {
        0 => Vec::new(),
        _ => vec![("material".to_owned(), PaintValue::ToolIndex(0))],
    }
}

fn square(offset: i64) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: offset, y: 0 },
                Point2 {
                    x: offset + SQUARE,
                    y: 0,
                },
                Point2 {
                    x: offset + SQUARE,
                    y: SQUARE,
                },
                Point2 {
                    x: offset,
                    y: SQUARE,
                },
            ],
        },
        holes: Vec::new(),
    }
}

impl ScheduleCase {
    /// Build the plan the system under test consumes.
    #[must_use]
    pub fn plan(&self) -> LayerPlanIR {
        let global_layers = self
            .layers
            .iter()
            .map(|layer| GlobalLayer {
                index: layer.index,
                z: 0.2 * (layer.index as f32 + 1.0),
                active_regions: layer
                    .regions
                    .iter()
                    .map(|region| ActiveRegion {
                        object_id: self.objects[region.object].clone(),
                        region_id: region.region_id,
                        resolved_config: ResolvedConfig {
                            support_layer_height_mm: self.targets_mm[region.object],
                            support_top_z_distance: self.top_distance_mm,
                            ..ResolvedConfig::default()
                        },
                        effective_layer_height: self.height_mm,
                        ..ActiveRegion::default()
                    })
                    .collect(),
                ..GlobalLayer::default()
            })
            .collect();
        LayerPlanIR {
            schema_version: SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            global_layers,
            object_participation: std::collections::HashMap::new(),
        }
    }

    /// Build the committed slices: one per plan layer, in plan order.
    #[must_use]
    pub fn slices(&self) -> Vec<SliceIR> {
        self.layers
            .iter()
            .map(|layer| SliceIR {
                global_layer_index: layer.index,
                z: 0.2 * (layer.index as f32 + 1.0),
                regions: layer
                    .regions
                    .iter()
                    .filter(|region| region.has_source)
                    .map(|region| SlicedRegion {
                        object_id: self.objects[region.object].clone(),
                        region_id: region.region_id,
                        polygons: vec![square(region.object as i64 * SQUARE * 2)],
                        variant_chain: chain(region.chain),
                        ..SlicedRegion::default()
                    })
                    .collect(),
                ..SliceIR::default()
            })
            .collect()
    }

    /// Independently derived emission schedule per object.
    #[must_use]
    pub fn expected_schedule(&self) -> BTreeMap<String, BTreeSet<u32>> {
        let mut schedule: BTreeMap<String, BTreeSet<u32>> = BTreeMap::new();
        let mut acc: BTreeMap<String, f32> = BTreeMap::new();
        for layer in &self.layers {
            let mut seen: BTreeMap<usize, ()> = BTreeMap::new();
            for region in &layer.regions {
                seen.insert(region.object, ());
            }
            for object in seen.keys() {
                let target = self.targets_mm[*object];
                let entry = acc.entry(self.objects[*object].clone()).or_insert(0.0);
                *entry += self.height_mm;
                if target == 0.0 || *entry >= target {
                    schedule
                        .entry(self.objects[*object].clone())
                        .or_default()
                        .insert(layer.index);
                    *entry = 0.0;
                }
            }
        }
        schedule
    }

    fn source_polygons(
        &self,
        layer_pos: usize,
        object: usize,
        region_id: u64,
    ) -> Option<ExPolygon> {
        let layer = &self.layers[layer_pos];
        layer
            .regions
            .iter()
            .find(|region| region.object == object && region.region_id == region_id)
            .filter(|region| region.has_source)
            .map(|_region| square(object as i64 * SQUARE * 2))
    }
}

/// Verify the real stage against the independent model.
pub fn verify(case: &ScheduleCase) -> Result<(), String> {
    let plan = case.plan();
    let slices = case.slices();
    let schedule = build_emit_schedule(&plan);
    let geometry = execute_support_geometry(&plan, &slices)
        .map_err(|error| format!("stage failed: {error:?}"))?;
    verify_observation(case, &schedule, &geometry)
}

/// Verify an arbitrary schedule/geometry observation.
///
/// The deterministic controls mutate a real observation and feed it back here.
pub fn verify_observation(
    case: &ScheduleCase,
    schedule: &std::collections::HashMap<String, BTreeSet<u32>>,
    geometry: &slicer_ir::SupportGeometryIR,
) -> Result<(), String> {
    let expected_schedule = case.expected_schedule();
    for (object, expected) in &expected_schedule {
        let actual = schedule.get(object);
        if actual != Some(expected) {
            return Err(format!(
                "schedule for {object} is {actual:?}, expected {expected:?}"
            ));
        }
    }
    for object in schedule.keys() {
        if !expected_schedule.contains_key(object) {
            return Err(format!("schedule names unknown object {object}"));
        }
    }

    // --- Entries ----------------------------------------------------------
    let mut expected: BTreeMap<(u32, String, u64, Vec<(String, PaintValue)>), usize> =
        BTreeMap::new();
    for (layer_pos, layer) in case.layers.iter().enumerate() {
        for region in &layer.regions {
            let object = &case.objects[region.object];
            let scheduled = expected_schedule
                .get(object)
                .is_some_and(|set| set.contains(&layer.index));
            if scheduled {
                match case.source_polygons(layer_pos, region.object, region.region_id) {
                    Some(_) => {
                        expected.insert(
                            (
                                layer.index,
                                object.clone(),
                                region.region_id,
                                chain(region.chain),
                            ),
                            1,
                        );
                    }
                    None => {
                        expected.insert(
                            (layer.index, object.clone(), region.region_id, Vec::new()),
                            0,
                        );
                    }
                }
            }
        }
    }

    // Intermediate-model pass: for each object, every layer within
    // `support_top_z_distance` of that object's column top contributes the same
    // source geometry under the `u32::MAX` sentinel.
    let mut column_tops: BTreeMap<String, f32> = BTreeMap::new();
    for layer in &case.layers {
        let z = 0.2 * (layer.index as f32 + 1.0);
        for region in &layer.regions {
            let object = &case.objects[region.object];
            let entry = column_tops.entry(object.clone()).or_insert(0.0);
            if z > *entry {
                *entry = z;
            }
        }
    }
    for (layer_pos, layer) in case.layers.iter().enumerate() {
        let z = 0.2 * (layer.index as f32 + 1.0);
        for region in &layer.regions {
            let object = &case.objects[region.object];
            let top = column_tops.get(object).copied().unwrap_or(0.0);
            if (z - top).abs() > case.top_distance_mm {
                continue;
            }
            match case.source_polygons(layer_pos, region.object, region.region_id) {
                Some(_) => {
                    expected.insert(
                        (
                            INTERMEDIATE_SENTINEL,
                            object.clone(),
                            region.region_id,
                            chain(region.chain),
                        ),
                        1,
                    );
                }
                None => {
                    expected.insert(
                        (
                            INTERMEDIATE_SENTINEL,
                            object.clone(),
                            region.region_id,
                            Vec::new(),
                        ),
                        0,
                    );
                }
            }
        }
    }

    let actual_keys: BTreeMap<(u32, String, u64, Vec<(String, PaintValue)>), usize> = geometry
        .entries
        .iter()
        .map(|(key, polygons)| {
            (
                (
                    key.global_support_layer_index,
                    key.object_id.clone(),
                    key.region_id,
                    key.variant_chain.clone(),
                ),
                polygons.len(),
            )
        })
        .collect();
    if actual_keys != expected {
        return Err(format!(
            "entry set mismatch: actual {:?} expected {:?}",
            actual_keys.keys().collect::<Vec<_>>(),
            expected.keys().collect::<Vec<_>>()
        ));
    }

    // Uniform IR-wide fields.
    if geometry.support_layer_height_mm != case.targets_mm[0]
        || geometry.support_top_z_distance != case.top_distance_mm
    {
        return Err(String::from(
            "the IR-wide settings must come from the first region",
        ));
    }

    // A scheduled region with no source keeps a present key with no polygons.
    if actual_keys.values().any(|count| *count > 1) {
        return Err(String::from(
            "a single source region must not be duplicated by the collector",
        ));
    }

    Ok(())
}

/// Strategy for one generated scheduling scene.
pub fn strategy(max_layers: usize, max_objects: usize) -> impl Strategy<Value = ScheduleCase> {
    (
        1..=max_layers.clamp(1, 4),
        1..=max_objects.clamp(1, 3),
        proptest::collection::vec(0u8..=3u8, 1..=4),
        proptest::collection::vec(0u8..=3u8, 1..=4),
        proptest::collection::vec(0u8..=1u8, 1..=4),
    )
        .prop_map(|(layer_count, object_count, targets, heights, chains)| {
            let objects: Vec<String> = (0..object_count).map(|i| format!("obj_{i}")).collect();
            // Dyadic heights and targets keep the threshold comparison exact.
            let height_mm = match heights[0] % 3 {
                0 => 0.2,
                1 => 0.25,
                _ => 0.1,
            };
            let targets_mm: Vec<f32> = (0..object_count)
                .map(|object| match targets[object % targets.len()] {
                    0 => 0.0,
                    1 => height_mm,
                    2 => height_mm * 2.0,
                    _ => height_mm * 3.0,
                })
                .collect();
            let mut layers = Vec::new();
            for layer in 0..layer_count {
                let index = layer as u32 * 2; // unique, never the sentinel
                let mut regions = Vec::new();
                for object in 0..object_count {
                    regions.push(ScheduleRegion {
                        object,
                        region_id: 0,
                        has_source: chains[(layer + object) % chains.len()] == 1,
                        chain: (layer + object) % 2,
                    });
                }
                layers.push(ScheduleLayer { index, regions });
            }
            ScheduleCase {
                objects,
                layers,
                targets_mm,
                height_mm,
                top_distance_mm: if targets[0] == 0 { 0.0 } else { 0.2 },
            }
        })
}

/// Registry entry point for `H-SUPPORT-SCHEDULE`.
pub fn run_property() -> Result<(), String> {
    let run = PropertyRun::new(
        "H-SUPPORT-SCHEDULE",
        "proptest_support_schedule_and_sources",
    )
    .map_err(|error| error.to_string())?;
    let max_layers = usize::try_from(run.bound("max_layers").map_err(|e| e.to_string())?)
        .map_err(|_| String::from("max_layers does not fit a usize"))?;
    let max_objects = usize::try_from(run.bound("max_objects").map_err(|e| e.to_string())?)
        .map_err(|_| String::from("max_objects does not fit a usize"))?;
    let max_regions = run.bound("max_regions").map_err(|e| e.to_string())?;
    let max_abs_units = run.bound("max_abs_units").map_err(|e| e.to_string())?;
    if max_regions < 1 || max_abs_units < u64::try_from(SQUARE * 4).unwrap_or(u64::MAX) {
        return Err(String::from("max_regions and max_abs_units are too small"));
    }

    // An empty plan yields no entries at all.
    let empty = ScheduleCase {
        objects: vec!["obj_0".to_owned()],
        layers: Vec::new(),
        targets_mm: vec![0.0],
        height_mm: 0.2,
        top_distance_mm: 0.0,
    };
    if !build_emit_schedule(&empty.plan()).is_empty() {
        return Err(String::from("an empty plan must schedule nothing"));
    }
    let geometry = execute_support_geometry(&empty.plan(), &empty.slices())
        .map_err(|error| format!("empty plan failed: {error:?}"))?;
    if !geometry.entries.is_empty() {
        return Err(String::from("an empty plan must publish no entries"));
    }

    let generated = strategy(max_layers, max_objects);
    run.check(&generated, |case: ScheduleCase, _guard: &mut CaseGuard| {
        verify(&case).map_err(TestCaseError::fail)
    })
    .map_err(|error| error.to_string())
}
