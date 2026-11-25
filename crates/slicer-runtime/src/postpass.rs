//! PostPass executor (TASK-033).
//!
//! This module defines the executor for post-layer-finalization stages
//! (GCodeEmit, GCodePostProcess, TextPostProcess). All stages run sequentially
//! with immutable access to layer_irs.
//!
//! Pipeline order:
//! 1. emit_gcode(layer_irs, blackboard) -> GCodeIR (host-built-in, TASK-034)
//! 2. Run all PostPassGCodePostProcess modules sequentially
//! 3. Run all PostPassTextPostProcess modules sequentially OR serialize GCodeIR
//!
//! Reference: docs/04_host_scheduler.md lines 778-810

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use slicer_gcode::{
    take_last_emit_estimate, CachedEmitEstimate, GCodeEmitError, GCodeEmitter, GCodeSerializer,
};
use slicer_ir::{
    ExtrusionRole, GCodeCommand, GCodeIR, LayerCollectionIR, ModuleId, PostpassError,
    PostpassOutput, StageId,
};

/// Compact 128-bit + count fingerprint over the kinematic commands of a
/// `GCodeCommand` stream (`Move`, `Retract`, `Unretract`, `ToolChange`, and
/// `Raw` `G92 E<val>` resets). Two streams with identical kinematic
/// fingerprints produce identical `estimate_print` outputs for any given
/// `(EstimatorLimits, tool_diameters)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KinematicFingerprint {
    count: u64,
    h1: u64,
    h2: u64,
}

#[inline]
fn mix_u64(state: &mut u64, prime: u64, val: u64) {
    *state ^= val;
    *state = state.wrapping_mul(prime);
}

#[inline]
fn feed_kinematic(h1: &mut u64, h2: &mut u64, val: u64) {
    mix_u64(h1, 0x0000_0100_0000_01b3, val);
    mix_u64(
        h2,
        0x9e37_79b9_7f4a_7c15,
        val.rotate_left(27) ^ 0xa076_1d64_78bd_642f,
    );
}

#[inline]
fn opt_f32_bits(v: Option<f32>) -> u64 {
    match v {
        Some(f) => 0x1_0000_0000 | (f.to_bits() as u64),
        None => 0,
    }
}

/// Compute the kinematic fingerprint of a `GCodeCommand` slice.
pub fn kinematic_fingerprint(commands: &[GCodeCommand]) -> KinematicFingerprint {
    let mut count = 0u64;
    let mut h1 = 0xcbf2_9ce4_8422_2325_u64;
    let mut h2 = 0x8422_2325_cbf2_9ce4_u64;

    for cmd in commands {
        match cmd {
            GCodeCommand::Move {
                x,
                y,
                z,
                e,
                f,
                role,
            } => {
                count += 1;
                let is_travel = matches!(role, ExtrusionRole::Custom(s) if s == "Travel");
                feed_kinematic(&mut h1, &mut h2, if is_travel { 0x11 } else { 0x10 });
                feed_kinematic(&mut h1, &mut h2, opt_f32_bits(*x));
                feed_kinematic(&mut h1, &mut h2, opt_f32_bits(*y));
                feed_kinematic(&mut h1, &mut h2, opt_f32_bits(*z));
                feed_kinematic(&mut h1, &mut h2, opt_f32_bits(*e));
                feed_kinematic(&mut h1, &mut h2, opt_f32_bits(*f));
            }
            GCodeCommand::Retract { length, speed, .. } => {
                count += 1;
                feed_kinematic(&mut h1, &mut h2, 0x20);
                feed_kinematic(&mut h1, &mut h2, length.to_bits() as u64);
                feed_kinematic(&mut h1, &mut h2, speed.to_bits() as u64);
            }
            GCodeCommand::Unretract { length, speed, .. } => {
                count += 1;
                feed_kinematic(&mut h1, &mut h2, 0x30);
                feed_kinematic(&mut h1, &mut h2, length.to_bits() as u64);
                feed_kinematic(&mut h1, &mut h2, speed.to_bits() as u64);
            }
            GCodeCommand::ToolChange { to, .. } => {
                count += 1;
                feed_kinematic(&mut h1, &mut h2, 0x40);
                feed_kinematic(&mut h1, &mut h2, *to as u64);
            }
            GCodeCommand::Raw { text } => {
                let trimmed = text.trim();
                if trimmed.starts_with("G92") {
                    if let Some(e_str) = trimmed
                        .split_whitespace()
                        .find(|tok| tok.starts_with('E') || tok.starts_with('e'))
                    {
                        if let Ok(val) = e_str[1..].parse::<f64>() {
                            count += 1;
                            feed_kinematic(&mut h1, &mut h2, 0x50);
                            feed_kinematic(&mut h1, &mut h2, val.to_bits());
                        }
                    }
                }
            }
            GCodeCommand::ExtrusionMode { .. }
            | GCodeCommand::Comment { .. }
            | GCodeCommand::FanSpeed { .. }
            | GCodeCommand::Temperature { .. } => {}
        }
    }

    KinematicFingerprint { count, h1, h2 }
}

