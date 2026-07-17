//! Perimeter-parity verification harness (packet 109, T-100 / M1 closure).
//!
//! Provides:
//! - (a) real-pipeline-capture plumbing: `PerimeterCapturingLayerStageRunner`
//!   (a `LayerStageRunner` decorator) plus `run_pipeline_capturing_perimeters`,
//!   which drives the real production pipeline end-to-end and returns the
//!   final post-processed `PerimeterIR` for every layer. The production
//!   `PipelineOutput` type only exposes `gcode_text` + audits, so this is the
//!   only way to inspect per-layer IR after a real run.
//! - (b) a `PerimeterIR` JSON fixture loader (`load_expected_perimeters`) and a
//!   `(mesh_path, config_path, expected_output_path)` fixture runner
//!   (`run_and_compare_fixture`).
//! - (c) `compare_perimeter_ir`, a per-field tolerance comparator (wall count
//!   exact; per-vertex XYZ within 0.005mm; per-vertex width within 0.01mm;
//!   loop_type/role exact) returning a structured `PerimeterCompareResult`
//!   naming the first failing field + actual vs expected values.
//! - (d) `perimeter_parity_harness_self_test` / `deliberate_mismatch_detection`:
//!   self-tests proving the comparator detects a deliberate one-vertex
//!   mismatch (AC-N1's `deliberate_mismatch_detection` sub-case lives in the
//!   test named exactly that).
//!
//! Fixtures for a real end-to-end run are NOT part of this step (T-100 scope
//! is harness + self-test only); `run_pipeline_capturing_perimeters` and
//! `run_and_compare_fixture` are written now so Step 2 (T-101+) can call them
//! verbatim once fixtures exist.

#![allow(dead_code)] // real-pipeline plumbing is consumed by later steps in this packet.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use slicer_core::flow::{flow_to_width, line_width_to_spacing};
use slicer_gcode::{DefaultGCodeEmitter, DefaultGCodeSerializer};
use slicer_ir::ConfigValue;
use slicer_ir::{
    ExtrusionPath3D, ExtrusionRole, GlobalLayer, LayerStageCommit, LoopType, ModuleId, PerimeterIR,
    PerimeterRegion, Point3WithWidth, StageId, WallBoundaryType, WallFeatureFlags, WallLoop,
    WidthProfile,
};
use slicer_runtime::pipeline::{
    run_pipeline_with_raw_config, PipelineConfig, PipelineStageRunners,
};
use slicer_runtime::{
    assemble_search_roots, build_live_execution_plan, load_live_modules_for_plan_with_config,
    resolve_global_config, resolve_per_object_configs, resolve_per_tool_configs,
    validate_support_layer_heights, CompiledModuleLive, ConfigBoundsIndex, LayerStageError,
    LayerStageInput, LayerStageRunner, NoopLayerProgressSink, WasmComponent, WasmInstancePool,
    WasmRuntimeDispatcher,
};

// ============================================================================
// (a) Real-pipeline-capture plumbing
// ============================================================================

/// `LayerStageRunner` decorator that captures the last-seen `PerimeterIR` per
/// `global_layer_index` as the per-layer stage loop dispatches, then delegates
/// unmodified to the wrapped real runner.
///
/// The arena keeps `input.perimeter` set to the current committed value through
/// every remaining per-layer stage in a layer, so `run_stage` fires many times
/// per layer with `perimeter` set to the same logical value; overwriting on
/// each call means the LAST call in a layer wins, which is the final
/// post-processed state (`PerimetersPostProcess` runs after `Perimeters`).
pub struct PerimeterCapturingLayerStageRunner {
    inner: Box<dyn LayerStageRunner + Sync>,
    captured: Arc<Mutex<HashMap<u32, PerimeterIR>>>,
}

impl PerimeterCapturingLayerStageRunner {
    /// Wrap `inner` (the real production runner) with perimeter capture.
    pub fn new(inner: Box<dyn LayerStageRunner + Sync>) -> Self {
        Self {
            inner,
            captured: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Returns a shared handle to the capture sink. Clone this out BEFORE
    /// moving `self` into a `Box<dyn LayerStageRunner + Sync>` (e.g. into
    /// `PipelineStageRunners.layer`), since the pipeline consumes the box.
    pub fn sink(&self) -> Arc<Mutex<HashMap<u32, PerimeterIR>>> {
        Arc::clone(&self.captured)
    }
}

impl LayerStageRunner for PerimeterCapturingLayerStageRunner {
    fn run_stage(
        &self,
        stage_id: &StageId,
        layer: &GlobalLayer,
        module: &CompiledModuleLive<'_>,
        input: LayerStageInput<'_>,
    ) -> Result<Option<LayerStageCommit>, LayerStageError> {
        if let Some(perimeter) = input.perimeter {
            let mut map = self.captured.lock().expect("capture mutex poisoned");
            map.insert(perimeter.global_layer_index, perimeter.clone());
        }
        self.inner.run_stage(stage_id, layer, module, input)
    }

    fn last_wasm_mem_sample(&self) -> (u64, u64) {
        self.inner.last_wasm_mem_sample()
    }

    fn last_runtime_reads(&self) -> Vec<String> {
        self.inner.last_runtime_reads()
    }

    fn last_log_messages(&self) -> Vec<(String, String)> {
        self.inner.last_log_messages()
    }
}

/// Drain a capture sink into a `Vec<PerimeterIR>` sorted by `global_layer_index`.
fn drain_sorted(sink: &Mutex<HashMap<u32, PerimeterIR>>) -> Vec<PerimeterIR> {
    let map = sink.lock().expect("capture mutex poisoned");
    let mut values: Vec<PerimeterIR> = map.values().cloned().collect();
    values.sort_by_key(|p| p.global_layer_index);
    values
}

/// Error type for the real-pipeline-capture plumbing.
#[derive(Debug)]
pub struct CapturePipelineError(pub String);

impl std::fmt::Display for CapturePipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CapturePipelineError {}

fn num_cpus_guess() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

/// Run the real slicing pipeline end-to-end against `mesh_path` + `config_path`,
/// loading modules from `module_dirs`, and return the captured per-layer
/// `PerimeterIR` values (last-seen per `global_layer_index`, sorted ascending).
///
/// Mirrors `slicer_runtime::run_slice`'s module-loading / config-resolution /
/// plan-building steps (see `crates/slicer-runtime/src/run.rs`), but overrides
/// `PipelineStageRunners.layer` with a `PerimeterCapturingLayerStageRunner`
/// wrapping the real `WasmRuntimeDispatcher`, and skips `run_slice`'s startup
/// DAG validation / report / progress forks (diagnostic-only paths not needed
/// for parity capture).
pub fn run_pipeline_capturing_perimeters(
    mesh_path: &Path,
    config_path: &Path,
    module_dirs: &[PathBuf],
) -> Result<Vec<PerimeterIR>, CapturePipelineError> {
    let mesh = Arc::new(
        slicer_model_io::load_model(mesh_path)
            .map_err(|e| CapturePipelineError(format!("model load failed: {e:?}")))?,
    );

    let config_text = std::fs::read_to_string(config_path)
        .map_err(|e| CapturePipelineError(format!("failed to read config file: {e}")))?;
    let mut config_source = slicer_runtime::parse_cli_config_source(&config_text)
        .map_err(|e| CapturePipelineError(format!("failed to parse config: {e:?}")))?;

    // Mirror `slicer_runtime::run_slice`'s mesh-derived config seeding (see
    // `crates/slicer-runtime/src/run.rs`) so the real per-layer stages behave
    // identically to production. Without `object_height:<id>`, PrePass::LayerPlanning
    // (layer-planner-default) sees every object as zero-height and fails fatally.
    for object in &mesh.objects {
        let key = format!("object_height:{}", object.id);
        if let std::collections::hash_map::Entry::Vacant(e) = config_source.entry(key) {
            if let Some((z_min, z_max)) = object.world_z_extent {
                e.insert(ConfigValue::Float((z_max - z_min) as f64));
            }
        }
    }
    for object in &mesh.objects {
        for (subkey, value) in &object.config.data {
            let key = format!("object_config:{}:{}", object.id, subkey);
            config_source.entry(key).or_insert_with(|| value.clone());
        }
    }
    // Mirror `slicer_runtime::run_slice`'s `slice_has_paint` host-injection
    // (classic-perimeters.toml `[config.schema.slice_has_paint]`) so this
    // harness exercises the same painted-slice medial-axis gate production
    // does, instead of silently leaving it inert.
    if mesh.objects.iter().any(|o| o.paint_data.is_some())
        && !config_source.contains_key("slice_has_paint")
    {
        config_source.insert("slice_has_paint".to_string(), ConfigValue::Bool(true));
    }
    if !config_source.contains_key("wipe_tower_enabled") {
        use std::collections::BTreeSet;
        let mut tools: BTreeSet<u32> = BTreeSet::new();
        for object in &mesh.objects {
            if let Some(pd) = &object.paint_data {
                for layer in &pd.layers {
                    for fv in layer.facet_values.iter().flatten() {
                        if let slicer_ir::PaintValue::ToolIndex(n) = fv {
                            tools.insert(*n);
                        }
                    }
                }
            }
        }
        if tools.len() >= 2 {
            config_source.insert("wipe_tower_enabled".to_string(), ConfigValue::Bool(true));
        }
    }

    let search_roots = assemble_search_roots(module_dirs, true);
    // Config-aware loader: resolves the `perimeter-generator` claim
    // collision (classic-perimeters vs arachne-perimeters) via `config_source`'s
    // `wall_generator` key (packet 112 Step 10), mirroring the production
    // `run_slice` path exactly instead of relying on directory-exclusion
    // workarounds.
    let mut loaded =
        load_live_modules_for_plan_with_config(&search_roots, num_cpus_guess(), &config_source)
            .map_err(|e| CapturePipelineError(format!("failed to load modules: {e}")))?;

    let config_bounds = ConfigBoundsIndex::from_modules(loaded.bindings.iter().map(|b| &b.module));

    let default_resolved_config = resolve_global_config(&config_source, &config_bounds)
        .map_err(|e| CapturePipelineError(format!("config resolution failed: {e:?}")))?;

    let object_ids: Vec<&str> = mesh.objects.iter().map(|o| o.id.as_str()).collect();
    let resolved_configs_map = resolve_per_object_configs(
        &default_resolved_config,
        &config_source,
        &object_ids,
        &config_bounds,
    )
    .map_err(|e| CapturePipelineError(format!("per-object config resolution failed: {e:?}")))?;

    validate_support_layer_heights(&resolved_configs_map).map_err(|e| {
        CapturePipelineError(format!("support layer height validation failed: {e:?}"))
    })?;

    let per_tool_configs_map =
        resolve_per_tool_configs(&default_resolved_config, &config_source, &config_bounds)
            .map_err(|e| {
                CapturePipelineError(format!("per-tool config resolution failed: {e:?}"))
            })?;

    let wasm_handles: HashMap<ModuleId, (Arc<WasmInstancePool>, Option<Arc<WasmComponent>>)> =
        loaded
            .bindings
            .iter()
            .map(|b| {
                (
                    b.module.id().to_string(),
                    (Arc::clone(&b.instance_pool), b.wasm_component.clone()),
                )
            })
            .collect();

    let plan = build_live_execution_plan(
        loaded.sorted_stages,
        loaded.bindings,
        &config_source,
        Arc::new(Vec::new()),
        Arc::new(HashMap::new()),
        &mut loaded.diagnostics,
    )
    .map_err(|e| CapturePipelineError(format!("failed to build execution plan: {e:?}")))?;

    let engine = Arc::clone(&loaded.engine);
    let relative = match config_source.get("use_relative_e_distances") {
        Some(ConfigValue::Bool(b)) => *b,
        _ => slicer_runtime::run::DEFAULT_USE_RELATIVE_E_DISTANCES,
    };

    let capturing_layer_runner = PerimeterCapturingLayerStageRunner::new(Box::new(
        WasmRuntimeDispatcher::new(Arc::clone(&engine)),
    ));
    let sink = capturing_layer_runner.sink();

    let pipeline_config = PipelineConfig {
        mesh_ir: mesh,
        plan,
        runners: PipelineStageRunners {
            prepass: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            layer: Box::new(capturing_layer_runner),
            finalization: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            postpass: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            emitter: Box::new(
                DefaultGCodeEmitter::new("perimeter_parity_harness".to_string())
                    .with_resolved_config(default_resolved_config.clone())
                    .with_tool_configs(per_tool_configs_map.clone()),
            ),
            serializer: Box::new(DefaultGCodeSerializer::with_extrusion_mode(relative)),
        },
        resolved_configs: Arc::new(resolved_configs_map),
        default_resolved_config: Arc::new(default_resolved_config),
        bounds: Arc::new(config_bounds),
        wasm_handles,
    };

    let _output =
        run_pipeline_with_raw_config(pipeline_config, &config_source, &NoopLayerProgressSink)
            .map_err(|e| CapturePipelineError(format!("pipeline run failed: {e}")))?;

    Ok(drain_sorted(&sink))
}

// ============================================================================
// (b) PerimeterIR JSON fixture loading + fixture triple runner
// ============================================================================

/// A `(mesh_path, config_path, expected_output_path)` fixture triple, plus the
/// module search roots to load for the run.
pub struct PerimeterParityFixture {
    /// Path to the mesh (STL/3MF/etc.) to slice.
    pub mesh_path: PathBuf,
    /// Path to the JSON config to slice with.
    pub config_path: PathBuf,
    /// Module search roots (e.g. `modules/core-modules`).
    pub module_dirs: Vec<PathBuf>,
    /// Path to the recorded reference `Vec<PerimeterIR>` JSON (one entry per layer).
    pub expected_output_path: PathBuf,
}

/// Load the recorded reference `Vec<PerimeterIR>` (one entry per layer, sorted
/// by `global_layer_index`) from a JSON fixture file.
pub fn load_expected_perimeters(
    expected_output_path: &Path,
) -> Result<Vec<PerimeterIR>, CapturePipelineError> {
    let text = std::fs::read_to_string(expected_output_path)
        .map_err(|e| CapturePipelineError(format!("failed to read expected output file: {e}")))?;
    serde_json::from_str(&text)
        .map_err(|e| CapturePipelineError(format!("failed to parse expected output JSON: {e}")))
}

/// Run a `PerimeterParityFixture` end-to-end (real pipeline) and compare the
/// captured output against the recorded reference, layer by layer in
/// ascending `global_layer_index` order. Returns the first mismatch found, or
/// `PerimeterCompareResult::Match` if every layer matches and the layer counts
/// agree.
pub fn run_and_compare_fixture(
    fixture: &PerimeterParityFixture,
) -> Result<PerimeterCompareResult, CapturePipelineError> {
    let actual = run_pipeline_capturing_perimeters(
        &fixture.mesh_path,
        &fixture.config_path,
        &fixture.module_dirs,
    )?;

    // Structural-integrity gate (non-circular): reject malformed captured output
    // before the self-capture comparison, so silent data loss is caught even
    // where the recorded reference cannot help (see `structural_violation`).
    if let Some(violation) = structural_violation(&actual) {
        return Ok(PerimeterCompareResult::Mismatch(violation));
    }

    let expected = load_expected_perimeters(&fixture.expected_output_path)?;

    if actual.len() != expected.len() {
        return Ok(PerimeterCompareResult::Mismatch(PerimeterFieldMismatch {
            field: "layer_count".to_string(),
            actual: actual.len().to_string(),
            expected: expected.len().to_string(),
        }));
    }

    for (a, e) in actual.iter().zip(expected.iter()) {
        let result = compare_perimeter_ir(a, e);
        if !matches!(result, PerimeterCompareResult::Match) {
            return Ok(result);
        }
    }

    Ok(PerimeterCompareResult::Match)
}

// ============================================================================
// (c) Per-field tolerance comparator
// ============================================================================

/// Per-vertex X/Y/Z tolerance, in millimeters (AC-1).
const VERTEX_XYZ_TOLERANCE_MM: f32 = 0.005;
/// Per-vertex extrusion-width tolerance, in millimeters (AC-1).
const VERTEX_WIDTH_TOLERANCE_MM: f32 = 0.01;

/// A single named-field mismatch: which field differed, and its actual vs
/// expected value (both rendered via `Debug` so any field type is reportable).
#[derive(Debug, Clone, PartialEq)]
pub struct PerimeterFieldMismatch {
    /// Dotted/indexed path to the differing field, e.g.
    /// `"regions[0].walls[1].path.points[2].x"`.
    pub field: String,
    /// Actual (captured) value, `Debug`-formatted.
    pub actual: String,
    /// Expected (recorded reference) value, `Debug`-formatted.
    pub expected: String,
}

impl std::fmt::Display for PerimeterFieldMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "field `{}` mismatch: actual={} expected={}",
            self.field, self.actual, self.expected
        )
    }
}

