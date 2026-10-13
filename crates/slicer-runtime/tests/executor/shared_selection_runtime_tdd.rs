//! AC-B4 / AC-BN1: the committed selection drives both real runtime executors.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use slicer_core::algos::paint_segmentation::paint_variant_region_id;
use slicer_ir::{
    AnchoredGeometryContract, BlackboardError, ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D,
    ExtrusionRole, GlobalLayer, LayerCollectionIR, LayerStageCommit, MeshIR, Point2,
    Point3WithWidth, Polygon, RegionKey, RegionMapIR, RegionPlan, ResolvedConfig, SliceIR,
    SlicedRegion, SupportPlanDeclineReason, SupportPlanEntry, SupportPlanIR, SupportPlanRole,
    SupportPlanRoleRegion,
};
use slicer_runtime::layer_executor::{
    execute_captured_stages_with_support_tools, module_invocation_allowed_on_layer,
    CaptureExecutionError, CaptureRequest, CapturedIr, LayerExecutionError, SupportToolSelection,
};
use slicer_runtime::{
    build_and_commit_layer_selection_index, build_wasm_instance_pool, execute_per_layer,
    Blackboard, CompiledModule, CompiledModuleBuilder, CompiledModuleLive, CompiledStage,
    ExecutionPlan, LayerStageError, LayerStageInput, LayerStageRunner, WasmArtifactMetadata,
    WasmComponent, WasmInstancePool, WasmRuntimeDispatcher,
};
use slicer_sdk::builders::{InfillOutputBuilder, PerimeterOutputBuilder, SupportOutputBuilder};
use slicer_sdk::error::ModuleError;
use slicer_sdk::native::{
    NativeLayerRequest, NativeLayerResponse, NativeStageEntry, NativeSupportOutput,
};
use slicer_sdk::test_support::fixtures::extrusion_path3d_base;
use slicer_sdk::views::SliceRegionView;
use slicer_wasm_host::selection::{
    build_layer_selection_index, LayerSelectionIndex, LayerSelectionIndexInput,
    LayerSelectionModuleInput, LayerTargetSource, OptionalProductPresence, SelectionError, STAGES,
};

type Handles = HashMap<
    String,
    (
        Arc<WasmInstancePool>,
        Option<Arc<WasmComponent>>,
        Option<NativeStageEntry>,
    ),
>;

const ANCHORED_ID: &str = "com.test.anchored-events-roundtrip";
// The real roundtrip guest emits events at this physical layer.
const LAYER: u32 = 7;
const SLICE_INDEX: usize = LAYER as usize;
const ANCHORED_GUEST: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../slicer-wasm-host/test-guests/anchored-events-roundtrip-guest.component.wasm"
);

#[derive(Clone, Copy)]
enum Output {
    None,
    Perimeters,
    Infill,
    Support,
}

#[derive(Clone)]
struct NativeCall {
    stage: &'static str,
    layer: u32,
    regions: Vec<SliceRegionView>,
}

struct ProbeState {
    output: Output,
    calls: Vec<NativeCall>,
}

// Function-pointer native entries cannot capture state. Invocation config keys
// select isolated probes, including when these two tests run concurrently.
static NEXT_PROBE: AtomicI64 = AtomicI64::new(1);
static PROBES: OnceLock<Mutex<HashMap<i64, Arc<Mutex<ProbeState>>>>> = OnceLock::new();

struct Probe {
    id: i64,
    state: Arc<Mutex<ProbeState>>,
}

impl Probe {
    fn new(output: Output) -> Self {
        let id = NEXT_PROBE.fetch_add(1, Ordering::Relaxed);
        let state = Arc::new(Mutex::new(ProbeState {
            output,
            calls: Vec::new(),
        }));
        PROBES
            .get_or_init(Mutex::default)
            .lock()
            .unwrap()
            .insert(id, Arc::clone(&state));
        Self { id, state }
    }

    fn module(&self, id: &str, semantics: &[&str], claims: &[&str]) -> CompiledModule {
        CompiledModuleBuilder::new(id)
            .region_split_semantics(semantics.iter().map(|s| (*s).into()).collect())
            .claims(claims.iter().map(|s| (*s).into()).collect())
            .config_view(Arc::new(ConfigView::from_map(HashMap::from([
                ("witness_id".into(), ConfigValue::Int(self.id)),
                ("selection_marker".into(), ConfigValue::Int(-1)),
            ]))))
            .build()
    }

    fn calls(&self) -> Vec<NativeCall> {
        self.state.lock().unwrap().calls.clone()
    }
}

impl Drop for Probe {
    fn drop(&mut self) {
        PROBES.get().unwrap().lock().unwrap().remove(&self.id);
    }
}

