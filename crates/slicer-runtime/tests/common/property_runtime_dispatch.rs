//! Charter `H-RUNTIME-DISPATCH`: semantic-set gating on the live selection path.
//!
//! The property compares two independent things:
//!
//! 1. the live `slicer_wasm_host::selection::paint_allows_layer` predicate
//!    against a standalone semantic-set model over the generated scene, and
//! 2. the real committed selection index built by
//!    `slicer_runtime::build_and_commit_layer_selection_index` for a neutral
//!    module-backed layer stage (`Layer::SlicePostProcess`, empty claims).
//!
//! The model is deliberately not a wrapper: it recomputes
//! `allow = S.is_empty() || slice.is_absent() || S ∩ names(slice) != ∅` from the
//! generated names only, and the fixture derives the `RegionMapIR` addresses
//! from the documented address rule rather than from production's own key
//! helper.
//!
//! Gating is per `(module × layer)`, never per region: a module with an empty
//! claim list selects **every** candidate region of a matching layer, so the
//! real-selection assertions count all candidates and fail on any
//! per-region-filtering implementation.
//!
//! Values (`PaintValue`) and polygon emptiness do not participate in the
//! predicate; the property asserts invariance under a value-only mutation and
//! under a permutation of committed region order.

#![allow(missing_docs)]

use std::collections::HashSet;
use std::sync::Arc;

use proptest::prelude::*;
use proptest::strategy::BoxedStrategy;
use proptest::test_runner::{TestCaseError, TestCaseResult};
use slicer_ir::{
    ConfigView, GlobalLayer, MeshIR, PaintValue, RegionKey, RegionMapIR, RegionPlan,
    ResolvedConfig, SliceIR, SlicedRegion,
};
use slicer_runtime::{
    build_and_commit_layer_selection_index, Blackboard, CompiledModuleBuilder, CompiledStage,
    ExecutionPlan,
};
use slicer_sdk::test_support::property_testing::PropertyRun;
use slicer_wasm_host::selection::{paint_allows_layer, LayerTargetSource};

/// Registry identity of this charter.
pub const PROPERTY_ID: &str = "H-RUNTIME-DISPATCH";
/// Exact registry test name.
pub const TEST_NAME: &str = "proptest_runtime_semantic_dispatch";
/// Neutral module-backed layer stage: no fill/support/non-region claim gates.
pub const NEUTRAL_STAGE: &str = "Layer::SlicePostProcess";
/// The single binding used by the real-selection fixture.
pub const MODULE_ID: &str = "com.pbt.runtime-dispatch";
/// Every generated region shares this object; ids stay unique per region index.
pub const SCENE_OBJECT: &str = "dispatch-scene-object";

/// Fixed normative semantic names.
///
/// They are a fixed part of the universe (never truncated by `max_text_bytes`)
/// and include one mixed-case spelling so case-sensitivity is reachable without
/// inventing a name outside the generated semantic universe.
pub const NORMATIVE_NAMES: [&str; 5] = ["material", "density", "seam", "wall", "Material"];

/// Written safety ceilings from the packet's registry section (domain design,
/// not measured runtime claims).
pub const MAX_LAYERS_CEILING: u64 = 4;
/// Ceiling for candidate regions per slice.
pub const MAX_REGIONS_CEILING: u64 = 4;
/// Ceiling for distinct semantic choices in the generated universe.
pub const MAX_SEMANTICS_CEILING: u64 = 8;
/// Ceiling for ordered chain entries per region.
pub const MAX_CHAIN_LEN_CEILING: u64 = 4;
/// Floor for bytes per generated (variable) semantic name.
pub const MIN_TEXT_BYTES: u64 = 8;
/// Ceiling for bytes per generated (variable) semantic name.
pub const MAX_TEXT_BYTES: u64 = 64;

/// Published paint-variant id stride, restated here so the fixture's
/// map-space address derivation does not call production's own key helper.
const PAINT_VARIANT_STRIDE: u64 = 1_000_000;

/// Checked generation bounds for `H-RUNTIME-DISPATCH`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DispatchLimits {
    /// Maximum committed global layers.
    pub max_layers: u32,
    /// Maximum candidate regions per slice.
    pub max_regions: usize,
    /// Maximum distinct semantic names in the universe.
    pub max_semantics: usize,
    /// Maximum ordered chain entries per region.
    pub max_chain_len: usize,
    /// Maximum bytes per generated variable semantic name.
    pub max_text_bytes: usize,
}