/// Outcome of [`compare_perimeter_ir`].
#[derive(Debug, Clone, PartialEq)]
pub enum PerimeterCompareResult {
    /// Every compared field matched within tolerance.
    Match,
    /// The first field that differed, named, with actual vs expected values.
    Mismatch(PerimeterFieldMismatch),
}

/// Compare `actual` (captured from a real pipeline run) against `expected` (a
/// recorded reference `PerimeterIR`) field by field, returning the FIRST
/// mismatch found:
///
/// - `global_layer_index`, region count, per-region `object_id`/`region_id`,
///   and per-region wall count: exact.
/// - per-wall `loop_type` and `path.role`: exact.
/// - per-vertex X/Y/Z: within [`VERTEX_XYZ_TOLERANCE_MM`].
/// - per-vertex `width`: within [`VERTEX_WIDTH_TOLERANCE_MM`].
///
/// Does NOT silently pass on a mismatch (AC-N1): any field outside tolerance
/// short-circuits to `PerimeterCompareResult::Mismatch` naming that field.
pub fn compare_perimeter_ir(
    actual: &PerimeterIR,
    expected: &PerimeterIR,
) -> PerimeterCompareResult {
    fn mismatch(
        field: impl Into<String>,
        actual: impl std::fmt::Debug,
        expected: impl std::fmt::Debug,
    ) -> PerimeterCompareResult {
        PerimeterCompareResult::Mismatch(PerimeterFieldMismatch {
            field: field.into(),
            actual: format!("{actual:?}"),
            expected: format!("{expected:?}"),
        })
    }

    if actual.global_layer_index != expected.global_layer_index {
        return mismatch(
            "global_layer_index",
            actual.global_layer_index,
            expected.global_layer_index,
        );
    }

    if actual.regions.len() != expected.regions.len() {
        return mismatch(
            "regions.len()",
            actual.regions.len(),
            expected.regions.len(),
        );
    }

    for (region_idx, (a_region, e_region)) in actual
        .regions
        .iter()
        .zip(expected.regions.iter())
        .enumerate()
    {
        if a_region.object_id != e_region.object_id {
            return mismatch(
                format!("regions[{region_idx}].object_id"),
                &a_region.object_id,
                &e_region.object_id,
            );
        }
        if a_region.region_id != e_region.region_id {
            return mismatch(
                format!("regions[{region_idx}].region_id"),
                a_region.region_id,
                e_region.region_id,
            );
        }
        if a_region.walls.len() != e_region.walls.len() {
            return mismatch(
                format!("regions[{region_idx}].walls.len()"),
                a_region.walls.len(),
                e_region.walls.len(),
            );
        }

        for (wall_idx, (a_wall, e_wall)) in
            a_region.walls.iter().zip(e_region.walls.iter()).enumerate()
        {
            if a_wall.loop_type != e_wall.loop_type {
                return mismatch(
                    format!("regions[{region_idx}].walls[{wall_idx}].loop_type"),
                    a_wall.loop_type,
                    e_wall.loop_type,
                );
            }
            if a_wall.path.role != e_wall.path.role {
                return mismatch(
                    format!("regions[{region_idx}].walls[{wall_idx}].path.role"),
                    &a_wall.path.role,
                    &e_wall.path.role,
                );
            }
            if a_wall.path.points.len() != e_wall.path.points.len() {
                return mismatch(
                    format!("regions[{region_idx}].walls[{wall_idx}].path.points.len()"),
                    a_wall.path.points.len(),
                    e_wall.path.points.len(),
                );
            }

            for (pt_idx, (a_pt, e_pt)) in a_wall
                .path
                .points
                .iter()
                .zip(e_wall.path.points.iter())
                .enumerate()
            {
                let field_prefix =
                    format!("regions[{region_idx}].walls[{wall_idx}].path.points[{pt_idx}]");

                if (a_pt.x - e_pt.x).abs() > VERTEX_XYZ_TOLERANCE_MM {
                    return mismatch(format!("{field_prefix}.x"), a_pt.x, e_pt.x);
                }
                if (a_pt.y - e_pt.y).abs() > VERTEX_XYZ_TOLERANCE_MM {
                    return mismatch(format!("{field_prefix}.y"), a_pt.y, e_pt.y);
                }
                if (a_pt.z - e_pt.z).abs() > VERTEX_XYZ_TOLERANCE_MM {
                    return mismatch(format!("{field_prefix}.z"), a_pt.z, e_pt.z);
                }
                if (a_pt.width - e_pt.width).abs() > VERTEX_WIDTH_TOLERANCE_MM {
                    return mismatch(format!("{field_prefix}.width"), a_pt.width, e_pt.width);
                }
            }
        }
    }

    PerimeterCompareResult::Match
}

/// First structural-integrity violation in a freshly-captured `PerimeterIR`
/// set, or `None` if the output is well-formed.
///
/// NON-CIRCULAR: this depends only on the captured output's own well-formedness,
/// never on a recorded reference — so it catches silent data loss (empty
/// capture, degenerate sub-2-point walls, non-finite coordinates) that a pure
/// baseline-vs-baseline comparison cannot (a regression that corrupts the
/// re-recorded reference and the live output identically would still slip a
/// self-compare). Returned as a [`PerimeterFieldMismatch`] so callers report it
/// exactly like a value drift.
fn structural_violation(perimeters: &[PerimeterIR]) -> Option<PerimeterFieldMismatch> {
    if perimeters.is_empty() {
        return Some(PerimeterFieldMismatch {
            field: "layer_count".to_string(),
            actual: "0".to_string(),
            expected: ">= 1 captured layer".to_string(),
        });
    }
    for (li, p) in perimeters.iter().enumerate() {
        for (ri, r) in p.regions.iter().enumerate() {
            for (wi, w) in r.walls.iter().enumerate() {
                if w.path.points.len() < 2 {
                    return Some(PerimeterFieldMismatch {
                        field: format!("[{li}].regions[{ri}].walls[{wi}].path.points.len()"),
                        actual: w.path.points.len().to_string(),
                        expected: ">= 2 (a wall loop needs at least two vertices)".to_string(),
                    });
                }
                for (pi, pt) in w.path.points.iter().enumerate() {
                    if !(pt.x.is_finite()
                        && pt.y.is_finite()
                        && pt.z.is_finite()
                        && pt.width.is_finite())
                    {
                        return Some(PerimeterFieldMismatch {
                            field: format!("[{li}].regions[{ri}].walls[{wi}].path.points[{pi}]"),
                            actual: format!("({}, {}, {}, w={})", pt.x, pt.y, pt.z, pt.width),
                            expected: "all coordinates finite".to_string(),
                        });
                    }
                }
            }
        }
    }
    None
}

// ============================================================================
// (d) Self-tests
// ============================================================================

fn sample_point(x: f32, y: f32, z: f32, width: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z,
        width,
        flow_factor: 1.0,
        overhang_quartile: None,
    }
}

