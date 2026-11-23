//! Per-print preparation extensions and metadata-only lifecycle observations.

pub use crate::run::{run_slice_preparation_capture, run_slice_with_runtime_extensions};

#[path = "preparation_visual_capture.rs"]
pub(crate) mod preparation_visual_capture;

pub use preparation_visual_capture::{
    PreparationViewRequest, PreparationVisualCapture, PreparationVisualCaptureError,
};

use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::layer_executor::StageCapture;
use slicer_ir::{GlobalLayer, LayerStageCommit, LayerStageError, ModuleId, StageId};
use slicer_scheduler::IntegratedModuleRegistration;
use slicer_sdk::native::NativePreparedStageEntry;
use slicer_wasm_host::{
    CompiledModuleLive, LayerStageInput, LayerStageRunner, WasmRuntimeDispatcher,
};

/// Additional artifacts and observers supplied to one slice, never ready plans.
#[derive(Default)]
pub struct SliceRuntimeExtensions {
    integrated: Vec<IntegratedModuleRegistration>,
    prepared_native: Vec<(
        ModuleId,
        NativePreparedStageEntry,
        slicer_schema::PreparationExportSchema,
    )>,
    anchored_entities: Vec<slicer_ir::AnchoredEntity>,
    observer: Option<Arc<dyn PreparationLifecycleObserver>>,
    scope_outlet: Option<Arc<dyn Fn(&Arc<PrintPreparationScope>) + Send + Sync>>,
    preparation_views: Vec<PreparationViewRequest>,
}

impl SliceRuntimeExtensions {
    /// Supply candidate artifacts, synthetic host entities and an optional observer.
    pub fn new(
        integrated: Vec<IntegratedModuleRegistration>,
        prepared_native: Vec<(
            ModuleId,
            NativePreparedStageEntry,
            slicer_schema::PreparationExportSchema,
        )>,
        anchored_entities: Vec<slicer_ir::AnchoredEntity>,
        observer: Option<Arc<dyn PreparationLifecycleObserver>>,
    ) -> Self {
        Self {
            integrated,
            prepared_native,
            anchored_entities,
            observer,
            scope_outlet: None,
            preparation_views: Vec::new(),
        }
    }

    /// Add an optional callback invoked once when this run's print scope is constructed.
    pub fn with_scope_outlet(
        mut self,
        outlet: Option<Arc<dyn Fn(&Arc<PrintPreparationScope>) + Send + Sync>>,
    ) -> Self {
        self.scope_outlet = outlet;
        self
    }

    /// Request owner-scoped typed preparation views for this print (opt-in).
    ///
    /// Without this call the extension stays semantically default: no view is
    /// requested, no diagnostic collection is allocated beyond the empty
    /// `Vec`, and preparation runs the plain publication path unchanged.
    pub fn with_preparation_views(mut self, views: Vec<PreparationViewRequest>) -> Self {
        self.preparation_views = views;
        self
    }

    /// Supply the generated prepared pairs for integrated modules that declare
    /// `[preparation]`.
    ///
    /// The pairs are looked up by module id when a selected module's provenance
    /// is `Integrated`. An integrated module that declares preparation but has
    /// no pair here is rejected at freeze time (`MissingExport`), because a
    /// plain artifact cannot satisfy a declared preparation export — so any
    /// entry point that may select such a module has to install the registry's
    /// `prepared_native_entries()`.
    pub fn with_prepared_native(
        mut self,
        prepared_native: Vec<(
            ModuleId,
            NativePreparedStageEntry,
            slicer_schema::PreparationExportSchema,
        )>,
    ) -> Self {
        self.prepared_native = prepared_native;
        self
    }

    pub(crate) fn into_run_parts(
        self,
        instrumented: bool,
        capture_requested: bool,
    ) -> (
        Vec<IntegratedModuleRegistration>,
        Vec<slicer_ir::AnchoredEntity>,
        Arc<PrintPreparationScope>,
    ) {
        let Self {
            integrated,
            prepared_native,
            anchored_entities,
            observer,
            scope_outlet,
            preparation_views,
        } = self;
        let scope = Arc::new(PrintPreparationScope {
            prepared_native,
            _observer: observer,
            instrumented,
            capture_requested,
            selected: Mutex::new(Vec::new()),
            committed: Mutex::new(None),
            owners: Mutex::new(Vec::new()),
            records: Mutex::new(Vec::new()),
            cancel_flag: Mutex::new(None),
            store: slicer_wasm_host::preparation::PreparedOwnerStore::new(),
            preparation_views,
            first_projection_failure: Mutex::new(None),
        });
        if let Some(outlet) = scope_outlet {
            outlet(&scope);
        }
        (integrated, anchored_entities, scope)
    }
}

/// Metadata snapshot; no opaque piece bytes or plan handles escape a print.
#[derive(Clone, Debug, Default)]
pub struct PreparationCapture {
    /// Eligible owners and their metadata before cleanup.
    pub owners: Vec<PreparationOwnerCapture>,
    /// Ordered lifecycle observations.
    pub lifecycle: Vec<PreparationLifecycleRecord>,
    /// Logical bytes retained after cleanup.
    pub retained_bytes: u64,
    /// Bytes returned by successful production piece reads.
    pub transferred_bytes: u64,
}

/// Metadata for one selected prepared owner; contains no payload or live handle.
#[derive(Clone, Debug)]
pub struct PreparationOwnerCapture {
    /// Fully qualified selected module identifier.
    pub module_id: ModuleId,
    /// Selected manifest version.
    pub module_version: String,
    /// Opaque identity of the selected artifact.
    pub artifact_identity: String,
    /// Ordinary consuming stage, including module-backed anchored events.
    pub consumer_stage: StageId,
    /// Lossless typed target identities across the print.
    pub selected_targets: Vec<slicer_sdk::preparation::PreparationSelectedTarget>,
    /// Published piece names and lengths only.
    pub pieces: Vec<PreparationPieceMetadata>,
    /// Store accounting sampled before owner disposal.
    pub accounting_before_disposal: PreparationAccountingSnapshot,
}

