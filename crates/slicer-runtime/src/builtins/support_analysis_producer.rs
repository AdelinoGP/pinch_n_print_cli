//! Host-owned, strategy-neutral support analysis.

use std::collections::BTreeMap;
use std::sync::Arc;

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
pub fn commit_support_analysis_builtin(
    blackboard: &mut Blackboard,
    enable_support: bool,
) -> Result<(), crate::BlackboardError> {
    let mut ir = SupportAnalysisIR::default();
    ir.shared_settings
        .insert("support_enabled".to_string(), enable_support.to_string());
    if enable_support {
        // Unit fixtures may not run region mapping, so preserve their deterministic
        // traditional fallback while production runs consume the committed map.
        let region_map = blackboard.region_map().cloned();
        if let (Some(slices), Some(plan)) = (blackboard.slice_ir(), blackboard.layer_plan()) {
            let mut id = 0_u64;
            let mut object_bounds: BTreeMap<String, (i64, i64, i64, i64)> = BTreeMap::new();
            let mut object_tops: BTreeMap<String, (u32, Vec<ExPolygon>)> = BTreeMap::new();
            for slice in slices.iter() {
                let z = plan
                    .global_layers
                    .get(slice.global_layer_index as usize)
                    .map_or(0.0, |layer| layer.z);
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
                    let source = SupportCandidateSource {
                        object_id: region.object_id.clone(),
                        region_id: region.region_id,
                        global_layer_index: slice.global_layer_index,
                        z_units: mm_to_units(z),
                    };
                    ir.candidates.push(SupportCandidate {
                        id,
                        geometry: region.polygons.clone(),
                        source,
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

    #[test]
    fn support_analysis_populates_all_derivable_inputs() {
        let polygon = square(10, 20, 30);
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
                        polygons: vec![polygon.clone()],
                        ..SlicedRegion::default()
                    }],
                    ..SliceIR::default()
                },
                SliceIR {
                    global_layer_index: 1,
                    regions: vec![SlicedRegion {
                        object_id: "object".to_string(),
                        region_id: 3,
                        polygons: vec![polygon.clone()],
                        ..SlicedRegion::default()
                    }],
                    ..SliceIR::default()
                },
            ]))
            .unwrap();

        commit_support_analysis_builtin(&mut blackboard, true).unwrap();
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
            Some(&vec![polygon.clone()])
        );
        assert_eq!(analysis.candidates.len(), 2);
        assert!(analysis
            .candidates
            .iter()
            .all(|candidate| candidate.geometry == vec![polygon.clone()]));
        assert_eq!(analysis.candidates[0].source.object_id, "object");
        assert_eq!(analysis.candidates[0].source.region_id, 3);
        assert!(!analysis.candidates[0].enforced);
        assert!(!analysis.candidates[0].blocked);
        let termination_key = SupportGeometryKey {
            global_support_layer_index: 1,
            object_id: "object".to_string(),
            region_id: 3,
        };
        assert_eq!(analysis.termination_surfaces.len(), 1);
        assert_eq!(analysis.termination_surfaces[&termination_key].len(), 2);
        assert_eq!(analysis.termination_surfaces[&termination_key][0], polygon);
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
    fn support_analysis_uses_region_map_family_precedence() {
        let mut blackboard = Blackboard::new(Arc::new(MeshIR::default()), 1);
        blackboard
            .commit_layer_plan(Arc::new(LayerPlanIR {
                global_layers: vec![GlobalLayer::default()],
                ..LayerPlanIR::default()
            }))
            .unwrap();
        blackboard
            .commit_slice_ir(Arc::new(vec![SliceIR {
                global_layer_index: 0,
                regions: (3..=5)
                    .map(|region_id| SlicedRegion {
                        object_id: "object".to_string(),
                        region_id,
                        polygons: vec![square(0, 0, 10)],
                        ..SlicedRegion::default()
                    })
                    .collect(),
                ..SliceIR::default()
            }]))
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
            let key = RegionKey {
                global_layer_index: 0,
                object_id: "object".to_string(),
                region_id,
                variant_chain: Vec::new(),
            };
            let config_id = region_map.intern_config(config);
            region_map.entries.insert(
                key,
                RegionPlan {
                    config: config_id,
                    ..RegionPlan::default()
                },
            );
        }
        blackboard.commit_region_map(Arc::new(region_map)).unwrap();

        commit_support_analysis_builtin(&mut blackboard, true).unwrap();
        let assignments = &blackboard.support_analysis().unwrap().family_assignments;
        assert_eq!(assignments[&(String::from("object"), 3)], "tree");
        assert_eq!(assignments[&(String::from("object"), 4)], "traditional");
        assert_eq!(assignments[&(String::from("object"), 5)], "tree");
    }
}
