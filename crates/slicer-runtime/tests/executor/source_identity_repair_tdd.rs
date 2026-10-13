//! AC-R6/R7/R8/R9: full-source identity repair drivers for the runtime executor.
//!
//! These are the Step 8 registered witnesses. Each driver runs every leg
//! through production drivers (`execute_per_layer` = ordinary,
//! `execute_captured_stages_with_support_tools` = capture) and collects leg
//! outcomes so a red leg never masks later legs; the summary assertion then
//! reports every unsatisfied full-source join in one failure.
//!
//! Pair-only production join sites these legs discriminate (pre-repair):
//! - `validate_infill_order_locks` (`crates/slicer-runtime/src/layer_executor.rs`)
//!   pairs previous/replacement regions by `(object_id, region_id)` only.
//! - `LayerStageCommit::PerimetersPostProcess` reattach pairs incoming
//!   regions against the original perimeter by `(object_id, region_id)` only.
//! - `LayerStageCommit::SlicePostProcess` apply positions regions by
//!   `(object_id, region_id)` only.
//! - `LayerStageCommit::Infill` raft attach finds slice regions by
//!   `(object_id, region_id)` only.
//! - `sync_perimeter_infill_areas_into_slice` uses the pair-keyed
//!   `perimeter_region_index` (`crates/slicer-wasm-host/src/dispatch.rs`).
//! - `marshal/native.rs` fabricates `variant_chain: Vec::new()` for the
//!   native `Layer::SlicePostProcess` conversion.
//!
//! Guest legs (R7 infill-linker, R9 lightning generator) are real artifact
//! calls; when the artifact is stale the leg fails with the exact typed
//! instantiation error (reported as DEFERRED, never skipped).

#![allow(missing_docs)]

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use slicer_core::algos::paint_segmentation::{
    paint_variant_region_id, PAINT_VARIANT_REGION_ID_STRIDE,
};
use slicer_ir::slice_ir::{SupportCandidate, SupportCandidateSource, SupportSourceKey};
use slicer_ir::{
    ActiveRegion, ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, GlobalLayer,
    InfillIR, InfillRegion, LayerStageCommit, LightningTreeEntry, LightningTreeIR, MeshIR,
    PaintValue, PerimeterIR, PerimeterRegion, Point2, Point3WithWidth, Polygon, RegionKey,
    RegionMapIR, RegionPlan, ResolvedConfig, SemVer, SliceIR, SlicedRegion, SupportPlanEntry,
    SupportPlanIR, SupportPlanRole, SupportPlanRoleRegion, CURRENT_INFILL_IR_SCHEMA_VERSION,
};
use slicer_ir::{
    IndexedTriangleSet, LayerPlanIR, ObjectLayerRef, ObjectMesh, Point3, SupportAnalysisIR,
    SupportGeometryIR, SupportGeometryKey, SurfaceClassificationIR, Transform3d,
};
use slicer_runtime::layer_executor::{
    execute_captured_stages_with_support_tools, CaptureExecutionError, CaptureRequest, CapturedIr,
    LayerExecutionError, SupportToolSelection,
};
use slicer_runtime::manifest::LoadedModuleBuilder;
use slicer_runtime::{
    build_and_commit_layer_selection_index, build_wasm_instance_pool, execute_per_layer,
    Blackboard, CompiledModule, CompiledModuleBuilder, CompiledModuleLive, CompiledStage,
    ExecutionPlan, LayerArena, LayerStageError, LayerStageInput, LayerStageRunner,
    WasmArtifactMetadata, WasmComponent, WasmInstancePool, WasmRuntimeDispatcher,
};
use slicer_sdk::builders::{InfillOutputBuilder, SlicePostprocessBuilder, SupportOutputBuilder};
use slicer_sdk::error::ModuleError;
use slicer_sdk::native::{
    NativeLayerRequest, NativeLayerResponse, NativeStageEntry, NativeSupportOutput,
};
use slicer_sdk::test_support::fixtures::extrusion_path3d_base;
use slicer_sdk::traits::PaintRegionLayerView;
use slicer_sdk::views::SliceRegionView;
use slicer_wasm_host::selection::LayerTargetSource;

use crate::common::{run_layer_and_commit_with_bundle, wasm_cache, TestModuleBundle};

const LAYER: u32 = 7;
const SLICE_INDEX: usize = LAYER as usize;

type Handles = HashMap<
    String,
    (
        Arc<WasmInstancePool>,
        Option<Arc<WasmComponent>>,
        Option<NativeStageEntry>,
    ),
>;

// ── geometry / identity fixtures ─────────────────────────────────────────────

fn chain_material(tool: u32) -> Vec<(String, PaintValue)> {
    vec![("material".to_string(), PaintValue::ToolIndex(tool))]
}

fn chain_permuted() -> Vec<(String, PaintValue)> {
    vec![
        ("fuzzy_skin".to_string(), PaintValue::Flag(true)),
        ("material".to_string(), PaintValue::ToolIndex(1)),
    ]
}

fn rect(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Vec<ExPolygon> {
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

fn rect_vertices(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Vec<(i64, i64)> {
    let at = |x: f32, y: f32| {
        let p = Point2::from_mm(x, y);
        (p.x, p.y)
    };
    let mut points = vec![
        at(min_x, min_y),
        at(max_x, min_y),
        at(max_x, max_y),
        at(min_x, max_y),
    ];
    points.sort_unstable();
    points
}

/// Sorted per-polygon and across-polygon vertex representation; rotation and
/// contour order of boolean results cannot leak into assertions.
fn poly_vertices(polygons: &[ExPolygon]) -> Vec<Vec<(i64, i64)>> {
    let mut out: Vec<Vec<(i64, i64)>> = polygons
        .iter()
        .map(|poly| {
            assert!(poly.holes.is_empty(), "fixture expects hole-free polygons");
            let mut points: Vec<(i64, i64)> =
                poly.contour.points.iter().map(|p| (p.x, p.y)).collect();
            points.sort_unstable();
            points
        })
        .collect();
    out.sort();
    out
}

fn ext_path(
    points: [(f32, f32); 2],
    lock: Option<u64>,
    z: f32,
    role: ExtrusionRole,
) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: points
            .iter()
            .map(|(x, y)| Point3WithWidth {
                x: *x,
                y: *y,
                z,
                width: 0.4,
                flow_factor: 1.0,
                ..Default::default()
            })
            .collect(),
        order_lock: lock,
        ..extrusion_path3d_base(role)
    }
}

fn empty_slice() -> SliceIR {
    SliceIR {
        global_layer_index: LAYER,
        z: 0.3,
        ..Default::default()
    }
}

fn sliced_region(
    object_id: &str,
    region_id: u64,
    chain: Vec<(String, PaintValue)>,
    polygons: Vec<ExPolygon>,
    prepass_sparse: Vec<ExPolygon>,
) -> SlicedRegion {
    SlicedRegion {
        object_id: object_id.to_string(),
        region_id,
        variant_chain: chain,
        effective_layer_height: 0.3,
        polygons,
        infill_areas: rect(0.0, 0.0, 10.0, 10.0),
        sparse_infill_area: prepass_sparse,
        ..Default::default()
    }
}

/// Two source regions sharing `(object, region)` but differing only by ordered
/// chain, each with an independent prepass sparse payload.
fn chained_slice_shared_pair() -> SliceIR {
    SliceIR {
        global_layer_index: LAYER,
        z: 0.3,
        regions: vec![
            sliced_region(
                "model",
                LAYER as u64,
                chain_material(1),
                rect(0.0, 0.0, 10.0, 10.0),
                rect(9.0, 9.0, 10.0, 10.0),
            ),
            sliced_region(
                "model",
                LAYER as u64,
                chain_material(2),
                rect(0.0, 0.0, 10.0, 10.0),
                rect(8.0, 8.0, 10.0, 10.0),
            ),
        ],
        ..Default::default()
    }
}

/// Production wire shape: source regions carry derived paint-variant ids
/// (`base * STRIDE + hash`) while the Layer::Infill wire adapter delivers the
/// historical map-space address to both legs; the full source identity is the
/// derived id plus the ordered chain.
fn chained_slice_derived_pair() -> SliceIR {
    let chain_a = chain_material(1);
    let chain_b = chain_material(2);
    SliceIR {
        global_layer_index: LAYER,
        z: 0.3,
        regions: vec![
            sliced_region(
                "model",
                paint_variant_region_id(7, &chain_a),
                chain_a,
                rect(0.0, 0.0, 10.0, 10.0),
                rect(9.0, 9.0, 10.0, 10.0),
            ),
            sliced_region(
                "model",
                paint_variant_region_id(7, &chain_b),
                chain_b,
                rect(0.0, 0.0, 10.0, 10.0),
                rect(8.0, 8.0, 10.0, 10.0),
            ),
        ],
        ..Default::default()
    }
}

// ── plan / board helpers ─────────────────────────────────────────────────────

fn execution_plan(stages: Vec<CompiledStage>) -> ExecutionPlan {
    // exhaustive: plan fixtures pin the stage list and the single physical layer.
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

fn compiled_stage(stage_id: &str, modules: Vec<CompiledModule>) -> CompiledStage {
    CompiledStage {
        stage_id: stage_id.to_string(),
        modules,
    }
}

fn plain_module(id: &str) -> CompiledModule {
    CompiledModuleBuilder::new(id)
        .config_view(Arc::new(ConfigView::from_map(HashMap::new())))
        .build()
}

fn map_space_region_id(region_id: u64, chain: &[(String, PaintValue)]) -> u64 {
    if chain.is_empty() || slicer_ir::modifier_base_region_id(region_id).is_some() {
        region_id
    } else {
        region_id / PAINT_VARIANT_REGION_ID_STRIDE
    }
}

fn region_map_for(slice: &SliceIR, sparse_holder: Option<&str>) -> RegionMapIR {
    let mut map = RegionMapIR::default();
    let config = map.intern_config(ResolvedConfig {
        sparse_fill_holder: sparse_holder.unwrap_or_default().to_string(),
        extensions: BTreeMap::from([(
            "infill_anchor_max".to_string(),
            ConfigValue::FloatOrPercent {
                value: 20.0,
                is_percent: false,
            },
        )]),
        ..Default::default()
    });
    for region in &slice.regions {
        map.entries.insert(
            RegionKey {
                global_layer_index: slice.global_layer_index,
                object_id: region.object_id.clone(),
                region_id: map_space_region_id(region.region_id, &region.variant_chain),
                variant_chain: region.variant_chain.clone(),
            },
            RegionPlan {
                config,
                ..Default::default()
            },
        );
    }
    map
}

fn board_with_slice(plan: &ExecutionPlan, slice: SliceIR) -> Blackboard {
    let mut board = Blackboard::new(Arc::new(MeshIR::default()), SLICE_INDEX + 1);
    crate::common::seed::seed_slice_ir(&mut board, plan);
    let mut slices = board
        .slice_ir()
        .expect("seeded slice vector")
        .as_ref()
        .clone();
    slices[SLICE_INDEX] = slice.clone();
    board
        .replace_slice_ir(Arc::new(slices))
        .expect("replace slice");
    board
        .commit_region_map(Arc::new(region_map_for(&slice, Some("lightning-infill"))))
        .expect("region map");
    board
}

// ── runners ──────────────────────────────────────────────────────────────────

/// Deterministic producer seam for commit-join legs: the executor and `apply`
/// stay production; only the commit payload is scripted per module id.
#[derive(Default)]
struct ScriptedRunner {
    queue: Mutex<HashMap<String, Vec<LayerStageCommit>>>,
}

impl ScriptedRunner {
    fn push(&self, module_id: &str, commit: LayerStageCommit) {
        self.queue
            .lock()
            .unwrap()
            .entry(module_id.to_string())
            .or_default()
            .push(commit);
    }

    fn has(&self, module_id: &str) -> bool {
        self.queue.lock().unwrap().contains_key(module_id)
    }
}

impl LayerStageRunner for ScriptedRunner {
    fn run_stage(
        &self,
        _stage: &String,
        _layer: &GlobalLayer,
        module: &CompiledModuleLive<'_>,
        _input: LayerStageInput<'_>,
    ) -> Result<Option<LayerStageCommit>, LayerStageError> {
        let mut queue = self.queue.lock().unwrap();
        let staged = match queue.get_mut(module.module_id.as_str()) {
            Some(commits) if !commits.is_empty() => Some(commits.remove(0)),
            _ => None,
        };
        Ok(staged)
    }
}

/// Scripted commits for the named module ids, production dispatcher for every
/// other module (native transport legs).
struct MixedRunner {
    scripted: ScriptedRunner,
    dispatcher: WasmRuntimeDispatcher,
}

impl LayerStageRunner for MixedRunner {
    fn run_stage(
        &self,
        stage: &String,
        layer: &GlobalLayer,
        module: &CompiledModuleLive<'_>,
        input: LayerStageInput<'_>,
    ) -> Result<Option<LayerStageCommit>, LayerStageError> {
        if self.scripted.has(module.module_id.as_str()) {
            self.scripted.run_stage(stage, layer, module, input)
        } else {
            self.dispatcher.run_stage(stage, layer, module, input)
        }
    }
}

// ── native transport probes ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct SeenRegion {
    object_id: String,
    region_id: u64,
    chain: Vec<(String, PaintValue)>,
    polygons: Vec<Vec<(i64, i64)>>,
    raft_fill: Vec<Vec<(i64, i64)>>,
    sparse_infill_area: Vec<Vec<(i64, i64)>>,
}

#[derive(Clone)]
struct ProbeCall {
    stage: &'static str,
    regions: Vec<SeenRegion>,
}

#[derive(Default)]
struct ProbeLog {
    calls: Vec<ProbeCall>,
}

static NEXT_PROBE: AtomicI64 = AtomicI64::new(1);
static PROBES: OnceLock<Mutex<HashMap<i64, Arc<Mutex<ProbeLog>>>>> = OnceLock::new();

struct Probe {
    id: i64,
    state: Arc<Mutex<ProbeLog>>,
}

impl Probe {
    fn new() -> Self {
        let id = NEXT_PROBE.fetch_add(1, Ordering::Relaxed);
        let state = Arc::new(Mutex::new(ProbeLog::default()));
        PROBES
            .get_or_init(Mutex::default)
            .lock()
            .unwrap()
            .insert(id, Arc::clone(&state));
        Self { id, state }
    }

    fn module(&self, id: &str, claims: &[&str], ints: &[(&str, i64)]) -> CompiledModule {
        let mut map = HashMap::from([("probe_id".to_string(), ConfigValue::Int(self.id))]);
        for (key, value) in ints {
            map.insert((*key).to_string(), ConfigValue::Int(*value));
        }
        CompiledModuleBuilder::new(id)
            .config_view(Arc::new(ConfigView::from_map(map)))
            .claims(claims.iter().map(|claim| (*claim).to_string()).collect())
            .build()
    }

    fn calls(&self) -> Vec<ProbeCall> {
        self.state.lock().unwrap().calls.clone()
    }
}

impl Drop for Probe {
    fn drop(&mut self) {
        if let Some(registry) = PROBES.get() {
            registry.lock().unwrap().remove(&self.id);
        }
    }
}

fn seen_region(region: &SliceRegionView) -> SeenRegion {
    SeenRegion {
        object_id: region.object_id().to_string(),
        region_id: *region.region_id(),
        chain: region.variant_chain().to_vec(),
        polygons: poly_vertices(region.polygons()),
        raft_fill: poly_vertices(region.raft_fill()),
        sparse_infill_area: poly_vertices(region.sparse_infill_area()),
    }
}

fn record_request(request: &NativeLayerRequest) -> Result<(), ModuleError> {
    let id = request.config.require_int("probe_id")?;
    let state = Arc::clone(
        PROBES
            .get()
            .expect("probe registry initialized")
            .lock()
            .unwrap()
            .get(&id)
            .expect("registered probe"),
    );
    state.lock().unwrap().calls.push(ProbeCall {
        stage: request.stage_export,
        regions: request.regions.iter().map(seen_region).collect(),
    });
    Ok(())
}

fn empty_native_response() -> NativeLayerResponse {
    // exhaustive: every native output channel is explicit; None proves a non-emitting observer.
    NativeLayerResponse {
        infill: None,
        perimeters: None,
        support: None,
        slice_postprocess: None,
        path_optimization: None,
        anchored_events: None,
    }
}

/// Records the delivered regions and commits nothing.
fn observer_entry(request: &NativeLayerRequest) -> Result<NativeLayerResponse, ModuleError> {
    record_request(request)?;
    Ok(empty_native_response())
}

/// Real native `Layer::Infill` producer: per-region sparse path plus a raft
/// polygon tagged with the delivered region's full origin. `extra_absent_chain`
/// adds a raft group whose chain has no slice region on the layer.
fn raft_producer_entry(request: &NativeLayerRequest) -> Result<NativeLayerResponse, ModuleError> {
    record_request(request)?;
    let mut builder = InfillOutputBuilder::new();
    for region in &request.regions {
        if region.object_id() != "model" {
            continue;
        }
        builder.begin_region(
            region.object_id(),
            *region.region_id(),
            region.variant_chain(),
        );
        builder.push_sparse_path(ext_path(
            [(0.0, 0.0), (1.0, 0.0)],
            None,
            region.z(),
            ExtrusionRole::SparseInfill,
        ))?;
        let raft = if region.variant_chain() == chain_material(2).as_slice() {
            rect(3.0, 3.0, 4.0, 4.0)
        } else {
            rect(1.0, 1.0, 2.0, 2.0)
        };
        builder.push_raft_fill(raft)?;
    }
    if request.config.require_int("extra_absent_chain")? == 1 {
        let absent = chain_material(3);
        builder.begin_region("model", 7, &absent);
        builder.push_raft_fill(rect(5.0, 5.0, 6.0, 6.0))?;
    }
    let mut response = empty_native_response();
    response.infill = Some(builder);
    Ok(response)
}

/// Real native `Layer::SlicePostProcess` producer: replaces the polygons of the
/// delivered region whose chain matches `target_tool` (source identity from the
/// delivered view), or of a pair-matching key with the target chain when no
/// delivered region matches (missing-chain control).
fn slice_postprocess_entry(
    request: &NativeLayerRequest,
) -> Result<NativeLayerResponse, ModuleError> {
    record_request(request)?;
    let target = chain_material(request.config.require_int("target_tool")? as u32);
    let delivered = request.regions.iter().find(|region| {
        region.object_id() == "model" && region.variant_chain() == target.as_slice()
    });
    let (object_id, region_id) = delivered
        .map(|region| (region.object_id().to_string(), *region.region_id()))
        .unwrap_or_else(|| ("model".to_string(), LAYER as u64));
    let mut builder = SlicePostprocessBuilder::new();
    builder.set_polygons(
        RegionKey {
            global_layer_index: request.layer_index,
            object_id,
            region_id,
            variant_chain: target,
        },
        rect(1.0, 1.0, 9.0, 9.0),
    )?;
    let mut response = empty_native_response();
    response.slice_postprocess = Some(builder);
    Ok(response)
}

/// Real native `Layer::Support` producer: one support path per delivered
/// carrier region, tagged with that region's full origin.
fn support_emit_entry(request: &NativeLayerRequest) -> Result<NativeLayerResponse, ModuleError> {
    record_request(request)?;
    let mut builder = SupportOutputBuilder::new();
    for region in &request.regions {
        builder.begin_region(
            region.object_id(),
            *region.region_id(),
            region.variant_chain(),
        );
        builder.push_support_path(ext_path(
            [(0.0, 0.0), (1.0, 0.0)],
            None,
            region.z(),
            ExtrusionRole::SupportMaterial,
        ))?;
    }
    let mut response = empty_native_response();
    response.support = Some(NativeSupportOutput {
        output: builder,
        collection: slicer_sdk::LayerCollectionBuilder::new(),
    });
    Ok(response)
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
        .expect("instance pool"),
    )
}

