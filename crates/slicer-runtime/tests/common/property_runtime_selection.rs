//! Charter `H-RUNTIME-SELECTION`: exact config authority and atomic publication.
//!
//! The property drives the real runtime publication seam
//! (`slicer_runtime::build_and_commit_layer_selection_index`) and reads back every
//! bound `(module, layer)` selection. It compares, per target:
//!
//! * the source-space identity (`LayerTargetSource::Model { key, members }`),
//! * the ordered committed slice identity (`slice_regions`),
//! * the per-target `line_width` marker read from the exact interned config.
//!
//! The expected table is derived from the fixture itself: the map-space config
//! address is recomputed from the documented rule (modifier-namespace ids and
//! empty chains stay verbatim, every other id is decoded by the paint stride)
//! and never by calling `region_map_key_for_sliced_region`. Every scene marker is
//! a distinct nondefault finite value, so a default / sibling / first-match
//! lookup cannot accidentally agree with the independent table.
//!
//! Slot 1 of every layer is a cross-object twin of slot 0: it repeats slot 0's
//! numeric source id and ordered chain verbatim under a third object id, so the
//! full identity differs while an object-blind `(region_id, chain)` key would
//! collapse the pair. The positive checker asserts both targets exist with their
//! own markers and exact object ids on every layer, including the reversed
//! insertion-order leg.
//!
//! The negative mode removes one exact `RegionKey` while leaving a
//! same-object/same-region default entry, a different-chain sibling and a
//! reversed-chain entry present, and asserts both the pure builder's
//! `MissingModelConfig` and the runtime wrapper's `FatalModule` with no
//! published index even though layer 0 was valid.

#![allow(missing_docs)]

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use proptest::prelude::*;
use proptest::strategy::BoxedStrategy;
use proptest::test_runner::{TestCaseError, TestCaseResult};
use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, GlobalLayer, MeshIR, PaintValue, Point2, Polygon,
    RegionKey, RegionMapIR, RegionPlan, ResolvedConfig, SliceIR, SlicedRegion,
};
use slicer_runtime::{
    build_and_commit_layer_selection_index, Blackboard, CompiledModuleBuilder, CompiledStage,
    ExecutionPlan, LayerStageError,
};
use slicer_sdk::test_support::property_testing::PropertyRun;
use slicer_wasm_host::selection::{
    build_layer_selection_index, LayerSelectionIndex, LayerSelectionIndexInput,
    LayerSelectionModuleInput, LayerTargetSource, SelectionError, STAGES,
};

/// Registry identity of this charter.
pub const PROPERTY_ID: &str = "H-RUNTIME-SELECTION";
/// Exact registry test name.
pub const TEST_NAME: &str = "proptest_runtime_selection_authority";
/// Neutral module-backed layer stage: no fill/support/non-region claim gates.
pub const NEUTRAL_STAGE: &str = "Layer::SlicePostProcess";
/// Module id used for the unbound-pair lookup.
pub const UNBOUND_MODULE_ID: &str = "com.pbt.runtime-selection-unbound";
/// Object used on even layers; adjacent layers use a different object so the
/// same numeric region id occurs under two distinct full identities.
pub const SCENE_OBJECT_EVEN: &str = "selection-scene-even";
/// Object used on odd layers.
pub const SCENE_OBJECT_ODD: &str = "selection-scene-odd";
/// Third object used by every layer's cross-object twin. The twin repeats
/// another region's numeric id and ordered chain, so only this object component
/// keeps the full identity distinct.
pub const SCENE_OBJECT_THIRD: &str = "selection-scene-third";

/// Published paint-variant id stride, restated so the expected table derives
/// the config-map address independently of production's key helper.
pub const PAINT_VARIANT_STRIDE: u64 = 1_000_000;
/// First ordinary (empty-chain) region id.
pub const ORDINARY_ID_BASE: u64 = 1;
/// First base id for painted variant regions.
pub const PAINT_BASE: u64 = 100;
/// First base id for modifier-namespace regions.
pub const MODIFIER_BASE: u64 = 200;
/// Decoy marker for a same-object/same-region default (empty-chain) entry.
pub const DEFAULT_DECOY_MARKER: f64 = 0.901;
/// Decoy marker for a same-object/same-region different-chain sibling.
pub const SIBLING_DECOY_MARKER: f64 = 0.902;
/// Decoy marker for a reversed ordered chain.
pub const REVERSED_DECOY_MARKER: f64 = 0.903;

/// Written safety ceilings from the packet's registry section.
pub const MAX_LAYERS_CEILING: u64 = 4;
/// Ceiling for regions per layer.
pub const MAX_REGIONS_CEILING: u64 = 4;
/// Ceiling for bound modules.
pub const MAX_MODULES_CEILING: u64 = 4;
/// Ceiling for ordered chain entries per region.
pub const MAX_CHAIN_LEN_CEILING: u64 = 4;

