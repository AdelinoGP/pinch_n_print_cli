//! Independent model and checker for `H-REGIONMAP`.
//!
//! The checker derives every expectation from the **fixture declarations** —
//! axis names, declared priorities, per-object paint values, the declared base
//! configs and the declared paint overlays. It never calls the production axis
//! sorter, the chain enumerator, the config interner or `config_for` to build
//! an expected answer; `execute_region_mapping_with_cap` is the only system
//! under test.
//!
//! The two rules the model encodes are the documented ones:
//!
//! 1. **Chain identity** — a parent's axes ascend by `(priority, semantic
//!    name)`, each chain is a subsequence of that order, and each axis is
//!    either absent or present with exactly one of its distinct values. The
//!    value sequence within an axis is the canonical
//!    `Flag(false) < Flag(true) < ToolIndex(n) < Custom(s)` order.
//! 2. **Config authority** — the parent's base config, then compatibility paint
//!    overlays in **lexical semantic namespace order** (each replacing the
//!    config wholesale, matching the compatibility path), then the tool overlay
//!    last. Chain order does not drive overlay order.

use std::collections::{BTreeMap, HashMap};

use proptest::prelude::*;
use proptest::sample::subsequence;
use proptest::test_runner::TestCaseResult;
use slicer_core::algos::region_mapping::{
    execute_region_mapping_inner, execute_region_mapping_with_cap, RegionMappingError,
    RegionMappingPlanProjection,
};
use slicer_ir::slice_ir::AggregatedRegionSplitEntry;
use slicer_ir::{
    ActiveRegion, FacetPaintData, GlobalLayer, IndexedTriangleSet, LayerPlanIR, ModuleInvocation,
    ObjectMesh, PaintLayer, PaintSemantic, PaintValue, RegionKey, RegionSplitValueType,
    ResolvedConfig, SemVer, StageId, Transform3d,
};
use slicer_sdk::test_support::property_testing::{CaseGuard, PropertyRun};

/// One declared region-split axis: namespace name, declared priority and the
/// value family its declaration admits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AxisDecl {
    /// Namespace spelling used in `variant_chain` pairs.
    pub name: &'static str,
    /// Declared priority — the primary key of the shared chain identity.
    pub priority: u32,
    /// Declared `value_type` family.
    pub kind: ValueKind,
}

/// The legal `[[region_split]]` value families a fixture axis can declare.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    /// `Flag(bool)` values.
    Flag,
    /// `ToolIndex(u32)` values.
    ToolIndex,
    /// `Custom(String)` values.
    Custom,
}

/// The registry's five legal fixture axes. Core priorities are fixed by
/// `CORE_REGION_SPLIT_PRIORITIES` (`material` 100, `fuzzy_skin` 200);
/// community priorities sit at or above `COMMUNITY_PRIORITY_FLOOR` (1000).
/// The `alpha`/`beta` tie is deliberate: it is what forces a lexical
/// tie-break, and `zeta` (1000) proves priority beats the name sort.
pub const AXIS_POOL: [AxisDecl; 5] = [
    AxisDecl {
        name: "material",
        priority: 100,
        kind: ValueKind::ToolIndex,
    },
    AxisDecl {
        name: "fuzzy_skin",
        priority: 200,
        kind: ValueKind::Flag,
    },
    AxisDecl {
        name: "com.example.zeta",
        priority: 1000,
        kind: ValueKind::ToolIndex,
    },
    AxisDecl {
        name: "com.example.alpha",
        priority: 1100,
        kind: ValueKind::Custom,
    },
    AxisDecl {
        name: "com.example.beta",
        priority: 1100,
        kind: ValueKind::Flag,
    },
];

/// Declared paint-config overlays, keyed by the axis they bind to. `line_width`
/// is the observed field; the values are dyadic so they compare exactly.
pub const OVERLAY_WIDTH_MM: [(&str, f64); 3] = [
    ("com.example.alpha", 1.25),
    ("fuzzy_skin", 0.5),
    ("material", 0.75),
];

