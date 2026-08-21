//! Host-owned, strategy-neutral support analysis.

use std::collections::BTreeMap;
use std::sync::Arc;

use rayon::prelude::*;
use slicer_core::algos::overhang_annotation::{detect_support_contacts, SupportContactParams};
use slicer_core::polygon_ops::union_ex;
use slicer_ir::mm_to_units;
use slicer_ir::slice_ir::{
    ExPolygon, Point2, Polygon, RegionKey, SupportAnalysisIR, SupportCandidate,
    SupportCandidateSource, SupportGeometryKey, SupportType,
};
use slicer_ir::{ConfigValue, ResolvedConfig};
use slicer_scheduler::execution_plan::{
    select_support_family, SUPPORT_FAMILY_CONFIG_KEY, SUPPORT_GENERATOR_CONFIG_KEY,
};

use crate::blackboard::Blackboard;

/// Build conservative candidates without propagating support bodies.
///
/// Candidates are **support contacts**, not model cross-sections. Each is the
/// angle-thresholded overhang region produced by
/// [`detect_support_contacts`](slicer_core::algos::overhang_annotation::detect_support_contacts),
/// mirroring canonical `detect_overhangs` (`SupportMaterial.cpp`): a contact
/// appears once, at the overhang's own Z, and geometry with no overhang yields
/// no candidates at all.
///
/// Before packet 224 this stage emitted one candidate per non-empty region per
/// layer, carrying the full model cross-section — no overhang detection of any
/// kind. Downstream planners were left to invent their own contact detection,
/// and any planner that trusted the stream necessarily produced support at
/// every layer of the model. Do not reintroduce an unfiltered candidate stream.
pub fn commit_support_analysis_builtin(
    blackboard: &mut Blackboard,
    config: &ResolvedConfig,
) -> Result<(), crate::BlackboardError> {
    let enable_support = config.support_enabled;
    // Read the typed field directly. `support_threshold_angle` is CLI-bound, so
    // `resolve_*` routes it into this field and never into `extensions` — an
    // extensions lookup here silently ignored every configured value. The macro
    // line in `slicer_ir::resolved_config` owns the default; there is no
    // host-side fallback constant and no `support_angle` fallback (that key is
    // canonical's support *pattern rotation*, not an overhang threshold).
    let threshold_angle_deg = config.support_threshold_angle;
    let mut ir = SupportAnalysisIR::default();
    ir.shared_settings
        .insert("support_enabled".to_string(), enable_support.to_string());
    ir.shared_settings.insert(
        "support_threshold_angle_deg".to_string(),
        threshold_angle_deg.to_string(),
    );
    if enable_support {
        // Unit fixtures may not run region mapping, so preserve their deterministic
        // traditional fallback while production runs consume the committed map.
        let region_map = blackboard.region_map().cloned();
        if let (Some(slices), Some(plan)) = (blackboard.slice_ir(), blackboard.layer_plan()) {
            let mut id = 0_u64;
            let mut object_bounds: BTreeMap<String, (i64, i64, i64, i64)> = BTreeMap::new();
            let mut object_tops: BTreeMap<String, (u32, Vec<ExPolygon>)> = BTreeMap::new();
            // Layer-major contact detection state. Canonical `detect_overhangs`
            // reads `object.layers()[layer_id - 1]->lslices` -- the union of
            // *all* regions of the layer below, at object level -- and diffs
            // each of the current layer's regions against it. So we accumulate
            // (a) the per-(object, layer) polygon set that becomes that union
            // and (b) one work item per (layer, object, region).
            let mut object_layer_polygons: BTreeMap<(String, u32), Vec<ExPolygon>> =
                BTreeMap::new();
            let mut contact_work: Vec<(u32, String, u64, Vec<ExPolygon>)> = Vec::new();
            for slice in slices.iter() {
                for region in &slice.regions {
                    if region.polygons.is_empty() {
                        continue;
                    }
                    let key = SupportGeometryKey {
                        global_support_layer_index: slice.global_layer_index,
                        object_id: region.object_id.clone(),
                        region_id: region.region_id,
                    };
                    ir.model_occupancy.insert(key, region.polygons.clone());
                    let bounds = object_bounds.entry(region.object_id.clone()).or_insert((
                        i64::MAX,
                        i64::MIN,
                        i64::MAX,
                        i64::MIN,
                    ));
                    for polygon in &region.polygons {
                        for point in &polygon.contour.points {
                            bounds.0 = bounds.0.min(point.x);
                            bounds.1 = bounds.1.max(point.x);
                            bounds.2 = bounds.2.min(point.y);
                            bounds.3 = bounds.3.max(point.y);
                        }
                    }
                    let top = object_tops
                        .entry(region.object_id.clone())
                        .or_insert((slice.global_layer_index, region.polygons.clone()));
                    if slice.global_layer_index > top.0 {
                        *top = (slice.global_layer_index, region.polygons.clone());
                    }
                    // Feed the layer-major detection state: this region's
                    // polygons join its object's layer union, and the region
                    // itself becomes one unit of contact-detection work.
                    object_layer_polygons
                        .entry((region.object_id.clone(), slice.global_layer_index))
                        .or_default()
                        .extend(region.polygons.iter().cloned());
                    contact_work.push((
                        slice.global_layer_index,
                        region.object_id.clone(),
                        region.region_id,
                        region.polygons.clone(),
                    ));
                }
            }

            // Angle-thresholded contact detection, layer-major then
            // region-major, mirroring canonical `detect_overhangs`
            // (`SupportMaterial.cpp`). The lower-layer set is the object's
            // whole layer below, unioned once, and every region of the layer
            // above is diffed against that same union. Keying the lower layer
            // per-region instead (the pre-parity shape) made a region that
            // first appears at layer `k` while sitting squarely on a *different*
            // region below emit its entire cross-section as a contact --
            // spurious full-area support on every multi-region object.
            let object_layer_union: BTreeMap<(String, u32), Vec<ExPolygon>> = object_layer_polygons
                .into_iter()
                .map(|(key, polygons)| (key, union_ex(&polygons)))
                .collect();
            let base_params = resolve_contact_params(config, threshold_angle_deg);
            // Deterministic order regardless of `SliceIR` ordering; the
            // parallel pass below reads only shared immutable state and
            // `rayon`'s `collect` into a `Vec` preserves this order, so the
            // committed candidate stream is byte-stable (the same
            // order-independence property the previous per-series `par_iter`
            // had).
            contact_work.sort_by(|a, b| (a.0, &a.1, a.2).cmp(&(b.0, &b.1, b.2)));
            let contacts: Vec<(u32, String, u64, Vec<ExPolygon>)> = contact_work
                .par_iter()
                .filter_map(|(layer_index, object_id, region_id, polygons)| {
                    // Layer 0 rests on the bed and has no layer below it.
                    let lower_index = layer_index.checked_sub(1)?;
                    let empty: Vec<ExPolygon> = Vec::new();
                    let lower = object_layer_union
                        .get(&(object_id.clone(), lower_index))
                        .unwrap_or(&empty);
                    let params = SupportContactParams {
                        // Canonical scales the offset by the *lower* layer's
                        // height.
                        lower_layer_height_mm: layer_height_mm(&plan.global_layers, lower_index),
                        ..base_params
                    };
                    // Support blockers are not available to this stage; see
                    // `detect_support_contacts`' "Not modelled" section.
                    let geometry = detect_support_contacts(polygons, lower, &[], &params);
                    if geometry.is_empty() {
                        return None;
                    }
                    Some((*layer_index, object_id.clone(), *region_id, geometry))
                })
                .collect();

            for (layer_index, object_id, region_id, geometry) in contacts {
                let z = plan
                    .global_layers
                    .get(layer_index as usize)
                    .map_or(0.0, |layer| layer.z);
                ir.candidates.push(SupportCandidate {
                    id,
                    geometry,
                    source: SupportCandidateSource {
                        object_id,
                        region_id,
                        global_layer_index: layer_index,
                        z_units: mm_to_units(z),
                    },
                    enforced: false,
                    blocked: false,
                });
                id += 1;
            }
            ir.candidates.sort_by_key(|candidate| {
                (
                    candidate.source.global_layer_index,
                    candidate.source.object_id.clone(),
                    candidate.source.region_id,
                    candidate.id,
                )
            });
            for candidate in &ir.candidates {
                let key = RegionKey {
                    global_layer_index: candidate.source.global_layer_index,
                    object_id: candidate.source.object_id.clone(),
                    region_id: candidate.source.region_id,
                    variant_chain: Vec::new(),
                };
                ir.family_assignments
                    .entry((
                        candidate.source.object_id.clone(),
                        candidate.source.region_id,
                    ))
                    .or_insert_with(|| {
                        region_map
                            .as_deref()
                            .and_then(|map| {
                                map.entries
                                    .get(&key)
                                    .map(|plan| map.config_for_raw(plan.config))
                            })
                            .map(support_family)
                            .unwrap_or_else(|| "traditional".to_string())
                    });
            }
            // SliceIR has no facet classification, so the highest observed
            // cross-section is the narrowest truthful model-termination
            // approximation. The exact-Z service uses the same fallback.
            for (object_id, (top_layer, top_polygons)) in object_tops {
                let Some(plate) = object_bounds
                    .get(&object_id)
                    .and_then(|bounds| rectangle_from_bounds(*bounds))
                else {
                    continue;
                };
                for key in ir.model_occupancy.keys() {
                    if key.object_id == object_id && key.global_support_layer_index == top_layer {
                        ir.termination_surfaces.insert(
                            key.clone(),
                            top_polygons
                                .clone()
                                .into_iter()
                                .chain(std::iter::once(plate.clone()))
                                .collect(),
                        );
                    }
                }
                ir.baseline_feasible_envelope.push(plate);
            }
        }
    }
    blackboard.commit_support_analysis(Arc::new(ir))
}