/// Checked generation bounds for `H-RUNTIME-SELECTION`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionLimits {
    /// Maximum committed global layers.
    pub max_layers: u32,
    /// Maximum regions per layer.
    pub max_regions: usize,
    /// Maximum bound modules.
    pub max_modules: usize,
    /// Maximum ordered chain entries per region.
    pub max_chain_len: usize,
}

/// Validate every registry bound against the written safety ceiling before any
/// strategy allocates. Smaller routine/extended values inside the ceiling are
/// accepted unchanged.
pub fn checked_selection_limits(
    max_layers: u64,
    max_regions: u64,
    max_modules: u64,
    max_chain_len: u64,
) -> Result<SelectionLimits, String> {
    check_bound("max_layers", max_layers, 2, MAX_LAYERS_CEILING)?;
    check_bound("max_regions", max_regions, 2, MAX_REGIONS_CEILING)?;
    check_bound("max_modules", max_modules, 2, MAX_MODULES_CEILING)?;
    check_bound("max_chain_len", max_chain_len, 2, MAX_CHAIN_LEN_CEILING)?;
    let max_layers =
        u32::try_from(max_layers).map_err(|_| String::from("max_layers does not fit u32"))?;
    let max_regions =
        usize::try_from(max_regions).map_err(|_| String::from("max_regions does not fit usize"))?;
    let max_modules =
        usize::try_from(max_modules).map_err(|_| String::from("max_modules does not fit usize"))?;
    let max_chain_len = usize::try_from(max_chain_len)
        .map_err(|_| String::from("max_chain_len does not fit usize"))?;
    max_regions
        .checked_mul(max_chain_len)
        .ok_or_else(|| String::from("max_regions * max_chain_len overflows usize"))?;
    Ok(SelectionLimits {
        max_layers,
        max_regions,
        max_modules,
        max_chain_len,
    })
}

fn check_bound(name: &str, value: u64, low: u64, high: u64) -> Result<(), String> {
    if (low..=high).contains(&value) {
        Ok(())
    } else {
        Err(format!("{name} must be in {low}..={high}, got {value}"))
    }
}

/// Generation kinds: every scene carries ordinary empty-chain, painted
/// variant and modifier-namespace identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RegionKind {
    /// Ordinary region with an empty ordered chain.
    Ordinary,
    /// Painted variant id with a nonempty ordered chain.
    Painted,
    /// Modifier-namespace id (chain either empty or nonempty).
    Modifier,
}

/// One generated scene. Everything below is derived from these raw draws, so
/// the strategy is total and shrinking cannot change the shape it names.
#[derive(Debug, Clone)]
pub struct SelectionScene {
    /// Committed global layer count.
    pub layers: u32,
    /// Regions committed on every layer.
    pub regions_per_layer: usize,
    /// Bound module ids, in plan order.
    pub modules: Vec<String>,
    /// Whether this case runs the missing-authority negative mode.
    pub negative: bool,
    /// Raw entropy, fixed length `max_regions * max_chain_len`.
    pub seeds: Vec<u32>,
}

/// Total scene strategy: no rejection budget, no filtered draws.
pub fn selection_strategy(limits: &SelectionLimits) -> BoxedStrategy<SelectionScene> {
    let limits = *limits;
    (
        (2u32..=limits.max_layers),
        (2usize..=limits.max_regions),
        (1usize..=limits.max_modules),
        proptest::sample::select(vec![false, true]).no_shrink(),
        proptest::collection::vec(any::<u32>(), limits.max_regions * limits.max_chain_len),
    )
        .prop_map(
            move |(layers, regions_per_layer, module_count, negative, seeds)| SelectionScene {
                layers,
                regions_per_layer,
                modules: (0..module_count)
                    .map(|index| format!("com.pbt.runtime-selection-{index:02}"))
                    .collect(),
                negative,
                seeds,
            },
        )
        .boxed()
}

/// One fixture region with its independently derived authority identities.
#[derive(Debug, Clone)]
pub struct ExpectedRegion {
    /// Committed layer of this region.
    pub layer: u32,
    /// Generation kind.
    pub kind: RegionKind,
    /// SOURCE-space object identity.
    pub object_id: String,
    /// SOURCE-space region id (verbatim in the committed slice).
    pub source_id: u64,
    /// Ordered paint chain, kept exactly as committed.
    pub chain: Vec<(String, PaintValue)>,
    /// Exact config-map address for this source identity.
    pub map_key: RegionKey,
    /// Exact source-space identity production must report.
    pub source_key: RegionKey,
    /// Unique nondefault `line_width` marker for this region.
    pub marker: f64,
}

/// Documented map-space address derivation, independent of production's helper.
fn map_space_id(source_id: u64, chain: &[(String, PaintValue)]) -> u64 {
    let modifier_namespace = source_id != u64::MAX && source_id & (1 << 63) != 0;
    if modifier_namespace || chain.is_empty() {
        source_id
    } else {
        source_id / PAINT_VARIANT_STRIDE
    }
}

fn object_for_layer(layer: u32) -> String {
    if layer.is_multiple_of(2) {
        SCENE_OBJECT_EVEN.to_string()
    } else {
        SCENE_OBJECT_ODD.to_string()
    }
}

