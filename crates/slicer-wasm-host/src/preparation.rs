//! Frozen preparation identity and owner-private transport failures.

use crate::host::HostExecutionContext;
use slicer_ir::{ModuleId, StageId};
use slicer_schema::preparation::{
    PreparationDeclaration, PreparationDeclarationError, PreparationTransportError,
};
use slicer_sdk::error::ModuleError;
use std::{fmt, sync::Arc};
use wasmtime::component::Component;
use wasmtime::{Engine, Store};

#[path = "preparation_bindings.rs"]
// The fixture-only re-exports make undocumented bindgen-generated members reachable.
// Keep this allowance on generated bindings, never on the authored public facade.
#[cfg_attr(feature = "preparation-test-fixtures", allow(missing_docs))]
mod preparation_bindings;
#[path = "preparation_host.rs"]
mod preparation_host;
#[path = "preparation_native.rs"]
mod preparation_native;
#[path = "preparation_wasm.rs"]
mod preparation_wasm;
/// Real WIT plan lookup handler, exposed only for transport contract fixtures.
#[cfg(feature = "preparation-test-fixtures")]
pub use preparation_bindings::perimeters::slicer::layer_preparation::plan_access::Host as WitPlanAccess;
/// Real WIT piece read and plan open handlers, plus their exact transport error.
#[cfg(feature = "preparation-test-fixtures")]
pub use preparation_bindings::perimeters::slicer::layer_preparation::preparation_types::{
    HostPreparedPiece as WitPreparedPiece, HostPreparedPlanView as WitPreparedPlanView,
    PreparationError as WitPreparationError,
};
pub use preparation_host::PreparedHostState;
pub use preparation_host::{PreparationOutputData, PreparedPieceData, PreparedPlanViewData};
pub use preparation_native::{freeze_native, NativePreparationArtifact, PreparationInputData};

pub use crate::preparation_store::{
    PreparationOwner, PreparationTransaction, PreparedOwnerStore, PreparedPieceHandle,
    PreparedPlanHandle, PreparedPlanMetadata, StagedPreparation,
};

/// Failure at declaration, execution, transport, or publication boundaries.
#[derive(Debug)]
pub enum PreparationFailure {
    /// Declaration validation failed.
    Declaration(PreparationDeclarationError),
    /// Opaque transport access failed.
    Transport(PreparationTransportError),
    /// The module reported an execution failure.
    Module(ModuleError),
    /// Preparation execution trapped.
    Trap {
        /// Execution trap detail.
        detail: String,
    },
    /// The print authority was disposed.
    Disposed,
    /// An active operation prevents disposal.
    Busy,
    /// The owner does not match the print or artifact.
    InvalidOwner,
    /// Preparation has already started for this owner.
    AlreadyStarted,
    /// Publication validation rejected the staged plan.
    PublicationRejected {
        /// Publication rejection detail.
        detail: String,
    },
}

impl fmt::Display for PreparationFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Declaration(error) => error.fmt(f),
            Self::Transport(error) => error.fmt(f),
            Self::Module(error) => error.fmt(f),
            Self::Trap { detail } => write!(f, "preparation trap: {detail}"),
            Self::Disposed => f.write_str("preparation disposed"),
            Self::Busy => f.write_str("preparation busy"),
            Self::InvalidOwner => f.write_str("invalid preparation owner"),
            Self::AlreadyStarted => f.write_str("preparation already started"),
            Self::PublicationRejected { detail } => {
                write!(f, "preparation publication rejected: {detail}")
            }
        }
    }
}
impl std::error::Error for PreparationFailure {}

/// Selected artifact identity. Only declaration-validated adapters construct it.
pub struct FrozenPreparationBinding {
    module_id: ModuleId,
    stage_id: StageId,
    artifact_kind: String,
    artifact_identity: Arc<()>,
    declaration: Option<PreparationDeclaration>,
    native: Option<slicer_sdk::native::NativePreparedStageEntry>,
    native_plain: Option<slicer_sdk::native::NativeStageEntry>,
    wasm: Option<preparation_wasm::FrozenWasmArtifact>,
}