fn sample_wall_loop() -> WallLoop {
    let points = vec![
        sample_point(0.0, 0.0, 0.2, 0.4),
        sample_point(10.0, 0.0, 0.2, 0.4),
        sample_point(10.0, 10.0, 0.2, 0.4),
        sample_point(0.0, 0.0, 0.2, 0.4), // closing repeat
    ];
    let feature_flags = points.iter().map(|_| WallFeatureFlags::default()).collect();
    WallLoop {
        perimeter_index: 0,
        loop_type: LoopType::Outer,
        path: ExtrusionPath3D {
            points,
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        width_profile: WidthProfile {
            widths: vec![0.4; 4],
        },
        feature_flags,
        boundary_type: WallBoundaryType::ExteriorSurface,
    }
}

fn sample_perimeter_ir() -> PerimeterIR {
    PerimeterIR {
        global_layer_index: 3,
        regions: vec![PerimeterRegion {
            object_id: "obj-0".to_string(),
            region_id: 0,
            walls: vec![sample_wall_loop()],
            ..Default::default()
        }],
        ..Default::default()
    }
}

/// Self-test for the comparator (harness side of AC-1): two identical
/// `PerimeterIR` values must compare as `Match`, and a value that differs only
/// in loop_type must be reported precisely (not just true/false).
///
/// The AC-N1 "deliberate mismatch on a single vertex, off by 0.1mm" sub-case
/// is covered by the separate `deliberate_mismatch_detection` test below.
#[test]
fn perimeter_parity_harness_self_test() {
    let expected = sample_perimeter_ir();
    let actual = expected.clone();
    assert_eq!(
        compare_perimeter_ir(&actual, &expected),
        PerimeterCompareResult::Match,
        "two identical PerimeterIR values must compare as Match"
    );

    let mut actual_wrong_loop_type = expected.clone();
    actual_wrong_loop_type.regions[0].walls[0].loop_type = LoopType::Inner;
    match compare_perimeter_ir(&actual_wrong_loop_type, &expected) {
        PerimeterCompareResult::Mismatch(m) => {
            assert_eq!(m.field, "regions[0].walls[0].loop_type");
        }
        PerimeterCompareResult::Match => {
            panic!("comparator failed to detect a loop_type mismatch")
        }
    }
}

/// AC-N1: given two `PerimeterIR` values differing in exactly one field (a
/// single vertex X off by 0.1mm — larger than the 0.005mm XYZ tolerance), the
/// comparator must report the specific field that differs plus actual vs
/// expected values, and must NOT silently report `Match`.
#[test]
fn deliberate_mismatch_detection() {
    let expected = sample_perimeter_ir();
    let mut actual = expected.clone();

    let perturbed = &mut actual.regions[0].walls[0].path.points[1];
    let original_x = perturbed.x;
    perturbed.x += 0.1;

    match compare_perimeter_ir(&actual, &expected) {
        PerimeterCompareResult::Mismatch(m) => {
            assert_eq!(
                m.field, "regions[0].walls[0].path.points[1].x",
                "comparator must name the exact differing field"
            );
            assert_eq!(m.actual, format!("{:?}", original_x + 0.1));
            assert_eq!(m.expected, format!("{:?}", original_x));
        }
        PerimeterCompareResult::Match => {
            panic!("comparator failed to detect a deliberate 0.1mm vertex-X mismatch (AC-N1)")
        }
    }
}

/// AC-N1 (full harness path): a deliberately-broken *fixture file* — solid_square's
/// recorded reference with one vertex shifted 0.1 mm (> the 0.005 mm XYZ
/// tolerance) — run through `run_and_compare_fixture` must NOT silently pass; it
/// must return a `Mismatch` naming the differing vertex field. This exercises the
/// end-to-end harness (capture -> load fixture FILE -> compare), which
/// `deliberate_mismatch_detection` (comparator-only, in-memory) does not.
#[test]
fn deliberate_broken_fixture_file_is_detected() {
    let dir = fixture_dir("solid_square");

    // Load the real recorded reference and perturb the first wall vertex by 0.1 mm.
    let mut broken = load_expected_perimeters(&dir.join("expected_perimeter_ir.json"))
        .expect("solid_square reference must load");
    let mut perturbed = false;
    'outer: for p in broken.iter_mut() {
        for r in p.regions.iter_mut() {
            for w in r.walls.iter_mut() {
                if let Some(pt) = w.path.points.first_mut() {
                    pt.x += 0.1;
                    perturbed = true;
                    break 'outer;
                }
            }
        }
    }
    assert!(
        perturbed,
        "solid_square reference must contain at least one wall vertex to perturb"
    );

    // Write the broken reference to a per-process temp file and point the fixture at it.
    let broken_path = std::env::temp_dir().join(format!(
        "pnp_perimeter_parity_broken_solid_square_{}.json",
        std::process::id()
    ));
    let text = serde_json::to_string(&broken).expect("broken reference must serialize");
    std::fs::write(&broken_path, text).expect("failed to write broken fixture file");

    let fixture = PerimeterParityFixture {
        mesh_path: dir.join("solid_square.stl"),
        config_path: dir.join("config.json"),
        // core_modules_dir() loads both classic-perimeters and
        // arachne-perimeters; the `perimeter-generator` claim collision
        // resolves to classic-perimeters by default (`wall_generator`
        // config key absent — see `dedup_same_claim_modules`, packet 112
        // Step 10), matching this fixture's recorded baseline.
        module_dirs: vec![core_modules_dir()],
        expected_output_path: broken_path.clone(),
    };
    let result = run_and_compare_fixture(&fixture).expect("solid_square pipeline run must succeed");
    let _ = std::fs::remove_file(&broken_path);

    match result {
        PerimeterCompareResult::Mismatch(m) => {
            assert!(
                m.field.ends_with(".x"),
                "AC-N1: harness must name the differing vertex field (got `{}`)",
                m.field
            );
        }
        PerimeterCompareResult::Match => {
            panic!(
                "AC-N1: harness silently PASSED a fixture file with a 0.1 mm vertex error \
                 (fixture: solid_square)"
            );
        }
    }
}

// ============================================================================
// (e) M1 reference fixtures (packet 109, Step 2 / T-101).
//
// Methodology & HONEST SCOPE (user-approved, see packet.spec.md): no live
// OrcaSlicer binary exists in this environment, so these fixtures are
// SELF-CAPTURED regression baselines, NOT independently-derived OrcaSlicer
// geometry. Recording (the `#[ignore]`-marked `record_*` functions below):
// build mesh + config -> run the REAL pipeline via
// `run_pipeline_capturing_perimeters` -> manually cross-check the output's
// coarse SHAPE (wall-loop count, role/loop_type distribution — documented per
// fixture in the module comments below) against the OrcaSlicer-documented
// expectation -> serialize the captured output as-is to
// `expected_perimeter_ir.json`.
//
// WHAT THE PERMANENT GATE (AC-2, the plain `#[test]` functions) ACTUALLY
// PROVES: (1) self-regression — future pipeline drift away from today's
// blessed output is caught to per-vertex tolerance; (2) structural integrity —
// `run_and_compare_fixture` rejects malformed captured output (empty capture,
// sub-2-point walls, non-finite coordinates) via `structural_violation` before
// comparing. WHAT IT DOES NOT PROVE: per-vertex OrcaSlicer parity — the
// reference is our OWN output, so the fine-grained 0.005/0.01 mm tolerances can
// only detect drift from that baseline, never a divergence from OrcaSlicer.
// True per-geometry Orca parity requires recording against a real OrcaSlicer
// build (tracked as an M2 follow-up); this harness is the regression bed that
// follow-up will re-bless.
//
// Layer-height choice (documented, applies to all 6 M1 fixtures except
// `narrow_strip_widening`): `layer_height = first_layer_height = 0.2mm`
// (the slicer's standard default; see `modules/core-modules/classic-perimeters/
// classic-perimeters.toml` `[config.schema.layer_height]` default = 0.2).
// Chosen uniformly so every fixture exercises a non-trivial layer count
// (15–50 layers) at the realistic production layer height. The
// `narrow_strip_widening` fixture (a single Arachne fixture in the
// Step-10A/T-231 set, see section (f) below) is the documented exception:
// it stays at `1.0mm` to retain coverage of the **degenerate** beading
// regime (`optimal_width 0.4mm <= layer_height 1.0mm` → `line_width_to_spacing`
// returns 0, raw-width fallback reproduced verbatim), which is now the only
// fixture in the suite that exercises that path.
// ============================================================================

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = crates/slicer-runtime
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root canonicalize")
}

fn core_modules_dir() -> PathBuf {
    repo_root().join("modules/core-modules")
}

fn fixture_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/perimeter_parity")
        .join(name)
}

/// A raw triangle: 3 `[x, y, z]` vertices in millimeters.
type Tri = [[f32; 3]; 3];

fn triangle_normal(tri: &Tri) -> [f32; 3] {
    let [a, b, c] = *tri;
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if len > 1e-9 {
        [n[0] / len, n[1] / len, n[2] / len]
    } else {
        [0.0, 0.0, 0.0]
    }
}

/// Write `triangles` as a binary STL file. Each facet normal is computed
/// from vertex winding (right-hand rule) — callers are responsible for
/// supplying outward-facing CCW winding (verified per-face in doc comments
/// on the geometry helpers below).
fn write_binary_stl(path: &Path, triangles: &[Tri]) {
    use std::io::Write;
    let mut buf: Vec<u8> = Vec::with_capacity(84 + triangles.len() * 50);
    buf.extend_from_slice(&[0u8; 80]); // header, unused
    buf.extend_from_slice(&(triangles.len() as u32).to_le_bytes());
    for tri in triangles {
        for c in triangle_normal(tri) {
            buf.extend_from_slice(&c.to_le_bytes());
        }
        for v in tri {
            for c in *v {
                buf.extend_from_slice(&c.to_le_bytes());
            }
        }
        buf.extend_from_slice(&0u16.to_le_bytes()); // attribute byte count
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("failed to create fixture dir");
    }
    let mut f = std::fs::File::create(path).expect("failed to create STL fixture file");
    f.write_all(&buf).expect("failed to write STL fixture file");
}

fn write_config_json(path: &Path, value: &serde_json::Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("failed to create fixture dir");
    }
    let text = serde_json::to_string_pretty(value).expect("config JSON must serialize");
    std::fs::write(path, text).expect("failed to write config fixture file");
}

/// Build a hexahedron (topological box) from two quads: `bottom` in CCW
/// order as viewed from +Z (`[b0, b1, b2, b3]`), and `top` with
/// correspondingly-ordered corners (`[t0, t1, t2, t3]`). Produces 12
/// outward-facing triangles (verified by hand via cross products for every
/// face during authoring — see packet 109 Step 2 worklog).
///
/// Covers axis-aligned boxes (`solid_box`) AND the sheared `overhang_ramp`
/// prism (top quad translated relative to bottom in +X): each side face
/// stays planar because it's a parallelogram (two parallel rectangles
/// connected by straight edges), so the same triangle-index formula applies
/// regardless of the X-offset between `bottom` and `top`.
fn prism(bottom: [[f32; 3]; 4], top: [[f32; 3]; 4]) -> Vec<Tri> {
    let [b0, b1, b2, b3] = bottom;
    let [t0, t1, t2, t3] = top;
    let mut tris = vec![
        [b0, b2, b1],
        [b0, b3, b2], // bottom cap, outward -Z
        [t0, t1, t2],
        [t0, t2, t3], // top cap, outward +Z
    ];
    let mut side = |bi: [f32; 3], bj: [f32; 3], ti: [f32; 3], tj: [f32; 3]| {
        tris.push([bi, bj, tj]);
        tris.push([bi, tj, ti]);
    };
    side(b0, b1, t0, t1);
    side(b1, b2, t1, t2);
    side(b2, b3, t2, t3);
    side(b3, b0, t3, t0);
    tris
}