fn install_native(
    handles: &mut Handles,
    stage: &str,
    module: &CompiledModule,
    entry: fn(&NativeLayerRequest) -> Result<NativeLayerResponse, ModuleError>,
) {
    handles.insert(
        module.module_id().clone(),
        (
            pool(stage, module.module_id()),
            None,
            Some(NativeStageEntry::Layer(entry)),
        ),
    );
}

// ── real guest bundles (R7 infill-linker, R9 lightning generator) ────────────

fn fixture_semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn guest_bundle(module_id: &str, stage: &str, wasm_name: &str) -> TestModuleBundle {
    let wasm_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("modules/core-modules")
        .join(wasm_name)
        .join(format!("{wasm_name}.wasm"));
    assert!(
        wasm_path.exists(),
        "{wasm_name}.wasm not found at {}; build it with `cargo xtask build-guests`",
        wasm_path.display()
    );
    let claims = if wasm_name == "lightning-infill" {
        vec!["claim:sparse-fill".to_string()]
    } else {
        vec!["claim:infill-link".to_string()]
    };
    let loaded = LoadedModuleBuilder::new(
        module_id,
        fixture_semver(0, 1, 0),
        stage,
        slicer_schema::TIER_LAYER,
        wasm_path.clone(),
    )
    .ir_reads(vec!["SliceIR".to_string(), "InfillIR".to_string()])
    .ir_writes(vec!["InfillIR".to_string()])
    .claims(claims.clone())
    .min_host_version(fixture_semver(0, 1, 0))
    .min_ir_schema(fixture_semver(3, 0, 0))
    .max_ir_schema(fixture_semver(5, 0, 0))
    .layer_parallel_safe(true)
    .build();
    let pool = Arc::new(
        build_wasm_instance_pool(
            loaded.id(),
            loaded.stage(),
            loaded.layer_parallel_safe(),
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("instance pool"),
    );
    let config = ConfigView::from_map(HashMap::from([
        ("layer_height".to_string(), ConfigValue::Float(0.2)),
        (
            "infill_anchor_max".to_string(),
            ConfigValue::FloatOrPercent {
                value: 20.0,
                is_percent: false,
            },
        ),
        ("infill_density".to_string(), ConfigValue::Float(0.2)),
        ("infill_overlap".to_string(), ConfigValue::Float(0.45)),
        ("infill_speed".to_string(), ConfigValue::Float(50.0)),
        ("line_width".to_string(), ConfigValue::Float(0.4)),
        ("bridge_line_width".to_string(), ConfigValue::Float(0.0)),
        (
            "initial_layer_line_width".to_string(),
            ConfigValue::Float(0.0),
        ),
    ]));
    let module = CompiledModuleBuilder::new(loaded.id().to_string())
        .config_view(Arc::new(config))
        .claims(claims)
        .build();
    TestModuleBundle {
        module,
        pool,
        component: Some(wasm_cache::compiled_component_at(&wasm_path)),
    }
}

// ── R6 legs: runtime commit joins ────────────────────────────────────────────

fn infill_region(chain: Vec<(String, PaintValue)>, paths: Vec<ExtrusionPath3D>) -> InfillRegion {
    InfillRegion {
        object_id: "model".to_string(),
        region_id: 7,
        variant_chain: chain,
        sparse_infill: paths,
        ..Default::default()
    }
}

fn infill_ir(regions: Vec<InfillRegion>) -> InfillIR {
    InfillIR {
        schema_version: CURRENT_INFILL_IR_SCHEMA_VERSION,
        global_layer_index: LAYER,
        regions,
        ..Default::default()
    }
}

fn perimeter_region(
    chain: Vec<(String, PaintValue)>,
    infill_areas: Vec<ExPolygon>,
) -> PerimeterRegion {
    PerimeterRegion {
        object_id: "model".to_string(),
        region_id: 7,
        variant_chain: chain,
        infill_areas,
        ..Default::default()
    }
}

fn perimeter_ir(regions: Vec<PerimeterRegion>) -> PerimeterIR {
    PerimeterIR {
        global_layer_index: LAYER,
        regions,
        ..Default::default()
    }
}

fn locked_path_a() -> ExtrusionPath3D {
    ext_path(
        [(0.0, 0.0), (1.0, 0.0)],
        Some(1),
        0.3,
        ExtrusionRole::SparseInfill,
    )
}

fn locked_path_b() -> ExtrusionPath3D {
    ext_path(
        [(0.0, 0.0), (2.0, 0.0)],
        Some(2),
        0.3,
        ExtrusionRole::SparseInfill,
    )
}

fn drive_lock_ordinary(
    replacement: InfillIR,
) -> Result<Vec<LayerCollectionIRAlias>, LayerExecutionError> {
    let infill_id = "com.test.r6.lock.infill";
    let post_id = "com.test.r6.lock.post";
    let plan = execution_plan(vec![
        compiled_stage("Layer::Infill", vec![plain_module(infill_id)]),
        compiled_stage("Layer::InfillPostProcess", vec![plain_module(post_id)]),
    ]);
    let mut board = board_with_slice(&plan, empty_slice());
    build_and_commit_layer_selection_index(&plan, &mut board).expect("selection index");
    let runner = ScriptedRunner::default();
    runner.push(
        infill_id,
        LayerStageCommit::Infill(infill_ir(vec![
            infill_region(chain_material(1), vec![locked_path_a()]),
            infill_region(chain_material(2), vec![locked_path_b()]),
        ])),
    );
    runner.push(post_id, LayerStageCommit::InfillPostProcess(replacement));
    let handles = Handles::new();
    execute_per_layer(&plan, &board, &runner, &handles)
}

type LayerCollectionIRAlias = slicer_ir::LayerCollectionIR;

/// AC-R6 negative: a chain-B-only replacement that reattributes chain-A's
/// locked payload is a full-source drop of the chain-A region and MUST raise
/// `OrderLockViolation`. Pair-only validates both blocks inside the first
/// pair-matching region and accepts.
fn leg_r6_lock_sibling_replacement_rejected() -> Result<(), String> {
    let replacement = infill_ir(vec![infill_region(
        chain_material(2),
        vec![locked_path_a(), locked_path_b()],
    )]);
    match drive_lock_ordinary(replacement) {
        Err(LayerExecutionError::FatalLayer { message, .. })
            if message.to_lowercase().contains("lock") =>
        {
            Ok(())
        }
        Err(other) => Err(format!(
            "expected OrderLockViolation, got different error: {other}"
        )),
        Ok(_) => Err(
            "pair-only validate_infill_order_locks accepted a chain-B-only replacement carrying \
             chain-A's locked payload (chain-A region dropped); expected OrderLockViolation"
                .to_string(),
        ),
    }
}

/// AC-R6 positive: the compliant two-variant replacement (each variant carries
/// its own locked payload) is exactly what a full-source join accepts. Pair-only
/// validates the chain-B block against the first pair-matching (chain-A)
/// region and rejects the compliant commit.
fn leg_r6_lock_compliant_two_variant_accepted() -> Result<(), String> {
    let replacement = infill_ir(vec![
        infill_region(chain_material(1), vec![locked_path_a()]),
        infill_region(chain_material(2), vec![locked_path_b()]),
    ]);
    match drive_lock_ordinary(replacement) {
        Ok(_) => Ok(()),
        Err(error) => Err(format!(
            "full-source compliant two-variant replacement must be accepted; pair-only rejected \
             it: {error}"
        )),
    }
}

fn drive_infill_capture(replacement: InfillIR) -> Result<Vec<CapturedIr>, CaptureExecutionError> {
    let infill_id = "com.test.r6.lock-capture.infill";
    let post_id = "com.test.r6.lock-capture.post";
    let plan = execution_plan(vec![
        compiled_stage("Layer::Infill", vec![plain_module(infill_id)]),
        compiled_stage("Layer::InfillPostProcess", vec![plain_module(post_id)]),
    ]);
    let mut board = board_with_slice(&plan, empty_slice());
    build_and_commit_layer_selection_index(&plan, &mut board).expect("selection index");
    let runner = ScriptedRunner::default();
    runner.push(
        infill_id,
        LayerStageCommit::Infill(infill_ir(vec![
            infill_region(chain_material(1), vec![locked_path_a()]),
            infill_region(chain_material(2), vec![locked_path_b()]),
        ])),
    );
    runner.push(post_id, LayerStageCommit::InfillPostProcess(replacement));
    let handles = Handles::new();
    let capture = execute_captured_stages_with_support_tools(
        &plan,
        &board,
        &runner,
        &handles,
        SupportToolSelection::default(),
        &CaptureRequest {
            stage_ids: vec!["Layer::Infill".into(), "Layer::InfillPostProcess".into()],
            layer_indices: vec![LAYER],
        },
    )?;
    Ok(capture.captures.into_iter().map(|c| c.ir).collect())
}

/// AC-R6 capture-driver twin of the compliant commit, plus source-output ID
/// roundtrip: both regions keep region id 7 and their own ordered chains and
/// payloads after the commit.
fn leg_r6_lock_compliant_two_variant_capture() -> Result<(), String> {
    let replacement = infill_ir(vec![
        infill_region(chain_material(1), vec![locked_path_a()]),
        infill_region(chain_material(2), vec![locked_path_b()]),
    ]);
    let mut problems = Vec::new();
    let captures = match drive_infill_capture(replacement) {
        Ok(captures) => captures,
        Err(error) => {
            return Err(format!(
                "capture driver must accept the compliant two-variant replacement; pair-only \
                 rejected it: {error}"
            ))
        }
    };
    let mut infill_captures = 0usize;
    for captured in captures {
        let CapturedIr::Infill(ir) = captured else {
            continue;
        };
        infill_captures += 1;
        if ir.regions.len() != 2 {
            problems.push(format!(
                "committed infill must keep both full-source variants; got {} regions",
                ir.regions.len()
            ));
            continue;
        }
        for (chain, expected_points) in [
            (chain_material(1), locked_path_a().points),
            (chain_material(2), locked_path_b().points),
        ] {
            let Some(region) = ir.regions.iter().find(|r| r.variant_chain == chain) else {
                problems.push(format!("committed infill lost chain {chain:?}"));
                continue;
            };
            if region.region_id != 7 {
                problems.push(format!(
                    "source-output ID must roundtrip unchanged for {chain:?}; got region_id {}",
                    region.region_id
                ));
            }
            if region.sparse_infill.len() != 1 || region.sparse_infill[0].points != expected_points
            {
                problems.push(format!(
                    "committed payload for {chain:?} must stay its own; got {:?}",
                    region.sparse_infill
                ));
            }
        }
    }
    if infill_captures == 0 {
        problems.push("capture produced no Infill tap".to_string());
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

fn drive_perimeter_capture(
    original: PerimeterIR,
    incoming: PerimeterIR,
) -> Result<PerimeterIR, String> {
    let pre_id = "com.test.r6.perimeter.pre";
    let post_id = "com.test.r6.perimeter.post";
    let plan = execution_plan(vec![
        compiled_stage("Layer::Perimeters", vec![plain_module(pre_id)]),
        compiled_stage("Layer::PerimetersPostProcess", vec![plain_module(post_id)]),
    ]);
    let mut board = board_with_slice(&plan, empty_slice());
    build_and_commit_layer_selection_index(&plan, &mut board).expect("selection index");
    let runner = ScriptedRunner::default();
    runner.push(pre_id, LayerStageCommit::Perimeters(original));
    runner.push(
        post_id,
        LayerStageCommit::PerimetersPostProcess(Some(incoming)),
    );
    let handles = Handles::new();
    let capture = execute_captured_stages_with_support_tools(
        &plan,
        &board,
        &runner,
        &handles,
        SupportToolSelection::default(),
        &CaptureRequest {
            stage_ids: vec![
                "Layer::Perimeters".into(),
                "Layer::PerimetersPostProcess".into(),
            ],
            layer_indices: vec![LAYER],
        },
    )
    .map_err(|error| format!("perimeter capture driver failed: {error:?}"))?;
    capture
        .captures
        .iter()
        .rev()
        .find_map(|capture| match &capture.ir {
            CapturedIr::Perimeter(ir) => Some(ir.clone()),
            _ => None,
        })
        .ok_or_else(|| "no Perimeter tap captured".to_string())
}

/// AC-R6 negative: a post-process region with chain B and empty preserved
/// fields must be reattached from the chain-B original, never from the first
/// pair-matching (chain-A) sibling.
fn leg_r6_perimeter_reattach_chain_b_own_fields() -> Result<(), String> {
    let original = perimeter_ir(vec![
        perimeter_region(chain_material(1), rect(2.0, 3.0, 8.0, 7.0)),
        perimeter_region(chain_material(2), rect(4.0, 3.0, 6.0, 7.0)),
    ]);
    let incoming = perimeter_ir(vec![perimeter_region(chain_material(2), Vec::new())]);
    let committed = drive_perimeter_capture(original, incoming)?;
    let region = committed
        .regions
        .iter()
        .find(|region| region.variant_chain == chain_material(2))
        .ok_or_else(|| "committed post-process perimeter lost chain B".to_string())?;
    let expected = vec![rect_vertices(4.0, 3.0, 6.0, 7.0)];
    let actual = poly_vertices(&region.infill_areas);
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "chain-B post-process reattach must preserve its own chain-B fields; pair-only \
             borrowed the chain-A sibling: actual={actual:?} expected={expected:?}"
        ))
    }
}