/// One object's paint declaration for one axis.
#[derive(Debug, Clone)]
pub struct ObjectAxis {
    /// Index into the case's axis list.
    pub axis: usize,
    /// Distinct declared values, in canonical value order.
    pub values: Vec<PaintValue>,
}

/// One fixture object.
#[derive(Debug, Clone)]
pub struct ObjectCase {
    /// Unique object id.
    pub id: String,
    /// Declared paint per axis; axes not listed are unpainted for this object.
    pub axes: Vec<ObjectAxis>,
}

/// One whole generated fixture.
#[derive(Debug, Clone)]
pub struct Case {
    /// Number of global layers; layer `i` has index `i` and `z = 0.2 * (i+1)`.
    pub layers: usize,
    /// Active regions minted per `(layer, object)`.
    pub regions_per_object: usize,
    /// Declared axes, in declaration order (the model sorts them itself).
    pub axes: Vec<AxisDecl>,
    /// Fixture objects, at least one, ids unique.
    pub objects: Vec<ObjectCase>,
}

impl Case {
    /// Layer Z for a zero-based layer position.
    #[must_use]
    pub fn layer_z(layer: usize) -> f32 {
        0.2 * (layer as f32 + 1.0)
    }

    /// Independently derived canonical axis order: ascending
    /// `(priority, semantic name)`.
    #[must_use]
    pub fn canonical_axes(&self) -> Vec<&'static str> {
        let mut pairs: Vec<(u32, &'static str)> =
            self.axes.iter().map(|a| (a.priority, a.name)).collect();
        pairs.sort();
        pairs.into_iter().map(|(_, name)| name).collect()
    }

    /// Base config declared for one `(object, region)` parent.
    #[must_use]
    pub fn base_config(object_index: usize, region_id: usize) -> ResolvedConfig {
        ResolvedConfig {
            wall_count: 1 + (object_index as u32) * 4 + region_id as u32,
            infill_density: 0.125 * (1.0 + region_id as f32),
            ..ResolvedConfig::default()
        }
    }
}

/// The declared paint overlay config for one axis name, if any.
#[must_use]
pub fn overlay_config(axis_name: &str) -> Option<ResolvedConfig> {
    OVERLAY_WIDTH_MM
        .iter()
        .find(|(name, _)| *name == axis_name)
        .map(|(_, width)| ResolvedConfig {
            line_width: *width,
            ..ResolvedConfig::default()
        })
}

/// The compatibility overlay applied to a chain, independently derived:
/// lexical namespace order over the chain's semantics, wholesale replacement,
/// so the lexically **last** declared overlay present wins.
#[must_use]
pub fn expected_compat_config(
    base: &ResolvedConfig,
    chain: &[(String, PaintValue)],
) -> ResolvedConfig {
    let mut overlays: Vec<(&str, ResolvedConfig)> = OVERLAY_WIDTH_MM
        .iter()
        .filter(|(name, _)| chain.iter().any(|(axis, _)| axis == name))
        .filter_map(|(name, _)| overlay_config(name).map(|config| (*name, config)))
        .collect();
    overlays.sort_by(|a, b| a.0.cmp(b.0));
    overlays
        .last()
        .map_or_else(|| base.clone(), |(_, config)| config.clone())
}