/// Object of one committed region slot on a layer.
///
/// Slot 1 is the cross-object twin of slot 0: it must not share slot 0's object,
/// otherwise the repeated numeric id and chain would be a duplicate full
/// identity. Slots 2 and 3 use distinct identities of their own, so any object
/// is safe.
fn object_for_index(layer: u32, index: usize) -> String {
    match index {
        0 => object_for_layer(layer),
        1 => SCENE_OBJECT_THIRD.to_string(),
        2 => object_for_layer(layer + 1),
        _ => object_for_layer(layer),
    }
}

/// The identity slot a committed region repeats.
///
/// Slot 1 repeats slot 0's numeric source id and ordered chain verbatim; only
/// its object differs. Every other slot owns a unique identity.
fn identity_slot(index: usize) -> usize {
    if index == 1 {
        0
    } else {
        index
    }
}

fn chain_name(layer: u32, index: usize, position: usize) -> String {
    format!("sem-{layer}-{index}-{position}")
}

fn modifier_footprint(layer: u32, index: usize) -> ExPolygon {
    let origin = (layer as i64) * 100 + index as i64;
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 {
                    x: origin,
                    y: origin,
                },
                Point2 {
                    x: origin + 10,
                    y: origin,
                },
                Point2 {
                    x: origin + 10,
                    y: origin + 10,
                },
                Point2 {
                    x: origin,
                    y: origin + 10,
                },
            ],
        },
        holes: Vec::new(),
    }
}

/// The committed chain and source identity for one identity slot.
fn expected_chain(
    layer: u32,
    slot: usize,
    seed: u32,
    limits: &SelectionLimits,
) -> (RegionKind, u64, Vec<(String, PaintValue)>) {
    let object_id = object_for_index(layer, slot);
    match slot {
        0 => (
            RegionKind::Ordinary,
            // Layer-qualified so same-parity layers (which share an object id)
            // never commit the same full identity.
            ORDINARY_ID_BASE + layer as u64 * 100,
            Vec::new(),
        ),
        2 => {
            let base = PAINT_BASE + layer as u64 * 100 + slot as u64;
            let chain_len = 2 + (seed as usize) % (limits.max_chain_len - 1);
            let chain = (0..chain_len)
                .map(|position| {
                    (
                        chain_name(layer, slot, position),
                        PaintValue::ToolIndex((position as u32) % 3 + 1),
                    )
                })
                .collect();
            let payload = 1 + u64::from(seed) % (PAINT_VARIANT_STRIDE - 1);
            (
                RegionKind::Painted,
                base * PAINT_VARIANT_STRIDE + payload,
                chain,
            )
        }
        _ => {
            let base = MODIFIER_BASE + layer as u64 * 100 + slot as u64;
            let footprint = modifier_footprint(layer, slot);
            let source_id = slicer_ir::modifier_sub_region_id(base, &object_id, &[footprint]);
            let chain = if (layer as usize + slot).is_multiple_of(2) {
                Vec::new()
            } else {
                let chain_len = 2 + (seed as usize) % (limits.max_chain_len - 1);
                (0..chain_len)
                    .map(|position| {
                        (
                            chain_name(layer, slot, position),
                            PaintValue::Flag(position % 2 == 0),
                        )
                    })
                    .collect()
            };
            (RegionKind::Modifier, source_id, chain)
        }
    }
}

/// Derive the full expected table for one scene, in committed order.
pub fn expected_regions(scene: &SelectionScene, limits: &SelectionLimits) -> Vec<ExpectedRegion> {
    let mut regions = Vec::with_capacity(scene.layers as usize * scene.regions_per_layer);
    for layer in 0..scene.layers {
        for index in 0..scene.regions_per_layer {
            // The twin reuses slot 0's seed as well as its id and chain, so a
            // seed-driven chain length or payload can never break the mirror.
            let slot = identity_slot(index);
            let unique = layer as usize * limits.max_regions + slot;
            let seed = scene.seeds[unique % scene.seeds.len()];
            let (kind, source_id, chain) = expected_chain(layer, slot, seed, limits);
            let object_id = object_for_index(layer, index);
            let map_id = map_space_id(source_id, &chain);
            let source_key = RegionKey {
                global_layer_index: layer,
                object_id: object_id.clone(),
                region_id: source_id,
                variant_chain: chain.clone(),
            };
            let map_key = RegionKey {
                region_id: map_id,
                ..source_key.clone()
            };
            regions.push(ExpectedRegion {
                layer,
                kind,
                object_id,
                source_id,
                chain,
                map_key,
                source_key,
                // Distinct per region by construction and never the 0.0
                // default, so a default/sibling lookup cannot coincide. The raw
                // index keeps the twin's marker distinct from slot 0's.
                marker: 0.15 + 0.05 * (layer as usize * limits.max_regions + index) as f64,
            });
        }
    }
    regions
}

