//! Deterministic authority and publication controls for `H-RUNTIME-SELECTION`.
//!
//! Each control drives the production seams directly:
//!
//! * `slicer_wasm_host::selection::build_layer_selection_index` for the pure
//!   exact-key authority contract (including raw unknown stage rejection), and
//! * `slicer_runtime::build_and_commit_layer_selection_index` for the runtime
//!   `FatalModule` mapping, no-partial publication and write-once publication.
//!
//! The expected identities are derived from the hand-built fixture and the
//! documented address rule, never from `region_map_key_for_sliced_region`.

#![allow(missing_docs)]

use std::collections::HashSet;
use std::sync::Arc;

use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, GlobalLayer, MeshIR, PaintValue, RegionKey, RegionMapIR,
    RegionPlan, ResolvedConfig, SliceIR, SlicedRegion,
};
use slicer_runtime::{
    build_and_commit_layer_selection_index, Blackboard, CompiledModuleBuilder, CompiledStage,
    ExecutionPlan, LayerStageError,
};
use slicer_wasm_host::selection::{
    build_layer_selection_index, LayerSelectionBuildError, LayerSelectionIndex,
    LayerSelectionIndexInput, LayerSelectionModuleInput, LayerTargetSource, SelectionError, STAGES,
};

/// Neutral module-backed layer stage used by every witness.
const NEUTRAL_STAGE: &str = "Layer::SlicePostProcess";
/// The single bound module.
const CONTROL_MODULE: &str = "com.pbt.selection-control";
/// Object identity shared by the chain-variant regions.
const CONTROL_OBJECT: &str = "selection-control-object";
/// Second object identity used by the cross-object authority controls.
const CONTROL_OBJECT_B: &str = "selection-control-object-b";
/// Numeric source region id shared by the cross-object pair.
const CROSS_OBJECT_ID: u64 = 41;
/// Published paint-variant id stride, restated for the address derivation.
const PAINT_VARIANT_STRIDE: u64 = 1_000_000;
/// The shared SOURCE region id of the two ordered-chain variants.
const SHARED_PAINTED_ID: u64 = 9 * PAINT_VARIANT_STRIDE + 3;
/// Stage id of the runtime write-once publication failure.
const SELECTION_STAGE: &str = "Layer::Selection";
/// Module id of the runtime write-once publication failure.
const SELECTION_MODULE: &str = "host";

/// One named AC-2 control: exact-key authority, ordered-chain distinction and
/// atomic write-once publication.
#[test]
fn runtime_selection_exact_key_and_publication_rejections() {
    crate::selection::shrink_audit();
    missing_exact_key_rejects_with_decoys_present();
    reversed_only_authority_rejects();
    sibling_only_authority_rejects();
    sibling_removal_succeeds_with_exact_markers();
    cross_object_exact_authority_succeeds();
    cross_object_missing_key_rejects();
    cross_object_authority_is_not_shared();
    raw_unknown_stage_rejects_at_the_pure_builder();
    duplicate_publication_preserves_the_committed_index();
}

// ─────────────────────────────────────────────────────────────────────────
// Fixture.
// ─────────────────────────────────────────────────────────────────────────

/// Hand-built committed scene plus its authored exact-key authority.
struct Scene {
    layers: Vec<GlobalLayer>,
    slices: Vec<SliceIR>,
    map: RegionMapIR,
}

impl Scene {
    fn new(layer_count: u32) -> Self {
        let layers = (0..layer_count)
            .map(|index| GlobalLayer {
                index,
                ..GlobalLayer::default()
            })
            .collect();
        let slices = (0..layer_count)
            .map(|layer| SliceIR {
                global_layer_index: layer,
                ..SliceIR::default()
            })
            .collect();
        let mut map = RegionMapIR::default();
        // Seed the interner so `ConfigId::default()` stays a valid index.
        map.intern_config(ResolvedConfig::default());
        Scene {
            layers,
            slices,
            map,
        }
    }

    /// Commit one region on `layer` and record its exact config authority.
    fn add_region(&mut self, layer: u32, region: SlicedRegion, marker: f64) -> RegionKey {
        let key = key_of(layer, &region);
        self.slices[layer as usize].regions.push(region);
        self.add_authority(key.clone(), marker);
        key
    }

