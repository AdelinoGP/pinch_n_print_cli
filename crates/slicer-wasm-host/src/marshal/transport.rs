//! Call-local current geometry under already-indexed source authority.
//!
//! This module neither selects targets nor resolves configuration. The adapters
//! will consume these private records in the next implementation step.

// Construction is connected before the split; adapter consumption follows in Step 5.
#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use slicer_ir::{
    ConfigView, PerimeterIR, PerimeterRegion, PreparedRegionData, RegionKey, SliceIR, SlicedRegion,
    SupportPlanIR, SurfaceClassificationIR,
};

use crate::selection::{LayerSelection, LayerTargetSource, SelectedLayerTarget};

use super::OriginId;

pub(crate) struct CurrentModelView<'a> {
    pub key: RegionKey,
    /// Indexed owner supplies config, held claims, coloring and resolved tool.
    pub target: &'a SelectedLayerTarget,
    pub region: SlicedRegion,
    pub prepared: PreparedRegionData,
    /// Exact own perimeter, or the sole permitted full-source donor.
    pub perimeter: Option<&'a PerimeterRegion>,
    pub wall_source: Option<OriginId>,
}

pub(crate) struct CurrentPerimeterView<'a> {
    pub origin: OriginId,
    pub config: &'a Arc<ConfigView>,
    /// None is an explicit empty view, not omission or default authority.
    pub region: Option<&'a PerimeterRegion>,
}

pub(crate) struct LayerTransport<'a> {
    pub models: Vec<CurrentModelView<'a>>,
    pub perimeter_sources: Vec<CurrentModelView<'a>>,
    pub perimeters: Vec<CurrentPerimeterView<'a>>,
    /// One deduplicated wire record per support-carrier full origin
    /// (object + source region + exact ordered chain), plus raft/anchored
    /// contexts; distinct carrier plan contexts remain in `selection.targets`.
    pub non_model_targets: Vec<&'a SelectedLayerTarget>,
    /// Current roles remain attached to the indexed carrier entry positions.
    pub support_plan: Option<&'a SupportPlanIR>,
}

fn origin(region: &SlicedRegion) -> OriginId {
    OriginId {
        object_id: region.object_id.clone(),
        region_id: region.region_id,
        variant_chain: region.variant_chain.clone(),
    }
}

fn key_origin(key: &RegionKey) -> OriginId {
    OriginId {
        object_id: key.object_id.clone(),
        region_id: key.region_id,
        variant_chain: key.variant_chain.clone(),
    }
}

fn associate_perimeter<'a>(
    region: &SlicedRegion,
    perimeters: &HashMap<OriginId, &'a PerimeterRegion>,
) -> (Option<&'a PerimeterRegion>, Option<OriginId>) {
    if let Some(own) = perimeters.get(&origin(region)) {
        return (Some(*own), None);
    }
    let donor = if let Some(parent) = slicer_ir::modifier_base_region_id(region.region_id) {
        OriginId {
            object_id: region.object_id.clone(),
            region_id: parent,
            variant_chain: region.variant_chain.clone(),
        }
    } else if !region.variant_chain.is_empty() {
        OriginId {
            object_id: region.object_id.clone(),
            region_id: region.region_id
                / slicer_core::algos::paint_segmentation::PAINT_VARIANT_REGION_ID_STRIDE,
            variant_chain: Vec::new(),
        }
    } else {
        return (None, None);
    };
    (perimeters.get(&donor).copied(), Some(donor))
}