/// AC-R6 positive twin: the same reattach for chain A (pair-first match)
/// preserves chain-A fields under both implementations; proves the harness
/// exercises the real reattach path.
fn leg_r6_perimeter_reattach_chain_a_positive_twin() -> Result<(), String> {
    let original = perimeter_ir(vec![
        perimeter_region(chain_material(1), rect(2.0, 3.0, 8.0, 7.0)),
        perimeter_region(chain_material(2), rect(4.0, 3.0, 6.0, 7.0)),
    ]);
    let incoming = perimeter_ir(vec![perimeter_region(chain_material(1), Vec::new())]);
    let committed = drive_perimeter_capture(original, incoming)?;
    let region = committed
        .regions
        .iter()
        .find(|region| region.variant_chain == chain_material(1))
        .ok_or_else(|| "committed post-process perimeter lost chain A".to_string())?;
    let expected = vec![rect_vertices(2.0, 3.0, 8.0, 7.0)];
    let actual = poly_vertices(&region.infill_areas);
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "chain-A reattach positive twin failed: actual={actual:?} expected={expected:?}"
        ))
    }
}

fn drive_slice_postprocess_scripted(
    target_chain: Vec<(String, PaintValue)>,
) -> Result<Vec<SeenRegion>, String> {
    let update_id = "com.test.r6.slice-post.update";
    let observer = Probe::new();
    let observer_id = "com.test.r6.slice-post.observer";
    let observer_module = observer.module(observer_id, &[], &[]);
    let plan = execution_plan(vec![
        compiled_stage("Layer::SlicePostProcess", vec![plain_module(update_id)]),
        compiled_stage("Layer::Infill", vec![observer_module.clone()]),
    ]);
    let slice = chained_slice_shared_pair();
    let mut board = board_with_slice(&plan, slice);
    build_and_commit_layer_selection_index(&plan, &mut board).expect("selection index");
    let scripted = ScriptedRunner::default();
    scripted.push(
        update_id,
        LayerStageCommit::SlicePostProcess {
            polygon_updates: vec![(
                RegionKey {
                    global_layer_index: LAYER,
                    object_id: "model".to_string(),
                    region_id: 7,
                    variant_chain: target_chain,
                },
                rect(1.0, 1.0, 9.0, 9.0),
            )],
            path_z_updates: Vec::new(),
        },
    );
    let runner = MixedRunner {
        scripted,
        dispatcher: WasmRuntimeDispatcher::new(wasm_cache::shared_engine()),
    };
    let mut handles = Handles::new();
    install_native(
        &mut handles,
        "Layer::Infill",
        &observer_module,
        observer_entry,
    );
    execute_per_layer(&plan, &board, &runner, &handles)
        .map_err(|error| format!("ordinary driver failed: {error}"))?;
    let calls = observer.calls();
    let last = calls
        .last()
        .ok_or_else(|| "slice-postprocess observer had no native call".to_string())?;
    if last.stage != "Layer::Infill" {
        return Err(format!(
            "slice-postprocess observer was invoked at unexpected stage '{}'",
            last.stage
        ));
    }
    Ok(last.regions.clone())
}