/// Published piece descriptor without its bytes.
#[derive(Clone, Debug)]
pub struct PreparationPieceMetadata {
    /// Exact module-owned piece name.
    pub name: String,
    /// Published payload length in bytes.
    pub length: u64,
}

/// Logical checked-store counters, not allocator measurements.
#[derive(Clone, Debug)]
pub struct PreparationAccountingSnapshot {
    /// Retained immutable payload bytes.
    pub retained_payload_bytes: u64,
    /// Retained piece-name bytes.
    pub retained_name_bytes: u64,
    /// Successful production piece-read bytes.
    pub returned_read_bytes: u64,
}

/// Metadata-only observer of real runtime ownership boundaries.
pub trait PreparationLifecycleObserver: Send + Sync {
    /// Observe a preparation, consumption or disposal transition.
    fn on_preparation_lifecycle(&self, record: &PreparationLifecycleRecord);
    /// Observe an arena boundary without retaining arena contents.
    fn on_layer_arena(&self, _global_layer_index: u32, _status: LayerArenaLifecycleStatus) {}
    /// Observe synthetic host work, which is not a prepared module read.
    fn on_anchored_host_closure(&self, _status: AnchoredHostClosureStatus) {}
}

/// Lifetime boundary of an actual per-layer arena.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerArenaLifecycleStatus {
    /// Arena allocated for a layer.
    Created,
    /// Layer output committed.
    Committed,
    /// Arena released, including on failure.
    Dropped,
}

/// Entry and return/unwind boundaries of synthetic anchored host work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnchoredHostClosureStatus {
    /// Host closure entered.
    Start,
    /// Host closure returned or unwound; does not imply success.
    Finished,
}

/// Preparation lifecycle axis, independent of ordinary scheduler stages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreparationLifecyclePhase {
    /// Whole-print producer work.
    Preparation,
    /// Actual ordinary module consumption.
    Consumption,
    /// Quiescent print cleanup.
    Disposal,
}

/// Observed lifecycle decision or transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreparationLifecycleStatus {
    /// Operation entered.
    Start,
    /// Atomic publication completed.
    Ready,
    /// Selected declaration had no eligible targets.
    NoWork,
    /// Operation failed with its original typed error.
    Failure,
    /// Cooperative cancellation was observed.
    Cancelled,
    /// Owner storage was released.
    Disposed,
}

/// Metadata-only lifecycle observation with selected artifact identity.
#[derive(Clone, Debug)]
pub struct PreparationLifecycleRecord {
    /// Fully qualified selected module identifier.
    pub module_id: ModuleId,
    /// Selected manifest version.
    pub module_version: String,
    /// Opaque selected artifact identity.
    pub artifact_identity: String,
    /// Ordinary consumer stage.
    pub consumer_stage: StageId,
    /// Actual consuming layer, absent for whole-print operations.
    pub global_layer_index: Option<u32>,
    /// Lifecycle axis, separate from the ordinary stage.
    pub lifecycle_phase: PreparationLifecyclePhase,
    /// Observed transition or decision.
    pub status: PreparationLifecycleStatus,
    /// Piece names and lengths, never payloads.
    pub pieces: Vec<(String, u64)>,
    /// Logical retained payload and name bytes.
    pub retained_bytes: u64,
    /// Successful production piece-read bytes.
    pub transferred_bytes: u64,
    /// Measured duration only when instrumentation is enabled.
    pub elapsed: Option<Duration>,
    /// Original typed failure, when present.
    pub failure: Option<Arc<slicer_wasm_host::preparation::PreparationFailure>>,
}

/// Opaque print scope retaining selected artifacts and committed plan authority.
pub struct PrintPreparationScope {
    pub(crate) prepared_native: Vec<(
        ModuleId,
        NativePreparedStageEntry,
        slicer_schema::PreparationExportSchema,
    )>,
    _observer: Option<Arc<dyn PreparationLifecycleObserver>>,
    instrumented: bool,
    capture_requested: bool,
    selected: Mutex<Vec<Arc<SelectedPreparationBinding>>>,
    committed: Mutex<Option<CommittedPreparationSelection>>,
    // Filled by the preparation closure, never by a consumer call.
    owners: Mutex<Vec<(usize, slicer_wasm_host::preparation::PreparationOwner)>>,
    records: Mutex<Vec<PreparationLifecycleRecord>>,
    cancel_flag: Mutex<Option<Arc<std::sync::atomic::AtomicBool>>>,
    store: slicer_wasm_host::preparation::PreparedOwnerStore,
    preparation_views: Vec<PreparationViewRequest>,
    /// First typed projection validation failure observed during publication.
    /// Populated only when the producer's failure is the typed
    /// `ProjectionRejected` variant, so an original module error/trap keeps
    /// precedence. Never derived from formatted diagnostic text.
    first_projection_failure: Mutex<
        Option<(
            ModuleId,
            String,
            slicer_schema::preparation::PreparationProjectionError,
        )>,
    >,
}

// Kept private to the print, never added to loader or pipeline envelopes.
pub(crate) struct SelectedPreparationBinding {
    pub(crate) module_id: ModuleId,
    pub(crate) module_version: String,
    pub(crate) artifact_identity: String,
    pub(crate) consumer_stage: StageId,
    pub(crate) binding: slicer_wasm_host::preparation::FrozenPreparationBinding,
}

struct CommittedPreparationSelection {
    inputs: Arc<slicer_wasm_host::preparation::CommittedPreparationInputs>,
    layer_indices: Vec<u32>,
    readers: Vec<(usize, slicer_wasm_host::preparation::PreparationInputData)>,
    // Indices into the identity-sorted frozen bindings: one entry per print owner.
    eligible: Vec<(
        usize,
        Option<slicer_wasm_host::preparation::PreparationSelectionContext>,
    )>,
}