fn slice_fixture(scene: &SelectionScene, regions: &[ExpectedRegion]) -> Vec<SliceIR> {
    (0..scene.layers)
        .map(|layer| SliceIR {
            global_layer_index: layer,
            regions: regions
                .iter()
                .filter(|region| region.layer == layer)
                .map(|region| SlicedRegion {
                    object_id: region.object_id.clone(),
                    region_id: region.source_id,
                    variant_chain: region.chain.clone(),
                    effective_layer_height: 0.2,
                    ..SlicedRegion::default()
                })
                .collect(),
            ..SliceIR::default()
        })
        .collect()
}

fn invocation_view() -> ConfigView {
    ConfigView::from_map(HashMap::from([(
        "line_width".to_string(),
        ConfigValue::Float(0.5),
    )]))
}

fn plan_fixture(scene: &SelectionScene, module_order: &[String]) -> ExecutionPlan {
    let semantics: HashSet<String> = HashSet::new();
    let invocation = Arc::new(invocation_view());
    let modules = module_order
        .iter()
        .map(|module_id| {
            CompiledModuleBuilder::new(module_id.clone())
                .claims(Vec::new())
                .region_split_semantics(semantics.clone())
                .config_view(Arc::clone(&invocation))
                .build()
        })
        .collect();
    ExecutionPlan {
        per_layer_stages: vec![CompiledStage {
            stage_id: NEUTRAL_STAGE.to_string(),
            modules,
        }],
        global_layers: Arc::new(
            (0..scene.layers)
                .map(|index| GlobalLayer {
                    index,
                    ..GlobalLayer::default()
                })
                .collect(),
        ),
        ..ExecutionPlan::default()
    }
}

/// One selected model target and the marker its exact config carried.
#[derive(Debug, Clone, PartialEq)]
pub struct ObservedTarget {
    /// Source-space key production reported.
    pub key: RegionKey,
    /// Merged membership identities production reported.
    pub members: Vec<RegionKey>,
    /// `line_width` from the target's config view.
    pub marker: Option<f64>,
}

/// One `(module, layer)` selection as the committed index reported it.
#[derive(Debug, Clone, PartialEq)]
pub struct ObservedSelection {
    /// Bound module.
    pub module: String,
    /// Committed layer.
    pub layer: u32,
    /// Whether the binding invokes on this layer.
    pub invoke: bool,
    /// Selected model targets in committed slice order.
    pub targets: Vec<ObservedTarget>,
    /// Ordered committed identities retained for the later perimeter products.
    pub slice_regions: Vec<(String, u64, Vec<(String, PaintValue)>)>,
}

/// Entry insertion order for the `RegionMapIR` fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertionOrder {
    /// Authored order.
    Forward,
    /// Reversed order.
    Reverse,
}

/// The exact key removed by the negative mode plus its surviving decoys.
#[derive(Debug, Clone)]
pub struct NegativeWitness {
    /// Exact `RegionKey` whose config authority is absent.
    pub removed: RegionKey,
    /// Default / sibling / reversed-chain entries that must not satisfy it.
    pub decoys: Vec<(RegionKey, f64)>,
}

/// Build the exact-key config authority for one scene.
pub fn region_map_fixture(
    regions: &[ExpectedRegion],
    order: InsertionOrder,
    negative: Option<&NegativeWitness>,
) -> RegionMapIR {
    let mut entries: Vec<(RegionKey, f64)> = regions
        .iter()
        .map(|region| (region.map_key.clone(), region.marker))
        .collect();
    if let Some(witness) = negative {
        entries.retain(|(key, _)| *key != witness.removed);
        entries.extend(witness.decoys.iter().cloned());
    }
    if order == InsertionOrder::Reverse {
        entries.reverse();
    }
    let mut map = RegionMapIR::default();
    for (key, marker) in entries {
        let config = map.intern_config(ResolvedConfig {
            line_width: marker,
            ..ResolvedConfig::default()
        });
        map.entries.insert(
            key,
            RegionPlan {
                config,
                ..RegionPlan::default()
            },
        );
    }
    map
}

/// Build the pure-builder input and run it. Everything the input borrows is
/// owned by this function for the duration of the call.
pub fn run_pure_builder(
    scene: &SelectionScene,
    regions: &[ExpectedRegion],
    map: &RegionMapIR,
    module_order: &[String],
) -> Result<LayerSelectionIndex, slicer_wasm_host::selection::LayerSelectionBuildError> {
    let slices = slice_fixture(scene, regions);
    let layers: Vec<GlobalLayer> = (0..scene.layers)
        .map(|index| GlobalLayer {
            index,
            ..GlobalLayer::default()
        })
        .collect();
    let claims: Vec<String> = Vec::new();
    let semantics: HashSet<String> = HashSet::new();
    let invocation = invocation_view();
    let modules: Vec<LayerSelectionModuleInput<'_>> = module_order
        .iter()
        // exhaustive: borrowed static binding authority has no Default; every field is consumed by the selection builder, so omitting one would fake the fixture.
        .map(|module_id| LayerSelectionModuleInput {
            stage_id: NEUTRAL_STAGE,
            module_id: module_id.as_str(),
            claims: &claims,
            paint_semantics: Some(&semantics),
            invocation_config: &invocation,
        })
        .collect();
    // exhaustive: the whole-print snapshot supplies every authority channel explicitly, including the absent support plan and the empty perimeter roster.
    build_layer_selection_index(LayerSelectionIndexInput {
        stage_roster: STAGES,
        global_layers: &layers,
        modules: &modules,
        slices: &slices,
        region_map: Some(map),
        support_plan: None,
        perimeters: &[],
    })
}

