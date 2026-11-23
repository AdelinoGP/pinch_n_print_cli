//! Deterministic oracle controls for `H-RUNTIME-DISPATCH`.
//!
//! Every control builds a hand-written witness, calls the production seam it
//! claims to test (`slicer_wasm_host::selection::paint_allows_layer`, the pure
//! whole-print selection builder, and the runtime publication wrapper) and
//! asserts the exact boolean or selection state. The known-wrong models are
//! spelled out independently here, so a defect the charter property must catch
//! is proven catchable without sampling.
//!
//! The controls are intentionally separate from the generative body: this file
//! is mounted by `tests/property_runtime_contracts.rs`, while the property lives
//! in `tests/property_runtime.rs`.

#![allow(missing_docs)]

use std::collections::HashSet;
use std::sync::Arc;

use slicer_ir::{
    ConfigView, ExPolygon, GlobalLayer, MeshIR, PaintValue, RegionKey, RegionMapIR, RegionPlan,
    ResolvedConfig, SliceIR, SlicedRegion,
};
use slicer_runtime::{
    build_and_commit_layer_selection_index, Blackboard, CompiledModuleBuilder, CompiledStage,
    ExecutionPlan, LayerStageError,
};
use slicer_wasm_host::selection::{
    build_layer_selection_index, paint_allows_layer, LayerSelectionBuildError, LayerSelectionIndex,
    LayerSelectionIndexInput, LayerSelectionModuleInput, LayerTargetSource, STAGES,
};

/// Neutral module-backed layer stage used by every real-selection witness.
const NEUTRAL_STAGE: &str = "Layer::SlicePostProcess";
/// The single module bound in the real fixtures.
const CONTROL_MODULE: &str = "com.pbt.dispatch-control";
/// Object identity of every control region.
const CONTROL_OBJECT: &str = "dispatch-control-object";

/// Published paint-variant id stride, restated for the fixture's address rule.
const PAINT_VARIANT_STRIDE: u64 = 1_000_000;

/// One named AC-1 control: every known-wrong dispatch model is falsified by the
/// live predicate, and the real committed selection proves per-`module × layer`
/// gating rather than per-region filtering.
#[test]
fn runtime_semantic_dispatch_oracle_controls() {
    crate::dispatch::shrink_audit();
    require_all_model_disagrees();
    value_sensitive_model_disagrees();
    per_region_filtering_disagrees_with_committed_selection();
    absent_slice_skip_disagrees();
    transparent_and_witness_booleans();
    non_paint_only_manifest_is_transparent();
}

// ─────────────────────────────────────────────────────────────────────────
// Known-wrong independent models.
// ─────────────────────────────────────────────────────────────────────────

/// Wrong model: allow only when *every* declared semantic name is present.
fn require_all_model(
    declared: &HashSet<String>,
    regions: Option<&[Vec<(String, PaintValue)>]>,
) -> bool {
    if declared.is_empty() {
        return true;
    }
    let Some(regions) = regions else {
        return false;
    };
    let names: HashSet<&str> = regions
        .iter()
        .flat_map(|chain| chain.iter().map(|(name, _)| name.as_str()))
        .collect();
    declared.iter().all(|name| names.contains(name.as_str()))
}

/// Wrong model: allow only when a declared name appears with a declared value.
fn value_sensitive_model(
    declared: &HashSet<String>,
    declared_values: &HashSet<PaintValue>,
    regions: Option<&[Vec<(String, PaintValue)>]>,
) -> bool {
    if declared.is_empty() {
        return true;
    }
    let Some(regions) = regions else {
        return false;
    };
    regions.iter().any(|chain| {
        chain
            .iter()
            .any(|(name, value)| declared.contains(name) && declared_values.contains(value))
    })
}

/// Wrong model: a nonempty declaration always skips an absent slice.
fn absent_slice_skip_model(declared: &HashSet<String>) -> bool {
    declared.is_empty()
}

// ─────────────────────────────────────────────────────────────────────────
// Fixtures.
// ─────────────────────────────────────────────────────────────────────────

fn slice_of(specs: Vec<(Vec<(&str, PaintValue)>, bool)>) -> SliceIR {
    SliceIR {
        global_layer_index: 0,
        regions: specs
            .into_iter()
            .enumerate()
            .map(|(index, (chain, with_geometry))| SlicedRegion {
                object_id: CONTROL_OBJECT.to_string(),
                region_id: 1_000 + index as u64,
                polygons: if with_geometry {
                    vec![ExPolygon::default()]
                } else {
                    Vec::new()
                },
                variant_chain: chain
                    .into_iter()
                    .map(|(name, value)| (name.to_string(), value))
                    .collect(),
                ..SlicedRegion::default()
            })
            .collect(),
        ..SliceIR::default()
    }
}