/// Resolves the config half of [`SupportContactParams`] once per slice.
///
/// * `fw` -- the external-perimeter extrusion width -- is read as extensions
///   `outer_wall_line_width`, falling back to the typed `line_width` field,
///   falling back to `0.4` mm. This mirrors `resolve_line_width_mm` in
///   `crate::builtins::overhang_annotation_producer`, the resolution
///   `annotate_overhangs`' caller already uses.
/// * `support_threshold_overlap` is canonical
///   `ConfigOptionFloatOrPercent(50., true)`, i.e. 50% of `fw` by default, and
///   resolves against `fw` as its base.
/// * `support_expansion` is canonical `coFloat`, default `0`.
///
/// `lower_layer_height_mm` is per-layer and is filled in by the caller.
fn resolve_contact_params(
    config: &ResolvedConfig,
    threshold_angle_deg: f32,
) -> SupportContactParams {
    let typed_line_width = if config.line_width > 0.0 {
        config.line_width
    } else {
        DEFAULT_LINE_WIDTH_MM
    };
    let external_perimeter_width_mm =
        extension_float(config, "outer_wall_line_width").unwrap_or(typed_line_width);
    let threshold_overlap_mm = extension_abs_value(
        config,
        "support_threshold_overlap",
        external_perimeter_width_mm,
    )
    .unwrap_or(DEFAULT_THRESHOLD_OVERLAP_FRACTION * external_perimeter_width_mm);
    SupportContactParams {
        threshold_angle_deg,
        lower_layer_height_mm: 0.0,
        external_perimeter_width_mm,
        threshold_overlap_mm,
        xy_expansion_mm: extension_float(config, "support_expansion").unwrap_or(0.0),
    }
}

