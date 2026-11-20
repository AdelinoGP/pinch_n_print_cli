//! Independent model for `H-SUPPORT-ANALYSIS`.
//!
//! `commit_support_analysis_builtin`
//! (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`) publishes
//! the host-owned analysis that every support family consumes. The model here
//! re-derives, from the declared slice regions alone:
//!
//! * `model_occupancy` — one entry per nonempty committed region, keyed by the
//!   full `(layer, object, region_id, ordered chain)` identity and carrying that
//!   region's polygons verbatim;
//! * each object's full XY bounds, and therefore
//!   `baseline_feasible_envelope`, which holds exactly one plate rectangle per
//!   object with a nonempty bound;
//! * `termination_surfaces` — for every occupancy key at that object's highest
//!   active layer, the top layer's polygons plus the object's plate rectangle;
//! * `support_territory_clearance_mm` — the resolved positive support line
//!   width in the literal mode the property declares.

use std::collections::{BTreeMap, BTreeSet};

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use slicer_ir::resolved_config::ResolvedFloatOrPercent;
use slicer_ir::{
    ActiveRegion, ExPolygon, GlobalLayer, LayerPlanIR, MeshIR, PaintValue, Point2, Polygon,
    ResolvedConfig, SemVer, SliceIR, SlicedRegion, SupportAnalysisIR,
};
use slicer_runtime::builtins::support_analysis_producer::commit_support_analysis_builtin;
use slicer_runtime::Blackboard;
use slicer_sdk::test_support::property_testing::{CaseGuard, PropertyRun};
use slicer_wasm_host::support_territory::SUPPORT_TERRITORY_CLEARANCE_KEY;

const MM: i64 = 10_000;

/// One declared region of one object on one layer.
#[derive(Debug, Clone)]
pub struct AnalysisRegion {
    /// Region id.
    pub region_id: u64,
    /// Chain spelling index (0 = empty, 1 = `material`/`ToolIndex(0)`).
    pub chain: usize,
    /// Rectangle `[x0, y0, x1, y1]` in millimetres.
    pub rect: [i64; 4],
}

/// One committed layer.
#[derive(Debug, Clone)]
pub struct AnalysisLayer {
    /// Global layer index.
    pub index: u32,
    /// Regions committed on this layer.
    pub regions: Vec<(usize, AnalysisRegion)>,
}

/// One generated analysis scene.
#[derive(Debug, Clone)]
pub struct AnalysisCase {
    /// Object ids.
    pub objects: Vec<String>,
    /// Layers in increasing `z`.
    pub layers: Vec<AnalysisLayer>,
    /// Whether support is enabled.
    pub enabled: bool,
    /// Literal support line width in millimetres.
    pub support_line_width_mm: f64,
    /// When set, a surface classification whose single overhang footprint sits
    /// far from every declared region, so that object's regions need no support.
    pub no_support_object: Option<String>,
}

/// The chain spelling for an index.
pub fn chain(spelling: usize) -> Vec<(String, PaintValue)> {
    match spelling % 2 {
        0 => Vec::new(),
        _ => vec![("material".to_owned(), PaintValue::ToolIndex(0))],
    }
}

fn rect_polygon(rect: [i64; 4]) -> ExPolygon {
    let [x0, y0, x1, y1] = rect.map(|value| value * MM);
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: x0, y: y0 },
                Point2 { x: x1, y: y0 },
                Point2 { x: x1, y: y1 },
                Point2 { x: x0, y: y1 },
            ],
        },
        holes: Vec::new(),
    }
}

impl AnalysisCase {
    fn z(layer_position: usize) -> f32 {
        0.2 * (layer_position as f32 + 1.0)
    }

    /// The layer plan.
    pub fn plan(&self) -> LayerPlanIR {
        LayerPlanIR {
            schema_version: SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            global_layers: self
                .layers
                .iter()
                .enumerate()
                .map(|(position, layer)| GlobalLayer {
                    index: layer.index,
                    z: Self::z(position),
                    active_regions: layer
                        .regions
                        .iter()
                        .map(|(object, region)| ActiveRegion {
                            object_id: self.objects[*object].clone(),
                            region_id: region.region_id,
                            resolved_config: ResolvedConfig::default(),
                            effective_layer_height: 0.2,
                            ..ActiveRegion::default()
                        })
                        .collect(),
                    ..GlobalLayer::default()
                })
                .collect(),
            object_participation: std::collections::HashMap::new(),
        }
    }