impl PrintPreparationScope {
    pub(crate) fn observer(&self) -> Option<&dyn PreparationLifecycleObserver> {
        self._observer.as_deref()
    }

    /// Owner-scoped typed views requested for this print; empty by default.
    pub(crate) fn preparation_views(&self) -> &[PreparationViewRequest] {
        &self.preparation_views
    }

    /// Take the first typed projection validation failure, when the producer's
    /// publication was rejected by the host projection validator. A module
    /// error/trap/cancellation never populates this sidecar.
    pub(crate) fn take_first_projection_failure(
        &self,
    ) -> Option<(
        ModuleId,
        String,
        slicer_schema::preparation::PreparationProjectionError,
    )> {
        self.first_projection_failure
            .lock()
            .expect("print projection failure poisoned")
            .take()
    }

    pub(crate) fn set_selected(&self, mut selected: Vec<SelectedPreparationBinding>) {
        selected.sort_by(|a, b| {
            (
                &a.module_id,
                &a.module_version,
                &a.consumer_stage,
                &a.artifact_identity,
            )
                .cmp(&(
                    &b.module_id,
                    &b.module_version,
                    &b.consumer_stage,
                    &b.artifact_identity,
                ))
        });
        *self.selected.lock().expect("print selection poisoned") =
            selected.into_iter().map(Arc::new).collect();
    }

    pub(crate) fn capture_committed_selection(
        &self,
        plan: &crate::ExecutionPlan,
        blackboard: &crate::Blackboard,
    ) -> Result<(), String> {
        use slicer_wasm_host::preparation::{
            CommittedPreparationInputs, PreparationSelectionContext,
        };
        let selected = self.selected.lock().expect("print selection poisoned");
        let Some(index) = blackboard.layer_selection_index().cloned() else {
            return Err("committed layer selection index missing".into());
        };
        let mut eligible = Vec::new();
        for (position, frozen) in selected.iter().enumerate() {
            if frozen.binding.declaration().is_none() {
                continue;
            }
            let module = plan
                .per_layer_stages
                .iter()
                .find(|stage| stage.stage_id == frozen.consumer_stage)
                .and_then(|stage| {
                    stage
                        .modules
                        .iter()
                        .find(|module| module.module_id() == &frozen.module_id)
                })
                .ok_or_else(|| format!("selected module {} missing from plan", frozen.module_id))?;
            // A fill-role module emits only where its claim survives the
            // per-region holder resolution. Preparing one that holds nothing
            // would be whole-print work with no consumer, contradicting the
            // zero-cost intent (ADR-0074). Non-fill preparation modules keep the
            // selection-only rule.
            let fill_role_module = module.claims().iter().any(|claim| {
                slicer_scheduler::validation::FILL_CLAIM_IDS.contains(&claim.as_str())
            }) && module.claims().iter().all(|claim| {
                claim == "claim:raft-fill"
                    || slicer_scheduler::validation::FILL_CLAIM_IDS.contains(&claim.as_str())
            });
            let participates = plan.global_layers.iter().any(|layer| {
                index
                    .selection_for(layer.index, &frozen.module_id)
                    .is_some_and(|selection| {
                        selection.invoke
                            && !selection.targets.is_empty()
                            && (!fill_role_module
                                || selection.targets.iter().any(|target| {
                                    target.held_claims.iter().any(|claim| {
                                        slicer_scheduler::validation::FILL_CLAIM_IDS
                                            .contains(&claim.as_str())
                                    })
                                }))
                    })
            });
            if !participates {
                continue;
            }
            eligible.push((
                position,
                Some(PreparationSelectionContext::new(
                    frozen.consumer_stage.clone(),
                    frozen.module_id.clone(),
                    module.claims().to_vec(),
                    module.region_split_semantics().clone(),
                    Arc::clone(module.config_view()),
                )),
            ));
        }
        // No input grants are needed for plain or no-work modules.
        if eligible.is_empty() {
            return Ok(());
        }
        let inputs = CommittedPreparationInputs::new(
            Arc::clone(blackboard.mesh()),
            blackboard
                .layer_plan()
                .cloned()
                .ok_or("committed layer plan missing")?,
            blackboard
                .slice_ir()
                .cloned()
                .ok_or("committed slices missing")?,
            blackboard
                .region_map()
                .cloned()
                .ok_or("committed region map missing")?,
            blackboard.surface_classification().cloned(),
            blackboard.seam_plan().cloned(),
            blackboard.support_analysis().cloned(),
            blackboard.support_geometry().cloned(),
            blackboard.support_plan().cloned(),
        )
        .map_err(|error| error.to_string())?;
        *self.committed.lock().expect("print inputs poisoned") =
            Some(CommittedPreparationSelection {
                inputs: Arc::new(inputs),
                layer_indices: plan.global_layers.iter().map(|layer| layer.index).collect(),
                readers: Vec::new(),
                eligible,
            });
        Ok(())
    }

    fn observe_owner(
        &self,
        frozen: &SelectedPreparationBinding,
        owner: Option<&slicer_wasm_host::preparation::PreparationOwner>,
        status: PreparationLifecycleStatus,
        elapsed: Option<Duration>,
        failure: Option<Arc<slicer_wasm_host::preparation::PreparationFailure>>,
    ) -> Result<(), slicer_wasm_host::preparation::PreparationFailure> {
        let metadata = owner.map(|owner| owner.metadata());
        let retained_bytes = match &metadata {
            Some(metadata) => metadata
                .retained_payload_bytes()
                .checked_add(metadata.retained_name_bytes())
                .ok_or_else(|| {
                    slicer_wasm_host::preparation::PreparationFailure::PublicationRejected {
                        detail: "preparation retention accounting overflow".into(),
                    }
                })?,
            None => 0,
        };
        let record = PreparationLifecycleRecord {
            module_id: frozen.module_id.clone(),
            module_version: frozen.module_version.clone(),
            artifact_identity: frozen.artifact_identity.clone(),
            consumer_stage: frozen.consumer_stage.clone(),
            global_layer_index: None,
            lifecycle_phase: if status == PreparationLifecycleStatus::Disposed {
                PreparationLifecyclePhase::Disposal
            } else {
                PreparationLifecyclePhase::Preparation
            },
            status,
            pieces: metadata
                .as_ref()
                .map_or_else(Vec::new, |metadata| metadata.pieces().to_vec()),
            retained_bytes,
            transferred_bytes: metadata
                .as_ref()
                .map_or(0, |metadata| metadata.returned_read_bytes()),
            elapsed,
            failure,
        };
        self.records
            .lock()
            .expect("print records poisoned")
            .push(record.clone());
        if let Some(observer) = &self._observer {
            observer.on_preparation_lifecycle(&record);
        }
        Ok(())
    }