/// One tagged generation mode, drawn through a `no_shrink` sampler so shrinking
/// can never change the mode a case is named for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DispatchMode {
    /// No committed slice at all.
    Absent,
    /// Present slice with an empty declared set (transparent).
    Transparent,
    /// Present slice with zero regions and a nonempty declared set.
    PresentEmpty,
    /// Present slice whose chains contain a declared name.
    Matching,
    /// Present slice whose chains contain no declared name.
    NonMatching,
    /// Present slice whose chains only carry case variants of declared names.
    CaseMismatch,
}

impl DispatchMode {
    /// Every mode, in a stable order.
    pub const ALL: [DispatchMode; 6] = [
        DispatchMode::Absent,
        DispatchMode::Transparent,
        DispatchMode::PresentEmpty,
        DispatchMode::Matching,
        DispatchMode::NonMatching,
        DispatchMode::CaseMismatch,
    ];

    /// Whether the generated scene commits a slice.
    pub fn commits_slice(self) -> bool {
        self != DispatchMode::Absent
    }
}

/// One generated dispatch scene: slice presence, ordered region chains and the
/// declared semantic set.
#[derive(Debug, Clone)]
pub struct DispatchScene {
    /// Tagged mode of this case.
    pub mode: DispatchMode,
    /// Number of committed global layers.
    pub layers: u32,
    /// Ordered region chains; `None` means the slice is absent.
    pub regions: Option<Vec<Vec<(String, PaintValue)>>>,
    /// Declared semantic set as the production API takes it.
    pub declared: HashSet<String>,
    /// Declared names in generation order (a deterministic witness pick).
    pub declared_ordered: Vec<String>,
}

/// Validate every registry bound against the written safety ceiling before any
/// strategy allocates. The routine/extended profiles carry smaller values, so
/// the checked range is the ceiling and any profile value inside it is accepted
/// unchanged.
pub fn checked_dispatch_limits(
    max_layers: u64,
    max_regions: u64,
    max_semantics: u64,
    max_chain_len: u64,
    max_text_bytes: u64,
) -> Result<DispatchLimits, String> {
    check_bound("max_layers", max_layers, 2, MAX_LAYERS_CEILING)?;
    check_bound("max_regions", max_regions, 2, MAX_REGIONS_CEILING)?;
    check_bound("max_semantics", max_semantics, 2, MAX_SEMANTICS_CEILING)?;
    check_bound("max_chain_len", max_chain_len, 2, MAX_CHAIN_LEN_CEILING)?;
    check_bound(
        "max_text_bytes",
        max_text_bytes,
        MIN_TEXT_BYTES,
        MAX_TEXT_BYTES,
    )?;
    let max_layers =
        u32::try_from(max_layers).map_err(|_| String::from("max_layers does not fit u32"))?;
    let max_regions =
        usize::try_from(max_regions).map_err(|_| String::from("max_regions does not fit usize"))?;
    let max_semantics = usize::try_from(max_semantics)
        .map_err(|_| String::from("max_semantics does not fit usize"))?;
    let max_chain_len = usize::try_from(max_chain_len)
        .map_err(|_| String::from("max_chain_len does not fit usize"))?;
    let max_text_bytes = usize::try_from(max_text_bytes)
        .map_err(|_| String::from("max_text_bytes does not fit usize"))?;
    max_regions
        .checked_mul(max_chain_len)
        .ok_or_else(|| String::from("max_regions * max_chain_len overflows usize"))?;
    Ok(DispatchLimits {
        max_layers,
        max_regions,
        max_semantics,
        max_chain_len,
        max_text_bytes,
    })
}

fn check_bound(name: &str, value: u64, low: u64, high: u64) -> Result<(), String> {
    if (low..=high).contains(&value) {
        Ok(())
    } else {
        Err(format!("{name} must be in {low}..={high}, got {value}"))
    }
}