    /// The committed slices.
    pub fn slices(&self) -> Vec<SliceIR> {
        self.layers
            .iter()
            .enumerate()
            .map(|(position, layer)| SliceIR {
                global_layer_index: layer.index,
                z: Self::z(position),
                regions: layer
                    .regions
                    .iter()
                    .map(|(object, region)| SlicedRegion {
                        object_id: self.objects[*object].clone(),
                        region_id: region.region_id,
                        polygons: vec![rect_polygon(region.rect)],
                        variant_chain: chain(region.chain),
                        ..SlicedRegion::default()
                    })
                    .collect(),
                ..SliceIR::default()
            })
            .collect()
    }

    /// The optional surface classification used to reach the no-candidate mode.
    pub fn classification(&self) -> Option<slicer_ir::SurfaceClassificationIR> {
        let object = self.no_support_object.as_ref()?;
        let far_away = ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 {
                        x: 900 * MM,
                        y: 900 * MM,
                    },
                    Point2 {
                        x: 901 * MM,
                        y: 900 * MM,
                    },
                    Point2 {
                        x: 901 * MM,
                        y: 901 * MM,
                    },
                    Point2 {
                        x: 900 * MM,
                        y: 901 * MM,
                    },
                ],
            },
            holes: Vec::new(),
        };
        Some(slicer_ir::SurfaceClassificationIR {
            per_object: std::collections::HashMap::from([(
                object.clone(),
                slicer_ir::ObjectSurfaceData {
                    // exhaustive: the classification fixture pins every field.
                    overhang_regions: vec![slicer_ir::OverhangRegion {
                        id: 0,
                        facet_indices: Vec::new(),
                        max_angle_deg: 45.0,
                        needs_support: true,
                        xy_footprint: vec![far_away],
                    }],
                    ..slicer_ir::ObjectSurfaceData::default()
                },
            )]),
            ..slicer_ir::SurfaceClassificationIR::default()
        })
    }

    /// The declared resolved config.
    pub fn config(&self) -> ResolvedConfig {
        ResolvedConfig {
            support_enabled: self.enabled,
            support_line_width: ResolvedFloatOrPercent {
                value: self.support_line_width_mm,
                is_percent: false,
            },
            ..ResolvedConfig::default()
        }
    }

    /// Expected occupancy: every nonempty region, keyed by full identity.
    fn expected_occupancy(&self) -> BTreeMap<(u32, String, u64, usize), ExPolygon> {
        let mut occupancy = BTreeMap::new();
        for layer in &self.layers {
            for (object, region) in &layer.regions {
                occupancy.insert(
                    (
                        layer.index,
                        self.objects[*object].clone(),
                        region.region_id,
                        region.chain,
                    ),
                    rect_polygon(region.rect),
                );
            }
        }
        occupancy
    }

    /// Per-object full XY bounds in canonical units.
    fn object_bounds(&self) -> BTreeMap<String, (i64, i64, i64, i64)> {
        let mut bounds: BTreeMap<String, (i64, i64, i64, i64)> = BTreeMap::new();
        for layer in &self.layers {
            for (object, region) in &layer.regions {
                let [x0, y0, x1, y1] = region.rect.map(|value| value * MM);
                let entry = bounds.entry(self.objects[*object].clone()).or_insert((
                    i64::MAX,
                    i64::MIN,
                    i64::MAX,
                    i64::MIN,
                ));
                entry.0 = entry.0.min(x0);
                entry.1 = entry.1.max(x1);
                entry.2 = entry.2.min(y0);
                entry.3 = entry.3.max(y1);
            }
        }
        bounds
    }
}