fn rectangle(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Vec<ExPolygon> {
    vec![ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(min_x, min_y),
                Point2::from_mm(max_x, min_y),
                Point2::from_mm(max_x, max_y),
                Point2::from_mm(min_x, max_y),
            ],
        },
        holes: Vec::new(),
    }]
}

fn path(z: f32, role: ExtrusionRole) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: vec![
            Point3WithWidth {
                x: 3.0,
                y: 4.0,
                z,
                width: 0.4,
                flow_factor: 1.0,
                ..Default::default()
            },
            Point3WithWidth {
                x: 7.0,
                y: 4.0,
                z,
                width: 0.4,
                flow_factor: 1.0,
                ..Default::default()
            },
        ],
        ..extrusion_path3d_base(role)
    }
}

fn native_entry(request: &NativeLayerRequest) -> Result<NativeLayerResponse, ModuleError> {
    let id = request.config.require_int("witness_id")?;
    let state = Arc::clone(
        PROBES
            .get()
            .unwrap()
            .lock()
            .unwrap()
            .get(&id)
            .expect("registered native runtime witness"),
    );
    let output = {
        let mut state = state.lock().unwrap();
        state.calls.push(NativeCall {
            stage: request.stage_export,
            layer: request.layer_index,
            regions: request.regions.clone(),
        });
        state.output
    };
    // exhaustive: each native output channel is explicit; None proves ordinary Ok(None) dispatch
    let mut response = NativeLayerResponse {
        infill: None,
        perimeters: None,
        support: None,
        slice_postprocess: None,
        path_optimization: None,
        anchored_events: None,
    };
    if !matches!(output, Output::None) {
        let region = request.regions.first().expect("positive selected region");
        match output {
            Output::Perimeters => {
                let mut builder = PerimeterOutputBuilder::new();
                builder.begin_region(
                    region.object_id(),
                    *region.region_id(),
                    region.variant_chain(),
                );
                builder.set_infill_areas(rectangle(2.0, 3.0, 8.0, 7.0))?;
                response.perimeters = Some(builder);
            }
            Output::Infill => {
                let mut builder = InfillOutputBuilder::new();
                builder.begin_region(
                    region.object_id(),
                    *region.region_id(),
                    region.variant_chain(),
                );
                builder.push_sparse_path(path(region.z(), ExtrusionRole::SparseInfill))?;
                response.infill = Some(builder);
            }
            Output::Support => {
                let mut builder = SupportOutputBuilder::new();
                builder.begin_region(
                    region.object_id(),
                    *region.region_id(),
                    region.variant_chain(),
                );
                builder.push_support_path(path(region.z(), ExtrusionRole::SupportMaterial))?;
                response.support = Some(NativeSupportOutput {
                    output: builder,
                    collection: slicer_sdk::LayerCollectionBuilder::new(),
                });
            }
            Output::None => unreachable!(),
        }
    }
    Ok(response)
}

fn install_native(handles: &mut Handles, stage: &str, module: &CompiledModule) {
    handles.insert(
        module.module_id().clone(),
        (
            pool(stage, module.module_id()),
            None,
            Some(NativeStageEntry::Layer(native_entry)),
        ),
    );
}

fn pool(stage: &str, id: &str) -> Arc<WasmInstancePool> {
    Arc::new(
        build_wasm_instance_pool(
            id,
            stage,
            true,
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("real runtime instance pool"),
    )
}

/// An observer, not an eligibility implementation: every call delegates to the
/// production dispatcher, and compares the borrowed authority by identity.
struct RuntimeWitness {
    dispatcher: WasmRuntimeDispatcher,
    index: Option<Arc<LayerSelectionIndex>>,
    commits: Mutex<Vec<(String, Option<LayerStageCommit>)>>,
}

impl RuntimeWitness {
    fn new(index: Option<Arc<LayerSelectionIndex>>) -> Self {
        Self {
            dispatcher: WasmRuntimeDispatcher::new(crate::common::wasm_cache::shared_engine()),
            index,
            commits: Mutex::new(Vec::new()),
        }
    }

    fn commits_for(&self, id: &str) -> Vec<Option<LayerStageCommit>> {
        self.commits
            .lock()
            .unwrap()
            .iter()
            .filter(|(module, _)| module == id)
            .map(|(_, commit)| commit.clone())
            .collect()
    }
}

impl LayerStageRunner for RuntimeWitness {
    fn run_stage(
        &self,
        stage: &String,
        layer: &GlobalLayer,
        module: &CompiledModuleLive<'_>,
        input: LayerStageInput<'_>,
    ) -> Result<Option<LayerStageCommit>, LayerStageError> {
        let expected = self
            .index
            .as_ref()
            .unwrap()
            .selection_for(layer.index, module.module_id)
            .expect("bound runtime selection");
        let delivered = input.selection.expect("runtime supplies indexed authority");
        assert!(
            std::ptr::eq(expected, delivered),
            "runtime must not reselect"
        );
        assert!(delivered.invoke, "declined work must not reach the runner");
        assert_eq!(&delivered.stage_id, stage);
        let result = self.dispatcher.run_stage(stage, layer, module, input);
        if let Ok(commit) = &result {
            self.commits
                .lock()
                .unwrap()
                .push((module.module_id.clone(), commit.clone()));
        }
        result
    }
}

/// Missing selection authority must fail before any module runner is entered.
#[derive(Default)]
struct RejectUnexpectedCall {
    calls: AtomicI64,
}

impl LayerStageRunner for RejectUnexpectedCall {
    fn run_stage(
        &self,
        stage: &String,
        layer: &GlobalLayer,
        module: &CompiledModuleLive<'_>,
        _input: LayerStageInput<'_>,
    ) -> Result<Option<LayerStageCommit>, LayerStageError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        panic!(
            "missing authority reached runner: {stage}/{} at layer {}",
            module.module_id, layer.index
        );
    }
}