    /// The same required-dependency guard protects publication completion and
    /// admission to the layer loop. Never infer Ready from adapter success.
    fn ensure_all_ready(&self, required: &[usize]) -> Result<(), crate::pipeline::PipelineError> {
        use slicer_wasm_host::preparation::PreparationFailure;
        let selected = self.selected.lock().expect("print selection poisoned");
        let owners = self.owners.lock().expect("print owners poisoned");
        for position in required {
            let result = owners
                .iter()
                .find(|(index, _)| index == position)
                .ok_or(PreparationFailure::InvalidOwner)
                .and_then(|(_, owner)| self.store.current(owner).map(|_| ()));
            result.map_err(|failure| crate::pipeline::PipelineError::Preparation {
                module_id: selected[*position].module_id.clone(),
                failure: Arc::new(failure),
            })?;
        }
        Ok(())
    }

    fn is_cancelled(&self) -> bool {
        self.cancel_flag
            .lock()
            .expect("print cancellation poisoned")
            .as_ref()
            .is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Acquire))
    }

    fn cancelled_error() -> crate::pipeline::PipelineError {
        crate::pipeline::PipelineError::LayerExecution(
            crate::layer_executor::LayerExecutionError::Cancelled,
        )
    }

    pub(crate) fn prepare_committed(
        &self,
        cancel_flag: Option<Arc<std::sync::atomic::AtomicBool>>,
        instrumentation: &(dyn crate::instrumentation::PipelineInstrumentation + Sync),
    ) -> Result<(), crate::pipeline::PipelineError> {
        use slicer_wasm_host::preparation::{
            PreparationFailure, PreparationInputData, PreparationProjectionRequest,
        };
        *self
            .cancel_flag
            .lock()
            .expect("print cancellation poisoned") = cancel_flag;
        let selected = self
            .selected
            .lock()
            .expect("print selection poisoned")
            .clone();
        let mut committed = self.committed.lock().expect("print inputs poisoned");
        let required: Vec<_> = committed.as_ref().map_or_else(Vec::new, |committed| {
            committed
                .eligible
                .iter()
                .map(|(position, _)| *position)
                .collect()
        });
        for (position, frozen) in selected.iter().enumerate() {
            if self.is_cancelled() {
                self.observe_owner(
                    frozen,
                    None,
                    PreparationLifecycleStatus::Cancelled,
                    None,
                    None,
                )
                .map_err(|failure| {
                    crate::pipeline::PipelineError::Preparation {
                        module_id: frozen.module_id.clone(),
                        failure: Arc::new(failure),
                    }
                })?;
                return Err(Self::cancelled_error());
            }
            if frozen.binding.declaration().is_none() {
                continue;
            }
            let error = |failure| crate::pipeline::PipelineError::Preparation {
                module_id: frozen.module_id.clone(),
                failure: Arc::new(failure),
            };
            if !required.contains(&position) {
                self.observe_owner(frozen, None, PreparationLifecycleStatus::NoWork, None, None)
                    .map_err(error)?;
                continue;
            }
            let owner = self.store.create_owner(&frozen.binding).map_err(error)?;
            self.owners
                .lock()
                .expect("print owners poisoned")
                .push((position, owner.clone()));
            self.observe_owner(
                frozen,
                Some(&owner),
                PreparationLifecycleStatus::Start,
                None,
                None,
            )
            .map_err(error)?;
            let started = self.instrumented.then(std::time::Instant::now);
            let committed = committed.as_mut().expect("eligible committed inputs");
            let inputs = Arc::clone(&committed.inputs);
            let selection = committed
                .eligible
                .iter_mut()
                .find(|(index, _)| *index == position)
                .and_then(|(_, selection)| selection.take())
                .ok_or_else(|| error(PreparationFailure::AlreadyStarted))?;
            // This decision belongs to the host validation closure, not to the
            // producer's diagnostic text. Original call errors bypass validation.
            let mut cancelled_at_publication = false;
            let result = PreparationInputData::from_committed(&owner, inputs, selection)
                .and_then(|input| {
                    if self.capture_requested {
                        committed.readers.push((position, input.clone()));
                    }
                    let views: Vec<String> = self
                        .preparation_views()
                        .iter()
                        .filter(|request| request.module_id == frozen.module_id)
                        .map(|request| request.view.clone())
                        .collect();
                    if views.is_empty() {
                        frozen.binding.prepare_with_publication(&owner, input, |_| {
                            if self.is_cancelled() {
                                cancelled_at_publication = true;
                                Err(PreparationFailure::PublicationRejected {
                                    detail: "cooperative cancellation before publication".into(),
                                })
                            } else {
                                Ok(())
                            }
                        })
                    } else {
                        let request =
                            PreparationProjectionRequest::new(views).map_err(|error| {
                                PreparationFailure::PublicationRejected {
                                    detail: format!(
                                        "preparation projection: {error:?}: owner={}",
                                        frozen.module_id
                                    ),
                                }
                            })?;
                        frozen
                            .binding
                            .prepare_with_diagnostics(&owner, input, request, |_| {
                                if self.is_cancelled() {
                                    cancelled_at_publication = true;
                                    Err(PreparationFailure::PublicationRejected {
                                        detail: "cooperative cancellation before publication"
                                            .into(),
                                    })
                                } else {
                                    Ok(())
                                }
                            })
                    }
                })
                .and_then(|()| self.store.current(&owner).map(|_| ()));
            let prepare_fuel = owner.prepare_fuel_consumed();
            if prepare_fuel > 0 {
                instrumentation.on_preparation_profile(&frozen.module_id, prepare_fuel);
            }
            let elapsed = started.map(|started| started.elapsed());
            if cancelled_at_publication {
                self.observe_owner(
                    frozen,
                    Some(&owner),
                    PreparationLifecycleStatus::Cancelled,
                    elapsed,
                    None,
                )
                .map_err(error)?;
                return Err(Self::cancelled_error());
            }
            if let Err(failure) = result {
                // Preserve the typed projection reason only when the host's
                // own publication validator rejected the producer's scene. A
                // module error/trap/cancellation is a different variant and
                // A typed projection rejection carries its own view and reason;
                // an original module error/trap keeps its precedence.
                if let PreparationFailure::ProjectionRejected { view, error, .. } = &failure {
                    *self
                        .first_projection_failure
                        .lock()
                        .expect("print projection failure poisoned") =
                        Some((frozen.module_id.clone(), view.clone(), *error));
                }
                let failure = Arc::new(failure);
                self.observe_owner(
                    frozen,
                    Some(&owner),
                    PreparationLifecycleStatus::Failure,
                    elapsed,
                    Some(Arc::clone(&failure)),
                )
                .map_err(error)?;
                return Err(crate::pipeline::PipelineError::Preparation {
                    module_id: frozen.module_id.clone(),
                    failure,
                });
            }
            self.observe_owner(
                frozen,
                Some(&owner),
                PreparationLifecycleStatus::Ready,
                elapsed,
                None,
            )
            .map_err(error)?;
        }
        drop(committed);
        if self.is_cancelled() {
            return Err(Self::cancelled_error());
        }
        self.ensure_all_ready(&required)
    }

    fn ensure_consumers_ready(&self) -> Result<(), crate::pipeline::PipelineError> {
        let required = self
            .committed
            .lock()
            .expect("print inputs poisoned")
            .as_ref()
            .map_or_else(Vec::new, |committed| {
                committed
                    .eligible
                    .iter()
                    .map(|(position, _)| *position)
                    .collect::<Vec<_>>()
            });
        self.ensure_all_ready(&required)
    }

    /// Copy only typed input DTOs and checked-store metadata, then release all
    /// owners before returning. No plan open/read operation is used here.
    pub(crate) fn snapshot_and_dispose(
        &self,
    ) -> Result<PreparationCapture, slicer_wasm_host::preparation::PreparationFailure> {
        use slicer_wasm_host::preparation::PreparationFailure;
        let overflow = || PreparationFailure::PublicationRejected {
            detail: "preparation capture accounting overflow".into(),
        };
        let selected = self
            .selected
            .lock()
            .expect("print selection poisoned")
            .clone();
        let owners = self.owners.lock().expect("print owners poisoned").clone();
        let mut capture = PreparationCapture::default();
        {
            let committed = self.committed.lock().expect("print inputs poisoned");
            for (position, owner) in &owners {
                let frozen = &selected[*position];
                let committed = committed.as_ref().ok_or(PreparationFailure::InvalidOwner)?;
                let reader = &committed
                    .readers
                    .iter()
                    .find(|(index, _)| index == position)
                    .ok_or(PreparationFailure::InvalidOwner)?
                    .1;
                let mut selected_targets = Vec::new();
                for layer in &committed.layer_indices {
                    let mut offset = 0u64;
                    loop {
                        let page = reader
                            .host_selected_targets(owner, *layer, offset, 128)
                            .map_err(PreparationFailure::Input)?;
                        if page.is_empty() {
                            break;
                        }
                        offset = offset
                            .checked_add(u64::try_from(page.len()).map_err(|_| overflow())?)
                            .ok_or_else(overflow)?;
                        selected_targets.extend(page);
                    }
                }
                let metadata = owner.metadata();
                capture.transferred_bytes = capture
                    .transferred_bytes
                    .checked_add(metadata.returned_read_bytes())
                    .ok_or_else(overflow)?;
                capture.owners.push(PreparationOwnerCapture {
                    module_id: frozen.module_id.clone(),
                    module_version: frozen.module_version.clone(),
                    artifact_identity: frozen.artifact_identity.clone(),
                    consumer_stage: frozen.consumer_stage.clone(),
                    selected_targets,
                    pieces: metadata
                        .pieces()
                        .iter()
                        .map(|(name, length)| PreparationPieceMetadata {
                            name: name.clone(),
                            length: *length,
                        })
                        .collect(),
                    accounting_before_disposal: PreparationAccountingSnapshot {
                        retained_payload_bytes: metadata.retained_payload_bytes(),
                        retained_name_bytes: metadata.retained_name_bytes(),
                        returned_read_bytes: metadata.returned_read_bytes(),
                    },
                });
            }
        }
        self.dispose()?;
        // `PreparationCapture::retained_bytes` is "logical bytes retained after
        // cleanup". Disposal zeroes every owner's accounting, so the value is
        // zero by construction; the pre-disposal snapshot lives on each
        // owner's `accounting_before_disposal`. Summing the zeroed metadata here
        // would be an unreachable overflow check, not a measurement.
        capture.lifecycle = self.records.lock().expect("print records poisoned").clone();
        Ok(capture)
    }

    /// Capture requested typed preparation views and the #05 metadata, then
    /// dispose every owner before returning.
    ///
    /// The committed guard is scoped so no plan authority outlives the reads;
    /// projection scenes are shared immutable snapshots, so the returned
    /// companion stays valid after disposal.
    pub(crate) fn capture_visual_and_dispose(
        &self,
        rendered_layers: &[u32],
        executed_stage_ids: Vec<StageId>,
        executed_layer_indices: Vec<u32>,
        tap_captures: Vec<StageCapture>,
    ) -> Result<PreparationVisualCapture, PreparationVisualCaptureError> {
        use crate::layer_executor::CapturedIr;
        use crate::run::SliceRunError;
        use slicer_sdk::preparation::PreparationProjectionError;

        let selected = self
            .selected
            .lock()
            .expect("print selection poisoned")
            .clone();
        let owners = self.owners.lock().expect("print owners poisoned").clone();
        let mut schedule = Vec::new();
        let mut projection_captures = Vec::new();
        let mut diagnostic_accounting = Vec::new();
        {
            let committed = self.committed.lock().expect("print inputs poisoned");
            let committed = committed.as_ref().ok_or_else(|| {
                PreparationVisualCaptureError::InvalidRequest {
                    detail: "preparation visual capture: committed selection missing".into(),
                }
            })?;
            for (position, owner) in &owners {
                let frozen = &selected[*position];
                let reader = committed
                    .readers
                    .iter()
                    .find(|(index, _)| index == position)
                    .map(|(_, reader)| reader)
                    .ok_or_else(|| {
                        PreparationVisualCaptureError::Slice(SliceRunError(format!(
                            "preparation visual capture: committed reader missing for owner '{}'",
                            frozen.module_id
                        )))
                    })?;
                if schedule.is_empty() {
                    schedule = reader.schedule_snapshot().map_err(|error| {
                        PreparationVisualCaptureError::Slice(SliceRunError(error.to_string()))
                    })?;
                }
                let handle = self.store.current(owner).map_err(|failure| {
                    PreparationVisualCaptureError::Slice(SliceRunError(failure.to_string()))
                })?;
                if let Some((view, reason)) =
                    self.store.projection_failure(owner).map_err(|failure| {
                        PreparationVisualCaptureError::Slice(SliceRunError(failure.to_string()))
                    })?
                {
                    return Err(PreparationVisualCaptureError::ProjectionRejected {
                        module_id: frozen.module_id.clone(),
                        view,
                        reason,
                    });
                }
                for request in self
                    .preparation_views()
                    .iter()
                    .filter(|request| request.module_id == frozen.module_id)
                {
                    let scene = handle
                        .projection(&request.view)
                        .map_err(|reason| PreparationVisualCaptureError::ProjectionRejected {
                            module_id: frozen.module_id.clone(),
                            view: request.view.clone(),
                            reason,
                        })?
                        .ok_or_else(|| PreparationVisualCaptureError::ProjectionRejected {
                            module_id: frozen.module_id.clone(),
                            view: request.view.clone(),
                            reason: PreparationProjectionError::MissingScene,
                        })?;
                    for &layer in rendered_layers {
                        let layer_z = scene.layer_z(layer).ok_or_else(|| {
                            PreparationVisualCaptureError::ProjectionRejected {
                                module_id: frozen.module_id.clone(),
                                view: request.view.clone(),
                                reason: PreparationProjectionError::InvalidReference,
                            }
                        })?;
                        projection_captures.push(StageCapture {
                            stage_id: frozen.consumer_stage.clone(),
                            layer_index: layer,
                            layer_z,
                            ir: CapturedIr::PreparationProjection(Arc::clone(&scene)),
                        });
                    }
                    // One accounting entry per owner/view, never per layer.
                    diagnostic_accounting
                        .push((frozen.module_id.clone(), scene.diagnostic_bytes()));
                }
            }
        }
        let metadata = self.snapshot_and_dispose().map_err(|failure| {
            PreparationVisualCaptureError::Slice(SliceRunError(failure.to_string()))
        })?;
        diagnostic_accounting.sort_by(|left, right| left.0.cmp(&right.0));
        projection_captures.sort_by(|left, right| {
            let left_key = match &left.ir {
                CapturedIr::PreparationProjection(scene) => {
                    (scene.owner_module_id(), scene.view(), left.layer_index)
                }
                _ => ("", "", left.layer_index),
            };
            let right_key = match &right.ir {
                CapturedIr::PreparationProjection(scene) => {
                    (scene.owner_module_id(), scene.view(), right.layer_index)
                }
                _ => ("", "", right.layer_index),
            };
            left_key.cmp(&right_key)
        });
        let mut captures = tap_captures;
        captures.extend(projection_captures);
        Ok(PreparationVisualCapture::from_parts(
            metadata,
            captures,
            diagnostic_accounting,
            executed_stage_ids,
            executed_layer_indices,
            rendered_layers.to_vec(),
            schedule,
        ))
    }

    /// Release print-owned state after the pipeline has joined all work, even
    /// when the SDK outlet keeps this scope alive beyond the run.
    pub(crate) fn dispose_after_run(
        &self,
    ) -> Result<(), slicer_wasm_host::preparation::PreparationFailure> {
        self.dispose()
    }

    /// Dispose retained plans, preserving the checked store's typed busy error.
    ///
    /// Bind this method to `slicer_sdk::preparation::dispose_prepared_plan` for
    /// SDK disposal. Active calls are not waited on, retried, or revoked; the
    /// caller must join them before disposal can succeed.
    pub fn dispose(&self) -> Result<(), slicer_wasm_host::preparation::PreparationFailure> {
        self.store.dispose()?;
        self.committed.lock().expect("print inputs poisoned").take();
        let selected = self
            .selected
            .lock()
            .expect("print selection poisoned")
            .clone();
        // Taking under the mutex is the record-once guard: later SDK/runtime/Drop
        // disposal calls see no owners and cannot emit duplicate Disposed records.
        let owners = std::mem::take(&mut *self.owners.lock().expect("print owners poisoned"));
        for (position, owner) in owners {
            self.observe_owner(
                &selected[position],
                Some(&owner),
                PreparationLifecycleStatus::Disposed,
                None,
                None,
            )?;
        }
        Ok(())
    }
}

