//! Immutable whole-print selection authority. Geometry remains call-local.
use crate::marshal::OriginId;
use slicer_ir::{
    ActiveRegion, ConfigId, ConfigView, GlobalLayer, PerimeterIR, RegionKey, RegionMapIR, SliceIR,
    SlicedRegion, SupportPlanIR,
};
use slicer_scheduler::{
    execution_plan::module_claims_match_active_region,
    validation::{resolve_held_claims, FillHolders, FILL_CLAIM_IDS},
};
pub use slicer_schema::{StageSpec, STAGES};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

/// Selection failure with the responsible binding's identity.
#[derive(Debug)]
pub struct LayerSelectionBuildError {
    /// Unchanged routing error.
    pub error: SelectionError,
    /// Failing stage.
    pub stage_id: String,
    /// Failing module.
    pub module_id: String,
}
/// Invalid stage or absent exact model authority.
#[derive(Debug, PartialEq, Eq)]
pub enum SelectionError {
    /// Not a schema-backed module Layer stage.
    UnknownLayerStage {
        /// Rejected stage identifier.
        stage_id: String,
    },
    /// No committed config exists for this full identity.
    MissingModelConfig {
        /// Exact missing key, including ordered paint chain.
        key: RegionKey,
    },
}
impl std::fmt::Display for SelectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::fmt::Display for LayerSelectionBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} / {}: {}", self.stage_id, self.module_id, self.error)
    }
}
impl std::error::Error for LayerSelectionBuildError {}
/// Immutable committed snapshot and static bindings for whole-print construction.
pub struct LayerSelectionIndexInput<'a> {
    /// Schema stage roster, normally [`STAGES`].
    pub stage_roster: &'a [StageSpec],
    /// Final promoted global layers.
    pub global_layers: &'a [GlobalLayer],
    /// All bound module-backed Layer modules.
    pub modules: &'a [LayerSelectionModuleInput<'a>],
    /// Committed slices, indexed internally by global layer.
    pub slices: &'a [SliceIR],
    /// Exact model config interner and keys.
    pub region_map: Option<&'a RegionMapIR>,
    /// Optional committed plan; absence is not an empty plan.
    pub support_plan: Option<&'a SupportPlanIR>,
    /// Perimeter products already committed at construction.
    pub perimeters: &'a [PerimeterIR],
}
/// Static binding authority, independent of native/WASM provenance.
pub struct LayerSelectionModuleInput<'a> {
    /// Schema stage identifier.
    pub stage_id: &'a str,
    /// Fully qualified module identifier.
    pub module_id: &'a str,
    /// Manifest-held claims.
    pub claims: &'a [String],
    /// Declared paint semantics; absent or empty is transparent.
    pub paint_semantics: Option<&'a HashSet<String>>,
    /// Already resolved, declared-key-filtered invocation view.
    pub invocation_config: &'a ConfigView,
}
/// Distinguishes an absent optional product from a present-empty one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionalProductPresence {
    /// No product was committed.
    Absent,
    /// A product was committed, possibly empty.
    Present,
}
/// Precomputed eligibility and authority, never delivered payload geometry.
#[derive(Debug)]
pub struct LayerSelection {
    /// Binding stage identity.
    pub stage_id: String,
    /// Whether the binding invokes on this layer, even with empty targets.
    pub invoke: bool,
    /// Request-wide and explicitly non-region config authority.
    pub invocation_config: Arc<ConfigView>,
    /// Build-time optional support-plan presence.
    pub support_plan_presence: OptionalProductPresence,
    /// Selected identities, claims and config authority.
    pub targets: Vec<SelectedLayerTarget>,
    /// Geometry-empty build-time model identity handles, not transport payload.
    pub slice_regions: Vec<SlicedRegion>,
    /// Exact full-origin authority for existing and later perimeter products.
    pub perimeter_configs: HashMap<OriginId, Arc<ConfigView>>,
}
/// One selected model or explicitly tagged non-region context.
#[derive(Debug)]
pub struct SelectedLayerTarget {
    /// Source identity and provenance.
    pub source: LayerTargetSource,
    /// Exact model or explicit invocation authority.
    pub config: Arc<ConfigView>,
    /// All effective held fill roles, or explicit non-region claims.
    pub held_claims: Vec<String>,
    /// Two-sided authored-coloring grant; never granted to non-region work.
    pub authored_coloring: bool,
    /// Build-time host tool authority; not surfaced through filtered module config views.
    pub resolved_role_tool: Option<u32>,
}
/// Model identity is never fabricated for non-region work.
#[derive(Debug, PartialEq, Eq)]
pub enum LayerTargetSource {
    /// Base-key authority with full matching modifier membership.
    Model {
        /// Exact config authority key.
        key: RegionKey,
        /// Base plus merged sources, retaining ordered paint identity.
        members: Vec<RegionKey>,
    },
    /// A qualifying support entry absent from the committed slice.
    SupportCarrier {
        /// Position in the immutable input plan, preserving distinct entries.
        entry_index: usize,
        /// Plan provenance, not a model config key.
        object_id: String,
        /// Plan region provenance.
        region_id: u64,
        /// Producing support family.
        family: String,
        /// Represented demands.
        demand_ids: Vec<String>,
        /// Represented support bodies.
        body_ids: Vec<String>,
        /// Producer provenance labels.
        provenance: Vec<String>,
        /// Anchor provenance; not carrier layer eligibility.
        anchor_layer_index: u32,
    },
    /// Explicit raft layer context for infill/support consumers.
    Raft {
        /// Physical global layer, never inferred from its index.
        global_layer_index: u32,
    },
    /// Scheduled anchored module generation, not a synthetic host closure.
    AnchoredStage {
        /// Physical layer receiving the module invocation.
        anchor_layer_index: u32,
    },
}
/// Whole-print index with one entry per committed layer and bound module.
#[derive(Debug, Default)]
pub struct LayerSelectionIndex {
    selections: HashMap<u32, HashMap<String, LayerSelection>>,
}
impl LayerSelectionIndex {
    /// Expected O(1) lookup without selection reconstruction or product scans.
    /// `None` identifies an unbound pair; consumers must reject missing bound pairs.
    pub fn selection_for(
        &self,
        global_layer_index: u32,
        module_id: &str,
    ) -> Option<&LayerSelection> {
        self.selections.get(&global_layer_index)?.get(module_id)
    }
}
/// Transparent for empty declarations or absent slices; otherwise matches a real chain semantic.
pub fn paint_allows_layer(declared: &HashSet<String>, slice: Option<&SliceIR>) -> bool {
    declared.is_empty()
        || slice.is_none_or(|slice| {
            slice.regions.iter().any(|r| {
                r.variant_chain
                    .iter()
                    .any(|(semantic, _)| declared.contains(semantic))
            })
        })
}
/// Construct all bound selections once, publishing no partial result on failure.
/// Duplicate layer/module pairs violate the immutable plan's construction invariants.
pub fn build_layer_selection_index(
    input: LayerSelectionIndexInput<'_>,
) -> Result<LayerSelectionIndex, LayerSelectionBuildError> {
    let mut index = LayerSelectionIndex::default();
    let slices: HashMap<_, _> = input
        .slices
        .iter()
        .map(|s| (s.global_layer_index, s))
        .collect();
    let mut perimeters: HashMap<u32, Vec<&PerimeterIR>> = HashMap::new();
    for p in input.perimeters {
        perimeters.entry(p.global_layer_index).or_default().push(p);
    }
    for module in input.modules {
        let fail = |error| LayerSelectionBuildError {
            error,
            stage_id: module.stage_id.into(),
            module_id: module.module_id.into(),
        };
        if !input.stage_roster.iter().any(|s| {
            s.stage_id == module.stage_id
                && s.tier_id == slicer_schema::TIER_LAYER
                && !s.wit_export.is_empty()
        }) || !slicer_schema::stage_by_id(module.stage_id)
            .is_some_and(|s| s.tier_id == slicer_schema::TIER_LAYER && !s.wit_export.is_empty())
        {
            return Err(fail(SelectionError::UnknownLayerStage {
                stage_id: module.stage_id.into(),
            }));
        }
        let invocation = Arc::new(module.invocation_config.clone());
        let mut configs = HashMap::new();
        for layer in input.global_layers {
            let selection = build_layer(
                &input,
                module,
                layer,
                slices.get(&layer.index).copied(),
                perimeters
                    .get(&layer.index)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
                &invocation,
                &mut configs,
            )
            .map_err(fail)?;
            assert!(
                index
                    .selections
                    .entry(layer.index)
                    .or_default()
                    .insert(module.module_id.into(), selection)
                    .is_none(),
                "duplicate bound layer/module selection"
            );
        }
    }
    Ok(index)
}
/// Invert `paint_variant_region_id` with `PAINT_VARIANT_REGION_ID_STRIDE` to
/// recover the base id used by `RegionMapIR`; modifier sub-ids and empty chains
/// already occupy map space and remain unchanged.
pub(crate) fn map_space_region_id(
    region_id: u64,
    variant_chain: &[(String, slicer_ir::PaintValue)],
) -> u64 {
    if slicer_ir::modifier_base_region_id(region_id).is_some() || variant_chain.is_empty() {
        region_id
    } else {
        region_id / slicer_core::algos::paint_segmentation::PAINT_VARIANT_REGION_ID_STRIDE
    }
}
fn key_for(layer: u32, region: &SlicedRegion) -> RegionKey {
    RegionKey {
        global_layer_index: layer,
        object_id: region.object_id.clone(),
        region_id: map_space_region_id(region.region_id, &region.variant_chain),
        variant_chain: region.variant_chain.clone(),
    }
}
fn origin(key: &RegionKey) -> OriginId {
    OriginId {
        object_id: key.object_id.clone(),
        region_id: key.region_id,
        variant_chain: key.variant_chain.clone(),
    }
}
fn config_for(
    input: &LayerSelectionIndexInput<'_>,
    module: &LayerSelectionModuleInput<'_>,
    key: &RegionKey,
    cache: &mut HashMap<ConfigId, Arc<ConfigView>>,
) -> Result<Arc<ConfigView>, SelectionError> {
    let map = input
        .region_map
        .ok_or_else(|| SelectionError::MissingModelConfig { key: key.clone() })?;
    let plan = map
        .entries
        .get(key)
        .ok_or_else(|| SelectionError::MissingModelConfig { key: key.clone() })?;
    Ok(cache
        .entry(plan.config)
        .or_insert_with(|| {
            Arc::new(
                slicer_scheduler::execution_plan::project_declared_config_view(
                    map.config_for(key),
                    module.invocation_config.keys().iter().map(String::as_str),
                ),
            )
        })
        .clone())
}
#[allow(clippy::too_many_arguments)]
fn build_layer(
    input: &LayerSelectionIndexInput<'_>,
    module: &LayerSelectionModuleInput<'_>,
    layer: &GlobalLayer,
    slice: Option<&SliceIR>,
    perimeters: &[&PerimeterIR],
    invocation: &Arc<ConfigView>,
    cache: &mut HashMap<ConfigId, Arc<ConfigView>>,
) -> Result<LayerSelection, SelectionError> {
    let mut result = LayerSelection {
        stage_id: module.stage_id.into(),
        invoke: true,
        invocation_config: invocation.clone(),
        support_plan_presence: if input.support_plan.is_some() {
            OptionalProductPresence::Present
        } else {
            OptionalProductPresence::Absent
        },
        targets: Vec::new(),
        slice_regions: Vec::new(),
        perimeter_configs: HashMap::new(),
    };
    if module
        .paint_semantics
        .is_some_and(|p| !paint_allows_layer(p, slice))
    {
        result.invoke = false;
        return Ok(result);
    }
    let candidates = slice
        .map(|s| {
            if module.stage_id == "Layer::Perimeters" {
                crate::marshal::perimeter_source_regions(s)
            } else {
                s.regions.clone()
            }
        })
        .unwrap_or_default();
    for region in candidates {
        let key = key_for(layer.index, &region);
        let config = config_for(input, module, &key, cache)?;
        let resolved = input
            .region_map
            .expect("validated exact map")
            .config_for(&key);
        let active = ActiveRegion {
            object_id: region.object_id.clone(),
            region_id: region.region_id,
            resolved_config: resolved.clone(),
            ..ActiveRegion::default()
        };
        if !module_claims_match_active_region(module.claims, &active) {
            continue;
        }
        let held = resolve_held_claims(
            module.module_id,
            module.claims,
            &FillHolders {
                top: &resolved.top_fill_holder,
                bottom: &resolved.bottom_fill_holder,
                bridge: &resolved.bridge_fill_holder,
                sparse: &resolved.sparse_fill_holder,
            },
        );
        let authored_coloring = crate::marshal::authored_coloring_granted(
            &held,
            &resolved.fill_authored_coloring,
            module.claims,
        );
        let resolved_role_tool = match resolved.extensions.get("extruder") {
            Some(slicer_ir::ConfigValue::Int(tool)) if *tool >= 0 => Some(*tool as u32),
            _ => None,
        };
        let mut members = vec![key.clone()];
        if module.stage_id == "Layer::Perimeters" {
            if let Some(slice) = slice {
                // Modifier children encode the raw painted parent, not its map-space id.
                members.extend(
                    slice
                        .regions
                        .iter()
                        .filter(|r| {
                            r.object_id == region.object_id
                                && r.variant_chain == region.variant_chain
                                && slicer_ir::modifier_base_region_id(r.region_id)
                                    == Some(region.region_id)
                        })
                        .map(|r| key_for(layer.index, r)),
                );
            }
        }
        result
            .perimeter_configs
            .insert(origin(&key), config.clone());
        result.targets.push(SelectedLayerTarget {
            source: LayerTargetSource::Model { key, members },
            config,
            held_claims: held,
            authored_coloring,
            resolved_role_tool,
        });
        // Build-time identity/eligibility only; never retain payload polygons.
        result.slice_regions.push(SlicedRegion {
            object_id: region.object_id,
            region_id: region.region_id,
            variant_chain: region.variant_chain,
            effective_layer_height: region.effective_layer_height,
            ..SlicedRegion::default()
        });
    }
    for p in perimeters {
        for r in &p.regions {
            let key = RegionKey {
                global_layer_index: layer.index,
                object_id: r.object_id.clone(),
                region_id: map_space_region_id(r.region_id, &r.variant_chain),
                variant_chain: r.variant_chain.clone(),
            };
            result
                .perimeter_configs
                .insert(origin(&key), config_for(input, module, &key, cache)?);
        }
    }
    let non_fill_claims = || {
        module
            .claims
            .iter()
            .filter(|c| !FILL_CLAIM_IDS.contains(&c.as_str()) && c.as_str() != "claim:raft-fill")
            .cloned()
            .collect()
    };
    if module.stage_id == "Layer::Support" {
        for (entry_index, entry) in
            eligible_carrier_entries(module.claims, layer.index, input.support_plan, slice)
        {
            result.targets.push(SelectedLayerTarget {
                source: LayerTargetSource::SupportCarrier {
                    entry_index,
                    object_id: entry.object_id.clone(),
                    region_id: entry.region_id,
                    family: entry.family_id.clone(),
                    demand_ids: entry.demand_ids.clone(),
                    body_ids: entry.body_ids.clone(),
                    provenance: entry.provenance.clone(),
                    anchor_layer_index: entry.anchor_layer_index,
                },
                config: invocation.clone(),
                held_claims: non_fill_claims(),
                authored_coloring: false,
                resolved_role_tool: None,
            });
        }
    }
    if layer.is_raft && matches!(module.stage_id, "Layer::Infill" | "Layer::Support") {
        result.targets.push(SelectedLayerTarget {
            source: LayerTargetSource::Raft {
                global_layer_index: layer.index,
            },
            config: invocation.clone(),
            held_claims: module.claims.to_vec(),
            authored_coloring: false,
            resolved_role_tool: None,
        });
    }
    if module.stage_id == "Layer::AnchoredEvents" {
        result.targets.push(SelectedLayerTarget {
            source: LayerTargetSource::AnchoredStage {
                anchor_layer_index: layer.index,
            },
            config: invocation.clone(),
            held_claims: module.claims.to_vec(),
            authored_coloring: false,
            resolved_role_tool: None,
        });
    }
    if module
        .claims
        .iter()
        .any(|c| c.starts_with("support-family:"))
    {
        result.invoke = !result.targets.is_empty();
    }
    Ok(result)
}
fn eligible_carrier_entries<'a>(
    claims: &[String],
    layer: u32,
    plan: Option<&'a SupportPlanIR>,
    slice: Option<&SliceIR>,
) -> Vec<(usize, &'a slicer_ir::SupportPlanEntry)> {
    let families: Vec<_> = claims
        .iter()
        .filter_map(|c| c.strip_prefix("support-family:"))
        .map(|f| slicer_ir::canonical_support_family(Some(f)))
        .collect();
    plan.into_iter()
        .flat_map(|p| p.entries.iter().enumerate())
        .filter(|(_, e)| {
            e.global_layer_index == layer as i32
                && e.decline_reason.is_none()
                && e.roles.iter().any(|r| !r.regions.is_empty())
                && families.contains(&slicer_ir::canonical_support_family(Some(&e.family_id)))
                && !slice.is_some_and(|s| {
                    s.regions
                        .iter()
                        .any(|r| r.object_id == e.object_id && r.region_id == e.region_id)
                })
        })
        .collect()
}
pub(crate) fn support_carrier_regions(
    claims: &[String],
    layer: u32,
    plan: Option<&SupportPlanIR>,
    slice: Option<&SliceIR>,
) -> Vec<SlicedRegion> {
    let mut carriers: Vec<SlicedRegion> = Vec::new();
    for (_, e) in eligible_carrier_entries(claims, layer, plan, slice) {
        if carriers
            .iter()
            .any(|c| c.object_id == e.object_id && c.region_id == e.region_id)
        {
            continue;
        }
        carriers.push(SlicedRegion {
            object_id: e.object_id.clone(),
            region_id: e.region_id,
            effective_layer_height: slice
                .and_then(|s| {
                    s.regions
                        .iter()
                        .find(|r| r.object_id == e.object_id)
                        .map(|r| r.effective_layer_height)
                })
                .unwrap_or(0.0),
            ..SlicedRegion::default()
        });
    }
    carriers.sort_by(|a, b| {
        a.object_id
            .cmp(&b.object_id)
            .then(a.region_id.cmp(&b.region_id))
    });
    carriers
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::{ConfigValue, PaintValue, RegionPlan, ResolvedConfig};

    #[test]
    fn map_space_region_id_inverts_paint_but_preserves_modifiers_and_empty_chains() {
        let chain = vec![("material".into(), PaintValue::ToolIndex(2))];
        assert_eq!(map_space_region_id(3, &chain), 0);
        assert_eq!(map_space_region_id(11_000_003, &chain), 11);
        assert_eq!(map_space_region_id(11_000_003, &[]), 11_000_003);
        let child_id = slicer_ir::modifier_sub_region_id(11_000_003, "model", &[]);
        assert_eq!(map_space_region_id(child_id, &chain), child_id);
        assert_eq!(map_space_region_id(child_id, &[]), child_id);
    }

    #[test]
    fn model_identity_and_non_region_authority() {
        let invocation =
            ConfigView::from_map(HashMap::from([("marker".into(), ConfigValue::Int(99))]));
        let claims = vec![
            "claim:top-fill".into(),
            "claim:sparse-fill".into(),
            "claim:authored-coloring".into(),
        ];
        let supported: Vec<_> = STAGES
            .iter()
            .filter(|s| s.tier_id == slicer_schema::TIER_LAYER && !s.wit_export.is_empty())
            .collect();
        let modules: Vec<_> = supported
            .iter()
            // exhaustive: borrowed binding fixture must specify all static selection authority.
            .map(|s| LayerSelectionModuleInput {
                stage_id: s.stage_id,
                module_id: s.stage_id,
                claims: &claims,
                paint_semantics: None,
                invocation_config: &invocation,
            })
            .collect();
        let layers = [
            GlobalLayer {
                index: 0,
                is_raft: true,
                ..GlobalLayer::default()
            },
            GlobalLayer {
                index: 7,
                ..GlobalLayer::default()
            },
        ];
        // Expectations are authored full identities, not recovered from the projection.
        let keys = [
            RegionKey {
                global_layer_index: 0,
                object_id: "model".into(),
                region_id: 11,
                variant_chain: vec![
                    ("material".into(), PaintValue::ToolIndex(1)),
                    ("density".into(), PaintValue::Scalar(0.5)),
                ],
            },
            RegionKey {
                global_layer_index: 0,
                object_id: "model".into(),
                region_id: 11,
                variant_chain: vec![("material".into(), PaintValue::ToolIndex(2))],
            },
        ];
        let slices = [SliceIR {
            regions: keys
                .iter()
                .rev()
                .map(|k| SlicedRegion {
                    object_id: "model".into(),
                    region_id: slicer_core::algos::paint_segmentation::paint_variant_region_id(
                        k.region_id,
                        &k.variant_chain,
                    ),
                    variant_chain: k.variant_chain.clone(),
                    ..SlicedRegion::default()
                })
                .collect(),
            ..SliceIR::default()
        }];
        let mut map = RegionMapIR::default();
        for (key, marker) in keys.iter().zip([21, 42]) {
            let id = map.intern_config(ResolvedConfig {
                top_fill_holder: "Layer::Infill".into(),
                sparse_fill_holder: "Layer::Infill".into(),
                fill_authored_coloring: vec!["claim:top-fill".into()],
                extensions: std::collections::BTreeMap::from([
                    ("marker".into(), ConfigValue::Int(marker)),
                    ("hidden".into(), ConfigValue::Int(-1)),
                ]),
                ..ResolvedConfig::default()
            });
            map.entries.insert(
                key.clone(),
                RegionPlan {
                    config: id,
                    ..RegionPlan::default()
                },
            );
        }
        let empty_plan = SupportPlanIR::default();
        let build = |region_map, support_plan| {
            // exhaustive: whole-print snapshot explicitly supplies every authority channel.
            build_layer_selection_index(LayerSelectionIndexInput {
                stage_roster: STAGES,
                global_layers: &layers,
                modules: &modules,
                slices: &slices,
                region_map,
                support_plan,
                perimeters: &[],
            })
        };
        let index = build(Some(&map), Some(&empty_plan)).unwrap();
        for layer in &layers {
            for module in &modules {
                let first = index
                    .selection_for(layer.index, module.module_id)
                    .expect("every bound pair");
                let second = index.selection_for(layer.index, module.module_id).unwrap();
                assert!(std::ptr::eq(first, second));
                assert_eq!(first.stage_id, module.stage_id);
                assert_eq!(
                    first.support_plan_presence,
                    OptionalProductPresence::Present
                );
                assert_eq!(first.invocation_config.get_int("marker"), Some(99));
                assert!(
                    first.invoke,
                    "transparent non-support stages invoke even empty"
                );
                for (key, expected) in keys.iter().zip([21, 42]) {
                    if layer.index != 0 {
                        continue;
                    }
                    let target = first.targets.iter().find(|t| matches!(&t.source, LayerTargetSource::Model { key: actual, .. } if actual == key)).unwrap();
                    assert_eq!(target.config.get_int("marker"), Some(expected));
                    assert!(target.config.get("hidden").is_none());
                    assert!(
                        matches!(&target.source, LayerTargetSource::Model { members, .. } if members == &vec![key.clone()])
                    );
                    assert_eq!(
                        first.perimeter_configs[&origin(key)].get_int("marker"),
                        Some(expected)
                    );
                    if module.stage_id == "Layer::Infill" {
                        assert_eq!(
                            target.held_claims,
                            vec!["claim:top-fill", "claim:sparse-fill"]
                        );
                        assert!(target.authored_coloring);
                    } else {
                        assert!(!target.authored_coloring);
                    }
                }
                for target in &first.targets {
                    if !matches!(target.source, LayerTargetSource::Model { .. }) {
                        assert_eq!(target.config.get_int("marker"), Some(99));
                        assert!(!target.authored_coloring);
                    }
                }
            }
        }
        assert!(index.selection_for(0, "unbound").is_none());
        assert!(index.selection_for(99, modules[0].module_id).is_none());
        assert!(index
            .selection_for(0, "Layer::Infill")
            .unwrap()
            .targets
            .iter()
            .any(|t| t.source
                == LayerTargetSource::Raft {
                    global_layer_index: 0
                }));
        assert!(index
            .selection_for(7, "Layer::AnchoredEvents")
            .unwrap()
            .targets
            .iter()
            .any(|t| t.source
                == LayerTargetSource::AnchoredStage {
                    anchor_layer_index: 7
                }));
        assert!(index
            .selection_for(7, "Layer::Infill")
            .unwrap()
            .targets
            .is_empty());
        let absent = build(Some(&map), None).unwrap();
        assert_eq!(
            absent
                .selection_for(7, "Layer::Support")
                .unwrap()
                .support_plan_presence,
            OptionalProductPresence::Absent
        );
        let error = build(None, None).unwrap_err();
        assert_eq!(error.stage_id, modules[0].stage_id);
        assert_eq!(error.module_id, modules[0].module_id);
        assert_eq!(
            error.error,
            SelectionError::MissingModelConfig {
                key: keys[1].clone()
            }
        );
        let mut incomplete = map.clone();
        incomplete.entries.remove(&keys[1]);
        assert_eq!(
            build(Some(&incomplete), None).unwrap_err().error,
            SelectionError::MissingModelConfig {
                key: keys[1].clone()
            }
        );
        for stage in STAGES
            .iter()
            .filter(|s| s.tier_id != slicer_schema::TIER_LAYER || s.wit_export.is_empty())
        {
            // exhaustive: rejected-stage control explicitly defines every borrowed binding field.
            let bad = [LayerSelectionModuleInput {
                stage_id: stage.stage_id,
                module_id: "bad",
                claims: &[],
                paint_semantics: None,
                invocation_config: &invocation,
            }];
            // exhaustive: stage rejection control has no committed model or support products.
            let error = build_layer_selection_index(LayerSelectionIndexInput {
                stage_roster: STAGES,
                global_layers: &layers,
                modules: &bad,
                slices: &[],
                region_map: None,
                support_plan: None,
                perimeters: &[],
            })
            .unwrap_err();
            assert_eq!(
                error.error,
                SelectionError::UnknownLayerStage {
                    stage_id: stage.stage_id.into()
                }
            );
        }
        let paint = HashSet::from(["absent-semantic".into()]);
        // exhaustive: paint-gate control explicitly specifies static binding authority.
        let gated = [LayerSelectionModuleInput {
            stage_id: "Layer::Infill",
            module_id: "gated",
            claims: &[],
            paint_semantics: Some(&paint),
            invocation_config: &invocation,
        }];
        // exhaustive: paint rejection must not demand missing model authority.
        let gated_index = build_layer_selection_index(LayerSelectionIndexInput {
            stage_roster: STAGES,
            global_layers: &layers,
            modules: &gated,
            slices: &slices,
            region_map: None,
            support_plan: None,
            perimeters: &[],
        })
        .unwrap();
        assert!(!gated_index.selection_for(0, "gated").unwrap().invoke);
        assert!(gated_index
            .selection_for(0, "gated")
            .unwrap()
            .targets
            .is_empty());
        assert!(gated_index.selection_for(7, "gated").unwrap().invoke);
        assert!(paint_allows_layer(&HashSet::new(), Some(&slices[0])));
        assert!(paint_allows_layer(
            &HashSet::from(["material".into()]),
            Some(&slices[0])
        ));

        let support_claims = vec!["support-family:traditional".into()];
        // exhaustive: support-family fixture specifies all borrowed module metadata.
        let support = [LayerSelectionModuleInput {
            stage_id: "Layer::Support",
            module_id: "support",
            claims: &support_claims,
            paint_semantics: None,
            invocation_config: &invocation,
        }];
        // exhaustive: carrier provenance fixture specifies all unbound SupportPlanEntry fields.
        let entry = slicer_ir::SupportPlanEntry {
            global_layer_index: 7,
            object_id: "air".into(),
            region_id: slicer_core::algos::paint_segmentation::paint_variant_region_id(
                55,
                &[("material".into(), PaintValue::ToolIndex(3))],
            ),
            family_id: "traditional".into(),
            demand_ids: vec!["demand".into()],
            body_ids: vec!["body".into()],
            anchor_layer_index: 3,
            anchor_z: 0,
            roles: vec![slicer_ir::SupportPlanRoleRegion {
                role: slicer_ir::SupportPlanRole::SupportBody,
                regions: vec![slicer_ir::ExPolygon::default()],
            }],
            skeleton: None,
            capabilities: vec![],
            provenance: vec!["planner".into()],
            decline_reason: None,
        };
        let mut wrong_layer = entry.clone();
        wrong_layer.global_layer_index = 3;
        wrong_layer.anchor_layer_index = 7;
        let mut second = entry.clone();
        second.body_ids = vec!["second-body".into()];
        let plan = SupportPlanIR {
            entries: vec![entry, wrong_layer, second],
            ..SupportPlanIR::default()
        };
        // exhaustive: carrier snapshot explicitly has no model config or slice.
        let support_index = build_layer_selection_index(LayerSelectionIndexInput {
            stage_roster: STAGES,
            global_layers: &layers,
            modules: &support,
            slices: &[],
            region_map: None,
            support_plan: Some(&plan),
            perimeters: &[],
        })
        .unwrap();
        let selected = support_index.selection_for(7, "support").unwrap();
        assert!(selected.invoke);
        assert_eq!(selected.targets.len(), 2);
        for (target, (entry_index, body)) in selected
            .targets
            .iter()
            .zip([(0, "body"), (2, "second-body")])
        {
            assert_eq!(
                target.source,
                LayerTargetSource::SupportCarrier {
                    entry_index,
                    object_id: "air".into(),
                    region_id: 55_000_004,
                    family: "traditional".into(),
                    demand_ids: vec!["demand".into()],
                    body_ids: vec![body.into()],
                    provenance: vec!["planner".into()],
                    anchor_layer_index: 3
                }
            );
            assert_eq!(target.config.get_int("marker"), Some(99));
            assert_eq!(target.held_claims, support_claims);
            assert!(!target.authored_coloring);
        }
        assert_eq!(
            support_carrier_regions(&support_claims, 7, Some(&plan), None).len(),
            1
        );
        assert!(selected.perimeter_configs.is_empty());
        let present_slice = SliceIR {
            global_layer_index: 7,
            regions: vec![SlicedRegion {
                object_id: "air".into(),
                region_id: slicer_core::algos::paint_segmentation::paint_variant_region_id(
                    55,
                    &[("material".into(), PaintValue::ToolIndex(3))],
                ),
                variant_chain: vec![("material".into(), PaintValue::ToolIndex(3))],
                ..SlicedRegion::default()
            }],
            ..SliceIR::default()
        };
        assert!(
            support_carrier_regions(&support_claims, 7, Some(&plan), Some(&present_slice))
                .is_empty(),
            "identity presence excludes carriers even with empty polygons"
        );
        // exhaustive: absent-support control specifies every optional product channel.
        let no_support = build_layer_selection_index(LayerSelectionIndexInput {
            stage_roster: STAGES,
            global_layers: &layers[1..],
            modules: &support,
            slices: &[],
            region_map: None,
            support_plan: None,
            perimeters: &[],
        })
        .unwrap();
        assert!(!no_support.selection_for(7, "support").unwrap().invoke);

        let parent_raw_id = slicer_core::algos::paint_segmentation::paint_variant_region_id(
            keys[0].region_id,
            &keys[0].variant_chain,
        );
        let child_id = slicer_ir::modifier_sub_region_id(parent_raw_id, "model", &[]);
        let child_key = RegionKey {
            region_id: child_id,
            ..keys[0].clone()
        };
        let merged_slices = [SliceIR {
            regions: vec![
                SlicedRegion {
                    object_id: "model".into(),
                    region_id: parent_raw_id,
                    variant_chain: keys[0].variant_chain.clone(),
                    polygons: vec![slicer_ir::ExPolygon {
                        contour: slicer_ir::Polygon {
                            points: vec![
                                slicer_ir::Point2::from_mm(0.0, 0.0),
                                slicer_ir::Point2::from_mm(1.0, 0.0),
                                slicer_ir::Point2::from_mm(0.0, 1.0),
                            ],
                        },
                        ..slicer_ir::ExPolygon::default()
                    }],
                    ..SlicedRegion::default()
                },
                SlicedRegion {
                    object_id: "model".into(),
                    region_id: child_id,
                    variant_chain: keys[0].variant_chain.clone(),
                    ..SlicedRegion::default()
                },
                SlicedRegion {
                    object_id: "model".into(),
                    region_id: slicer_ir::MODIFIER_FOOTPRINT_REGION_ID,
                    ..SlicedRegion::default()
                },
            ],
            ..SliceIR::default()
        }];
        // exhaustive: perimeter binding specifies all borrowed selection authority.
        let perimeter_modules = [LayerSelectionModuleInput {
            stage_id: "Layer::Perimeters",
            module_id: "walls",
            claims: &[],
            paint_semantics: None,
            invocation_config: &invocation,
        }];
        // exhaustive: merged membership fixture deliberately supplies only base-key authority.
        let merged = build_layer_selection_index(LayerSelectionIndexInput {
            stage_roster: STAGES,
            global_layers: &layers,
            modules: &perimeter_modules,
            slices: &merged_slices,
            region_map: Some(&map),
            support_plan: None,
            perimeters: &[],
        })
        .unwrap();
        let selection = merged.selection_for(0, "walls").unwrap();
        assert_eq!(selection.targets.len(), 1);
        assert_eq!(
            selection.targets[0].source,
            LayerTargetSource::Model {
                key: keys[0].clone(),
                members: vec![keys[0].clone(), child_key]
            }
        );
        assert_eq!(selection.targets[0].config.get_int("marker"), Some(21));
        assert!(selection.slice_regions[0].polygons.is_empty());

        let extra_key = RegionKey {
            region_id: 77,
            ..keys[0].clone()
        };
        let perimeters = [PerimeterIR {
            regions: vec![slicer_ir::PerimeterRegion {
                object_id: "model".into(),
                region_id: slicer_core::algos::paint_segmentation::paint_variant_region_id(
                    extra_key.region_id,
                    &extra_key.variant_chain,
                ),
                variant_chain: keys[0].variant_chain.clone(),
                ..slicer_ir::PerimeterRegion::default()
            }],
            ..PerimeterIR::default()
        }];
        // exhaustive: committed perimeter rejection control specifies missing exact authority.
        let perimeter_error = build_layer_selection_index(LayerSelectionIndexInput {
            stage_roster: STAGES,
            global_layers: &layers,
            modules: &perimeter_modules,
            slices: &[],
            region_map: Some(&map),
            support_plan: None,
            perimeters: &perimeters,
        })
        .unwrap_err();
        assert_eq!(
            perimeter_error.error,
            SelectionError::MissingModelConfig {
                key: extra_key.clone()
            }
        );
        let id = map.intern_config(ResolvedConfig {
            extensions: std::collections::BTreeMap::from([("marker".into(), ConfigValue::Int(73))]),
            ..ResolvedConfig::default()
        });
        map.entries.insert(
            extra_key.clone(),
            RegionPlan {
                config: id,
                ..RegionPlan::default()
            },
        );
        // exhaustive: committed perimeter snapshot explicitly supplies every authority channel.
        let perimeter_index = build_layer_selection_index(LayerSelectionIndexInput {
            stage_roster: STAGES,
            global_layers: &layers,
            modules: &perimeter_modules,
            slices: &[],
            region_map: Some(&map),
            support_plan: None,
            perimeters: &perimeters,
        })
        .unwrap();
        assert_eq!(
            perimeter_index
                .selection_for(0, "walls")
                .unwrap()
                .perimeter_configs[&origin(&extra_key)]
                .get_int("marker"),
            Some(73)
        );
    }
}
