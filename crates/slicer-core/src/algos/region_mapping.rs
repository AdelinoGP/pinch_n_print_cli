// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/PrintRegion.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Pure region-mapping kernel — IR-only, no scheduler or runtime deps.
//!
//! Compiles a [`RegionMapIR`] from a committed [`LayerPlanIR`] and a
//! [`RegionMappingPlanProjection`] that supplies the precomputed
//! `(StageId, Vec<ModuleInvocation>)` pairs the slicer-runtime wrapper has
//! already extracted from the scheduler's plan.
//!
//! Scope: produce one `RegionPlan` per `(layer, region)` pair, snapshotting
//! the region's `ResolvedConfig` and listing the topo-sorted module
//! invocations. See docs/04_host_scheduler.md §"RegionMapIR Compilation"
//! and IR 5 in docs/02_ir_schemas.md.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use slicer_ir::slice_ir::AggregatedRegionSplitEntry;
use slicer_ir::{
    is_modifier_namespace_id, modifier_sub_region_id, modifier_sub_region_id_fits,
    region_split_registry::enumerate_canonical_chains, ConfigValue, LayerPlanIR, ModifierVolume,
    ModuleInvocation, ObjectId, ObjectMesh, PaintSemantic, PaintValue, RegionKey, RegionMapIR,
    RegionPlan, ResolvedConfig, StageId,
};

use crate::algos::paint_segmentation::paint_variant_region_id;

/// Default cap on `RegionMapIR` entry count per docs/04_host_scheduler.md.
pub use slicer_ir::DEFAULT_REGION_MAP_CAP;

/// Borrow projection of the scheduler plan fields consumed by the
/// region-mapping kernel. The slicer-runtime wrapper precomputes this
/// from the scheduler's `per_layer_stages` and `postpass_stages` fields
/// so the kernel remains IR-only and `slicer-core` does not acquire a
/// `slicer-scheduler` dep.
pub struct RegionMappingPlanProjection<'a> {
    /// Precomputed `(stage_id, module_invocations)` pairs, chaining
    /// `per_layer_stages` then `postpass_stages` in that order.
    pub stage_invocations: &'a [(StageId, Vec<ModuleInvocation>)],
}

/// Top contributing module/object for overflow diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopContributor {
    /// Object that contributed the most regions.
    pub object_id: String,
    /// Number of regions contributed by this object.
    pub region_count: usize,
    /// Number of layers this object appears on.
    pub layer_count: usize,
}

/// Structured region-mapping failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegionMappingError {
    /// `RegionMapIR` entry count exceeded the configured cap.
    CapExceeded {
        /// Computed entry count.
        entry_count: usize,
        /// Configured cap.
        cap: usize,
        /// Top contributing objects sorted by region_count descending.
        top_contributors: Vec<TopContributor>,
        /// Remediation hint.
        remediation: String,
    },
    /// `LayerPlanIR` contained duplicate `(layer_index, object_id, region_id)` keys.
    DuplicateRegionKey {
        /// The offending key.
        key: RegionKey,
    },
    /// A parent region id is too large for the modifier namespace payload.
    ModifierParentRegionIdOutOfRange {
        /// The parent region key whose modifier child could not be minted.
        key: RegionKey,
    },
    /// A `PaintValue::Scalar(_)` was encountered for a semantic that was
    /// declared in `aggregated_region_split` (i.e. opted into the region-split
    /// cross-product). Scalars cannot drive a discrete variant axis — the
    /// semantic must emit `Flag`/`ToolIndex`/`Custom` values when participating
    /// in region split. AC-N3.
    ScalarInRegionSplitFacetValue {
        /// Object whose `paint_data` carried the offending scalar.
        object_id: ObjectId,
        /// Semantic name (as declared in `[[region_split]]`) carrying the scalar.
        semantic: String,
        /// The offending scalar value, stored as `f32::to_bits()` so the
        /// enclosing enum can derive `Eq`. Reconstruct the float via
        /// [`RegionMappingError::scalar`] or `f32::from_bits(scalar_bits)`.
        scalar_bits: u32,
    },
}

impl RegionMappingError {
    /// Reconstructs the offending Scalar paint value when the error is
    /// `ScalarInRegionSplitFacetValue`. Returns `f32::NAN` for other variants
    /// (callers should match on the variant first).
    pub fn scalar(&self) -> f32 {
        match self {
            Self::ScalarInRegionSplitFacetValue { scalar_bits, .. } => f32::from_bits(*scalar_bits),
            _ => f32::NAN,
        }
    }
}