impl Drop for PrintPreparationScope {
    fn drop(&mut self) {
        // Last Arc: all synchronous prepare/read calls have returned/unwound.
        // The layer executor's Rayon collect joins scheduled work before return;
        // synthetic anchored host work also returns before the pipeline does.
        // Busy is an invariant violation here, never a request to poll/revoke.
        self.dispose()
            .expect("print preparation cleanup requires quiescence");
    }
}

/// Production forwarding slot; plain dispatch preserves all existing sampling.
pub(crate) struct PreparedLayerStageRunner {
    plain: Box<dyn LayerStageRunner + Send + Sync>,
    prepared_dispatcher: WasmRuntimeDispatcher,
    lifecycle: Arc<PrintPreparationScope>,
    last_prepared_call: Mutex<std::collections::HashMap<std::thread::ThreadId, bool>>,
}

struct ConsumptionObservation<'a> {
    observer: Option<&'a dyn PreparationLifecycleObserver>,
    record: PreparationLifecycleRecord,
    failed: bool,
    started: Option<std::time::Instant>,
}

impl Drop for ConsumptionObservation<'_> {
    fn drop(&mut self) {
        if self.failed {
            self.record.elapsed = self.started.map(|started| started.elapsed());
            if self.record.status != PreparationLifecycleStatus::Cancelled {
                self.record.status = PreparationLifecycleStatus::Failure;
            }
            if let Some(observer) = self.observer {
                observer.on_preparation_lifecycle(&self.record);
            }
        }
    }
}

