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

use std::collections::{BTreeMap, BTreeSet, HashMap};

use slicer_ir::{
    region_split_registry::enumerate_canonical_chains, ConfigValue, LayerPlanIR, ModifierVolume,
    ModuleInvocation, ObjectId, ObjectMesh, PaintSemantic, PaintValue, RegionKey,
    RegionMapIR, RegionPlan, ResolvedConfig, StageId,
};
use slicer_scheduler::region_split::AggregatedRegionSplitEntry;

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

/// Apply a paint-semantic `ResolvedConfig` on top of a base `ResolvedConfig`.
///
/// For each field in `overlay` that differs from `ResolvedConfig::default()`,
/// the overlay value is written into `base`. This implements the
/// global → per_object → per_paint_semantic precedence chain: the paint
/// overlay wins over the per-object config for any field it explicitly sets.
fn overlay_resolved(base: ResolvedConfig, overlay: &ResolvedConfig) -> ResolvedConfig {
    let d = ResolvedConfig::default();
    let mut r = base;
    if overlay.layer_height != d.layer_height {
        r.layer_height = overlay.layer_height;
    }
    if overlay.line_width != d.line_width {
        r.line_width = overlay.line_width;
    }
    if overlay.first_layer_height != d.first_layer_height {
        r.first_layer_height = overlay.first_layer_height;
    }
    if overlay.first_layer_line_width != d.first_layer_line_width {
        r.first_layer_line_width = overlay.first_layer_line_width;
    }
    if overlay.wall_count != d.wall_count {
        r.wall_count = overlay.wall_count;
    }
    if overlay.outer_wall_speed != d.outer_wall_speed {
        r.outer_wall_speed = overlay.outer_wall_speed;
    }
    if overlay.inner_wall_speed != d.inner_wall_speed {
        r.inner_wall_speed = overlay.inner_wall_speed;
    }
    if overlay.wall_generator != d.wall_generator {
        r.wall_generator = overlay.wall_generator;
    }
    if overlay.arachne_min_feature_size != d.arachne_min_feature_size {
        r.arachne_min_feature_size = overlay.arachne_min_feature_size;
    }
    if overlay.infill_type != d.infill_type {
        r.infill_type = overlay.infill_type;
    }
    if overlay.infill_density != d.infill_density {
        r.infill_density = overlay.infill_density;
    }
    if overlay.infill_angle != d.infill_angle {
        r.infill_angle = overlay.infill_angle;
    }
    if overlay.infill_speed != d.infill_speed {
        r.infill_speed = overlay.infill_speed;
    }
    if overlay.solid_infill_speed != d.solid_infill_speed {
        r.solid_infill_speed = overlay.solid_infill_speed;
    }
    if overlay.top_shell_layers != d.top_shell_layers {
        r.top_shell_layers = overlay.top_shell_layers;
    }
    if overlay.bottom_shell_layers != d.bottom_shell_layers {
        r.bottom_shell_layers = overlay.bottom_shell_layers;
    }
    if overlay.top_fill_holder != d.top_fill_holder {
        r.top_fill_holder = overlay.top_fill_holder.clone();
    }
    if overlay.bottom_fill_holder != d.bottom_fill_holder {
        r.bottom_fill_holder = overlay.bottom_fill_holder.clone();
    }
    if overlay.bridge_fill_holder != d.bridge_fill_holder {
        r.bridge_fill_holder = overlay.bridge_fill_holder.clone();
    }
    if overlay.sparse_fill_holder != d.sparse_fill_holder {
        r.sparse_fill_holder = overlay.sparse_fill_holder.clone();
    }
    if overlay.support_enabled != d.support_enabled {
        r.support_enabled = overlay.support_enabled;
    }
    if overlay.support_type != d.support_type {
        r.support_type = overlay.support_type;
    }
    if overlay.support_overhang_angle != d.support_overhang_angle {
        r.support_overhang_angle = overlay.support_overhang_angle;
    }
    if overlay.nonplanar_max_angle_deg != d.nonplanar_max_angle_deg {
        r.nonplanar_max_angle_deg = overlay.nonplanar_max_angle_deg;
    }
    if overlay.nonplanar_shell_count != d.nonplanar_shell_count {
        r.nonplanar_shell_count = overlay.nonplanar_shell_count;
    }
    if overlay.nonplanar_amplitude != d.nonplanar_amplitude {
        r.nonplanar_amplitude = overlay.nonplanar_amplitude;
    }
    if overlay.smoothificator_target_height != d.smoothificator_target_height {
        r.smoothificator_target_height = overlay.smoothificator_target_height;
    }
    if overlay.smoothificator_adaptive != d.smoothificator_adaptive {
        r.smoothificator_adaptive = overlay.smoothificator_adaptive;
    }
    // Merge extension keys from overlay into base.
    for (k, v) in &overlay.extensions {
        r.extensions.insert(k.clone(), v.clone());
    }
    r
}