/// Commit the case and return the published analysis.
pub fn committed(case: &AnalysisCase) -> Result<SupportAnalysisIR, String> {
    let mut blackboard = Blackboard::new(std::sync::Arc::new(MeshIR::default()), 1);
    blackboard
        .commit_layer_plan(std::sync::Arc::new(case.plan()))
        .map_err(|error| format!("layer plan commit failed: {error:?}"))?;
    if let Some(classification) = case.classification() {
        blackboard
            .commit_surface_classification(std::sync::Arc::new(classification))
            .map_err(|error| format!("classification commit failed: {error:?}"))?;
    }
    blackboard
        .commit_slice_ir(std::sync::Arc::new(case.slices()))
        .map_err(|error| format!("slice commit failed: {error:?}"))?;
    commit_support_analysis_builtin(&mut blackboard, &case.config())
        .map_err(|error| format!("analysis commit failed: {error:?}"))?;
    blackboard
        .support_analysis()
        .map(|analysis| analysis.as_ref().clone())
        .ok_or_else(|| String::from("the builtin must commit an analysis"))
}

/// Verify the committed analysis against the model.
pub fn verify(case: &AnalysisCase) -> Result<(), String> {
    let mut blackboard = Blackboard::new(std::sync::Arc::new(MeshIR::default()), 1);
    blackboard
        .commit_layer_plan(std::sync::Arc::new(case.plan()))
        .map_err(|error| format!("layer plan commit failed: {error:?}"))?;
    if let Some(classification) = case.classification() {
        blackboard
            .commit_surface_classification(std::sync::Arc::new(classification))
            .map_err(|error| format!("classification commit failed: {error:?}"))?;
    }
    blackboard
        .commit_slice_ir(std::sync::Arc::new(case.slices()))
        .map_err(|error| format!("slice commit failed: {error:?}"))?;
    commit_support_analysis_builtin(&mut blackboard, &case.config())
        .map_err(|error| format!("analysis commit failed: {error:?}"))?;
    let analysis: &SupportAnalysisIR = blackboard
        .support_analysis()
        .ok_or_else(|| String::from("the builtin must commit an analysis"))?;
    verify_observation(case, analysis)
}