/// Total dispatch strategy: every field derives from the raw draws, so no case
/// can be rejected and shrinking never leaves the tagged mode.
pub fn dispatch_strategy(limits: &DispatchLimits) -> BoxedStrategy<DispatchScene> {
    let limits = *limits;
    (
        proptest::sample::select(DispatchMode::ALL.to_vec()).no_shrink(),
        (2u32..=limits.max_layers),
        any::<u32>(),
        any::<u32>(),
        proptest::collection::vec(any::<u32>(), limits.max_regions * limits.max_chain_len),
    )
        .prop_map(move |(mode, layers, region_seed, mask, seeds)| {
            build_scene(mode, layers, region_seed, mask, &seeds, &limits)
        })
        .boxed()
}

fn build_scene(
    mode: DispatchMode,
    layers: u32,
    region_seed: u32,
    mask: u32,
    seeds: &[u32],
    limits: &DispatchLimits,
) -> DispatchScene {
    let declared_ordered = declared_set(mode, mask, seeds, limits);
    DispatchScene {
        mode,
        layers,
        regions: build_regions(mode, region_seed, &declared_ordered, seeds, limits),
        declared: declared_ordered.iter().cloned().collect(),
        declared_ordered,
    }
}

fn semantic_universe(limits: &DispatchLimits, seeds: &[u32]) -> Vec<String> {
    let mut names: Vec<String> = NORMATIVE_NAMES
        .iter()
        .take(limits.max_semantics)
        .map(|name| (*name).to_string())
        .collect();
    let mut index = 0usize;
    while names.len() < limits.max_semantics {
        let seed = seeds.get(index).copied().unwrap_or_default();
        // `semNN` is five bytes; the filler keeps the total inside the accepted
        // `max_text_bytes` bound without ever widening it.
        let filler_len = 1 + (seed as usize) % (limits.max_text_bytes - 5);
        let filler = "a".repeat(filler_len);
        names.push(format!("sem{index:02}{filler}"));
        index += 1;
    }
    names
}

fn declared_set(
    mode: DispatchMode,
    mask: u32,
    seeds: &[u32],
    limits: &DispatchLimits,
) -> Vec<String> {
    if mode == DispatchMode::Transparent {
        return Vec::new();
    }
    let universe = semantic_universe(limits, seeds);
    let candidates: Vec<&String> = universe
        .iter()
        .filter(|name| mode != DispatchMode::CaseMismatch || !name.chars().any(char::is_uppercase))
        .collect();
    // A proper, nonempty subset by construction: the first candidate is always
    // declared and the last is always left outside, so the non-matching modes
    // have a nonempty complement without sampling rejections.
    debug_assert!(
        candidates.len() >= 2,
        "universe always carries a case-free pair"
    );
    let mut declared = vec![candidates[0].clone()];
    for (index, name) in candidates
        .iter()
        .enumerate()
        .take(candidates.len().saturating_sub(1))
        .skip(1)
    {
        if (mask >> index) & 1 == 1 {
            declared.push((*name).clone());
        }
    }
    declared
}

fn non_matching_names(universe: &[String], declared: &[String], mode: DispatchMode) -> Vec<String> {
    match mode {
        DispatchMode::NonMatching => universe
            .iter()
            .filter(|name| !declared.contains(name))
            .cloned()
            .collect(),
        DispatchMode::CaseMismatch => {
            let mut names: Vec<String> = Vec::new();
            for name in declared.iter().chain(universe.iter()) {
                let variant = name.to_uppercase();
                if !declared.contains(&variant) && !names.contains(&variant) {
                    names.push(variant);
                }
            }
            names
        }
        _ => universe.to_vec(),
    }
}

fn build_regions(
    mode: DispatchMode,
    region_seed: u32,
    declared: &[String],
    seeds: &[u32],
    limits: &DispatchLimits,
) -> Option<Vec<Vec<(String, PaintValue)>>> {
    if !mode.commits_slice() {
        return None;
    }
    let count = match mode {
        DispatchMode::Absent => 0,
        DispatchMode::PresentEmpty => 0,
        DispatchMode::Transparent => (region_seed as usize) % (limits.max_regions + 1),
        DispatchMode::Matching | DispatchMode::NonMatching | DispatchMode::CaseMismatch => {
            1 + (region_seed as usize) % limits.max_regions
        }
    };
    let universe = semantic_universe(limits, seeds);
    let allowed = non_matching_names(&universe, declared, mode);
    let mut regions = Vec::with_capacity(count);
    for region_index in 0..count {
        regions.push(chain_for(
            mode,
            region_index,
            &allowed,
            declared,
            seeds,
            limits,
        ));
    }
    Some(regions)
}