impl std::fmt::Display for RegionMappingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CapExceeded {
                entry_count,
                cap,
                top_contributors,
                remediation,
            } => {
                write!(
                    f,
                    "region map has {entry_count} entries, exceeding cap of {cap}; "
                )?;
                if !top_contributors.is_empty() {
                    let contribs: Vec<String> = top_contributors
                        .iter()
                        .map(|c| {
                            format!(
                                "{}({} regions, {} layers)",
                                c.object_id, c.region_count, c.layer_count
                            )
                        })
                        .collect();
                    write!(f, "top contributors: {}; ", contribs.join(", "))?;
                }
                write!(f, "{remediation}")
            }
            Self::DuplicateRegionKey { key } => write!(
                f,
                "layer plan has duplicate active region (layer={}, object='{}', region={})",
                key.global_layer_index, key.object_id, key.region_id
            ),
            Self::ModifierParentRegionIdOutOfRange { key } => write!(
                f,
                "layer plan region cannot mint modifier sub-region id (layer={}, object='{}', region={})",
                key.global_layer_index, key.object_id, key.region_id
            ),
            Self::ScalarInRegionSplitFacetValue {
                object_id,
                semantic,
                scalar_bits,
            } => {
                let scalar = f32::from_bits(*scalar_bits);
                write!(
                    f,
                    "PaintValue::Scalar({scalar}) encountered for region-split semantic \
                     '{semantic}' on object '{object_id}'; region-split semantics must emit \
                     Flag/ToolIndex/Custom values (scalars cannot drive a discrete variant axis)"
                )
            }
        }
    }
}

impl std::error::Error for RegionMappingError {}

/// Build the `CapExceeded` error for a map that would exceed `cap` at
/// `next_entry_count` entries, naming the top contributing objects.
///
/// Shared by the per-insert guards around the variant-chain expansion and the
/// ticket-18 modifier sub-region minting (both run after the whole-plan
/// precheck, which stays where it is; a per-insert check can still trip when
/// cross-product or sub-region expansion pushes past the precheck's count).
fn cap_exceeded(
    entries: &HashMap<RegionKey, RegionPlan>,
    cap: usize,
    next_entry_count: usize,
) -> RegionMappingError {
    let mut sorted: Vec<(String, usize)> = entries
        .keys()
        .fold(HashMap::<String, usize>::new(), |mut acc, k| {
            *acc.entry(k.object_id.clone()).or_insert(0) += 1;
            acc
        })
        .into_iter()
        .collect();
    sorted.sort_by_key(|b| std::cmp::Reverse(b.1));
    let layer_count = entries
        .keys()
        .map(|k| k.global_layer_index)
        .collect::<HashSet<_>>()
        .len();
    let top_contributors: Vec<TopContributor> = sorted
        .into_iter()
        .take(5)
        .map(|(object_id, region_count)| TopContributor {
            object_id,
            region_count,
            layer_count,
        })
        .collect();
    RegionMappingError::CapExceeded {
        entry_count: next_entry_count,
        cap,
        top_contributors,
        remediation: "reduce region granularity, raise cap, or split job".to_string(),
    }
}

const RESOLVED_TARGET_PREFIX: &str = "\0resolved-target:";

fn append_key_part(key: &mut String, value: &str) {
    use std::fmt::Write as _;
    let _ = write!(key, "{}:{value}", value.len());
}

fn resolved_target_key(
    object_id: &str,
    modifier_ids: &[String],
    paint_semantics: &[String],
    tool_index: Option<u32>,
) -> String {
    if modifier_ids.is_empty() && paint_semantics.is_empty() && tool_index.is_none() {
        return object_id.to_owned();
    }
    let mut key = RESOLVED_TARGET_PREFIX.to_owned();
    append_key_part(&mut key, object_id);
    key.push('|');
    for modifier_id in modifier_ids {
        append_key_part(&mut key, modifier_id);
        key.push(',');
    }
    key.push('|');
    for semantic in paint_semantics {
        append_key_part(&mut key, semantic);
        key.push(',');
    }
    key.push('|');
    if let Some(tool_index) = tool_index {
        use std::fmt::Write as _;
        let _ = write!(key, "{tool_index}");
    }
    key
}

fn ordered_modifier_ids(modifier_volumes: &[ModifierVolume]) -> Vec<String> {
    let mut order: Vec<usize> = (0..modifier_volumes.len()).collect();
    order.sort_by_key(|&index| (modifier_volumes[index].priority, std::cmp::Reverse(index)));
    order
        .into_iter()
        .map(|index| modifier_volumes[index].id.clone())
        .collect()
}

fn resolved_target_config<'a>(
    resolved_configs: &'a BTreeMap<String, ResolvedConfig>,
    object_id: &str,
    modifier_volumes: &[ModifierVolume],
    chain: &[(String, PaintValue)],
    tool_index: Option<u32>,
) -> Option<&'a ResolvedConfig> {
    let modifier_ids = ordered_modifier_ids(modifier_volumes);
    let mut paint_semantics: Vec<String> =
        chain.iter().map(|(semantic, _)| semantic.clone()).collect();
    paint_semantics.sort();
    paint_semantics.dedup();
    resolved_configs.get(&resolved_target_key(
        object_id,
        &modifier_ids,
        &paint_semantics,
        tool_index,
    ))
}