pub(crate) fn assemble_layer_transport<'a>(
    selection: &'a LayerSelection,
    layer_index: u32,
    slice: Option<&SliceIR>,
    perimeter: Option<&'a PerimeterIR>,
    support_plan: Option<&'a SupportPlanIR>,
    prepared: Option<&[PreparedRegionData]>,
    surface_classification: Option<&SurfaceClassificationIR>,
) -> LayerTransport<'a> {
    let slice = slice.filter(|slice| slice.global_layer_index == layer_index);
    let current = slice.map_or(&[][..], |slice| slice.regions.as_slice());
    let current_by_origin: HashMap<_, _> = current.iter().map(|r| (origin(r), r)).collect();
    // Bind the arena's source-order records BEFORE any filtering or reordering.
    let prepared_by_origin: HashMap<_, _> = current
        .iter()
        .zip(prepared.unwrap_or(&[]))
        .map(|(region, data)| (origin(region), data))
        .collect();
    let perimeters: HashMap<_, _> = perimeter
        .filter(|perimeter| perimeter.global_layer_index == layer_index)
        .into_iter()
        .flat_map(|perimeter| &perimeter.regions)
        .map(|region| {
            (
                OriginId {
                    object_id: region.object_id.clone(),
                    region_id: region.region_id,
                    variant_chain: region.variant_chain.clone(),
                },
                region,
            )
        })
        .collect();
    let mut models = Vec::new();
    let mut non_model_targets = Vec::new();
    let mut seen = HashSet::new();
    // One wire record per full support-carrier origin (object + source region
    // + exact ordered chain). Same-full-origin entries keep their distinct
    // selected contexts/demands/bodies/provenance in `selection.targets`; a
    // different ordered chain MUST retain a distinct wire record.
    let mut carrier_origins: HashSet<OriginId> = HashSet::new();
    for target in &selection.targets {
        let LayerTargetSource::Model { key, members } = &target.source else {
            if let LayerTargetSource::SupportCarrier {
                object_id,
                region_id,
                variant_chain,
                ..
            } = &target.source
            {
                let carrier = OriginId {
                    object_id: object_id.clone(),
                    region_id: *region_id,
                    variant_chain: variant_chain.clone(),
                };
                if !carrier_origins.insert(carrier) {
                    continue;
                }
            }
            non_model_targets.push(target);
            continue;
        };
        for member in std::iter::once(key).chain(members) {
            let identity = key_origin(member);
            if member.global_layer_index != layer_index || !seen.insert(identity.clone()) {
                continue;
            }
            let present = current_by_origin.get(&identity);
            let region = present.map_or_else(
                || SlicedRegion {
                    object_id: member.object_id.clone(),
                    region_id: member.region_id,
                    variant_chain: member.variant_chain.clone(),
                    ..SlicedRegion::default()
                },
                |region| (*region).clone(),
            );
            let prepared = if present.is_none() {
                PreparedRegionData::default()
            } else if let Some(data) = prepared_by_origin.get(&identity) {
                (*data).clone()
            } else {
                super::prepare_regions(
                    std::slice::from_ref(&region),
                    surface_classification,
                    layer_index,
                )
                .remove(0)
            };
            let (perimeter, wall_source) = associate_perimeter(&region, &perimeters);
            models.push(CurrentModelView {
                key: member.clone(),
                target,
                region,
                prepared,
                perimeter,
                wall_source,
            });
        }
    }
    // Only stored members reach the existing union/angle-pairing helper.
    let selected_slice = SliceIR {
        global_layer_index: layer_index,
        regions: models.iter().map(|view| view.region.clone()).collect(),
        ..SliceIR::default()
    };
    let projected = super::perimeter_source_regions(&selected_slice);
    // Recompute geometry-derived preparation on the filtered union: an arena
    // perimeter-source record may already contain an unindexed child's geometry.
    let projected_prepared: HashMap<_, _> = projected
        .iter()
        .map(origin)
        .zip(super::prepare_regions(
            &projected,
            surface_classification,
            layer_index,
        ))
        .collect();
    let owners: HashMap<_, _> = models
        .iter()
        .map(|view| (key_origin(&view.key), (view.key.clone(), view.target)))
        .collect();
    let perimeter_sources = projected
        .into_iter()
        .map(|region| {
            let identity = origin(&region);
            let (key, target) = owners[&identity].clone();
            let (perimeter, wall_source) = associate_perimeter(&region, &perimeters);
            CurrentModelView {
                key,
                target,
                prepared: projected_prepared[&identity].clone(),
                region,
                perimeter,
                wall_source,
            }
        })
        .collect();
    let mut perimeter_views: Vec<_> = selection
        .perimeter_configs
        .iter()
        .map(|(origin, config)| CurrentPerimeterView {
            origin: origin.clone(),
            config,
            region: perimeters.get(origin).copied(),
        })
        .collect();
    // Both adapters preserve this delivery order, so randomized config-map
    // iteration must not choose the emitted region-block/travel order. Compare
    // complete source identities; chain elements keep their exact order.
    perimeter_views.sort_by(|left, right| {
        (
            &left.origin.object_id,
            left.origin.region_id,
            left.origin.variant_chain.as_slice(),
        )
            .cmp(&(
                &right.origin.object_id,
                right.origin.region_id,
                right.origin.variant_chain.as_slice(),
            ))
    });
    LayerTransport {
        models,
        perimeter_sources,
        perimeters: perimeter_views,
        non_model_targets,
        support_plan,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selection::OptionalProductPresence;
    use slicer_ir::{ExPolygon, PaintValue, Point2};

    fn region(id: u64, chain: Vec<(String, PaintValue)>) -> SlicedRegion {
        SlicedRegion {
            object_id: "object".into(),
            region_id: id,
            variant_chain: chain,
            ..SlicedRegion::default()
        }
    }

    fn key(region: &SlicedRegion) -> RegionKey {
        RegionKey {
            global_layer_index: 7,
            object_id: region.object_id.clone(),
            region_id: region.region_id,
            variant_chain: region.variant_chain.clone(),
        }
    }

    fn selection(regions: &[SlicedRegion]) -> LayerSelection {
        // exhaustive: private assembly fixture explicitly provisions all indexed authority.
        LayerSelection {
            stage_id: "Layer::Perimeters".into(),
            invoke: true,
            invocation_config: Arc::new(ConfigView::default()),
            support_plan_presence: OptionalProductPresence::Absent,
            targets: regions
                .iter()
                .map(|region| {
                    // exhaustive: all model claims and grants are explicit for the assembly oracle.
                    SelectedLayerTarget {
                        source: LayerTargetSource::Model {
                            key: key(region),
                            members: vec![key(region)],
                        },
                        config: Arc::new(ConfigView::default()),
                        held_claims: vec!["perimeter".into()],
                        authored_coloring: false,
                        resolved_role_tool: Some(3),
                    }
                })
                .collect(),
            slice_regions: regions.to_vec(),
            perimeter_configs: HashMap::new(),
        }
    }

    fn rectangle(x: f32) -> ExPolygon {
        ExPolygon {
            contour: slicer_ir::Polygon {
                points: vec![
                    Point2::from_mm(x, 0.0),
                    Point2::from_mm(x + 1.0, 0.0),
                    Point2::from_mm(x + 1.0, 1.0),
                    Point2::from_mm(x, 1.0),
                ],
            },
            holes: Vec::new(),
        }
    }

    #[test]
    fn unindexed_child_is_excluded_before_union_and_bridge_pairing() {
        let mut base = region(2, Vec::new());
        base.polygons = vec![rectangle(0.0)];
        base.internal_bridge_areas = vec![rectangle(0.0)];
        base.internal_bridge_angles_deg = vec![10.0];
        let mut stored = region(
            slicer_ir::modifier_sub_region_id(2, "stored", &[]),
            Vec::new(),
        );
        stored.polygons = vec![rectangle(2.0)];
        stored.internal_bridge_areas = vec![rectangle(2.0)];
        stored.internal_bridge_angles_deg = vec![20.0];
        let mut extra = region(
            slicer_ir::modifier_sub_region_id(2, "extra", &[]),
            Vec::new(),
        );
        extra.polygons = vec![rectangle(9.0)];
        extra.internal_bridge_areas = vec![rectangle(9.0)];
        extra.internal_bridge_angles_deg = vec![99.0];
        let mut selection = selection(std::slice::from_ref(&base));
        if let LayerTargetSource::Model { members, .. } = &mut selection.targets[0].source {
            members.push(key(&stored));
        }
        let slice = SliceIR {
            global_layer_index: 7,
            regions: vec![extra, stored, base],
            ..SliceIR::default()
        };
        let transport =
            assemble_layer_transport(&selection, 7, Some(&slice), None, None, None, None);
        assert_eq!(transport.models.len(), 2);
        assert_eq!(transport.perimeter_sources.len(), 1);
        let merged = &transport.perimeter_sources[0].region;
        assert_eq!(merged.polygons.len(), 2);
        assert_eq!(merged.internal_bridge_angles_deg, vec![10.0, 20.0]);
        assert_eq!(
            merged.internal_bridge_areas,
            vec![rectangle(0.0), rectangle(2.0)]
        );
        assert!(merged
            .polygons
            .iter()
            .flat_map(|p| &p.contour.points)
            .all(|p| p.x < Point2::from_mm(9.0, 0.0).x));
    }

    fn perimeter(region: &SlicedRegion) -> PerimeterRegion {
        PerimeterRegion {
            object_id: region.object_id.clone(),
            region_id: region.region_id,
            variant_chain: region.variant_chain.clone(),
            infill_areas: vec![rectangle(4.0)],
            ..PerimeterRegion::default()
        }
    }

    #[test]
    fn painted_fallback_uses_decoded_source_and_empty_chain_then_own_wins() {
        let painted = region(
            2 * slicer_core::algos::paint_segmentation::PAINT_VARIANT_REGION_ID_STRIDE + 11,
            vec![("material".into(), PaintValue::ToolIndex(3))],
        );
        let selection = selection(std::slice::from_ref(&painted));
        let base = region(2, Vec::new());
        let wrong_chain = region(2, painted.variant_chain.clone());
        let mut perimeters = PerimeterIR {
            global_layer_index: 7,
            regions: vec![perimeter(&wrong_chain), perimeter(&base)],
            ..PerimeterIR::default()
        };
        let transport =
            assemble_layer_transport(&selection, 7, None, Some(&perimeters), None, None, None);
        let view = &transport.models[0];
        assert_eq!(view.perimeter.unwrap().region_id, 2);
        assert_eq!(view.perimeter.unwrap().variant_chain, Vec::new());
        assert_eq!(view.wall_source.as_ref().unwrap().region_id, 2);
        assert_eq!(view.wall_source.as_ref().unwrap().variant_chain, Vec::new());
        perimeters.regions.push(perimeter(&painted));
        let transport =
            assemble_layer_transport(&selection, 7, None, Some(&perimeters), None, None, None);
        assert_eq!(
            transport.models[0].perimeter.unwrap().region_id,
            painted.region_id
        );
        assert_eq!(transport.models[0].wall_source, None);
    }

    #[test]
    fn modifier_donor_preserves_parent_source_and_ordered_chain_without_sibling_fallback() {
        let chain = vec![("material".into(), PaintValue::ToolIndex(3))];
        let parent = region(12345, chain.clone());
        let child = region(
            slicer_ir::modifier_sub_region_id(12345, "child", &[]),
            chain,
        );
        let selection = selection(std::slice::from_ref(&child));
        let mut perimeters = PerimeterIR {
            global_layer_index: 7,
            regions: vec![perimeter(&region(12345, Vec::new()))],
            ..PerimeterIR::default()
        };
        let transport =
            assemble_layer_transport(&selection, 7, None, Some(&perimeters), None, None, None);
        assert!(transport.models[0].perimeter.is_none());
        assert_eq!(
            transport.models[0].wall_source.as_ref().unwrap().region_id,
            12345
        );
        assert_eq!(
            transport.models[0]
                .wall_source
                .as_ref()
                .unwrap()
                .variant_chain,
            parent.variant_chain
        );
        perimeters.regions.push(perimeter(&parent));
        let transport =
            assemble_layer_transport(&selection, 7, None, Some(&perimeters), None, None, None);
        assert_eq!(
            transport.models[0].perimeter.unwrap().variant_chain,
            parent.variant_chain
        );
    }

    #[test]
    fn perimeter_delivery_orders_full_source_identity_without_reordering_chains() {
        let alpha = ("alpha".into(), PaintValue::Flag(true));
        let beta = ("beta".into(), PaintValue::ToolIndex(3));
        let mut other_object = region(0, Vec::new());
        other_object.object_id = "z-object".into();
        // Independently authored full-key order, including empty and reversed chains.
        let ordered = vec![
            region(7, Vec::new()),
            region(7, vec![alpha.clone()]),
            region(7, vec![alpha.clone(), beta.clone()]),
            region(7, vec![beta, alpha]),
            region(8, Vec::new()),
            other_object,
        ];
        let configs: Vec<_> = ordered
            .iter()
            .enumerate()
            .map(|(index, _)| {
                Arc::new(ConfigView::from_map(HashMap::from([(
                    "marker".into(),
                    slicer_ir::ConfigValue::Int(index as i64),
                )])))
            })
            .collect();
        let mut selection = selection(&[]);
        for (region, config) in ordered.iter().zip(&configs).rev() {
            selection
                .perimeter_configs
                .insert(origin(region), config.clone());
        }
        let perimeters = PerimeterIR {
            global_layer_index: 7,
            regions: ordered.iter().map(perimeter).collect(),
            ..PerimeterIR::default()
        };
        let transport =
            assemble_layer_transport(&selection, 7, None, Some(&perimeters), None, None, None);
        assert_eq!(
            transport
                .perimeters
                .iter()
                .map(|view| view.origin.clone())
                .collect::<Vec<_>>(),
            ordered.iter().map(origin).collect::<Vec<_>>()
        );
        for (index, view) in transport.perimeters.iter().enumerate() {
            assert!(Arc::ptr_eq(view.config, &configs[index]));
            assert_eq!(view.region, Some(&perimeters.regions[index]));
        }
    }

    #[test]
    fn missing_selected_geometry_is_empty_not_snapshot_or_omission() {
        let mut snapshot = region(5, Vec::new());
        snapshot.polygons = vec![rectangle(0.0)];
        snapshot.top_solid_fill = vec![rectangle(0.0)];
        let mut selection = selection(std::slice::from_ref(&snapshot));
        selection
            .perimeter_configs
            .insert(origin(&snapshot), selection.targets[0].config.clone());
        let transport = assemble_layer_transport(&selection, 7, None, None, None, None, None);
        assert_eq!(transport.models.len(), 1);
        assert_eq!(transport.perimeter_sources.len(), 1);
        assert_eq!(transport.models[0].key.region_id, 5);
        assert_eq!(transport.models[0].region, region(5, Vec::new()));
        assert_eq!(transport.models[0].prepared, PreparedRegionData::default());
        assert_eq!(transport.models[0].target.held_claims, vec!["perimeter"]);
        assert!(Arc::ptr_eq(
            &transport.models[0].target.config,
            &selection.targets[0].config
        ));
        assert_eq!(transport.perimeters.len(), 1);
        assert!(transport.perimeters[0].region.is_none());
    }

    #[test]
    fn prepared_association_survives_filtering_and_reversed_current_order() {
        let a = region(2, Vec::new());
        // Same numeric source ID, distinct ordered chain: pair-only joins lose B.
        let b = region(2, vec![("material".into(), PaintValue::ToolIndex(3))]);
        let extra = region(99, Vec::new());
        let selection = selection(&[a.clone(), b.clone()]);
        let annotations = [
            PreparedRegionData {
                needs_support: true,
                prev_layer_boundary: vec![rectangle(2.0)],
                ..PreparedRegionData::default()
            },
            PreparedRegionData {
                needs_support: false,
                prev_layer_boundary: vec![rectangle(3.0)],
                ..PreparedRegionData::default()
            },
            PreparedRegionData {
                needs_support: true,
                prev_layer_boundary: vec![rectangle(99.0)],
                ..PreparedRegionData::default()
            },
        ];
        let slice = SliceIR {
            global_layer_index: 7,
            regions: vec![extra.clone(), b.clone(), a.clone()],
            ..SliceIR::default()
        };
        let data = vec![
            annotations[2].clone(),
            annotations[1].clone(),
            annotations[0].clone(),
        ];
        let forward = SliceIR {
            global_layer_index: 7,
            regions: vec![a, extra, b],
            ..SliceIR::default()
        };
        let forward_data = vec![
            annotations[0].clone(),
            annotations[2].clone(),
            annotations[1].clone(),
        ];
        let reversed =
            assemble_layer_transport(&selection, 7, Some(&slice), None, None, Some(&data), None);
        let normal = assemble_layer_transport(
            &selection,
            7,
            Some(&forward),
            None,
            None,
            Some(&forward_data),
            None,
        );
        assert_eq!(reversed.models.len(), 2);
        assert_eq!(normal.models.len(), 2);
        assert_eq!(reversed.models[0].key.region_id, 2);
        assert!(reversed.models[0].prepared.needs_support);
        assert_eq!(
            reversed.models[0].prepared.prev_layer_boundary,
            vec![rectangle(2.0)]
        );
        assert_eq!(reversed.models[1].key.region_id, 2);
        assert_eq!(
            reversed.models[1].key.variant_chain,
            vec![("material".into(), PaintValue::ToolIndex(3))]
        );
        assert!(!reversed.models[1].prepared.needs_support);
        assert_eq!(
            reversed.models[1].prepared.prev_layer_boundary,
            vec![rectangle(3.0)]
        );
        assert_eq!(
            normal.models[0].prepared.prev_layer_boundary,
            vec![rectangle(2.0)]
        );
        assert_eq!(
            normal.models[1].prepared.prev_layer_boundary,
            vec![rectangle(3.0)]
        );
    }

    fn carrier(
        entry_index: usize,
        chain: Vec<(String, PaintValue)>,
        demand: &str,
    ) -> SelectedLayerTarget {
        // exhaustive: carrier wire fixture provisions every indexed authority field for the dedup oracle.
        SelectedLayerTarget {
            source: LayerTargetSource::SupportCarrier {
                entry_index,
                object_id: "carrier".into(),
                region_id: 70,
                variant_chain: chain,
                family: "traditional".into(),
                demand_ids: vec![demand.into()],
                body_ids: vec![format!("body-{entry_index}")],
                provenance: vec![format!("producer-{entry_index}")],
                anchor_layer_index: 3,
            },
            config: Arc::new(ConfigView::default()),
            held_claims: vec!["support-body".into()],
            authored_coloring: false,
            resolved_role_tool: None,
        }
    }

    #[test]
    fn same_origin_carrier_contexts_share_one_wire_record_and_chains_stay_distinct() {
        // exhaustive: non-model-only selection provisions the empty model and authority maps for the dedup oracle.
        let selection = LayerSelection {
            stage_id: "Layer::Support".into(),
            invoke: true,
            invocation_config: Arc::new(ConfigView::default()),
            support_plan_presence: OptionalProductPresence::Present,
            targets: vec![
                carrier(0, Vec::new(), "demand-0"),
                carrier(1, Vec::new(), "demand-1"),
                carrier(
                    2,
                    vec![("material".into(), PaintValue::ToolIndex(3))],
                    "demand-2",
                ),
            ],
            slice_regions: Vec::new(),
            perimeter_configs: HashMap::new(),
        };
        let transport = assemble_layer_transport(&selection, 7, None, None, None, None, None);
        // Both same-full-origin plan contexts keep their selected
        // contexts/demands/provenance on the selection.
        assert_eq!(selection.targets.len(), 3);
        let wire: Vec<&LayerTargetSource> = transport
            .non_model_targets
            .iter()
            .map(|target| &target.source)
            .collect();
        assert_eq!(
            wire.len(),
            2,
            "two same-full-origin contexts share one wire record; the additional \
             nonempty-chain origin stays distinct"
        );
        match wire[0] {
            LayerTargetSource::SupportCarrier {
                demand_ids,
                provenance,
                variant_chain,
                ..
            } => {
                assert_eq!(demand_ids, &vec!["demand-0"]);
                assert_eq!(provenance, &vec!["producer-0"]);
                assert!(variant_chain.is_empty());
            }
            other => panic!("expected the first same-origin wire record, got {other:?}"),
        }
        match wire[1] {
            LayerTargetSource::SupportCarrier {
                demand_ids,
                variant_chain,
                ..
            } => {
                assert_eq!(demand_ids, &vec!["demand-2"]);
                assert_eq!(
                    variant_chain,
                    &vec![("material".into(), PaintValue::ToolIndex(3))]
                );
            }
            other => panic!("expected the nonempty-chain wire record, got {other:?}"),
        }
    }
}