fn chain_for(
    mode: DispatchMode,
    region_index: usize,
    allowed: &[String],
    declared: &[String],
    seeds: &[u32],
    limits: &DispatchLimits,
) -> Vec<(String, PaintValue)> {
    let chain_len = (seeds[region_index % seeds.len()] as usize) % (limits.max_chain_len + 1);
    let mut chain = Vec::with_capacity(chain_len);
    for position in 0..chain_len {
        let seed = seeds[(region_index * limits.max_chain_len + position) % seeds.len()];
        chain.push((
            allowed[(seed as usize) % allowed.len()].clone(),
            paint_value(seed),
        ));
    }
    if mode == DispatchMode::Matching && region_index == 0 {
        // Constructive guarantee: exactly one chain always carries a declared
        // name, so the matching mode never depends on sampling luck.
        let matched = (declared[0].clone(), paint_value(seeds[0]));
        if chain.is_empty() {
            chain.push(matched);
        } else {
            chain[0] = matched;
        }
    }
    chain
}

fn paint_value(seed: u32) -> PaintValue {
    // Scalar paint values are outside the declared valid region-split domain
    // this charter commits, so generated selection scenes use only the declared
    // value shapes. Value *independence* still holds: the predicate never reads
    // the value, and `mutate_value` changes it in place.
    match seed % 3 {
        0 => PaintValue::Flag(seed.is_multiple_of(2)),
        1 => PaintValue::ToolIndex(seed % 8),
        _ => PaintValue::Custom(format!("paint-value-{}", seed % 5)),
    }
}

/// Change only the value of one chain entry, keeping every semantic name.
fn mutate_value(value: &PaintValue) -> PaintValue {
    match value {
        PaintValue::Flag(flag) => PaintValue::Flag(!flag),
        PaintValue::Scalar(scalar) => PaintValue::Scalar(scalar + 1.0),
        PaintValue::ToolIndex(index) => PaintValue::ToolIndex(index + 1),
        PaintValue::Custom(text) => PaintValue::Custom(format!("{text}-mutated")),
    }
}

/// Independent semantic-set model over the scene, never a call to production.
pub fn model_allows(
    declared: &HashSet<String>,
    regions: Option<&[Vec<(String, PaintValue)>]>,
) -> bool {
    if declared.is_empty() {
        return true;
    }
    let Some(regions) = regions else {
        return true;
    };
    regions
        .iter()
        .any(|chain| chain.iter().any(|(name, _)| declared.contains(name)))
}

/// Documented map-space address of one source region: modifier-namespace ids and
/// empty chains stay verbatim, every other id is decoded by the paint stride.
fn map_space_id(source_id: u64, chain: &[(String, PaintValue)]) -> u64 {
    let modifier_namespace = source_id != u64::MAX && source_id & (1 << 63) != 0;
    if modifier_namespace || chain.is_empty() {
        source_id
    } else {
        source_id / PAINT_VARIANT_STRIDE
    }
}

fn first_region_id(index: u64) -> u64 {
    1_000 + index
}

fn slice_for(regions: &[Vec<(String, PaintValue)>]) -> SliceIR {
    SliceIR {
        regions: regions
            .iter()
            .enumerate()
            .map(|(index, chain)| SlicedRegion {
                object_id: SCENE_OBJECT.to_string(),
                region_id: first_region_id(index as u64),
                variant_chain: chain.clone(),
                effective_layer_height: 0.2,
                ..SlicedRegion::default()
            })
            .collect(),
        ..SliceIR::default()
    }
}

/// One selected model target as the committed index reported it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedTarget {
    /// Source-space key (not the config-map address).
    pub key: RegionKey,
    /// Merged membership identities.
    pub members: Vec<RegionKey>,
}

/// One `(layer, module)` selection as the committed index reported it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedSelection {
    /// Whether the binding invokes on this layer.
    pub invoke: bool,
    /// Selected model targets in committed slice order.
    pub targets: Vec<ObservedTarget>,
}

fn plan_for(layers: u32, semantics: HashSet<String>) -> ExecutionPlan {
    let module = CompiledModuleBuilder::new(MODULE_ID.to_string())
        .claims(Vec::new())
        .region_split_semantics(semantics)
        .config_view(Arc::new(ConfigView::new()))
        .build();
    ExecutionPlan {
        per_layer_stages: vec![CompiledStage {
            stage_id: NEUTRAL_STAGE.to_string(),
            modules: vec![module],
        }],
        global_layers: Arc::new(
            (0..layers)
                .map(|index| GlobalLayer {
                    index,
                    ..GlobalLayer::default()
                })
                .collect(),
        ),
        ..ExecutionPlan::default()
    }
}