/// Stamps `config_delta.fields` from each [`ModifierVolume`] entry into
/// `base_config.extensions`, except the `"subtype"` key. Modifier volumes
/// whose subtype is `support_enforcer` or `support_blocker` are skipped
/// entirely — OrcaSlicer parity (`PrintApply.cpp:590-594`,
/// `model_volume_solid_or_modifier()` excludes ENFORCER and BLOCKER from
/// region-config merging).
///
/// Modifiers are applied in priority-ascending order so the highest-priority
/// modifier wins via [`overlay_resolved`]'s last-writer semantics.
///
/// Applies globally per object (no bbox/polygon overlap check): the only
/// in-use [`slicer_ir::ModifierScope`] variant is `AllFeatures`, and
/// per-layer Z intervals are out of scope for this packet.
fn stamp_modifier_config_deltas(
    base_config: ResolvedConfig,
    modifier_volumes: &[ModifierVolume],
) -> ResolvedConfig {
    // Sort modifier indices by priority ascending so higher-priority writes
    // last (overlay_resolved is last-writer-wins on the `extensions` map).
    let mut order: Vec<usize> = (0..modifier_volumes.len()).collect();
    order.sort_by_key(|&i| modifier_volumes[i].priority);

    let mut result = base_config;
    for idx in order {
        let mv = &modifier_volumes[idx];
        // OrcaSlicer parity: skip support_enforcer / support_blocker entirely.
        if let Some(ConfigValue::String(s)) = mv.config_delta.fields.get("subtype") {
            if s == "support_enforcer" || s == "support_blocker" {
                continue;
            }
        }
        // Build a synthetic ResolvedConfig that carries only the non-subtype
        // delta keys in its `extensions` bucket. All declared fields stay at
        // their `Default` so `overlay_resolved` will leave the base values
        // untouched — only the extension keys are merged.
        //
        // Truly-empty values (empty string, empty list) are skipped per
        // design.md "ConfigValue defaults" row to avoid noise in extensions.
        // Numeric/boolean zeros (`Int(0)`, `Float(0.0)`, `Bool(false)`) are
        // meaningful and stamped — e.g., `Int(0)` for `extruder` is tool 0.
        let mut overlay = ResolvedConfig::default();
        for (k, v) in &mv.config_delta.fields {
            if k == "subtype" {
                continue;
            }
            match v {
                ConfigValue::String(s) if s.is_empty() => continue,
                ConfigValue::List(l) if l.is_empty() => continue,
                _ => {}
            }
            overlay.extensions.insert(k.clone(), v.clone());
        }
        if overlay.extensions.is_empty() {
            continue;
        }
        result = overlay_resolved(result, &overlay);
    }
    result
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
/// fields into `RegionPlan.config.extensions` (Packet 68 —
/// `stamp_modifier_config_deltas`). Pass `&[]` to disable modifier stamping
/// and preserve the pre-Packet-68 path (test fixtures with no modifier data).
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
    // Host config authority for `RegionPlan.config` (packet 76, 1a). When
    // `Some((per_object, default))`, each region's base config is taken from
    // the host's per-object map (falling back to `default`) rather than the
    // module-emitted `region.resolved_config`; modifier deltas and paint
    // overlays are then stamped on top in a single pass. When `None`, the
    // module-emitted `region.resolved_config` is used as the base (preserves
    // the pre-commit `execute_region_mapping` test/e2e callers).
    host_config: Option<(&BTreeMap<String, ResolvedConfig>, &ResolvedConfig)>,
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

            // Stamp modifier-volume config_delta keys into the base config
            // (Packet 68). Ordering: per-object base → modifier_delta →
            // paint_overrides. We compute the modifier-stamped base first so
            // paint overlays (which run last per chain) can still override
            // stamped values, matching global → per-object → modifier → paint
            // precedence.
            let modifier_stamped_base =
                if let Some(obj) = objects.iter().find(|o| o.id == region.object_id) {
                    if obj.modifier_volumes.is_empty() {
                        base_config
                    } else {
                        stamp_modifier_config_deltas(base_config, &obj.modifier_volumes)
                    }
                } else {
                    base_config
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
                let mut effective = modifier_stamped_base.clone();
                let mut paint_overrides: BTreeMap<PaintSemantic, ResolvedConfig> = BTreeMap::new();
                for (sem_name, _value) in &chain {
                    // Match the canonical semantic name against the existing
                    // `paint_semantic_configs` keys via
                    // `paint_semantic_namespace_key`, mirroring the idiom in
                    // `slicer-scheduler::config_resolution::resolve_paint_overrides`.
                    let matched_key = paint_semantic_configs
                        .keys()
                        .find(|sem| &paint_semantic_namespace_key(sem) == sem_name);
                    if let Some(sem_key) = matched_key {
                        if let Some(sem_cfg) = paint_semantic_configs.get(sem_key) {
                            effective = overlay_resolved(effective, sem_cfg);
                            paint_overrides.insert(sem_key.clone(), sem_cfg.clone());
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
                    let mut sorted: Vec<(String, usize)> = region_map_out
                        .entries
                        .keys()
                        .fold(HashMap::<String, usize>::new(), |mut acc, k| {
                            *acc.entry(k.object_id.clone()).or_insert(0) += 1;
                            acc
                        })
                        .into_iter()
                        .collect();
                    sorted.sort_by_key(|b| std::cmp::Reverse(b.1));
                    let layer_count = region_map_out
                        .entries
                        .keys()
                        .map(|k| k.global_layer_index)
                        .collect::<std::collections::HashSet<_>>()
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
                    return Err(RegionMappingError::CapExceeded {
                        entry_count: region_map_out.entries.len() + 1,
                        cap,
                        top_contributors,
                        remediation: "reduce region granularity, raise cap, or split job"
                            .to_string(),
                    });
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