/// Packet 132 (AC-4) — bind a modifier's config delta to the modifier's
/// minted sub-region `RegionKey` instead of stamping the whole object.
///
/// This returns a `region_id → config` map that keeps the parent region's
/// config untouched while binding a pre-resolved modifier scope stack to a
/// *separate* config keyed by the minted sub-region id.
///
/// Concretely, given a base `infill_density = 0.15` and a modifier volume
/// carrying `infill_density = 0.40`:
/// * `map[base_region_id]`  → `base_config` (0.15, unchanged)
/// * `map[sub_region_id]`   → the runtime-pre-resolved modifier target (0.40)
fn stamp_pre_resolved_sub_region_configs(
    base_config: ResolvedConfig,
    mut sub_config: ResolvedConfig,
    modifier_volumes: &[ModifierVolume],
    base_region_id: u64,
    sub_region_id: u64,
) -> BTreeMap<u64, ResolvedConfig> {
    let mut order: Vec<usize> = (0..modifier_volumes.len()).collect();
    order.sort_by_key(|&index| (modifier_volumes[index].priority, std::cmp::Reverse(index)));
    for index in order {
        let modifier = &modifier_volumes[index];
        match modifier.kind() {
            slicer_ir::ModifierKind::ParameterModifier => {}
            slicer_ir::ModifierKind::NegativePart => {}
            slicer_ir::ModifierKind::SupportEnforcer => continue,
            slicer_ir::ModifierKind::SupportBlocker => continue,
        }
        for (key, value) in &modifier.config_delta.fields {
            if key == "subtype"
                || matches!(value, ConfigValue::String(value) if value.is_empty())
                || matches!(value, ConfigValue::List(value) if value.is_empty())
            {
                continue;
            }
            sub_config.extensions.insert(key.clone(), value.clone());
        }
    }

    let mut map = BTreeMap::new();
    map.insert(base_region_id, base_config);
    map.insert(sub_region_id, sub_config);
    map
}

/// Compatibility adapter for callers that exercise modifier stamping without
/// the runtime's typed scope resolver. Production region mapping uses
/// [`stamp_pre_resolved_sub_region_configs`] instead.
pub fn stamp_modifier_sub_region_configs(
    base_config: ResolvedConfig,
    base_region_id: u64,
    sub_region_id: u64,
    modifier_volumes: &[ModifierVolume],
) -> BTreeMap<u64, ResolvedConfig> {
    let mut sub_config = base_config.clone();
    let mut order: Vec<usize> = (0..modifier_volumes.len()).collect();
    order.sort_by_key(|&index| (modifier_volumes[index].priority, std::cmp::Reverse(index)));
    for index in order {
        let modifier = &modifier_volumes[index];
        match modifier.kind() {
            slicer_ir::ModifierKind::ParameterModifier => {}
            slicer_ir::ModifierKind::NegativePart => {}
            slicer_ir::ModifierKind::SupportEnforcer => continue,
            slicer_ir::ModifierKind::SupportBlocker => continue,
        }
        for (key, value) in &modifier.config_delta.fields {
            if key == "subtype"
                || matches!(value, ConfigValue::String(value) if value.is_empty())
                || matches!(value, ConfigValue::List(value) if value.is_empty())
            {
                continue;
            }
            sub_config.extensions.insert(key.clone(), value.clone());
        }
    }
    stamp_pre_resolved_sub_region_configs(
        base_config,
        sub_config,
        &[],
        base_region_id,
        sub_region_id,
    )
}

/// Slice and group parameter-modifier footprints for one object/layer pair.
///
/// Modifiers with the same footprint share one sub-region, so their deltas are
/// resolved together by [`stamp_modifier_sub_region_configs`]. Support
/// enforcer/blocker volumes stay on the paint annotation path and are not part
/// of this map.
fn modifier_footprint_groups(
    object: &ObjectMesh,
    layer_z: f32,
) -> Vec<(Vec<slicer_ir::ExPolygon>, Vec<ModifierVolume>)> {
    let mut groups: Vec<(Vec<slicer_ir::ExPolygon>, Vec<ModifierVolume>)> = Vec::new();
    for modifier in &object.modifier_volumes {
        match modifier.kind() {
            slicer_ir::ModifierKind::ParameterModifier => {}
            slicer_ir::ModifierKind::NegativePart => {}
            slicer_ir::ModifierKind::SupportEnforcer => continue,
            slicer_ir::ModifierKind::SupportBlocker => continue,
        }
        if modifier.mesh.vertices.is_empty() || modifier.mesh.indices.is_empty() {
            continue;
        }
        let footprint = crate::slice_mesh_ex(&modifier.mesh, &[layer_z])
            .into_iter()
            .next()
            .unwrap_or_default();
        if footprint.is_empty() {
            continue;
        }
        if let Some((_, modifiers)) = groups
            .iter_mut()
            .find(|(existing, _)| existing == &footprint)
        {
            modifiers.push(modifier.clone());
        } else {
            groups.push((footprint, vec![modifier.clone()]));
        }
    }
    groups
}

