//! `PipelineInstrumentation` adapter that emits phase-, layer-, stage- and
//! module-grained progress events to the same `RuntimeProgressSink` the rest
//! of the slicer uses.
//!
//! Runs in one of two tiers ([`ProgressTier`]):
//! - `Core` (the default for every `slice` run): emits the docs/09 core
//!   contract events — `phase_*`, `layer_*`, `module_error`.
//! - `Instrumented` (`--instrument-stderr`): additionally emits
//!   `stage_start`/`stage_complete`/`module_start`/`module_complete`
//!   timing events (see `docs/specs/agent-cli-debugging.md` §4.2
//!   "supersedes" rule).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use slicer_ir::{ModuleId, StageId};

use crate::instrumentation::{Phase, PipelineInstrumentation, SerialEdge, TierKind};
use crate::layer_executor::LayerProgressSink;
use crate::progress_events::{ProgressEvent, ProgressPhase};

/// Convert the bracket trait's `Phase` to the wire `ProgressPhase`.
fn to_progress_phase(phase: Phase) -> ProgressPhase {
    match phase {
        Phase::PrePass => ProgressPhase::Prepass,
        Phase::PerLayer => ProgressPhase::PerLayer,
        Phase::PostPass => ProgressPhase::Postpass,
    }
}

/// Convert raw `wasm_peak_bytes` to ceiling-rounded KiB.
///
/// `wasm_peak_bytes == 0` (host built-ins) maps to `0`. Any non-zero value
/// rounds up so we never report a smaller footprint than the runtime saw.
fn wasm_bytes_to_kb(bytes: u64) -> u64 {
    if bytes == 0 {
        0
    } else {
        bytes.div_ceil(1024)
    }
}

/// Infer the wire phase from a stage id by prefix.
///
/// Stage ids in `STAGE_ORDER` carry their tier as a prefix
/// (`PrePass::`, `Layer::`, `PostPass::`). Unknown prefixes default to
/// `PerLayer` — the bracket-shaped trait calls `on_phase_start` first, so
/// in practice the phase is already known by the time stage callbacks fire,
/// but this fallback keeps the adapter robust if the order is ever changed.
fn phase_from_stage(stage: &StageId) -> ProgressPhase {
    let s = stage.as_str();
    if s.starts_with("PrePass::") {
        ProgressPhase::Prepass
    } else if s.starts_with("PostPass::") {
        ProgressPhase::Postpass
    } else {
        ProgressPhase::PerLayer
    }
}

/// Adapter that bridges the bracket-shaped `PipelineInstrumentation` trait
/// onto the existing `LayerProgressSink` so live timing flows out as JSONL.
///
/// Holds per-(stage, layer) and per-(module, stage, layer) start-time tables
/// so it can compute `elapsed_ms` at the matching `*_end` callback. Mutex
/// locks are held only across `HashMap::insert`/`remove`; the JSON emission
/// happens outside the lock to avoid blocking the scheduler hot path.
pub struct ProgressPipelineInstrumentation {
    sink: Arc<dyn LayerProgressSink + Send + Sync>,
    slice_id: String,
    tier: ProgressTier,
    phase_starts: Mutex<HashMap<Phase, Instant>>,
    stage_starts: Mutex<HashMap<(String, Option<u32>), Instant>>,
    module_starts: Mutex<HashMap<(String, String, Option<u32>), Instant>>,
    layer_starts: Mutex<HashMap<u32, Instant>>,
}

/// Emission tier for [`ProgressPipelineInstrumentation`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressTier {
    /// Default stream: core-contract events only (phase/layer/module_error).
    Core,
    /// `--instrument-stderr`: core plus stage/module timing events.
    Instrumented,
}

/// Unix epoch time in milliseconds (0 if the clock is before the epoch).
pub(crate) fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl ProgressPipelineInstrumentation {
    /// Build an adapter that forwards events to `sink` with the given
    /// `slice_id` stamped on every event, at the `Instrumented` tier.
    pub fn new(sink: Arc<dyn LayerProgressSink + Send + Sync>, slice_id: String) -> Self {
        Self::with_tier(sink, slice_id, ProgressTier::Instrumented)
    }

    /// Build an adapter with an explicit emission tier.
    pub fn with_tier(
        sink: Arc<dyn LayerProgressSink + Send + Sync>,
        slice_id: String,
        tier: ProgressTier,
    ) -> Self {
        Self {
            sink,
            slice_id,
            tier,
            phase_starts: Mutex::new(HashMap::new()),
            stage_starts: Mutex::new(HashMap::new()),
            module_starts: Mutex::new(HashMap::new()),
            layer_starts: Mutex::new(HashMap::new()),
        }
    }

    fn now_unix_ms() -> u64 {
        now_unix_ms()
    }
}