/// Documented map-space address: modifier ids and empty chains verbatim, every
/// other id decoded once by the paint stride.
fn map_space_id(source_id: u64, chain: &[(String, PaintValue)]) -> u64 {
    let modifier_namespace = source_id != u64::MAX && source_id & (1 << 63) != 0;
    if modifier_namespace || chain.is_empty() {
        source_id
    } else {
        source_id / PAINT_VARIANT_STRIDE
    }
}

fn map_for(slices: &[SliceIR]) -> RegionMapIR {
    let mut map = RegionMapIR::default();
    let config = map.intern_config(ResolvedConfig::default());
    for slice in slices {
        for region in &slice.regions {
            map.entries.insert(
                RegionKey {
                    global_layer_index: slice.global_layer_index,
                    object_id: region.object_id.clone(),
                    region_id: map_space_id(region.region_id, &region.variant_chain),
                    variant_chain: region.variant_chain.clone(),
                },
                RegionPlan {
                    config,
                    ..RegionPlan::default()
                },
            );
        }
    }
    map
}

fn layer() -> GlobalLayer {
    GlobalLayer {
        index: 0,
        ..GlobalLayer::default()
    }
}

fn plan_for(semantics: &HashSet<String>) -> ExecutionPlan {
    let module = CompiledModuleBuilder::new(CONTROL_MODULE.to_string())
        .claims(Vec::new())
        .region_split_semantics(semantics.clone())
        .config_view(Arc::new(ConfigView::new()))
        .build();
    ExecutionPlan {
        per_layer_stages: vec![CompiledStage {
            stage_id: NEUTRAL_STAGE.to_string(),
            modules: vec![module],
        }],
        global_layers: Arc::new(vec![layer()]),
        ..ExecutionPlan::default()
    }
}

fn run_pure(
    slices: &[SliceIR],
    map: &RegionMapIR,
    semantics: Option<&HashSet<String>>,
) -> Result<LayerSelectionIndex, LayerSelectionBuildError> {
    let layers = [layer()];
    let claims: Vec<String> = Vec::new();
    let invocation = ConfigView::new();
    // exhaustive: borrowed static binding authority has no Default; every field is consumed by the selection builder, so omitting one would fake the fixture.
    let modules = [LayerSelectionModuleInput {
        stage_id: NEUTRAL_STAGE,
        module_id: CONTROL_MODULE,
        claims: &claims,
        paint_semantics: semantics,
        invocation_config: &invocation,
    }];
    // exhaustive: the whole-print snapshot supplies every authority channel explicitly, including the absent support plan and the empty perimeter roster.
    build_layer_selection_index(LayerSelectionIndexInput {
        stage_roster: STAGES,
        global_layers: &layers,
        modules: &modules,
        slices,
        region_map: Some(map),
        support_plan: None,
        perimeters: &[],
    })
}

fn run_runtime(
    slice: Option<SliceIR>,
    map: RegionMapIR,
    semantics: &HashSet<String>,
) -> (Blackboard, Result<(), LayerStageError>) {
    let mut blackboard = Blackboard::new(Arc::new(MeshIR::default()), 1);
    if let Some(slice) = slice {
        blackboard
            .commit_slice_ir(Arc::new(vec![slice]))
            .expect("control slice must be accepted");
    }
    blackboard
        .commit_region_map(Arc::new(map))
        .expect("control region map must be accepted");
    let plan = plan_for(semantics);
    let result = build_and_commit_layer_selection_index(&plan, &mut blackboard);
    (blackboard, result)
}

fn committed_selection(blackboard: &Blackboard) -> &slicer_wasm_host::selection::LayerSelection {
    blackboard
        .layer_selection_index()
        .expect("the control fixture publishes a selection index")
        .selection_for(0, CONTROL_MODULE)
        .expect("the control module is bound on layer 0")
}

