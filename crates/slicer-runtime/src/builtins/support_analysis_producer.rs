//! Host-owned, strategy-neutral support analysis.

use std::collections::BTreeMap;
use std::sync::Arc;

use slicer_core::algos::overhang_annotation::detect_support_overhangs;
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
/// [`detect_support_overhangs`](slicer_core::algos::overhang_annotation::detect_support_overhangs),
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
            // Per-(object, region) footprint series, keyed by layer index, used
            // for angle-thresholded contact detection after the sweep.
            let mut region_series: BTreeMap<(String, u64), BTreeMap<u32, Vec<ExPolygon>>> =
                BTreeMap::new();
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
                    // Collect the per-region footprint series; candidates are
                    // derived from it after the sweep, once every layer of each
                    // region is known.
                    region_series
                        .entry((region.object_id.clone(), region.region_id))
                        .or_default()
                        .insert(slice.global_layer_index, region.polygons.clone());
                }
            }

            // Angle-thresholded contact detection per (object, region), so
            // overhang attribution survives multi-object and multi-region
            // plates. `detect_support_overhangs` requires physically adjacent
            // layers, so each series is densified across its own contiguous
            // layer span: a region that is absent on a layer contributes an
            // empty footprint there. That is the correct semantics rather than
            // a papering-over — a region appearing above empty space is wholly
            // unsupported, and `diff(current, [])` yields exactly that.
            for ((object_id, region_id), by_layer) in region_series {
                let (Some(&first), Some(&last)) =
                    (by_layer.keys().next(), by_layer.keys().next_back())
                else {
                    continue;
                };
                let series = (first..=last)
                    .map(|layer_index| {
                        (
                            layer_index,
                            layer_height_mm(&plan.global_layers, layer_index),
                            by_layer.get(&layer_index).cloned().unwrap_or_default(),
                        )
                    })
                    .collect::<Vec<_>>();

                let contacts = detect_support_overhangs(&series, threshold_angle_deg);
                let mut contact_layers = contacts.into_iter().collect::<Vec<_>>();
                contact_layers.sort_by_key(|(layer_index, _)| *layer_index);
                for (layer_index, geometry) in contact_layers {
                    let z = plan
                        .global_layers
                        .get(layer_index as usize)
                        .map_or(0.0, |layer| layer.z);
                    ir.candidates.push(SupportCandidate {
                        id,
                        geometry,
                        source: SupportCandidateSource {
                            object_id: object_id.clone(),
                            region_id,
                            global_layer_index: layer_index,
                            z_units: mm_to_units(z),
                        },
                        enforced: false,
                        blocked: false,
                    });
                    id += 1;
                }
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

    fn square(x: i64, y: i64, size: i64) -> ExPolygon {
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 { x, y },
                    Point2 { x: x + size, y },
                    Point2 {
                        x: x + size,
                        y: y + size,
                    },
                    Point2 { x, y: y + size },
                ],
            },
            holes: Vec::new(),
        }
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
                global_layers: vec![GlobalLayer::default(), GlobalLayer::default()],
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
        // Layer 1 is wider than layer 0, so layer 1 genuinely overhangs.
        let lower = square(10, 20, 30);
        let upper = square(0, 10, 50);
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
        let wide = square(0, 0, 40);
        let narrow = square(10, 10, 10);
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
        let polygon = square(10, 20, 30);
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
        let polygon = square(10, 20, 30);

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
                global_layers: vec![GlobalLayer::default()],
                ..LayerPlanIR::default()
            }))
            .unwrap();
        blackboard
            // Family assignment is keyed off candidates, and candidates are now
            // support contacts, so each region needs a genuine overhang: layer 1
            // is wider than layer 0.
            .commit_slice_ir(Arc::new(
                [(0_u32, 10_i64), (1_u32, 20_i64)]
                    .into_iter()
                    .map(|(global_layer_index, size)| SliceIR {
                        global_layer_index,
                        regions: (3..=5)
                            .map(|region_id| SlicedRegion {
                                object_id: "object".to_string(),
                                region_id,
                                polygons: vec![square(0, 0, size)],
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