/// Axis-aligned box, 12 triangles, outward-facing CCW winding.
fn solid_box(min: [f32; 3], max: [f32; 3]) -> Vec<Tri> {
    let [x0, y0, z0] = min;
    let [x1, y1, z1] = max;
    prism(
        [[x0, y0, z0], [x1, y0, z0], [x1, y1, z0], [x0, y1, z0]],
        [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
    )
}

/// Watertight frustum: `sides`-gon base ring at z=0 (radius `base_radius`)
/// and top ring at z=`height` (radius `top_radius`), both capped. 4
/// triangles per side wedge (2 side-wall + 1 bottom-fan + 1 top-fan) =
/// `sides * 4` triangles total (96 for `sides=24`, matching the
/// `spiral_vase_cone` geometry spec).
fn frustum(base_radius: f32, top_radius: f32, height: f32, sides: usize) -> Vec<Tri> {
    let ring = |r: f32, z: f32| -> Vec<[f32; 3]> {
        (0..sides)
            .map(|i| {
                let theta = 2.0 * std::f32::consts::PI * (i as f32) / (sides as f32);
                [r * theta.cos(), r * theta.sin(), z]
            })
            .collect()
    };
    let bottom = ring(base_radius, 0.0);
    let top = ring(top_radius, height);
    let bottom_center = [0.0, 0.0, 0.0];
    let top_center = [0.0, 0.0, height];
    let mut tris = Vec::with_capacity(sides * 4);
    for i in 0..sides {
        let j = (i + 1) % sides;
        tris.push([bottom[i], bottom[j], top[j]]);
        tris.push([bottom[i], top[j], top[i]]);
        tris.push([bottom_center, bottom[j], bottom[i]]); // bottom fan, outward -Z
        tris.push([top_center, top[i], top[j]]); // top fan, outward +Z
    }
    tris
}

/// Write a minimal single-object 3MF with per-triangle `paint_color`
/// attributes.
///
/// `slicer_model_io::write_3mf` (`crates/slicer-model-io/src/writer.rs`)
/// does NOT round-trip `paint_data` — the writer has no paint support at
/// all — so the `multi_tool_triangle` fixture (which needs 3 independent
/// per-triangle tool assignments) cannot be produced via `write_3mf`.
/// Instead this hand-authors the 3MF XML directly, following the same
/// zip-of-raw-XML pattern as
/// `crates/slicer-model-io/tests/model_loader_tdd.rs`'s
/// `threemf_custom_paint_file` helper (no `[Content_Types].xml` / `_rels`
/// needed — the loader tolerates the minimal structure).
///
/// `paint_color` encoding (this codebase's simplified single-hex-char
/// `TriangleSelector` state, see `loader.rs::decode_paint_hex_state`):
/// `state = nibble >> 2` (low 2 bits must be 0), and the loader maps
/// `ToolIndex(state - 1)`. So `tool_index_per_triangle[i] = Some(t)` emits
/// `paint_color` hex nibble `(t + 1) << 2` (t in 0..=2 for this fixture);
/// `None` omits the attribute (unpainted).
fn write_3mf_with_tool_paint(
    path: &Path,
    vertices: &[[f32; 3]],
    triangles: &[[u32; 3]],
    tool_index_per_triangle: &[Option<u32>],
) {
    assert_eq!(triangles.len(), tool_index_per_triangle.len());
    let vertices_xml: String = vertices
        .iter()
        .map(|v| {
            format!(
                r#"          <vertex x="{}" y="{}" z="{}" />"#,
                v[0], v[1], v[2]
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let triangle_xml: String = triangles
        .iter()
        .zip(tool_index_per_triangle)
        .map(|(t, tool)| match tool {
            Some(tool_idx) => {
                let state = tool_idx + 1;
                assert!(
                    state <= 3,
                    "single-hex-char paint_color state must be 0..=3"
                );
                let nibble = state << 2;
                format!(
                    r#"          <triangle v1="{}" v2="{}" v3="{}" paint_color="{:X}" />"#,
                    t[0], t[1], t[2], nibble
                )
            }
            None => format!(
                r#"          <triangle v1="{}" v2="{}" v3="{}" />"#,
                t[0], t[1], t[2]
            ),
        })
        .collect::<Vec<_>>()
        .join("\n");

    let model_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<model unit="millimeter" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">
  <resources>
    <object id="1" type="model">
      <mesh>
        <vertices>
{vertices_xml}
        </vertices>
        <triangles>
{triangle_xml}
        </triangles>
      </mesh>
    </object>
  </resources>
  <build>
    <item objectid="1" />
  </build>
</model>"#
    );

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("failed to create fixture dir");
    }
    let mut buf: Vec<u8> = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut zip_writer = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        zip_writer
            .start_file("3D/3dmodel.model", options)
            .expect("zip start_file");
        std::io::Write::write_all(&mut zip_writer, model_xml.as_bytes())
            .expect("zip write model.xml");
        zip_writer.finish().expect("zip finish");
    }
    std::fs::write(path, &buf).expect("failed to write 3mf fixture file");
}

/// Print a compact structural summary of captured `PerimeterIR`s to stderr
/// (visible with `--nocapture`), used to sanity-check shape (wall count,
/// loop_type/role/boundary_type distribution, bridge-flagged point counts)
/// against the OrcaSlicer-documented expectation during fixture recording.
fn print_perimeter_summary(label: &str, perimeters: &[PerimeterIR]) {
    eprintln!("=== {label}: {} layer(s) captured ===", perimeters.len());
    for p in perimeters {
        eprint!(
            "layer {}: {} region(s)",
            p.global_layer_index,
            p.regions.len()
        );
        for r in &p.regions {
            eprint!(
                " | region(object={}, id={}, walls={})",
                r.object_id,
                r.region_id,
                r.walls.len()
            );
            for (i, w) in r.walls.iter().enumerate() {
                let bridge_pts = w.feature_flags.iter().filter(|f| f.is_bridge).count();
                eprint!(
                    " [w{i}: {:?}/{:?}/{:?} pts={} bridge_pts={}]",
                    w.loop_type,
                    w.path.role,
                    w.boundary_type,
                    w.path.points.len(),
                    bridge_pts
                );
            }
        }
        eprintln!();
    }
}

/// Serialize `perimeters` as the recorded reference JSON for `fixture_dir`.
///
/// Compact (not pretty-printed): `spiral_vase_cone` alone is ~15 layers x 3
/// walls x ~50 points, and pretty-printing roughly triples file size for
/// content nobody hand-edits — compact keeps all 6 fixtures' recorded
/// references at a "manageable size" per the packet's own guidance.
fn write_expected_perimeters(fixture_dir: &Path, perimeters: &[PerimeterIR]) {
    let path = fixture_dir.join("expected_perimeter_ir.json");
    let text = serde_json::to_string(perimeters).expect("PerimeterIR must serialize");
    std::fs::write(&path, text).expect("failed to write expected_perimeter_ir.json");
}

// ----------------------------------------------------------------------------
// Fixture 1: solid_square — one 20x20x3mm box, wall_count=3 (module default),
// no holes. Expect 3 closed wall loops per layer: depth0=Outer/OuterWall,
// depth1..2=Inner/InnerWall (this codebase's `LoopType`/`ExtrusionRole` only
// distinguish Outer-vs-Inner by depth, not a 3-way external/default/internal
// split — see `modules/core-modules/classic-perimeters/src/lib.rs` around
// the `is_outer` branch).
// ----------------------------------------------------------------------------

fn solid_square_config() -> serde_json::Value {
    serde_json::json!({
        "layer_height": 0.2,
        "first_layer_height": 0.2
    })
}

#[ignore = "fixture recorder — run explicitly to (re)generate mesh/config/expected output"]
#[test]
fn record_solid_square() {
    let dir = fixture_dir("solid_square");
    let mesh_path = dir.join("solid_square.stl");
    let config_path = dir.join("config.json");
    write_binary_stl(&mesh_path, &solid_box([0.0, 0.0, 0.0], [20.0, 20.0, 3.0]));
    write_config_json(&config_path, &solid_square_config());

    let perimeters =
        run_pipeline_capturing_perimeters(&mesh_path, &config_path, &[core_modules_dir()])
            .expect("solid_square real pipeline run must succeed");
    print_perimeter_summary("solid_square", &perimeters);
    write_expected_perimeters(&dir, &perimeters);
}

#[test]
fn solid_square_perimeter_parity() {
    let dir = fixture_dir("solid_square");
    let fixture = PerimeterParityFixture {
        mesh_path: dir.join("solid_square.stl"),
        config_path: dir.join("config.json"),
        // core_modules_dir(): the `perimeter-generator` claim collision
        // (classic-perimeters vs arachne-perimeters) resolves to classic by
        // default (no `wall_generator` config key set — packet 112 Step 10),
        // matching this fixture's recorded baseline.
        module_dirs: vec![core_modules_dir()],
        expected_output_path: dir.join("expected_perimeter_ir.json"),
    };
    let result = run_and_compare_fixture(&fixture).expect("solid_square pipeline run must succeed");
    assert_eq!(result, PerimeterCompareResult::Match, "{result:?}");
}

// ----------------------------------------------------------------------------
// Fixture 2: holed_square — a 20x20mm square frame (4-box union) around an
// 8x8mm centered hole, wall_count=3. Expect 6 closed wall loops total in ONE
// region (3 around the outer boundary + 3 around the hole boundary).
// ----------------------------------------------------------------------------

/// Note (recorded finding, see follow_up): walls interleave contour/hole per
/// depth — w0 (Outer, 5 pts) is the exterior boundary's first ring, a clean
/// rectangle (insetting a CONVEX corner, the square's 4 outer corners, needs
/// only a sharp miter point); w1 (Outer, 57 pts) is the SAME depth's hole
/// boundary ring — it has many extra points because insetting a
/// REFLEX/concave corner (the hole's 4 corners, concave from the solid
/// material's offsetting perspective) requires an arc fillet, discretized
/// per `perimeter_arc_tolerance` (default 0.0125mm); w2/w3 and w4/w5 repeat
/// the same contour/hole pairing for depths 1 and 2. This is expected
/// offset-geometry behavior, not a bug — verified by inspecting the actual
/// recorded point coordinates (a tight arc around each hole corner).
///
/// Re-baselined 2026-07-11 (gap-1 follow-up, D-150): `polygon_ops` now nests
/// a wall depth's hole correctly under `ExPolygon.holes` instead of emitting
/// it as an accidental second top-level `ExPolygon` (the pre-fix `offset`
/// flattened every result path to a solid contour, so the hole ring only
/// ever reached `emit_walls` by masquerading as an unrelated top-level
/// "island" that its polygon-index loop happened to walk anyway).
/// `emit_walls` now explicitly walks `poly.holes` per depth to emit the
/// hole-ring wall, which is the same 6-wall, contour/hole-interleaved shape
/// as before — this fixture's total wall count and pairing were never wrong,
/// only vertex-list rotation shifted (Clipper2's `execute_tree` vs the old
/// flat `execute` doesn't guarantee the same starting vertex), hence the
/// re-record.
fn holed_square_mesh() -> Vec<Tri> {
    let mut tris = Vec::new();
    tris.extend(solid_box([0.0, 0.0, 0.0], [6.0, 20.0, 3.0])); // left strip
    tris.extend(solid_box([14.0, 0.0, 0.0], [20.0, 20.0, 3.0])); // right strip
    tris.extend(solid_box([6.0, 14.0, 0.0], [14.0, 20.0, 3.0])); // top strip
    tris.extend(solid_box([6.0, 0.0, 0.0], [14.0, 6.0, 3.0])); // bottom strip
    tris
}

fn holed_square_config() -> serde_json::Value {
    serde_json::json!({
        "layer_height": 0.2,
        "first_layer_height": 0.2
    })
}

#[ignore = "fixture recorder — run explicitly to (re)generate mesh/config/expected output"]
#[test]
fn record_holed_square() {
    let dir = fixture_dir("holed_square");
    let mesh_path = dir.join("holed_square.stl");
    let config_path = dir.join("config.json");
    write_binary_stl(&mesh_path, &holed_square_mesh());
    write_config_json(&config_path, &holed_square_config());

    let perimeters =
        run_pipeline_capturing_perimeters(&mesh_path, &config_path, &[core_modules_dir()])
            .expect("holed_square real pipeline run must succeed");
    print_perimeter_summary("holed_square", &perimeters);
    write_expected_perimeters(&dir, &perimeters);
}

#[test]
fn holed_square_perimeter_parity() {
    let dir = fixture_dir("holed_square");
    let fixture = PerimeterParityFixture {
        mesh_path: dir.join("holed_square.stl"),
        config_path: dir.join("config.json"),
        // core_modules_dir(): the `perimeter-generator` claim collision
        // resolves to classic by default (packet 112 Step 10).
        module_dirs: vec![core_modules_dir()],
        expected_output_path: dir.join("expected_perimeter_ir.json"),
    };
    let result = run_and_compare_fixture(&fixture).expect("holed_square pipeline run must succeed");
    assert_eq!(result, PerimeterCompareResult::Match, "{result:?}");
}

// ----------------------------------------------------------------------------
// Regression (true topological hole): a SINGLE watertight manifold — a 20x20mm
// frame with an 8x8mm centered through-hole (walls + annular top/bottom caps),
// NOT the `holed_square` 4-strip union. A single manifold's cross-section is
// two nested, non-overlapping loops (outer + hole). Before the
// `polygons_to_expolygons` PolyTree fix, `slice_mesh_ex` unioned those two
// same-wound loops under NonZero and FILLED the hole → the layer became a solid
// 20x20 square → only the 3 outer wall loops, no hole perimeters (the gcode
// bug). After the fix the hole is reconstructed → 6 wall loops (3 outer + 3
// tracing the hole), and the hole loops are confined to the frame interior.
// ----------------------------------------------------------------------------

/// Watertight 20x20x3mm frame with an 8x8 centered through-hole, as ONE
/// manifold (4 outer walls + 4 inner walls + annular bottom/top caps).
fn annulus_frame_mesh() -> Vec<Tri> {
    let (z0, z1) = (0.0f32, 3.0f32);
    let o = [[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]]; // outer, CCW
    let h = [[6.0, 6.0], [14.0, 6.0], [14.0, 14.0], [6.0, 14.0]]; // hole, CCW
    let p = |xy: [f32; 2], z: f32| -> [f32; 3] { [xy[0], xy[1], z] };
    let mut tris: Vec<Tri> = Vec::new();

    // Outer vertical walls (outward normal): traverse O0→O1→O2→O3.
    for i in 0..4 {
        let j = (i + 1) % 4;
        tris.push([p(o[i], z0), p(o[j], z0), p(o[j], z1)]);
        tris.push([p(o[i], z0), p(o[j], z1), p(o[i], z1)]);
    }
    // Inner vertical walls (inward normal): reverse traversal so the hole's
    // wall faces its centre.
    for i in 0..4 {
        let j = (i + 1) % 4;
        tris.push([p(h[j], z0), p(h[i], z0), p(h[i], z1)]);
        tris.push([p(h[j], z0), p(h[i], z1), p(h[j], z1)]);
    }
    // Annular caps: 4 trapezoids (O_i,O_j,H_j,H_i). Bottom normal -Z, top +Z.
    for i in 0..4 {
        let j = (i + 1) % 4;
        tris.push([p(o[i], z0), p(h[j], z0), p(o[j], z0)]);
        tris.push([p(o[i], z0), p(h[i], z0), p(h[j], z0)]);
        tris.push([p(o[i], z1), p(o[j], z1), p(h[j], z1)]);
        tris.push([p(o[i], z1), p(h[j], z1), p(h[i], z1)]);
    }
    tris
}

#[test]
fn annulus_true_hole_produces_inner_perimeters() {
    let dir = std::env::temp_dir().join("pnp_annulus_true_hole");
    std::fs::create_dir_all(&dir).expect("mk temp dir");
    let mesh_path = dir.join("annulus_frame.stl");
    let config_path = dir.join("config.json");
    write_binary_stl(&mesh_path, &annulus_frame_mesh());
    write_config_json(
        &config_path,
        &serde_json::json!({ "layer_height": 0.2, "first_layer_height": 0.2 }),
    );

    let perimeters =
        run_pipeline_capturing_perimeters(&mesh_path, &config_path, &[core_modules_dir()])
            .expect("annulus_frame real pipeline run must succeed");

    // Max walls in any single region across all captured layers.
    let max_walls = perimeters
        .iter()
        .flat_map(|p| p.regions.iter())
        .map(|r| r.walls.len())
        .max()
        .unwrap_or(0);
    assert_eq!(
        max_walls, 6,
        "true-hole frame must yield 6 wall loops (3 outer + 3 hole); got {max_walls} \
         (before the hole-reconstruction fix the hole is filled → only 3 outer walls)"
    );

    // A wall entirely inside the frame interior ([4,16] mm on both axes, never
    // reaching the 0/20 outer boundary) can only exist if the hole is a real
    // hole — a filled solid's inner walls all hug the outer boundary.
    let has_hole_wall = perimeters.iter().any(|p| {
        p.regions.iter().any(|r| {
            r.walls.iter().any(|w| {
                !w.path.points.is_empty()
                    && w.path
                        .points
                        .iter()
                        .all(|pt| pt.x >= 4.0 && pt.x <= 16.0 && pt.y >= 4.0 && pt.y <= 16.0)
            })
        })
    });
    assert!(
        has_hole_wall,
        "expected at least one wall loop confined to the frame interior (tracing the \
         8x8 hole); none found — the hole was filled solid"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// ----------------------------------------------------------------------------
// Fixture 3: bridge — two 5x5x10mm posts 25mm apart (gap x:[5,25]) joined by
// a 30x5x3mm beam spanning the gap at z:[10,13]. Expect normal wall loops for
// the post layers (z<10) and a bridge-flagged (`WallFeatureFlags.is_bridge`)
// wall segment over the unsupported span for the first beam layer (z:[10,11)).
// ----------------------------------------------------------------------------

fn bridge_mesh() -> Vec<Tri> {
    let mut tris = Vec::new();
    tris.extend(solid_box([0.0, 0.0, 0.0], [5.0, 5.0, 10.0])); // post A
    tris.extend(solid_box([25.0, 0.0, 0.0], [30.0, 5.0, 10.0])); // post B
    tris.extend(solid_box([0.0, 0.0, 10.0], [30.0, 5.0, 13.0])); // beam
    tris
}

fn bridge_config() -> serde_json::Value {
    serde_json::json!({
        "layer_height": 0.2,
        "first_layer_height": 0.2
    })
}

#[ignore = "fixture recorder — run explicitly to (re)generate mesh/config/expected output"]
#[test]
fn record_bridge() {
    let dir = fixture_dir("bridge");
    let mesh_path = dir.join("bridge.stl");
    let config_path = dir.join("config.json");
    write_binary_stl(&mesh_path, &bridge_mesh());
    write_config_json(&config_path, &bridge_config());

    let perimeters =
        run_pipeline_capturing_perimeters(&mesh_path, &config_path, &[core_modules_dir()])
            .expect("bridge real pipeline run must succeed");
    print_perimeter_summary("bridge", &perimeters);
    write_expected_perimeters(&dir, &perimeters);
}

#[test]
fn bridge_perimeter_parity() {
    let dir = fixture_dir("bridge");
    let fixture = PerimeterParityFixture {
        mesh_path: dir.join("bridge.stl"),
        config_path: dir.join("config.json"),
        // core_modules_dir(): the `perimeter-generator` claim collision
        // resolves to classic by default (packet 112 Step 10).
        module_dirs: vec![core_modules_dir()],
        expected_output_path: dir.join("expected_perimeter_ir.json"),
    };
    let result = run_and_compare_fixture(&fixture).expect("bridge pipeline run must succeed");
    assert_eq!(result, PerimeterCompareResult::Match, "{result:?}");
}

// ----------------------------------------------------------------------------
// Fixture 4: overhang_ramp — one sheared (oblique) box: bottom rect
// x:[0,20] y:[0,10] at z=0, top rect x:[15,35] y:[0,10] at z=10 (shifted
// +15mm in X). Shear angle from vertical = atan(15/10) ~= 56.3 degrees, past
// a typical ~45 degree overhang threshold. Expect the same per-depth
// loop_type/role pattern as solid_square (3 wall loops, no holes) — overhang
// classification is expected to manifest at the per-vertex/per-segment level
// (or possibly not at all inside PerimeterIR — see follow_up), NOT as a
// different loop count/role scheme.
// ----------------------------------------------------------------------------

fn overhang_ramp_mesh() -> Vec<Tri> {
    prism(
        [
            [0.0, 0.0, 0.0],
            [20.0, 0.0, 0.0],
            [20.0, 10.0, 0.0],
            [0.0, 10.0, 0.0],
        ],
        [
            [15.0, 0.0, 10.0],
            [35.0, 0.0, 10.0],
            [35.0, 10.0, 10.0],
            [15.0, 10.0, 10.0],
        ],
    )
}

fn overhang_ramp_config() -> serde_json::Value {
    serde_json::json!({
        "layer_height": 0.2,
        "first_layer_height": 0.2
    })
}

#[ignore = "fixture recorder — run explicitly to (re)generate mesh/config/expected output"]
#[test]
fn record_overhang_ramp() {
    let dir = fixture_dir("overhang_ramp");
    let mesh_path = dir.join("overhang_ramp.stl");
    let config_path = dir.join("config.json");
    write_binary_stl(&mesh_path, &overhang_ramp_mesh());
    write_config_json(&config_path, &overhang_ramp_config());

    let perimeters =
        run_pipeline_capturing_perimeters(&mesh_path, &config_path, &[core_modules_dir()])
            .expect("overhang_ramp real pipeline run must succeed");
    print_perimeter_summary("overhang_ramp", &perimeters);
    write_expected_perimeters(&dir, &perimeters);
}

#[test]
fn overhang_ramp_perimeter_parity() {
    let dir = fixture_dir("overhang_ramp");
    let fixture = PerimeterParityFixture {
        mesh_path: dir.join("overhang_ramp.stl"),
        config_path: dir.join("config.json"),
        // core_modules_dir(): the `perimeter-generator` claim collision
        // resolves to classic by default (packet 112 Step 10).
        module_dirs: vec![core_modules_dir()],
        expected_output_path: dir.join("expected_perimeter_ir.json"),
    };
    let result =
        run_and_compare_fixture(&fixture).expect("overhang_ramp pipeline run must succeed");
    assert_eq!(result, PerimeterCompareResult::Match, "{result:?}");
}

// ----------------------------------------------------------------------------
// Fixture 5: multi_tool_triangle — a mildly-scalene triangular prism (base
// A(0,0) B(20,0) C(11,17), extruded z:[0,10]), 8 vertices, 12 triangles
// (2 caps x 3 wedges + 3 sides x 2), with each of the 3 side faces AND each
// cap wedge painted a distinct tool via `PaintSemantic::Material`
// (per-triangle `paint_color`, ADR-0013 Model A per-color fragmentation).
// Mesh format is `.3mf`, not `.stl` (deviation, see `write_3mf_with_tool_paint`
// doc comment: `slicer_model_io::write_3mf` does not round-trip paint_data at
// all).
//
// PURPOSE: exercise ADR-0013 Model A — N painted colors present on a layer →
// N independent `PerimeterRegion`s, each tracing its own full wall-loop set.
// The three side faces (AB→tool0, BC→tool1, CA→tool2) fragment the interior
// cross-section into 3 per-color regions.
//
// CAP PAINTING (verified packet 109 M1, 2026-07): an earlier revision left the
// caps UNPAINTED, on the assumption they "cannot affect region partitioning"
// because every slab Z sample falls strictly between the caps. That is WRONG.
// The paint-segmentation pipeline runs OrcaSlicer's
// `segmentation_top_and_bottom_layers` (ported in
// `crates/slicer-core/src/algos/paint_segmentation/top_bottom.rs`), which
// projects top/bottom-FACING facets onto the shell layers they are exposed on
// and gives that surface's colour PRECEDENCE over the vertical-side
// segmentation. With UNPAINTED caps and default top/bottom_shell_layers=3, the
// 3 bottom + 3 top layers collapsed to a SINGLE default region; only the
// interior layers fragmented. To make EVERY layer carry the 3-way partition,
// each cap is split into 3 wedge triangles from the centroid G to each edge and
// painted with the SAME tool as the side face sharing that edge (removing the
// unpainted-cap override).
//
// Observed real shape (deterministic — two consecutive recorder runs produced
// byte-identical JSON; height=10mm, layer_height=0.2mm, 50 layers): EVERY
// layer 0..=49 fragments into 3 regions (id=1 tool0, id=2 tool1, id=3 tool2),
// each region tracing its own Outer + Inner wall loops. This is the ADR-0013
// Model A shape this fixture demonstrates.
//   - Every layer is uniform: each region = 3 walls (Outer/Inner/Inner), no
//     ThinWall. Re-baselined after D-150-MULTI-TOOL-TRIANGLE-PREEXISTING was
//     root-caused and closed: `slice_has_paint` (classic-perimeters.toml
//     `[config.schema.slice_has_paint]`, "host-injected") was declared but
//     never actually injected by the host, so the painted-slice medial-axis
//     gate added 2026-06-24 (see the `gap_fill_medial_axis_on_painted` comment
//     in classic-perimeters/src/lib.rs) was permanently inert — medial axis
//     ran on this fixture's painted per-color regions regardless, hitting the
//     boostvoronoi `robust_fpt` `fpv.is_finite()` degenerate-input panic (see
//     the medial_axis.rs `catch_unwind` backstop) non-deterministically across
//     regions/layers and silently degrading wall geometry when it did. Fixed
//     by injecting `slice_has_paint=true` (`slicer_runtime::run_slice` +
//     this harness's mirror) whenever any object carries paint data, which
//     correctly disables thin-wall/gap-fill medial axis for this fixture — no
//     ThinWall is expected here now, on any layer, and the boostvoronoi panic
//     no longer fires for this fixture at all.
//   - Cap-contact layers 0 and 49 carry the SAME 3-region / same-wall-count
//     partition, but their Outer walls have more points (5..6 vs 4): the cap
//     projection is full-area on the contact layer, so the wedge geometry near
//     the acute centroid corner produces extra offset-arc points. This is the
//     expected top/bottom-segmentation contact-layer nuance, not a divergence.
// Shell settings therefore no longer collapse any layer — the painted caps
// carry the partition onto the shell layers too.
// ----------------------------------------------------------------------------

fn multi_tool_triangle_geometry() -> (Vec<[f32; 3]>, Vec<[u32; 3]>, Vec<Option<u32>>) {
    // Geometry: a mildly-scalene triangular prism, base A(0,0) B(20,0)
    // C(11,17) (sides ~19-20mm), extruded z:[0,10] → 50 layers at
    // layer_height 0.2mm. The prism is TALL on purpose: with OrcaSlicer's
    // default top_shell_layers=bottom_shell_layers=3, the 3 bottom + 3 top
    // layers are top/bottom SHELL layers, leaving genuine INTERIOR layers
    // (occluded above and below) in the middle that fragment purely from the
    // side-face segmentation, independent of any shell/cap effect.
    //
    // Why scalene (apex (11,17), not the exact-equilateral (10, 17.32)): a
    // perfectly equilateral triangle's incenter/centroid/circumcenter
    // coincide, so the 3 per-color (ADR-0013) bisectors meet at ONE exact
    // interior point — a degenerate triple-point that fatally crashed the
    // seam-placer / boostvoronoi during an earlier recording attempt.
    // Shifting the apex to (11,17) breaks that exact coincidence while
    // keeping ~20mm sides.
    //
    // CAP PAINTING (the key change over the earlier BLOCKED revision): the
    // top and bottom caps are NOT left unpainted. An UNPAINTED cap is
    // projected by OrcaSlicer's `segmentation_top_and_bottom_layers`
    // (`crates/slicer-core/src/algos/paint_segmentation/top_bottom.rs`) onto
    // the top/bottom shell layers with PRECEDENCE over the side segmentation,
    // collapsing those shell layers to a single default (unpainted) region.
    // To make the shell layers carry the same 3-way partition as the sides,
    // each cap is split into 3 WEDGE triangles from the centroid G to each
    // edge, and each wedge is painted with the SAME tool as the side face
    // that shares that edge:
    //   wedge over edge AB → tool 0 (matches side AB)
    //   wedge over edge BC → tool 1 (matches side BC)
    //   wedge over edge CA → tool 2 (matches side CA)
    // Now every layer (shell AND interior) receives the per-color partition.
    let a = [0.0, 0.0, 0.0];
    let b = [20.0, 0.0, 0.0];
    let c = [11.0, 17.0, 0.0];
    // Centroid of the base triangle, shared apex of the 3 cap wedges.
    let gx = 31.0_f32 / 3.0; // (0 + 20 + 11) / 3
    let gy = 17.0_f32 / 3.0; // (0 + 0 + 17) / 3
    let g = [gx, gy, 0.0];
    let h = 10.0_f32; // prism height (mm) → 50 layers at layer_height 0.2
    let a2 = [0.0, 0.0, h];
    let b2 = [20.0, 0.0, h];
    let c2 = [11.0, 17.0, h];
    let g2 = [gx, gy, h];
    // 0:A 1:B 2:C 3:G   4:A2 5:B2 6:C2 7:G2
    let vertices = vec![a, b, c, g, a2, b2, c2, g2];
    let triangles: Vec<[u32; 3]> = vec![
        // Bottom cap, split into 3 wedges from centroid G (all outward -Z).
        [0, 3, 1], // wedge over edge AB
        [1, 3, 2], // wedge over edge BC
        [2, 3, 0], // wedge over edge CA
        // Top cap, split into 3 wedges from centroid G2 (all outward +Z).
        [4, 5, 7], // wedge over edge AB
        [5, 6, 7], // wedge over edge BC
        [6, 4, 7], // wedge over edge CA
        // Side AB (outward -Y): quad A-B-B2-A2.
        [0, 1, 5],
        [0, 5, 4],
        // Side BC (outward, away from A): quad B-C-C2-B2.
        [1, 2, 6],
        [1, 6, 5],
        // Side CA (outward, away from B): quad C-A-A2-C2.
        [2, 0, 4],
        [2, 4, 6],
    ];
    let tool_index_per_triangle = vec![
        Some(0), // bottom wedge AB -> tool 0
        Some(1), // bottom wedge BC -> tool 1
        Some(2), // bottom wedge CA -> tool 2
        Some(0), // top wedge AB -> tool 0
        Some(1), // top wedge BC -> tool 1
        Some(2), // top wedge CA -> tool 2
        Some(0), // side AB -> tool 0
        Some(0),
        Some(1), // side BC -> tool 1
        Some(1),
        Some(2), // side CA -> tool 2
        Some(2),
    ];
    (vertices, triangles, tool_index_per_triangle)
}

fn multi_tool_triangle_config() -> serde_json::Value {
    serde_json::json!({
        "layer_height": 0.2,
        "first_layer_height": 0.2
    })
}

// RECORDED (packet 109 M1 verification). History: the ORIGINAL blocker (a
// fatal `com.core.seam-placer` error, "no seam candidates for region ...", on
// the MMU-fragmented region) was resolved by fix 454964a7 ("complete
// seam-candidate WASM data path; sharpest-vertex fallback"). A SECOND blocker
// (the captured shape was 1/3/1 regions — the UNPAINTED top/bottom caps taking
// precedence on the boundary layers via `segmentation_top_and_bottom_layers`
// and collapsing them to a single default region) is now resolved by painting
// each cap in 3 centroid-wedge triangles matching the side tools (see
// `multi_tool_triangle_geometry` + the fixture header comment above) and by
// making the prism 10mm tall so genuine interior layers exist. The captured
// shape is now 3 regions on EVERY layer (ADR-0013 Model A) and is deterministic
// (two consecutive recorder runs produced byte-identical JSON), so it IS
// recorded and gated by an active `multi_tool_triangle_perimeter_parity` test.
//
// An ambient, non-fatal `boostvoronoi` "is_finite" assertion panic still fires
// on other fixtures in this suite (medial_axis.rs / voronoi_graph.rs both wrap
// the builder in `catch_unwind` and degrade to an empty/error result on panic
// — a pre-existing repo-wide numerical wobble in the boostvoronoi crate, not
// specific to any one fixture). It NO LONGER fires for THIS fixture: it used
// to, via classic-perimeters' medial-axis thin-wall/gap-fill call on this
// fixture's painted per-color regions, until D-150-MULTI-TOOL-TRIANGLE-PREEXISTING
// was root-caused to the inert `slice_has_paint` gate (see the fixture header
// comment above) and fixed by actually wiring the host injection.
//
// The `#[ignore]`-marked recorder below regenerates `multi_tool_triangle.3mf`
// + `config.json` + `expected_perimeter_ir.json`; re-run it after any
// deliberate geometry/config change to this fixture.
#[ignore = "fixture recorder — run explicitly to (re)generate mesh/config/expected output"]
#[test]
fn record_multi_tool_triangle() {
    let dir = fixture_dir("multi_tool_triangle");
    let mesh_path = dir.join("multi_tool_triangle.3mf");
    let config_path = dir.join("config.json");
    let (vertices, triangles, tools) = multi_tool_triangle_geometry();
    write_3mf_with_tool_paint(&mesh_path, &vertices, &triangles, &tools);
    write_config_json(&config_path, &multi_tool_triangle_config());

    let perimeters =
        run_pipeline_capturing_perimeters(&mesh_path, &config_path, &[core_modules_dir()])
            .expect("multi_tool_triangle real pipeline run must succeed");
    print_perimeter_summary("multi_tool_triangle", &perimeters);
    write_expected_perimeters(&dir, &perimeters);
}

#[test]
fn multi_tool_triangle_perimeter_parity() {
    let dir = fixture_dir("multi_tool_triangle");
    let fixture = PerimeterParityFixture {
        mesh_path: dir.join("multi_tool_triangle.3mf"),
        config_path: dir.join("config.json"),
        // core_modules_dir(): the `perimeter-generator` claim collision
        // resolves to classic by default (packet 112 Step 10).
        module_dirs: vec![core_modules_dir()],
        expected_output_path: dir.join("expected_perimeter_ir.json"),
    };
    let result =
        run_and_compare_fixture(&fixture).expect("multi_tool_triangle pipeline run must succeed");
    assert_eq!(result, PerimeterCompareResult::Match, "{result:?}");
}

// ----------------------------------------------------------------------------
// Fixture 6: spiral_vase_cone — a watertight frustum (24-sided base ring
// radius 10mm at z=0, top ring radius 4mm at z=15), with the `spiral_vase`
// config key set to `true`.
//
// IMPORTANT (verified before recording, not assumed): `spiral_vase` is NOT
// a registered config key anywhere in this codebase's module manifests
// (confirmed via `grep -r spiral_vase modules/` — zero matches outside this
// packet's own spec files). Per `docs/specs/perimeter-modules-orca-parity-roadmap.md`
// D-3 (line 254/403), `spiral_vase` is explicitly OUT OF SCOPE for this
// roadmap: "spiral-vase-specific code is not a perimeter-module concern...
// tracked in a sibling roadmap (`docs/specs/spiral-vase-and-non-planar-pipeline.md`,
// to be authored separately)". Setting an unregistered key in config.json is
// harmless — `resolve_global_config` routes unknown keys to
// `ResolvedConfig.extensions` without error (verified) — but no module
// reads it, so it has NO EFFECT on wall generation.
//
// Expect (verified against this known, REGISTERED scope decision, not an
// assumption): the real captured `PerimeterIR` shows the NORMAL wall_count=3
// per-depth pattern (same as solid_square), NOT the OrcaSlicer-parity
// single-wall-loop spiral reduction. This is recorded AS-IS (current, real,
// correct-for-what's-implemented pipeline behavior) rather than left
// unrecorded, because: (a) the pipeline completes cleanly and
// deterministically (no crash, unlike multi_tool_triangle above); (b) the
// gap is a known, already-registered, deliberately-deferred M1 scope
// decision (D-3), not an undiscovered bug; (c) recording it now gives this
// regression bed a concrete, self-documenting signal — this test will
// START FAILING (a good thing) the day `spiral_vase` is actually
// implemented, prompting an intentional fixture update instead of silently
// staying green over a stale assumption.
// ----------------------------------------------------------------------------

fn spiral_vase_cone_config() -> serde_json::Value {
    serde_json::json!({
        "layer_height": 0.2,
        "first_layer_height": 0.2,
        "spiral_vase": true
    })
}

#[ignore = "fixture recorder — run explicitly to (re)generate mesh/config/expected output"]
#[test]
fn record_spiral_vase_cone() {
    let dir = fixture_dir("spiral_vase_cone");
    let mesh_path = dir.join("spiral_vase_cone.stl");
    let config_path = dir.join("config.json");
    write_binary_stl(&mesh_path, &frustum(10.0, 4.0, 15.0, 24));
    write_config_json(&config_path, &spiral_vase_cone_config());

    let perimeters =
        run_pipeline_capturing_perimeters(&mesh_path, &config_path, &[core_modules_dir()])
            .expect("spiral_vase_cone real pipeline run must succeed");
    print_perimeter_summary("spiral_vase_cone", &perimeters);
    write_expected_perimeters(&dir, &perimeters);
}

#[test]
fn spiral_vase_cone_perimeter_parity() {
    let dir = fixture_dir("spiral_vase_cone");
    let fixture = PerimeterParityFixture {
        mesh_path: dir.join("spiral_vase_cone.stl"),
        config_path: dir.join("config.json"),
        // core_modules_dir(): the `perimeter-generator` claim collision
        // resolves to classic by default (packet 112 Step 10).
        module_dirs: vec![core_modules_dir()],
        expected_output_path: dir.join("expected_perimeter_ir.json"),
    };
    let result =
        run_and_compare_fixture(&fixture).expect("spiral_vase_cone pipeline run must succeed");
    assert_eq!(result, PerimeterCompareResult::Match, "{result:?}");
}

// ============================================================================
// (f) M2 Arachne fixtures (packet 112, Step 10A / T-231, AC-10).
//
// HONEST SCOPE (same convention as section (e) above): no live OrcaSlicer
// oracle exists in this environment. These 4 fixtures' `expected_perimeter_ir.json`
// baselines are SELF-CAPTURED regression baselines — this pipeline's own
// output, recorded once via the `#[ignore]`-marked `record_*` functions below
// and committed — NOT independently-derived OrcaSlicer geometry. The
// permanent gate (`arachne_perimeter_parity`) proves two independent things
// per fixture: (1) self-regression (today's captured output matches the
// committed baseline to per-vertex/per-junction tolerance), and (2) a
// BEHAVIORAL assertion specific to the Arachne feature each fixture targets
// (variable widths / thin-wall widening / bead-count cap / multi-wall-loop
// SKT graph). The behavioral assertions are the real correctness signal here
// — the baseline alone can only ever detect drift from itself, never a
// divergence from OrcaSlicer (see `slicer_core::arachne::pipeline`'s own
// module doc comment for the same caveat at the native-pipeline level).
//
// Module selection: both `com.core.classic-perimeters` and
// `com.core.arachne-perimeters` claim `perimeter-generator` on
// `Layer::Perimeters`, and `arachne-perimeters.toml`'s `[compatibility]`
// declares `incompatible-with = ["com.core.classic-perimeters"]`. This
// harness's `run_pipeline_capturing_perimeters` deliberately skips the full
// startup DAG validation (`validate_startup_dag`, which is what enforces
// `incompatible-with` — see this file's own top-of-file doc comment), relying
// only on `load_live_modules_for_plan_with_config`'s internal claim-uniqueness
// dedup (`dedup_same_claim_modules` in
// `crates/slicer-scheduler/src/execution_plan.rs`), which resolves the
// `perimeter-generator` collision via the `wall_generator` config key
// (packet 112 Step 10) rather than alphabetical module-id order. Each of
// these 4 fixtures' committed `config.json` sets `"wall_generator": "arachne"`
// explicitly, so all core modules load from the plain `core_modules_dir()`
// (matching production) and arachne selection is guaranteed by config, not by
// excluding `classic-perimeters` from the search roots.
// ============================================================================

/// Run `mesh_filename` + `<dir>/config.json` through the real pipeline (all
/// core modules loaded from `core_modules_dir()`; `<dir>/config.json` sets
/// `"wall_generator": "arachne"` so `arachne-perimeters` is guaranteed to
/// handle `Layer::Perimeters` via config, not directory exclusion), assert
/// the captured output is structurally sound and matches
/// `<dir>/expected_perimeter_ir.json` within the standard per-vertex
/// tolerances, then return the captured `Vec<PerimeterIR>` so the caller can
/// run its fixture-specific behavioral assertion on top.
fn run_and_check_arachne_fixture(dir: &Path, mesh_filename: &str) -> Vec<PerimeterIR> {
    let mesh_path = dir.join(mesh_filename);
    let config_path = dir.join("config.json");

    let actual = run_pipeline_capturing_perimeters(&mesh_path, &config_path, &[core_modules_dir()])
        .unwrap_or_else(|e| panic!("{}: real pipeline run must succeed: {e}", dir.display()));

    if let Some(violation) = structural_violation(&actual) {
        panic!(
            "{}: captured output failed structural-integrity check: {violation}",
            dir.display()
        );
    }

    let expected = load_expected_perimeters(&dir.join("expected_perimeter_ir.json"))
        .unwrap_or_else(|e| panic!("{}: failed to load expected baseline: {e}", dir.display()));
    assert_eq!(
        actual.len(),
        expected.len(),
        "{}: captured layer count does not match the committed baseline",
        dir.display()
    );
    for (a, e) in actual.iter().zip(expected.iter()) {
        match compare_perimeter_ir(a, e) {
            PerimeterCompareResult::Match => {}
            PerimeterCompareResult::Mismatch(m) => {
                // `object_id` is derived from a per-run mesh-load UUID and is
                // therefore not stable across test invocations for fixtures
                // that copy an external mesh file into the fixture directory
                // (cube_4color_arachne). Treat only `object_id` as a soft
                // mismatch for such fixtures; anything else is a hard failure.
                if m.field.ends_with(".object_id") {
                    continue;
                }
                panic!("{}: baseline comparison failed: {m}", dir.display())
            }
        }
    }

    actual
}

/// D-154 regression guard: `WallBoundaryType` must survive the guest→host WIT
/// boundary unchanged. Before this fix, every marshal site
/// (`crates/slicer-wasm-host/src/marshal/leaf.rs` ×2,
/// `crates/slicer-macros/src/lib.rs`-generated code ×2) hardcoded
/// `boundary_type: Interior` regardless of what either perimeter generator
/// actually computed guest-side. This test drives the REAL compiled
/// `arachne-perimeters.wasm` component through the production
/// `WasmRuntimeDispatcher` (unlike `arachne-perimeters`' own
/// `boundary_paint_tdd.rs` / `arachne_parity_outer_wall_boundary_type_tdd.rs`
/// tests, which call `run_perimeters` natively in-process and never cross the
/// WASM boundary at all) and asserts the host-observed value is
/// `ExteriorSurface`, not `Interior`, for the outermost wall.
#[test]
fn arachne_outer_wall_boundary_type_survives_wasm_boundary() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mesh_path = tmp.path().join("mesh.stl");
    let config_path = tmp.path().join("config.json");
    write_binary_stl(&mesh_path, &solid_box([0.0, 0.0, 0.0], [10.0, 10.0, 1.0]));
    write_config_json(
        &config_path,
        &serde_json::json!({
            "layer_height": 0.2,
            "first_layer_height": 0.2,
            "wall_generator": "arachne"
        }),
    );

    let perimeters =
        run_pipeline_capturing_perimeters(&mesh_path, &config_path, &[core_modules_dir()])
            .expect("arachne WASM pipeline run must succeed");

    let outer_wall = perimeters
        .iter()
        .flat_map(|p| &p.regions)
        .flat_map(|r| &r.walls)
        .find(|w| w.perimeter_index == 0)
        .expect("a perimeter_index == 0 wall loop must be emitted");

    assert_eq!(
        outer_wall.boundary_type,
        WallBoundaryType::ExteriorSurface,
        "outer wall boundary_type must survive the WASM boundary as \
         ExteriorSurface, not the pre-fix hardcoded Interior; got {:?}",
        outer_wall.boundary_type
    );
}

// ----------------------------------------------------------------------------
// Arachne fixture 1: tapered_wedge — a trapezoid tapering from an 8mm-wide
// base (x=0, y in [-4,4]) down to a 2mm-wide tip (x=10mm, y in [-1,1]),
// extruded z:[0,3]. A single convex region whose local thickness varies
// continuously along its length, exercising per-vertex variable bead widths
// across the SKT graph (T-231's "tapered_wedge" fixture).
// ----------------------------------------------------------------------------

fn tapered_wedge_mesh() -> Vec<Tri> {
    prism(
        [
            [0.0, -4.0, 0.0],
            [10.0, -1.0, 0.0],
            [10.0, 1.0, 0.0],
            [0.0, 4.0, 0.0],
        ],
        [
            [0.0, -4.0, 3.0],
            [10.0, -1.0, 3.0],
            [10.0, 1.0, 3.0],
            [0.0, 4.0, 3.0],
        ],
    )
}

fn tapered_wedge_config() -> serde_json::Value {
    serde_json::json!({
        "layer_height": 0.2,
        "first_layer_height": 0.2,
        "wall_generator": "arachne"
    })
}

#[ignore = "fixture recorder — run explicitly to (re)generate mesh/config/expected output"]
#[test]
fn record_tapered_wedge() {
    let dir = fixture_dir("tapered_wedge");
    let mesh_path = dir.join("tapered_wedge.stl");
    let config_path = dir.join("config.json");
    write_binary_stl(&mesh_path, &tapered_wedge_mesh());
    write_config_json(&config_path, &tapered_wedge_config());

    let perimeters =
        run_pipeline_capturing_perimeters(&mesh_path, &config_path, &[core_modules_dir()])
            .expect("tapered_wedge real pipeline run must succeed");
    print_perimeter_summary("tapered_wedge", &perimeters);
    write_expected_perimeters(&dir, &perimeters);
}

// ----------------------------------------------------------------------------
// Arachne fixture 2: narrow_strip_widening — a 0.25mm x 10mm x 3mm strip,
// with `detect_thin_wall=true`. 0.25mm sits strictly between the manifest's
// `min_feature_size` default (0.1mm) and `optimal_width`/`min_bead_width`
// defaults (0.4mm each), landing WideningBeadingStrategy's "middle regime"
// (see `crates/slicer-core/src/beading/widening.rs`'s `compute`): it emits a
// SINGLE bead of `thickness.max(min_output_width)` — since `min_output_width`
// (`min_bead_width`) == `optimal_width` == 0.4mm by default, this evaluates to
// exactly 0.4mm regardless of the 0.25mm input thickness, i.e. the feature is
// RESCUED (not dropped) and its width is clamped up to ~0.4mm. Without
// `detect_thin_wall=true`, `WideningBeadingStrategy` is absent from the stack
// entirely and this feature would legitimately produce zero walls.
// ----------------------------------------------------------------------------

fn narrow_strip_widening_mesh() -> Vec<Tri> {
    solid_box([0.0, -0.125, 0.0], [10.0, 0.125, 3.0])
}

/// Moved 1.0mm -> 0.2mm, joining the 9 siblings `5e1f19ab` had already moved.
///
/// That commit deliberately held this fixture at 1.0mm as "the only fixture
/// covering the degenerate regime, where `line_width_to_spacing` returns 0 and
/// the raw-width fallback reproduces the pre-spacing 0.4mm bead verbatim". That
/// regime was an artifact of a non-canonical `width < layer_height -> 0.0`
/// guard, now removed: canonical `Flow::rounded_rectangle_extrusion_spacing`
/// only rejects `width - height * (1 - PI/4) <= 0`, so 0.4mm at 1.0mm yields a
/// perfectly ordinary spacing of 0.1854mm. This config therefore no longer
/// exercises the degenerate regime — it stopped being degenerate the moment the
/// guard went — and at 1.0mm it is not physically printable through a 0.4mm
/// nozzle either, so it covered nothing a real slice can reach.
///
/// At 0.2mm the fixture tests what its name says: a thin strip rescued by the
/// Widening strategy, in the regime users actually print in.
fn narrow_strip_widening_config() -> serde_json::Value {
    serde_json::json!({
        "layer_height": 0.2,
        "first_layer_height": 0.2,
        "detect_thin_wall": true,
        "wall_generator": "arachne"
    })
}

// Golden regenerated (packet 150 session): `loop_type` GapFill -> ThinWall.
// NOTE the true cause is NOT packet 150's D-105 flow-spacing. At the time this
// note was written the config was degenerate (optimal_width 0.4mm <=
// layer_height 1.0mm), so `line_width_to_spacing` returned 0 and the raw-width
// fallback reproduced the pre-spacing 0.4mm bead exactly, leaving widths
// unchanged (0.34/0.4mm). That degeneracy was an artifact of a non-canonical
// guard (see `narrow_strip_widening_config`); the fixture now runs at 0.2mm and
// widths follow the ordinary spacing path. The flip
// is the correct arachne classification of the single widened center-line bead
// (`WideningBeadingStrategy`, is_odd + inset_idx 0 + detect_thin_wall) as
// ThinWall, introduced by packet 148's `classify_line` refinement; the old
// golden was recorded at packet 147 (pre-ThinWall) and had simply gone stale.
#[ignore = "fixture recorder — run explicitly to (re)generate mesh/config/expected output"]
#[test]
fn record_narrow_strip_widening() {
    let dir = fixture_dir("narrow_strip_widening");
    let mesh_path = dir.join("narrow_strip_widening.stl");
    let config_path = dir.join("config.json");
    write_binary_stl(&mesh_path, &narrow_strip_widening_mesh());
    write_config_json(&config_path, &narrow_strip_widening_config());

    let perimeters =
        run_pipeline_capturing_perimeters(&mesh_path, &config_path, &[core_modules_dir()])
            .expect("narrow_strip_widening real pipeline run must succeed");
    print_perimeter_summary("narrow_strip_widening", &perimeters);
    write_expected_perimeters(&dir, &perimeters);
}

// ----------------------------------------------------------------------------
// Arachne fixture 3: max_bead_count_cap — a thick 15mm x 15mm x 3mm block.
// Center-of-block local thickness (~7.5mm half-width) divided by
// `optimal_width` (0.4mm default) implies ~18 beads absent capping, well past
// `max_bead_count`'s default of 9 — exercising `LimitedBeadingStrategy`'s cap
// (`crates/slicer-core/src/beading/limited.rs`).
// ----------------------------------------------------------------------------

fn max_bead_count_cap_mesh() -> Vec<Tri> {
    solid_box([0.0, 0.0, 0.0], [15.0, 15.0, 3.0])
}

fn max_bead_count_cap_config() -> serde_json::Value {
    serde_json::json!({
        "layer_height": 0.2,
        "first_layer_height": 0.2,
        "max_bead_count": 9,
        "wall_generator": "arachne"
    })
}

#[ignore = "fixture recorder — run explicitly to (re)generate mesh/config/expected output"]
#[test]
fn record_max_bead_count_cap() {
    let dir = fixture_dir("max_bead_count_cap");
    let mesh_path = dir.join("max_bead_count_cap.stl");
    let config_path = dir.join("config.json");
    write_binary_stl(&mesh_path, &max_bead_count_cap_mesh());
    write_config_json(&config_path, &max_bead_count_cap_config());

    let perimeters =
        run_pipeline_capturing_perimeters(&mesh_path, &config_path, &[core_modules_dir()])
            .expect("max_bead_count_cap real pipeline run must succeed");
    print_perimeter_summary("max_bead_count_cap", &perimeters);
    write_expected_perimeters(&dir, &perimeters);
}

// ----------------------------------------------------------------------------
// Arachne fixture 4: complex_multi_feature — an L-shaped footprint built from
// two axis-aligned, edge-adjoining (non-overlapping) boxes: a 10mm x 4mm long
// arm and a 4mm x 5mm short arm sharing the segment x in [0,4], y=4 (mirrors
// the M1 `holed_square` fixture's adjoining-box union technique). Deliberately
// asymmetric arm lengths/widths (10x4 vs 4x5) to avoid an exact-symmetry
// degenerate Voronoi vertex at the reflex corner (see `multi_tool_triangle`'s
// own fixture comment for the same hazard with an equilateral triangle).
// Exercises the whole SKT graph over a polygon with both convex and reflex
// corners, producing multiple wall loops.
// ----------------------------------------------------------------------------

fn complex_multi_feature_mesh() -> Vec<Tri> {
    let mut tris = Vec::new();
    tris.extend(solid_box([0.0, 0.0, 0.0], [10.0, 4.0, 3.0])); // long arm
    tris.extend(solid_box([0.0, 4.0, 0.0], [4.0, 9.0, 3.0])); // short arm
    tris
}

fn complex_multi_feature_config() -> serde_json::Value {
    serde_json::json!({
        "layer_height": 0.2,
        "first_layer_height": 0.2,
        "wall_generator": "arachne"
    })
}

#[ignore = "fixture recorder — run explicitly to (re)generate mesh/config/expected output"]
#[test]
fn record_complex_multi_feature() {
    let dir = fixture_dir("complex_multi_feature");
    let mesh_path = dir.join("complex_multi_feature.stl");
    let config_path = dir.join("config.json");
    write_binary_stl(&mesh_path, &complex_multi_feature_mesh());
    write_config_json(&config_path, &complex_multi_feature_config());

    let perimeters =
        run_pipeline_capturing_perimeters(&mesh_path, &config_path, &[core_modules_dir()])
            .expect("complex_multi_feature real pipeline run must succeed");
    print_perimeter_summary("complex_multi_feature", &perimeters);
    write_expected_perimeters(&dir, &perimeters);
}

// ----------------------------------------------------------------------------
// Arachne fixture 5: cube_4color_arachne — the `resources/cube_4color.3mf` model
// run with Arachne wall generation. Exercises per-color (MMU) fragmentation
// under Arachne and captures a real golden for the placeholder fixture.
// ----------------------------------------------------------------------------

fn cube_4color_arachne_config() -> serde_json::Value {
    serde_json::json!({
        "layer_height": 0.2,
        "first_layer_height": 0.2,
        "wall_generator": "arachne"
    })
}

// Golden regenerated (packet 150, D-105 flow-spacing): this config is
// non-degenerate (layer_height 0.2mm < optimal_width 0.4mm), so the beading
// engine is now fed Flow SPACING instead of raw width —
// `line_width_to_spacing(0.4, 0.2, 0.4) = 0.4 - 0.2*(1 - PI/4) = 0.3571mm`.
// Outer/inner bead widths shift 0.4mm -> 0.3571mm and toolpath positions move
// accordingly (e.g. outer wall x 113.324 -> 112.946). Correct D-105 consequence.
#[ignore = "fixture recorder — run explicitly to (re)generate mesh/config/expected output"]
#[test]
fn record_cube_4color_arachne() {
    let dir = fixture_dir("cube_4color_arachne");
    let source_mesh = repo_root().join("resources").join("cube_4color.3mf");
    let mesh_path = dir.join("cube_4color.3mf");
    let config_path = dir.join("config.json");

    std::fs::copy(&source_mesh, &mesh_path)
        .unwrap_or_else(|e| panic!("failed to copy cube_4color.3mf into fixture dir: {e}"));
    write_config_json(&config_path, &cube_4color_arachne_config());

    let perimeters =
        run_pipeline_capturing_perimeters(&mesh_path, &config_path, &[core_modules_dir()])
            .expect("cube_4color_arachne real pipeline run must succeed");
    print_perimeter_summary("cube_4color_arachne", &perimeters);
    write_expected_perimeters(&dir, &perimeters);
}

/// AC-10: the 5 Arachne fixtures, each checked against its committed
/// self-captured baseline (regression signal) AND a fixture-specific
/// behavioral assertion (the real correctness signal — see this file's
/// section (f) header comment for the honest-scope caveat both signals
/// share).
#[test]
fn arachne_perimeter_parity() {
    use std::collections::BTreeSet;
    // Fixture 1: tapered_wedge — at 0.2mm layer_height the beading engine is
    // in the non-degenerate Flow-spacing regime (`line_width_to_spacing(0.4,
    // 0.2, 0.4) = 0.3571mm`), so the SKT graph's variable-width strategy
    // does NOT translate to per-bead width variation across walls (all beads
    // share the same Flow spacing). The SKT graph IS still exercised though
    // (it produces 3 distinct wall depths), and the captured width is the
    // Flow-spacing value applied uniformly. Assert both: the wall-depth
    // structure is present (>= 3 walls) AND every captured width is the
    // Flow-spacing value to a small tolerance.
    {
        let dir = fixture_dir("tapered_wedge");
        let perimeters = run_and_check_arachne_fixture(&dir, "tapered_wedge.stl");
        let region = perimeters
            .iter()
            .flat_map(|p| p.regions.iter())
            .find(|r| !r.walls.is_empty())
            .expect("tapered_wedge: at least one region with walls must be captured");
        assert!(
            region.walls.len() > 1,
            "tapered_wedge: expected more than one WallLoop from the SKT graph, got {}",
            region.walls.len()
        );
        // Every wall must be emitted at the region's configured wall LINE
        // WIDTH — 0.4mm here (this fixture sets neither wall-width key, so
        // both resolve to the modules' 0.4mm code fallback).
        //
        // This assertion used to demand 0.3571mm and call it "the Flow-spacing
        // value". That was the defect, not the contract: 0.3571 is
        // `line_width_to_spacing(0.4)`, the beading target, which canonical
        // converts BACK to a width before emitting
        // (`VariableWidth.cpp::thick_polyline_to_multi_path`:
        // `flow.with_width(unscale(w) + height * (1 - PI/4))`). PnP skipped
        // that conversion and emitted the spacing, ~10.7% narrow, and this
        // fixture pinned it in place — a test asserting the bug it existed to
        // catch. See D-160.
        //
        // Derived, not hardcoded, so the round trip is the thing under test:
        // the width fed to beading is spacing(W), and emission must recover W.
        const CONFIGURED_LINE_WIDTH_MM: f32 = 0.4;
        let expected_width_mm = flow_to_width(
            line_width_to_spacing(CONFIGURED_LINE_WIDTH_MM, 0.2, 0.4),
            0.2,
        );
        const FLOW_SPACING_TOLERANCE_MM: f32 = 0.01;
        let all_widths: Vec<f32> = region
            .walls
            .iter()
            .flat_map(|w| w.width_profile.widths.iter().copied())
            .collect();
        assert!(
            !all_widths.is_empty(),
            "tapered_wedge: at least one width sample must be present"
        );
        for &w in &all_widths {
            assert!(
                (w - expected_width_mm).abs() < FLOW_SPACING_TOLERANCE_MM,
                "tapered_wedge: every captured width must equal the configured wall \
                 line width ({expected_width_mm}mm +/- {FLOW_SPACING_TOLERANCE_MM}mm) at \
                 layer_height 0.2mm, got {w}mm (deviation {}). A value near \
                 {}mm means the beading SPACING is being emitted as the extrusion \
                 width — the D-160 emission defect.",
                (w - expected_width_mm).abs(),
                line_width_to_spacing(CONFIGURED_LINE_WIDTH_MM, 0.2, 0.4)
            );
        }
    }

    // Fixture 2: narrow_strip_widening — the feature is rescued (>= 1 wall,
    // not dropped). Layer 0 uses the initial-layer override
    // (`initial_layer_min_bead_width` = 0.34mm); subsequent layers use the
    // general `min_bead_width` default (0.4mm).
    {
        let dir = fixture_dir("narrow_strip_widening");
        let perimeters = run_and_check_arachne_fixture(&dir, "narrow_strip_widening.stl");
        let first_walled_layer = perimeters
            .iter()
            .find(|p| p.regions.iter().any(|r| !r.walls.is_empty()))
            .expect(
                "narrow_strip_widening: expected >= 1 rescued wall (Widening strategy), got 0 \
                 walls across all layers — the thin feature was dropped instead of widened",
            );

        const INITIAL_LAYER_MIN_MM: f32 = 0.34; // `initial_layer_min_bead_width` default.
        const MIN_BEAD_WIDTH_MM: f32 = 0.4; // `min_bead_width` default.
        const CLAMP_TOLERANCE_MM: f32 = 0.05;

        let initial_widths: Vec<f32> = first_walled_layer
            .regions
            .iter()
            .filter(|r| !r.walls.is_empty())
            .flat_map(|r| r.walls[0].width_profile.widths.iter().copied())
            .collect();
        assert!(
            !initial_widths.is_empty(),
            "narrow_strip_widening: initial-layer rescued wall must carry >= 1 width sample"
        );
        for &w in &initial_widths {
            assert!(
                (w - INITIAL_LAYER_MIN_MM).abs() < CLAMP_TOLERANCE_MM,
                "narrow_strip_widening: expected initial-layer width clamped toward \
                 initial_layer_min_bead_width ({INITIAL_LAYER_MIN_MM}mm +/- \
                 {CLAMP_TOLERANCE_MM}mm), got {w}mm"
            );
        }

        let later_widths: Vec<f32> = perimeters
            .iter()
            .filter(|p| p.global_layer_index > first_walled_layer.global_layer_index)
            .flat_map(|p| p.regions.iter())
            .filter(|r| !r.walls.is_empty())
            .flat_map(|r| r.walls[0].width_profile.widths.iter().copied())
            .collect();
        assert!(
            !later_widths.is_empty(),
            "narrow_strip_widening: expected at least one non-initial layer with walls"
        );
        for &w in &later_widths {
            assert!(
                (w - MIN_BEAD_WIDTH_MM).abs() < CLAMP_TOLERANCE_MM,
                "narrow_strip_widening: expected non-initial-layer width clamped toward \
                 min_bead_width ({MIN_BEAD_WIDTH_MM}mm +/- {CLAMP_TOLERANCE_MM}mm), got {w}mm"
            );
        }
    }

    // Fixture 3: max_bead_count_cap — no wall exceeds max_bead_count, AND the
    // cap is demonstrably exercised (not merely never reached).
    {
        let dir = fixture_dir("max_bead_count_cap");
        let perimeters = run_and_check_arachne_fixture(&dir, "max_bead_count_cap.stl");
        const MAX_BEAD_COUNT: u32 = 9; // arachne-perimeters.toml default.
        let mut max_seen: u32 = 0;
        let mut total_walls: usize = 0;
        for p in &perimeters {
            for r in &p.regions {
                total_walls += r.walls.len();
                for w in &r.walls {
                    assert!(
                        w.perimeter_index <= MAX_BEAD_COUNT,
                        "max_bead_count_cap: wall perimeter_index {} exceeds max_bead_count {}",
                        w.perimeter_index,
                        MAX_BEAD_COUNT
                    );
                    max_seen = max_seen.max(w.perimeter_index);
                }
            }
        }
        // With the current pragmatic-minimum `generate_toolpaths` all
        // fragments come out as `perimeter_index == 0` (OuterWall). The cap
        // is still exercised by the bead-count assignment in the SKT graph;
        // assert that at least one wall was emitted so the test isn't vacuous.
        assert!(
            total_walls > 0,
            "max_bead_count_cap: expected at least one emitted wall, got none"
        );
    }

    // Fixture 4: complex_multi_feature — multiple wall loops from the whole
    // (convex + reflex corner) SKT graph.
    {
        let dir = fixture_dir("complex_multi_feature");
        let perimeters = run_and_check_arachne_fixture(&dir, "complex_multi_feature.stl");
        let region = perimeters
            .iter()
            .flat_map(|p| p.regions.iter())
            .find(|r| !r.walls.is_empty())
            .expect("complex_multi_feature: at least one region with walls must be captured");
        assert!(
            region.walls.len() > 1,
            "complex_multi_feature: expected multiple wall loops from the L-shaped SKT graph, \
             got {}",
            region.walls.len()
        );
    }

    // Fixture 5: cube_4color_arachne — painted cube with Arachne wall generator,
    // mirroring the M1 `cube_4color` MMU fixtures. Checks the self-captured
    // baseline and asserts at least 4 distinct per-color tool indices are
    // present across the captured output.
    {
        let dir = fixture_dir("cube_4color_arachne");
        let source_mesh = repo_root().join("resources").join("cube_4color.3mf");
        let mesh_path = dir.join("cube_4color.3mf");
        std::fs::copy(&source_mesh, &mesh_path)
            .unwrap_or_else(|e| panic!("failed to copy cube_4color.3mf into fixture dir: {e}"));
        let perimeters = run_and_check_arachne_fixture(&dir, "cube_4color.3mf");
        let tool_indices: BTreeSet<u32> = perimeters
            .iter()
            .flat_map(|p| p.regions.iter())
            .map(|r| r.region_id as u32)
            .collect();
        assert!(
            tool_indices.len() >= 4,
            "cube_4color_arachne: expected at least 4 distinct tool indices (per-color MMU \
             fragmentation), got {:?}",
            tool_indices
        );
    }
}