fn run_selection(
    scene_regions: Option<&[Vec<(String, PaintValue)>]>,
    layers: u32,
    declared: &HashSet<String>,
) -> Result<Vec<ObservedSelection>, String> {
    let mut blackboard = Blackboard::new(Arc::new(MeshIR::default()), layers as usize);
    if let Some(regions) = scene_regions {
        let slices: Vec<SliceIR> = (0..layers)
            .map(|layer| SliceIR {
                global_layer_index: layer,
                ..slice_for(regions)
            })
            .collect();
        blackboard
            .commit_slice_ir(Arc::new(slices))
            .map_err(|error| format!("slice fixture rejected: {error}"))?;
    }
    let mut map = RegionMapIR::default();
    let config = map.intern_config(ResolvedConfig::default());
    if let Some(regions) = scene_regions {
        for layer in 0..layers {
            for (index, chain) in regions.iter().enumerate() {
                let source_id = first_region_id(index as u64);
                map.entries.insert(
                    RegionKey {
                        global_layer_index: layer,
                        object_id: SCENE_OBJECT.to_string(),
                        region_id: map_space_id(source_id, chain),
                        variant_chain: chain.clone(),
                    },
                    RegionPlan {
                        config,
                        ..RegionPlan::default()
                    },
                );
            }
        }
    }
    blackboard
        .commit_region_map(Arc::new(map))
        .map_err(|error| format!("region map fixture rejected: {error}"))?;
    let plan = plan_for(layers, declared.clone());
    build_and_commit_layer_selection_index(&plan, &mut blackboard)
        .map_err(|error| format!("neutral stage selection index rejected: {error}"))?;
    let index = blackboard
        .layer_selection_index()
        .ok_or_else(|| String::from("selection index was not published"))?;
    let mut observed = Vec::with_capacity(layers as usize);
    for layer in 0..layers {
        let selection = index
            .selection_for(layer, MODULE_ID)
            .ok_or_else(|| format!("no bound selection for layer {layer}"))?;
        let mut targets = Vec::with_capacity(selection.targets.len());
        for target in &selection.targets {
            match &target.source {
                LayerTargetSource::Model { key, members } => targets.push(ObservedTarget {
                    key: key.clone(),
                    members: members.clone(),
                }),
                other => {
                    return Err(format!(
                        "neutral stage {NEUTRAL_STAGE} reported a non-model source {other:?}"
                    ))
                }
            }
        }
        observed.push(ObservedSelection {
            invoke: selection.invoke,
            targets,
        });
    }
    Ok(observed)
}

fn expected_keys(regions: &[Vec<(String, PaintValue)>], layer: u32, allow: bool) -> Vec<RegionKey> {
    if !allow {
        return Vec::new();
    }
    regions
        .iter()
        .enumerate()
        .map(|(index, chain)| RegionKey {
            global_layer_index: layer,
            object_id: SCENE_OBJECT.to_string(),
            region_id: first_region_id(index as u64),
            variant_chain: chain.clone(),
        })
        .collect()
}

fn describe(scene: &DispatchScene) -> String {
    let shapes: Vec<usize> = scene
        .regions
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(Vec::len)
        .collect();
    format!(
        "mode={:?} layers={} region_chain_lens={shapes:?} declared={:?}",
        scene.mode, scene.layers, scene.declared_ordered
    )
}