/// Every ordered chain a parent can carry, in canonical axis order.
///
/// Independent ordinal decode (mixed radix over `1 + K_i`), not a copy of the
/// production recursive enumerator: digit `0` omits the axis, digit `d > 0`
/// selects `values[d - 1]`.
pub fn expected_chains(
    canonical_axes: &[&'static str],
    object: &ObjectCase,
    case: &Case,
) -> Result<Vec<Vec<(String, PaintValue)>>, String> {
    let mut radices: Vec<usize> = Vec::new();
    let mut value_lists: Vec<Option<&Vec<PaintValue>>> = Vec::new();
    for name in canonical_axes {
        let axis_index = case
            .axes
            .iter()
            .position(|axis| axis.name == *name)
            .ok_or_else(|| format!("axis {name} missing from the declaration"))?;
        match object.axes.iter().find(|a| a.axis == axis_index) {
            Some(present) => {
                radices.push(1 + present.values.len());
                value_lists.push(Some(&present.values));
            }
            None => {
                radices.push(1);
                value_lists.push(None);
            }
        }
    }
    let total = radices.iter().try_fold(1usize, |acc, radix| {
        acc.checked_mul(*radix)
            .ok_or_else(|| String::from("chain count overflow"))
    })?;
    let mut chains = Vec::with_capacity(total);
    for ordinal in 0..total {
        let mut rest = ordinal;
        let mut chain = Vec::new();
        for (position, radix) in radices.iter().enumerate() {
            let digit = rest % radix;
            rest /= radix;
            if digit > 0 {
                let values = value_lists[position]
                    .ok_or_else(|| String::from("selected digit on an absent axis"))?;
                chain.push((
                    canonical_axes[position].to_string(),
                    values[digit - 1].clone(),
                ));
            }
        }
        chains.push(chain);
    }
    Ok(chains)
}

/// Expected full `RegionKey` set and its independently derived config.
#[derive(Debug, Clone, PartialEq)]
pub struct ExpectedEntry {
    /// Full map key: exact `(layer, object, base region, ordered chain)`.
    pub key: RegionKey,
    /// The config the compatibility path must resolve for that key.
    pub config: ResolvedConfig,
}

/// Expected observation for a case.
#[derive(Debug, Clone)]
pub struct Expectation {
    /// Every expected entry, in no particular order.
    pub entries: Vec<ExpectedEntry>,
    /// Distinct expected configs, default first (the pre-seeded pool slot).
    pub distinct_configs: Vec<ResolvedConfig>,
    /// Entry count the cross-product must expand to.
    pub total: usize,
    /// Active `(layer, region)` parents before chain expansion.
    pub parents: usize,
}

/// Build the full expectation from the fixture declarations alone.
pub fn expectation(case: &Case) -> Result<Expectation, String> {
    let canonical_axes = case.canonical_axes();
    let mut entries: Vec<ExpectedEntry> = Vec::new();
    let mut parents = 0usize;
    for layer in 0..case.layers {
        for (object_index, object) in case.objects.iter().enumerate() {
            let chains = expected_chains(&canonical_axes, object, case)?;
            for region_id in 0..case.regions_per_object {
                parents += 1;
                let base = Case::base_config(object_index, region_id);
                for chain in &chains {
                    entries.push(ExpectedEntry {
                        key: RegionKey {
                            global_layer_index: layer as u32,
                            object_id: object.id.clone(),
                            region_id: region_id as u64,
                            variant_chain: chain.clone(),
                        },
                        config: expected_compat_config(&base, chain),
                    });
                }
            }
        }
    }
    let total = entries.len();
    let mut distinct_configs: Vec<ResolvedConfig> = vec![ResolvedConfig::default()];
    for entry in &entries {
        if !distinct_configs.contains(&entry.config) {
            distinct_configs.push(entry.config.clone());
        }
    }
    Ok(Expectation {
        entries,
        distinct_configs,
        total,
        parents,
    })
}

/// What the system under test produced for one case.
#[derive(Debug, Clone)]
pub struct Observed {
    /// `(key, config_for, config_for_raw)` for every committed entry.
    pub entries: Vec<(RegionKey, ResolvedConfig, ResolvedConfig)>,
    /// The committer's interned config pool, in insertion order.
    pub interned: Vec<ResolvedConfig>,
    /// Entry count of the exact-count cap call.
    pub exact_cap_len: usize,
    /// Outcome of the one-below cap call.
    pub below_cap: BelowCap,
}

/// The one-below-cap outcome, reduced to the fields the model predicts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BelowCap {
    /// The call unexpectedly succeeded.
    Succeeded(usize),
    /// The documented `CapExceeded` shape.
    Exceeded {
        /// Reported prospective entry count.
        entry_count: usize,
        /// The configured cap.
        cap: usize,
    },
}

