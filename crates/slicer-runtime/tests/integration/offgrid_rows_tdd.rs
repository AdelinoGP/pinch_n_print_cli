//! TASK-401 (packet 239a) — RED tests for anchored host seams: off-grid
//! `same-z-support` entities must be lowered into their own synthesized
//! `LayerCollectionIR` row at their declared Z.
//!
//! These are written at PIPELINE level (against the row sequence a full
//! `run_pipeline_with_instrumentation` run hands to the G-code emitter), NOT at
//! executor level. Finding F1 of the plan of record established that the
//! executor's routing partition is already total — `route_of`'s positive
//! filter (in `append_same_z_entities`) and its negated filter (in the
//! anchored-collection path) are exact complements — so an executor-level test
//! would pass vacuously and prove nothing.
//!
//! Coordinate discipline: a declared planar Z is already in canonical units
//! (1 unit = 100 nm); a `LayerCollectionIR.z` is in mm. All comparisons happen
//! in i64 unit space via `slicer_ir::mm_to_units`.

use crate::common;
use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    AnchoredEntity, AnchoredEntityProvenance, AnchoredGeometryContract, ExtrusionRole, GCodeIR,
    GlobalLayer, LayerCollectionIR, LayerPlanIR, LayerStageCommit, MeshIR, Point3WithWidth,
    StageId,
};
use slicer_runtime::pipeline::{run_pipeline, run_pipeline_with_instrumentation, PipelineConfig};
use slicer_runtime::{
    CompiledModule, CompiledModuleBuilder, CompiledModuleLive, CompiledStage, ExecutionPlan,
    FinalizationError, FinalizationOutput, FinalizationStageInput, FinalizationStageRunner,
    GCodeEmitError, GCodeSerializer, LayerStageError, LayerStageInput, LayerStageRunner,
    NoopInstrumentation, NoopLayerProgressSink, PostpassError, PostpassOutput, PostpassStageInput,
    PostpassStageRunner, PrepassRunnerError, PrepassStageInput, PrepassStageOutput,
    PrepassStageRunner,
};

// ── Grid / off-grid Z constants (mm; converted to units at every comparison) ──

/// First global layer Z.
const GRID_Z0_MM: f32 = 0.2;
/// Second global layer Z; the anchor layer for every entity below.
const GRID_Z1_MM: f32 = 0.4;
/// Declared plane of the off-grid entity — strictly between the two grid Zs and
/// far outside `COORDINATE_TOLERANCE_UNITS` of either.
const OFFGRID_Z_MM: f32 = 0.3;

/// Global layer index the anchored entities are anchored to (`GRID_Z1_MM`).
const ANCHOR_LAYER_INDEX: u32 = 1;

fn grid_z0_units() -> i64 {
    slicer_ir::mm_to_units(GRID_Z0_MM)
}

fn grid_z1_units() -> i64 {
    slicer_ir::mm_to_units(GRID_Z1_MM)
}

fn offgrid_z_units() -> i64 {
    slicer_ir::mm_to_units(OFFGRID_Z_MM)
}

fn tolerance_units() -> i64 {
    AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS
}

// ── row helpers ──────────────────────────────────────────────────────────────

/// Indices of captured rows whose `z` (mm) matches `z_units` (canonical units)
/// within the anchored-contract coordinate tolerance. Comparison is in i64 unit
/// space — never float mm against units.
fn rows_at(rows: &[LayerCollectionIR], z_units: i64) -> Vec<usize> {
    rows.iter()
        .enumerate()
        .filter(|(_, row)| (slicer_ir::mm_to_units(row.z) - z_units).abs() <= tolerance_units())
        .map(|(index, _)| index)
        .collect()
}

/// Indices of captured rows carrying a `PrintEntity` produced from the anchored
/// entity with `local_id == entity_id` (`append_same_z_entities` stamps
/// `PrintEntity.entity_id` from `AnchoredEntity.local_id`).
fn rows_carrying(rows: &[LayerCollectionIR], entity_id: u64) -> Vec<usize> {
    rows.iter()
        .enumerate()
        .filter(|(_, row)| {
            row.ordered_entities
                .iter()
                .any(|entity| entity.entity_id == entity_id)
        })
        .map(|(index, _)| index)
        .collect()
}

/// Total number of `PrintEntity` occurrences across EVERY row's
/// `ordered_entities`, not the number of rows carrying at least one.
///
/// `rows_carrying` collapses duplicates within a single row (it tests
/// membership with `any`), so it can prove "on exactly these rows" but never
/// "exactly once". AC-2's totality claim is that a routed entity is neither
/// dropped nor DUPLICATED, which needs an occurrence count.
fn occurrences_of(rows: &[LayerCollectionIR], entity_id: u64) -> usize {
    rows.iter()
        .flat_map(|row| row.ordered_entities.iter())
        .filter(|entity| entity.entity_id == entity_id)
        .count()
}