/// Serialize a `PaintSemantic` to its namespace key string for sort ordering.
///
/// Built-in variants serialize as `material`/`fuzzy_skin`/`support_enforcer`/
/// `support_blocker`; `Custom(s)` serializes as the raw `s`.
/// Inlined here to avoid a `slicer-scheduler` dep in `slicer-core`.
fn paint_semantic_namespace_key(s: &PaintSemantic) -> String {
    match s {
        PaintSemantic::Material => "material".to_string(),
        PaintSemantic::FuzzySkin => "fuzzy_skin".to_string(),
        PaintSemantic::SupportEnforcer => "support_enforcer".to_string(),
        PaintSemantic::SupportBlocker => "support_blocker".to_string(),
        PaintSemantic::Custom(name) => name.clone(),
    }
}

/// Canonical comparator for `PaintValue` per P93 requirements.md.
///
/// Ordering: `Flag(false) < Flag(true) < ToolIndex(0) < ToolIndex(1) < … < Custom(s_lex)`.
/// `Scalar` is rejected upstream (AC-N3) and never reaches this comparator.
fn paint_value_canonical_cmp(a: &PaintValue, b: &PaintValue) -> std::cmp::Ordering {
    fn discriminant_rank(v: &PaintValue) -> u8 {
        match v {
            PaintValue::Flag(_) => 0,
            PaintValue::Scalar(_) => 1,
            PaintValue::ToolIndex(_) => 2,
            PaintValue::Custom(_) => 3,
        }
    }
    match (a, b) {
        (PaintValue::Flag(x), PaintValue::Flag(y)) => x.cmp(y),
        (PaintValue::ToolIndex(x), PaintValue::ToolIndex(y)) => x.cmp(y),
        (PaintValue::Custom(x), PaintValue::Custom(y)) => x.cmp(y),
        (PaintValue::Scalar(x), PaintValue::Scalar(y)) => x.to_bits().cmp(&y.to_bits()),
        _ => discriminant_rank(a).cmp(&discriminant_rank(b)),
    }
}

/// Wrapper around `PaintValue` that implements `Ord` via the canonical
/// comparator so we can de-dup with `BTreeSet`. The wrapper exists purely
/// to satisfy the `BTreeSet` `Ord` bound; the underlying `PaintValue`'s
/// `Hash`/`Eq` semantics (used by `RegionKey`) are unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
struct OrdPaintValue(PaintValue);

impl PartialOrd for OrdPaintValue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for OrdPaintValue {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        paint_value_canonical_cmp(&self.0, &other.0)
    }
}

/// Scan each object's `paint_data` for distinct `PaintValue`s per opted-in
/// region-split semantic (AC-2 / AC-N3).
///
/// Only semantics that appear as keys in `aggregated` are recorded — semantics
/// outside the region-split registry do not drive variant expansion. Objects
/// with no paint data, or no paint values matching `aggregated` keys, return an
/// empty inner map (which the chain enumerator treats as "no axes → empty chain").
///
/// Per-value ordering follows the canonical comparator in
/// `paint_value_canonical_cmp` so the chain enumeration is deterministic.
///
/// AC-N3: encountering `PaintValue::Scalar(_)` for an opted-in semantic is a
/// hard error (`RegionMappingError::ScalarInRegionSplitFacetValue`).
fn scan_paint_data(
    objects: &[ObjectMesh],
    aggregated: &BTreeMap<String, AggregatedRegionSplitEntry>,
) -> Result<HashMap<ObjectId, HashMap<String, Vec<PaintValue>>>, RegionMappingError> {
    let mut out: HashMap<ObjectId, HashMap<String, Vec<PaintValue>>> = HashMap::new();

    for obj in objects {
        let Some(paint_data) = obj.paint_data.as_ref() else {
            continue;
        };

        // Per-object accumulator: semantic-name → de-dup'd values
        // (BTreeSet keyed by the canonical comparator).
        let mut per_semantic: BTreeMap<String, BTreeSet<OrdPaintValue>> = BTreeMap::new();

        for layer in &paint_data.layers {
            let semantic_name = paint_semantic_namespace_key(&layer.semantic);
            // Skip semantics that did not opt into region split.
            if !aggregated.contains_key(&semantic_name) {
                continue;
            }
            for cell in &layer.facet_values {
                let Some(value) = cell else {
                    continue;
                };
                if let PaintValue::Scalar(s) = value {
                    return Err(RegionMappingError::ScalarInRegionSplitFacetValue {
                        object_id: obj.id.clone(),
                        semantic: semantic_name,
                        scalar_bits: s.to_bits(),
                    });
                }
                per_semantic
                    .entry(semantic_name.clone())
                    .or_default()
                    .insert(OrdPaintValue(value.clone()));
            }
        }

        if per_semantic.is_empty() {
            continue;
        }

        let mut object_map: HashMap<String, Vec<PaintValue>> = HashMap::new();
        for (sem, set) in per_semantic {
            let values: Vec<PaintValue> = set.into_iter().map(|w| w.0).collect();
            object_map.insert(sem, values);
        }
        out.insert(obj.id.clone(), object_map);
    }

    Ok(out)
}