/// Build and commit through the real runtime seam, returning the blackboard so
/// callers can observe its published state.
pub fn run_runtime_seam(
    scene: &SelectionScene,
    regions: &[ExpectedRegion],
    map: RegionMapIR,
    module_order: &[String],
) -> (Blackboard, Result<(), LayerStageError>) {
    let mut blackboard = Blackboard::new(Arc::new(MeshIR::default()), scene.layers as usize);
    blackboard
        .commit_slice_ir(Arc::new(slice_fixture(scene, regions)))
        .expect("selection slice fixture must be accepted");
    blackboard
        .commit_region_map(Arc::new(map))
        .expect("selection region-map fixture must be accepted");
    let plan = plan_fixture(scene, module_order);
    let result = build_and_commit_layer_selection_index(&plan, &mut blackboard);
    (blackboard, result)
}

/// Read every bound `(module, layer)` selection from a published index.
pub fn observe_selections(
    index: &LayerSelectionIndex,
    scene: &SelectionScene,
    module_order: &[String],
) -> Vec<ObservedSelection> {
    let mut observed = Vec::with_capacity(module_order.len() * scene.layers as usize);
    for module in module_order {
        for layer in 0..scene.layers {
            let selection = index
                .selection_for(layer, module)
                .expect("every bound (layer, module) pair must be present");
            let targets = selection
                .targets
                .iter()
                .map(|target| match &target.source {
                    LayerTargetSource::Model { key, members } => ObservedTarget {
                        key: key.clone(),
                        members: members.clone(),
                        marker: target.config.get_float("line_width"),
                    },
                    other => {
                        panic!("neutral stage {NEUTRAL_STAGE} yielded non-model source {other:?}")
                    }
                })
                .collect();
            let slice_regions = selection
                .slice_regions
                .iter()
                .map(|region| {
                    (
                        region.object_id.clone(),
                        region.region_id,
                        region.variant_chain.clone(),
                    )
                })
                .collect();
            observed.push(ObservedSelection {
                module: module.clone(),
                layer,
                invoke: selection.invoke,
                targets,
                slice_regions,
            });
        }
    }
    observed
}

/// Build the negative witness: one ordinary region on layer >= 1 loses its exact
/// key while a default entry at another layer, a different-chain sibling and a
/// reversed two-entry chain stay present.
///
/// The reversed-chain decoy is fixed rather than copied from a committed painted
/// region, so the witness exists for every generated region count (a 2-region
/// layer carries the cross-object twin pair but no painted slot).
pub fn negative_witness(regions: &[ExpectedRegion]) -> NegativeWitness {
    let removed = regions
        .iter()
        .find(|region| region.layer == 1 && region.kind == RegionKind::Ordinary)
        .expect("the fixture always commits an ordinary region on layer 1")
        .source_key
        .clone();
    let mut reversed_chain = vec![
        (String::from("decoy-a"), PaintValue::ToolIndex(1)),
        (String::from("decoy-b"), PaintValue::ToolIndex(2)),
    ];
    reversed_chain.reverse();
    let mut reversed_key = removed.clone();
    reversed_key.variant_chain = reversed_chain;
    let decoys = vec![
        (
            RegionKey {
                global_layer_index: 0,
                object_id: removed.object_id.clone(),
                region_id: removed.region_id,
                variant_chain: Vec::new(),
            },
            DEFAULT_DECOY_MARKER,
        ),
        (
            RegionKey {
                global_layer_index: removed.global_layer_index,
                object_id: removed.object_id.clone(),
                region_id: removed.region_id,
                variant_chain: vec![(String::from("sibling-semantic"), PaintValue::Flag(true))],
            },
            SIBLING_DECOY_MARKER,
        ),
        (reversed_key, REVERSED_DECOY_MARKER),
    ];
    NegativeWitness { removed, decoys }
}

fn describe(scene: &SelectionScene) -> String {
    format!(
        "layers={} regions_per_layer={} modules={:?} negative={}",
        scene.layers, scene.regions_per_layer, scene.modules, scene.negative
    )
}

fn sorted_sequence(
    observed: &[ObservedSelection],
) -> Vec<(String, u32, Vec<(RegionKey, Option<f64>)>)> {
    let mut sequence: Vec<(String, u32, Vec<(RegionKey, Option<f64>)>)> = observed
        .iter()
        .map(|selection| {
            (
                selection.module.clone(),
                selection.layer,
                selection
                    .targets
                    .iter()
                    .map(|target| (target.key.clone(), target.marker))
                    .collect(),
            )
        })
        .collect();
    // `Option<f64>` is not `Ord`; (module, layer) is a total unique key here.
    sequence.sort_by(|left, right| (&left.0, left.1).cmp(&(&right.0, right.1)));
    sequence
}