/// Line width used when neither `outer_wall_line_width` nor the typed
/// `line_width` field carries a positive value (the typed field defaults to
/// `0.0`, which would silently disable the tiny-spot filter). Matches the
/// guest-side default used by `classic-perimeters`/`arachne-perimeters`.
const DEFAULT_LINE_WIDTH_MM: f32 = 0.4;

/// Canonical `support_threshold_overlap` default: `ConfigOptionFloatOrPercent(50., true)`.
const DEFAULT_THRESHOLD_OVERLAP_FRACTION: f32 = 0.5;

/// Absolute (non-percent) float read from `extensions`.
fn extension_float(config: &ResolvedConfig, key: &str) -> Option<f32> {
    match config.extensions.get(key)? {
        ConfigValue::Float(value) => Some(*value as f32),
        ConfigValue::FloatOrPercent {
            value,
            is_percent: false,
        } => Some(*value as f32),
        _ => None,
    }
}

/// `extensions` read mirroring `ConfigOptionFloatOrPercent::get_abs_value`:
/// a percent resolves against `base`, an absolute value is returned unchanged.
fn extension_abs_value(config: &ResolvedConfig, key: &str, base: f32) -> Option<f32> {
    match config.extensions.get(key)? {
        ConfigValue::Percent(percent) => (base > 0.0).then(|| *percent as f32 / 100.0 * base),
        ConfigValue::FloatOrPercent { value, is_percent } => {
            if *is_percent {
                (base > 0.0).then(|| *value as f32 / 100.0 * base)
            } else {
                Some(*value as f32)
            }
        }
        ConfigValue::Float(value) => Some(*value as f32),
        _ => None,
    }
}