/// Returns `true` iff `after_commands` has the exact same kinematic command
/// sequence as `before`, proving `GCodePostProcess` did not insert, delete,
/// reorder, or alter any kinematic command (`Move`, `Retract`, `Unretract`,
/// `ToolChange`, or `G92 E<val>` reset).
pub fn postpass_preserves_kinematics(
    before: &KinematicFingerprint,
    after_commands: &[GCodeCommand],
) -> bool {
    *before == kinematic_fingerprint(after_commands)
}

thread_local! {
    /// The final `GCodeIR` of the most recent successful postpass on this
    /// thread (after every `GCodePostProcess` module ran, i.e. exactly what
    /// was serialized). Surfaced this way — rather than through
    /// `PipelineOutput` — because `pipeline.rs` is frozen for packet 169
    /// Step 3; `run.rs` drains it via [`take_final_gcode_ir`] immediately
    /// after the pipeline returns (same thread: the pipeline, including
    /// postpass, executes synchronously on the caller's thread) to derive the
    /// `slice_stats` progress event via `slicer_gcode::estimate_print`
    /// without re-implementing any estimator math.
    static FINAL_GCODE_IR: RefCell<Option<GCodeIR>> = const { RefCell::new(None) };
    /// Optional cached `PrintEstimate` from `GCodeEmit` when `GCodePostProcess`
    /// proved kinematic invariance (`postpass_preserves_kinematics`), eliminating
    /// the redundant second `estimate_print` pass in `run.rs`.
    static FINAL_GCODE_ESTIMATE: RefCell<Option<CachedEmitEstimate>> = const { RefCell::new(None) };
}

fn stash_final_gcode_ir(ir: GCodeIR, cached_estimate: Option<CachedEmitEstimate>) {
    FINAL_GCODE_IR.with(|cell| *cell.borrow_mut() = Some(ir));
    FINAL_GCODE_ESTIMATE.with(|cell| *cell.borrow_mut() = cached_estimate);
}

/// Take (and clear) the final `GCodeIR` stashed by the most recent successful
/// postpass on this thread. `None` when no postpass has completed since the
/// last take. `gcode_ir.metadata.estimated_print_time_s` is already filled by
/// the emitter (`DefaultGCodeEmitter::emit_gcode`, packet 169 Step 2).
pub fn take_final_gcode_ir() -> Option<GCodeIR> {
    FINAL_GCODE_IR.with(|cell| cell.borrow_mut().take())
}

/// Take (and clear) the cached `PrintEstimate` stashed by the most recent
/// successful postpass on this thread (present iff `GCodeEmit` produced it and
/// `GCodePostProcess` preserved kinematics).
pub fn take_final_gcode_estimate() -> Option<CachedEmitEstimate> {
    FINAL_GCODE_ESTIMATE.with(|cell| cell.borrow_mut().take())
}

/// Translate a `slicer_gcode::GCodeEmitError` into the matching
/// `slicer_ir::PostpassError` variant.
///
/// Defined here (rather than as a `From` impl) because of Rust's orphan rule:
/// `PostpassError` lives in `slicer-ir` and `GCodeEmitError` lives in
/// `slicer-gcode`; neither type is owned by `slicer-runtime`. The conversion
/// is the runtime-side seam between the two crates, so it lives next to the
/// only call sites (`emit_gcode` / `serialize_gcode` inside
/// `execute_postpass_with_instrumentation`).
fn gcode_emit_error_to_postpass(e: GCodeEmitError) -> PostpassError {
    match e {
        GCodeEmitError::MissingToolchangePurge {
            layer_index,
            tool_change_index,
        } => PostpassError::MissingToolchangePurge {
            layer_index,
            tool_change_index,
        },
        GCodeEmitError::Emit(message) => PostpassError::GCodeEmit { message },
        GCodeEmitError::Serialization(message) => PostpassError::GCodeSerialization { message },
        GCodeEmitError::ToolIndexOutOfRange { tool, max } => PostpassError::GCodeEmit {
            message: format!("tool index {tool} out of range (max plausible {max})"),
        },
    }
}
use slicer_wasm_host::{
    CompiledModuleLive, PostpassStageInput, PostpassStageRunner, WasmComponent, WasmInstancePool,
};