/// Execute the built-in `PrePass::RegionMapping` stage.
///
/// Iteration is stable: layers, active regions within a layer, and
/// module invocations within a stage are all visited in the order they
/// appear in their source `Vec`s, so repeated invocations over the same
/// inputs produce a `RegionMapIR` with identical content.
///
/// When `paint_regions` is `None` or `paint_semantic_configs` is empty, the
/// output is bit-identical to the pre-packet path (invariant 9).
pub fn execute_region_mapping(
    layer_plan: &LayerPlanIR,
    projection: &RegionMappingPlanProjection<'_>,
    paint_semantic_configs: &BTreeMap<PaintSemantic, ResolvedConfig>,
    aggregated_region_split: &BTreeMap<String, AggregatedRegionSplitEntry>,
    objects: &[ObjectMesh],
) -> Result<RegionMapIR, RegionMappingError> {
    execute_region_mapping_with_cap(
        layer_plan,
        projection,
        paint_semantic_configs,
        aggregated_region_split,
        objects,
        DEFAULT_REGION_MAP_CAP,
    )
}

/// Same as [`execute_region_mapping`] with a caller-supplied cap.
///
/// `objects` carries the per-object [`ObjectMesh`] data used to look up each
/// region's `modifier_volumes` and stamp their non-`subtype` `config_delta`
/// fields into `RegionPlan.config.extensions` (Packet 68). Pass `&[]` to
/// disable modifier stamping and preserve the pre-Packet-68 path (test
/// fixtures with no modifier data).
///
/// Stamping order per region: `region.resolved_config` → modifier deltas
/// (priority-ascending) → paint-semantic overlays. Paint overlays therefore
/// win over modifier deltas, matching the
/// global → per-object → modifier → paint precedence chain.
pub fn execute_region_mapping_with_cap(
    layer_plan: &LayerPlanIR,
    projection: &RegionMappingPlanProjection<'_>,
    paint_semantic_configs: &BTreeMap<PaintSemantic, ResolvedConfig>,
    aggregated_region_split: &BTreeMap<String, AggregatedRegionSplitEntry>,
    objects: &[ObjectMesh],
    cap: usize,
) -> Result<RegionMapIR, RegionMappingError> {
    execute_region_mapping_inner(
        layer_plan,
        projection,
        paint_semantic_configs,
        aggregated_region_split,
        objects,
        None,
        // No per-tool overlays on the legacy/test entry point.
        &BTreeMap::new(),
        cap,
    )
}