fn plan(stages: Vec<CompiledStage>) -> ExecutionPlan {
    ExecutionPlan {
        per_layer_stages: stages,
        global_layers: Arc::new(vec![GlobalLayer {
            index: LAYER,
            z: 0.3,
            ..Default::default()
        }]),
        ..Default::default()
    }
}

fn painted_key() -> RegionKey {
    RegionKey {
        global_layer_index: LAYER,
        object_id: "model".into(),
        region_id: 7,
        variant_chain: vec![("material".into(), slicer_ir::PaintValue::ToolIndex(1))],
    }
}

/// Source-space twin of [`painted_key`]: production `build_layer` keys `Model`
/// targets by the slice's minted source region id (`paint_variant_region_id`),
/// while `painted_key()` names the map-space base entry the chain resolves to.
fn painted_source_key() -> RegionKey {
    let key = painted_key();
    let region_id = paint_variant_region_id(key.region_id, &key.variant_chain);
    RegionKey { region_id, ..key }
}

fn painted_slice() -> SliceIR {
    let key = painted_key();
    SliceIR {
        global_layer_index: LAYER,
        z: 0.3,
        regions: vec![SlicedRegion {
            object_id: key.object_id,
            region_id: paint_variant_region_id(key.region_id, &key.variant_chain),
            variant_chain: key.variant_chain,
            effective_layer_height: 0.3,
            polygons: rectangle(0.0, 0.0, 10.0, 10.0),
            infill_areas: rectangle(0.0, 0.0, 10.0, 10.0),
            sparse_infill_area: rectangle(0.0, 0.0, 10.0, 10.0),
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn board(
    plan: &ExecutionPlan,
    slice: Option<SliceIR>,
    map: Option<RegionMapIR>,
    support: Option<SupportPlanIR>,
) -> Blackboard {
    let mut board = Blackboard::new(Arc::new(MeshIR::default()), SLICE_INDEX + 1);
    crate::common::seed::seed_slice_ir(&mut board, plan);
    if let Some(slice) = slice {
        // Same public pre-Tier-2 replacement channel used by paint segmentation.
        let mut slices = board.slice_ir().unwrap().as_ref().clone();
        slices[SLICE_INDEX] = slice;
        board.replace_slice_ir(Arc::new(slices)).unwrap();
    }
    if let Some(map) = map {
        board.commit_region_map(Arc::new(map)).unwrap();
    }
    if let Some(support) = support {
        board.commit_support_plan(Arc::new(support)).unwrap();
    }
    board
}

fn exact_map(infill_id: &str) -> RegionMapIR {
    let mut map = RegionMapIR::default();
    let config = map.intern_config(ResolvedConfig {
        top_fill_holder: infill_id.into(),
        sparse_fill_holder: infill_id.into(),
        bottom_fill_holder: "com.test.other-holder".into(),
        extensions: BTreeMap::from([("selection_marker".into(), ConfigValue::Int(41))]),
        ..Default::default()
    });
    map.entries.insert(
        painted_key(),
        RegionPlan {
            config,
            ..Default::default()
        },
    );
    map
}

fn polygon_vertices(polygons: &[ExPolygon]) -> Vec<(i64, i64)> {
    assert_eq!(polygons.len(), 1, "independently expected single rectangle");
    assert!(polygons[0].holes.is_empty());
    let mut points: Vec<_> = polygons[0]
        .contour
        .points
        .iter()
        .map(|p| (p.x, p.y))
        .collect();
    points.sort_unstable();
    points
}

#[test]
fn schema_stages_and_production_eligibility() {
    let specs: Vec<_> = STAGES
        .iter()
        .filter(|s| s.tier_id == slicer_schema::TIER_LAYER && !s.wit_export.is_empty())
        .collect();
    assert!(
        !specs.is_empty(),
        "schema-backed stage population must exist"
    );
    let mut handles = Handles::new();
    let mut probes = Vec::new();
    let mut stages = Vec::new();
    let mut tested_stages = BTreeSet::new();
    let mut infill_id = String::new();
    for spec in specs {
        assert_eq!(spec.tier_id, slicer_schema::TIER_LAYER);
        assert!(!spec.wit_export.is_empty());
        let mut modules = Vec::new();
        for (case, semantics, invoke) in [
            ("transparent", Vec::new(), true),
            ("matching", vec!["material"], true),
            ("nonmatching", vec!["fuzzy_skin"], false),
        ] {
            let id = format!("com.test.{}.{}", spec.method, case);
            let output = match (spec.stage_id, case) {
                ("Layer::Perimeters", "transparent") => Output::Perimeters,
                ("Layer::Infill", "transparent") => Output::Infill,
                _ => Output::None,
            };
            let probe = Probe::new(output);
            let claims = if spec.stage_id == "Layer::Infill" {
                vec!["claim:top-fill", "claim:bottom-fill", "claim:sparse-fill"]
            } else {
                Vec::new()
            };
            let module = probe.module(&id, &semantics, &claims);
            if spec.stage_id == "Layer::Infill" && case == "transparent" {
                infill_id = id.clone();
            }
            install_native(&mut handles, spec.stage_id, &module);
            probes.push((spec.stage_id, id, probe, invoke));
            modules.push(module);
        }
        if spec.stage_id == "Layer::AnchoredEvents" {
            let engine = crate::common::wasm_cache::shared_engine();
            let bytes = std::fs::read(PathBuf::from(ANCHORED_GUEST))
                .expect("anchored positive-control component must exist; never skip");
            handles.insert(
                ANCHORED_ID.into(),
                (
                    pool(spec.stage_id, ANCHORED_ID),
                    Some(Arc::new(engine.compile_component(&bytes).unwrap())),
                    None,
                ),
            );
            modules.push(
                CompiledModuleBuilder::new(ANCHORED_ID)
                    .config_view(Arc::new(ConfigView::from_map(HashMap::from([(
                        "anchored_event_count".into(),
                        ConfigValue::Int(2),
                    )]))))
                    .build(),
            );
        }
        stages.push(CompiledStage {
            stage_id: spec.stage_id.into(),
            modules,
        });
    }
    assert!(!infill_id.is_empty());
    let plan = plan(stages);
    let mut board = board(
        &plan,
        Some(painted_slice()),
        Some(exact_map(&infill_id)),
        None,
    );
    build_and_commit_layer_selection_index(&plan, &mut board).unwrap();
    let index = Arc::clone(board.layer_selection_index().unwrap());
    let runner = RuntimeWitness::new(Some(Arc::clone(&index)));
    let infill_authority = index.selection_for(LAYER, &infill_id).unwrap();
    assert_eq!(infill_authority.targets.len(), 1);
    let infill_config = Arc::clone(&infill_authority.targets[0].config);
    let held = vec![
        "claim:top-fill".to_string(),
        "claim:sparse-fill".to_string(),
    ];
    assert_eq!(infill_authority.targets[0].held_claims, held);
    assert_eq!(infill_config.get_int("selection_marker"), Some(41));
    assert_eq!(
        infill_authority
            .invocation_config
            .get_int("selection_marker"),
        Some(-1)
    );

    for run_count in 1..=2 {
        if run_count == 1 {
            let layers = execute_per_layer(&plan, &board, &runner, &handles).unwrap();
            assert_eq!(layers.len(), 1);
            assert!(
                !layers[0].ordered_entities.is_empty(),
                "real native Infill committed"
            );
        } else {
            let captured = execute_captured_stages_with_support_tools(
                &plan,
                &board,
                &runner,
                &handles,
                SupportToolSelection::default(),
                &CaptureRequest {
                    stage_ids: vec![
                        "Layer::Perimeters".into(),
                        "Layer::Infill".into(),
                        "Layer::PathOptimization".into(),
                    ],
                    layer_indices: vec![LAYER],
                },
            )
            .unwrap();
            assert_eq!(captured.executed_layer_indices, vec![LAYER]);
            assert_eq!(captured.captures.len(), 3);
            for capture in captured.captures {
                match capture.ir {
                    CapturedIr::Perimeter(ir) => {
                        assert_eq!(ir.regions.len(), 1);
                        assert_eq!(
                            polygon_vertices(&ir.regions[0].infill_areas),
                            vec![
                                (20_000, 30_000),
                                (20_000, 70_000),
                                (80_000, 30_000),
                                (80_000, 70_000)
                            ]
                        );
                    }
                    CapturedIr::Infill(ir) => {
                        assert_eq!(ir.regions.len(), 1);
                        assert_eq!(ir.regions[0].sparse_infill.len(), 1);
                    }
                    CapturedIr::LayerCollection(ir) => assert!(!ir.ordered_entities.is_empty()),
                    other => panic!("unexpected schema-runtime tap: {other:?}"),
                }
            }
        }
        for (stage, id, probe, invoke) in &probes {
            let selection = index.selection_for(LAYER, id).unwrap();
            assert_eq!(
                selection.invoke, *invoke,
                "public decision for {stage}/{id}"
            );
            assert_eq!(
                module_invocation_allowed_on_layer(
                    plan.per_layer_stages
                        .iter()
                        .find(|s| s.stage_id == *stage)
                        .unwrap()
                        .modules
                        .iter()
                        .find(|m| m.module_id() == id)
                        .unwrap()
                        .region_split_semantics(),
                    Some(&board.slice_ir().unwrap()[SLICE_INDEX]),
                ),
                *invoke
            );
            let calls = probe.calls();
            assert_eq!(
                calls.len(),
                if *invoke { run_count } else { 0 },
                "native calls for {stage}/{id}"
            );
            assert_eq!(
                runner.commits_for(id).len(),
                calls.len(),
                "runtime skip before dispatcher"
            );
            if *invoke {
                tested_stages.insert(*stage);
                let call = calls.last().unwrap();
                assert_eq!(call.stage, *stage);
                assert_eq!(call.layer, LAYER);
                assert_eq!(call.regions.len(), 1);
                assert_eq!(
                    call.regions[0]
                        .config()
                        .unwrap()
                        .get_int("selection_marker"),
                    Some(41)
                );
                assert!(selection.targets.iter().any(|t| matches!(&t.source,
                    LayerTargetSource::Model { key, .. } if key == &painted_source_key())));
                if id == &infill_id {
                    // Analytic rectangle: the old 10x10 sparse domain is replaced
                    // by the 6x4 wall inset, with no bridge/bottom/top exclusions.
                    assert_eq!(
                        polygon_vertices(call.regions[0].sparse_infill_area()),
                        vec![
                            (20_000, 30_000),
                            (20_000, 70_000),
                            (80_000, 30_000),
                            (80_000, 70_000)
                        ]
                    );
                    assert_eq!(call.regions[0].held_claims(), held);
                    assert!(Arc::ptr_eq(&selection.targets[0].config, &infill_config));
                }
            } else {
                assert!(selection.targets.is_empty());
            }
        }
        let anchored = runner.commits_for(ANCHORED_ID);
        assert_eq!(
            anchored.len(),
            run_count,
            "actual anchored guest invokes once per driver"
        );
        match anchored.last().unwrap() {
            Some(LayerStageCommit::AnchoredEvents(collections)) => {
                assert_eq!(collections.len(), 1);
                assert_eq!(collections[0].anchor_global_layer_index, LAYER);
                assert_eq!(collections[0].events.len(), 2);
                assert_eq!(
                    collections[0].events[0].geometry,
                    AnchoredGeometryContract::Planar { z: 3000 }
                );
            }
            other => panic!("real anchored guest must commit nonempty events: {other:?}"),
        }
        assert!(Arc::ptr_eq(board.layer_selection_index().unwrap(), &index));
        assert_eq!(
            polygon_vertices(&board.slice_ir().unwrap()[SLICE_INDEX].regions[0].sparse_infill_area),
            vec![(0, 0), (0, 100_000), (100_000, 0), (100_000, 100_000)],
            "build-time slice is unchanged; only the current arena was partitioned"
        );
    }
    let schema_set: BTreeSet<_> = STAGES
        .iter()
        .filter(|s| s.tier_id == slicer_schema::TIER_LAYER && !s.wit_export.is_empty())
        .map(|s| s.stage_id)
        .collect();
    assert_eq!(tested_stages, schema_set, "no hand-maintained Layer roster");

    // Public seam rejects all schema non-Layer/host-only bindings, a known
    // non-roster builtin, and an unknown stage with the unchanged inner error.
    let rejected = STAGES
        .iter()
        .filter(|s| s.tier_id != slicer_schema::TIER_LAYER || s.wit_export.is_empty())
        .map(|s| s.stage_id)
        .chain(["Layer::Slice", "Layer::UnknownRuntimeWitness"]);
    let config = ConfigView::new();
    for stage in rejected {
        // exhaustive: all borrowed construction inputs are explicit for stage-validation control
        let failure = build_layer_selection_index(LayerSelectionIndexInput {
            stage_roster: STAGES,
            global_layers: plan.global_layers.as_slice(),
            // exhaustive: metadata channels of the rejected binding are explicit
            modules: &[LayerSelectionModuleInput {
                stage_id: stage,
                module_id: "com.test.rejected",
                claims: &[],
                paint_semantics: None,
                invocation_config: &config,
            }],
            slices: &[],
            region_map: None,
            support_plan: None,
            perimeters: &[],
        })
        .unwrap_err();
        assert_eq!(failure.stage_id, stage);
        assert_eq!(failure.module_id, "com.test.rejected");
        assert_eq!(
            failure.error,
            SelectionError::UnknownLayerStage {
                stage_id: stage.into()
            }
        );
    }

    missing_config_fails_before_layer_calls();
    missing_selection_authority_fails_before_layer_calls();
    for (support, invoke) in [
        (support_plan("tree", None), true),
        (support_plan("traditional", None), false),
        (
            support_plan("tree", Some(SupportPlanDeclineReason::DeclinedPolicy)),
            false,
        ),
    ] {
        support_carrier_case(Some(support), OptionalProductPresence::Present, invoke);
    }
}

fn missing_selection_authority_fails_before_layer_calls() {
    let stage = "Layer::Perimeters";
    let id = "com.test.missing-selection-authority";
    let probe = Probe::new(Output::Perimeters);
    let module = probe.module(id, &[], &[]);
    let mut handles = Handles::new();
    install_native(&mut handles, stage, &module);
    let execution_plan = plan(vec![CompiledStage {
        stage_id: stage.into(),
        modules: vec![module],
    }]);
    for missing_pair in [false, true] {
        for capture in [false, true] {
            let mut board = board(
                &execution_plan,
                Some(painted_slice()),
                Some(exact_map("com.test.unused")),
                None,
            );
            let detail = if missing_pair {
                // Commit through the real late-bind gate, but omit the scheduled
                // module from its input: the execution plan still schedules it.
                let binding_plan = plan(vec![CompiledStage {
                    stage_id: stage.into(),
                    modules: Vec::new(),
                }]);
                build_and_commit_layer_selection_index(&binding_plan, &mut board).unwrap();
                assert!(board
                    .layer_selection_index()
                    .unwrap()
                    .selection_for(LAYER, id)
                    .is_none());
                format!("internal consistency: layer selection is missing for layer {LAYER}")
            } else {
                assert!(board.layer_selection_index().is_none());
                "internal consistency: layer selection index is not committed".into()
            };
            let runner = RejectUnexpectedCall::default();
            let expected = LayerExecutionError::FatalLayer {
                layer_index: LAYER,
                stage_id: stage.into(),
                module_id: id.into(),
                message: LayerStageError::FatalModule {
                    stage_id: stage.into(),
                    module_id: id.into(),
                    message: detail,
                }
                .to_string(),
            };
            if capture {
                let result = execute_captured_stages_with_support_tools(
                    &execution_plan,
                    &board,
                    &runner,
                    &handles,
                    SupportToolSelection::default(),
                    &CaptureRequest {
                        stage_ids: vec![stage.into()],
                        layer_indices: vec![LAYER],
                    },
                );
                match result {
                    Err(CaptureExecutionError::Layer(error)) => assert_eq!(error, expected),
                    other => {
                        panic!("missing authority must abort capture without output: {other:?}")
                    }
                }
            } else {
                assert_eq!(
                    execute_per_layer(&execution_plan, &board, &runner, &handles),
                    Err(expected),
                    "missing authority must abort ordinary execution without output"
                );
            }
            assert_eq!(runner.calls.load(Ordering::Relaxed), 0);
            assert_eq!(probe.calls().len(), 0);
            assert_eq!(
                board.drain_layer_outputs(),
                Err(BlackboardError::IncompleteLayerDrain {
                    missing_indices: (0..=SLICE_INDEX).collect()
                }),
                "neither driver may commit a layer output after missing authority"
            );
        }
    }
}

fn missing_config_fails_before_layer_calls() {
    let probe = Probe::new(Output::Perimeters);
    let module = probe.module("com.test.missing-exact-variant", &[], &[]);
    let mut handles = Handles::new();
    install_native(&mut handles, "Layer::Perimeters", &module);
    let plan = plan(vec![CompiledStage {
        stage_id: "Layer::Perimeters".into(),
        modules: vec![module],
    }]);
    let mut map = exact_map("com.test.unused");
    let missing = painted_key();
    // A real base config is present. It must NOT authorize the painted variant.
    let value = map.entries.remove(&missing).unwrap();
    map.entries.insert(
        RegionKey {
            variant_chain: Vec::new(),
            ..missing.clone()
        },
        value,
    );
    let mut board = board(&plan, Some(painted_slice()), Some(map), None);
    let mut runner = RuntimeWitness::new(None);
    // The production late-bind gate, not a direct projector call: a false
    // success actually proceeds into Layer execution and trips the zero-call pin.
    let result = build_and_commit_layer_selection_index(&plan, &mut board).map(|()| {
        runner.index = board.layer_selection_index().cloned();
        execute_per_layer(&plan, &board, &runner, &handles)
    });
    match result {
        Err(LayerStageError::FatalModule {
            stage_id,
            module_id,
            message,
        }) => {
            assert_eq!(stage_id, "Layer::Perimeters");
            assert_eq!(module_id, "com.test.missing-exact-variant");
            assert_eq!(
                message,
                format!("{:?}", SelectionError::MissingModelConfig { key: missing })
            );
        }
        other => panic!("missing exact model config must fail at late bind: {other:?}"),
    }
    assert_eq!(probe.calls().len(), 0);
    assert_eq!(runner.commits.lock().unwrap().len(), 0);
    assert!(board.layer_selection_index().is_none(), "no partial index");
    assert_eq!(
        board.drain_layer_outputs(),
        Err(BlackboardError::IncompleteLayerDrain {
            missing_indices: (0..=SLICE_INDEX).collect()
        })
    );
}

fn support_plan(family: &str, decline_reason: Option<SupportPlanDeclineReason>) -> SupportPlanIR {
    SupportPlanIR {
        // exhaustive: producer provenance and eligibility channels are explicit for carrier controls
        entries: vec![SupportPlanEntry {
            global_layer_index: LAYER as i32,
            object_id: "support-only".into(),
            region_id: 13,
            family_id: family.into(),
            demand_ids: vec!["demand-13".into()],
            body_ids: vec!["body-13".into()],
            anchor_layer_index: 3,
            anchor_z: 3000,
            roles: vec![SupportPlanRoleRegion {
                role: SupportPlanRole::SupportBody,
                regions: rectangle(2.0, 3.0, 8.0, 7.0),
            }],
            skeleton: None,
            capabilities: vec!["extrusion.paths".into()],
            provenance: vec!["runtime-carrier-control".into()],
            decline_reason,
            // Genuinely chain-empty fixture source (design.md: empty chain is exact, never a wildcard).
            variant_chain: Vec::new(),
        }],
        ..Default::default()
    }
}

fn support_carrier_case(
    support: Option<SupportPlanIR>,
    presence: OptionalProductPresence,
    invoke: bool,
) {
    let probe = Probe::new(Output::Support);
    let id = "com.test.carrier-only-support";
    let module = probe.module(id, &[], &["support-family:tree"]);
    let mut handles = Handles::new();
    install_native(&mut handles, "Layer::Support", &module);
    let plan = plan(vec![CompiledStage {
        stage_id: "Layer::Support".into(),
        modules: vec![module],
    }]);
    let mut board = board(&plan, None, None, support);
    build_and_commit_layer_selection_index(&plan, &mut board).unwrap();
    let index = Arc::clone(board.layer_selection_index().unwrap());
    let selection = index.selection_for(LAYER, id).unwrap();
    assert_eq!(selection.support_plan_presence, presence);
    assert_eq!(selection.invoke, invoke);
    assert_eq!(selection.targets.len(), usize::from(invoke));
    assert!(selection.slice_regions.is_empty());
    assert!(selection.perimeter_configs.is_empty());
    assert!(
        board.region_map().is_none(),
        "carrier must not demand or fabricate a model config"
    );
    if invoke {
        let target = &selection.targets[0];
        assert_eq!(
            target.source,
            LayerTargetSource::SupportCarrier {
                entry_index: 0,
                object_id: "support-only".into(),
                region_id: 13,
                // Genuinely chain-empty fixture source (design.md: empty chain is exact, never a wildcard).
                variant_chain: Vec::new(),
                family: "tree".into(),
                demand_ids: vec!["demand-13".into()],
                body_ids: vec!["body-13".into()],
                provenance: vec!["runtime-carrier-control".into()],
                anchor_layer_index: 3,
            }
        );
        assert!(Arc::ptr_eq(&target.config, &selection.invocation_config));
        assert_eq!(target.held_claims, vec!["support-family:tree"]);
        assert!(!target.authored_coloring);
    }
    let runner = RuntimeWitness::new(Some(Arc::clone(&index)));
    let layers = execute_per_layer(&plan, &board, &runner, &handles).unwrap();
    assert_eq!(layers.len(), 1);
    assert_eq!(layers[0].ordered_entities.is_empty(), !invoke);
    assert_eq!(probe.calls().len(), usize::from(invoke));
    assert_eq!(runner.commits_for(id).len(), usize::from(invoke));
    let capture = execute_captured_stages_with_support_tools(
        &plan,
        &board,
        &runner,
        &handles,
        SupportToolSelection::default(),
        &CaptureRequest {
            stage_ids: vec!["Layer::Support".into()],
            layer_indices: vec![LAYER],
        },
    );
    if invoke {
        let capture = capture.unwrap();
        assert_eq!(capture.captures.len(), 1);
        match &capture.captures[0].ir {
            CapturedIr::Support(ir) => {
                assert_eq!(ir.entries.len(), 1);
                assert_eq!(ir.entries[0].object_id, "support-only");
                assert_eq!(ir.entries[0].region_id, 13);
                assert_eq!(ir.entries[0].family_id, "tree");
                assert_eq!(ir.entries[0].body_id, "body-13");
                assert_eq!(ir.entries[0].demand_ids, vec!["demand-13"]);
                assert_eq!(ir.entries[0].paths.len(), 1);
                assert_eq!(ir.entries[0].paths[0].points.len(), 2);
            }
            other => panic!("carrier positive must commit real SupportIR: {other:?}"),
        }
        let calls = probe.calls();
        assert_eq!(
            calls.len(),
            2,
            "one native call in each real runtime driver"
        );
        for call in calls {
            assert_eq!(call.regions.len(), 1);
            assert_eq!(call.regions[0].object_id(), "support-only");
            assert!(
                call.regions[0].polygons().is_empty(),
                "non-model carrier handle"
            );
            assert_eq!(call.regions[0].held_claims(), &["support-family:tree"]);
            assert_eq!(
                call.regions[0]
                    .config()
                    .unwrap()
                    .get_int("selection_marker"),
                Some(-1)
            );
        }
    } else {
        assert!(
            matches!(capture, Err(CaptureExecutionError::TapSourceUnavailable { stage_id, layer_index }) if stage_id == "Layer::Support" && layer_index == LAYER)
        );
        assert_eq!(probe.calls().len(), 0);
    }
    assert_eq!(runner.commits_for(id).len(), if invoke { 2 } else { 0 });
    assert!(Arc::ptr_eq(board.layer_selection_index().unwrap(), &index));
}

#[test]
fn optional_support_absence_and_explicit_empty() {
    for (support, presence, invoke) in [
        (None, OptionalProductPresence::Absent, false),
        (
            Some(SupportPlanIR::default()),
            OptionalProductPresence::Present,
            false,
        ),
        (
            Some(support_plan("tree", None)),
            OptionalProductPresence::Present,
            true,
        ),
    ] {
        support_carrier_case(support, presence, invoke);
    }

    let probe = Probe::new(Output::None);
    let id = "com.test.empty-transparent-infill";
    let module = probe.module(id, &[], &[]);
    let mut handles = Handles::new();
    install_native(&mut handles, "Layer::Infill", &module);
    let plan = plan(vec![CompiledStage {
        stage_id: "Layer::Infill".into(),
        modules: vec![module],
    }]);
    let mut board = board(&plan, None, None, Some(SupportPlanIR::default()));
    build_and_commit_layer_selection_index(&plan, &mut board).unwrap();
    let index = Arc::clone(board.layer_selection_index().unwrap());
    let selection = index.selection_for(LAYER, id).unwrap();
    assert!(
        selection.invoke,
        "transparent non-support invocation is not target nonemptiness"
    );
    assert!(selection.targets.is_empty());
    assert_eq!(
        selection.support_plan_presence,
        OptionalProductPresence::Present
    );
    let runner = RuntimeWitness::new(Some(index));
    let layers: Vec<LayerCollectionIR> =
        execute_per_layer(&plan, &board, &runner, &handles).unwrap();
    assert_eq!(layers.len(), 1);
    assert!(layers[0].ordered_entities.is_empty());
    let capture = execute_captured_stages_with_support_tools(
        &plan,
        &board,
        &runner,
        &handles,
        SupportToolSelection::default(),
        &CaptureRequest {
            stage_ids: vec!["Layer::Infill".into()],
            layer_indices: vec![LAYER],
        },
    );
    assert!(
        matches!(capture,
        Err(CaptureExecutionError::TapSourceUnavailable { stage_id, layer_index })
        if stage_id == "Layer::Infill" && layer_index == LAYER),
        "Ok(None) must not fabricate an Infill product for capture"
    );
    let calls = probe.calls();
    assert_eq!(calls.len(), 2);
    for call in calls {
        assert_eq!(call.stage, "Layer::Infill");
        assert!(
            call.regions.is_empty(),
            "explicitly empty delivered view list"
        );
    }
    assert_eq!(
        runner.commits_for(id),
        vec![None, None],
        "real dispatcher preserves Ok(None)"
    );
}