/// `(global_layer_index, z_mm, z_units, entity ids)` for every captured row —
/// the diagnostic body of every assertion message below.
fn describe(rows: &[LayerCollectionIR]) -> String {
    rows.iter()
        .map(|row| {
            format!(
                "(idx {}, z {} mm = {} units, entities {:?})",
                row.global_layer_index,
                row.z,
                slicer_ir::mm_to_units(row.z),
                row.ordered_entities
                    .iter()
                    .map(|entity| entity.entity_id)
                    .collect::<Vec<_>>()
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

// ── fixture construction ─────────────────────────────────────────────────────

/// An `AnchoredEntity` on a single declared plane, with one path point sitting
/// exactly on that plane so `validate_anchored_entity` accepts it.
fn planar_entity(local_id: u64, plane_units: i64, plane_mm: f32, feature: &str) -> AnchoredEntity {
    // exhaustive: no Default impl for AnchoredEntity; anchored-contract fixture pins every field
    AnchoredEntity {
        local_id,
        anchor_global_layer_index: ANCHOR_LAYER_INDEX,
        geometry: AnchoredGeometryContract::Planar { z: plane_units },
        input_capabilities: Vec::new(),
        output_capabilities: Vec::new(),
        provenance: AnchoredEntityProvenance {
            requesting_feature: feature.to_string(),
            source_plan_entry: feature.to_string(),
        },
        path_points: vec![
            Point3WithWidth {
                x: 1.0,
                y: 1.0,
                z: plane_mm,
                width: 0.45,
                flow_factor: 1.0,
                ..Default::default()
            },
            Point3WithWidth {
                x: 2.0,
                y: 1.0,
                z: plane_mm,
                width: 0.45,
                flow_factor: 1.0,
                ..Default::default()
            },
        ],
        role: ExtrusionRole::SupportMaterial,
    }
}

/// The off-grid `same-z-support` entity: declared at `OFFGRID_Z_MM`, anchored to
/// the `GRID_Z1_MM` layer, so it is NOT within tolerance of its anchor.
fn offgrid_entity(local_id: u64) -> AnchoredEntity {
    planar_entity(local_id, offgrid_z_units(), OFFGRID_Z_MM, "same-z-support")
}

/// An on-grid `same-z-support` entity: declared exactly on its anchor layer's Z,
/// so `route_of` routes it into that layer's `ordered_entities`.
fn ongrid_entity(local_id: u64) -> AnchoredEntity {
    planar_entity(local_id, grid_z1_units(), GRID_Z1_MM, "same-z-support")
}

fn make_global_layer(index: u32, z: f32) -> GlobalLayer {
    GlobalLayer {
        index,
        z,
        ..Default::default()
    }
}

fn make_dummy_module(module_id: &str) -> CompiledModule {
    CompiledModuleBuilder::new(module_id).build()
}

/// Prepass that commits the two-layer global grid (`GRID_Z0_MM`, `GRID_Z1_MM`).
struct TwoLayerGridPrepass;
impl PrepassStageRunner for TwoLayerGridPrepass {
    fn run_stage(
        &self,
        _stage_id: &StageId,
        _module: &CompiledModuleLive<'_>,
        _input: PrepassStageInput<'_>,
    ) -> Result<PrepassStageOutput, PrepassRunnerError> {
        Ok(PrepassStageOutput::LayerPlan(Arc::new(LayerPlanIR {
            global_layers: vec![
                make_global_layer(0, GRID_Z0_MM),
                make_global_layer(ANCHOR_LAYER_INDEX, GRID_Z1_MM),
            ],
            ..Default::default()
        })))
    }
}

struct NoopLayerRunner;
impl LayerStageRunner for NoopLayerRunner {
    fn run_stage(
        &self,
        _stage_id: &StageId,
        _layer: &GlobalLayer,
        _module: &CompiledModuleLive<'_>,
        _input: LayerStageInput<'_>,
    ) -> Result<Option<LayerStageCommit>, LayerStageError> {
        Ok(None)
    }
}

struct AnchoredProducerLayerRunner;
impl LayerStageRunner for AnchoredProducerLayerRunner {
    fn run_stage(
        &self,
        _stage_id: &StageId,
        layer: &GlobalLayer,
        _module: &CompiledModuleLive<'_>,
        _input: LayerStageInput<'_>,
    ) -> Result<Option<LayerStageCommit>, LayerStageError> {
        if layer.index != ANCHOR_LAYER_INDEX {
            return Ok(None);
        }
        Ok(Some(LayerStageCommit::AnchoredEvents(vec![
            slicer_ir::OrderedEventCollection {
                anchor_global_layer_index: ANCHOR_LAYER_INDEX,
                events: vec![offgrid_entity(901)],
                runtime_hooks: Default::default(),
            },
        ])))
    }
}

struct NoopFinalizationRunner;
impl FinalizationStageRunner for NoopFinalizationRunner {
    fn run_stage(
        &self,
        _stage_id: &StageId,
        _module: &CompiledModuleLive<'_>,
        _input: FinalizationStageInput<'_>,
        _layers: &mut Vec<LayerCollectionIR>,
    ) -> Result<FinalizationOutput, FinalizationError> {
        Ok(FinalizationOutput::Success)
    }
}

struct NoopPostpassRunner;
impl PostpassStageRunner for NoopPostpassRunner {
    fn run_gcode_postprocess(
        &self,
        _stage_id: &StageId,
        _module: &CompiledModuleLive<'_>,
        _input: PostpassStageInput<'_>,
        _commands: &mut Vec<slicer_ir::GCodeCommand>,
    ) -> Result<PostpassOutput, PostpassError> {
        Ok(PostpassOutput::GCodeSuccess)
    }

    fn run_text_postprocess(
        &self,
        _stage_id: &StageId,
        _module: &CompiledModuleLive<'_>,
        _input: PostpassStageInput<'_>,
        text: String,
    ) -> Result<PostpassOutput, PostpassError> {
        Ok(PostpassOutput::TextSuccess { text })
    }
}

struct MinimalSerializer;
impl GCodeSerializer for MinimalSerializer {
    fn serialize_gcode(&self, _gcode_ir: &GCodeIR) -> Result<String, GCodeEmitError> {
        Ok(String::new())
    }
}

fn grid_plan() -> ExecutionPlan {
    ExecutionPlan {
        prepass_stages: vec![CompiledStage {
            stage_id: "PrePass::LayerPlanning".into(),
            modules: vec![make_dummy_module("layer-planner")],
        }],
        per_layer_stages: Vec::new(),
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(Vec::new()),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
        ..Default::default()
    }
}

fn producer_plan() -> ExecutionPlan {
    let mut plan = grid_plan();
    plan.per_layer_stages.push(CompiledStage {
        stage_id: "Layer::Support".into(),
        modules: vec![make_dummy_module("anchored-support-producer")],
    });
    plan
}

fn empty_mesh_ir() -> Arc<MeshIR> {
    Arc::new(MeshIR::default())
}

/// Runs a full pipeline with the supplied `anchored_entities` and returns the
/// exact `LayerCollectionIR` row sequence handed to the G-code emitter.
fn capture_rows(anchored_entities: Vec<AnchoredEntity>) -> Vec<LayerCollectionIR> {
    let emitter = common::CapturedRowsEmitter::new();
    // Clone the capture handle out BEFORE the emitter is boxed and moved.
    let captured_handle = emitter.handle();

    let config = PipelineConfig {
        anchored_entities,
        ..common::pipeline_config_base(
            empty_mesh_ir(),
            grid_plan(),
            common::pipeline_stage_runners_base(
                Box::new(TwoLayerGridPrepass),
                Box::new(NoopLayerRunner),
                Box::new(NoopFinalizationRunner),
                Box::new(NoopPostpassRunner),
                Box::new(emitter),
                Box::new(MinimalSerializer),
            ),
        )
    };

    let raw_config: HashMap<slicer_ir::ConfigKey, slicer_ir::ConfigValue> = HashMap::new();
    run_pipeline_with_instrumentation(
        config,
        &raw_config,
        &NoopLayerProgressSink,
        &NoopInstrumentation,
    )
    .expect("pipeline with anchored entities must succeed");

    let rows = captured_handle.lock().unwrap().clone();
    assert!(!rows.is_empty(), "emitter captured no rows at all");
    rows
}

/// Regression for packet 239c's producer drain: anchored collections committed
/// by a per-layer module must survive the worker arena and reach row synthesis.
#[test]
fn layer_stage_anchored_commit_reaches_synthesized_rows() {
    const PRODUCER_ENTITY_ID: u64 = 901;
    const CONFIGURED_ENTITY_ID: u64 = 902;
    let emitter = common::CapturedRowsEmitter::new();
    let captured_handle = emitter.handle();
    let mut config = common::pipeline_config_base(
        empty_mesh_ir(),
        producer_plan(),
        common::pipeline_stage_runners_base(
            Box::new(TwoLayerGridPrepass),
            Box::new(AnchoredProducerLayerRunner),
            Box::new(NoopFinalizationRunner),
            Box::new(NoopPostpassRunner),
            Box::new(emitter),
            Box::new(MinimalSerializer),
        ),
    );
    config.anchored_entities = vec![offgrid_entity(CONFIGURED_ENTITY_ID)];

    run_pipeline(config).expect("module-produced anchored collection must reach the emitter");
    let rows = captured_handle.lock().unwrap().clone();
    let row = require_declared_z_row(&rows);
    assert_eq!(
        rows_carrying(&rows, PRODUCER_ENTITY_ID),
        vec![row],
        "per-layer LayerStageCommit::AnchoredEvents was dropped before synthesis: [{}]",
        describe(&rows)
    );
    assert_eq!(
        rows_carrying(&rows, CONFIGURED_ENTITY_ID),
        vec![row],
        "merging producer collections dropped configured anchored entities: [{}]",
        describe(&rows)
    );
}

/// Locate the single row at the declared off-grid Z, failing with a message
/// that names that row when it is absent.
fn require_declared_z_row(rows: &[LayerCollectionIR]) -> usize {
    let matches = rows_at(rows, offgrid_z_units());
    assert_eq!(
        matches.len(),
        1,
        "missing declared-Z row: expected exactly one captured row at the declared \
         off-grid Z {OFFGRID_Z_MM} mm ({} units), found {} such row(s) among {} captured rows: [{}]",
        offgrid_z_units(),
        matches.len(),
        rows.len(),
        describe(rows)
    );
    matches[0]
}

// ── AC-1 ─────────────────────────────────────────────────────────────────────

/// AC-1: one off-grid `same-z-support` entity declared at `OFFGRID_Z_MM` must be
/// lowered into its own row at that Z, ordered strictly between the two grid
/// rows, and its paths must land on exactly that one row.
#[test]
fn offgrid_support_row_emitted_at_declared_z() {
    const ENTITY_ID: u64 = 1;
    let rows = capture_rows(vec![offgrid_entity(ENTITY_ID)]);

    let declared_row = require_declared_z_row(&rows);

    // Z equality in mm, to the AC-1 tolerance.
    assert!(
        (rows[declared_row].z - OFFGRID_Z_MM).abs() < 1e-6,
        "missing declared-Z row at the correct Z: row {declared_row} has z {} mm, \
         want {OFFGRID_Z_MM} mm; captured rows: [{}]",
        rows[declared_row].z,
        describe(&rows)
    );

    // Strict ordering between the two grid rows.
    let grid0 = rows_at(&rows, grid_z0_units());
    let grid1 = rows_at(&rows, grid_z1_units());
    assert_eq!(
        grid0.len(),
        1,
        "expected one row at {GRID_Z0_MM} mm: [{}]",
        describe(&rows)
    );
    assert_eq!(
        grid1.len(),
        1,
        "expected one row at {GRID_Z1_MM} mm: [{}]",
        describe(&rows)
    );
    assert!(
        grid0[0] < declared_row && declared_row < grid1[0],
        "the declared-Z row at {OFFGRID_Z_MM} mm must be ordered strictly between the \
         {GRID_Z0_MM} mm row (position {}) and the {GRID_Z1_MM} mm row (position {}), \
         but it sits at position {declared_row}; captured rows: [{}]",
        grid0[0],
        grid1[0],
        describe(&rows)
    );

    // The entity's paths appear on exactly that one row.
    let carriers = rows_carrying(&rows, ENTITY_ID);
    assert_eq!(
        carriers,
        vec![declared_row],
        "off-grid entity {ENTITY_ID} paths must appear only on the declared-Z row at \
         {OFFGRID_Z_MM} mm (position {declared_row}), but were found on rows {carriers:?}; \
         captured rows: [{}]",
        describe(&rows)
    );
}

// ── AC-2 ─────────────────────────────────────────────────────────────────────

/// AC-2: routing is a total partition at pipeline level — an on-grid and an
/// off-grid `same-z-support` entity each land on exactly one row, neither
/// dropped nor duplicated.
#[test]
fn every_same_z_support_entity_routes_exactly_once() {
    const ONGRID_ID: u64 = 10;
    const OFFGRID_ID: u64 = 20;
    let rows = capture_rows(vec![ongrid_entity(ONGRID_ID), offgrid_entity(OFFGRID_ID)]);

    // The declared-Z row must exist before any routing claim can be checked.
    let declared_row = require_declared_z_row(&rows);

    let ongrid_rows = rows_at(&rows, grid_z1_units());
    assert_eq!(
        ongrid_rows.len(),
        1,
        "expected exactly one anchor row at {GRID_Z1_MM} mm: [{}]",
        describe(&rows)
    );

    let ongrid_carriers = rows_carrying(&rows, ONGRID_ID);
    assert_eq!(
        ongrid_carriers,
        vec![ongrid_rows[0]],
        "on-grid entity {ONGRID_ID} must appear exactly once, inside its anchor row's \
         ordered_entities (position {}), but was found on rows {ongrid_carriers:?}; \
         captured rows: [{}]",
        ongrid_rows[0],
        describe(&rows)
    );

    let offgrid_carriers = rows_carrying(&rows, OFFGRID_ID);
    assert_eq!(
        offgrid_carriers,
        vec![declared_row],
        "off-grid entity {OFFGRID_ID} must appear exactly once, on its own declared-Z row \
         at {OFFGRID_Z_MM} mm (position {declared_row}), but was found on rows \
         {offgrid_carriers:?}; captured rows: [{}]",
        describe(&rows)
    );

    // `rows_carrying` is row-granular: it uses `any`, so two copies of the same
    // entity inside ONE row's `ordered_entities` would satisfy both assertions
    // above. AC-2 claims each entity's paths appear EXACTLY once across the
    // whole sequence, so count occurrences, not carrier rows.
    for (label, id) in [("on-grid", ONGRID_ID), ("off-grid", OFFGRID_ID)] {
        assert_eq!(
            occurrences_of(&rows, id),
            1,
            "{label} entity {id} must appear exactly ONCE across all rows' ordered_entities \
             (neither dropped nor duplicated within a row); captured rows: [{}]",
            describe(&rows)
        );
    }
}

// ── AC-N2 ────────────────────────────────────────────────────────────────────

/// AC-N2: an off-grid entity whose plane differs from EVERY global-layer Z by
/// more than `COORDINATE_TOLERANCE_UNITS` must never be merged into a grid row.
#[test]
fn offgrid_entity_never_merged_into_grid_layers() {
    const ENTITY_ID: u64 = 7;

    // Premise of the AC: the declared plane is off-grid w.r.t. every grid Z.
    for grid_units in [grid_z0_units(), grid_z1_units()] {
        assert!(
            (offgrid_z_units() - grid_units).abs() > tolerance_units(),
            "fixture premise broken: declared plane {} units is within tolerance {} of \
             grid Z {grid_units} units",
            offgrid_z_units(),
            tolerance_units()
        );
    }

    let rows = capture_rows(vec![offgrid_entity(ENTITY_ID)]);

    // The synthesized declared-Z row must exist — otherwise "not merged into a
    // grid row" is vacuously true and proves nothing.
    let declared_row = require_declared_z_row(&rows);

    let grid_rows: Vec<usize> = rows_at(&rows, grid_z0_units())
        .into_iter()
        .chain(rows_at(&rows, grid_z1_units()))
        .collect();
    let carriers = rows_carrying(&rows, ENTITY_ID);

    for grid_row in &grid_rows {
        assert!(
            !carriers.contains(grid_row),
            "off-grid entity {ENTITY_ID} was merged into grid row {grid_row}; it must appear \
             only on the synthesized declared-Z row at {OFFGRID_Z_MM} mm (position \
             {declared_row}); captured rows: [{}]",
            describe(&rows)
        );
    }

    assert_eq!(
        carriers,
        vec![declared_row],
        "off-grid entity {ENTITY_ID} must appear only on the declared-Z row at \
         {OFFGRID_Z_MM} mm (position {declared_row}), but was found on rows {carriers:?}; \
         captured rows: [{}]",
        describe(&rows)
    );
}

// ── AC-3 ─────────────────────────────────────────────────────────────────────

/// Manifest text for a `Layer::PathOptimization` module. `layer-parallel-safe`
/// is left as a `{safe}` placeholder so the same text serves both modes.
const PARALLEL_SAFE_MANIFEST: &str = r#"
[module]
id = "test.anchored.offgrid"
version = "1.0.0"

[stage]
id = "Layer::PathOptimization"

[ir-access]
reads = []
writes = []

[claims]
holds = []
requires = []

[compatibility]
incompatible-with = []
requires = []
min-host-version = "0.1.0"
min-ir-schema = "0.1.0"
max-ir-schema = "1.0.0"

[config.overridable-per-region]
keys = []

[config.overridable-per-layer]
keys = []

[hints]
layer-parallel-safe = true
"#;

/// Load a `layer-parallel-safe` module through the scheduler, mirroring the
/// fixture idiom of `anchored_parallel_determinism.rs`. The caller owns the
/// `TempDir` so the manifest outlives the returned module.
fn parallel_safe_module(directory: &tempfile::TempDir) -> slicer_scheduler::manifest::LoadedModule {
    let manifest_path = directory.path().join("anchored.toml");
    let wasm_path = directory.path().join("anchored.wasm");
    std::fs::write(&manifest_path, PARALLEL_SAFE_MANIFEST)
        .expect("manifest fixture must be written");
    std::fs::write(&wasm_path, b"fixture").expect("wasm fixture must be written");
    slicer_scheduler::manifest::load_module_from_paths(&manifest_path, &wasm_path)
        .expect("manifest fixture must load through the scheduler")
}

/// `(z_units, global_layer_index)` for every row — the FULL pair sequence AC-3
/// compares. Comparing `z` alone would let a nondeterministic or wrongly
/// attributed `global_layer_index` slip through.
fn z_index_pairs(rows: &[LayerCollectionIR]) -> Vec<(i64, u32)> {
    rows.iter()
        .map(|row| (slicer_ir::mm_to_units(row.z), row.global_layer_index))
        .collect()
}

/// Per-row entity-id ordering — the second half of the AC-3 comparison.
fn entity_orders(rows: &[LayerCollectionIR]) -> Vec<Vec<u64>> {
    rows.iter()
        .map(|row| {
            row.ordered_entities
                .iter()
                .map(|entity| entity.entity_id)
                .collect()
        })
        .collect()
}

/// AC-3: the synthesized row order is identical whether the anchored
/// collections were executed serially or in parallel.
///
/// Scoped at the EXECUTOR CALL, not the pipeline: `force_parallel` is the third
/// positional parameter of
/// `layer_executor::execute_anchored_event_collections_with_mode`. There is no
/// `force_parallel` config key or `PipelineConfig` field, and this packet
/// creates none — pipeline-level parallel determinism is out of scope.
///
/// Both executions are lowered through `synthesize_anchored_rows` against the
/// SAME fixed set of `CommittedLayerEvent::Model` rows, then compared on the
/// full `(z, global_layer_index)` PAIR sequence and on per-row entity ordering.
/// The pair comparison is load-bearing: it pins the locked index-assignment
/// rule of ADR-0059 (a solo synthesized row adopts the UPPER anchor layer's
/// index), which a `z`-only comparison would ignore.
#[test]
fn offgrid_row_order_identical_serial_and_parallel() {
    use slicer_ir::OrderedEventCollection;
    use slicer_runtime::anchored_rows::synthesize_anchored_rows;
    use slicer_runtime::layer_executor::{
        execute_anchored_event_collections_with_mode, CommittedLayerEvent,
    };

    /// Three distinct intermediate planes, all strictly between the two grid Zs
    /// and all far outside `COORDINATE_TOLERANCE_UNITS` of either.
    const PLANE_A_MM: f32 = 0.25;
    const PLANE_B_MM: f32 = 0.30;
    const PLANE_C_MM: f32 = 0.35;

    /// Two entities per plane, submitted in descending id order so that per-row
    /// entity ordering is a real constraint rather than input echo.
    fn plane_pair(base: u64, z_mm: f32) -> Vec<AnchoredEntity> {
        let z_units = slicer_ir::mm_to_units(z_mm);
        vec![
            planar_entity(base + 2, z_units, z_mm, "same-z-support"),
            planar_entity(base + 1, z_units, z_mm, "same-z-support"),
        ]
    }

    let plan = ExecutionPlan {
        global_layers: Arc::new(vec![
            make_global_layer(0, GRID_Z0_MM),
            make_global_layer(ANCHOR_LAYER_INDEX, GRID_Z1_MM),
        ]),
        ..Default::default()
    };

    // Planes submitted out of ascending order, ids descending within each plane.
    let entities: Vec<AnchoredEntity> = [(30, PLANE_C_MM), (10, PLANE_A_MM), (20, PLANE_B_MM)]
        .into_iter()
        .flat_map(|(base, z_mm)| plane_pair(base, z_mm))
        .collect();

    // Premise: every declared plane is genuinely off-grid, so each one really
    // does take the synthesized-row route.
    for plane_mm in [PLANE_A_MM, PLANE_B_MM, PLANE_C_MM] {
        for grid_units in [grid_z0_units(), grid_z1_units()] {
            assert!(
                (slicer_ir::mm_to_units(plane_mm) - grid_units).abs() > tolerance_units(),
                "AC-3 premise broken: plane {plane_mm} mm is within tolerance {} of grid Z \
                 {grid_units} units",
                tolerance_units()
            );
        }
    }

    let directory = tempfile::tempdir().expect("manifest fixture directory must be created");
    let module = parallel_safe_module(&directory);
    assert!(
        module.layer_parallel_safe(),
        "AC-3 premise: the fixture module must be layer-parallel-safe"
    );
    // Premise: parallel mode really engages — the executor falls back to serial
    // unless EVERY entity's invocation is parallel-safe, which would make the
    // comparison below vacuous.
    for entity in &entities {
        assert!(
            plan.anchored_invocation(entity, module.layer_parallel_safe())
                .layer_parallel_safe,
            "AC-3 premise: entity {} must be parallel-safe, else `force_parallel = true` \
             silently falls back to serial and the comparison proves nothing",
            entity.local_id
        );
    }

    let (serial_collections, _serial_accounting) =
        execute_anchored_event_collections_with_mode(&plan, &entities, false, &module)
            .expect("serial anchored execution must succeed");
    let (parallel_collections, _parallel_accounting) =
        execute_anchored_event_collections_with_mode(&plan, &entities, true, &module)
            .expect("parallel anchored execution must succeed");

    // The SAME fixed object rows are used for both lowerings.
    fn fixed_model_rows() -> Vec<CommittedLayerEvent> {
        vec![
            CommittedLayerEvent::Model(LayerCollectionIR {
                global_layer_index: 0,
                z: GRID_Z0_MM,
                ..Default::default()
            }),
            CommittedLayerEvent::Model(LayerCollectionIR {
                global_layer_index: ANCHOR_LAYER_INDEX,
                z: GRID_Z1_MM,
                ..Default::default()
            }),
        ]
    }

    let lower = |collections: Vec<OrderedEventCollection>| -> Vec<LayerCollectionIR> {
        let mut committed: Vec<CommittedLayerEvent> = collections
            .into_iter()
            .map(CommittedLayerEvent::Anchored)
            .collect();
        committed.extend(fixed_model_rows());
        synthesize_anchored_rows(committed).expect("determinism fixture must synthesize cleanly")
    };

    let serial_rows = lower(serial_collections);
    let parallel_rows = lower(parallel_collections);

    // ── Non-vacuity + the locked index-assignment rule ───────────────────────
    // Every solo synthesized row must adopt the UPPER anchor layer's index
    // (ADR-0059). Spelling the expected pair sequence out here is what makes a
    // wrongly attributed index a FAILURE rather than a silently-equal pair of
    // wrong answers.
    let expected_pairs: Vec<(i64, u32)> = vec![
        (grid_z0_units(), 0),
        (slicer_ir::mm_to_units(PLANE_A_MM), ANCHOR_LAYER_INDEX),
        (slicer_ir::mm_to_units(PLANE_B_MM), ANCHOR_LAYER_INDEX),
        (slicer_ir::mm_to_units(PLANE_C_MM), ANCHOR_LAYER_INDEX),
        (grid_z1_units(), ANCHOR_LAYER_INDEX),
    ];
    assert_eq!(
        z_index_pairs(&serial_rows),
        expected_pairs,
        "serial lowering produced the wrong (z, global_layer_index) pair sequence — each solo \
         synthesized row must adopt the UPPER anchor layer's index (ADR-0059); rows: [{}]",
        describe(&serial_rows)
    );

    let expected_orders: Vec<Vec<u64>> = vec![
        Vec::new(),
        vec![11, 12],
        vec![21, 22],
        vec![31, 32],
        Vec::new(),
    ];
    assert_eq!(
        entity_orders(&serial_rows),
        expected_orders,
        "serial lowering produced the wrong per-row entity ordering; rows: [{}]",
        describe(&serial_rows)
    );

    // ── The AC-3 claim proper: serial and parallel agree ─────────────────────
    assert_eq!(
        z_index_pairs(&parallel_rows),
        z_index_pairs(&serial_rows),
        "parallel execution changed the (z, global_layer_index) PAIR sequence.\n serial:   \
         [{}]\n parallel: [{}]",
        describe(&serial_rows),
        describe(&parallel_rows)
    );
    assert_eq!(
        entity_orders(&parallel_rows),
        entity_orders(&serial_rows),
        "parallel execution changed per-row entity ordering.\n serial:   [{}]\n parallel: [{}]",
        describe(&serial_rows),
        describe(&parallel_rows)
    );
    assert_eq!(
        parallel_rows,
        serial_rows,
        "parallel execution changed the synthesized row sequence.\n serial:   [{}]\n parallel: \
         [{}]",
        describe(&serial_rows),
        describe(&parallel_rows)
    );
}

// ── AC-4 ─────────────────────────────────────────────────────────────────────

/// Positions inside `row.ordered_entities` carrying `entity_id`.
fn block_in_row(row: &LayerCollectionIR, entity_id: u64) -> Vec<usize> {
    row.ordered_entities
        .iter()
        .enumerate()
        .filter(|(_, entity)| entity.entity_id == entity_id)
        .map(|(index, _)| index)
        .collect()
}

/// AC-4: a `ZSpanning` `same-z-support` entity spanning several object layers
/// executes as ONE CONTIGUOUS BLOCK inside its anchor layer's ordinary row.
///
/// `docs/adr/0059-support-families-and-anchored-entities.md`: "A future atomic
/// Z-spanning entity may extend outside its anchor layer's Z interval **while
/// still executing at that layer's normal position**." Atomicity is unchanged;
/// what the ADR pins is WHERE the block lives.
///
/// Three properties are asserted explicitly:
/// (a) contiguity — the entity's paths occupy consecutive indices;
/// (b) location — inside the ANCHOR layer's `Model` row, at that layer's normal
///     position, never on a synthesized row;
/// (c) no extra row — the output row count equals the object row count.
///
/// The entity is driven through the real executor
/// (`execute_anchored_event_collections_with_mode`) and only then lowered, so
/// this is the integration-level counterpart of the in-module unit test
/// `z_spanning_entity_lands_in_its_anchor_row`, not a reuse of its internals.
#[test]
fn zspanning_support_entity_emits_atomic_single_block() {
    use slicer_runtime::anchored_rows::synthesize_anchored_rows;
    use slicer_runtime::layer_executor::{
        execute_anchored_event_collections_with_mode, CommittedLayerEvent,
    };

    /// Third grid layer, so the spanning entity crosses SEVERAL object layers.
    const GRID_Z2_MM: f32 = 0.6;
    const TOP_LAYER_INDEX: u32 = 2;
    /// The Z-spanning entity under test, and a second one sharing its anchor —
    /// without a companion, "one contiguous block" would hold trivially.
    const SPANNING_ID: u64 = 501;
    const COMPANION_ID: u64 = 502;
    /// An ordinary model path already present on the anchor row, so the block's
    /// placement "inside the ordinary row" is observable.
    const MODEL_ENTITY_ID: u64 = 900;

    /// A `ZSpanning` `same-z-support` entity whose path points sit at the given
    /// Zs — several of them on DIFFERENT object layers.
    fn spanning_entity(
        local_id: u64,
        min_mm: f32,
        max_mm: f32,
        point_zs: &[f32],
    ) -> AnchoredEntity {
        let mut entity = planar_entity(
            local_id,
            slicer_ir::mm_to_units(min_mm),
            min_mm,
            "same-z-support",
        );
        entity.geometry = AnchoredGeometryContract::ZSpanning {
            min_z: slicer_ir::mm_to_units(min_mm),
            max_z: slicer_ir::mm_to_units(max_mm),
        };
        entity.path_points = point_zs
            .iter()
            .enumerate()
            .map(|(index, z)| Point3WithWidth {
                x: index as f32,
                y: 0.0,
                z: *z,
                width: 0.45,
                flow_factor: 1.0,
                ..Default::default()
            })
            .collect();
        entity
    }

    /// The ordinary model path already staged on the anchor row.
    fn model_entity() -> slicer_ir::PrintEntity {
        // exhaustive: PrintEntity has no Default derive (tool_index must be set explicitly)
        slicer_ir::PrintEntity {
            entity_id: MODEL_ENTITY_ID,
            // exhaustive: ExtrusionPath3D has no Default derive
            path: slicer_ir::ExtrusionPath3D {
                points: Vec::new(),
                role: slicer_ir::ExtrusionRole::OuterWall,
                speed_factor: 1.0,
                tool_index: None,
                order_lock: None,
            },
            role: slicer_ir::ExtrusionRole::OuterWall,
            region_key: slicer_ir::RegionKey::default(),
            topo_order: 0,
            tool_index: 0,
        }
    }

    // The spanning path points, crossing all three object layers.
    let spanning_point_zs = [GRID_Z0_MM, GRID_Z1_MM, GRID_Z2_MM];

    let plan = ExecutionPlan {
        global_layers: Arc::new(vec![
            make_global_layer(0, GRID_Z0_MM),
            make_global_layer(ANCHOR_LAYER_INDEX, GRID_Z1_MM),
            make_global_layer(TOP_LAYER_INDEX, GRID_Z2_MM),
        ]),
        ..Default::default()
    };

    let entities = vec![
        spanning_entity(SPANNING_ID, GRID_Z0_MM, GRID_Z2_MM, &spanning_point_zs),
        spanning_entity(
            COMPANION_ID,
            GRID_Z1_MM,
            GRID_Z2_MM,
            &[GRID_Z1_MM, GRID_Z2_MM],
        ),
    ];

    // Premise: the span really does cross several object layers.
    assert!(
        slicer_ir::mm_to_units(GRID_Z0_MM) < slicer_ir::mm_to_units(GRID_Z1_MM)
            && slicer_ir::mm_to_units(GRID_Z1_MM) < slicer_ir::mm_to_units(GRID_Z2_MM),
        "AC-4 premise: the three grid Zs must be strictly ascending"
    );

    let directory = tempfile::tempdir().expect("manifest fixture directory must be created");
    let module = parallel_safe_module(&directory);
    let (collections, _accounting) =
        execute_anchored_event_collections_with_mode(&plan, &entities, false, &module)
            .expect("anchored execution of the Z-spanning entity must succeed");
    assert!(
        collections
            .iter()
            .any(|collection| collection.anchor_global_layer_index == ANCHOR_LAYER_INDEX),
        "AC-4 premise: the Z-spanning entities must reach an anchored collection for layer \
         {ANCHOR_LAYER_INDEX}"
    );

    // Fixed object rows: three layers, the anchor row already carrying one
    // ordinary model path.
    let object_rows = vec![
        LayerCollectionIR {
            global_layer_index: 0,
            z: GRID_Z0_MM,
            ..Default::default()
        },
        LayerCollectionIR {
            global_layer_index: ANCHOR_LAYER_INDEX,
            z: GRID_Z1_MM,
            ordered_entities: vec![model_entity()],
            ..Default::default()
        },
        LayerCollectionIR {
            global_layer_index: TOP_LAYER_INDEX,
            z: GRID_Z2_MM,
            ..Default::default()
        },
    ];
    let object_row_count = object_rows.len();

    let mut committed: Vec<CommittedLayerEvent> = collections
        .into_iter()
        .map(CommittedLayerEvent::Anchored)
        .collect();
    committed.extend(object_rows.into_iter().map(CommittedLayerEvent::Model));
    let rows = synthesize_anchored_rows(committed)
        .expect("the z-spanning entity anchors to a committed model row");

    // ── (c) no extra row ─────────────────────────────────────────────────────
    assert_eq!(
        rows.len(),
        object_row_count,
        "a Z-spanning entity must NOT get a synthesized row of its own: expected the \
         {object_row_count} object rows, got {}; rows: [{}]",
        rows.len(),
        describe(&rows)
    );
    let expected_pairs: Vec<(i64, u32)> = vec![
        (grid_z0_units(), 0),
        (grid_z1_units(), ANCHOR_LAYER_INDEX),
        (slicer_ir::mm_to_units(GRID_Z2_MM), TOP_LAYER_INDEX),
    ];
    assert_eq!(
        z_index_pairs(&rows),
        expected_pairs,
        "the row sequence must be exactly the object rows, unchanged in Z and index; rows: [{}]",
        describe(&rows)
    );

    // ── (b) location: the ANCHOR layer's Model row, at its normal position ───
    let carriers = rows_carrying(&rows, SPANNING_ID);
    let anchor_position = rows
        .iter()
        .position(|row| row.global_layer_index == ANCHOR_LAYER_INDEX)
        .expect("the anchor layer's model row must be present");
    assert_eq!(
        carriers,
        vec![anchor_position],
        "the Z-spanning entity must appear ONLY in its anchor layer's ordinary row (position \
         {anchor_position}), never on a synthesized row nor split across object layers, but was \
         found on rows {carriers:?}; rows: [{}]",
        describe(&rows)
    );
    let anchor_row = &rows[anchor_position];
    assert_eq!(
        slicer_ir::mm_to_units(anchor_row.z),
        grid_z1_units(),
        "the anchor row must keep its own Z — the entity executes at that layer's NORMAL \
         position; rows: [{}]",
        describe(&rows)
    );
    assert_eq!(
        anchor_row.ordered_entities.first().map(|e| e.entity_id),
        Some(MODEL_ENTITY_ID),
        "the anchor row's pre-existing ordinary model path must still lead the row; rows: [{}]",
        describe(&rows)
    );

    // ── (a) contiguity: consecutive indices, one block, never fragmented ─────
    // Per-entity presence first.
    for entity_id in [SPANNING_ID, COMPANION_ID] {
        assert!(
            !block_in_row(anchor_row, entity_id).is_empty(),
            "Z-spanning entity {entity_id} vanished from the anchor row; rows: [{}]",
            describe(&rows)
        );
    }
    // Then the property that actually has teeth: the collection's Z-spanning
    // entities occupy ONE unbroken run of positions. A per-entity `windows(2)`
    // check cannot fail while each entity contributes a single path (asserted
    // below), so it would be vacuous; interleaving the two spanning entities
    // with an ordinary model path is the real failure this guards.
    let mut spanning_positions: Vec<usize> = [SPANNING_ID, COMPANION_ID]
        .iter()
        .flat_map(|id| block_in_row(anchor_row, *id))
        .collect();
    spanning_positions.sort_unstable();
    assert!(
        spanning_positions
            .windows(2)
            .all(|pair| pair[1] == pair[0] + 1),
        "the Z-spanning entities must form ONE contiguous block inside the anchor row, but they \
         occupy non-consecutive positions {spanning_positions:?}; rows: [{}]",
        describe(&rows)
    );

    // Atomicity of the geometry itself: the single block carries the whole
    // declared span, not one fragment per object layer.
    let spanning_block = block_in_row(anchor_row, SPANNING_ID);
    assert_eq!(
        spanning_block.len(),
        1,
        "the Z-spanning entity must contribute exactly ONE path block, not one per object \
         layer; it occupies positions {spanning_block:?}; rows: [{}]",
        describe(&rows)
    );
    let emitted_zs: Vec<i64> = anchor_row.ordered_entities[spanning_block[0]]
        .path
        .points
        .iter()
        .map(|point| slicer_ir::mm_to_units(point.z))
        .collect();
    let declared_zs: Vec<i64> = spanning_point_zs
        .iter()
        .map(|z| slicer_ir::mm_to_units(*z))
        .collect();
    assert_eq!(
        emitted_zs, declared_zs,
        "the Z-spanning block must carry every declared path point, in order and unsplit — the \
         span crosses several object layers and must not be fragmented at layer boundaries"
    );
}

// ── AC-5 ─────────────────────────────────────────────────────────────────────

/// AC-5: `synthesize_anchored_rows` reproduces the canonical
/// `GCode::collect_layers_to_print` merge rule, with the threshold sourced from
/// `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS`.
///
/// Called DIRECTLY on a committed event stream — no pipeline run — because the
/// synthesis function is pure over an already-ordered sequence. A declared
/// plane within the merge epsilon of an object row merges into it (one row, no
/// duplicate Z); a plane beyond that epsilon and lower emits its own solo row
/// first.
#[test]
fn offgrid_row_merge_matches_canonical_epsilon_rule() {
    use slicer_ir::{AnchoredEventRuntimeHooks, OrderedEventCollection};
    use slicer_runtime::anchored_rows::synthesize_anchored_rows;
    use slicer_runtime::layer_executor::CommittedLayerEvent;

    fn object_row(z_units: i64) -> LayerCollectionIR {
        LayerCollectionIR {
            global_layer_index: ANCHOR_LAYER_INDEX,
            z: slicer_ir::units_to_mm(z_units),
            ..Default::default()
        }
    }

    fn committed(entity: AnchoredEntity, object_z_units: i64) -> Vec<CommittedLayerEvent> {
        vec![
            CommittedLayerEvent::Anchored(OrderedEventCollection {
                anchor_global_layer_index: ANCHOR_LAYER_INDEX,
                events: vec![entity],
                runtime_hooks: AnchoredEventRuntimeHooks::default(),
            }),
            CommittedLayerEvent::Model(object_row(object_z_units)),
        ]
    }

    let object_z = grid_z1_units();

    // ── within epsilon: exactly one row, no duplicate Z ──────────────────────
    const MERGED_ID: u64 = 101;
    let near_z = object_z - tolerance_units();
    let merged = synthesize_anchored_rows(committed(
        planar_entity(
            MERGED_ID,
            near_z,
            slicer_ir::units_to_mm(near_z),
            "same-z-support",
        ),
        object_z,
    ))
    .expect("planar-only fixture must synthesize cleanly");

    assert_eq!(
        merged.len(),
        1,
        "a plane {} units from the object row is within the merge epsilon ({} units) and must \
         produce ONE row, got: [{}]",
        (near_z - object_z).abs(),
        tolerance_units(),
        describe(&merged)
    );
    assert_eq!(
        slicer_ir::mm_to_units(merged[0].z),
        object_z,
        "the merged row keeps the OBJECT row's Z; got: [{}]",
        describe(&merged)
    );
    assert_eq!(
        rows_carrying(&merged, MERGED_ID),
        vec![0],
        "the merged entity must ride on the single object row; got: [{}]",
        describe(&merged)
    );

    // ── beyond epsilon, lower Z: a solo row is emitted first ─────────────────
    const SOLO_ID: u64 = 202;
    let far_z = object_z - (tolerance_units() + 1);
    let split = synthesize_anchored_rows(committed(
        planar_entity(
            SOLO_ID,
            far_z,
            slicer_ir::units_to_mm(far_z),
            "same-z-support",
        ),
        object_z,
    ))
    .expect("planar-only fixture must synthesize cleanly");

    assert_eq!(
        split.len(),
        2,
        "a plane {} units below the object row is beyond the merge epsilon ({} units) and must \
         emit its own row, got: [{}]",
        (far_z - object_z).abs(),
        tolerance_units(),
        describe(&split)
    );
    assert_eq!(
        slicer_ir::mm_to_units(split[0].z),
        far_z,
        "the LOWER side emits first, at its declared Z; got: [{}]",
        describe(&split)
    );
    assert_eq!(
        slicer_ir::mm_to_units(split[1].z),
        object_z,
        "the object row follows unchanged; got: [{}]",
        describe(&split)
    );
    assert_eq!(
        split[0].global_layer_index,
        ANCHOR_LAYER_INDEX,
        "the solo row adopts the UPPER global layer's index (ADR-0059); got: [{}]",
        describe(&split)
    );
    assert_eq!(
        rows_carrying(&split, SOLO_ID),
        vec![0],
        "the off-grid entity must appear only on its own solo row; got: [{}]",
        describe(&split)
    );
    assert_eq!(
        split[0].schema_version,
        slicer_ir::CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION,
        "synthesized rows read the live schema-version constant"
    );
}

// ── AC-6 ─────────────────────────────────────────────────────────────────────

/// AC-6 (empty-collection equivalence): with ZERO anchored entities, the row
/// sequence handed to the emitter on the NEW committed path
/// (`run_pipeline_with_events` →
/// `execute_per_layer_with_committed_anchored_events_and_support_tools` →
/// `synthesize_anchored_rows`) must be element-wise identical — in length,
/// `global_layer_index`, and `z` — to the sequence recorded BEFORE the switch.
///
/// The baseline is the committed pre-change recording in
/// `pipeline_tdd.rs::payload_capturing_emitter_records_row_sequence` (TASK-400,
/// captured before any executor switch existed): `len == 3`, rows
/// `[(0, 0.2), (1, 0.4), (2, 0.6)]`. Those literals are reproduced here on
/// purpose. If this test fails, the switch changed the support-free row
/// sequence — a real regression. The baseline must never be edited to match.
#[test]
fn support_free_slice_row_sequence_unchanged() {
    /// Baseline row count recorded by TASK-400, pre-switch.
    const BASELINE_ROW_COUNT: usize = 3;
    /// Baseline `(global_layer_index, z_mm)` sequence recorded by TASK-400.
    ///
    /// This is the EXPECTATION only. It is deliberately NOT the constant that
    /// drives the fixture below: if one array fed both the prepass and the
    /// assertion, the test would be self-consistent by construction and could
    /// not detect a wrong Z schedule — only insertion, drop, or reorder.
    const BASELINE_ROWS: [(u32, f32); BASELINE_ROW_COUNT] = [(0, 0.2), (1, 0.4), (2, 0.6)];

    /// Layer schedule the fixture prepass PRODUCES, written out independently
    /// of `BASELINE_ROWS` so the two can disagree and be caught.
    const FIXTURE_LAYERS: [(u32, f32); 3] = [(0, 0.2), (1, 0.4), (2, 0.6)];

    /// Prepass mirroring the baseline's `ThreeLayerPrepass` exactly.
    struct BaselineThreeLayerPrepass;
    impl PrepassStageRunner for BaselineThreeLayerPrepass {
        fn run_stage(
            &self,
            _stage_id: &StageId,
            _module: &CompiledModuleLive<'_>,
            _input: PrepassStageInput<'_>,
        ) -> Result<PrepassStageOutput, PrepassRunnerError> {
            Ok(PrepassStageOutput::LayerPlan(Arc::new(LayerPlanIR {
                global_layers: FIXTURE_LAYERS
                    .iter()
                    .map(|(index, z)| make_global_layer(*index, *z))
                    .collect(),
                ..Default::default()
            })))
        }
    }

    let emitter = common::CapturedRowsEmitter::new();
    // Clone the capture handle out BEFORE the emitter is boxed and moved.
    let captured_handle = emitter.handle();

    let config = PipelineConfig {
        ..common::pipeline_config_base(
            empty_mesh_ir(),
            grid_plan(),
            common::pipeline_stage_runners_base(
                Box::new(BaselineThreeLayerPrepass),
                Box::new(NoopLayerRunner),
                Box::new(NoopFinalizationRunner),
                Box::new(NoopPostpassRunner),
                Box::new(emitter),
                Box::new(MinimalSerializer),
            ),
        )
    };

    assert!(
        config.anchored_entities.is_empty(),
        "AC-6 premise: the equivalence claim only holds on the support-free \
         (empty anchored-collection) path"
    );

    // `run_pipeline` → `run_pipeline_with_events`, the entry point TASK-400
    // recorded the baseline through and the one TASK-405 switched.
    run_pipeline(config).expect("support-free pipeline must succeed");

    let rows = captured_handle.lock().unwrap().clone();

    assert_eq!(
        rows.len(),
        BASELINE_ROW_COUNT,
        "committed path changed the support-free row COUNT: got {} rows, baseline recorded \
         {BASELINE_ROW_COUNT}; captured rows: [{}]",
        rows.len(),
        describe(&rows)
    );

    for (i, (baseline_index, baseline_z)) in BASELINE_ROWS.iter().enumerate() {
        assert_eq!(
            rows[i].global_layer_index,
            *baseline_index,
            "committed path changed row {i} global_layer_index: got {}, baseline recorded \
             {baseline_index}; captured rows: [{}]",
            rows[i].global_layer_index,
            describe(&rows)
        );
        assert!(
            (rows[i].z - *baseline_z).abs() < 1e-6,
            "committed path changed row {i} z: got {} mm, baseline recorded {baseline_z} mm; \
             captured rows: [{}]",
            rows[i].z,
            describe(&rows)
        );
    }
}

// ── AC-N3 ────────────────────────────────────────────────────────────────────

/// AC-N3 (support-disabled emits nothing): with `anchored_entities` empty and
/// support disabled — `grid_plan()` schedules no `Layer::Support*` stage at all
/// and the layer runner commits nothing — the new committed path must
/// synthesize ZERO anchored rows, and the emitted G-code must carry no
/// `;TYPE:Support` fragment.
///
/// "Zero synthesized rows" is asserted positionally: every captured row must sit
/// on a Z some global layer declared and carry no ordered entities, so no row
/// and no path was introduced by `synthesize_anchored_rows`.
///
/// The `;TYPE:Support` half needs an emitter/serializer pair that would actually
/// print the fragment if a support entity reached them — `MinimalSerializer`
/// returns an empty string, which would make the absence vacuous.
#[test]
fn support_disabled_pipeline_emits_no_anchored_rows() {
    /// Emits one `Move` per ordered entity, carrying that entity's role.
    struct RoleMoveEmitter;
    impl slicer_runtime::GCodeEmitter for RoleMoveEmitter {
        fn emit_gcode(&self, layer_irs: &[LayerCollectionIR]) -> Result<GCodeIR, GCodeEmitError> {
            let mut ir = GCodeIR::default();
            for row in layer_irs {
                for entity in &row.ordered_entities {
                    ir.commands.push(slicer_ir::GCodeCommand::Move {
                        x: None,
                        y: None,
                        z: Some(row.z),
                        e: None,
                        f: None,
                        role: entity.role.clone(),
                    });
                }
            }
            ir.metadata.layer_count = layer_irs.len() as u32;
            Ok(ir)
        }
    }

    /// Renders each `Move` role as a `;TYPE:<label>` line, collapsing the three
    /// support roles onto the canonical `Support` label the assertion greps for.
    struct TypeCommentSerializer;
    impl GCodeSerializer for TypeCommentSerializer {
        fn serialize_gcode(&self, gcode_ir: &GCodeIR) -> Result<String, GCodeEmitError> {
            let mut out = String::new();
            for command in &gcode_ir.commands {
                if let slicer_ir::GCodeCommand::Move { role, .. } = command {
                    match role {
                        slicer_ir::ExtrusionRole::SupportMaterial
                        | slicer_ir::ExtrusionRole::SupportInterface
                        | slicer_ir::ExtrusionRole::SupportBaseInterface => {
                            out.push_str(";TYPE:Support\n");
                        }
                        other => out.push_str(&format!(";TYPE:{other:?}\n")),
                    }
                }
            }
            Ok(out)
        }
    }

    // Non-vacuity check: the serializer under test really does print the
    // fragment when a support role reaches it, so its absence below is evidence.
    let support_probe = GCodeIR {
        commands: vec![slicer_ir::GCodeCommand::Move {
            x: None,
            y: None,
            z: Some(GRID_Z0_MM),
            e: None,
            f: None,
            role: slicer_ir::ExtrusionRole::SupportMaterial,
        }],
        ..Default::default()
    };
    assert!(
        TypeCommentSerializer
            .serialize_gcode(&support_probe)
            .unwrap()
            .contains(";TYPE:Support"),
        "the AC-N3 serializer must emit `;TYPE:Support` for a support role, otherwise the \
         absence assertion below is vacuous"
    );

    // Premise: the plan schedules no per-layer stage whatsoever, so no support
    // module can contribute. Asserted rather than assumed — if `grid_plan()`
    // ever grows a support stage, "support disabled" stops being true.
    assert!(
        grid_plan().per_layer_stages.is_empty(),
        "AC-N3 premise: the fixture plan must schedule no per-layer (support) stage"
    );

    // Two runs share one plan/prepass shape: one captures the row sequence, one
    // captures the serialized text (a single `GCodeEmitter` cannot do both).
    let emitter = common::CapturedRowsEmitter::new();
    let captured_handle = emitter.handle();

    let capture_config = PipelineConfig {
        ..common::pipeline_config_base(
            empty_mesh_ir(),
            grid_plan(),
            common::pipeline_stage_runners_base(
                Box::new(TwoLayerGridPrepass),
                Box::new(NoopLayerRunner),
                Box::new(NoopFinalizationRunner),
                Box::new(NoopPostpassRunner),
                Box::new(emitter),
                Box::new(MinimalSerializer),
            ),
        )
    };
    assert!(
        capture_config.anchored_entities.is_empty(),
        "AC-N3 premise: anchored_entities must be empty"
    );
    run_pipeline(capture_config).expect("support-disabled pipeline must succeed");

    let rows = captured_handle.lock().unwrap().clone();
    let grid_zs = [grid_z0_units(), grid_z1_units()];

    assert_eq!(
        rows.len(),
        grid_zs.len(),
        "support-disabled run synthesized extra rows: expected exactly the {} declared grid \
         rows, got {}; captured rows: [{}]",
        grid_zs.len(),
        rows.len(),
        describe(&rows)
    );
    for (i, row) in rows.iter().enumerate() {
        assert_eq!(
            slicer_ir::mm_to_units(row.z),
            grid_zs[i],
            "row {i} sits at a Z no global layer declared — it was synthesized; captured \
             rows: [{}]",
            describe(&rows)
        );
        assert!(
            row.ordered_entities.is_empty(),
            "row {i} carries anchored entities on the support-disabled path; captured rows: \
             [{}]",
            describe(&rows)
        );
    }

    let gcode_config = PipelineConfig {
        ..common::pipeline_config_base(
            empty_mesh_ir(),
            grid_plan(),
            common::pipeline_stage_runners_base(
                Box::new(TwoLayerGridPrepass),
                Box::new(NoopLayerRunner),
                Box::new(NoopFinalizationRunner),
                Box::new(NoopPostpassRunner),
                Box::new(RoleMoveEmitter),
                Box::new(TypeCommentSerializer),
            ),
        )
    };
    let output = run_pipeline(gcode_config).expect("support-disabled pipeline must succeed");

    assert!(
        !output.gcode_text.contains(";TYPE:Support"),
        "support-disabled run emitted a support fragment; gcode was:\n{}",
        output.gcode_text
    );
}