impl PreparedLayerStageRunner {
    pub(crate) fn new(
        plain: Box<dyn LayerStageRunner + Send + Sync>,
        prepared_dispatcher: WasmRuntimeDispatcher,
        lifecycle: Arc<PrintPreparationScope>,
    ) -> Self {
        Self {
            plain,
            prepared_dispatcher,
            lifecycle,
            last_prepared_call: Mutex::new(std::collections::HashMap::new()),
        }
    }

    fn last_runner(&self) -> &dyn LayerStageRunner {
        // Executor sampling immediately follows run_stage on the same worker.
        // Keep routing print-private rather than sharing a TLS lifecycle flag.
        if self
            .last_prepared_call
            .lock()
            .expect("runner sampling poisoned")
            .get(&std::thread::current().id())
            .copied()
            .unwrap_or(false)
        {
            &self.prepared_dispatcher
        } else {
            self.plain.as_ref()
        }
    }

    fn set_last_prepared_call(&self, prepared: bool) {
        self.last_prepared_call
            .lock()
            .expect("runner sampling poisoned")
            .insert(std::thread::current().id(), prepared);
    }
}

impl LayerStageRunner for PreparedLayerStageRunner {
    fn run_stage(
        &self,
        stage_id: &StageId,
        layer: &GlobalLayer,
        module: &CompiledModuleLive<'_>,
        input: LayerStageInput<'_>,
    ) -> Result<Option<LayerStageCommit>, LayerStageError> {
        self.set_last_prepared_call(false);
        if self.lifecycle.is_cancelled() {
            // The pipeline's phase/layer checkpoint returns typed Cancelled.
            // Do not turn cooperative cancellation into a fatal module error.
            return Ok(None);
        }
        let selected = self
            .lifecycle
            .selected
            .lock()
            .expect("print selection poisoned");
        let frozen = selected
            .iter()
            .enumerate()
            .find(|(_, frozen)| {
                &frozen.module_id == module.module_id && frozen.consumer_stage == *stage_id
            })
            .map(|(position, frozen)| (position, Arc::clone(frozen)));
        drop(selected);
        let Some((position, frozen)) =
            frozen.filter(|(_, frozen)| frozen.binding.declaration().is_some())
        else {
            return self.plain.run_stage(stage_id, layer, module, input);
        };
        // Plain invoke=true/empty delivery is intentionally not changed above.
        if !crate::layer_executor::prepared_call_has_available_work(layer.index, &input) {
            return Ok(None);
        }
        let owner = self
            .lifecycle
            .owners
            .lock()
            .expect("print owners poisoned")
            .iter()
            .find(|(index, _)| *index == position)
            .map(|(_, owner)| owner.clone());
        let Some(owner) = owner else {
            // The module declares preparation but has no committed eligible
            // owner: the eligibility gate excludes fill-role modules whose
            // claims resolve empty on this print. Its plain export is the
            // correct path and emits nothing.
            return self.plain.run_stage(stage_id, layer, module, input);
        };
        self.lifecycle
            .ensure_consumers_ready()
            .map_err(|error| LayerStageError::FatalModule {
                stage_id: stage_id.clone(),
                module_id: module.module_id.clone(),
                message: error.to_string(),
            })?;
        self.set_last_prepared_call(true);
        let observer = self.lifecycle._observer.as_deref();
        let mut observation = ConsumptionObservation {
            observer,
            record: PreparationLifecycleRecord {
                module_id: frozen.module_id.clone(),
                module_version: frozen.module_version.clone(),
                artifact_identity: frozen.artifact_identity.clone(),
                consumer_stage: frozen.consumer_stage.clone(),
                global_layer_index: Some(layer.index),
                lifecycle_phase: PreparationLifecyclePhase::Consumption,
                status: PreparationLifecycleStatus::Start,
                pieces: Vec::new(),
                retained_bytes: 0,
                transferred_bytes: 0,
                elapsed: None,
                failure: None,
            },
            failed: true,
            started: self.lifecycle.instrumented.then(std::time::Instant::now),
        };
        let fail = |message: String| LayerStageError::FatalModule {
            stage_id: stage_id.clone(),
            module_id: module.module_id.clone(),
            message,
        };
        let metadata = owner.metadata();
        observation.record.pieces = metadata.pieces().to_vec();
        observation.record.retained_bytes = metadata
            .retained_payload_bytes()
            .checked_add(metadata.retained_name_bytes())
            .ok_or_else(|| fail("prepared retention accounting overflow".into()))?;
        observation.record.transferred_bytes = metadata.returned_read_bytes();
        if let Some(observer) = observer {
            observer.on_preparation_lifecycle(&observation.record);
        }
        if self.lifecycle.is_cancelled() {
            observation.record.status = PreparationLifecycleStatus::Cancelled;
            return Ok(None);
        }
        // A readiness probe retains no plan handle across calls. The frozen
        // adapter owns the checked active read guard for the actual invocation.
        self.lifecycle.store.current(&owner).map_err(|error| {
            let message = error.to_string();
            observation.record.failure = Some(Arc::new(error));
            fail(message)
        })?;
        let result = self.prepared_dispatcher.run_prepared_stage_with_failure(
            stage_id,
            layer,
            module,
            input,
            &frozen.binding,
            &owner,
            &mut observation.record.failure,
        );
        observation.failed = result.is_err();
        if result.is_ok() && self.lifecycle.is_cancelled() {
            observation.record.status = PreparationLifecycleStatus::Cancelled;
            observation.failed = true;
        }
        observation.record.transferred_bytes = owner.metadata().returned_read_bytes();
        result
    }