/// Check one generated scene against the independent model and the committed
/// selection index.
pub fn check_scene(scene: &DispatchScene, limits: &DispatchLimits) -> TestCaseResult {
    // The generator's own domain promise: generated variable names respect the
    // accepted byte bound and chains respect the accepted length bound.
    for chain in scene.regions.iter().flatten() {
        prop_assert!(
            chain.len() <= limits.max_chain_len,
            "generated chain of {} entries exceeds max_chain_len {} ({})",
            chain.len(),
            limits.max_chain_len,
            describe(scene)
        );
        for (name, _) in chain {
            prop_assert!(
                name.len() <= limits.max_text_bytes,
                "generated semantic name {name:?} of {} bytes exceeds max_text_bytes {} ({})",
                name.len(),
                limits.max_text_bytes,
                describe(scene)
            );
        }
    }
    for name in &scene.declared_ordered {
        prop_assert!(
            name.len() <= limits.max_text_bytes,
            "declared semantic name {name:?} of {} bytes exceeds max_text_bytes {} ({})",
            name.len(),
            limits.max_text_bytes,
            describe(scene)
        );
    }

    let expected_allow = model_allows(&scene.declared, scene.regions.as_deref());
    let slice = scene.regions.as_deref().map(slice_for);
    prop_assert_eq!(
        paint_allows_layer(&scene.declared, slice.as_ref()),
        expected_allow,
        "paint_allows_layer disagreed with the semantic-set model ({})",
        describe(scene)
    );

    // Value-only mutation: same names, different values, identical verdict.
    let mutated: Option<Vec<Vec<(String, PaintValue)>>> = scene.regions.as_ref().map(|regions| {
        regions
            .iter()
            .map(|chain| {
                chain
                    .iter()
                    .map(|(name, value)| (name.clone(), mutate_value(value)))
                    .collect()
            })
            .collect()
    });
    let mutated_slice = mutated.as_deref().map(slice_for);
    prop_assert_eq!(
        paint_allows_layer(&scene.declared, mutated_slice.as_ref()),
        expected_allow,
        "value-only mutation changed the paint verdict ({})",
        describe(scene)
    );
    prop_assert_eq!(
        model_allows(&scene.declared, mutated.as_deref()),
        expected_allow,
        "value-only mutation changed the independent model ({})",
        describe(scene)
    );

    // Region-order permutation: the predicate is order-invariant.
    let permuted: Option<Vec<Vec<(String, PaintValue)>>> = scene.regions.as_ref().map(|regions| {
        let mut permuted = regions.clone();
        permuted.reverse();
        permuted
    });
    let permuted_slice = permuted.as_deref().map(slice_for);
    prop_assert_eq!(
        paint_allows_layer(&scene.declared, permuted_slice.as_ref()),
        expected_allow,
        "region-order permutation changed the paint verdict ({})",
        describe(scene)
    );

    // Mode-level expectation and real committed selection.
    let observed = match run_selection(scene.regions.as_deref(), scene.layers, &scene.declared) {
        Ok(observed) => observed,
        Err(message) => return Err(TestCaseError::fail(message)),
    };
    let candidates = scene.regions.as_deref().unwrap_or(&[]);
    let expected_target_count = match scene.mode {
        DispatchMode::Absent => 0,
        DispatchMode::Transparent | DispatchMode::Matching => {
            if expected_allow {
                candidates.len()
            } else {
                0
            }
        }
        DispatchMode::PresentEmpty | DispatchMode::NonMatching | DispatchMode::CaseMismatch => 0,
    };
    prop_assert_eq!(
        observed.len(),
        scene.layers as usize,
        "expected one selection per committed layer ({})",
        describe(scene)
    );
    for (layer, selection) in observed.iter().enumerate() {
        let layer = u32::try_from(layer).expect("layer index fits u32");
        prop_assert_eq!(
            selection.invoke,
            expected_allow,
            "layer {} invoke must equal the semantic verdict ({})",
            layer,
            describe(scene)
        );
        prop_assert_eq!(
            selection.targets.len(),
            expected_target_count,
            "layer {} target count disagrees with the mode expectation ({})",
            layer,
            describe(scene)
        );
        let keys = expected_keys(candidates, layer, expected_allow);
        prop_assert_eq!(
            selection
                .targets
                .iter()
                .map(|target| target.key.clone())
                .collect::<Vec<_>>(),
            keys.clone(),
            "layer {} selected model keys must be every candidate in committed order ({})",
            layer,
            describe(scene)
        );
        for target in &selection.targets {
            prop_assert_eq!(
                &target.members,
                &vec![target.key.clone()],
                "a neutral model target must carry exactly its own source key ({})",
                describe(scene)
            );
        }
    }

    // The reversed committed order must surface verbatim, never sorted.
    let reversed = match run_selection(permuted.as_deref(), scene.layers, &scene.declared) {
        Ok(observed) => observed,
        Err(message) => return Err(TestCaseError::fail(message)),
    };
    for (layer, selection) in reversed.iter().enumerate() {
        let layer = u32::try_from(layer).expect("layer index fits u32");
        // The permuted scene commits the reversed chain list, and `slice_for`
        // assigns region ids by committed position, so the expectation must
        // derive ids from that same list rather than reversing the original
        // keys (which would keep each chain on its original id).
        let keys = expected_keys(permuted.as_deref().unwrap_or(&[]), layer, expected_allow);
        prop_assert_eq!(
            selection
                .targets
                .iter()
                .map(|target| target.key.clone())
                .collect::<Vec<_>>(),
            keys,
            "layer {} must preserve the permuted committed region order ({})",
            layer,
            describe(scene)
        );
    }
    Ok(())
}