use crate::instrumentation::{NoopInstrumentation, PipelineInstrumentation};
use crate::{Blackboard, ExecutionPlan, ModuleAccessAudit};

// PostpassStageRunner trait is now defined in slicer-wasm-host::traits and re-exported
// from slicer_runtime via the transitional re-exports block in lib.rs (P83 Step 4c+4d).
// Signature changes:
//   run_gcode_postprocess: module: &CompiledModule → &CompiledModuleLive<'_>,
//                          blackboard: &Blackboard removed,
//                          gcode_ir: &mut GCodeIR → commands: &mut Vec<GCodeCommand>
//   run_text_postprocess:  module: &CompiledModule → &CompiledModuleLive<'_>,
//                          blackboard: &Blackboard removed (mesh projected into PostpassStageInput)
//
// `GCodeEmitter` and `GCodeSerializer` traits live in `slicer-gcode` as of
// packet 86 Step 3; they are re-imported above so the postpass executor
// continues to accept `&dyn GCodeEmitter` / `&dyn GCodeSerializer` callers.

/// Executes the PostPass pipeline.
///
/// Pipeline stages:
/// 1. Call `emitter.emit_gcode()` to produce initial GCodeIR from layers
/// 2. For each module in PostPass::GCodePostProcess, call `runner.run_gcode_postprocess()`
/// 3. Either:
///    - If PostPass::TextPostProcess modules exist, serialize GCodeIR and run them
///    - Otherwise, serialize GCodeIR directly to produce final text
///
/// # Arguments
///
/// * `plan` - The execution plan containing postpass stages
/// * `layer_irs` - Immutable reference to finalized layer collections
/// * `blackboard` - The blackboard for read-only access to prepass IRs
/// * `emitter` - GCode emission implementation (host-built-in)
/// * `serializer` - GCode serialization implementation (host-built-in)
/// * `runner` - The stage runner for module execution
///
/// # Returns
///
/// The final G-code string and collected runtime access audits, or an error if
/// any stage fails fatally.
pub fn execute_postpass(
    plan: &ExecutionPlan,
    layer_irs: &[LayerCollectionIR],
    blackboard: &Blackboard,
    emitter: &dyn GCodeEmitter,
    serializer: &dyn GCodeSerializer,
    runner: &mut dyn PostpassStageRunner,
    wasm_handles: &HashMap<
        ModuleId,
        (
            Arc<WasmInstancePool>,
            Option<Arc<WasmComponent>>,
            Option<slicer_sdk::native::NativeStageEntry>,
        ),
    >,
) -> Result<(String, Vec<ModuleAccessAudit>), PostpassError> {
    execute_postpass_with_instrumentation(
        plan,
        layer_irs,
        blackboard,
        emitter,
        serializer,
        runner,
        &NoopInstrumentation,
        wasm_handles,
    )
}

/// Drain the profiling marks and fuel totals a postpass dispatch left in the
/// runner's per-call stashes and hand them to `instrumentation` (ADR-0055).
///
/// Unlike the layer/prepass path — where `last_profile_marks` is a thread-local
/// holding exactly the last call — `PostpassStageRunner` accumulates one entry
/// per dispatch and drains them all at once. Calling this right after each
/// dispatch means the stash holds exactly one call's worth, which is what makes
/// the attribution to `module_id` correct; letting entries pile up and draining
/// at the end would leave no way to say which module produced which entry.
///
/// The mark vectors are flattened because a single postpass dispatch produces
/// one entry; a runner that pushed several would still be reported under the
/// same module, which is the truthful answer.
fn drain_postpass_profile(
    runner: &mut dyn PostpassStageRunner,
    instrumentation: &(dyn PipelineInstrumentation + Sync),
    stage_id: &StageId,
    module_id: &ModuleId,
) {
    let marks: Vec<slicer_wasm_host::profiling::ProfileMark> =
        runner.take_profile_marks().into_iter().flatten().collect();
    let call_fuel: u64 = runner.take_call_fuel().into_iter().sum();
    if marks.is_empty() && call_fuel == 0 {
        return;
    }
    instrumentation.on_module_profile(stage_id, None, module_id, &marks, call_fuel);
}