    fn last_wasm_mem_sample(&self) -> (u64, u64) {
        self.last_runner().last_wasm_mem_sample()
    }
    fn last_runtime_reads(&self) -> Vec<String> {
        self.last_runner().last_runtime_reads()
    }
    fn last_batch_calls(&self) -> Vec<(String, u32)> {
        self.last_runner().last_batch_calls()
    }
    fn last_log_messages(&self) -> Vec<(String, String)> {
        self.last_runner().last_log_messages()
    }
    fn last_profile_marks(&self) -> Vec<slicer_wasm_host::profiling::ProfileMark> {
        self.last_runner().last_profile_marks()
    }
    fn last_call_fuel(&self) -> u64 {
        self.last_runner().last_call_fuel()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use preparation_lifecycle_guest::PreparationLifecycleGuest;
    use slicer_sdk::preparation::{PreparedPlanOutput, PrintPreparationInput};
    use slicer_wasm_host::preparation::{
        freeze_native, NativePreparationArtifact, PreparationFailure,
    };

    #[test]
    fn missing_ready_blocks_consumer() {
        let (_, _, scope) = SliceRuntimeExtensions::default().into_run_parts(false, false);
        let manifest = include_str!(
            "../../slicer-wasm-host/test-guests/preparation-lifecycle-guest/module.toml"
        );
        let module_id: ModuleId = "com.test.preparation-lifecycle-guest".into();
        let successful_prepare = NativePreparedStageEntry {
            stage: PreparationLifecycleGuest::__slicer_prepared_native_entry().stage,
            prepare: |_, _| Ok(()),
        };
        let binding = freeze_native(
            module_id.clone(),
            slicer_scheduler::preparation::parse_preparation_declaration(manifest).unwrap(),
            NativePreparationArtifact::Prepared {
                entry: successful_prepare,
                schema: PreparationLifecycleGuest::__SLICER_PREPARATION_SCHEMA,
            },
        )
        .unwrap();
        scope.set_selected(vec![SelectedPreparationBinding {
            module_id: module_id.clone(),
            module_version: "0.1.0".into(),
            artifact_identity: "missing-ready-native".into(),
            consumer_stage: "Layer::PathOptimization".into(),
            binding,
        }]);
        let frozen = Arc::clone(&scope.selected.lock().unwrap()[0]);
        let owner = scope.store.create_owner(&frozen.binding).unwrap();
        scope.owners.lock().unwrap().push((0, owner.clone()));
        let mut consumers = 0;
        let mut prepare_completions = 0;
        let result = (|| -> Result<(), crate::pipeline::PipelineError> {
            scope
                .observe_owner(
                    &frozen,
                    Some(&owner),
                    PreparationLifecycleStatus::Start,
                    None,
                    None,
                )
                .unwrap();
            // Model malformed adapter completion: the real native prepare entry
            // returns success, but its transaction is dropped without finish.
            let transaction = Arc::new(Mutex::new(scope.store.begin(&owner).unwrap()));
            let writer = Arc::clone(&transaction);
            let mut output = PreparedPlanOutput::from_host_writer(Arc::new(move |name, bytes| {
                writer.lock().unwrap().put(name, bytes)
            }));
            let input = PrintPreparationInput::from_host_reader(Arc::new(|_, _| {
                Err(slicer_schema::preparation::PreparationTransportError::AccessDenied)
            }));
            (successful_prepare.prepare)(&input, &mut output).unwrap();
            prepare_completions += 1;
            drop(output);
            drop(transaction);
            // This is the production closure's admission guard, not a mock
            // predicate. Moving/removing it would admit the callback below.
            scope.ensure_all_ready(&[0])?;
            scope
                .observe_owner(
                    &frozen,
                    Some(&owner),
                    PreparationLifecycleStatus::Ready,
                    None,
                    None,
                )
                .unwrap();
            consumers += 1;
            Ok(())
        })();
        assert_eq!(prepare_completions, 1);
        assert!(
            matches!(result, Err(crate::pipeline::PipelineError::Preparation {
            module_id: failed_module,
            failure,
        }) if failed_module == module_id && matches!(failure.as_ref(),
            PreparationFailure::Transport(slicer_schema::preparation::PreparationTransportError::NotReady)))
        );
        assert_eq!(consumers, 0);
        let records = scope.records.lock().unwrap();
        assert_eq!(records.len(), 1);
        assert!(matches!(
            records[0].status,
            PreparationLifecycleStatus::Start
        ));
        assert!(!owner.metadata().ready());
        scope.store.dispose().unwrap();
    }
}