/// Check one generated scene against the independent expectation table.
pub fn check_scene(scene: &SelectionScene, limits: &SelectionLimits) -> TestCaseResult {
    let regions = expected_regions(scene, limits);
    prop_assert_eq!(
        regions.len(),
        scene.layers as usize * scene.regions_per_layer,
        "the fixture must commit exactly one table row per committed region ({})",
        describe(scene)
    );
    // Fixture precondition from the packet: every full source identity is
    // unique, otherwise an exact-key assertion could pass on a duplicate row.
    let mut identities: Vec<(String, u64, Vec<(String, PaintValue)>)> = regions
        .iter()
        .map(|region| {
            (
                region.object_id.clone(),
                region.source_id,
                region.chain.clone(),
            )
        })
        .collect();
    identities.sort();
    let unique = identities.len();
    identities.dedup();
    prop_assert_eq!(
        identities.len(),
        unique,
        "generated full source identities must be unique ({})",
        describe(scene)
    );

    if scene.negative {
        return check_negative(scene, &regions);
    }
    check_positive(scene, &regions)
}

/// Assert one selection exposes the layer's cross-object twin pair.
///
/// Slot 0 and slot 1 repeat the same numeric source id and ordered chain under
/// different objects, so only the full object-bearing identity distinguishes
/// them. An object-blind `(region_id, chain)` key would report one target (and
/// one marker) for the pair.
fn assert_cross_object_pair(
    scene: &SelectionScene,
    regions: &[ExpectedRegion],
    selection: &ObservedSelection,
) -> TestCaseResult {
    let layer_regions: Vec<&ExpectedRegion> = regions
        .iter()
        .filter(|region| region.layer == selection.layer)
        .collect();
    let mut pairs: Vec<(&ExpectedRegion, &ExpectedRegion)> = Vec::new();
    for (position, left) in layer_regions.iter().enumerate() {
        for right in &layer_regions[position + 1..] {
            if left.source_id == right.source_id
                && left.chain == right.chain
                && left.object_id != right.object_id
            {
                pairs.push((left, right));
            }
        }
    }
    prop_assert_eq!(
        pairs.len(),
        1,
        "every layer must commit exactly one same-identity/different-object pair \
         (module {} layer {}; {})",
        selection.module,
        selection.layer,
        describe(scene)
    );
    let (left, right) = pairs[0];
    prop_assert_ne!(
        &left.object_id,
        &right.object_id,
        "the twin pair must differ in object id (module {} layer {}; {})",
        selection.module,
        selection.layer,
        describe(scene)
    );
    let find_target = |region: &ExpectedRegion| {
        selection
            .targets
            .iter()
            .find(|target| target.key == region.source_key)
    };
    let left_target = find_target(left).ok_or_else(|| {
        TestCaseError::fail(format!(
            "the object {} target of the twin pair is missing (module {} layer {}; {})",
            left.object_id,
            selection.module,
            selection.layer,
            describe(scene)
        ))
    })?;
    let right_target = find_target(right).ok_or_else(|| {
        TestCaseError::fail(format!(
            "the object {} target of the twin pair is missing (module {} layer {}; {})",
            right.object_id,
            selection.module,
            selection.layer,
            describe(scene)
        ))
    })?;
    prop_assert_eq!(
        &left_target.key.object_id,
        &left.object_id,
        "the twin target must keep its exact object id, never its sibling's \
         (module {} layer {}; {})",
        selection.module,
        selection.layer,
        describe(scene)
    );
    prop_assert_eq!(
        &right_target.key.object_id,
        &right.object_id,
        "the twin target must keep its exact object id, never its sibling's \
         (module {} layer {}; {})",
        selection.module,
        selection.layer,
        describe(scene)
    );
    prop_assert_eq!(
        left_target.key.region_id,
        right_target.key.region_id,
        "the twin pair must share the numeric region id (module {} layer {}; {})",
        selection.module,
        selection.layer,
        describe(scene)
    );
    prop_assert_eq!(
        &left_target.key.variant_chain,
        &right_target.key.variant_chain,
        "the twin pair must share the ordered chain (module {} layer {}; {})",
        selection.module,
        selection.layer,
        describe(scene)
    );
    prop_assert_eq!(
        left_target.marker,
        Some(left.marker),
        "the object {} twin must carry its own exact marker, never a shared \
         object-blind value (module {} layer {}; {})",
        left.object_id,
        selection.module,
        selection.layer,
        describe(scene)
    );
    prop_assert_eq!(
        right_target.marker,
        Some(right.marker),
        "the object {} twin must carry its own exact marker, never a shared \
         object-blind value (module {} layer {}; {})",
        right.object_id,
        selection.module,
        selection.layer,
        describe(scene)
    );
    prop_assert_ne!(
        left.marker,
        right.marker,
        "the twin pair's markers must differ, or a collapsed lookup could pass \
         (module {} layer {}; {})",
        selection.module,
        selection.layer,
        describe(scene)
    );
    let object_blind_targets = selection
        .targets
        .iter()
        .filter(|target| {
            target.key.region_id == left.source_id && target.key.variant_chain == left.chain
        })
        .count();
    prop_assert_eq!(
        object_blind_targets,
        2,
        "an object-blind (region_id, chain) key would collapse the twin pair to \
         one target (module {} layer {}; {})",
        selection.module,
        selection.layer,
        describe(scene)
    );
    Ok(())
}