/// AC-R6 negative: a post-process update addressed to the chain-B source must
/// rewrite the chain-B region; the pair-positional apply rewrites the first
/// pair-matching (chain-A) region instead, rewriting identity.
fn leg_r6_slice_update_chain_b_identity() -> Result<(), String> {
    let regions = drive_slice_postprocess_scripted(chain_material(2))?;
    let mut problems = Vec::new();
    let region_a = regions.iter().find(|r| r.chain == chain_material(1));
    let region_b = regions.iter().find(|r| r.chain == chain_material(2));
    match region_a {
        Some(region) => {
            let expected = vec![rect_vertices(0.0, 0.0, 10.0, 10.0)];
            if region.polygons != expected {
                problems.push(format!(
                    "chain-A region must be untouched by a chain-B update; pair-positional apply \
                     rewrote it: actual={:?} expected={expected:?}",
                    region.polygons
                ));
            }
        }
        None => problems.push("chain-A region absent after commit".to_string()),
    }
    match region_b {
        Some(region) => {
            let expected = vec![rect_vertices(1.0, 1.0, 9.0, 9.0)];
            if region.polygons != expected {
                problems.push(format!(
                    "chain-B region must receive the chain-B update: actual={:?} expected={expected:?}",
                    region.polygons
                ));
            }
        }
        None => problems.push("chain-B region absent after commit".to_string()),
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

/// AC-R6 negative: an update keyed to a chain absent from the slice has no
/// full-source region; it must be rejected, never resolved to a pair sibling.
fn leg_r6_slice_update_missing_chain_rejected() -> Result<(), String> {
    let update_id = "com.test.r6.slice-missing.update";
    let plan = execution_plan(vec![compiled_stage(
        "Layer::SlicePostProcess",
        vec![plain_module(update_id)],
    )]);
    let mut board = board_with_slice(&plan, chained_slice_shared_pair());
    build_and_commit_layer_selection_index(&plan, &mut board).expect("selection index");
    let runner = ScriptedRunner::default();
    runner.push(
        update_id,
        LayerStageCommit::SlicePostProcess {
            polygon_updates: vec![(
                RegionKey {
                    global_layer_index: LAYER,
                    object_id: "model".to_string(),
                    region_id: 7,
                    variant_chain: chain_material(3),
                },
                rect(1.0, 1.0, 9.0, 9.0),
            )],
            path_z_updates: Vec::new(),
        },
    );
    match execute_per_layer(&plan, &board, &runner, &Handles::new()) {
        Err(error) if error.to_string().contains("unknown region") => Ok(()),
        Err(error) => Err(format!(
            "missing-chain update must be rejected as an unknown full-source region; got error: \
             {error}"
        )),
        Ok(_) => Err(
            "missing-chain (chain-C) SlicePostProcess update was accepted by the pair-positional \
             apply and rewrote a pair sibling; expected unknown-region rejection"
                .to_string(),
        ),
    }
}

/// AC-R6 native transport negative: the native `Layer::SlicePostProcess`
/// conversion fabricates an empty chain, so a chain-B update addressed through
/// the native producer must not land on the chain-A sibling.
fn leg_r6_native_slice_update_chain_b() -> Result<(), String> {
    let producer = Probe::new();
    let producer_id = "com.test.r6.native-slice.producer";
    let producer_module = producer.module(producer_id, &[], &[("target_tool", 2)]);
    let observer = Probe::new();
    let observer_id = "com.test.r6.native-slice.observer";
    let observer_module = observer.module(observer_id, &[], &[]);
    let plan = execution_plan(vec![
        compiled_stage("Layer::SlicePostProcess", vec![producer_module]),
        compiled_stage("Layer::Infill", vec![observer_module]),
    ]);
    let mut board = board_with_slice(&plan, chained_slice_shared_pair());
    build_and_commit_layer_selection_index(&plan, &mut board).expect("selection index");
    let mut handles = Handles::new();
    install_native(
        &mut handles,
        "Layer::SlicePostProcess",
        &plan.per_layer_stages[0].modules[0],
        slice_postprocess_entry,
    );
    install_native(
        &mut handles,
        "Layer::Infill",
        &plan.per_layer_stages[1].modules[0],
        observer_entry,
    );
    let runner = WasmRuntimeDispatcher::new(wasm_cache::shared_engine());
    execute_per_layer(&plan, &board, &runner, &handles)
        .map_err(|error| format!("native transport driver failed: {error}"))?;
    if producer.calls().is_empty() {
        return Err(
            "native SlicePostProcess producer was not invoked (nonempty input guard)".into(),
        );
    }
    let calls = observer.calls();
    let last = calls
        .last()
        .ok_or_else(|| "native slice-postprocess observer had no call".to_string())?;
    let mut problems = Vec::new();
    let region_a = last.regions.iter().find(|r| r.chain == chain_material(1));
    let region_b = last.regions.iter().find(|r| r.chain == chain_material(2));
    match region_a {
        Some(region) => {
            let expected = vec![rect_vertices(0.0, 0.0, 10.0, 10.0)];
            if region.polygons != expected {
                problems.push(format!(
                    "native chain-B update must not rewrite chain A; conversion dropped the \
                     delivered chain and the pair-positional apply hit the sibling: actual={:?} \
                     expected={expected:?}",
                    region.polygons
                ));
            }
        }
        None => problems.push("chain-A region absent from observer input".to_string()),
    }
    match region_b {
        Some(region) => {
            let expected = vec![rect_vertices(1.0, 1.0, 9.0, 9.0)];
            if region.polygons != expected {
                problems.push(format!(
                    "native chain-B region must receive the chain-B update: actual={:?} \
                     expected={expected:?}",
                    region.polygons
                ));
            }
        }
        None => problems.push("chain-B region absent from observer input".to_string()),
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

#[test]
fn runtime_commit_joins_use_full_source_identity() {
    summarize(
        "runtime_commit_joins_use_full_source_identity",
        vec![
            (
                "r6_lock_sibling_replacement_rejected_ordinary",
                leg_r6_lock_sibling_replacement_rejected(),
            ),
            (
                "r6_lock_compliant_two_variant_accepted_ordinary",
                leg_r6_lock_compliant_two_variant_accepted(),
            ),
            (
                "r6_lock_compliant_two_variant_accepted_capture",
                leg_r6_lock_compliant_two_variant_capture(),
            ),
            (
                "r6_perimeter_reattach_chain_b_own_fields",
                leg_r6_perimeter_reattach_chain_b_own_fields(),
            ),
            (
                "r6_perimeter_reattach_chain_a_positive_twin",
                leg_r6_perimeter_reattach_chain_a_positive_twin(),
            ),
            (
                "r6_slice_update_chain_b_identity",
                leg_r6_slice_update_chain_b_identity(),
            ),
            (
                "r6_slice_update_missing_chain_rejected",
                leg_r6_slice_update_missing_chain_rejected(),
            ),
            (
                "r6_native_slice_update_chain_b",
                leg_r6_native_slice_update_chain_b(),
            ),
        ],
    );
}

// ── R7 legs: conversion / infill-linker / same-layer partition ───────────────

fn drive_partition(
    tag: &str,
    slice: SliceIR,
    donors: Vec<PerimeterRegion>,
) -> Result<Vec<SeenRegion>, String> {
    let observer = Probe::new();
    let observer_id = format!("com.test.r7.partition.observer.{tag}");
    let producer_id = format!("com.test.r7.partition.perimeter.{tag}");
    let observer_module = observer.module(&observer_id, &[], &[]);
    let plan = execution_plan(vec![
        compiled_stage("Layer::Perimeters", vec![plain_module(&producer_id)]),
        compiled_stage("Layer::Infill", vec![observer_module.clone()]),
    ]);
    let mut board = board_with_slice(&plan, slice);
    build_and_commit_layer_selection_index(&plan, &mut board).expect("selection index");
    let scripted = ScriptedRunner::default();
    scripted.push(
        &producer_id,
        LayerStageCommit::Perimeters(perimeter_ir(donors)),
    );
    let runner = MixedRunner {
        scripted,
        dispatcher: WasmRuntimeDispatcher::new(wasm_cache::shared_engine()),
    };
    let mut handles = Handles::new();
    install_native(
        &mut handles,
        "Layer::Infill",
        &observer_module,
        observer_entry,
    );
    execute_per_layer(&plan, &board, &runner, &handles)
        .map_err(|error| format!("partition driver failed: {error}"))?;
    let calls = observer.calls();
    let last = calls
        .last()
        .ok_or_else(|| "partition observer had no native call".to_string())?;
    Ok(last.regions.clone())
}

/// Control: a single variant with its own donor partitions from its donor;
/// proves the partition hook runs before any full-source claim is made.
fn leg_r7_single_variant_control() -> Result<(), String> {
    let slice = SliceIR {
        global_layer_index: LAYER,
        z: 0.3,
        regions: vec![sliced_region(
            "model",
            7,
            chain_material(1),
            rect(0.0, 0.0, 10.0, 10.0),
            rect(9.0, 9.0, 10.0, 10.0),
        )],
        ..Default::default()
    };
    let regions = drive_partition(
        "single",
        slice,
        vec![perimeter_region(
            chain_material(1),
            rect(2.0, 3.0, 8.0, 7.0),
        )],
    )?;
    let region = regions
        .iter()
        .find(|r| r.chain == chain_material(1))
        .ok_or_else(|| "single-variant observer lost its region".to_string())?;
    let expected = vec![rect_vertices(2.0, 3.0, 8.0, 7.0)];
    if region.sparse_infill_area == expected {
        Ok(())
    } else {
        Err(format!(
            "single-variant partition control must derive sparse from its own donor: \
             actual={:?} expected={expected:?}",
            region.sparse_infill_area
        ))
    }
}

/// AC-R7 negative (own masks): two source regions sharing `(object, region)`
/// with distinct chains and distinct own donors must partition from their own
/// donor. The pair-keyed `perimeter_region_index` collapses to one donor, so
/// the chain-A region borrows the chain-B donor.
fn leg_r7_own_masks_keep_variants_separate() -> Result<(), String> {
    let slice = chained_slice_shared_pair();
    let regions = drive_partition(
        "own",
        slice,
        vec![
            perimeter_region(chain_material(1), rect(2.0, 3.0, 8.0, 7.0)),
            perimeter_region(chain_material(2), rect(4.0, 3.0, 6.0, 7.0)),
        ],
    )?;
    let mut problems = Vec::new();
    for (chain, expected) in [
        (chain_material(1), vec![rect_vertices(2.0, 3.0, 8.0, 7.0)]),
        (chain_material(2), vec![rect_vertices(4.0, 3.0, 6.0, 7.0)]),
    ] {
        let Some(region) = regions.iter().find(|r| r.chain == chain) else {
            problems.push(format!("observer lost chain {chain:?}"));
            continue;
        };
        if region.sparse_infill_area != expected {
            problems.push(format!(
                "own-mask chain {chain:?} must partition from its own donor; pair-keyed \
                 perimeter index collapsed donors: actual={:?} expected={expected:?}",
                region.sparse_infill_area
            ));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

/// AC-R7 negative (modifier-parent masks): two modifier children sharing one
/// child identity with distinct chains borrow their parent region's donor;
/// the pair-keyed parent lookup must keep the child variants separate.
fn leg_r7_modifier_parent_masks_keep_variants_separate() -> Result<(), String> {
    let child_id = slicer_ir::modifier_sub_region_id(7, "model", &rect(0.0, 0.0, 10.0, 10.0));
    assert!(
        slicer_ir::is_modifier_namespace_id(child_id),
        "fixture child id must be a modifier sub-region id"
    );
    let slice = SliceIR {
        global_layer_index: LAYER,
        z: 0.3,
        regions: vec![
            sliced_region(
                "model",
                child_id,
                chain_material(1),
                rect(0.0, 0.0, 10.0, 10.0),
                rect(9.0, 9.0, 10.0, 10.0),
            ),
            sliced_region(
                "model",
                child_id,
                chain_material(2),
                rect(0.0, 0.0, 10.0, 10.0),
                rect(8.0, 8.0, 10.0, 10.0),
            ),
        ],
        ..Default::default()
    };
    let regions = drive_partition(
        "modifier",
        slice,
        vec![
            perimeter_region(chain_material(1), rect(2.0, 3.0, 8.0, 7.0)),
            perimeter_region(chain_material(2), rect(4.0, 3.0, 6.0, 7.0)),
        ],
    )?;
    let mut problems = Vec::new();
    for (chain, expected) in [
        (chain_material(1), vec![rect_vertices(2.0, 3.0, 8.0, 7.0)]),
        (chain_material(2), vec![rect_vertices(4.0, 3.0, 6.0, 7.0)]),
    ] {
        let Some(region) = regions.iter().find(|r| r.chain == chain) else {
            problems.push(format!("observer lost modifier chain {chain:?}"));
            continue;
        };
        if region.sparse_infill_area != expected {
            problems.push(format!(
                "modifier-parent chain {chain:?} must borrow the chain-matching parent donor; \
                 pair-keyed parent lookup collapsed donors: actual={:?} expected={expected:?}",
                region.sparse_infill_area
            ));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

/// AC-R7 negative (chain-order permutation): a permuted chain (same members,
/// different order) has no own donor and must not borrow the sibling result;
/// intentional virtual-variant behavior leaves prepass masks untouched.
fn leg_r7_permuted_chain_cannot_borrow_sibling() -> Result<(), String> {
    let slice = SliceIR {
        global_layer_index: LAYER,
        z: 0.3,
        regions: vec![sliced_region(
            "model",
            7,
            chain_permuted(),
            rect(0.0, 0.0, 10.0, 10.0),
            rect(0.0, 0.0, 5.0, 5.0),
        )],
        ..Default::default()
    };
    let regions = drive_partition(
        "permuted",
        slice,
        vec![perimeter_region(
            chain_material(1),
            rect(2.0, 2.0, 8.0, 8.0),
        )],
    )?;
    let region = regions
        .iter()
        .find(|r| r.chain == chain_permuted())
        .ok_or_else(|| "permuted-chain observer lost its region".to_string())?;
    let untouched = vec![rect_vertices(0.0, 0.0, 5.0, 5.0)];
    let borrowed = vec![rect_vertices(2.0, 2.0, 8.0, 8.0)];
    if region.sparse_infill_area == untouched {
        Ok(())
    } else if region.sparse_infill_area == borrowed {
        Err(
            "chain-order permutation borrowed the sibling donor result; a permuted chain must \
             leave the intentional virtual-variant prepass masks unchanged"
                .to_string(),
        )
    } else {
        Err(format!(
            "unexpected permuted-chain partition: actual={:?} untouched={untouched:?}",
            region.sparse_infill_area
        ))
    }
}

/// AC-R7 real guest leg: production infill-linker at `Layer::InfillPostProcess`
/// over two chained source variants. Stale artifacts fail here with the typed
/// instantiation error (DEFERRED, never skipped).
fn leg_r7_infill_linker_guest() -> Result<(), String> {
    let chain_a = vec![
        ("material".to_string(), PaintValue::ToolIndex(1)),
        ("fuzzy_skin".to_string(), PaintValue::Flag(true)),
    ];
    let chain_b = chain_permuted();
    // Same source number, different ORDERED chains: dropping or sorting the
    // chain must turn the no-match control into an observable (wrong) join.
    let source_id = paint_variant_region_id(7, &chain_a);
    let slice = SliceIR {
        global_layer_index: LAYER,
        z: 0.2,
        regions: vec![
            sliced_region(
                "lightning-object",
                source_id,
                chain_a.clone(),
                rect(0.0, 0.0, 10.0, 10.0),
                rect(0.0, 0.0, 10.0, 10.0),
            ),
            sliced_region(
                "lightning-object",
                source_id,
                chain_material(2),
                rect(0.0, 0.0, 10.0, 10.0),
                rect(0.0, 0.0, 10.0, 10.0),
            ),
        ],
        ..Default::default()
    };
    let mut blackboard = Blackboard::new(Arc::new(MeshIR::default()), SLICE_INDEX + 1);
    blackboard
        .commit_region_map(Arc::new(region_map_for(&slice, Some("lightning-infill"))))
        .expect("region map");
    let mut arena = LayerArena::new();
    arena.set_slice(slice).expect("slice");
    let rails = |y: f32| {
        [y, y + 1.0]
            .into_iter()
            .map(|y| {
                ext_path(
                    [(0.0, y), (10.0, y)],
                    None,
                    0.2,
                    ExtrusionRole::SparseInfill,
                )
            })
            .collect::<Vec<_>>()
    };
    let matched_paths = rails(2.0);
    let sibling_paths = rails(5.0);
    let unkeyed_paths = rails(7.0);
    let prior = |object: &str, paint, paths| InfillRegion {
        object_id: object.into(),
        region_id: source_id,
        variant_chain: paint,
        sparse_infill: paths,
        ..Default::default()
    };
    arena
        .set_infill(InfillIR {
            schema_version: CURRENT_INFILL_IR_SCHEMA_VERSION,
            global_layer_index: LAYER,
            regions: vec![
                prior("lightning-object", chain_a.clone(), matched_paths.clone()),
                // Deliberately lacks an exact delivered view. Its pair sibling
                // DOES have a linkable boundary and must not lend that boundary.
                prior("lightning-object", chain_b.clone(), sibling_paths.clone()),
                prior("unkeyed-prior", chain_b.clone(), unkeyed_paths.clone()),
            ],
            ..Default::default()
        })
        .expect("prior infill");
    let layer = GlobalLayer {
        index: LAYER,
        z: 0.2,
        ..Default::default()
    };
    let bundle = guest_bundle(
        "com.core.infill-linker",
        "Layer::InfillPostProcess",
        "infill-linker",
    );
    let dispatcher = WasmRuntimeDispatcher::new(wasm_cache::shared_engine());
    run_layer_and_commit_with_bundle(
        &dispatcher,
        "Layer::InfillPostProcess",
        &layer,
        &bundle,
        &blackboard,
        &mut arena,
    )
    .map_err(|error| format!("AC-R7 production infill-linker guest dispatch failed: {error}"))?;
    let infill = arena
        .infill()
        .ok_or_else(|| "linker committed no InfillIR".to_string())?;
    assert_eq!(
        infill.regions.len(),
        3,
        "AC-R7 matched, sibling and unkeyed origins all remain distinct"
    );
    let matched = infill
        .regions
        .iter()
        .find(|r| {
            r.object_id == "lightning-object"
                && r.region_id == source_id
                && r.variant_chain == chain_a
        })
        .ok_or_else(|| "AC-R7 matched full-source origin was lost".to_string())?;
    assert!(matched.sparse_infill.iter().any(|path| {
        path.points.iter().any(|p| (p.y - 2.0).abs() < 0.001)
            && path.points.iter().any(|p| (p.y - 3.0).abs() < 0.001)
    }), "AC-R7 exact full-source match must actually JOIN the two rails; unchanged no-match pass-through cannot pass");
    assert_ne!(
        matched.sparse_infill, matched_paths,
        "AC-R7 production guest must change matched raw segments into linked output"
    );
    for (object, paint, expected) in [
        ("lightning-object", chain_b.clone(), sibling_paths),
        ("unkeyed-prior", chain_b, unkeyed_paths),
    ] {
        let region = infill
            .regions
            .iter()
            .find(|r| r.object_id == object && r.region_id == source_id && r.variant_chain == paint)
            .ok_or_else(|| format!("AC-R7 missing negative-control origin {object}/{paint:?}"))?;
        assert_eq!(region.sparse_infill, expected, "AC-R7 sibling or genuinely unkeyed prior must remain UNLINKED, never borrow a delivered view");
    }
    Ok(())
}

#[test]
fn guest_and_partition_joins_preserve_source_identity() {
    summarize(
        "guest_and_partition_joins_preserve_source_identity",
        vec![
            ("r7_single_variant_control", leg_r7_single_variant_control()),
            (
                "r7_own_masks_keep_variants_separate",
                leg_r7_own_masks_keep_variants_separate(),
            ),
            (
                "r7_modifier_parent_masks_keep_variants_separate",
                leg_r7_modifier_parent_masks_keep_variants_separate(),
            ),
            (
                "r7_permuted_chain_cannot_borrow_sibling",
                leg_r7_permuted_chain_cannot_borrow_sibling(),
            ),
            ("r7_infill_linker_guest", leg_r7_infill_linker_guest()),
        ],
    );
}

// ── R8 legs: support chain through planners/renderers/output ─────────────────

fn support_plan_entry(
    chain: Vec<(String, PaintValue)>,
    demand: &str,
    body: &str,
) -> SupportPlanEntry {
    support_plan_entry_with_family(chain, demand, body, "tree")
}

/// AC-R8 fixture: the producer provenance/eligibility channels are explicit;
/// `family` selects which real support module's claims the carrier binds to
/// (`tree` for the native control leg, `traditional` for the real renderer
/// guest whose manifest holds `support-family:traditional`).
fn support_plan_entry_with_family(
    chain: Vec<(String, PaintValue)>,
    demand: &str,
    body: &str,
    family: &str,
) -> SupportPlanEntry {
    // exhaustive: producer provenance and eligibility channels are explicit for carrier controls.
    SupportPlanEntry {
        global_layer_index: LAYER as i32,
        object_id: "carrier-obj".to_string(),
        region_id: 13,
        family_id: family.to_string(),
        demand_ids: vec![demand.to_string()],
        body_ids: vec![body.to_string()],
        anchor_layer_index: LAYER,
        anchor_z: 3000,
        roles: vec![SupportPlanRoleRegion {
            role: SupportPlanRole::SupportBody,
            regions: rect(2.0, 3.0, 8.0, 7.0),
        }],
        skeleton: None,
        capabilities: vec!["extrusion.paths".to_string()],
        provenance: vec!["source-identity-repair-tdd".to_string()],
        decline_reason: None,
        variant_chain: chain,
    }
}

fn drive_support_carrier() -> Result<(), String> {
    let chain_a = chain_material(1);
    let support = SupportPlanIR {
        entries: vec![support_plan_entry(
            chain_a.clone(),
            "demand-13a",
            "body-13a",
        )],
        ..Default::default()
    };
    let probe = Probe::new();
    let probe_id = "com.test.r8.support-carrier";
    let module = probe.module(probe_id, &["support-family:tree"], &[]);
    let plan = execution_plan(vec![compiled_stage("Layer::Support", vec![module.clone()])]);
    let mut board = board_with_slice(&plan, empty_slice());
    if let Err(error) = board.commit_support_plan(Arc::new(support.clone())) {
        return Err(format!(
            "support plan commit rejected the nonempty-chain absent-source carrier: {error:?}"
        ));
    }
    build_and_commit_layer_selection_index(&plan, &mut board).expect("selection index");
    let index = Arc::clone(board.layer_selection_index().expect("index"));
    let selection = index
        .selection_for(LAYER, probe_id)
        .expect("bound support selection");

    let mut problems = Vec::new();
    if selection.targets.len() != 1 {
        problems.push(format!(
            "the nonempty-chain absent source must keep exactly one carrier target; got {}",
            selection.targets.len()
        ));
    }
    {
        let (entry_index, chain, demand, body) =
            (0usize, chain_a.clone(), "demand-13a", "body-13a");
        let expected = LayerTargetSource::SupportCarrier {
            entry_index,
            object_id: "carrier-obj".to_string(),
            region_id: 13,
            variant_chain: chain.clone(),
            family: "tree".to_string(),
            demand_ids: vec![demand.to_string()],
            body_ids: vec![body.to_string()],
            provenance: vec!["source-identity-repair-tdd".to_string()],
            anchor_layer_index: LAYER,
        };
        if !selection.targets.iter().any(|t| t.source == expected) {
            problems.push(format!(
                "carrier target for {chain:?} must preserve the exact plan chain; targets={:?}",
                selection.targets
            ));
        }
    }

    let mut handles = Handles::new();
    install_native(&mut handles, "Layer::Support", &module, support_emit_entry);
    let runner = WasmRuntimeDispatcher::new(wasm_cache::shared_engine());
    execute_per_layer(&plan, &board, &runner, &handles)
        .map_err(|error| format!("support native ordinary driver failed: {error}"))?;
    let captured = execute_captured_stages_with_support_tools(
        &plan,
        &board,
        &runner,
        &handles,
        SupportToolSelection::default(),
        &CaptureRequest {
            stage_ids: vec!["Layer::Support".into()],
            layer_indices: vec![LAYER],
        },
    )
    .map_err(|error| format!("support native capture driver failed: {error:?}"))?;

    let calls = probe.calls();
    if calls.is_empty() {
        problems.push("native support transport saw no call (nonempty input guard)".to_string());
    }
    for call in &calls {
        for chain in [chain_a.clone()] {
            let Some(region) = call
                .regions
                .iter()
                .find(|r| r.object_id == "carrier-obj" && r.chain == chain)
            else {
                problems.push(format!(
                    "native support transport lost carrier {chain:?}; delivered={:?}",
                    call.regions
                ));
                continue;
            };
            if region.region_id != 13 {
                problems.push(format!(
                    "carrier {chain:?} source ID must survive transport; got {}",
                    region.region_id
                ));
            }
        }
    }
    let mut support_captures = 0usize;
    for capture in &captured.captures {
        let CapturedIr::Support(ir) = &capture.ir else {
            continue;
        };
        support_captures += 1;
        for (chain, demand, body) in [(chain_a.clone(), "demand-13a", "body-13a")] {
            let Some(entry) = ir.entries.iter().find(|entry| entry.variant_chain == chain) else {
                problems.push(format!(
                    "committed SupportEntry lost chain {chain:?}; entries={:?}",
                    ir.entries
                ));
                continue;
            };
            if entry.object_id != "carrier-obj" || entry.region_id != 13 {
                problems.push(format!(
                    "committed SupportEntry {chain:?} identity changed: object='{}' region={}",
                    entry.object_id, entry.region_id
                ));
            }
            if entry.family_id != "tree" || entry.demand_ids != vec![demand.to_string()] {
                problems.push(format!(
                    "committed SupportEntry {chain:?} lost family/demand provenance: \
                     family='{}' demands={:?}",
                    entry.family_id, entry.demand_ids
                ));
            }
            if entry.body_id != body {
                problems.push(format!(
                    "committed SupportEntry {chain:?} body changed: '{}'",
                    entry.body_id
                ));
            }
            if entry.paths.is_empty() {
                problems.push(format!("committed SupportEntry {chain:?} has no paths"));
            }
        }
    }
    if support_captures == 0 {
        problems.push("capture produced no Support tap".to_string());
    }

    // Renderer/query negative controls: chain-mismatched and chainless sibling
    // lookups must not resolve to a full-source plan entry.
    let view = PaintRegionLayerView::new(LAYER).with_support_plan(Arc::new(support));
    if !view
        .support_plan_entries_for("carrier-obj", 13, &chain_material(3))
        .is_empty()
    {
        problems.push("missing-chain sibling query resolved a plan entry".to_string());
    }
    if !view
        .support_plan_entries_for("carrier-obj", 13, &[])
        .is_empty()
    {
        problems.push("chainless sibling query resolved a plan entry".to_string());
    }
    if !view
        .support_plan_entries_for("carrier-obj", 13, &chain_permuted())
        .is_empty()
    {
        problems.push("permuted-chain query borrowed a sibling plan entry".to_string());
    }
    if view
        .support_plan_entries_for("carrier-obj", 13, &chain_material(1))
        .is_empty()
    {
        problems.push("exact full-source plan query lost its own entry".to_string());
    }

    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

/// AC-R8 negative: two plan sources sharing `(layer, object, region)` but
/// differing only by ordered chain must commit as distinct carriers. The
/// blackboard's pair-keyed duplicate check rejects them (`DuplicateSupportPlanEntry`),
/// collapsing same-number sources that differ only by chain.
fn leg_r8_chain_distinct_plan_entries() -> Result<(), String> {
    let support = SupportPlanIR {
        entries: vec![
            support_plan_entry(chain_material(1), "demand-13a", "body-13a"),
            support_plan_entry(chain_material(2), "demand-13b", "body-13b"),
        ],
        ..Default::default()
    };
    let probe = Probe::new();
    let module = probe.module("com.test.r8.chain-distinct", &["support-family:tree"], &[]);
    let plan = execution_plan(vec![compiled_stage("Layer::Support", vec![module])]);
    let mut board = board_with_slice(&plan, empty_slice());
    if let Err(error) = board.commit_support_plan(Arc::new(support)) {
        return Err(format!(
            "full-origin plan dedup collapsed chain-distinct same-number sources; the commit \
             boundary rejected two entries differing only by ordered chain: {error:?}"
        ));
    }
    build_and_commit_layer_selection_index(&plan, &mut board).expect("selection index");
    let index = Arc::clone(board.layer_selection_index().expect("index"));
    let selection = index
        .selection_for(LAYER, "com.test.r8.chain-distinct")
        .expect("bound selection");
    if selection.targets.len() != 2 {
        return Err(format!(
            "chain-distinct same-number sources must keep two carrier targets; got {}",
            selection.targets.len()
        ));
    }
    Ok(())
}

/// AC-R8 fixture bundle for the REAL `com.core.traditional-support` renderer:
/// mirrors its manifest (`modules/core-modules/traditional-support/traditional-support.toml`)
/// — stage `Layer::Support`, claims `support-generator`/`support-family:traditional`,
/// IR reads SliceIR/SurfaceClassificationIR/PaintRegionIR/SupportPlanIR, writes
/// SupportIR, schema window 1.0.0–5.0.0. Config keys a guest may read resolve
/// from the map; undeclared/unset keys fall back to the manifest defaults.
fn guest_bundle_traditional_support() -> TestModuleBundle {
    let wasm_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("modules/core-modules")
        .join("traditional-support")
        .join("traditional-support.wasm");
    assert!(
        wasm_path.exists(),
        "traditional-support.wasm not found at {}; build it with `cargo xtask build-guests`",
        wasm_path.display()
    );
    let claims = vec![
        "support-generator".to_string(),
        "support-family:traditional".to_string(),
    ];
    let loaded = LoadedModuleBuilder::new(
        "com.core.traditional-support",
        fixture_semver(0, 1, 0),
        "Layer::Support",
        slicer_schema::TIER_LAYER,
        wasm_path.clone(),
    )
    .ir_reads(vec![
        "SliceIR".to_string(),
        "SurfaceClassificationIR".to_string(),
        "PaintRegionIR".to_string(),
        "SupportPlanIR".to_string(),
    ])
    .ir_writes(vec!["SupportIR".to_string()])
    .claims(claims.clone())
    .min_host_version(fixture_semver(0, 1, 0))
    .min_ir_schema(fixture_semver(1, 0, 0))
    .max_ir_schema(fixture_semver(5, 0, 0))
    .layer_parallel_safe(true)
    .build();
    let pool = Arc::new(
        build_wasm_instance_pool(
            loaded.id(),
            loaded.stage(),
            loaded.layer_parallel_safe(),
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("instance pool"),
    );
    let config = ConfigView::from_map(HashMap::from([
        // Full manifest-derived fixture view: the guest rejects missing keys of
        // these types at the TypedExportCall config check (enable_support first).
        ("enable_support".to_string(), ConfigValue::Bool(true)),
        (
            "support_base_pattern_spacing".to_string(),
            ConfigValue::Float(2.5),
        ),
        ("support_angle".to_string(), ConfigValue::Float(60.0)),
        ("support_speed".to_string(), ConfigValue::Float(50.0)),
        ("line_width".to_string(), ConfigValue::Float(0.4)),
        (
            "support_interface_spacing".to_string(),
            ConfigValue::Float(0.4),
        ),
        // The manifest types this key as `percent`; the guest's typed check
        // rejects FloatOrPercent for it (observed at the TypedExportCall check).
        (
            "support_interface_flow".to_string(),
            ConfigValue::Percent(100.0),
        ),
        (
            "support_bottom_interface_spacing".to_string(),
            ConfigValue::Float(0.5),
        ),
        (
            "support_style".to_string(),
            ConfigValue::String("default".into()),
        ),
        ("nozzle_diameter".to_string(), ConfigValue::Float(0.4)),
        ("layer_height".to_string(), ConfigValue::Float(0.2)),
        (
            "support_line_width".to_string(),
            ConfigValue::FloatOrPercent {
                value: 0.0,
                is_percent: false,
            },
        ),
    ]));
    let module = CompiledModuleBuilder::new(loaded.id().to_string())
        .config_view(Arc::new(config))
        .claims(claims)
        .build();
    TestModuleBundle {
        module,
        pool,
        component: Some(wasm_cache::compiled_component_at(&wasm_path)),
    }
}

/// Load the actual manifest, resolve supplied typed values and bind declared keys.
/// `resolve_config` does not seed absent manifest defaults; callers supply them.
/// Used by the tree renderer and the two production prepass planners.
fn guest_bundle_r8(stem: &str, overrides: HashMap<String, ConfigValue>) -> TestModuleBundle {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../modules/core-modules")
        .join(stem);
    let wasm_path = dir.join(format!("{stem}.wasm"));
    assert!(
        wasm_path.exists(),
        "missing real guest: {}",
        wasm_path.display()
    );
    let loaded = slicer_runtime::manifest::load_module_from_paths(
        &dir.join(format!("{stem}.toml")),
        &wasm_path,
    )
    .expect("real support manifest");
    let bounds = slicer_runtime::ConfigBoundsIndex::from_modules([&loaded]);
    let resolved = slicer_runtime::config_resolution::resolve_config(
        &overrides,
        &bounds,
        &slicer_config::ResolutionTarget::default(),
        &slicer_config::ExpansionContext {
            nozzle_diameter_mm: 0.4,
            ..Default::default()
        },
    )
    .expect("manifest-derived support config");
    let module = CompiledModuleBuilder::new(loaded.id())
        .config_view(slicer_runtime::bind_module_config_view(&loaded, &resolved))
        .claims(loaded.claims().to_vec())
        .build();
    TestModuleBundle {
        pool: pool(loaded.stage(), loaded.id()),
        component: Some(wasm_cache::compiled_component_at(&wasm_path)),
        module,
    }
}

/// Production renderer transport roundtrip. The plan may be a renderer-only
/// control or actual planner output; neither changes the dispatch/commit path.
fn drive_r8_renderer(
    bundle: &TestModuleBundle,
    support: &SupportPlanIR,
    native_entry: NativeStageEntry,
    native: bool,
) -> Result<slicer_ir::SupportIR, String> {
    let transport = if native { "native" } else { "wasm" };
    let plan = execution_plan(vec![compiled_stage(
        "Layer::Support",
        vec![bundle.module.clone()],
    )]);
    let mut board = board_with_slice(&plan, empty_slice());
    board
        .commit_support_plan(Arc::new(support.clone()))
        .map_err(|error| format!("production renderer plan commit: {error:?}"))?;
    build_and_commit_layer_selection_index(&plan, &mut board).expect("selection index");
    let index = Arc::clone(board.layer_selection_index().expect("index"));
    let selection = index
        .selection_for(LAYER, bundle.module.module_id())
        .ok_or_else(|| "production renderer leg bound no selection".to_string())?;
    let expected = support
        .entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| {
            entry.global_layer_index == LAYER as i32
                && bundle
                    .module
                    .claims()
                    .contains(&format!("support-family:{}", entry.family_id))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        expected.len(),
        2,
        "AC-R8 two same-number chained renderer inputs"
    );
    assert_eq!(
        selection.targets.len(),
        2,
        "AC-R8 carriers must not collapse"
    );
    assert!(
        selection.slice_regions.is_empty(),
        "AC-R8 both sources absent from slice"
    );
    for (entry_index, entry) in &expected {
        let expected_target = LayerTargetSource::SupportCarrier {
            entry_index: *entry_index,
            object_id: entry.object_id.clone(),
            region_id: entry.region_id,
            variant_chain: entry.variant_chain.clone(),
            family: entry.family_id.clone(),
            demand_ids: entry.demand_ids.clone(),
            body_ids: entry.body_ids.clone(),
            provenance: entry.provenance.clone(),
            anchor_layer_index: entry.anchor_layer_index,
        };
        let target = selection
            .targets
            .iter()
            .find(|t| t.source == expected_target)
            .expect("AC-R8 exact absent-source carrier chain and opaque provenance");
        assert!(
            Arc::ptr_eq(&target.config, &selection.invocation_config),
            "AC-R8 carrier config must remain invocation-authoritative"
        );
        assert!(
            !target.authored_coloring,
            "AC-R8 carriers acquire no coloring grant"
        );
    }

    // Native uses the actual module's macro-generated SDK entry; WASM has no
    // native entry installed and must execute the real guest component.
    let handles = HashMap::from([(
        bundle.module.module_id().clone(),
        (
            Arc::clone(&bundle.pool),
            bundle.component.clone(),
            native.then_some(native_entry),
        ),
    )]);
    let runner = WasmRuntimeDispatcher::new(wasm_cache::shared_engine());
    let captures = execute_captured_stages_with_support_tools(
        &plan,
        &board,
        &runner,
        &handles,
        SupportToolSelection::default(),
        &CaptureRequest {
            stage_ids: vec!["Layer::Support".into()],
            layer_indices: vec![LAYER],
        },
    )
    .map_err(|error| {
        format!(
            "AC-R8 {transport} {} renderer failed: {error}",
            bundle.module.module_id()
        )
    })?;
    let ir = captures
        .captures
        .iter()
        .find_map(|capture| match &capture.ir {
            CapturedIr::Support(ir) => Some(ir),
            _ => None,
        })
        .ok_or_else(|| format!("AC-R8 {transport} renderer committed no SupportIR"))?;
    assert!(
        !ir.entries.is_empty(),
        "AC-R8 renderer must actually render"
    );
    for (_, source) in &expected {
        let entries = ir
            .entries
            .iter()
            .filter(|e| e.variant_chain == source.variant_chain)
            .collect::<Vec<_>>();
        assert!(
            !entries.is_empty(),
            "AC-R8 {transport} missing own chained output: {:?}",
            source.variant_chain
        );
        for entry in entries {
            // test-quality: planner -> SDK/WIT query -> SupportOutputBuilder -> committed SupportIR roundtrip, not a geometry oracle.
            assert_eq!(
                (
                    &entry.object_id,
                    entry.region_id,
                    &entry.family_id,
                    &entry.demand_ids
                ),
                (
                    &source.object_id,
                    source.region_id,
                    &source.family_id,
                    &source.demand_ids
                ),
                "AC-R8 {transport} exact source/family/demand provenance"
            );
            assert!(
                source.body_ids.contains(&entry.body_id),
                "AC-R8 opaque body ID changed"
            );
            assert!(
                !entry.paths.is_empty(),
                "AC-R8 nonempty paths for BOTH sources"
            );
            assert!(
                entry.paths.iter().all(|path| path.points.len() >= 2),
                "AC-R8 printable paths"
            );
        }
    }
    assert!(
        ir.entries.iter().all(|entry| expected
            .iter()
            .any(|(_, source)| source.variant_chain == entry.variant_chain)),
        "AC-R8 no chainless, reordered or foreign-family sibling output"
    );
    Ok(ir.clone())
}

/// AC-R8 renderer-only controls: BOTH real renderers, BOTH transports, both
/// plan orders. Planner generation is not inferred from these hand-built input
/// plans; `leg_r8_production_planner_output` separately drives real planners.
fn leg_r8_production_renderer_output() -> Result<(), String> {
    for (family, bundle, entry) in [
        (
            "traditional",
            guest_bundle_traditional_support(),
            traditional_support::TraditionalSupport::__slicer_native_entry(),
        ),
        (
            "tree",
            // Manifest-derived defaults in the view: `resolve_config` with
            // empty overrides does NOT materialize schema defaults, and the
            // tree guest early-returns (rendering nothing, committing no
            // SupportIR) when `enable_support` is absent — observed as
            // TapSourceUnavailable on the first real-guest execution.
            guest_bundle_r8(
                "tree-support",
                HashMap::from([
                    ("enable_support".to_string(), ConfigValue::Bool(true)),
                    ("tree_support_wall_count".to_string(), ConfigValue::Int(1)),
                    (
                        "support_base_pattern_spacing".to_string(),
                        ConfigValue::Float(2.5),
                    ),
                    ("support_angle".to_string(), ConfigValue::Float(60.0)),
                    ("support_speed".to_string(), ConfigValue::Float(50.0)),
                    ("line_width".to_string(), ConfigValue::Float(0.4)),
                    (
                        "support_interface_spacing".to_string(),
                        ConfigValue::Float(0.4),
                    ),
                    (
                        "support_interface_flow".to_string(),
                        ConfigValue::Percent(100.0),
                    ),
                    (
                        "support_bottom_interface_spacing".to_string(),
                        ConfigValue::Float(0.5),
                    ),
                    ("nozzle_diameter".to_string(), ConfigValue::Float(0.4)),
                    ("layer_height".to_string(), ConfigValue::Float(0.2)),
                    (
                        "support_line_width".to_string(),
                        ConfigValue::FloatOrPercent {
                            value: 0.0,
                            is_percent: false,
                        },
                    ),
                ]),
            ),
            tree_support::TreeSupport::__slicer_native_entry(),
        ),
    ] {
        for reverse in [false, true] {
            let mut support = SupportPlanIR {
                entries: vec![
                    support_plan_entry_with_family(
                        chain_material(1),
                        "demand-13a",
                        "body-13a",
                        family,
                    ),
                    support_plan_entry_with_family(
                        chain_material(2),
                        "demand-13b",
                        "body-13b",
                        family,
                    ),
                ],
                ..Default::default()
            };
            support.entries[1].roles[0].regions = rect(22.0, 3.0, 28.0, 7.0);
            if reverse {
                support.entries.reverse();
            }
            for native in [true, false] {
                let ir = drive_r8_renderer(&bundle, &support, entry, native)?;
                assert_eq!(ir.entries.len(), 2, "AC-R8 exactly two body-role outputs");
                // f32→unit rounding at region boundaries loses one 100 nm unit
                // (observed x=1.9999 mm vs min 2.0 mm, i.e. 19999 vs 20000
                // units). A 1e-3 mm tolerance stays two orders below the
                // fixture's smallest meaningful spacing (0.4 mm line width) and
                // 14 mm away from the sibling rect, so the no-borrow control
                // keeps its discriminating power.
                const MM_EPS: f32 = 1e-3;
                for (chain, demand, body, min_x, max_x) in [
                    (chain_material(1), "demand-13a", "body-13a", 2.0, 8.0),
                    (chain_material(2), "demand-13b", "body-13b", 22.0, 28.0),
                ] {
                    let output = ir
                        .entries
                        .iter()
                        .find(|e| e.variant_chain == chain)
                        .expect("exact chain");
                    assert_eq!(output.demand_ids, [demand]);
                    assert_eq!(output.body_id, body);
                    assert!(output.paths.iter().flat_map(|p| &p.points).all(|p| (min_x - MM_EPS..=max_x + MM_EPS).contains(&p.x)),
                        "AC-R8 {family} renderer must not borrow sibling geometry, in either plan order: chain={chain:?}, outside={:?}",
                        output.paths.iter().flat_map(|p| &p.points).filter(|p| !(min_x..=max_x).contains(&p.x)).collect::<Vec<_>>());
                }
            }
        }
    }
    Ok(())
}

/// Materialize the real manifest defaults before the production config resolver.
/// The bottom-interface automatic sentinel is expanded to the default top count,
/// as a complete already-expanded fixture view must be delivered to the planner.
fn r8_planner_bundle(stem: &str) -> TestModuleBundle {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../modules/core-modules")
        .join(stem);
    let loaded = slicer_runtime::manifest::load_module_from_paths(
        &dir.join(format!("{stem}.toml")),
        &dir.join(format!("{stem}.wasm")),
    )
    .expect("real planner manifest");
    let mut values: HashMap<_, _> =
        loaded
            .config_schema()
            .entries
            .iter()
            .map(|(key, field)| {
                let raw = field.default.as_deref().expect("planner default");
                let value = field.parsed_default.clone().unwrap_or_else(|| {
                    match field.field_type.as_str() {
                        "bool" => ConfigValue::Bool(raw.parse().expect("bool default")),
                        "int" => ConfigValue::Int(raw.parse().expect("int default")),
                        "float" => ConfigValue::Float(raw.parse().expect("float default")),
                        "string" | "enum" => ConfigValue::String(raw.to_string()),
                        other => panic!("unhandled planner default type: {key} / {other}"),
                    }
                });
                (key.clone(), value)
            })
            .collect();
    // The manifest declares both spellings for compatibility, but production
    // config application rejects supplying the canonical key and its alias.
    values.remove("support_overhang_angle");
    values.insert(
        "support_interface_bottom_layers".into(),
        values["support_interface_top_layers"].clone(),
    );
    guest_bundle_r8(stem, values)
}

/// Four same-number origins: each family's pair differs ONLY in chain order.
/// A fifth, genuinely chain-less candidate has no ownership grant and must not
/// borrow either family's assignment or produce a plan entry.
fn r8_planner_sources() -> Vec<(Vec<(String, PaintValue)>, Option<&'static str>)> {
    let mut traditional = chain_permuted();
    traditional.reverse();
    let tree = vec![
        ("fuzzy_skin".to_string(), PaintValue::Flag(true)),
        ("material".to_string(), PaintValue::ToolIndex(2)),
    ];
    let mut tree_reversed = tree.clone();
    tree_reversed.reverse();
    vec![
        (chain_permuted(), Some("traditional")),
        (traditional, Some("traditional")),
        (tree, Some("tree")),
        (tree_reversed, Some("tree")),
        (Vec::new(), None),
    ]
}

/// Schema-faithful planner input, not a manufactured SupportPlanIR. The closed
/// floating roof starts at Z=1.6; all emitted support planes are below it, so
/// production exact-Z aggregation validates real empty space beneath the roof.
/// SupportGeometryIR contains coarse model outlines, while SupportAnalysisIR
/// candidates carry the contact demands (the traditional planner ignores coarse
/// outlines as contacts, and the tree planner uses them for collision).
fn r8_planner_board(plan: &ExecutionPlan, reverse: bool) -> Blackboard {
    let object_id = "planner-source";
    let mesh = MeshIR {
        objects: vec![ObjectMesh {
            id: object_id.into(),
            mesh: IndexedTriangleSet {
                vertices: [1.6, 1.8]
                    .into_iter()
                    .flat_map(|z| {
                        [(0.0, 0.0), (80.0, 0.0), (80.0, 10.0), (0.0, 10.0)]
                            .into_iter()
                            .map(move |(x, y)| Point3 { x, y, z })
                    })
                    .collect(),
                indices: vec![
                    0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3,
                    7, 2, 7, 6, 3, 0, 4, 3, 4, 7,
                ],
            },
            transform: Transform3d {
                matrix: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
            },
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut board = Blackboard::new(Arc::new(mesh), SLICE_INDEX + 1);
    crate::common::seed::seed_slice_ir(&mut board, plan);
    board
        .commit_surface_classification(Arc::new(SurfaceClassificationIR::default()))
        .expect("planner surface prerequisite");
    board
        .commit_layer_plan(Arc::new(LayerPlanIR {
            global_layers: plan.global_layers.as_ref().clone(),
            object_participation: HashMap::from([(
                object_id.into(),
                (0..=LAYER)
                    .map(|layer| ObjectLayerRef {
                        local_layer_index: layer,
                        global_layer_index: layer,
                        effective_layer_height: 0.2,
                    })
                    .collect(),
            )]),
            ..Default::default()
        }))
        .expect("planner layer prerequisite");
    let mut regions = RegionMapIR::default();
    let mut geometry = SupportGeometryIR {
        support_layer_height_mm: 0.0,
        support_top_z_distance_mm: 0.2,
        ..Default::default()
    };
    let mut analysis = SupportAnalysisIR::default();
    // Both production marshal legs enumerate segmentation from committed
    // SliceIR SOURCE origins, not the config-address rows in RegionMapIR.
    let mut slices = board
        .slice_ir()
        .expect("seeded planner slices")
        .as_ref()
        .clone();
    for (index, (chain, family)) in r8_planner_sources().into_iter().enumerate() {
        let x = 2.0 + index as f32 * 15.0;
        let contact = rect(x, 3.0, x + 6.0, 7.0);
        for layer in 0..=LAYER {
            let polygons = if layer == LAYER {
                contact.clone()
            } else {
                Vec::new()
            };
            slices[layer as usize].regions.push(SlicedRegion {
                object_id: object_id.into(),
                region_id: 13,
                variant_chain: chain.clone(),
                effective_layer_height: 0.2,
                polygons: polygons.clone(),
                ..Default::default()
            });
            regions.entries.insert(
                RegionKey {
                    global_layer_index: layer,
                    object_id: object_id.into(),
                    region_id: map_space_region_id(13, &chain),
                    variant_chain: chain.clone(),
                },
                RegionPlan::default(),
            );
            geometry.entries.insert(
                SupportGeometryKey {
                    global_support_layer_index: layer,
                    object_id: object_id.into(),
                    region_id: 13,
                    variant_chain: chain.clone(),
                },
                polygons,
            );
        }
        if let Some(family) = family {
            analysis.family_assignments.insert(
                SupportSourceKey {
                    object_id: object_id.into(),
                    region_id: 13,
                    variant_chain: chain.clone(),
                },
                family.into(),
            );
        }
        // exhaustive: the planner input pins contact geometry, source, and both policy flags; SupportCandidate has no Default.
        analysis.candidates.push(SupportCandidate {
            id: index as u64 + 1,
            geometry: contact,
            // exhaustive: the candidate source pins the complete ordered origin and physical contact plane.
            source: SupportCandidateSource {
                object_id: object_id.into(),
                region_id: 13,
                global_layer_index: LAYER,
                z_units: slicer_ir::mm_to_units(1.6),
                variant_chain: chain,
            },
            enforced: false,
            blocked: false,
        });
    }
    if reverse {
        analysis.candidates.reverse();
        for slice in &mut slices {
            slice.regions.reverse();
        }
    }
    board
        .replace_slice_ir(Arc::new(slices))
        .expect("planner source segmentation");
    board
        .commit_region_map(Arc::new(regions))
        .expect("planner regions");
    board
        .commit_support_geometry(Arc::new(geometry))
        .expect("planner coarse geometry");
    board
        .commit_support_analysis(Arc::new(analysis))
        .expect("planner contact analysis");
    board
}

/// AC-R8: BOTH real planners in one production prepass stage, on native macro
/// entries and real WASM components. Reversing planner/candidate order cannot
/// collapse same-number, order-distinct sources or lend ownership to chain-less
/// siblings. Plans are observed after production aggregation and blackboard commit.
fn leg_r8_production_planner_output() -> Result<(), String> {
    let planners = [
        (
            "traditional",
            r8_planner_bundle("traditional-support-planner"),
            traditional_support_planner::SupportPlanner::__slicer_native_entry(),
            "traditional-support-planner",
        ),
        (
            "tree",
            r8_planner_bundle("tree-support-planner"),
            tree_support_planner::SupportPlanner::__slicer_native_entry(),
            "support-planner",
        ),
    ];
    for reverse in [false, true] {
        for (leg, native) in [("native", true), ("wasm", false)] {
            let mut modules: Vec<_> = planners
                .iter()
                .map(|(_, b, _, _)| b.module.clone())
                .collect();
            if reverse {
                modules.reverse();
            }
            let plan = ExecutionPlan {
                prepass_stages: vec![compiled_stage("PrePass::SupportGeometry", modules)],
                global_layers: Arc::new(
                    (0..=LAYER)
                        .map(|index| GlobalLayer {
                            index,
                            z: (index + 1) as f32 * 0.2,
                            active_regions: vec![ActiveRegion {
                                object_id: "planner-source".into(),
                                region_id: 13,
                                effective_layer_height: 0.2,
                                ..Default::default()
                            }],
                            ..Default::default()
                        })
                        .collect(),
                ),
                ..Default::default()
            };
            let mut board = r8_planner_board(&plan, reverse);
            let handles: Handles = planners
                .iter()
                .map(|(_, bundle, entry, _)| {
                    (
                        bundle.module.module_id().clone(),
                        (
                            Arc::clone(&bundle.pool),
                            bundle.component.clone(),
                            native.then_some(*entry),
                        ),
                    )
                })
                .collect();
            let runner = WasmRuntimeDispatcher::new(wasm_cache::shared_engine());
            let audits = slicer_runtime::execute_prepass(&plan, &mut board, &runner, &handles)
                .map_err(|error| format!("AC-R8 {leg} planner prepass failed: {error}"))?;
            let output = board
                .support_plan()
                .ok_or_else(|| format!("AC-R8 {leg} no planner commit"))?;
            assert!(
                !output.entries.is_empty(),
                "AC-R8 {leg} planners must produce plans"
            );
            for (family, bundle, _, provenance) in &planners {
                assert!(bundle
                    .module
                    .claims()
                    .contains(&"support-planner".to_string()));
                assert!(bundle
                    .module
                    .claims()
                    .contains(&format!("support-family:{family}")));
                let audit = audits
                    .iter()
                    .find(|a| &a.module_id == bundle.module.module_id())
                    .expect("AC-R8 both real planners invoked with manifest producer attribution");
                assert_eq!(
                    audit.runtime_writes,
                    ["SupportPlanIR"],
                    "AC-R8 {leg} {family} planner wrote a plan"
                );
                assert!(
                    audit.diagnostics.is_empty(),
                    "AC-R8 {leg} {family} rejected/declined demand: {:?}",
                    audit.diagnostics
                );
                for (chain, _) in r8_planner_sources()
                    .into_iter()
                    .filter(|(_, f)| f.as_ref() == Some(family))
                {
                    let own: Vec<_> = output
                        .entries
                        .iter()
                        .filter(|e| e.variant_chain == chain)
                        .collect();
                    assert!(
                        !own.is_empty(),
                        "AC-R8 {leg} {family} lost ordered source {chain:?}"
                    );
                    for entry in own {
                        assert_eq!(
                            (&entry.object_id, entry.region_id, entry.family_id.as_str()),
                            (&"planner-source".to_string(), 13, *family)
                        );
                        assert!(
                            entry.provenance.contains(&provenance.to_string()),
                            "AC-R8 {leg} planner provenance preserved"
                        );
                        assert!(entry.decline_reason.is_none());
                        assert!(
                            !entry.demand_ids.is_empty() && !entry.body_ids.is_empty(),
                            "AC-R8 {leg} planner demand/body provenance"
                        );
                        assert!(
                            entry.roles.iter().any(|role| !role.regions.is_empty()),
                            "AC-R8 {leg} each source has nonempty role geometry"
                        );
                        assert_eq!(
                            output
                                .entries
                                .iter()
                                .filter(|e| e.global_layer_index == entry.global_layer_index
                                    && e.object_id == entry.object_id
                                    && e.region_id == entry.region_id
                                    && e.variant_chain == chain)
                                .count(),
                            1,
                            "AC-R8 {leg} one distinct entry per complete source on each plane"
                        );
                        assert_eq!(
                            output
                                .entries
                                .iter()
                                .filter(|e| e.global_layer_index == entry.global_layer_index
                                    && e.family_id == *family)
                                .count(),
                            2,
                            "AC-R8 {leg} both ordered siblings survive on each emitted plane"
                        );
                    }
                }
            }
            assert!(
                output.entries.iter().all(|e| r8_planner_sources()
                    .iter()
                    .any(|(chain, family)| e.variant_chain == *chain
                        && family.is_some_and(|f| f == e.family_id))),
                "AC-R8 {leg} no chainless/unassigned/reordered sibling borrowed ownership"
            );
            let view = PaintRegionLayerView::new(0).with_support_plan(Arc::clone(output));
            assert!(
                view.support_plan_entries_for("planner-source", 13, &[])
                    .is_empty(),
                "AC-R8 {leg} chainless sibling query must not borrow a plan"
            );
            assert!(
                view.support_plan_entries_for("planner-source", 13, &chain_material(3))
                    .is_empty(),
                "AC-R8 {leg} missing-chain sibling query must not borrow a plan"
            );
        }
    }
    Ok(())
}

/// Planner grounding is at the documented prepass input boundary: synthetic
/// schema-faithful geometry/analysis -> real planners -> committed plans. The
/// renderer controls separately prove absent-source carriers and output. This
/// does not claim a full painted-model candidate-discovery witness: the prior
/// `cube_4color.3mf` probes legitimately produced no unsupported contacts in
/// `commit_support_analysis_builtin` (crates/slicer-runtime/src/builtins/support_analysis_producer.rs).
#[test]
fn support_chain_reaches_planners_renderers_and_output() {
    summarize(
        "support_chain_reaches_planners_renderers_and_output",
        vec![
            ("r8_support_carrier_transport", drive_support_carrier()),
            (
                "r8_chain_distinct_plan_entries",
                leg_r8_chain_distinct_plan_entries(),
            ),
            (
                "r8_production_planner_output",
                leg_r8_production_planner_output(),
            ),
            (
                "r8_production_renderer_output",
                leg_r8_production_renderer_output(),
            ),
        ],
    );
}

// ── R9 legs: lightning and raft full-source roundtrip ────────────────────────

fn drive_raft(tag: &str, extra_absent_chain: i64) -> Result<Vec<SeenRegion>, String> {
    let producer = Probe::new();
    let producer_id = format!("com.test.r9.raft.producer.{tag}");
    let producer_module = producer.module(
        &producer_id,
        &[],
        &[("extra_absent_chain", extra_absent_chain)],
    );
    let observer = Probe::new();
    let observer_id = format!("com.test.r9.raft.observer.{tag}");
    let observer_module = observer.module(&observer_id, &[], &[]);
    let plan = execution_plan(vec![
        compiled_stage("Layer::Infill", vec![producer_module.clone()]),
        compiled_stage("Layer::InfillPostProcess", vec![observer_module.clone()]),
    ]);
    let mut board = board_with_slice(&plan, chained_slice_derived_pair());
    build_and_commit_layer_selection_index(&plan, &mut board).expect("selection index");
    let mut handles = Handles::new();
    install_native(
        &mut handles,
        "Layer::Infill",
        &producer_module,
        raft_producer_entry,
    );
    install_native(
        &mut handles,
        "Layer::InfillPostProcess",
        &observer_module,
        observer_entry,
    );
    let runner = WasmRuntimeDispatcher::new(wasm_cache::shared_engine());
    execute_per_layer(&plan, &board, &runner, &handles)
        .map_err(|error| format!("raft driver failed: {error}"))?;
    if producer.calls().is_empty() {
        return Err("raft producer was not invoked (nonempty input guard)".to_string());
    }
    let calls = observer.calls();
    let last = calls
        .last()
        .ok_or_else(|| "raft observer had no native call".to_string())?;
    Ok(last.regions.clone())
}

/// AC-R9 negative: raft polygons tagged with the chain-B origin must attach to
/// the chain-B slice region; the pair-keyed raft attach lands them on the
/// first pair-matching (chain-A) sibling.
fn leg_r9_raft_attach_full_source() -> Result<(), String> {
    let regions = drive_raft("own", 0)?;
    let mut problems = Vec::new();
    for (chain, expected) in [
        (chain_material(1), vec![rect_vertices(1.0, 1.0, 2.0, 2.0)]),
        (chain_material(2), vec![rect_vertices(3.0, 3.0, 4.0, 4.0)]),
    ] {
        let Some(region) = regions.iter().find(|r| r.chain == chain) else {
            problems.push(format!("raft observer lost chain {chain:?}"));
            continue;
        };
        if region.raft_fill != expected {
            problems.push(format!(
                "raft polygon must attach to its own full-source region {chain:?}; pair-keyed \
                 raft attach collapsed it: actual={:?} expected={expected:?}",
                region.raft_fill
            ));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

/// AC-R9 negative: a raft group whose chain has no slice region must not
/// attach anywhere (no producer map-space normalization, no chainless fallback).
fn leg_r9_absent_chain_raft_not_attached() -> Result<(), String> {
    let regions = drive_raft("absent", 1)?;
    let absent = vec![rect_vertices(5.0, 5.0, 6.0, 6.0)];
    let mut problems = Vec::new();
    for region in &regions {
        if region.raft_fill == absent {
            problems.push(format!(
                "raft group for a chain absent from the slice attached to {region:?}"
            ));
        }
    }
    for (chain, expected) in [
        (chain_material(1), vec![rect_vertices(1.0, 1.0, 2.0, 2.0)]),
        (chain_material(2), vec![rect_vertices(3.0, 3.0, 4.0, 4.0)]),
    ] {
        let Some(region) = regions.iter().find(|r| r.chain == chain) else {
            problems.push(format!("raft observer lost chain {chain:?}"));
            continue;
        };
        if region.raft_fill != expected {
            problems.push(format!(
                "own raft group for {chain:?} must attach once; actual={:?} expected={expected:?}",
                region.raft_fill
            ));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

/// AC-R9 SDK query legs: `LightningTreeEntry` full-source lookup keeps exact
/// ordered chains and rejects chainless / permuted sibling queries.
fn leg_r9_lightning_sdk_queries() -> Result<(), String> {
    let chain_a = chain_material(1);
    let chain_b = chain_material(2);
    let seg_a = [
        Point2 { x: 1_000, y: 5_000 },
        Point2 {
            x: 30_000,
            y: 5_000,
        },
    ];
    let seg_b = [
        Point2 { x: 1_000, y: 6_000 },
        Point2 {
            x: 59_000,
            y: 6_000,
        },
    ];
    // exhaustive: pins every LightningTreeEntry field for the query fixture.
    let tree = LightningTreeIR {
        entries: vec![
            // exhaustive: pins every LightningTreeEntry field for the full-source query fixture.
            LightningTreeEntry {
                object_id: "lightning-object".to_string(),
                global_layer_index: LAYER as i32,
                region_id: paint_variant_region_id(7, &chain_a),
                tree_edge_segments: vec![seg_a],
                variant_chain: chain_a.clone(),
            },
            // exhaustive: pins every LightningTreeEntry field for the full-source query fixture.
            LightningTreeEntry {
                object_id: "lightning-object".to_string(),
                global_layer_index: LAYER as i32,
                region_id: paint_variant_region_id(7, &chain_b),
                tree_edge_segments: vec![seg_b],
                variant_chain: chain_b.clone(),
            },
        ],
        ..Default::default()
    };
    let view = PaintRegionLayerView::new(LAYER).with_lightning_tree_ir(Arc::new(tree));
    let mut problems = Vec::new();
    let query = |chain: &[(String, PaintValue)]| {
        view.lightning_tree_segments_for(
            "lightning-object",
            paint_variant_region_id(7, chain),
            chain,
        )
    };
    if query(&chain_a) != vec![seg_a] {
        problems.push("chain-A lightning query must return its own segments".to_string());
    }
    if query(&chain_b) != vec![seg_b] {
        problems.push("chain-B lightning query must return its own segments".to_string());
    }
    if !query(&chain_permuted()).is_empty() {
        problems.push("permuted-chain lightning query borrowed sibling segments".to_string());
    }
    if !view
        .lightning_tree_segments_for("lightning-object", paint_variant_region_id(7, &[]), &[])
        .is_empty()
    {
        problems.push("chainless lightning query succeeded against a chained entry".to_string());
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

/// AC-R9: real host builtin generation followed by production SDK/native and
/// WIT/WASM infill dispatch. No hand-built tree substitutes for generation.
fn leg_r9_lightning_generator_guest() -> Result<(), String> {
    let chain_a = chain_material(1);
    let chain_b = chain_material(2);
    let source_id = paint_variant_region_id(7, &chain_a);
    let slice = SliceIR {
        global_layer_index: LAYER,
        z: 0.2,
        regions: vec![
            sliced_region(
                "lightning-object",
                source_id,
                chain_a.clone(),
                rect(0.0, 0.0, 12.0, 12.0),
                rect(0.0, 0.0, 12.0, 12.0),
            ),
            sliced_region(
                "lightning-object",
                source_id,
                chain_b.clone(),
                rect(30.0, 0.0, 42.0, 12.0),
                rect(30.0, 0.0, 42.0, 12.0),
            ),
        ],
        ..Default::default()
    };
    let bundle = guest_bundle(
        "com.core.lightning-infill",
        "Layer::Infill",
        "lightning-infill",
    );
    let plan = execution_plan(vec![compiled_stage(
        "Layer::Infill",
        vec![bundle.module.clone()],
    )]);
    let mut board = board_with_slice(&plan, slice.clone());
    // A smaller preceding layer supplies a real internal overhang band. A
    // single-layer fixture has no predecessor and cannot seed lightning trees.
    let mut slices = board.slice_ir().unwrap().as_ref().clone();
    slices[SLICE_INDEX - 1] = SliceIR {
        global_layer_index: LAYER - 1,
        z: 0.1,
        regions: vec![
            sliced_region(
                "lightning-object",
                source_id,
                chain_a.clone(),
                rect(0.0, 0.0, 10.0, 10.0),
                vec![],
            ),
            sliced_region(
                "lightning-object",
                source_id,
                chain_b.clone(),
                rect(30.0, 0.0, 40.0, 10.0),
                vec![],
            ),
        ],
        ..Default::default()
    };
    board
        .replace_slice_ir(Arc::new(slices))
        .expect("generation slices");
    slicer_runtime::commit_lightning_tree_ir_builtin(
        &mut board,
        &ResolvedConfig {
            sparse_fill_holder: "lightning-infill".into(),
            line_width: 0.4,
            infill_density: 0.2,
            layer_height: 0.2,
            ..Default::default()
        },
    )
    .map_err(|error| format!("AC-R9 production LightningTreeGen builtin failed: {error}"))?;
    let tree = Arc::clone(
        board
            .lightning_tree_ir()
            .expect("AC-R9 generator must commit LightningTreeIR"),
    );
    let own = tree
        .entries
        .iter()
        .filter(|entry| entry.global_layer_index == LAYER as i32)
        .collect::<Vec<_>>();
    assert_eq!(
        own.len(),
        2,
        "AC-R9 GENERATOR must keep two same-number source chains distinct"
    );
    let view = PaintRegionLayerView::new(LAYER).with_lightning_tree_ir(Arc::clone(&tree));
    for (paint, min_x, max_x) in [(&chain_a, 0.0, 12.0), (&chain_b, 30.0, 42.0)] {
        let entry = own
            .iter()
            .find(|entry| entry.variant_chain == *paint)
            .expect("AC-R9 GENERATOR must retain the complete chain");
        assert_eq!(
            (&entry.object_id, entry.region_id),
            (&"lightning-object".to_string(), source_id),
            "AC-R9 GENERATOR source ID must not become map-space region 7"
        );
        assert!(
            !entry.tree_edge_segments.is_empty(),
            "AC-R9 GENERATOR must actually generate segments"
        );
        assert!(
            entry
                .tree_edge_segments
                .iter()
                .flatten()
                .all(|p| (min_x..=max_x).contains(&slicer_ir::units_to_mm(p.x))),
            "AC-R9 GENERATOR must not mix sibling geometry"
        );
        // test-quality: generate_lightning_trees -> SDK query transport roundtrip; geometry containment and source IDs above have independent expectations.
        assert_eq!(
            view.lightning_tree_segments_for("lightning-object", source_id, paint),
            entry.tree_edge_segments,
            "AC-R9 SDK query must return the generated own-chain segments"
        );
    }
    assert!(
        view.lightning_tree_segments_for("lightning-object", source_id, &[])
            .is_empty(),
        "AC-R9 same source ID WITHOUT the chain must not resolve generated segments"
    );
    assert!(
        view.lightning_tree_segments_for("lightning-object", 7, &chain_a)
            .is_empty(),
        "AC-R9 map-space-normalized producer/query identity must not resolve source segments"
    );
    build_and_commit_layer_selection_index(&plan, &mut board)
        .expect("generated lightning selection index");
    for (leg, native) in [("native", true), ("wasm", false)] {
        let handles = HashMap::from([(
            bundle.module.module_id().clone(),
            (
                Arc::clone(&bundle.pool),
                bundle.component.clone(),
                native.then(lightning_infill::LightningInfill::__slicer_native_entry),
            ),
        )]);
        let runner = WasmRuntimeDispatcher::new(wasm_cache::shared_engine());
        let captures = execute_captured_stages_with_support_tools(
            &plan,
            &board,
            &runner,
            &handles,
            SupportToolSelection::default(),
            &CaptureRequest {
                stage_ids: vec!["Layer::Infill".into()],
                layer_indices: vec![LAYER],
            },
        )
        .map_err(|error| format!("AC-R9 {leg} generated-tree infill executor failed: {error}"))?;
        let ir = captures
            .captures
            .iter()
            .find_map(|capture| match &capture.ir {
                CapturedIr::Infill(ir) => Some(ir),
                _ => None,
            })
            .ok_or_else(|| format!("AC-R9 {leg} generated-tree infill COMMIT missing"))?;
        assert_eq!(
            ir.regions.len(),
            2,
            "AC-R9 {leg} WIT/SDK query -> infill COMMIT must retain both full keys"
        );
        for (paint, min_x, max_x) in [(&chain_a, 0.0, 12.0), (&chain_b, 30.0, 42.0)] {
            let region = ir
                .regions
                .iter()
                .find(|r| r.variant_chain == *paint)
                .expect("AC-R9 infill COMMIT must preserve generated chain");
            assert_eq!(
                (region.object_id.as_str(), region.region_id),
                ("lightning-object", source_id),
                "AC-R9 {leg} infill COMMIT must retain SOURCE ID, not map-space normalization"
            );
            assert!(
                !region.sparse_infill.is_empty(),
                "AC-R9 {leg} query must actually reach the generated tree"
            );
            assert!(
                region
                    .sparse_infill
                    .iter()
                    .flat_map(|p| &p.points)
                    .all(|p| (min_x..=max_x).contains(&p.x)),
                "AC-R9 {leg} infill COMMIT must not borrow sibling generated segments"
            );
        }
    }
    Ok(())
}

/// Real raft module output, not a native test emitter standing in for WASM.
fn leg_r9_raft_module_both_transports() -> Result<(), String> {
    let source_id = paint_variant_region_id(7, &chain_material(1));
    for (leg, native) in [("native", true), ("wasm", false)] {
        let observer = Probe::new();
        let observer_module = observer.module("com.test.r9.raft-module.observer", &[], &[]);
        let module = CompiledModuleBuilder::new("com.core.raft-default")
            .claims(vec!["claim:raft-fill".into()])
            .config_view(Arc::new(ConfigView::from_map(HashMap::from([
                ("raft_expansion".into(), ConfigValue::Float(0.0)),
                ("raft_first_layer_expansion".into(), ConfigValue::Float(0.0)),
                ("raft_contact_distance".into(), ConfigValue::Float(0.0)),
                ("raft_line_spacing".into(), ConfigValue::Float(5.0)),
            ]))))
            .build();
        let mut plan = execution_plan(vec![
            compiled_stage("Layer::Infill", vec![module.clone()]),
            compiled_stage("Layer::InfillPostProcess", vec![observer_module.clone()]),
        ]);
        Arc::make_mut(&mut plan.global_layers)[0].is_raft = true;
        let mut slice = chained_slice_shared_pair();
        for (region, x) in slice.regions.iter_mut().zip([0.0, 30.0]) {
            region.region_id = source_id;
            region.polygons = rect(x, 0.0, x + 10.0, 10.0);
        }
        let mut board = board_with_slice(&plan, slice);
        board
            .commit_support_plan(Arc::new(SupportPlanIR {
                raft_plan: Some(slicer_ir::RaftPlan {
                    raft_layers: LAYER + 1,
                    base_raft_layers: LAYER,
                    ..Default::default()
                }),
                ..Default::default()
            }))
            .expect("AC-R9 explicit print-wide raft plan");
        build_and_commit_layer_selection_index(&plan, &mut board).expect("raft module selection");
        let mut handles = Handles::from([(
            module.module_id().clone(),
            (
                pool("Layer::Infill", module.module_id()),
                Some(wasm_cache::compiled_component_at(
                    &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("../../modules/core-modules/raft-default/raft-default.wasm"),
                )),
                native.then(raft_default::RaftDefault::__slicer_native_entry),
            ),
        )]);
        install_native(
            &mut handles,
            "Layer::InfillPostProcess",
            &observer_module,
            observer_entry,
        );
        let runner = WasmRuntimeDispatcher::new(wasm_cache::shared_engine());
        let captured = execute_captured_stages_with_support_tools(
            &plan,
            &board,
            &runner,
            &handles,
            SupportToolSelection::default(),
            &CaptureRequest {
                stage_ids: vec!["Layer::Infill".into(), "Layer::InfillPostProcess".into()],
                layer_indices: vec![LAYER],
            },
        )
        .map_err(|error| format!("AC-R9 {leg} production raft module executor failed: {error}"))?;
        let ir = captured
            .captures
            .iter()
            .find_map(|capture| match &capture.ir {
                CapturedIr::Infill(ir) => Some(ir),
                _ => None,
            })
            .ok_or_else(|| format!("AC-R9 {leg} raft COMMIT tap missing"))?;
        assert_eq!(
            ir.raft_regions.len(),
            2,
            "AC-R9 {leg} module must commit two full-key InfillRaftRegion groups"
        );
        let calls = observer.calls();
        let delivered = &calls
            .last()
            .expect("AC-R9 committed raft must reach following production stage")
            .regions;
        for (paint, min_x, max_x) in [
            (chain_material(1), 0.0, 10.0),
            (chain_material(2), 30.0, 40.0),
        ] {
            let raft = ir
                .raft_regions
                .iter()
                .find(|r| r.variant_chain == paint)
                .expect("AC-R9 raft COMMIT must retain exact chain");
            assert_eq!(
                (raft.object_id.as_str(), raft.region_id),
                ("model", source_id),
                "AC-R9 {leg} raft COMMIT must not normalize source IDs"
            );
            let region = delivered
                .iter()
                .find(|r| r.chain == paint)
                .expect("AC-R9 full-key raft attach must retain both slice variants");
            assert!(
                !region.raft_fill.is_empty(),
                "AC-R9 {leg} raft fill must attach to its own full source key"
            );
            assert!(
                region
                    .raft_fill
                    .iter()
                    .flatten()
                    .all(
                        |(x, _)| (slicer_ir::mm_to_units(min_x)..=slicer_ir::mm_to_units(max_x))
                            .contains(x)
                    ),
                "AC-R9 {leg} raft attach must not borrow the same-number sibling's polygons"
            );
        }
    }
    Ok(())
}

#[test]
fn lightning_and_raft_full_source_roundtrip() {
    summarize(
        "lightning_and_raft_full_source_roundtrip",
        vec![
            (
                "r9_raft_attach_full_source",
                leg_r9_raft_attach_full_source(),
            ),
            (
                "r9_absent_chain_raft_not_attached",
                leg_r9_absent_chain_raft_not_attached(),
            ),
            ("r9_lightning_sdk_queries", leg_r9_lightning_sdk_queries()),
            (
                "r9_lightning_generator_guest",
                leg_r9_lightning_generator_guest(),
            ),
            (
                "r9_raft_module_both_transports",
                leg_r9_raft_module_both_transports(),
            ),
        ],
    );
}

// ── summary ──────────────────────────────────────────────────────────────────

fn summarize(driver: &str, legs: Vec<(&'static str, Result<(), String>)>) {
    let mut failures = Vec::new();
    let mut passed = 0usize;
    let total = legs.len();
    for (name, outcome) in &legs {
        match outcome {
            Ok(()) => passed += 1,
            Err(evidence) => failures.push(format!("LEG {name}: {evidence}")),
        }
    }
    assert!(
        failures.is_empty(),
        "{driver}: {passed}/{total} legs satisfied; unsatisfied full-source identity joins:\n{}",
        failures.join("\n"),
    );
}