fn support_family(config: &ResolvedConfig) -> String {
    let support_family = config
        .extensions
        .get(SUPPORT_FAMILY_CONFIG_KEY)
        .and_then(|value| match value {
            ConfigValue::String(value) => Some(value.as_str()),
            _ => None,
        });
    let support_type = config
        .extensions
        .get(SUPPORT_GENERATOR_CONFIG_KEY)
        .and_then(|value| match value {
            ConfigValue::String(value) => Some(value.as_str()),
            _ => None,
        })
        .or(match config.support_type {
            SupportType::Tree => Some("tree"),
            SupportType::Traditional => None,
        });
    select_support_family(support_family, support_type).to_string()
}

/// Effective printed height of `layer_index`, derived as the Z delta from the
/// layer below. Canonical scales the contact offset by the *lower* layer's
/// height, so this is the value the detector needs per entry. Layer 0 has no
/// predecessor and takes its own Z as its height.
fn layer_height_mm(global_layers: &[slicer_ir::slice_ir::GlobalLayer], layer_index: u32) -> f32 {
    let Some(layer) = global_layers.iter().find(|l| l.index == layer_index) else {
        return 0.0;
    };
    if layer_index == 0 {
        return layer.z;
    }
    global_layers
        .iter()
        .find(|l| l.index == layer_index - 1)
        .map_or(layer.z, |below| layer.z - below.z)
}

