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

use std::fmt;

use slicer_ir::{GCodeIR, LayerCollectionIR, ModuleId, StageId};

use crate::{Blackboard, CompiledModule, ExecutionPlan};

/// Output produced by a single postpass module invocation.
#[derive(Debug, Clone, PartialEq)]
pub enum PostpassOutput {
    /// GCodePostProcess module completed successfully.
    GCodeSuccess,
    /// TextPostProcess module completed successfully, returning the final text.
    TextSuccess {
        /// The final G-code text produced by the module.
        text: String,
    },
    /// Module encountered a non-fatal error, continue with next module.
    NonFatalError {
        /// Stable human-readable detail.
        message: String,
    },
}

/// Fatal error from a postpass module or executor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostpassError {
    /// Fatal error from a module, abort postpass.
    FatalModule {
        /// Stage being executed.
        stage_id: StageId,
        /// Module that failed.
        module_id: ModuleId,
        /// Stable human-readable detail.
        message: String,
    },
    /// GCode emit failed.
    GCodeEmit {
        /// Stable human-readable detail.
        message: String,
    },
    /// GCode serialization failed.
    GCodeSerialization {
        /// Stable human-readable detail.
        message: String,
    },
}

impl fmt::Display for PostpassError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FatalModule {
                stage_id,
                module_id,
                message,
            } => write!(
                f,
                "fatal postpass module failure in {stage_id} for {module_id}: {message}"
            ),
            Self::GCodeEmit { message } => write!(f, "gcode emit failed: {message}"),
            Self::GCodeSerialization { message } => {
                write!(f, "gcode serialization failed: {message}")
            }
        }
    }
}

impl std::error::Error for PostpassError {}

/// Callback surface used by tests and future runtime bindings.
///
/// The runner is responsible for executing a single postpass module.
/// For GCodePostProcess stages, the runner mutates the provided GCodeIR.
/// For TextPostProcess stages, the runner receives the serialized text.
pub trait PostpassStageRunner {
    /// Execute one compiled GCodePostProcess module.
    ///
    /// The module may mutate `gcode_ir` in place.
    fn run_gcode_postprocess(
        &self,
        stage_id: &StageId,
        module: &CompiledModule,
        blackboard: &Blackboard,
        gcode_ir: &mut GCodeIR,
    ) -> Result<PostpassOutput, PostpassError>;

    /// Execute one compiled TextPostProcess module.
    ///
    /// The module receives the serialized G-code text and returns the modified text.
    fn run_text_postprocess(
        &self,
        stage_id: &StageId,
        module: &CompiledModule,
        blackboard: &Blackboard,
        text: String,
    ) -> Result<PostpassOutput, PostpassError>;
}

/// Trait for GCode emission (host-built-in, will be implemented in TASK-034).
pub trait GCodeEmitter {
    /// Emit GCode IR from layer collections.
    fn emit_gcode(
        &self,
        layer_irs: &[LayerCollectionIR],
        blackboard: &Blackboard,
    ) -> Result<GCodeIR, PostpassError>;
}

/// Trait for GCode serialization (host-built-in).
pub trait GCodeSerializer {
    /// Serialize GCodeIR to text.
    fn serialize_gcode(&self, gcode_ir: &GCodeIR) -> Result<String, PostpassError>;
}

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
/// The final G-code string, or an error if any stage fails fatally.
pub fn execute_postpass(
    plan: &ExecutionPlan,
    layer_irs: &[LayerCollectionIR],
    blackboard: &Blackboard,
    emitter: &dyn GCodeEmitter,
    serializer: &dyn GCodeSerializer,
    runner: &dyn PostpassStageRunner,
) -> Result<String, PostpassError> {
    // Step 1: Emit initial GCodeIR from layers
    let mut gcode_ir = emitter.emit_gcode(layer_irs, blackboard)?;

    // Step 2: Run all GCodePostProcess modules sequentially
    for stage in &plan.postpass_stages {
        if stage.stage_id.contains("GCodePostProcess") {
            for module in &stage.modules {
                match runner.run_gcode_postprocess(
                    &stage.stage_id,
                    module,
                    blackboard,
                    &mut gcode_ir,
                )? {
                    PostpassOutput::GCodeSuccess => {
                        // Continue to next module
                    }
                    PostpassOutput::NonFatalError { message: _ } => {
                        // Log warning but continue to next module
                    }
                    PostpassOutput::TextSuccess { text: _ } => {
                        // Unexpected from GCodePostProcess, but not fatal - continue
                    }
                }
            }
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
        return serializer.serialize_gcode(&gcode_ir);
    }

    // Step 4: Serialize GCodeIR to text for TextPostProcess modules
    let mut text = serializer.serialize_gcode(&gcode_ir)?;

    // Step 5: Run all TextPostProcess modules sequentially
    for stage in text_postprocess_stages {
        for module in &stage.modules {
            match runner.run_text_postprocess(&stage.stage_id, module, blackboard, text)? {
                PostpassOutput::TextSuccess { text: new_text } => {
                    text = new_text;
                }
                PostpassOutput::NonFatalError { message: _ } => {
                    // Log warning but continue - text remains unchanged from serialization
                    // Since we consumed `text` we need to re-serialize for the next module
                    text = serializer.serialize_gcode(&gcode_ir)?;
                }
                PostpassOutput::GCodeSuccess => {
                    // Unexpected from TextPostProcess, but not fatal
                    // Re-serialize since we consumed text
                    text = serializer.serialize_gcode(&gcode_ir)?;
                }
            }
        }
    }

    Ok(text)
}