/// Instrumented variant of [`execute_postpass`] that brackets each
/// postpass stage and module (including host built-ins) via
/// `instrumentation`.
pub fn execute_postpass_with_instrumentation(
    plan: &ExecutionPlan,
    layer_irs: &[LayerCollectionIR],
    blackboard: &Blackboard,
    emitter: &dyn GCodeEmitter,
    serializer: &dyn GCodeSerializer,
    runner: &mut dyn PostpassStageRunner,
    instrumentation: &(dyn PipelineInstrumentation + Sync),
    wasm_handles: &HashMap<
        ModuleId,
        (
            Arc<WasmInstancePool>,
            Option<Arc<WasmComponent>>,
            Option<slicer_sdk::native::NativeStageEntry>,
        ),
    >,
) -> Result<(String, Vec<ModuleAccessAudit>), PostpassError> {
    execute_postpass_with_capture(
        plan,
        layer_irs,
        blackboard,
        emitter,
        serializer,
        runner,
        instrumentation,
        wasm_handles,
        None,
    )
}

/// Renderer-owned, read-only snapshot of the whole-print PostPass IR
/// (packet 161, Step 5): the finalized `Vec<LayerCollectionIR>` (after
/// [`slicer_gcode::reconcile_finalization_travel`], immediately before
/// G-code emission — tap `PostPass::LayerFinalization`) paired with the
/// initially emitted `GCodeIR` (right after `emitter.emit_gcode()`, before
/// any `GCodePostProcess` module mutates it — tap `PostPass::GCodeEmit`).
///
/// Populated only when a caller passes `Some(&mut PostPassCapture)` to
/// [`execute_postpass_with_capture`]; the finalized layers are moved into the
/// capture and the `GCodeIR` is cloned once, immediately, from the locals
/// already computed for ordinary emission — capturing never alters what is
/// emitted (ADR-0037 "never a live borrow retained past this call", mirrored
/// here for the whole-print tier).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PostPassCapture {
    /// The finalized, travel-reconciled layers fed into `emit_gcode`.
    pub finalized_layers: Vec<LayerCollectionIR>,
    /// The `GCodeIR` as initially emitted, before any `GCodePostProcess`
    /// module runs.
    pub gcode_ir: GCodeIR,
}

/// [`execute_postpass_with_instrumentation`] with an optional read-only
/// capture sink (packet 161, Step 5). Ordinary callers (the production
/// pipeline via [`execute_postpass_with_instrumentation`], and every
/// existing test via [`execute_postpass`]) pass `None` and are byte-for-byte
/// unaffected — the sink only receives already-computed locals, it never
/// alters emission. `pnp-cli`'s visual-debug PostPass taps are the only
/// caller that passes `Some`; because they need the layers after emission,
/// they take this borrowing entry point's copy and the owned-emission
/// shortcut applies only when no capture is requested.
#[allow(clippy::too_many_arguments)]
pub fn execute_postpass_with_capture(
    plan: &ExecutionPlan,
    layer_irs: &[LayerCollectionIR],
    blackboard: &Blackboard,
    emitter: &dyn GCodeEmitter,
    serializer: &dyn GCodeSerializer,
    runner: &mut dyn PostpassStageRunner,
    instrumentation: &(dyn PipelineInstrumentation + Sync),
    wasm_handles: &HashMap<
        ModuleId,
        (
            Arc<WasmInstancePool>,
            Option<Arc<WasmComponent>>,
            Option<slicer_sdk::native::NativeStageEntry>,
        ),
    >,
    capture: Option<&mut PostPassCapture>,
) -> Result<(String, Vec<ModuleAccessAudit>), PostpassError> {
    execute_postpass_with_owned_capture(
        plan,
        layer_irs.to_vec(),
        blackboard,
        emitter,
        serializer,
        runner,
        instrumentation,
        wasm_handles,
        capture,
    )
}