/// Build the fixture IR the system under test consumes.
fn fixture(
    case: &Case,
) -> (
    LayerPlanIR,
    Vec<ObjectMesh>,
    BTreeMap<String, AggregatedRegionSplitEntry>,
) {
    let mut global_layers = Vec::with_capacity(case.layers);
    for layer in 0..case.layers {
        let mut active_regions = Vec::new();
        for (object_index, object) in case.objects.iter().enumerate() {
            for region_id in 0..case.regions_per_object {
                active_regions.push(ActiveRegion {
                    object_id: object.id.clone(),
                    region_id: region_id as u64,
                    resolved_config: Case::base_config(object_index, region_id),
                    effective_layer_height: 0.2,
                    ..ActiveRegion::default()
                });
            }
        }
        global_layers.push(GlobalLayer {
            index: layer as u32,
            z: Case::layer_z(layer),
            active_regions,
            ..GlobalLayer::default()
        });
    }
    let layer_plan = LayerPlanIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layers,
        object_participation: HashMap::new(),
    };

    let mut aggregated: BTreeMap<String, AggregatedRegionSplitEntry> = BTreeMap::new();
    for axis in &case.axes {
        aggregated.insert(
            axis.name.to_string(),
            AggregatedRegionSplitEntry {
                priority: axis.priority,
                value_type: match axis.kind {
                    ValueKind::Flag => RegionSplitValueType::Flag,
                    ValueKind::ToolIndex => RegionSplitValueType::ToolIndex,
                    ValueKind::Custom => RegionSplitValueType::CustomString,
                },
                declaring_modules: Vec::new(),
            },
        );
    }

    let objects = case
        .objects
        .iter()
        .map(|object| {
            let mut layers = Vec::new();
            for (axis_index, axis) in case.axes.iter().enumerate() {
                let Some(present) = object.axes.iter().find(|a| a.axis == axis_index) else {
                    continue;
                };
                layers.push(PaintLayer {
                    // `scan_paint_data` keys by namespace spelling, so a Custom
                    // semantic carrying the axis name addresses the same axis.
                    semantic: paint_semantic(axis.name),
                    facet_values: present.values.iter().cloned().map(Some).collect(),
                    strokes: Vec::new(),
                });
            }
            ObjectMesh {
                id: object.id.clone(),
                mesh: IndexedTriangleSet::default(),
                transform: Transform3d {
                    matrix: [
                        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0,
                        1.0,
                    ],
                },
                paint_data: (!layers.is_empty()).then_some(FacetPaintData { layers }),
                ..ObjectMesh::default()
            }
        })
        .collect();

    (layer_plan, objects, aggregated)
}

/// The `PaintSemantic` whose namespace spelling is `name`. Core names keep
/// their typed variants so the fixture exercises the real matching rule.
fn paint_semantic(name: &str) -> PaintSemantic {
    match name {
        "material" => PaintSemantic::Material,
        "fuzzy_skin" => PaintSemantic::FuzzySkin,
        other => PaintSemantic::Custom(other.to_string()),
    }
}

/// The declared compatibility paint overlays for a case.
fn paint_semantic_configs(case: &Case) -> BTreeMap<PaintSemantic, ResolvedConfig> {
    let mut configs = BTreeMap::new();
    for axis in &case.axes {
        if let Some(config) = overlay_config(axis.name) {
            configs.insert(paint_semantic(axis.name), config);
        }
    }
    configs
}