fn check_positive(scene: &SelectionScene, regions: &[ExpectedRegion]) -> TestCaseResult {
    let map = region_map_fixture(regions, InsertionOrder::Forward, None);
    let (blackboard, result) = run_runtime_seam(scene, regions, map.clone(), &scene.modules);
    if let Err(error) = result {
        return Err(TestCaseError::fail(format!(
            "valid exact-key authority was rejected: {error} ({})",
            describe(scene)
        )));
    }
    let index = blackboard
        .layer_selection_index()
        .ok_or_else(|| {
            TestCaseError::fail(format!(
                "a successful publication left no selection index ({})",
                describe(scene)
            ))
        })?
        .clone();
    let observed = observe_selections(&index, scene, &scene.modules);
    prop_assert_eq!(
        observed.len(),
        scene.modules.len() * scene.layers as usize,
        "one observation per bound pair ({})",
        describe(scene)
    );
    for selection in &observed {
        prop_assert!(
            selection.invoke,
            "a transparent neutral binding must invoke on every layer ({})",
            describe(scene)
        );
        let expected: Vec<&ExpectedRegion> = regions
            .iter()
            .filter(|region| region.layer == selection.layer)
            .collect();
        prop_assert_eq!(
            selection.targets.len(),
            expected.len(),
            "every candidate region must be selected on module {} layer {} ({})",
            selection.module,
            selection.layer,
            describe(scene)
        );
        for (target, region) in selection.targets.iter().zip(&expected) {
            prop_assert_eq!(
                &target.key,
                &region.source_key,
                "target key must be the source-space identity of the region at that index \
                 (module {} layer {}; {})",
                selection.module,
                selection.layer,
                describe(scene)
            );
            prop_assert_eq!(
                &target.members,
                &vec![region.source_key.clone()],
                "a neutral model target must carry exactly its own key ({})",
                describe(scene)
            );
            prop_assert_eq!(
                target.marker,
                Some(region.marker),
                "target config must carry the exact-key marker for {:?}, never a \
                 default/sibling/first-match value (module {} layer {}; {})",
                region.source_key,
                selection.module,
                selection.layer,
                describe(scene)
            );
        }
        let identities: Vec<(String, u64, Vec<(String, PaintValue)>)> = expected
            .iter()
            .map(|region| {
                (
                    region.object_id.clone(),
                    region.source_id,
                    region.chain.clone(),
                )
            })
            .collect();
        prop_assert_eq!(
            &selection.slice_regions,
            &identities,
            "slice_regions must preserve the ordered committed source identity and chain \
             (module {} layer {}; {})",
            selection.module,
            selection.layer,
            describe(scene)
        );
        assert_cross_object_pair(scene, regions, selection)?;
    }
    prop_assert!(
        index.selection_for(0, UNBOUND_MODULE_ID).is_none(),
        "an unbound module id must not report a selection ({})",
        describe(scene)
    );

    // Insertion order and module binding order must not change the observed
    // per-module/layer `(key, marker)` sequence.
    let reversed_modules: Vec<String> = scene.modules.iter().rev().cloned().collect();
    let reversed_map = region_map_fixture(regions, InsertionOrder::Reverse, None);
    let (reversed_blackboard, reversed_result) =
        run_runtime_seam(scene, regions, reversed_map, &reversed_modules);
    if let Err(error) = reversed_result {
        return Err(TestCaseError::fail(format!(
            "reversed insertion order was rejected: {error} ({})",
            describe(scene)
        )));
    }
    let reversed_index = reversed_blackboard
        .layer_selection_index()
        .ok_or_else(|| {
            TestCaseError::fail(format!(
                "the reversed publication left no selection index ({})",
                describe(scene)
            ))
        })?
        .clone();
    let reversed_observed = observe_selections(&reversed_index, scene, &reversed_modules);
    prop_assert_eq!(
        sorted_sequence(&observed),
        sorted_sequence(&reversed_observed),
        "map insertion and module binding order must not change the selected \
         (key, marker) sequence ({})",
        describe(scene)
    );
    // The cross-object pair is present in the permuted leg too: insertion order
    // may not merge the two object identities or swap their markers.
    for selection in &reversed_observed {
        assert_cross_object_pair(scene, regions, selection)?;
    }
    Ok(())
}