/// Verify an arbitrary committed observation against the model.
///
/// The deterministic controls mutate a real analysis and feed it back here.
pub fn verify_observation(case: &AnalysisCase, analysis: &SupportAnalysisIR) -> Result<(), String> {
    // Settings are always published.
    if analysis.shared_settings.get("support_enabled") != Some(&case.enabled.to_string()) {
        return Err(String::from("support_enabled setting mismatch"));
    }

    if !case.enabled {
        if !analysis.model_occupancy.is_empty()
            || !analysis.baseline_feasible_envelope.is_empty()
            || !analysis.candidates.is_empty()
        {
            return Err(String::from(
                "a disabled analysis must publish no occupancy, envelope or candidates",
            ));
        }
        return Ok(());
    }

    // --- Occupancy --------------------------------------------------------
    let expected = case.expected_occupancy();
    if analysis.model_occupancy.len() != expected.len() {
        return Err(format!(
            "occupancy holds {} entries, expected {}",
            analysis.model_occupancy.len(),
            expected.len()
        ));
    }
    for ((layer, object, region_id, chain_index), polygon) in &expected {
        let key = slicer_ir::SupportGeometryKey {
            global_support_layer_index: *layer,
            object_id: object.clone(),
            region_id: *region_id,
            variant_chain: chain(*chain_index),
        };
        let observed = analysis
            .model_occupancy
            .get(&key)
            .ok_or_else(|| format!("occupancy is missing {key:?}"))?;
        if observed != &vec![polygon.clone()] {
            return Err(format!(
                "occupancy for {key:?} differs from the committed region"
            ));
        }
    }

    // --- Envelope ---------------------------------------------------------
    let bounds = case.object_bounds();
    let mut expected_plates: BTreeSet<(i64, i64, i64, i64)> = BTreeSet::new();
    for (min_x, max_x, min_y, max_y) in bounds.values() {
        expected_plates.insert((*min_x, *max_x, *min_y, *max_y));
    }
    let mut observed_plates: BTreeSet<(i64, i64, i64, i64)> = BTreeSet::new();
    for plate in &analysis.baseline_feasible_envelope {
        let points = &plate.contour.points;
        if points.len() != 4 {
            return Err(String::from("the envelope plate is not a rectangle"));
        }
        let xs: Vec<i64> = points.iter().map(|point| point.x).collect();
        let ys: Vec<i64> = points.iter().map(|point| point.y).collect();
        observed_plates.insert((
            *xs.iter().min().unwrap_or(&0),
            *xs.iter().max().unwrap_or(&0),
            *ys.iter().min().unwrap_or(&0),
            *ys.iter().max().unwrap_or(&0),
        ));
    }
    if observed_plates != expected_plates {
        return Err(format!(
            "envelope {observed_plates:?} != expected {expected_plates:?}"
        ));
    }

    // --- Termination ------------------------------------------------------
    let mut top_layers: BTreeMap<String, u32> = BTreeMap::new();
    for layer in &case.layers {
        for (object, _) in &layer.regions {
            let entry = top_layers
                .entry(case.objects[*object].clone())
                .or_insert(layer.index);
            if layer.index > *entry {
                *entry = layer.index;
            }
        }
    }
    for (layer, object, region_id, chain_index) in expected.keys() {
        let is_top = top_layers.get(object) == Some(layer);
        let key = slicer_ir::SupportGeometryKey {
            global_support_layer_index: *layer,
            object_id: object.clone(),
            region_id: *region_id,
            variant_chain: chain(*chain_index),
        };
        let termination = analysis.termination_surfaces.get(&key);
        if is_top {
            let termination = termination
                .ok_or_else(|| format!("the top-layer key {key:?} needs a termination surface"))?;
            // The top layer's footprint plus the object's plate rectangle.
            if termination.len() != 2 {
                return Err(String::from(
                    "a top termination must hold the footprint and the plate",
                ));
            }
            let expected_footprint = expected
                .get(&(*layer, object.clone(), *region_id, *chain_index))
                .cloned()
                .ok_or_else(|| String::from("the top key has no declared footprint"))?;
            if !termination
                .iter()
                .any(|polygon| polygon == &expected_footprint)
            {
                return Err(format!(
                    "the top termination for {key:?} must contain its own footprint"
                ));
            }
            let plate = bounds
                .get(object)
                .and_then(|(min_x, max_x, min_y, max_y)| {
                    (min_x < max_x && min_y < max_y).then_some((*min_x, *max_x, *min_y, *max_y))
                })
                .ok_or_else(|| String::from("the object must have a plate rectangle"))?;
            let has_plate = termination.iter().any(|polygon| {
                let xs: Vec<i64> = polygon.contour.points.iter().map(|p| p.x).collect();
                let ys: Vec<i64> = polygon.contour.points.iter().map(|p| p.y).collect();
                let observed = (
                    xs.iter().copied().min().unwrap_or(0),
                    xs.iter().copied().max().unwrap_or(0),
                    ys.iter().copied().min().unwrap_or(0),
                    ys.iter().copied().max().unwrap_or(0),
                );
                observed == plate
            });
            if !has_plate {
                return Err(format!(
                    "the top termination for {key:?} must contain the object plate"
                ));
            }
        } else if termination.is_some() {
            return Err(format!("non-top key {key:?} must not terminate"));
        }
    }

    // --- Candidates -------------------------------------------------------
    // `derive_needs_support` returns true when the classification carries no
    // entry for an object, so every nonempty region becomes a candidate. When a
    // classification places one object's only overhang footprint far from its
    // regions, that object contributes none.
    // A candidate appears only when the region is not fully covered by that
    // SAME object's layer below, because contact detection diffs a region
    // against its own object's lower union. One region per object per layer
    // keeps the coverage test exact for axis-aligned rectangles.
    let expected_candidates: BTreeSet<(String, u64, u32, usize)> = expected
        .keys()
        .filter(|(layer, object, region_id, chain_index)| {
            if case.no_support_object.as_ref() == Some(object) {
                return false;
            }
            let Some(position) = case
                .layers
                .iter()
                .position(|candidate| candidate.index == *layer)
            else {
                return false;
            };
            if position == 0 {
                return false;
            }
            let lower_position = position - 1;
            let own = case.layers[position]
                .regions
                .iter()
                .find(|(_, region)| region.region_id == *region_id)
                .map(|(_, region)| region.rect);
            let lower = case.layers[lower_position]
                .regions
                .iter()
                .filter(|(object_index, _)| case.objects[*object_index] == *object)
                .map(|(_, region)| region.rect)
                .collect::<Vec<_>>();
            let Some(own) = own else { return false };
            let covered = lower.iter().any(|rect| {
                rect[0] <= own[0] && rect[1] <= own[1] && rect[2] >= own[2] && rect[3] >= own[3]
            });
            let _ = chain_index;
            !covered
        })
        .map(|(layer, object, region_id, chain_index)| {
            (object.clone(), *region_id, *layer, *chain_index)
        })
        .collect();
    if analysis.candidates.len() != expected_candidates.len() {
        return Err(format!(
            "candidate count {} != expected {}",
            analysis.candidates.len(),
            expected_candidates.len()
        ));
    }
    for candidate in &analysis.candidates {
        let chain_index = if candidate.source.variant_chain == chain(1) {
            1
        } else if candidate.source.variant_chain.is_empty() {
            0
        } else {
            return Err(format!(
                "candidate {} carries an undeclared chain {:?}",
                candidate.id, candidate.source.variant_chain
            ));
        };
        let identity = (
            candidate.source.object_id.clone(),
            candidate.source.region_id,
            candidate.source.global_layer_index,
            chain_index,
        );
        if !expected_candidates.contains(&identity) {
            return Err(format!("unexpected candidate source {identity:?}"));
        }
        // The geometry must belong to ITS OWN source region, never a
        // neighbour's: every point stays inside that region's rectangle.
        let source_rect = case
            .layers
            .iter()
            .find(|layer| layer.index == candidate.source.global_layer_index)
            .and_then(|layer| {
                layer
                    .regions
                    .iter()
                    .find(|(object_index, region)| {
                        case.objects[*object_index] == candidate.source.object_id
                            && region.region_id == candidate.source.region_id
                    })
                    .map(|(_, region)| region.rect.map(|value| value * MM))
            })
            .ok_or_else(|| format!("candidate {} has an unknown source", candidate.id))?;
        for polygon in &candidate.geometry {
            for point in &polygon.contour.points {
                if point.x < source_rect[0]
                    || point.x > source_rect[2]
                    || point.y < source_rect[1]
                    || point.y > source_rect[3]
                {
                    return Err(format!(
                        "candidate {} geometry leaves its own source region at {point:?}",
                        candidate.id
                    ));
                }
                // The contact is `upper \ lower`, so no candidate vertex may
                // sit strictly inside this object's layer below.
                if let Some(position) = case
                    .layers
                    .iter()
                    .position(|l| l.index == candidate.source.global_layer_index)
                {
                    if position > 0 {
                        for (object_index, lower) in &case.layers[position - 1].regions {
                            if case.objects[*object_index] != candidate.source.object_id {
                                continue;
                            }
                            let [x0, y0, x1, y1] = lower.rect.map(|value| value * MM);
                            if point.x > x0 && point.x < x1 && point.y > y0 && point.y < y1 {
                                return Err(format!(
                                    "candidate {} has a vertex inside the supported lower region",
                                    candidate.id
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    // --- Clearance --------------------------------------------------------
    let clearance = analysis
        .shared_settings
        .get(SUPPORT_TERRITORY_CLEARANCE_KEY)
        .ok_or_else(|| String::from("the clearance setting must be published"))?;
    let observed: f32 = clearance
        .parse()
        .map_err(|_| format!("the clearance {clearance:?} is not a number"))?;
    let expected = case.support_line_width_mm as f32;
    if (observed - expected).abs() > f32::EPSILON {
        return Err(format!(
            "clearance {observed} != the literal support line width {expected}"
        ));
    }
    if observed <= 0.0 {
        return Err(String::from("the clearance must be positive"));
    }

    Ok(())
}

/// The percent-mode fallback formula, checked separately from the property.
pub fn percent_clearance() -> Result<(), String> {
    let case = AnalysisCase {
        objects: vec!["obj_0".to_owned()],
        layers: vec![AnalysisLayer {
            index: 0,
            regions: vec![(
                0,
                AnalysisRegion {
                    region_id: 0,
                    chain: 0,
                    rect: [0, 0, 2, 2],
                },
            )],
        }],
        enabled: true,
        support_line_width_mm: 50.0,
        no_support_object: None,
    };
    let mut config = case.config();
    config.support_line_width = ResolvedFloatOrPercent {
        value: 50.0,
        is_percent: true,
    };
    config.line_width = 0.8;
    let mut blackboard = Blackboard::new(std::sync::Arc::new(MeshIR::default()), 1);
    blackboard
        .commit_layer_plan(std::sync::Arc::new(case.plan()))
        .map_err(|error| format!("layer plan commit failed: {error:?}"))?;
    blackboard
        .commit_slice_ir(std::sync::Arc::new(case.slices()))
        .map_err(|error| format!("slice commit failed: {error:?}"))?;
    commit_support_analysis_builtin(&mut blackboard, &config)
        .map_err(|error| format!("analysis commit failed: {error:?}"))?;
    let analysis = blackboard
        .support_analysis()
        .ok_or_else(|| String::from("the builtin must commit an analysis"))?;
    let clearance: f32 = analysis
        .shared_settings
        .get(SUPPORT_TERRITORY_CLEARANCE_KEY)
        .ok_or_else(|| String::from("clearance missing"))?
        .parse()
        .map_err(|_| String::from("clearance is not a number"))?;
    // 50% of the 0.8 mm fallback line width.
    if (clearance - 0.4).abs() > 1e-6 {
        return Err(format!("percent clearance {clearance} != 0.4"));
    }
    Ok(())
}

/// Strategy for one generated analysis scene.
pub fn strategy(max_layers: usize, max_objects: usize) -> impl Strategy<Value = AnalysisCase> {
    (
        1..=max_layers.clamp(1, 3),
        1..=max_objects.clamp(1, 2),
        proptest::collection::vec(
            (0i64..4, 0i64..4, 1i64..2, 1i64..2, 0usize..2, any::<bool>()),
            1..=6,
        ),
        any::<bool>(),
    )
        .prop_map(|(layer_count, object_count, specs, enabled)| {
            let objects: Vec<String> = (0..object_count).map(|i| format!("obj_{i}")).collect();
            let mut layers = Vec::new();
            for position in 0..layer_count {
                let index = position as u32;
                let mut regions = Vec::new();
                for (object, (x, y, w, h, chain_index, present)) in specs.iter().enumerate() {
                    if !*present || object >= object_count {
                        continue;
                    }
                    regions.push((
                        object,
                        AnalysisRegion {
                            region_id: (position as u64) * 10 + object as u64,
                            chain: *chain_index,
                            rect: [*x, *y, x + w, y + h],
                        },
                    ));
                }
                // Every layer carries at least one region so the top layer is
                // well defined.
                if regions.is_empty() {
                    regions.push((
                        0,
                        AnalysisRegion {
                            region_id: position as u64,
                            chain: 0,
                            rect: [0, 0, 2, 2],
                        },
                    ));
                }
                layers.push(AnalysisLayer { index, regions });
            }
            AnalysisCase {
                objects,
                layers,
                enabled,
                support_line_width_mm: 0.5,
                no_support_object: None,
            }
        })
}

/// Registry entry point for `H-SUPPORT-ANALYSIS`.
pub fn run_property() -> Result<(), String> {
    let run = PropertyRun::new(
        "H-SUPPORT-ANALYSIS",
        "proptest_support_analysis_layered_scenes",
    )
    .map_err(|error| error.to_string())?;
    let max_layers = usize::try_from(run.bound("max_layers").map_err(|e| e.to_string())?)
        .map_err(|_| String::from("max_layers does not fit a usize"))?;
    let max_objects = usize::try_from(run.bound("max_objects").map_err(|e| e.to_string())?)
        .map_err(|_| String::from("max_objects does not fit a usize"))?;
    for bound in ["max_regions", "max_abs_units", "max_cells"] {
        run.bound(bound).map_err(|error| error.to_string())?;
    }

    percent_clearance()?;

    // Enabled witness: one object, one rectangle, one layer.
    verify(&AnalysisCase {
        objects: vec!["obj_0".to_owned()],
        layers: vec![AnalysisLayer {
            index: 0,
            regions: vec![(
                0,
                AnalysisRegion {
                    region_id: 0,
                    chain: 0,
                    rect: [0, 0, 2, 2],
                },
            )],
        }],
        enabled: true,
        support_line_width_mm: 0.5,
        no_support_object: None,
    })?;

    // Disabled witness: settings only.
    verify(&AnalysisCase {
        objects: vec!["obj_0".to_owned()],
        layers: vec![AnalysisLayer {
            index: 0,
            regions: vec![(
                0,
                AnalysisRegion {
                    region_id: 0,
                    chain: 0,
                    rect: [0, 0, 2, 2],
                },
            )],
        }],
        enabled: false,
        support_line_width_mm: 0.5,
        no_support_object: None,
    })?;

    // Same-object lower vs a misleading foreign object: obj_1's upper overhangs
    // its OWN lower by 1 mm, while obj_0's lower would cover it entirely. A
    // producer that borrowed another object's lower footprint would suppress the
    // candidate and fail this witness.
    verify(&AnalysisCase {
        objects: vec!["obj_0".to_owned(), "obj_1".to_owned()],
        layers: vec![
            AnalysisLayer {
                index: 0,
                regions: vec![
                    (
                        0,
                        AnalysisRegion {
                            region_id: 0,
                            chain: 0,
                            rect: [-10, -10, 10, 10],
                        },
                    ),
                    (
                        1,
                        AnalysisRegion {
                            region_id: 1,
                            chain: 0,
                            rect: [0, 0, 10, 10],
                        },
                    ),
                ],
            },
            AnalysisLayer {
                index: 1,
                regions: vec![(
                    1,
                    AnalysisRegion {
                        region_id: 1,
                        chain: 1,
                        rect: [0, 0, 11, 10],
                    },
                )],
            },
        ],
        enabled: true,
        support_line_width_mm: 0.5,
        no_support_object: None,
    })?;

    // The no-candidate mode: an upper region fully inside its own lower region.
    verify(&AnalysisCase {
        objects: vec!["obj_0".to_owned()],
        layers: vec![
            AnalysisLayer {
                index: 0,
                regions: vec![(
                    0,
                    AnalysisRegion {
                        region_id: 0,
                        chain: 0,
                        rect: [0, 0, 10, 10],
                    },
                )],
            },
            AnalysisLayer {
                index: 1,
                regions: vec![(
                    0,
                    AnalysisRegion {
                        region_id: 0,
                        chain: 0,
                        rect: [2, 2, 8, 8],
                    },
                )],
            },
        ],
        enabled: true,
        support_line_width_mm: 0.5,
        no_support_object: None,
    })?;

    // The surface-classification no-support mode: an object whose only overhang
    // footprint sits far from its regions contributes no candidate.
    verify(&AnalysisCase {
        objects: vec!["obj_0".to_owned(), "obj_1".to_owned()],
        layers: vec![
            AnalysisLayer {
                index: 0,
                regions: vec![(
                    0,
                    AnalysisRegion {
                        region_id: 0,
                        chain: 0,
                        rect: [0, 0, 4, 4],
                    },
                )],
            },
            AnalysisLayer {
                index: 1,
                regions: vec![
                    (
                        0,
                        AnalysisRegion {
                            region_id: 0,
                            chain: 0,
                            rect: [0, 0, 5, 4],
                        },
                    ),
                    (
                        1,
                        AnalysisRegion {
                            region_id: 0,
                            chain: 0,
                            rect: [6, 6, 10, 10],
                        },
                    ),
                ],
            },
        ],
        enabled: true,
        support_line_width_mm: 0.5,
        no_support_object: Some("obj_0".to_owned()),
    })?;

    let generated = strategy(max_layers, max_objects);
    run.check(&generated, |case: AnalysisCase, _guard: &mut CaseGuard| {
        verify(&case).map_err(TestCaseError::fail)
    })
    .map_err(|error| error.to_string())
}