/// [`execute_postpass_with_capture`] taking the finalized layers by value, so
/// the pre-emit travel reconciliation mutates the production caller's own
/// vector in place instead of paying one extra whole-print clone + drop
/// (`PostPass::GCodeEmit` perf finding, 2026-10-09 refresh). Behavioral
/// contract is unchanged: the reconciled layer contents are identical to the
/// copy-then-mutate form, callers take ownership and never see the layers
/// again (`run_pipeline_core` drops `layer_irs` right after), and capture
/// sinks still receive an independent clone.
#[allow(clippy::too_many_arguments)]
pub fn execute_postpass_with_owned_capture(
    plan: &ExecutionPlan,
    layer_irs: Vec<LayerCollectionIR>,
    blackboard: &Blackboard,
    emitter: &dyn GCodeEmitter,
    serializer: &dyn GCodeSerializer,
    runner: &mut dyn PostpassStageRunner,
    instrumentation: &(dyn PipelineInstrumentation + Sync),
    wasm_handles: &HashMap<
        ModuleId,
        (
            Arc<WasmInstancePool>,
            Option<Arc<WasmComponent>>,
            Option<slicer_sdk::native::NativeStageEntry>,
        ),
    >,
    capture: Option<&mut PostPassCapture>,
) -> Result<(String, Vec<ModuleAccessAudit>), PostpassError> {
    // Step 1a: Reconcile finalization-aware travel moves before emission.
    // This adjusts travel_moves to route through Skirt/Brim and WipeTower
    // geometry without modifying ordered_entities. The layers arrive owned;
    // reconciling in place avoids the historical whole-print `to_vec` copy.
    let mut reconciled_layers: Vec<LayerCollectionIR> = layer_irs;
    let travel_f = emitter.travel_feedrate_mm_per_min();
    for layer in &mut reconciled_layers {
        slicer_gcode::reconcile_finalization_travel(layer, travel_f);
    }

    // Step 1b: Emit initial GCodeIR from (reconciled) layers.
    //
    // Ownership: the production pipeline hands the collection over and never
    // reads it again, so the emitter takes it (`emit_gcode_owned`) and the
    // collection is dropped exactly once, inside this bracket, instead of being
    // deep-cloned here and dropped a second time at postpass teardown (t90;
    // supports-on collections make both operations seconds of host time).
    // The visual-debug capture sinks are the one caller that still needs the
    // layers afterwards: they take the borrowed path and move the collection
    // into the capture, which is the same content an independent clone carried
    // before.
    let emit_stage = "PostPass::GCodeEmit".to_string();
    let emit_module = "host:gcode_emit".to_string();
    instrumentation.on_stage_start(&emit_stage, None);
    instrumentation.on_module_start(&emit_stage, None, &emit_module);
    let emit_result = if capture.is_some() {
        emitter
            .emit_gcode(&reconciled_layers)
            .map(|ir| (ir, Some(reconciled_layers)))
    } else {
        emitter
            .emit_gcode_owned(reconciled_layers)
            .map(|ir| (ir, None))
    };
    let (mut gcode_ir, captured_layers) = emit_result
        .map_err(gcode_emit_error_to_postpass)
        .inspect_err(|_| {
            instrumentation.on_stage_end(&emit_stage, None);
        })?;
    instrumentation.on_module_end(&emit_stage, None, &emit_module, 0, 0);
    instrumentation.on_stage_end(&emit_stage, None);

    // Step 1c (packet 161, Step 5): read-only capture of the finalized
    // layers (`PostPass::LayerFinalization`) and the just-emitted GCodeIR
    // (`PostPass::GCodeEmit`), taken before any GCodePostProcess module gets
    // a chance to mutate `gcode_ir.commands`. No-op for every ordinary
    // caller (`capture` is `None`); the one caller that captures receives the
    // layer collection itself (the emitter borrows it on that path), so the
    // capture costs no whole-print copy.
    if let Some(cap) = capture {
        if let Some(layers) = captured_layers {
            cap.finalized_layers = layers;
        }
        cap.gcode_ir = gcode_ir.clone();
    }

    let mut cached_estimate = take_last_emit_estimate();
    let has_gcode_postprocess = plan
        .postpass_stages
        .iter()
        .any(|s| s.stage_id.contains("GCodePostProcess") && !s.modules.is_empty());
    let pre_postprocess_fp = if cached_estimate.is_some() && has_gcode_postprocess {
        Some(kinematic_fingerprint(&gcode_ir.commands))
    } else {
        None
    };

    let mut audits = Vec::new();

    // Step 2: Run all GCodePostProcess modules sequentially
    for stage in &plan.postpass_stages {
        if stage.stage_id.contains("GCodePostProcess") {
            instrumentation.on_stage_start(&stage.stage_id, None);
            for module in &stage.modules {
                instrumentation.on_module_start(&stage.stage_id, None, module.module_id());
                // Build IR-typed borrow structs for the new slicer-wasm-host trait boundary.
                let (instance_pool, wasm_component, native_entry) = wasm_handles
                    .get(module.module_id().as_str())
                    .map(|(p, c, e)| (Arc::clone(p), c.clone(), *e))
                    .unwrap_or_else(|| (WasmInstancePool::placeholder(), None, None));
                let mut live_module = CompiledModuleLive::new(
                    module.module_id(),
                    instance_pool,
                    wasm_component,
                    module.claims(),
                    Arc::clone(module.config_view()),
                );
                if let Some(entry) = native_entry {
                    live_module = live_module.with_native_entry(entry);
                }
                let input = PostpassStageInput {
                    mesh: std::sync::Arc::clone(blackboard.mesh()),
                    _phantom: std::marker::PhantomData,
                };
                let res = runner.run_gcode_postprocess(
                    &stage.stage_id,
                    &live_module,
                    input,
                    &mut gcode_ir.commands,
                );
                // Fuel/scope marks (ADR-0055). Drained per dispatch rather than
                // once at the end like `take_batch_calls`: the per-dispatcher
                // stash is a `Vec` per call, and a batch drain would have no way
                // to say which module each entry came from.
                drain_postpass_profile(
                    runner,
                    instrumentation,
                    &stage.stage_id,
                    module.module_id(),
                );
                instrumentation.on_module_end(&stage.stage_id, None, module.module_id(), 0, 0);
                // Drain module log messages (already forwarded to the human
                // `log` facade inside the dispatcher; this clears the
                // thread-local stash) and forward each onto the structured
                // stream as a `module_log` event. Drained here rather than
                // inside the `GCodeSuccess` arm so the non-fatal, text-output,
                // and error paths clear the stash too — otherwise a module's
                // records leak into the next module's drain.
                for (level, message) in runner.last_log_messages() {
                    instrumentation.on_module_log(
                        &stage.stage_id,
                        None,
                        module.module_id(),
                        &level,
                        &message,
                    );
                }
                let result = match res {
                    Ok(r) => r,
                    Err(e) => {
                        instrumentation.on_module_error(
                            &stage.stage_id,
                            None,
                            module.module_id(),
                            &e.to_string(),
                            true,
                        );
                        instrumentation.on_stage_end(&stage.stage_id, None);
                        return Err(e);
                    }
                };
                match result {
                    PostpassOutput::GCodeSuccess => {
                        // Record runtime audit for GCodePostProcess modules.
                        // runtime_reads are drained via take_runtime_reads().
                        let runtime_reads = runner.take_runtime_reads();
                        let reads = runtime_reads.into_iter().flatten().collect();
                        let batch_calls = runner.take_batch_calls().into_iter().flatten().collect();
                        audits.push(ModuleAccessAudit {
                            module_id: module.module_id().to_owned(),
                            runtime_reads: reads,
                            runtime_writes: vec![String::from("GCodeIR")],
                            batch_calls,
                            diagnostics: Vec::new(),
                        });
                    }
                    PostpassOutput::NonFatalError { message: _ } => {
                        // Log warning but continue to next module
                    }
                    PostpassOutput::TextSuccess { text: _ } => {
                        // Unexpected from GCodePostProcess, but not fatal - continue
                    }
                }
            }
            instrumentation.on_stage_end(&stage.stage_id, None);
        }
    }

    if let Some(before_fp) = pre_postprocess_fp {
        if !postpass_preserves_kinematics(&before_fp, &gcode_ir.commands) {
            cached_estimate = None;
        }
    }

    // Step 3: Check if any TextPostProcess modules exist
    let text_postprocess_stages: Vec<_> = plan
        .postpass_stages
        .iter()
        .filter(|s| s.stage_id.contains("TextPostProcess"))
        .collect();

    if text_postprocess_stages.is_empty() {
        // No TextPostProcess modules - serialize directly
        let ser_stage = "PostPass::GCodeSerialize".to_string();
        let ser_module = "host:gcode_serialize".to_string();
        instrumentation.on_stage_start(&ser_stage, None);
        instrumentation.on_module_start(&ser_stage, None, &ser_module);
        let text = serializer
            .serialize_gcode(&gcode_ir)
            .map_err(gcode_emit_error_to_postpass)
            .inspect_err(|_| {
                instrumentation.on_stage_end(&ser_stage, None);
            })?;
        instrumentation.on_module_end(&ser_stage, None, &ser_module, 0, 0);
        instrumentation.on_stage_end(&ser_stage, None);
        stash_final_gcode_ir(gcode_ir, cached_estimate);
        return Ok((text, audits));
    }

    // Step 4: Serialize GCodeIR to text for TextPostProcess modules
    let ser_stage = "PostPass::GCodeSerialize".to_string();
    let ser_module = "host:gcode_serialize".to_string();
    instrumentation.on_stage_start(&ser_stage, None);
    instrumentation.on_module_start(&ser_stage, None, &ser_module);
    let mut text = serializer
        .serialize_gcode(&gcode_ir)
        .map_err(gcode_emit_error_to_postpass)
        .inspect_err(|_| {
            instrumentation.on_stage_end(&ser_stage, None);
        })?;
    instrumentation.on_module_end(&ser_stage, None, &ser_module, 0, 0);
    instrumentation.on_stage_end(&ser_stage, None);

    // Step 5: Run all TextPostProcess modules sequentially
    for stage in text_postprocess_stages {
        instrumentation.on_stage_start(&stage.stage_id, None);
        for module in &stage.modules {
            instrumentation.on_module_start(&stage.stage_id, None, module.module_id());
            let (instance_pool, wasm_component, native_entry) = wasm_handles
                .get(module.module_id().as_str())
                .map(|(p, c, e)| (Arc::clone(p), c.clone(), *e))
                .unwrap_or_else(|| (WasmInstancePool::placeholder(), None, None));
            let mut live_module = CompiledModuleLive::new(
                module.module_id(),
                instance_pool,
                wasm_component,
                module.claims(),
                Arc::clone(module.config_view()),
            );
            if let Some(entry) = native_entry {
                live_module = live_module.with_native_entry(entry);
            }
            let input = PostpassStageInput {
                mesh: std::sync::Arc::clone(blackboard.mesh()),
                _phantom: std::marker::PhantomData,
            };
            let res = runner.run_text_postprocess(&stage.stage_id, &live_module, input, text);
            drain_postpass_profile(runner, instrumentation, &stage.stage_id, module.module_id());
            instrumentation.on_module_end(&stage.stage_id, None, module.module_id(), 0, 0);
            let result = match res {
                Ok(r) => r,
                Err(e) => {
                    instrumentation.on_module_error(
                        &stage.stage_id,
                        None,
                        module.module_id(),
                        &e.to_string(),
                        true,
                    );
                    instrumentation.on_stage_end(&stage.stage_id, None);
                    return Err(e);
                }
            };
            match result {
                PostpassOutput::TextSuccess { text: new_text } => {
                    // Record runtime audit for TextPostProcess modules.
                    // Extract runtime reads collected during this dispatch call.
                    // TextPostProcess modules produce final text output.
                    let runtime_reads = runner.take_runtime_reads();
                    let reads = runtime_reads.into_iter().flatten().collect();
                    let batch_calls = runner.take_batch_calls().into_iter().flatten().collect();
                    audits.push(ModuleAccessAudit {
                        module_id: module.module_id().to_owned(),
                        runtime_reads: reads,
                        runtime_writes: vec![String::from("GCodeIR")],
                        batch_calls,
                        diagnostics: Vec::new(),
                    });
                    text = new_text;
                }
                PostpassOutput::NonFatalError { message: _ } => {
                    // Log warning but continue - text remains unchanged from serialization
                    // Since we consumed `text` we need to re-serialize for the next module
                    let ser_stage = "PostPass::GCodeSerialize".to_string();
                    let ser_module = "host:gcode_serialize".to_string();
                    instrumentation.on_stage_start(&ser_stage, None);
                    instrumentation.on_module_start(&ser_stage, None, &ser_module);
                    text = serializer
                        .serialize_gcode(&gcode_ir)
                        .map_err(gcode_emit_error_to_postpass)
                        .inspect_err(|_| {
                            instrumentation.on_stage_end(&ser_stage, None);
                        })?;
                    instrumentation.on_module_end(&ser_stage, None, &ser_module, 0, 0);
                    instrumentation.on_stage_end(&ser_stage, None);
                }
                PostpassOutput::GCodeSuccess => {
                    // Unexpected from TextPostProcess, but not fatal
                    // Re-serialize since we consumed text
                    let ser_stage = "PostPass::GCodeSerialize".to_string();
                    let ser_module = "host:gcode_serialize".to_string();
                    instrumentation.on_stage_start(&ser_stage, None);
                    instrumentation.on_module_start(&ser_stage, None, &ser_module);
                    text = serializer
                        .serialize_gcode(&gcode_ir)
                        .map_err(gcode_emit_error_to_postpass)
                        .inspect_err(|_| {
                            instrumentation.on_stage_end(&ser_stage, None);
                        })?;
                    instrumentation.on_module_end(&ser_stage, None, &ser_module, 0, 0);
                    instrumentation.on_stage_end(&ser_stage, None);
                }
            }
        }
        instrumentation.on_stage_end(&stage.stage_id, None);
    }

    stash_final_gcode_ir(gcode_ir, cached_estimate);
    Ok((text, audits))
}