/// Run the real system under test for one case.
pub fn observe(case: &Case) -> Result<Observed, String> {
    let expectation = expectation(case)?;
    let (layer_plan, objects, aggregated) = fixture(case);
    let stage_invocations: Vec<(StageId, Vec<ModuleInvocation>)> = Vec::new();
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    let configs = paint_semantic_configs(case);

    let exact = execute_region_mapping_with_cap(
        &layer_plan,
        &projection,
        &configs,
        &aggregated,
        &objects,
        expectation.total,
    )
    .map_err(|error| format!("exact-count cap call failed: {error:?}"))?;

    let entries = exact
        .entries
        .iter()
        .map(|(key, plan)| {
            (
                key.clone(),
                exact.config_for(key).clone(),
                exact.config_for_raw(plan.config).clone(),
            )
        })
        .collect();

    let below = match execute_region_mapping_with_cap(
        &layer_plan,
        &projection,
        &configs,
        &aggregated,
        &objects,
        expectation.total - 1,
    ) {
        Ok(map) => BelowCap::Succeeded(map.entries.len()),
        Err(RegionMappingError::CapExceeded {
            entry_count, cap, ..
        }) => BelowCap::Exceeded { entry_count, cap },
        Err(other) => {
            return Err(format!(
                "one-below-cap call must fail with CapExceeded, got {other:?}"
            ))
        }
    };

    Ok(Observed {
        entries,
        interned: exact.configs.clone(),
        exact_cap_len: exact.entries.len(),
        below_cap: below,
    })
}

/// Compare an observation against the independent expectation.
///
/// Returns the first discrepancy; the controls use that to prove each mutation
/// is caught.
pub fn verify(case: &Case, observed: &Observed) -> Result<(), String> {
    let expectation = expectation(case)?;

    // --- Full key set (ordered chains included) ---------------------------
    let mut actual: HashMap<RegionKey, ()> = HashMap::new();
    for (key, _, _) in &observed.entries {
        if actual.insert(key.clone(), ()).is_some() {
            return Err(format!("the committer emitted {key:?} more than once"));
        }
    }
    if actual.len() != expectation.entries.len() {
        return Err(format!(
            "entry count {} != expected {}",
            actual.len(),
            expectation.entries.len()
        ));
    }
    if observed.exact_cap_len != expectation.total {
        return Err(format!(
            "exact-cap call committed {} entries, expected {}",
            observed.exact_cap_len, expectation.total
        ));
    }
    for expected in &expectation.entries {
        if !actual.contains_key(&expected.key) {
            return Err(format!("missing expected key {:?}", expected.key));
        }
    }

    // --- Per-key config authority -----------------------------------------
    for expected in &expectation.entries {
        let (_, resolved, raw) = observed
            .entries
            .iter()
            .find(|(key, _, _)| key == &expected.key)
            .ok_or_else(|| format!("missing expected key {:?}", expected.key))?;
        if resolved != &expected.config {
            return Err(format!(
                "config_for({:?}) = {resolved:?}, expected {:?}",
                expected.key, expected.config
            ));
        }
        if raw != &expected.config {
            return Err(format!(
                "config_for_raw({:?}) = {raw:?}, expected {:?}",
                expected.key, expected.config
            ));
        }
    }

    // --- Interner: exactly the distinct expected configs, no duplicates ---
    let mut seen: Vec<&ResolvedConfig> = Vec::new();
    for config in &observed.interned {
        if seen.contains(&config) {
            return Err(String::from(
                "the config pool holds a duplicate (interning is not idempotent)",
            ));
        }
        seen.push(config);
    }
    if observed.interned.len() != expectation.distinct_configs.len() {
        return Err(format!(
            "config pool holds {} configs, expected {} distinct",
            observed.interned.len(),
            expectation.distinct_configs.len()
        ));
    }
    for expected_config in &expectation.distinct_configs {
        if !observed
            .interned
            .iter()
            .any(|config| config == expected_config)
        {
            return Err(format!(
                "config pool is missing the expected config {expected_config:?}"
            ));
        }
    }

    // --- Cap boundary ------------------------------------------------------
    let expected_below = BelowCap::Exceeded {
        entry_count: expectation.total,
        cap: expectation.total - 1,
    };
    if observed.below_cap != expected_below {
        return Err(format!(
            "one-below-cap outcome {:?} != expected {:?}",
            observed.below_cap, expected_below
        ));
    }

    Ok(())
}