impl FrozenPreparationBinding {
    /// Invoke a frozen WASM component in a fresh owner-bound store.
    pub fn with_wasm_plan<R>(
        &self,
        owner: &PreparationOwner,
        ordinary_context: HostExecutionContext,
        call: impl FnOnce(&Component, &mut Store<PreparedHostState>) -> wasmtime::Result<R>,
    ) -> Result<R, PreparationFailure> {
        owner.originating_store()?.validate_owner(owner, self)?;
        let _guard = owner.active_guard()?;
        let artifact = self.wasm.as_ref().ok_or(PreparationFailure::InvalidOwner)?;
        let mut store = Store::new(
            &artifact.engine,
            PreparedHostState::for_binding(ordinary_context, owner.clone(), self),
        );
        call(&artifact.component, &mut store).map_err(|error| PreparationFailure::Trap {
            detail: error.to_string(),
        })
    }
    /// Return the selected module identity.
    pub fn module_id(&self) -> &ModuleId {
        &self.module_id
    }
    /// Return the selected stage identity.
    pub fn stage_id(&self) -> &StageId {
        &self.stage_id
    }
    /// Return the selected artifact kind.
    pub fn artifact_kind(&self) -> &str {
        &self.artifact_kind
    }
    /// Return the validated preparation declaration.
    pub fn declaration(&self) -> Option<&PreparationDeclaration> {
        self.declaration.as_ref()
    }
    pub(crate) fn artifact_identity(&self) -> Arc<()> {
        Arc::clone(&self.artifact_identity)
    }
}

#[cfg(test)]
pub(crate) fn test_binding() -> FrozenPreparationBinding {
    FrozenPreparationBinding {
        module_id: "test.module".into(),
        stage_id: "layer".into(),
        artifact_kind: "native".into(),
        artifact_identity: Arc::new(()),
        declaration: Some(PreparationDeclaration::new(
            "slicer:layer-preparation/prepare@1.0.0",
            vec![],
            vec!["view".into()],
        )),
        native: None,
        native_plain: None,
        wasm: None,
    }
}

/// Freeze one WASM artifact after typed validation of its declared preparation capability.
pub fn freeze_wasm(
    module_id: ModuleId,
    declaration: Option<PreparationDeclaration>,
    engine: Engine,
    component: Component,
    stage_id: StageId,
) -> Result<FrozenPreparationBinding, PreparationFailure> {
    use PreparationDeclarationError as D;
    let reject = |error| PreparationFailure::Declaration(error);
    if !Engine::same(&engine, component.engine()) {
        return Err(reject(D::IncompatibleExport(module_id)));
    }
    let exported = component
        .get_export_index(None, "slicer:layer-preparation/prepare@1.0.0")
        .is_some();
    match (&declaration, exported) {
        (Some(_), false) => return Err(reject(D::MissingExport(module_id))),
        (None, true) => return Err(reject(D::UndeclaredExport(module_id))),
        (Some(declared), true)
            if format!("{}#prepare-print", declared.interface())
                != slicer_schema::PREPARATION_QUALIFIED_EXPORT =>
        {
            return Err(reject(D::IncompatibleExport(module_id)));
        }
        _ => {}
    }
    let stage = slicer_schema::STAGES
        .iter()
        .find(|stage| stage.stage_id == stage_id)
        .filter(|stage| stage.tier_id == slicer_schema::TIER_LAYER)
        .ok_or_else(|| reject(D::UnsupportedStage(module_id.clone())))?;
    let indices = preparation_wasm::validate_indices(&component, stage.wit_world, exported)
        .map_err(|_| reject(D::IncompatibleExport(module_id.clone())))?;
    Ok(FrozenPreparationBinding {
        module_id,
        stage_id: stage_id.clone(),
        artifact_kind: "wasm".into(),
        artifact_identity: Arc::new(()),
        declaration,
        native: None,
        native_plain: None,
        wasm: Some(preparation_wasm::FrozenWasmArtifact {
            engine,
            component,
            stage_id,
            indices,
        }),
    })
}