#[cfg(test)]
mod kinematic_fingerprint_tests {
    use super::*;

    fn sample_stream() -> Vec<GCodeCommand> {
        vec![
            GCodeCommand::ExtrusionMode { absolute: false },
            GCodeCommand::Move {
                x: Some(0.0),
                y: Some(0.0),
                z: Some(0.2),
                e: None,
                f: Some(6000.0),
                role: ExtrusionRole::Custom("Travel".to_string()),
            },
            GCodeCommand::Move {
                x: Some(10.0),
                y: Some(0.0),
                z: Some(0.2),
                e: Some(0.4),
                f: Some(3000.0),
                role: ExtrusionRole::OuterWall,
            },
            GCodeCommand::Retract {
                length: 0.8,
                speed: 2400.0,
                mode: slicer_ir::RetractMode::Gcode,
            },
        ]
    }

    #[test]
    fn postpass_preserves_kinematics_accepts_non_kinematic_wrappers() {
        let base = sample_stream();
        let before = kinematic_fingerprint(&base);
        let mut wrapped = vec![
            GCodeCommand::Raw {
                text: "; machine_start_gcode\nM140 S60".to_string(),
            },
            GCodeCommand::Comment {
                text: "custom header".to_string(),
            },
        ];
        wrapped.extend(base);
        wrapped.push(GCodeCommand::FanSpeed { value: 0 });
        wrapped.push(GCodeCommand::Raw {
            text: "; machine_end_gcode\nM84".to_string(),
        });
        assert!(postpass_preserves_kinematics(&before, &wrapped));
    }