fn check_negative(scene: &SelectionScene, regions: &[ExpectedRegion]) -> TestCaseResult {
    let witness = negative_witness(regions);
    let map = region_map_fixture(regions, InsertionOrder::Forward, Some(&witness));
    let failing_module = scene.modules[0].clone();

    let pure = run_pure_builder(scene, regions, &map, &scene.modules);
    let error = match pure {
        Ok(_) => {
            return Err(TestCaseError::fail(format!(
                "the pure builder accepted a missing exact key {:?} ({})",
                witness.removed,
                describe(scene)
            )))
        }
        Err(error) => error,
    };
    prop_assert_eq!(
        &error.error,
        &SelectionError::MissingModelConfig {
            key: witness.removed.clone()
        },
        "the pure builder must reject the exact removed RegionKey ({})",
        describe(scene)
    );
    prop_assert_eq!(
        error.stage_id.as_str(),
        NEUTRAL_STAGE,
        "the pure rejection must name the responsible stage ({})",
        describe(scene)
    );
    prop_assert_eq!(
        &error.module_id,
        &failing_module,
        "the pure rejection must name the first bound module whose layer 1 lookup failed ({})",
        describe(scene)
    );

    let (blackboard, runtime) = run_runtime_seam(scene, regions, map, &scene.modules);
    let runtime_error = match runtime {
        Ok(()) => {
            return Err(TestCaseError::fail(format!(
                "the runtime wrapper published an index with missing exact key {:?} ({})",
                witness.removed,
                describe(scene)
            )))
        }
        Err(error) => error,
    };
    match runtime_error {
        LayerStageError::FatalModule {
            stage_id,
            module_id,
            message,
        } => {
            prop_assert_eq!(
                stage_id.as_str(),
                NEUTRAL_STAGE,
                "the runtime rejection must name the failing stage ({})",
                describe(scene)
            );
            prop_assert_eq!(
                &module_id,
                &failing_module,
                "the runtime rejection must name the failing module ({})",
                describe(scene)
            );
            prop_assert!(
                message.contains(&format!("{:?}", witness.removed)),
                "the runtime FatalModule message must carry the exact missing key \
                 {:?}, got {message:?} ({})",
                witness.removed,
                describe(scene)
            );
        }
        other => {
            return Err(TestCaseError::fail(format!(
                "expected LayerStageError::FatalModule for missing authority, got {other:?} ({})",
                describe(scene)
            )))
        }
    }
    prop_assert!(
        blackboard.layer_selection_index().is_none(),
        "no partial index may be published after a rejected binding ({})",
        describe(scene)
    );
    Ok(())
}

/// Number of `negative` values the shrink audit must reach.
const SHRINK_AUDIT_MODES: usize = 2;

/// Fixed-seed AC-N2 shrink audit.
///
/// Walks `simplify()` on 128 fixed-seed trees and proves, per tree, that every
/// shrunk witness keeps its `negative` (missing-authority) mode and still
/// satisfies `check_scene` — the same checker `run_property` runs. The sampler
/// is `no_shrink`, so a lost mode means the generator stopped pinning the tag
/// rather than a legal simplification.
pub fn shrink_audit() {
    let limits =
        checked_selection_limits(3, 3, 2, 3).expect("H-RUNTIME-SELECTION shrink audit ceilings");
    let strategy = selection_strategy(&limits);
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
        let negative = initial.negative;
        seen.insert(negative);
        check_scene(&initial, &limits).unwrap_or_else(|error| {
            panic!(
                "H-RUNTIME-SELECTION: the unsimplified audit tree must satisfy the contract \
                 (negative={negative}): {error}"
            )
        });
        let mut steps = 0_usize;
        while steps < 64 && tree.simplify() {
            steps += 1;
            simplifications += 1;
            let current = tree.current();
            assert_eq!(
                current.negative, negative,
                "H-RUNTIME-SELECTION: shrinking changed the case mode"
            );
            check_scene(&current, &limits).unwrap_or_else(|error| {
                panic!(
                    "H-RUNTIME-SELECTION: the shrunk witness no longer satisfies the contract \
                     (negative={negative}): {error}"
                )
            });
        }
    }
    assert!(
        seen.len() >= SHRINK_AUDIT_MODES,
        "H-RUNTIME-SELECTION: shrink audit reached {} modes, expected at least {}",
        seen.len(),
        SHRINK_AUDIT_MODES
    );
    assert!(
        simplifications > 0,
        "H-RUNTIME-SELECTION: the shrink audit simplified nothing, so it certifies nothing"
    );
}

/// Run the registry-selected `H-RUNTIME-SELECTION` property.
pub fn run_property() {
    let run =
        PropertyRun::new(PROPERTY_ID, TEST_NAME).expect("H-RUNTIME-SELECTION registry context");
    let limits = checked_selection_limits(
        run.bound("max_layers").expect("max_layers bound"),
        run.bound("max_regions").expect("max_regions bound"),
        run.bound("max_modules").expect("max_modules bound"),
        run.bound("max_chain_len").expect("max_chain_len bound"),
    )
    .expect("H-RUNTIME-SELECTION safety ceilings");

    shrink_audit();
    let strategy = selection_strategy(&limits);
    run.check(&strategy, |scene, _guard| check_scene(&scene, &limits))
        .expect("H-RUNTIME-SELECTION exact-key authority contract");
}