fn model_target_keys(selection: &slicer_wasm_host::selection::LayerSelection) -> Vec<RegionKey> {
    selection
        .targets
        .iter()
        .map(|target| match &target.source {
            LayerTargetSource::Model { key, .. } => key.clone(),
            other => panic!("neutral stage yielded non-model source {other:?}"),
        })
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────
// Controls.
// ─────────────────────────────────────────────────────────────────────────

fn require_all_model_disagrees() {
    // Only "alpha" is present even though both names are declared.
    let declared: HashSet<String> = ["alpha", "beta"].iter().map(|s| (*s).to_string()).collect();
    let slice = slice_of(vec![(vec![("alpha", PaintValue::Flag(true))], true)]);
    let chains = vec![vec![("alpha".to_string(), PaintValue::Flag(true))]];

    let live = paint_allows_layer(&declared, Some(&slice));
    let wrong = require_all_model(&declared, Some(chains.as_slice()));
    assert!(
        live,
        "the live predicate allows one matching declared name among two declarations"
    );
    assert!(
        !wrong,
        "the require-all model must reject a witness with one of two names present"
    );
    assert_ne!(
        live, wrong,
        "the require-all model must disagree with paint_allows_layer on this witness"
    );
}

fn value_sensitive_model_disagrees() {
    // The declared *name* matches, but the region's PaintValue does not.
    let declared: HashSet<String> = ["material"].iter().map(|s| (*s).to_string()).collect();
    let slice = slice_of(vec![(vec![("material", PaintValue::ToolIndex(1))], true)]);
    let chains = vec![vec![("material".to_string(), PaintValue::ToolIndex(1))]];
    let declared_values: HashSet<PaintValue> = [PaintValue::ToolIndex(2)].into_iter().collect();

    let live = paint_allows_layer(&declared, Some(&slice));
    let wrong = value_sensitive_model(&declared, &declared_values, Some(chains.as_slice()));
    assert!(
        live,
        "paint gating is semantic-name based, so the value mismatch still allows the layer"
    );
    assert!(
        !wrong,
        "a value-sensitive model must reject a name match with a different PaintValue"
    );
    assert_ne!(
        live, wrong,
        "the value-sensitive model must disagree with paint_allows_layer"
    );
}

fn per_region_filtering_disagrees_with_committed_selection() {
    // One region matches the declaration; two carry unrelated semantics.
    let slice = slice_of(vec![
        (vec![("material", PaintValue::ToolIndex(1))], true),
        (vec![("unrelated-a", PaintValue::Flag(true))], true),
        (vec![("unrelated-b", PaintValue::Flag(false))], true),
    ]);
    let declared: HashSet<String> = ["material"].iter().map(|s| (*s).to_string()).collect();
    assert!(
        paint_allows_layer(&declared, Some(&slice)),
        "the witness must be a matching scene"
    );
    let map = map_for(std::slice::from_ref(&slice));
    let (blackboard, result) = run_runtime(Some(slice.clone()), map, &declared);
    result.expect("the matching witness must build a committed selection");
    let selection = committed_selection(&blackboard);

    assert!(
        selection.invoke,
        "the matching witness invokes on the layer"
    );
    // The wrong expectation: only the matching region is a target.
    let per_region_filtered = 1usize;
    assert_eq!(
        selection.targets.len(),
        3,
        "gating is per module × layer, so all three candidates are selected"
    );
    assert_ne!(
        selection.targets.len(),
        per_region_filtered,
        "a per-region-filtering expectation must disagree with the committed selection"
    );
    let expected: Vec<RegionKey> = slice
        .regions
        .iter()
        .map(|region| RegionKey {
            global_layer_index: 0,
            object_id: region.object_id.clone(),
            region_id: region.region_id,
            variant_chain: region.variant_chain.clone(),
        })
        .collect();
    assert_eq!(
        model_target_keys(selection),
        expected,
        "every candidate region is a selected model target in committed order"
    );
}

fn absent_slice_skip_disagrees() {
    let declared: HashSet<String> = ["material"].iter().map(|s| (*s).to_string()).collect();
    // The absent slice carries no regions; the map is present but unused.
    let map = map_for(&[]);
    let (blackboard, result) = run_runtime(None, map, &declared);
    result.expect("an absent slice is not a selection failure");

    let live_predicate = paint_allows_layer(&declared, None);
    let live_selection = committed_selection(&blackboard);
    let wrong = absent_slice_skip_model(&declared);
    assert!(
        live_predicate,
        "an absent slice is conservatively allowed for a nonempty declaration"
    );
    assert!(
        live_selection.invoke,
        "the absent-slice selection must still invoke"
    );
    assert!(
        live_selection.targets.is_empty(),
        "an absent slice contributes no model targets"
    );
    assert!(
        !wrong,
        "the skip model claims a nonempty declaration suppresses an absent slice"
    );
    assert_ne!(
        live_selection.invoke, wrong,
        "the absent-slice-skip expectation must disagree with the committed selection"
    );
}

fn transparent_and_witness_booleans() {
    let empty: HashSet<String> = HashSet::new();
    let nonempty: HashSet<String> = ["material"].iter().map(|s| (*s).to_string()).collect();

    // Transparent declared set: allowed for a present slice with any regions.
    let matching = slice_of(vec![(vec![("material", PaintValue::ToolIndex(1))], true)]);
    assert!(
        paint_allows_layer(&empty, Some(&matching)),
        "an empty declared set is transparent for a present slice"
    );
    assert!(
        paint_allows_layer(&empty, None),
        "an empty declared set is transparent for an absent slice"
    );

    // Transparent present-empty slice keeps invoke = true and selects nothing.
    let present_empty = slice_of(Vec::new());
    let (blackboard, result) = run_runtime(Some(present_empty.clone()), map_for(&[]), &empty);
    result.expect("a transparent present-empty slice must build a selection");
    let selection = committed_selection(&blackboard);
    assert!(
        selection.invoke,
        "a transparent present-empty slice still invokes"
    );
    assert!(
        selection.targets.is_empty(),
        "a present-empty slice contributes no targets"
    );

    // Present-empty slice with a nonempty declaration: skip.
    assert!(
        !paint_allows_layer(&nonempty, Some(&present_empty)),
        "a present-empty slice must not satisfy a nonempty declaration"
    );
    let (blackboard, result) = run_runtime(Some(present_empty), map_for(&[]), &nonempty);
    result.expect("the gated present-empty witness must build a selection");
    let selection = committed_selection(&blackboard);
    assert!(!selection.invoke, "the gated selection must not invoke");
    assert!(
        selection.targets.is_empty(),
        "a gated layer selects no model targets"
    );

    // No matching name at all.
    let unrelated = slice_of(vec![(vec![("unrelated", PaintValue::ToolIndex(2))], true)]);
    assert!(
        !paint_allows_layer(&nonempty, Some(&unrelated)),
        "a nonempty declaration with no matching name must be skipped"
    );

    // One matching name amid unrelated entries.
    let mixed = slice_of(vec![
        (vec![("unrelated", PaintValue::Flag(true))], true),
        (
            vec![
                ("density", PaintValue::Scalar(0.5)),
                ("material", PaintValue::ToolIndex(3)),
            ],
            true,
        ),
    ]);
    assert!(
        paint_allows_layer(&nonempty, Some(&mixed)),
        "one matching name anywhere on the slice allows the layer"
    );

    // Pure-predicate scalar witness only. Scalar paint values are outside the
    // declared valid region-split domain, so no committed selection case in this
    // charter may carry one; `paint_allows_layer` reads only names, so a scalar
    // value on a declared name must still allow.
    let scalar_valued = slice_of(vec![(vec![("material", PaintValue::Scalar(0.25))], true)]);
    assert!(
        paint_allows_layer(&nonempty, Some(&scalar_valued)),
        "the live predicate is value-blind even for a scalar-valued declared name \
         (pure predicate only; not a committed selection case)"
    );

    // Matching name only on an empty-geometry region.
    let empty_geometry = slice_of(vec![(vec![("material", PaintValue::Flag(true))], false)]);
    assert!(
        empty_geometry.regions[0].polygons.is_empty(),
        "the witness must carry an empty-geometry region"
    );
    assert!(
        paint_allows_layer(&nonempty, Some(&empty_geometry)),
        "polygon emptiness does not change the semantic-set verdict"
    );

    // Case mismatch is a mismatch: names are case-sensitive.
    let case_only = slice_of(vec![(vec![("Material", PaintValue::Flag(true))], true)]);
    assert!(
        !paint_allows_layer(&nonempty, Some(&case_only)),
        "a case-only difference is not a matching semantic"
    );
}

fn non_paint_only_manifest_is_transparent() {
    // A manifest without `paint_only` has no declared set at all; the pure
    // builder must select every candidate on a semantically nonmatching slice.
    let declared: HashSet<String> = ["material"].iter().map(|s| (*s).to_string()).collect();
    let slice = slice_of(vec![
        (vec![("alpha", PaintValue::Flag(true))], true),
        (vec![("beta", PaintValue::Flag(false))], true),
    ]);
    assert!(
        !paint_allows_layer(&declared, Some(&slice)),
        "the fixture slice must not satisfy the declaration, or the None branch proves nothing"
    );
    let map = map_for(std::slice::from_ref(&slice));
    let index = run_pure(std::slice::from_ref(&slice), &map, None)
        .expect("a manifest without paint semantics must not be gated");
    let selection = index
        .selection_for(0, CONTROL_MODULE)
        .expect("the control module is bound on layer 0");
    assert!(
        selection.invoke,
        "a non-paint-only manifest invokes on a nonmatching layer"
    );
    assert_eq!(
        model_target_keys(selection).len(),
        2,
        "a non-paint-only manifest selects every candidate region"
    );
}