/// Low-level kernel for region-map compilation.
///
/// Made `pub` so the slicer-runtime wrapper can call it directly with a host
/// config authority (`host_config = Some(...)`) without duplicating the logic.
/// (Minor deviation from AC-1's "private helpers" wording — recorded in
/// packet deviations.)
pub fn execute_region_mapping_inner(
    layer_plan: &LayerPlanIR,
    projection: &RegionMappingPlanProjection<'_>,
    paint_semantic_configs: &BTreeMap<PaintSemantic, ResolvedConfig>,
    // P93: cross-product expansion of `(layer × ActiveRegion × variant_chain)`.
    // Keys are region-split semantic names declared by loaded modules; the
    // chain enumerator uses these as the canonical axis order. Pass an empty
    // map to preserve the pre-P93 single-variant flow.
    aggregated_region_split: &BTreeMap<String, AggregatedRegionSplitEntry>,
    objects: &[ObjectMesh],
    // Host config authority for `RegionPlan.config` (packet 76, 1a). The map
    // contains runtime-pre-resolved object/modifier/paint/tool target stacks.
    // When `None`, the module-emitted `region.resolved_config` remains the
    // compatibility base for `execute_region_mapping` test/e2e callers.
    host_config: Option<(&BTreeMap<String, ResolvedConfig>, &ResolvedConfig)>,
    // Standalone compatibility configs. Production target stacks already have
    // tool-last precedence applied by the runtime resolver.
    tool_configs: &BTreeMap<u32, ResolvedConfig>,
    cap: usize,
) -> Result<RegionMapIR, RegionMappingError> {
    // --- Cap check with top-contributor diagnostics (docs/04 normative memory budget) ----
    let mut entry_count = 0usize;
    // Per-object region/layer counters for overflow diagnostics.
    let mut region_counts: HashMap<String, usize> = HashMap::new();
    let mut layer_counts: HashMap<String, usize> = HashMap::new();
    for layer in &layer_plan.global_layers {
        entry_count = entry_count.saturating_add(layer.active_regions.len());
        for region in &layer.active_regions {
            *region_counts.entry(region.object_id.clone()).or_insert(0) += 1;
        }
        layer_counts.insert(layer.index.to_string(), layer.active_regions.len());
    }
    if entry_count > cap {
        // Build top contributors: sort objects by region_count descending, take top 5.
        let mut sorted: Vec<(String, usize)> = region_counts.into_iter().collect();
        sorted.sort_by_key(|b| std::cmp::Reverse(b.1));
        let top_contributors: Vec<TopContributor> = sorted
            .into_iter()
            .take(5)
            .map(|(object_id, region_count)| {
                let layer_count = layer_counts.len();
                TopContributor {
                    object_id,
                    region_count,
                    layer_count,
                }
            })
            .collect();
        let remediation = "reduce region granularity, raise cap, or split job".to_string();
        return Err(RegionMappingError::CapExceeded {
            entry_count,
            cap,
            top_contributors,
            remediation,
        });
    }

    // --- Precompute per-stage ModuleInvocation lists ------------------
    // These lists are identical across every region in this step
    // (we are not yet applying per-region config disables / claim
    // resolution). The wrapper has already extracted them from the
    // scheduler plan into the projection — clone to a local
    // Vec to preserve the rest of the kernel body verbatim.
    let stage_invocations: Vec<(StageId, Vec<ModuleInvocation>)> =
        projection.stage_invocations.to_vec();

    // --- P93 cross-product preflight: scan paint data + canonical axis order.
    // `aggregated_region_split` keys define which semantics drive expansion;
    // `scan_paint_data` produces the per-object value sets and rejects
    // `Scalar` values for opted-in semantics (AC-N3).
    let painting_variants_per_object = scan_paint_data(objects, aggregated_region_split)?;
    let canonical_order: Vec<String> = aggregated_region_split.keys().cloned().collect();
    let empty_variants: HashMap<String, Vec<PaintValue>> = HashMap::new();

    // --- Build entries ------------------------------------------------
    let mut region_map_out = RegionMapIR::default();
    for layer in &layer_plan.global_layers {
        for region in &layer.active_regions {
            let mut stage_modules: HashMap<StageId, Vec<ModuleInvocation>> =
                HashMap::with_capacity(stage_invocations.len());
            for (sid, invs) in &stage_invocations {
                stage_modules.insert(sid.clone(), invs.clone());
            }

            // Select the per-region base config. With a host authority, the
            // host's per-object map (or its default) wins over the
            // module-emitted `region.resolved_config`; without one, the
            // module-emitted config is the base.
            let base_config = match host_config {
                Some((per_object, default)) => per_object
                    .get(&region.object_id)
                    .cloned()
                    .unwrap_or_else(|| default.clone()),
                None => region.resolved_config.clone(),
            };

            // Ticket 18 — bind each modifier's config delta to its minted
            // sub-region (packet 132's geometric counterpart). The Tier-2
            // split (`slicer_runtime::region_partition::split_modifier_footprints`)
            // mints a `SlicedRegion` per stampable modifier footprint; its id is
            // a pure function of `(base_region_id, object_id, footprint polygons)`
            // (FNV-1a in `slicer_ir::modifier_sub_region_id`), and the footprint
            // polygons are this modifier mesh's cross-section at the layer Z via
            // `crate::slice_mesh_ex` — the exact inputs
            // `slicer_runtime::layer_executor::stage_modifier_footprints` staged.
            // Re-deriving both here (same slice, same hash) reproduces the
            // Tier-2 ids byte-for-byte, so this entry's `RegionKey` matches the
            // arena region the pipeline will mint.
            //
            // Re-derivation is the only viable binding: region mapping runs in
            // prepass BEFORE `PrePass::Slice` and long before `Layer::Perimeters`
            // mints the sub-regions in the per-layer arena, so a binding carried
            // on the staged footprint cannot reach this kernel.
            //
            // Only the OWNING modifier's delta is stamped onto the base for its
            // entry (per-modifier group, priority-ascending last-writer-wins via
            // `stamp_modifier_sub_region_configs`); the region's own base
            // (empty-chain) entry keeps the pure base config (see the chain
            // fold below). Modifiers whose cross-sections hash to the same
            // sub-region id (identical geometry) share one entry and merge
            // their deltas. Support subtypes are skipped exactly as both
            // stamping functions and the staging site skip them, and empty
            // meshes have no cross-section.
            //
            // An entry may be minted for a layer where the Tier-2 split mints no
            // geometry (footprint non-empty but disjoint from the base region's
            // polygons) — an orphan RegionKey is harmless: no `SlicedRegion` ever
            // materialises it, so no consumer iterates its config.
            if !is_modifier_namespace_id(region.region_id) {
                if let Some(obj) = objects.iter().find(|o| o.id == region.object_id) {
                    let mut subs: Vec<(Vec<slicer_ir::ExPolygon>, Vec<&ModifierVolume>)> =
                        Vec::new();
                    for mv in &obj.modifier_volumes {
                        // Same skip rules as `stamp_modifier_sub_region_configs`
                        // and `stage_modifier_footprints`.
                        match mv.kind() {
                            slicer_ir::ModifierKind::ParameterModifier => {}
                            slicer_ir::ModifierKind::NegativePart => {}
                            slicer_ir::ModifierKind::SupportEnforcer => continue,
                            slicer_ir::ModifierKind::SupportBlocker => continue,
                        }
                        if mv.mesh.vertices.is_empty() || mv.mesh.indices.is_empty() {
                            continue;
                        }
                        let polygons = crate::slice_mesh_ex(&mv.mesh, &[layer.z])
                            .into_iter()
                            .next()
                            .unwrap_or_default();
                        if polygons.is_empty() {
                            continue;
                        }
                        if let Some((_, modifiers)) =
                            subs.iter_mut().find(|(existing, _)| existing == &polygons)
                        {
                            modifiers.push(mv);
                        } else {
                            subs.push((polygons, vec![mv]));
                        }
                    }
                    for (footprint, mvs) in subs {
                        if !modifier_sub_region_id_fits(region.region_id) {
                            return Err(RegionMappingError::ModifierParentRegionIdOutOfRange {
                                key: RegionKey {
                                    global_layer_index: layer.index,
                                    object_id: region.object_id.clone(),
                                    region_id: region.region_id,
                                    variant_chain: Vec::new(),
                                },
                            });
                        }
                        let sub_id = modifier_sub_region_id(region.region_id, &obj.id, &footprint);
                        if region_map_out.entries.len() >= cap {
                            return Err(cap_exceeded(
                                &region_map_out.entries,
                                cap,
                                region_map_out.entries.len() + 1,
                            ));
                        }
                        let modifiers: Vec<ModifierVolume> = mvs.into_iter().cloned().collect();
                        let resolved_sub_config = host_config
                            .and_then(|(configs, _)| {
                                resolved_target_config(
                                    configs,
                                    &region.object_id,
                                    &modifiers,
                                    &[],
                                    None,
                                )
                            })
                            .cloned()
                            .unwrap_or_else(|| base_config.clone());
                        let sub_config = stamp_pre_resolved_sub_region_configs(
                            base_config.clone(),
                            resolved_sub_config,
                            &modifiers,
                            region.region_id,
                            sub_id,
                        )
                        .remove(&sub_id)
                        .expect(
                            "stamp_modifier_sub_region_configs always emits the sub-region entry",
                        );
                        let key = RegionKey {
                            global_layer_index: layer.index,
                            object_id: region.object_id.clone(),
                            region_id: sub_id,
                            variant_chain: Vec::new(),
                        };
                        let config = region_map_out.intern_config(sub_config);
                        if region_map_out
                            .entries
                            .insert(
                                key.clone(),
                                RegionPlan {
                                    config,
                                    stage_modules: stage_modules.clone(),
                                    paint_overrides: BTreeMap::new(),
                                },
                            )
                            .is_some()
                        {
                            return Err(RegionMappingError::DuplicateRegionKey { key });
                        }
                    }
                }
            }

            let modifier_groups = if is_modifier_namespace_id(region.region_id) {
                Vec::new()
            } else {
                objects
                    .iter()
                    .find(|object| object.id == region.object_id)
                    .map(|object| modifier_footprint_groups(object, layer.z))
                    .unwrap_or_default()
            };

            // Enumerate canonical chains for this object. Objects absent from
            // `painting_variants_per_object` (no opted-in paint values) yield
            // exactly one chain: the empty subset, reproducing the pre-P93
            // single-variant flow.
            let variants_for_obj = painting_variants_per_object
                .get(&region.object_id)
                .unwrap_or(&empty_variants);
            let chains = enumerate_canonical_chains(variants_for_obj, &canonical_order);

            // Sorted list of `paint_semantic_configs` keys, used for the
            // semantic-name → `PaintSemantic` lookup inside the chain fold.
            // (Reused per region, but rebuilt once outside the hot inner loop
            // would require restructuring; the map is tiny so cloning is fine.)
            for chain in chains {
                // Modifier deltas are bound to the spatial child entries
                // below. Keeping every parent chain pure prevents a modifier
                // from leaking outside its footprint, including across paint
                // variants.
                let mut effective = host_config
                    .and_then(|(configs, _)| {
                        resolved_target_config(
                            configs,
                            &region.object_id,
                            &[],
                            &chain,
                            chain.iter().find_map(|(semantic, value)| {
                                (semantic == "material").then_some(value).and_then(|value| {
                                    match value {
                                        PaintValue::ToolIndex(index) => Some(*index),
                                        _ => None,
                                    }
                                })
                            }),
                        )
                    })
                    .cloned()
                    .unwrap_or_else(|| base_config.clone());
                let mut paint_overrides: BTreeMap<PaintSemantic, ResolvedConfig> = BTreeMap::new();
                // The chain's painted material tool (if any). Captured here so the
                // per-tool config can be overlaid LAST (highest precedence), after
                // the paint overlays below.
                let mut chain_tool_index: Option<u32> = None;
                for (sem_name, value) in &chain {
                    // Match the canonical semantic name against the existing
                    // `paint_semantic_configs` keys via
                    // `paint_semantic_namespace_key`, mirroring the idiom in
                    // `slicer-scheduler::config_resolution::resolve_paint_overrides`.
                    let matched_key = paint_semantic_configs
                        .keys()
                        .find(|sem| &paint_semantic_namespace_key(sem) == sem_name);
                    if let Some(sem_key) = matched_key {
                        if let Some(sem_cfg) = paint_semantic_configs.get(sem_key) {
                            paint_overrides.insert(sem_key.clone(), sem_cfg.clone());
                            if host_config.is_none() {
                                effective = sem_cfg.clone();
                            }
                        }
                    }
                    // A material chain entry carries the region's tool selector.
                    if sem_name == "material" {
                        if let PaintValue::ToolIndex(n) = value {
                            chain_tool_index = Some(*n);
                        }
                    }
                }

                // Compatibility callers without the production target map can
                // still provide a fully resolved tool target.
                if host_config.is_none() {
                    if let Some(tool_config) =
                        chain_tool_index.and_then(|index| tool_configs.get(&index))
                    {
                        effective = tool_config.clone();
                    }
                }

                // Painted regions receive a distinct parent id during paint
                // segmentation. Mint their modifier children from that id and
                // retain the chain on the child key, so the later prepass
                // splitter and config lookup address the same region.
                if !chain.is_empty() && !modifier_groups.is_empty() {
                    let parent_region_id = paint_variant_region_id(region.region_id, &chain);
                    if !modifier_sub_region_id_fits(parent_region_id) {
                        return Err(RegionMappingError::ModifierParentRegionIdOutOfRange {
                            key: RegionKey {
                                global_layer_index: layer.index,
                                object_id: region.object_id.clone(),
                                region_id: parent_region_id,
                                variant_chain: chain.clone(),
                            },
                        });
                    }

                    for (footprint, modifiers) in &modifier_groups {
                        let sub_id =
                            modifier_sub_region_id(parent_region_id, &region.object_id, footprint);
                        let resolved_child_config = host_config
                            .and_then(|(configs, _)| {
                                resolved_target_config(
                                    configs,
                                    &region.object_id,
                                    modifiers,
                                    &chain,
                                    chain_tool_index,
                                )
                            })
                            .cloned()
                            .unwrap_or_else(|| base_config.clone());
                        let child_config = stamp_pre_resolved_sub_region_configs(
                            base_config.clone(),
                            resolved_child_config,
                            modifiers,
                            parent_region_id,
                            sub_id,
                        )
                        .remove(&sub_id)
                        .expect("modifier config helper always emits the child entry");

                        let key = RegionKey {
                            global_layer_index: layer.index,
                            object_id: region.object_id.clone(),
                            region_id: sub_id,
                            variant_chain: chain.clone(),
                        };
                        if region_map_out.entries.len() >= cap {
                            return Err(cap_exceeded(
                                &region_map_out.entries,
                                cap,
                                region_map_out.entries.len() + 1,
                            ));
                        }
                        let config = region_map_out.intern_config(child_config);
                        if region_map_out
                            .entries
                            .insert(
                                key.clone(),
                                RegionPlan {
                                    config,
                                    stage_modules: stage_modules.clone(),
                                    paint_overrides: paint_overrides.clone(),
                                },
                            )
                            .is_some()
                        {
                            return Err(RegionMappingError::DuplicateRegionKey { key });
                        }
                    }
                }

                let config_id = region_map_out.intern_config(effective);
                let plan_entry = RegionPlan {
                    config: config_id,
                    stage_modules: stage_modules.clone(),
                    paint_overrides,
                };

                let key = RegionKey {
                    global_layer_index: layer.index,
                    object_id: region.object_id.clone(),
                    region_id: region.region_id,
                    variant_chain: chain,
                };

                // Per-insert cap guard. Cross-product expansion may push the
                // entry count past `cap` even when the unexpanded precheck
                // above passed. Reuse the precheck's top-contributor shape.
                if region_map_out.entries.len() >= cap {
                    return Err(cap_exceeded(
                        &region_map_out.entries,
                        cap,
                        region_map_out.entries.len() + 1,
                    ));
                }

                if region_map_out
                    .entries
                    .insert(key.clone(), plan_entry)
                    .is_some()
                {
                    return Err(RegionMappingError::DuplicateRegionKey { key });
                }
            }
        }
    }

    Ok(region_map_out)
}