/// Number of `DispatchMode` variants the shrink audit must reach.
const SHRINK_AUDIT_MODES: usize = DispatchMode::ALL.len();

/// Fixed-seed AC-N2 shrink audit.
///
/// Walks `simplify()` on 128 fixed-seed trees and proves, per tree, that every
/// shrunk witness still carries the case's `DispatchMode` and still satisfies
/// `check_scene` — the same checker `run_property` runs. The mode sampler is
/// `no_shrink`, so a lost mode means the generator stopped pinning the tag
/// rather than a legal simplification.
pub fn shrink_audit() {
    let limits =
        checked_dispatch_limits(3, 3, 4, 3, 16).expect("H-RUNTIME-DISPATCH shrink audit ceilings");
    let strategy = dispatch_strategy(&limits);
    let mut runner = proptest::test_runner::TestRunner::new(proptest::test_runner::Config {
        cases: 1,
        max_local_rejects: 0,
        max_global_rejects: 0,
        max_flat_map_regens: 0,
        rng_algorithm: proptest::test_runner::RngAlgorithm::ChaCha,
        rng_seed: proptest::test_runner::RngSeed::Fixed(0x5eed_c0de),
        failure_persistence: None,
        ..proptest::test_runner::Config::default()
    });
    let mut seen = std::collections::BTreeSet::new();
    let mut simplifications = 0_usize;
    for _ in 0..128 {
        let mut tree = strategy.new_tree(&mut runner).expect("shrink audit tree");
        let initial = tree.current();
        let mode = initial.mode;
        seen.insert(mode);
        check_scene(&initial, &limits).unwrap_or_else(|error| {
            panic!(
                "H-RUNTIME-DISPATCH: the unsimplified audit tree must satisfy the contract \
                 (mode {mode:?}): {error}"
            )
        });
        let mut steps = 0_usize;
        while steps < 64 && tree.simplify() {
            steps += 1;
            simplifications += 1;
            let current = tree.current();
            assert_eq!(
                current.mode, mode,
                "H-RUNTIME-DISPATCH: shrinking changed the case mode"
            );
            check_scene(&current, &limits).unwrap_or_else(|error| {
                panic!(
                    "H-RUNTIME-DISPATCH: the shrunk witness no longer satisfies the contract \
                     (mode {mode:?}): {error}"
                )
            });
        }
    }
    assert!(
        seen.len() >= SHRINK_AUDIT_MODES,
        "H-RUNTIME-DISPATCH: shrink audit reached {} modes, expected at least {}",
        seen.len(),
        SHRINK_AUDIT_MODES
    );
    assert!(
        simplifications > 0,
        "H-RUNTIME-DISPATCH: the shrink audit simplified nothing, so it certifies nothing"
    );
}

/// Run the registry-selected `H-RUNTIME-DISPATCH` property.
pub fn run_property() {
    let run =
        PropertyRun::new(PROPERTY_ID, TEST_NAME).expect("H-RUNTIME-DISPATCH registry context");
    let limits = checked_dispatch_limits(
        run.bound("max_layers").expect("max_layers bound"),
        run.bound("max_regions").expect("max_regions bound"),
        run.bound("max_semantics").expect("max_semantics bound"),
        run.bound("max_chain_len").expect("max_chain_len bound"),
        run.bound("max_text_bytes").expect("max_text_bytes bound"),
    )
    .expect("H-RUNTIME-DISPATCH safety ceilings");

    shrink_audit();
    let strategy = dispatch_strategy(&limits);
    run.check(&strategy, |scene, _guard| check_scene(&scene, &limits))
        .expect("H-RUNTIME-DISPATCH semantic-set dispatch contract");
}
