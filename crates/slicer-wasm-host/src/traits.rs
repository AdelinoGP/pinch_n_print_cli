//! Runner trait definitions for the 4 stage families.
//!
//! Lifted from the per-stage executor files in `slicer-runtime` (packet 83 Step 4a-ii).
//! Signatures redesigned for the symmetric IR-typed trait boundary (packet 83 design.md):
//! IR-typed inputs via `*StageInput<'_>` borrow structs, IR-typed outputs via
//! `slicer-ir`'s `LayerStageCommitData` / `slicer-core`'s `PrepassStageOutput` / etc.
//! No `&Blackboard`, `&mut LayerArena`, or `slicer_wasm_host::HostExecutionContext` in
//! any trait signature.

use slicer_core::PrepassStageOutput;
use slicer_ir::{
    FinalizationError, FinalizationOutput, GCodeCommand, GlobalLayer, LayerCollectionIR,
    LayerStageCommitData, LayerStageError, PostpassError, PostpassOutput, PrepassRunnerError,
    StageId,
};

use crate::binding::{
    CompiledModuleLive, FinalizationStageInput, LayerStageInput, PostpassStageInput,
    PrepassStageInput,
};

/// Runner for layer-stage dispatch (infill, perimeter, seam, support, etc.).
pub trait LayerStageRunner {
    /// Execute one stage for one layer.
    fn run_stage(
        &self,
        stage_id: &StageId,
        layer: &GlobalLayer,
        module: &CompiledModuleLive<'_>,
        input: LayerStageInput<'_>,
    ) -> Result<LayerStageCommitData, LayerStageError>;

    /// Returns the last sampled (current, peak) WASM linear-memory usage in bytes.
    /// Implementations that do not instrument memory return `(0, 0)`.
    fn last_wasm_mem_sample(&self) -> (u64, u64) {
        (0, 0)
    }

    /// Returns the runtime-read field paths captured during the most recent
    /// `run_stage` call. Used by the executor to populate `ModuleAccessAudit.runtime_reads`.
    /// Default returns an empty `Vec` for runners that do not instrument reads.
    fn last_runtime_reads(&self) -> Vec<String> {
        Vec::new()
    }
}

/// Runner for prepass-stage dispatch (mesh analysis, support geometry, seam planning, etc.).
pub trait PrepassStageRunner {
    /// Execute one prepass stage for the whole model.
    fn run_stage(
        &self,
        stage_id: &StageId,
        module: &CompiledModuleLive<'_>,
        input: PrepassStageInput<'_>,
    ) -> Result<PrepassStageOutput, PrepassRunnerError>;

    /// Returns the runtime-read field paths captured during the most recent
    /// `run_stage` call. Used by the executor to populate `ModuleAccessAudit.runtime_reads`.
    /// Default returns an empty `Vec` for runners that do not instrument reads.
    fn last_runtime_reads(&self) -> Vec<String> {
        Vec::new()
    }
}

/// Runner for postpass-stage dispatch (G-code and text postprocessing).
pub trait PostpassStageRunner {
    /// Execute a G-code postprocessing stage, mutating `commands` in place.
    fn run_gcode_postprocess(
        &self,
        stage_id: &StageId,
        module: &CompiledModuleLive<'_>,
        input: PostpassStageInput<'_>,
        commands: &mut Vec<GCodeCommand>,
    ) -> Result<PostpassOutput, PostpassError>;

    /// Execute a text postprocessing stage, consuming and returning the G-code string.
    fn run_text_postprocess(
        &self,
        stage_id: &StageId,
        module: &CompiledModuleLive<'_>,
        input: PostpassStageInput<'_>,
        text: String,
    ) -> Result<PostpassOutput, PostpassError>;

    /// Drain any runtime-side read captures accumulated during postprocessing.
    /// Default returns an empty `Vec`.
    fn take_runtime_reads(&mut self) -> Vec<Vec<String>> {
        Vec::new()
    }
}

/// Runner for finalization-stage dispatch (layer-collection assembly).
pub trait FinalizationStageRunner {
    /// Execute one finalization stage, appending to `layers`.
    fn run_stage(
        &self,
        stage_id: &StageId,
        module: &CompiledModuleLive<'_>,
        input: FinalizationStageInput<'_>,
        layers: &mut Vec<LayerCollectionIR>,
    ) -> Result<FinalizationOutput, FinalizationError>;
}