impl PipelineInstrumentation for ProgressPipelineInstrumentation {
    fn on_phase_start(&self, phase: Phase) {
        self.phase_starts
            .lock()
            .expect("phase_starts poisoned")
            .insert(phase, Instant::now());
        self.sink.record(ProgressEvent::phase_start(
            self.slice_id.clone(),
            to_progress_phase(phase),
            Self::now_unix_ms(),
        ));
    }

    fn on_phase_end(&self, phase: Phase) {
        let elapsed_ms = self
            .phase_starts
            .lock()
            .expect("phase_starts poisoned")
            .remove(&phase)
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);
        self.sink.record(ProgressEvent::phase_complete(
            self.slice_id.clone(),
            to_progress_phase(phase),
            Self::now_unix_ms(),
            elapsed_ms,
            crate::progress_events::ProgressStatus::Ok,
        ));
    }

    fn on_stage_start(&self, stage: &StageId, layer: Option<u32>) {
        if self.tier == ProgressTier::Core {
            return;
        }
        let key = (stage.to_string(), layer);
        self.stage_starts
            .lock()
            .expect("stage_starts poisoned")
            .insert(key, Instant::now());
        self.sink.record(ProgressEvent::stage_start(
            self.slice_id.clone(),
            phase_from_stage(stage),
            stage.to_string(),
            layer,
            Self::now_unix_ms(),
        ));
    }

    fn on_stage_end(&self, stage: &StageId, layer: Option<u32>) {
        if self.tier == ProgressTier::Core {
            return;
        }
        let key = (stage.to_string(), layer);
        let elapsed_ms = self
            .stage_starts
            .lock()
            .expect("stage_starts poisoned")
            .remove(&key)
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);
        self.sink.record(ProgressEvent::stage_complete(
            self.slice_id.clone(),
            phase_from_stage(stage),
            stage.to_string(),
            layer,
            Self::now_unix_ms(),
            elapsed_ms,
        ));
    }

    fn on_module_start(&self, stage: &StageId, layer: Option<u32>, module: &ModuleId) {
        if self.tier == ProgressTier::Core {
            return;
        }
        let key = (module.to_string(), stage.to_string(), layer);
        self.module_starts
            .lock()
            .expect("module_starts poisoned")
            .insert(key, Instant::now());
        self.sink.record(ProgressEvent::module_start(
            self.slice_id.clone(),
            phase_from_stage(stage),
            stage.to_string(),
            module.to_string(),
            layer,
            Self::now_unix_ms(),
        ));
    }

    fn on_module_end(
        &self,
        stage: &StageId,
        layer: Option<u32>,
        module: &ModuleId,
        _wasm_initial_bytes: u64,
        wasm_peak_bytes: u64,
    ) {
        if self.tier == ProgressTier::Core {
            return;
        }
        let key = (module.to_string(), stage.to_string(), layer);
        let elapsed_ms = self
            .module_starts
            .lock()
            .expect("module_starts poisoned")
            .remove(&key)
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);
        self.sink.record(ProgressEvent::module_complete(
            self.slice_id.clone(),
            phase_from_stage(stage),
            stage.to_string(),
            module.to_string(),
            layer,
            Self::now_unix_ms(),
            elapsed_ms,
            wasm_bytes_to_kb(wasm_peak_bytes),
        ));
    }

    fn on_layer_start(&self, layer: u32, _z_mm: f32) {
        self.layer_starts
            .lock()
            .expect("layer_starts poisoned")
            .insert(layer, Instant::now());
        self.sink.record(ProgressEvent::layer_start(
            self.slice_id.clone(),
            ProgressPhase::PerLayer,
            layer,
            Self::now_unix_ms(),
        ));
    }

    fn on_layer_end(&self, layer: u32) {
        let elapsed_ms = self
            .layer_starts
            .lock()
            .expect("layer_starts poisoned")
            .remove(&layer)
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);
        self.sink.record(ProgressEvent::layer_complete(
            self.slice_id.clone(),
            ProgressPhase::PerLayer,
            layer,
            Self::now_unix_ms(),
            elapsed_ms,
            crate::progress_events::ProgressStatus::Ok,
            false,
        ));
    }

    fn on_module_error(
        &self,
        stage: &StageId,
        layer: Option<u32>,
        module: &ModuleId,
        message: &str,
        fatal: bool,
    ) {
        self.sink.record(ProgressEvent::module_error(
            self.slice_id.clone(),
            phase_from_stage(stage),
            stage.to_string(),
            layer,
            module.to_string(),
            Self::now_unix_ms(),
            crate::progress_events::ProgressError {
                code: crate::progress_events::MODULE_DISPATCH_FATAL_CODE,
                message: message.to_string(),
                fatal,
                suggestion: None,
                reason: None,
            },
        ));
    }

    fn record_edges(&self, _stage: &StageId, _tier: TierKind, _edges: &[SerialEdge]) {
        // No corresponding wire event in the JSONL schema. Edges are surfaced
        // via the `dag` subcommand instead.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer_executor::LayerProgressSink;
    use crate::progress_events::ProgressEventType;
    use std::sync::Mutex;

    #[derive(Default)]
    struct RecordingSink {
        events: Mutex<Vec<ProgressEvent>>,
    }

    impl LayerProgressSink for RecordingSink {
        fn record(&self, event: ProgressEvent) {
            self.events.lock().unwrap().push(event);
        }
    }

    #[test]
    fn wasm_bytes_to_kb_rounds_up() {
        assert_eq!(wasm_bytes_to_kb(0), 0);
        assert_eq!(wasm_bytes_to_kb(1), 1);
        assert_eq!(wasm_bytes_to_kb(1023), 1);
        assert_eq!(wasm_bytes_to_kb(1024), 1);
        assert_eq!(wasm_bytes_to_kb(1025), 2);
        assert_eq!(wasm_bytes_to_kb(2 * 1024), 2);
        assert_eq!(wasm_bytes_to_kb(2 * 1024 + 1), 3);
    }

    #[test]
    fn phase_mapping_covers_all_variants() {
        assert_eq!(to_progress_phase(Phase::PrePass), ProgressPhase::Prepass);
        assert_eq!(to_progress_phase(Phase::PerLayer), ProgressPhase::PerLayer);
        assert_eq!(to_progress_phase(Phase::PostPass), ProgressPhase::Postpass);
    }

    #[test]
    fn phase_from_stage_uses_prefix() {
        assert_eq!(
            phase_from_stage(&StageId::from("PrePass::MeshAnalysis")),
            ProgressPhase::Prepass
        );
        assert_eq!(
            phase_from_stage(&StageId::from("Layer::Infill")),
            ProgressPhase::PerLayer
        );
        assert_eq!(
            phase_from_stage(&StageId::from("PostPass::GCodeEmit")),
            ProgressPhase::Postpass
        );
    }

    #[test]
    fn stage_start_emits_correct_event() {
        let sink = Arc::new(RecordingSink::default());
        let pi = ProgressPipelineInstrumentation::new(
            sink.clone() as Arc<dyn LayerProgressSink + Send + Sync>,
            "slice-test".to_string(),
        );
        let stage = StageId::from("PrePass::MeshAnalysis");
        pi.on_stage_start(&stage, None);

        let events = sink.events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event, ProgressEventType::StageStart);
        assert_eq!(events[0].stage.as_deref(), Some("PrePass::MeshAnalysis"));
        assert_eq!(events[0].phase, Some(ProgressPhase::Prepass));
        assert_eq!(events[0].layer_index, None);
        assert_eq!(events[0].slice_id, "slice-test");
    }

    #[test]
    fn module_complete_carries_elapsed_and_wasm_peak_kb() {
        let sink = Arc::new(RecordingSink::default());
        let pi = ProgressPipelineInstrumentation::new(
            sink.clone() as Arc<dyn LayerProgressSink + Send + Sync>,
            "slice-test".to_string(),
        );
        let stage = StageId::from("Layer::Perimeters");
        let module = ModuleId::from("com.example.perimeters");

        pi.on_module_start(&stage, Some(7), &module);
        std::thread::sleep(std::time::Duration::from_millis(2));
        pi.on_module_end(&stage, Some(7), &module, 0, 1025);

        let events = sink.events.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[1].event, ProgressEventType::ModuleComplete);
        assert!(events[1].elapsed_ms.unwrap_or(0) >= 1);
        assert_eq!(events[1].wasm_peak_kb, Some(2));
        assert_eq!(events[1].layer_index, Some(7));
    }

    #[test]
    fn host_builtin_zero_bytes_produces_zero_kb() {
        let sink = Arc::new(RecordingSink::default());
        let pi = ProgressPipelineInstrumentation::new(
            sink.clone() as Arc<dyn LayerProgressSink + Send + Sync>,
            "slice-test".to_string(),
        );
        let stage = StageId::from("PrePass::MeshAnalysis");
        let module = ModuleId::from("host::mesh_analysis");

        pi.on_module_start(&stage, None, &module);
        pi.on_module_end(&stage, None, &module, 0, 0);

        let events = sink.events.lock().unwrap();
        let last = events.last().unwrap();
        assert_eq!(last.event, ProgressEventType::ModuleComplete);
        assert_eq!(last.wasm_peak_kb, Some(0));
    }

    #[test]
    fn core_tier_suppresses_stage_and_module_events() {
        let sink = Arc::new(RecordingSink::default());
        let pi = ProgressPipelineInstrumentation::with_tier(
            sink.clone() as Arc<dyn LayerProgressSink + Send + Sync>,
            "slice-test".to_string(),
            ProgressTier::Core,
        );
        let stage = StageId::from("Layer::Perimeters");
        let module = ModuleId::from("com.example.perimeters");

        pi.on_stage_start(&stage, Some(0));
        pi.on_module_start(&stage, Some(0), &module);
        pi.on_module_end(&stage, Some(0), &module, 0, 1024);
        pi.on_stage_end(&stage, Some(0));
        assert!(
            sink.events.lock().unwrap().is_empty(),
            "Core tier must not emit stage/module lifecycle events"
        );

        pi.on_phase_start(Phase::PerLayer);
        pi.on_layer_start(0, 0.2);
        pi.on_layer_end(0);
        pi.on_phase_end(Phase::PerLayer);
        let events = sink.events.lock().unwrap();
        let kinds: Vec<ProgressEventType> = events.iter().map(|e| e.event).collect();
        assert_eq!(
            kinds,
            vec![
                ProgressEventType::PhaseStart,
                ProgressEventType::LayerStart,
                ProgressEventType::LayerComplete,
                ProgressEventType::PhaseComplete,
            ]
        );
    }

    #[test]
    fn on_module_error_emits_fatal_module_error_event_in_both_tiers() {
        for tier in [ProgressTier::Core, ProgressTier::Instrumented] {
            let sink = Arc::new(RecordingSink::default());
            let pi = ProgressPipelineInstrumentation::with_tier(
                sink.clone() as Arc<dyn LayerProgressSink + Send + Sync>,
                "slice-test".to_string(),
                tier,
            );
            let stage = StageId::from("Layer::Infill");
            let module = ModuleId::from("com.example.infill");
            pi.on_module_error(&stage, Some(3), &module, "dispatch trap", true);

            let events = sink.events.lock().unwrap();
            assert_eq!(events.len(), 1, "tier {tier:?}");
            let e = &events[0];
            assert_eq!(e.event, ProgressEventType::ModuleError);
            assert_eq!(e.phase, Some(ProgressPhase::PerLayer));
            assert_eq!(e.stage.as_deref(), Some("Layer::Infill"));
            assert_eq!(e.layer_index, Some(3));
            assert_eq!(e.module_id.as_deref(), Some("com.example.infill"));
            assert_eq!(e.status, crate::progress_events::ProgressStatus::FatalError);
            let err = e.error.as_ref().expect("module_error carries error");
            assert!(err.fatal);
            assert_eq!(err.code, crate::progress_events::MODULE_DISPATCH_FATAL_CODE);
            assert_eq!(err.message, "dispatch trap");
        }
    }

    #[test]
    fn phase_start_and_phase_end_emit_paired_events() {
        let sink = Arc::new(RecordingSink::default());
        let pi = ProgressPipelineInstrumentation::new(
            sink.clone() as Arc<dyn LayerProgressSink + Send + Sync>,
            "slice-test".to_string(),
        );
        pi.on_phase_start(Phase::PrePass);
        pi.on_phase_end(Phase::PrePass);

        let events = sink.events.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event, ProgressEventType::PhaseStart);
        assert_eq!(events[1].event, ProgressEventType::PhaseComplete);
        assert_eq!(events[1].phase, Some(ProgressPhase::Prepass));
    }
}