/// Canonical `PaintValue` order, implemented from the documented rule rather
/// than calling the production comparator: `Flag < ToolIndex < Custom`, with
/// `Flag(false) < Flag(true)`, numeric `ToolIndex`, lexical `Custom`.
pub fn canonical_value_cmp(a: &PaintValue, b: &PaintValue) -> std::cmp::Ordering {
    fn rank(value: &PaintValue) -> u8 {
        match value {
            PaintValue::Flag(_) => 0,
            PaintValue::ToolIndex(_) => 1,
            PaintValue::Custom(_) => 2,
            PaintValue::Scalar(_) => 3,
        }
    }
    match (a, b) {
        (PaintValue::Flag(x), PaintValue::Flag(y)) => x.cmp(y),
        (PaintValue::ToolIndex(x), PaintValue::ToolIndex(y)) => x.cmp(y),
        (PaintValue::Custom(x), PaintValue::Custom(y)) => x.cmp(y),
        _ => rank(a).cmp(&rank(b)),
    }
}

fn value_alphabet(kind: ValueKind) -> Vec<PaintValue> {
    match kind {
        ValueKind::Flag => vec![PaintValue::Flag(false), PaintValue::Flag(true)],
        ValueKind::ToolIndex => (0..4).map(PaintValue::ToolIndex).collect(),
        ValueKind::Custom => ["alpha", "beta", "gamma", "delta"]
            .into_iter()
            .map(|name| PaintValue::Custom(name.to_string()))
            .collect(),
    }
}

/// Strategy for one generated fixture.
///
/// `max_semantics` bounds the declared axes, `max_values_per_semantic` the
/// distinct values per painted axis, `max_layers` / `max_objects` /
/// `max_regions` the plan dimensions. The strategy is total: no `prop_assume`,
/// no rejection.
pub fn strategy(
    max_layers: usize,
    max_objects: usize,
    max_regions: usize,
    max_semantics: usize,
    max_values_per_semantic: usize,
) -> impl Strategy<Value = Case> {
    let axes = subsequence(AXIS_POOL.to_vec(), 0..=max_semantics.min(AXIS_POOL.len()));
    (
        1..=max_layers.max(1),
        1..=max_objects.max(1),
        1..=max_regions.max(1),
        axes,
    )
        .prop_flat_map(move |(layers, object_count, regions, axes)| {
            // One 4-bit selection mask per (object, axis). Bit `i` selects
            // alphabet entry `i`; bits beyond an axis's alphabet size are
            // ignored, so one uniform element strategy covers every value
            // family and no draw can index out of bounds. A zero mask means
            // the object does not paint that axis.
            let masks = proptest::collection::vec(
                proptest::collection::vec(0u8..16u8, axes.len()),
                object_count,
            );
            let axes_for_mapping = axes.clone();
            let max_values = max_values_per_semantic.max(1);
            masks.prop_map(move |rows| Case {
                layers,
                regions_per_object: regions,
                axes: axes_for_mapping.clone(),
                objects: rows
                    .into_iter()
                    .enumerate()
                    .map(|(object_index, row)| ObjectCase {
                        id: format!("obj_{object_index}"),
                        axes: row
                            .into_iter()
                            .enumerate()
                            .filter_map(|(axis_index, mask)| {
                                if mask == 0 {
                                    return None;
                                }
                                let alphabet = value_alphabet(axes_for_mapping[axis_index].kind);
                                let mut values: Vec<PaintValue> = alphabet
                                    .iter()
                                    .enumerate()
                                    .filter(|(bit, _)| mask & (1u8 << bit) != 0)
                                    .map(|(_, value)| value.clone())
                                    .take(max_values)
                                    .collect();
                                if values.is_empty() {
                                    return None;
                                }
                                values.sort_by(canonical_value_cmp);
                                Some(ObjectAxis {
                                    axis: axis_index,
                                    values,
                                })
                            })
                            .collect(),
                    })
                    .collect(),
            })
        })
}