fn rectangle_from_bounds((min_x, max_x, min_y, max_y): (i64, i64, i64, i64)) -> Option<ExPolygon> {
    if min_x >= max_x || min_y >= max_y {
        return None;
    }
    Some(ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: min_x, y: min_y },
                Point2 { x: max_x, y: min_y },
                Point2 { x: max_x, y: max_y },
                Point2 { x: min_x, y: max_y },
            ],
        },
        holes: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::{
        ConfigValue, GlobalLayer, LayerPlanIR, MeshIR, RegionMapIR, RegionPlan, ResolvedConfig,
        SliceIR, SlicedRegion,
    };

    /// Axis-aligned square in **millimetres**.
    ///
    /// Fixtures here must be mm-scale: contact detection now runs canonical's
    /// `-0.1 * fw` tiny-spot filter, so the raw-unit squares this module used
    /// before (~30 units ~= 0.003mm) are far below one line width and are
    /// filtered away entirely. The geometry below is sized in whole
    /// millimetres for the same reason.
    fn square(x_mm: f32, y_mm: f32, size_mm: f32) -> ExPolygon {
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2::from_mm(x_mm, y_mm),
                    Point2::from_mm(x_mm + size_mm, y_mm),
                    Point2::from_mm(x_mm + size_mm, y_mm + size_mm),
                    Point2::from_mm(x_mm, y_mm + size_mm),
                ],
            },
            holes: Vec::new(),
        }
    }

    /// Two 0.2mm-thick layers at realistic Z. `GlobalLayer::default()` has
    /// `z == 0`, which makes every layer height 0 and therefore every
    /// `lower_layer_offset` 0 — a plain difference, never the angle-thresholded
    /// path these fixtures mean to exercise.
    fn global_layers(count: u32) -> Vec<GlobalLayer> {
        (0..count)
            .map(|index| GlobalLayer {
                index,
                z: (index + 1) as f32 * 0.2,
                ..GlobalLayer::default()
            })
            .collect()
    }

    fn support_enabled_config() -> ResolvedConfig {
        ResolvedConfig {
            support_enabled: true,
            ..ResolvedConfig::default()
        }
    }

    /// Commits a two-layer, single-region slice stack for object `"object"`,
    /// region 3, with the given lower and upper footprints.
    fn blackboard_with_stack(lower: &ExPolygon, upper: &ExPolygon) -> Blackboard {
        let mut blackboard = Blackboard::new(Arc::new(MeshIR::default()), 2);
        blackboard
            .commit_layer_plan(Arc::new(LayerPlanIR {
                global_layers: global_layers(2),
                ..LayerPlanIR::default()
            }))
            .unwrap();
        blackboard
            .commit_slice_ir(Arc::new(vec![
                SliceIR {
                    global_layer_index: 0,
                    regions: vec![SlicedRegion {
                        object_id: "object".to_string(),
                        region_id: 3,
                        polygons: vec![lower.clone()],
                        ..SlicedRegion::default()
                    }],
                    ..SliceIR::default()
                },
                SliceIR {
                    global_layer_index: 1,
                    regions: vec![SlicedRegion {
                        object_id: "object".to_string(),
                        region_id: 3,
                        polygons: vec![upper.clone()],
                        ..SlicedRegion::default()
                    }],
                    ..SliceIR::default()
                },
            ]))
            .unwrap();
        blackboard
    }

    #[test]
    fn support_analysis_populates_all_derivable_inputs() {
        // Layer 1 is a 5mm square overhanging a 3mm one by 1mm on every side,
        // so layer 1 genuinely overhangs at the default 30-degree threshold
        // (0.2mm layer / tan(31 deg) = 0.33mm of required overlap).
        let lower = square(1.0, 2.0, 3.0);
        let upper = square(0.0, 1.0, 5.0);
        let mut blackboard = blackboard_with_stack(&lower, &upper);

        commit_support_analysis_builtin(&mut blackboard, &support_enabled_config()).unwrap();
        let analysis = blackboard.support_analysis().unwrap();
        let key = SupportGeometryKey {
            global_support_layer_index: 0,
            object_id: "object".to_string(),
            region_id: 3,
        };
        assert_eq!(
            analysis.shared_settings.get("support_enabled"),
            Some(&"true".to_string())
        );
        assert_eq!(
            analysis.model_occupancy.get(&key),
            Some(&vec![lower.clone()])
        );

        // Candidates are support contacts, not cross-sections: exactly one, at
        // the overhanging layer, and never at the supported layer below it.
        assert_eq!(analysis.candidates.len(), 1);
        assert_eq!(analysis.candidates[0].source.global_layer_index, 1);
        assert_eq!(analysis.candidates[0].source.object_id, "object");
        assert_eq!(analysis.candidates[0].source.region_id, 3);
        assert!(!analysis.candidates[0].enforced);
        assert!(!analysis.candidates[0].blocked);
        // The contact is the overhanging remainder, strictly smaller than the
        // upper cross-section it came from.
        assert!(!analysis.candidates[0].geometry.is_empty());
        assert_ne!(analysis.candidates[0].geometry, vec![upper.clone()]);

        let termination_key = SupportGeometryKey {
            global_support_layer_index: 1,
            object_id: "object".to_string(),
            region_id: 3,
        };
        assert_eq!(analysis.termination_surfaces.len(), 1);
        assert_eq!(analysis.termination_surfaces[&termination_key].len(), 2);
        assert_eq!(analysis.termination_surfaces[&termination_key][0], upper);
        assert!(!analysis.termination_surfaces.contains_key(&key));
        assert_eq!(analysis.baseline_feasible_envelope.len(), 1);
        assert_eq!(
            analysis.baseline_feasible_envelope[0].contour.points.len(),
            4
        );
        assert_eq!(
            analysis.family_assignments.get(&("object".to_string(), 3)),
            Some(&"traditional".to_string())
        );
    }

    #[test]
    fn region_covered_by_the_layer_above_is_not_a_candidate() {
        // A region fully covered from above is not an overhang, so no contact
        // may be emitted for it. This rule lives here, not in a planner: the
        // planners receive finished contacts and own routing, not the judgement
        // of what counts as an overhang.
        //
        // Inherited from the traditional planner's former
        // `fully_covered_candidate_is_declined`, which asserted this shape but
        // actually passed on an unrelated empty-mesh path.
        let wide = square(0.0, 0.0, 4.0);
        let narrow = square(1.0, 1.0, 1.0);
        // Layer 0 is wide, layer 1 is narrow and sits entirely within it.
        let mut blackboard = blackboard_with_stack(&wide, &narrow);

        commit_support_analysis_builtin(&mut blackboard, &support_enabled_config()).unwrap();
        let analysis = blackboard.support_analysis().unwrap();

        assert!(
            analysis.candidates.is_empty(),
            "a region wholly covered by the layer below it overhangs nothing, so it \
             must yield no candidates; got {:?}",
            analysis.candidates
        );
    }

    #[test]
    fn straight_column_yields_no_support_candidates() {
        // Regression pin for packet 224 RC-0: this stage previously emitted one
        // candidate per non-empty region per layer with no overhang detection
        // whatsoever, so a straight column produced support candidates at every
        // layer. Identical footprints must now produce none.
        let polygon = square(1.0, 2.0, 3.0);
        let mut blackboard = blackboard_with_stack(&polygon, &polygon);

        commit_support_analysis_builtin(&mut blackboard, &support_enabled_config()).unwrap();
        let analysis = blackboard.support_analysis().unwrap();

        assert!(
            analysis.candidates.is_empty(),
            "a straight column has no overhang and must yield no candidates, got {:?}",
            analysis.candidates
        );
        // Occupancy and termination are independent of contact detection and
        // must still be populated.
        assert_eq!(analysis.model_occupancy.len(), 2);
        assert_eq!(analysis.termination_surfaces.len(), 1);
    }

    /// F-2 regression pin. `support_threshold_angle` is CLI-bound, so
    /// `resolve_*` routes it to the typed field and never to `extensions`. This
    /// stage used to read `extensions` only, so it fell through to a hardcoded
    /// 45.0 on every slice and the user's configured angle was never applied.
    ///
    /// Asserts both halves: the default is the canonical 30.0 (OrcaSlicer
    /// `PrintConfig.cpp` `support_threshold_angle`, `ConfigOptionInt(30)`), and
    /// a configured value reaches the detector rather than the default.
    #[test]
    fn configured_threshold_angle_reaches_detection() {
        let polygon = square(1.0, 2.0, 3.0);

        let mut blackboard = blackboard_with_stack(&polygon, &polygon);
        commit_support_analysis_builtin(&mut blackboard, &support_enabled_config()).unwrap();
        assert_eq!(
            blackboard
                .support_analysis()
                .unwrap()
                .shared_settings
                .get("support_threshold_angle_deg"),
            Some(&"30".to_string()),
            "default must be the canonical 30 deg, owned by the ResolvedConfig macro line"
        );

        let mut blackboard = blackboard_with_stack(&polygon, &polygon);
        let config = ResolvedConfig {
            support_enabled: true,
            support_threshold_angle: 12.5,
            ..ResolvedConfig::default()
        };
        commit_support_analysis_builtin(&mut blackboard, &config).unwrap();
        assert_eq!(
            blackboard
                .support_analysis()
                .unwrap()
                .shared_settings
                .get("support_threshold_angle_deg"),
            Some(&"12.5".to_string()),
            "the configured typed field must reach detection; reading `extensions`              instead silently pinned this to the default"
        );
    }

    #[test]
    fn support_analysis_uses_region_map_family_precedence() {
        let mut blackboard = Blackboard::new(Arc::new(MeshIR::default()), 1);
        blackboard
            .commit_layer_plan(Arc::new(LayerPlanIR {
                global_layers: global_layers(2),
                ..LayerPlanIR::default()
            }))
            .unwrap();
        blackboard
            // Family assignment is keyed off candidates, and candidates are now
            // support contacts, so each region needs a genuine overhang: layer 1
            // is wider than layer 0.
            .commit_slice_ir(Arc::new(
                [(0_u32, 1.0_f32), (1_u32, 2.0_f32)]
                    .into_iter()
                    .map(|(global_layer_index, size)| SliceIR {
                        global_layer_index,
                        regions: (3..=5)
                            .map(|region_id| SlicedRegion {
                                object_id: "object".to_string(),
                                region_id,
                                polygons: vec![square(0.0, 0.0, size)],
                                ..SlicedRegion::default()
                            })
                            .collect(),
                        ..SliceIR::default()
                    })
                    .collect::<Vec<_>>(),
            ))
            .unwrap();

        let mut region_map = RegionMapIR::default();
        let canonical_tree = ResolvedConfig {
            extensions: [(
                "support_family".to_string(),
                ConfigValue::String("tree".to_string()),
            )]
            .into_iter()
            .collect(),
            ..ResolvedConfig::default()
        };
        let alias_override = ResolvedConfig {
            extensions: [
                (
                    "support_family".to_string(),
                    ConfigValue::String("tree".to_string()),
                ),
                (
                    "support_type".to_string(),
                    ConfigValue::String("normal(auto)".to_string()),
                ),
            ]
            .into_iter()
            .collect(),
            ..ResolvedConfig::default()
        };
        let enum_tree = ResolvedConfig {
            support_type: slicer_ir::SupportType::Tree,
            ..ResolvedConfig::default()
        };
        for (region_id, config) in [(3, canonical_tree), (4, alias_override), (5, enum_tree)] {
            let config_id = region_map.intern_config(config);
            // Family is resolved at the contact layer, so the map must carry the
            // region on every layer it exists on — as a production region map does.
            for global_layer_index in 0..=1 {
                region_map.entries.insert(
                    RegionKey {
                        global_layer_index,
                        object_id: "object".to_string(),
                        region_id,
                        variant_chain: Vec::new(),
                    },
                    RegionPlan {
                        config: config_id,
                        ..RegionPlan::default()
                    },
                );
            }
        }
        blackboard.commit_region_map(Arc::new(region_map)).unwrap();

        commit_support_analysis_builtin(&mut blackboard, &support_enabled_config()).unwrap();
        let assignments = &blackboard.support_analysis().unwrap().family_assignments;
        assert_eq!(assignments[&(String::from("object"), 3)], "tree");
        assert_eq!(assignments[&(String::from("object"), 4)], "traditional");
        assert_eq!(assignments[&(String::from("object"), 5)], "tree");
    }
}