    /// Record one exact config-map entry without committing a slice region.
    fn add_authority(&mut self, key: RegionKey, marker: f64) {
        let config = self.map.intern_config(ResolvedConfig {
            line_width: marker,
            ..ResolvedConfig::default()
        });
        self.map.entries.insert(
            key,
            RegionPlan {
                config,
                ..RegionPlan::default()
            },
        );
    }

    fn remove_authority(&mut self, key: &RegionKey) {
        self.map.entries.remove(key);
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

/// Exact config-map address for one committed region.
fn key_of(layer: u32, region: &SlicedRegion) -> RegionKey {
    RegionKey {
        global_layer_index: layer,
        object_id: region.object_id.clone(),
        region_id: map_space_id(region.region_id, &region.variant_chain),
        variant_chain: region.variant_chain.clone(),
    }
}

/// SOURCE-space identity production must report for one committed region.
fn source_key_of(layer: u32, region: &SlicedRegion) -> RegionKey {
    RegionKey {
        global_layer_index: layer,
        object_id: region.object_id.clone(),
        region_id: region.region_id,
        variant_chain: region.variant_chain.clone(),
    }
}

fn ordinary_region(region_id: u64) -> SlicedRegion {
    SlicedRegion {
        object_id: CONTROL_OBJECT.to_string(),
        region_id,
        polygons: vec![ExPolygon::default()],
        ..SlicedRegion::default()
    }
}

fn painted_region(chain: &[(&str, PaintValue)]) -> SlicedRegion {
    SlicedRegion {
        object_id: CONTROL_OBJECT.to_string(),
        region_id: SHARED_PAINTED_ID,
        polygons: vec![ExPolygon::default()],
        variant_chain: chain
            .iter()
            .map(|(name, value)| ((*name).to_string(), value.clone()))
            .collect(),
        ..SlicedRegion::default()
    }
}

fn chain_a() -> Vec<(&'static str, PaintValue)> {
    vec![
        ("ordered-a", PaintValue::ToolIndex(1)),
        ("ordered-b", PaintValue::ToolIndex(2)),
    ]
}

fn reversed_chain_a() -> Vec<(&'static str, PaintValue)> {
    let mut chain = chain_a();
    chain.reverse();
    chain
}

/// Layer 0 is always a valid ordinary region; layer 1 carries an ordinary
/// region plus the supplied painted chain variants (markers 0.75, 1.0, ...).
fn scene_with(layer_one_chains: &[Vec<(&'static str, PaintValue)>]) -> Scene {
    let mut scene = Scene::new(2);
    scene.add_region(0, ordinary_region(1), 0.25);
    scene.add_region(1, ordinary_region(2), 0.30);
    for (index, chain) in layer_one_chains.iter().enumerate() {
        // Dyadic markers are exactly representable, so marker comparisons are
        // exact rather than epsilon-based.
        scene.add_region(1, painted_region(chain), 0.75 + 0.25 * index as f64);
    }
    scene
}

/// One region of the cross-object pair: same numeric id and empty chain as its
/// object-A sibling, only the object differs.
fn cross_object_region(object_id: &str) -> SlicedRegion {
    SlicedRegion {
        object_id: object_id.to_string(),
        region_id: CROSS_OBJECT_ID,
        polygons: vec![ExPolygon::default()],
        ..SlicedRegion::default()
    }
}

/// One layer carrying the cross-object pair (A first, B second, distinct
/// dyadic markers).
fn cross_object_scene() -> Scene {
    let mut scene = Scene::new(1);
    scene.add_region(0, cross_object_region(CONTROL_OBJECT), 0.125);
    scene.add_region(0, cross_object_region(CONTROL_OBJECT_B), 0.25);
    scene
}

fn invocation_view() -> ConfigView {
    ConfigView::from_map(std::collections::HashMap::from([(
        "line_width".to_string(),
        ConfigValue::Float(0.4),
    )]))
}

fn plan(scene: &Scene) -> ExecutionPlan {
    let semantics: HashSet<String> = HashSet::new();
    let module = CompiledModuleBuilder::new(CONTROL_MODULE.to_string())
        .claims(Vec::new())
        .region_split_semantics(semantics)
        .config_view(Arc::new(invocation_view()))
        .build();
    ExecutionPlan {
        per_layer_stages: vec![CompiledStage {
            stage_id: NEUTRAL_STAGE.to_string(),
            modules: vec![module],
        }],
        global_layers: Arc::new(scene.layers.clone()),
        ..ExecutionPlan::default()
    }
}

fn run_pure_stage(
    scene: &Scene,
    stage_id: &str,
) -> Result<LayerSelectionIndex, LayerSelectionBuildError> {
    let claims: Vec<String> = Vec::new();
    let semantics: HashSet<String> = HashSet::new();
    let invocation = invocation_view();
    // exhaustive: borrowed static binding authority has no Default; every field is consumed by the selection builder, so omitting one would fake the fixture.
    let modules = [LayerSelectionModuleInput {
        stage_id,
        module_id: CONTROL_MODULE,
        claims: &claims,
        paint_semantics: Some(&semantics),
        invocation_config: &invocation,
    }];
    // exhaustive: the whole-print snapshot supplies every authority channel explicitly, including the absent support plan and the empty perimeter roster.
    build_layer_selection_index(LayerSelectionIndexInput {
        stage_roster: STAGES,
        global_layers: &scene.layers,
        modules: &modules,
        slices: &scene.slices,
        region_map: Some(&scene.map),
        support_plan: None,
        perimeters: &[],
    })
}

fn run_pure(scene: &Scene) -> Result<LayerSelectionIndex, LayerSelectionBuildError> {
    run_pure_stage(scene, NEUTRAL_STAGE)
}

fn prepare_blackboard(scene: &Scene) -> (Blackboard, ExecutionPlan) {
    let mut blackboard = Blackboard::new(Arc::new(MeshIR::default()), scene.layers.len());
    blackboard
        .commit_slice_ir(Arc::new(scene.slices.clone()))
        .expect("control slices must be accepted");
    blackboard
        .commit_region_map(Arc::new(scene.map.clone()))
        .expect("control region map must be accepted");
    (blackboard, plan(scene))
}

fn run_runtime(scene: &Scene) -> (Blackboard, Result<(), LayerStageError>) {
    let (mut blackboard, plan) = prepare_blackboard(scene);
    let result = build_and_commit_layer_selection_index(&plan, &mut blackboard);
    (blackboard, result)
}

fn model_targets(index: &LayerSelectionIndex, layer: u32) -> Vec<RegionKey> {
    index
        .selection_for(layer, CONTROL_MODULE)
        .expect("the control module is bound on every committed layer")
        .targets
        .iter()
        .map(|target| match &target.source {
            LayerTargetSource::Model { key, .. } => key.clone(),
            other => panic!("neutral stage yielded non-model source {other:?}"),
        })
        .collect()
}

fn target_marker(index: &LayerSelectionIndex, layer: u32, key: &RegionKey) -> Option<f64> {
    index
        .selection_for(layer, CONTROL_MODULE)
        .expect("the control module is bound on every committed layer")
        .targets
        .iter()
        .find_map(|target| match &target.source {
            LayerTargetSource::Model { key: actual, .. } if actual == key => Some(
                target
                    .config
                    .get_float("line_width")
                    .expect("declared marker"),
            ),
            _ => None,
        })
}

// ─────────────────────────────────────────────────────────────────────────
// Controls.
// ─────────────────────────────────────────────────────────────────────────

fn missing_exact_key_rejects_with_decoys_present() {
    let mut scene = scene_with(&[chain_a(), reversed_chain_a()]);
    let exact = key_of(1, &painted_region(&chain_a()));
    let reversed = key_of(1, &painted_region(&reversed_chain_a()));
    assert_ne!(
        exact, reversed,
        "the reversed ordered chain must be a distinct full identity"
    );
    scene.remove_authority(&exact);
    // Surviving decoys: a default (empty-chain) entry for the same
    // object/region, a different-chain sibling, and the reversed chain.
    scene.add_authority(
        RegionKey {
            variant_chain: Vec::new(),
            ..exact.clone()
        },
        3.0,
    );
    scene.add_authority(
        RegionKey {
            variant_chain: vec![(String::from("sibling"), PaintValue::Flag(true))],
            ..exact.clone()
        },
        4.0,
    );

    let error = run_pure(&scene).expect_err("a missing exact key must be rejected");
    assert_eq!(
        error.error,
        SelectionError::MissingModelConfig { key: exact.clone() },
        "the pure builder must name the exact removed RegionKey, not a decoy"
    );
    assert_eq!(error.stage_id.as_str(), NEUTRAL_STAGE);
    assert_eq!(error.module_id, CONTROL_MODULE);

    // Prior valid work exists because layer 0's exact authority is present; a
    // partial publication would be observable.
    let (blackboard, result) = run_runtime(&scene);
    let runtime_error = result.expect_err("the runtime wrapper must reject missing authority");
    match runtime_error {
        LayerStageError::FatalModule {
            stage_id,
            module_id,
            message,
        } => {
            assert_eq!(stage_id.as_str(), NEUTRAL_STAGE);
            assert_eq!(module_id, CONTROL_MODULE);
            assert!(
                message.contains(&format!("{exact:?}")),
                "the FatalModule message must carry the exact missing key, got {message:?}"
            );
        }
        other => panic!("expected LayerStageError::FatalModule, got {other:?}"),
    }
    assert!(
        blackboard.layer_selection_index().is_none(),
        "a rejected binding must publish no partial index after valid layer-0 work"
    );
}

fn reversed_only_authority_rejects() {
    let mut scene = scene_with(&[chain_a()]);
    let exact = key_of(1, &painted_region(&chain_a()));
    let reversed = key_of(1, &painted_region(&reversed_chain_a()));
    scene.remove_authority(&exact);
    scene.add_authority(reversed, 5.0);

    let error =
        run_pure(&scene).expect_err("reversed-chain authority must not satisfy the exact key");
    assert_eq!(
        error.error,
        SelectionError::MissingModelConfig { key: exact.clone() },
        "only the exact ordered chain is authoritative"
    );

    let (blackboard, result) = run_runtime(&scene);
    let runtime_error =
        result.expect_err("the runtime wrapper must reject reversed-only authority");
    assert!(
        matches!(
            &runtime_error,
            LayerStageError::FatalModule { stage_id, module_id, message }
                if stage_id == NEUTRAL_STAGE
                    && module_id == CONTROL_MODULE
                    && message.contains(&format!("{exact:?}"))
        ),
        "runtime rejection must name the stage, module and exact key, got {runtime_error:?}"
    );
    assert!(blackboard.layer_selection_index().is_none());
}

fn sibling_only_authority_rejects() {
    let mut scene = scene_with(&[chain_a()]);
    let exact = key_of(1, &painted_region(&chain_a()));
    scene.remove_authority(&exact);
    scene.add_authority(
        RegionKey {
            variant_chain: vec![(String::from("sibling"), PaintValue::Flag(true))],
            ..exact.clone()
        },
        6.0,
    );

    let error =
        run_pure(&scene).expect_err("sibling-chain authority must not satisfy the exact key");
    assert_eq!(
        error.error,
        SelectionError::MissingModelConfig { key: exact.clone() },
        "a sibling chain is not the exact ordered identity"
    );

    let (blackboard, result) = run_runtime(&scene);
    let runtime_error = result.expect_err("the runtime wrapper must reject sibling-only authority");
    assert!(
        matches!(
            &runtime_error,
            LayerStageError::FatalModule { stage_id, module_id, message }
                if stage_id == NEUTRAL_STAGE
                    && module_id == CONTROL_MODULE
                    && message.contains(&format!("{exact:?}"))
        ),
        "runtime rejection must name the stage, module and exact key, got {runtime_error:?}"
    );
    assert!(blackboard.layer_selection_index().is_none());
}

fn sibling_removal_succeeds_with_exact_markers() {
    // Both ordered-chain variants of one object/region keep their exact
    // authority; a default and a sibling entry are present but must never be
    // consulted.
    let mut scene = scene_with(&[chain_a(), reversed_chain_a()]);
    let exact_a = key_of(1, &painted_region(&chain_a()));
    let exact_b = key_of(1, &painted_region(&reversed_chain_a()));
    let source_a = source_key_of(1, &painted_region(&chain_a()));
    let source_b = source_key_of(1, &painted_region(&reversed_chain_a()));
    scene.add_authority(
        RegionKey {
            variant_chain: Vec::new(),
            ..exact_a.clone()
        },
        3.0,
    );
    scene.add_authority(
        RegionKey {
            variant_chain: vec![(String::from("sibling"), PaintValue::Flag(true))],
            ..exact_a.clone()
        },
        4.0,
    );

    let index = run_pure(&scene).expect("exact-key authority must build without decoys winning");
    let keys = model_targets(&index, 1);
    assert_eq!(
        keys,
        vec![
            source_key_of(1, &ordinary_region(2)),
            source_a.clone(),
            source_b.clone(),
        ],
        "every committed region is selected in committed order with its source-space key"
    );
    assert_ne!(
        exact_a, exact_b,
        "the ordered chains are distinct config-authority identities"
    );
    assert_ne!(
        source_a, source_b,
        "the reversed ordered chain is a distinct source identity"
    );
    assert_eq!(
        target_marker(&index, 1, &source_a),
        Some(0.75),
        "the first chain variant keeps its own exact marker, never the default/sibling value"
    );
    assert_eq!(
        target_marker(&index, 1, &source_b),
        Some(1.0),
        "the reversed chain variant keeps its own exact marker"
    );

    let (blackboard, result) = run_runtime(&scene);
    result.expect("the runtime wrapper must accept exact-key authority");
    let published = blackboard
        .layer_selection_index()
        .expect("successful publication must present an index");
    assert_eq!(
        model_targets(published, 1),
        keys,
        "the runtime publication must select the same exact identities"
    );
    assert_eq!(
        target_marker(published, 1, &source_a),
        Some(0.75),
        "the runtime target config must carry its own exact marker"
    );
}

/// Both exact keys present: the pair is selected as two distinct targets with
/// their own markers, even though `(region_id, chain)` is identical.
fn cross_object_exact_authority_succeeds() {
    let scene = cross_object_scene();
    let key_a = key_of(0, &cross_object_region(CONTROL_OBJECT));
    let key_b = key_of(0, &cross_object_region(CONTROL_OBJECT_B));
    assert_eq!(
        key_a.region_id, key_b.region_id,
        "the control pair must share the numeric region id"
    );
    assert_eq!(
        key_a.variant_chain, key_b.variant_chain,
        "the control pair must share the exact ordered chain"
    );
    assert_ne!(
        key_a.object_id, key_b.object_id,
        "the control pair must differ only by object identity"
    );

    let source_a = source_key_of(0, &cross_object_region(CONTROL_OBJECT));
    let source_b = source_key_of(0, &cross_object_region(CONTROL_OBJECT_B));
    let index = run_pure(&scene).expect("both exact cross-object keys are present");
    assert_eq!(
        model_targets(&index, 0),
        vec![source_a.clone(), source_b.clone()],
        "an object-bearing exact key must select both regions in committed order"
    );
    assert_eq!(
        target_marker(&index, 0, &source_a),
        Some(0.125),
        "object A's target must carry its own marker"
    );
    assert_eq!(
        target_marker(&index, 0, &source_b),
        Some(0.25),
        "object B's target must carry its own marker, never object A's"
    );

    let (blackboard, result) = run_runtime(&scene);
    result.expect("the runtime wrapper must accept both exact cross-object keys");
    let published = blackboard
        .layer_selection_index()
        .expect("a successful publication must present an index");
    assert_eq!(
        model_targets(published, 0),
        vec![source_a, source_b],
        "the runtime publication must keep both cross-object identities distinct"
    );
}

/// Object B's exact key removed, object A's identical `(region_id, chain)` key
/// kept: the pure builder and the runtime wrapper must both reject, naming B.
fn cross_object_missing_key_rejects() {
    let mut scene = cross_object_scene();
    let key_a = key_of(0, &cross_object_region(CONTROL_OBJECT));
    let key_b = key_of(0, &cross_object_region(CONTROL_OBJECT_B));
    scene.remove_authority(&key_b);
    assert!(
        scene.map.entries.contains_key(&key_a),
        "object A's exact key must survive as the decoy authority"
    );
    assert!(
        !scene.map.entries.contains_key(&key_b),
        "object B's exact key is the one under test"
    );

    let error = run_pure(&scene).expect_err("a missing exact object-B key must be rejected");
    match &error.error {
        SelectionError::MissingModelConfig { key } => assert_eq!(
            key.object_id, CONTROL_OBJECT_B,
            "the rejection must name object B, never the surviving object A authority"
        ),
        other => panic!("expected MissingModelConfig, got {other:?}"),
    }
    assert_eq!(
        error.error,
        SelectionError::MissingModelConfig { key: key_b.clone() },
        "the pure builder must report B's exact key, not A's same-numeric-id key"
    );
    assert_eq!(error.stage_id.as_str(), NEUTRAL_STAGE);
    assert_eq!(error.module_id, CONTROL_MODULE);

    let (blackboard, result) = run_runtime(&scene);
    let runtime_error =
        result.expect_err("the runtime wrapper must reject missing object-B authority");
    match runtime_error {
        LayerStageError::FatalModule {
            stage_id,
            module_id,
            message,
        } => {
            assert_eq!(stage_id.as_str(), NEUTRAL_STAGE);
            assert_eq!(module_id, CONTROL_MODULE);
            assert!(
                message.contains(&format!("{key_b:?}")),
                "the FatalModule message must carry the exact object-B key, got {message:?}"
            );
        }
        other => panic!("expected LayerStageError::FatalModule, got {other:?}"),
    }
    assert!(
        blackboard.layer_selection_index().is_none(),
        "a rejected cross-object binding must publish no partial index"
    );
}

/// Object A's exact key removed while object B's survives: B's same numeric id
/// and chain must not satisfy A's lookup.
fn cross_object_authority_is_not_shared() {
    let mut scene = cross_object_scene();
    let key_a = key_of(0, &cross_object_region(CONTROL_OBJECT));
    let key_b = key_of(0, &cross_object_region(CONTROL_OBJECT_B));
    scene.remove_authority(&key_a);
    assert!(
        scene.map.entries.contains_key(&key_b),
        "object B's exact key must survive as the cross-object decoy"
    );

    let error =
        run_pure(&scene).expect_err("object B's exact key must not satisfy object A's lookup");
    assert_eq!(
        error.error,
        SelectionError::MissingModelConfig { key: key_a.clone() },
        "the rejection must name object A even though B keeps the same numeric id and chain"
    );

    let (blackboard, result) = run_runtime(&scene);
    let runtime_error =
        result.expect_err("the runtime wrapper must reject missing object-A authority");
    assert!(
        matches!(
            &runtime_error,
            LayerStageError::FatalModule { stage_id, module_id, message }
                if stage_id == NEUTRAL_STAGE
                    && module_id == CONTROL_MODULE
                    && message.contains(&format!("{key_a:?}"))
        ),
        "runtime rejection must name the exact object-A key, got {runtime_error:?}"
    );
    assert!(blackboard.layer_selection_index().is_none());
}

fn raw_unknown_stage_rejects_at_the_pure_builder() {
    let scene = scene_with(&[]);
    let error = run_pure_stage(&scene, "Layer::NotAStage")
        .expect_err("a raw unknown stage string must be rejected");
    assert_eq!(
        error.error,
        SelectionError::UnknownLayerStage {
            stage_id: String::from("Layer::NotAStage")
        },
        "the pure builder must reject the raw unknown stage"
    );
    assert_eq!(error.stage_id, "Layer::NotAStage");
    assert_eq!(error.module_id, CONTROL_MODULE);
}

fn duplicate_publication_preserves_the_committed_index() {
    let scene = scene_with(&[chain_a(), reversed_chain_a()]);
    let (mut blackboard, plan) = prepare_blackboard(&scene);
    build_and_commit_layer_selection_index(&plan, &mut blackboard)
        .expect("the first publication must succeed");
    let original = Arc::clone(
        blackboard
            .layer_selection_index()
            .expect("the first publication must present an index"),
    );
    let first_targets = model_targets(&original, 1);

    let second = build_and_commit_layer_selection_index(&plan, &mut blackboard);
    let error = second.expect_err("a second publication must be rejected by the write-once seam");
    match error {
        LayerStageError::FatalModule {
            stage_id,
            module_id,
            ..
        } => {
            assert_eq!(stage_id.as_str(), SELECTION_STAGE);
            assert_eq!(module_id.as_str(), SELECTION_MODULE);
        }
        other => panic!("expected the write-once FatalModule, got {other:?}"),
    }

    let retained = blackboard
        .layer_selection_index()
        .expect("the original index must remain published");
    assert!(
        Arc::ptr_eq(&original, retained),
        "the previously committed index identity must be preserved"
    );
    assert_eq!(
        model_targets(retained, 1),
        first_targets,
        "the preserved index must keep its original observations"
    );
}