fn bound_usize(run: &PropertyRun, name: &str) -> Result<usize, String> {
    let value = run.bound(name).map_err(|error| error.to_string())?;
    usize::try_from(value).map_err(|_| format!("bound {name} does not fit a usize"))
}

/// Reject a profile whose declared bounds admit more chains than `max_chains`.
///
/// Shared by the registry entry point and the deterministic bound controls, so
/// a control that proves a narrowing bound is rejected exercises the exact
/// check the property runs.
pub fn validate_bounds(
    max_semantics: usize,
    max_values_per_semantic: usize,
    max_chains: usize,
) -> Result<(), String> {
    let widest = (0..max_semantics.min(AXIS_POOL.len())).try_fold(1usize, |acc, _| {
        acc.checked_mul(1 + max_values_per_semantic)
    });
    match widest {
        Some(widest) if widest <= max_chains => Ok(()),
        Some(widest) => Err(format!(
            "profile bounds admit {widest} chains per parent but max_chains is {max_chains}"
        )),
        None => Err(String::from("max_chains bound overflow")),
    }
}

/// Run the system under test with a caller-supplied cap.
///
/// Deterministic controls use this to inspect the cap error shape and the
/// contributor diagnostics; the property goes through [`observe`].
pub fn run_with_cap(case: &Case, cap: usize) -> Result<slicer_ir::RegionMapIR, RegionMappingError> {
    let (layer_plan, objects, aggregated) = fixture(case);
    let stage_invocations: Vec<(StageId, Vec<ModuleInvocation>)> = Vec::new();
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    execute_region_mapping_with_cap(
        &layer_plan,
        &projection,
        &paint_semantic_configs(case),
        &aggregated,
        &objects,
        cap,
    )
}

/// Observed `line_width` of the combined `material` + `fuzzy_skin` key on the
/// compatibility path.
///
/// The fixture declares material 0.75 mm and fuzzy_skin 0.5 mm. Lexical
/// semantic namespace order is `fuzzy_skin` then `material`, so the combined
/// key resolves to 0.75 mm. Passing a tool-0 config adds the tool overlay,
/// which is applied last and must win (1.0 mm).
pub fn lexical_overlay_witness(tool_width: Option<f64>) -> Result<f64, String> {
    let case = material_fuzzy_case();
    let (layer_plan, objects, aggregated) = fixture(&case);
    let stage_invocations: Vec<(StageId, Vec<ModuleInvocation>)> = Vec::new();
    let projection = RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };
    let configs: BTreeMap<PaintSemantic, ResolvedConfig> = BTreeMap::from([
        (
            PaintSemantic::Material,
            ResolvedConfig {
                line_width: 0.75,
                ..ResolvedConfig::default()
            },
        ),
        (
            PaintSemantic::FuzzySkin,
            ResolvedConfig {
                line_width: 0.5,
                ..ResolvedConfig::default()
            },
        ),
    ]);
    let mut tool_configs: BTreeMap<u32, ResolvedConfig> = BTreeMap::new();
    if let Some(width) = tool_width {
        tool_configs.insert(
            0,
            ResolvedConfig {
                line_width: width,
                ..ResolvedConfig::default()
            },
        );
    }
    // The compatibility layer (`host_config = None`) is the one that applies
    // the paint/tool overlays itself, so this witness drives the inner kernel
    // with an explicit tool config map. `execute_region_mapping_with_cap`
    // deliberately passes no tool overlays.
    let map = execute_region_mapping_inner(
        &layer_plan,
        &projection,
        &configs,
        &aggregated,
        &objects,
        None,
        &tool_configs,
        1_000_000,
        None,
    )
    .map_err(|error| format!("lexical overlay witness failed: {error:?}"))?;
    let combined = vec![
        ("material".to_string(), PaintValue::ToolIndex(0)),
        ("fuzzy_skin".to_string(), PaintValue::Flag(true)),
    ];
    let key = map
        .entries
        .keys()
        .find(|key| key.variant_chain == combined)
        .ok_or_else(|| String::from("the combined material/fuzzy key was not committed"))?;
    Ok(map.config_for(key).line_width)
}