    #[test]
    fn postpass_preserves_kinematics_rejects_any_kinematic_mutation() {
        let base = sample_stream();
        let before = kinematic_fingerprint(&base);

        // Inserted Move
        let mut inserted = base.clone();
        inserted.push(GCodeCommand::Move {
            x: Some(15.0),
            y: Some(0.0),
            z: Some(0.2),
            e: Some(0.6),
            f: Some(3000.0),
            role: ExtrusionRole::OuterWall,
        });
        assert!(!postpass_preserves_kinematics(&before, &inserted));

        // Altered feedrate
        let mut altered_f = base.clone();
        if let GCodeCommand::Move { f, .. } = &mut altered_f[2] {
            *f = Some(2500.0);
        }
        assert!(!postpass_preserves_kinematics(&before, &altered_f));

        // Altered E
        let mut altered_e = base.clone();
        if let GCodeCommand::Move { e, .. } = &mut altered_e[2] {
            *e = Some(0.5);
        }
        assert!(!postpass_preserves_kinematics(&before, &altered_e));

        // Reordered moves
        let mut reordered = base.clone();
        reordered.swap(1, 2);
        assert!(!postpass_preserves_kinematics(&before, &reordered));

        // Inserted G92 E0 reset
        let mut with_g92 = base;
        with_g92.push(GCodeCommand::Raw {
            text: "G92 E0".to_string(),
        });
        assert!(!postpass_preserves_kinematics(&before, &with_g92));
    }
}