/// The four explicit material/fuzzy chains the packet names, used as the live
/// pre-sampling witness. `material` is 100 and `fuzzy_skin` is 200, so the
/// combined chain is material-then-fuzzy — the opposite of name order.
#[must_use]
pub fn material_fuzzy_case() -> Case {
    Case {
        layers: 1,
        regions_per_object: 1,
        axes: vec![AXIS_POOL[0], AXIS_POOL[1]],
        objects: vec![ObjectCase {
            id: "obj_0".to_string(),
            axes: vec![
                ObjectAxis {
                    axis: 0,
                    values: vec![PaintValue::ToolIndex(0)],
                },
                ObjectAxis {
                    axis: 1,
                    values: vec![PaintValue::Flag(true)],
                },
            ],
        }],
    }
}

/// Registry entry point for `H-REGIONMAP`.
///
/// Runs the mandatory deterministic witness through the same checker first,
/// then the generated campaign.
pub fn run_property() -> Result<(), String> {
    let run = PropertyRun::new("H-REGIONMAP", "proptest_region_mapping_expansion")
        .map_err(|error| error.to_string())?;
    let max_layers = bound_usize(&run, "max_layers")?;
    let max_objects = bound_usize(&run, "max_objects")?;
    let max_regions = bound_usize(&run, "max_regions")?;
    let max_semantics = bound_usize(&run, "max_semantics")?;
    let max_values = bound_usize(&run, "max_values_per_semantic")?;
    let max_chains = bound_usize(&run, "max_chains")?;

    // Domain proof before any allocation: the widest reachable product must fit
    // the declared bound.
    validate_bounds(max_semantics, max_values, max_chains)?;

    // Rare witness, run deterministically before sampling: the four named
    // chains must be exactly what the real kernel commits.
    let witness = material_fuzzy_case();
    let observed_witness = observe(&witness)?;
    verify(&witness, &observed_witness)?;
    let witness_keys: Vec<Vec<(String, PaintValue)>> = observed_witness
        .entries
        .iter()
        .map(|(key, _, _)| key.variant_chain.clone())
        .collect();
    let expected_witness = vec![
        vec![],
        vec![("material".to_string(), PaintValue::ToolIndex(0))],
        vec![("fuzzy_skin".to_string(), PaintValue::Flag(true))],
        vec![
            ("material".to_string(), PaintValue::ToolIndex(0)),
            ("fuzzy_skin".to_string(), PaintValue::Flag(true)),
        ],
    ];
    for chain in &expected_witness {
        if !witness_keys.contains(chain) {
            return Err(format!("witness chain {chain:?} was not committed"));
        }
    }

    let generated = strategy(
        max_layers,
        max_objects,
        max_regions,
        max_semantics,
        max_values,
    );
    run.check(
        &generated,
        |case: Case, _guard: &mut CaseGuard| -> TestCaseResult {
            let observed = observe(&case).map_err(TestCaseError::fail)?;
            verify(&case, &observed).map_err(TestCaseError::fail)
        },
    )
    .map_err(|error| error.to_string())
}
