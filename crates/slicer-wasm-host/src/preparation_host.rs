//! Concrete owner-private backing types for composed preparation resources.

use super::preparation_bindings::perimeters::slicer::layer_preparation::{
    plan_access, preparation_types as cap,
};
pub use super::preparation_native::PreparationInputData;
use super::{PreparationFailure, PreparationOwner};
use super::{PreparationTransaction, PreparedPieceHandle, PreparedPlanHandle};
use crate::host::HostExecutionContext;
use std::ops::{Deref, DerefMut};
use wasmtime::component::{Linker, Resource};

/// Call-local authority for writing unpublished preparation pieces.
pub struct PreparationOutputData {
    transaction: PreparationTransaction,
    owner: PreparationOwner,
}

impl Deref for PreparationOutputData {
    type Target = PreparationTransaction;

    fn deref(&self) -> &Self::Target {
        &self.transaction
    }
}

impl DerefMut for PreparationOutputData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.transaction
    }
}

/// Weak owner authority for reading a published preparation plan.
pub struct PreparedPlanViewData {
    plan: PreparedPlanHandle,
    owner: PreparationOwner,
}

impl Deref for PreparedPlanViewData {
    type Target = PreparedPlanHandle;

    fn deref(&self) -> &Self::Target {
        &self.plan
    }
}

/// Weak owner authority for reading one published opaque piece.
pub struct PreparedPieceData {
    piece: PreparedPieceHandle,
    owner: PreparationOwner,
}

/// Fresh owner-bound WASM state sharing the ordinary resource table.
pub struct PreparedHostState {
    ordinary: HostExecutionContext,
    owner: PreparationOwner,
    fixture_permission: bool,
    // Owned guest resource destruction must not abort call finalization.
    dropped_output: Option<PreparationOutputData>,
}

impl PreparedHostState {
    /// Wrap ordinary execution state with an explicit preparation owner.
    pub fn new(ordinary: HostExecutionContext, owner: PreparationOwner) -> Self {
        Self {
            ordinary,
            owner,
            fixture_permission: false,
            dropped_output: None,
        }
    }

    pub(super) fn for_binding(
        ordinary: HostExecutionContext,
        owner: PreparationOwner,
        binding: &super::FrozenPreparationBinding,
    ) -> Self {
        let mut state = Self::new(ordinary, owner);
        state.fixture_permission = binding.declaration().is_some_and(|declaration| {
            declaration
                .input_reads()
                .iter()
                .any(|permission| permission == "fixture")
        });
        state
    }

    fn authorized(&self, owner: &PreparationOwner) -> Result<(), cap::PreparationError> {
        if !self.owner.same_owner(owner) {
            return Err(cap::PreparationError::AccessDenied);
        }
        owner
            .active_guard()
            .map(|_| ())
            .map_err(|_| cap::PreparationError::AccessDenied)
    }

    /// Register the selected ordinary world once, then only preparation imports.
    pub fn linker(engine: &wasmtime::Engine, stage_id: &str) -> wasmtime::Result<Linker<Self>> {
        super::preparation_wasm::linker(engine, stage_id)
    }

    /// Read ordinary G-code output before the owner-bound call's store is dropped.
    pub fn gcode_output(&self) -> &crate::host::GcodeOutputCollected {
        self.ordinary.gcode_output()
    }

    pub(super) fn ordinary_mut(&mut self) -> &mut HostExecutionContext {
        &mut self.ordinary
    }
}

impl super::FrozenPreparationBinding {
    pub(super) fn prepare_wasm(
        &self,
        owner: &PreparationOwner,
        input: PreparationInputData,
        validate: impl FnOnce(&super::StagedPreparation) -> Result<(), PreparationFailure>,
    ) -> Result<(), PreparationFailure> {
        let authority = owner.originating_store()?;
        authority.validate_owner(owner, self)?;
        let _guard = owner.active_guard()?;
        let artifact = self.wasm.as_ref().ok_or(PreparationFailure::InvalidOwner)?;
        if self.declaration().is_none() {
            return Err(PreparationFailure::Declaration(
                super::PreparationDeclarationError::MissingExport(self.module_id().clone()),
            ));
        }
        let transaction = authority.begin(owner)?;
        let ordinary =
            crate::host::HostExecutionContextBuilder::new(self.module_id().as_str(), 0.0, 0.0)
                .build();
        let mut state = PreparedHostState::for_binding(ordinary, owner.clone(), self);
        let trap = |error: wasmtime::Error| PreparationFailure::Trap {
            detail: error.to_string(),
        };
        let input = state
            .ordinary
            .table
            .push(input)
            .map_err(|error| trap(error.into()))?;
        let output = state
            .ordinary
            .table
            .push(PreparationOutputData {
                transaction,
                owner: owner.clone(),
            })
            .map_err(|error| trap(error.into()))?;
        let output_rep = output.rep();
        let mut store = wasmtime::Store::new(&artifact.engine, state);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let linker =
                PreparedHostState::linker(&artifact.engine, &artifact.stage_id).map_err(trap)?;
            let instance = linker
                .instantiate(&mut store, &artifact.component)
                .map_err(trap)?;
            owner.record_prepare_invocation()?;
            artifact
                .indices
                .prepare(&mut store, &instance, input, output)
                .map_err(trap)?
                .map_err(|error| {
                    let error = slicer_sdk::error::ModuleError {
                        code: error.code,
                        message: error.message,
                        fatal: error.fatal,
                    };
                    if slicer_sdk::preparation::is_poisoned_preparation_error(&error) {
                        return PreparationFailure::Transport(
                            slicer_schema::preparation::PreparationTransportError::PoisonedOutput,
                        );
                    }
                    PreparationFailure::Module(error)
                })
        }))
        .unwrap_or_else(|_| {
            Err(PreparationFailure::Trap {
                detail: "WASM preparation panic".into(),
            })
        });
        let state = store.data_mut();
        let mut output = match state.dropped_output.take() {
            Some(output) => output,
            // Traps can interrupt the guest before its owned resource drops.
            None => state
                .ordinary
                .table
                .delete(Resource::<PreparationOutputData>::new_own(output_rep))
                .map_err(|error| trap(error.into()))?,
        };
        drop(store);
        if matches!(
            &result,
            Err(PreparationFailure::Transport(
                slicer_schema::preparation::PreparationTransportError::PoisonedOutput
            ))
        ) {
            output.transaction.record_writer_fault(
                slicer_schema::preparation::PreparationTransportError::PoisonedOutput,
            );
        }
        output
            .transaction
            .finish(result, |_| validate(&owner.staged_description()?))
            .map(|_| ())
    }
}

fn boundary(error: slicer_schema::preparation::PreparationTransportError) -> cap::PreparationError {
    use slicer_schema::preparation::PreparationTransportError as E;
    match error {
        E::AccessDenied => cap::PreparationError::AccessDenied,
        E::NotReady => cap::PreparationError::NotReady,
        E::InvalidName => cap::PreparationError::InvalidName,
        E::DuplicateName => cap::PreparationError::DuplicateName,
        E::RangeError => cap::PreparationError::RangeError,
        E::ArithmeticOverflow => cap::PreparationError::ArithmeticOverflow,
        E::PoisonedOutput => cap::PreparationError::PoisonedOutput,
    }
}

impl cap::Host for PreparedHostState {}
impl cap::HostPrintPreparationInput for PreparedHostState {
    fn read_fixture(
        &mut self,
        resource: Resource<PreparationInputData>,
        offset: u64,
        max_bytes: u64,
    ) -> wasmtime::Result<Result<Vec<u8>, cap::PreparationError>> {
        #[cfg(feature = "preparation-test-fixtures")]
        if self.fixture_permission {
            let Ok(input) = self.ordinary.table.get(&resource) else {
                return Ok(Err(cap::PreparationError::AccessDenied));
            };
            return Ok(input.read(&self.owner, offset, max_bytes).map_err(boundary));
        }
        let _ = (resource, offset, max_bytes);
        Ok(Err(cap::PreparationError::AccessDenied))
    }
    fn drop(&mut self, resource: Resource<PreparationInputData>) -> wasmtime::Result<()> {
        self.ordinary.table.delete(resource)?;
        Ok(())
    }
}
impl cap::HostPreparedPlanOutput for PreparedHostState {
    fn put(
        &mut self,
        resource: Resource<PreparationOutputData>,
        name: String,
        bytes: Vec<u8>,
    ) -> wasmtime::Result<Result<(), cap::PreparationError>> {
        let Ok(data) = self.ordinary.table.get(&resource) else {
            return Ok(Err(cap::PreparationError::AccessDenied));
        };
        if let Err(error) = self.authorized(&data.owner) {
            return Ok(Err(error));
        }
        Ok(self
            .ordinary
            .table
            .get_mut(&resource)?
            .transaction
            .put(&name, &bytes)
            .map_err(boundary))
    }
    fn drop(&mut self, resource: Resource<PreparationOutputData>) -> wasmtime::Result<()> {
        self.dropped_output = Some(self.ordinary.table.delete(resource)?);
        Ok(())
    }
}
impl cap::HostPreparedPlanView for PreparedHostState {
    fn open(
        &mut self,
        resource: Resource<PreparedPlanViewData>,
        name: String,
    ) -> wasmtime::Result<Result<Option<Resource<PreparedPieceData>>, cap::PreparationError>> {
        let Ok(data) = self.ordinary.table.get(&resource) else {
            return Ok(Err(cap::PreparationError::AccessDenied));
        };
        if let Err(error) = self.authorized(&data.owner) {
            return Ok(Err(error));
        }
        let owner = data.owner.clone();
        match data.plan.open(&name) {
            Ok(piece) => Ok(Ok(piece
                .map(|piece| self.ordinary.table.push(PreparedPieceData { piece, owner }))
                .transpose()?)),
            Err(error) => Ok(Err(boundary(error))),
        }
    }
    fn drop(&mut self, resource: Resource<PreparedPlanViewData>) -> wasmtime::Result<()> {
        self.ordinary.table.delete(resource)?;
        Ok(())
    }
}
impl cap::HostPreparedPiece for PreparedHostState {
    fn len(&mut self, resource: Resource<PreparedPieceData>) -> wasmtime::Result<u64> {
        Ok(self.ordinary.table.get(&resource)?.piece.len())
    }
    fn read(
        &mut self,
        resource: Resource<PreparedPieceData>,
        offset: u64,
        max_bytes: u64,
    ) -> wasmtime::Result<Result<Vec<u8>, cap::PreparationError>> {
        let Ok(data) = self.ordinary.table.get(&resource) else {
            return Ok(Err(cap::PreparationError::AccessDenied));
        };
        if let Err(error) = self.authorized(&data.owner) {
            return Ok(Err(error));
        }
        Ok(data.piece.read(offset, max_bytes).map_err(boundary))
    }
    fn drop(&mut self, resource: Resource<PreparedPieceData>) -> wasmtime::Result<()> {
        self.ordinary.table.delete(resource)?;
        Ok(())
    }
}
impl plan_access::Host for PreparedHostState {
    fn current(
        &mut self,
    ) -> wasmtime::Result<Result<Resource<PreparedPlanViewData>, cap::PreparationError>> {
        let plan = self
            .owner
            .originating_store()
            .and_then(|store| store.current(&self.owner));
        match plan {
            Ok(plan) => Ok(Ok(self.ordinary.table.push(PreparedPlanViewData {
                plan,
                owner: self.owner.clone(),
            })?)),
            Err(PreparationFailure::Transport(error)) => Ok(Err(boundary(error))),
            Err(_) => Ok(Err(cap::PreparationError::AccessDenied)),
        }
    }
}

impl Deref for PreparedPieceData {
    type Target = PreparedPieceHandle;

    fn deref(&self) -> &Self::Target {
        &self.piece
    }
}

#[cfg(test)]
mod tests {
    use super::super::{test_binding, PreparedOwnerStore};
    use super::*;
    use crate::host::HostExecutionContextBuilder;
    use wasmtime::component::HasSelf;

    fn state(owner: PreparationOwner) -> PreparedHostState {
        PreparedHostState::new(
            HostExecutionContextBuilder::new("test.module", 0.0, 0.2).build(),
            owner,
        )
    }

    #[test]
    fn fixture_owner_reads_only_with_exact_grant() {
        let mut binding = test_binding();
        binding.declaration = Some(slicer_schema::preparation::PreparationDeclaration::new(
            "slicer:layer-preparation/prepare@1.0.0",
            vec!["fixture".into()],
            vec!["view".into()],
        ));
        let store = PreparedOwnerStore::new();
        let owner = store.create_owner(&binding).expect("owner");
        let other = store.create_owner(&binding).expect("different owner");
        let ordinary = || HostExecutionContextBuilder::new("test.module", 0.0, 0.2).build();
        let mut state = PreparedHostState::for_binding(ordinary(), owner.clone(), &binding);
        #[cfg(feature = "preparation-test-fixtures")]
        let fixture =
            || PreparationInputData::controlled_fixture(&owner, std::sync::Arc::from([3, 5]));
        #[cfg(not(feature = "preparation-test-fixtures"))]
        let fixture = PreparationInputData::default;
        let input = state.ordinary.table.push(fixture()).expect("input");
        #[cfg(feature = "preparation-test-fixtures")]
        let expected = Ok(vec![3, 5]);
        #[cfg(not(feature = "preparation-test-fixtures"))]
        let expected = Err(cap::PreparationError::AccessDenied);
        assert_eq!(
            cap::HostPrintPreparationInput::read_fixture(&mut state, input, 0, u64::MAX)
                .expect("correct-owner boundary"),
            expected
        );
        let mut foreign = PreparedHostState::for_binding(ordinary(), other, &binding);
        let input = foreign
            .ordinary
            .table
            .push(fixture())
            .expect("foreign input");
        assert_eq!(
            cap::HostPrintPreparationInput::read_fixture(&mut foreign, input, 0, 2)
                .expect("foreign-owner boundary"),
            Err(cap::PreparationError::AccessDenied)
        );
        binding.declaration = Some(slicer_schema::preparation::PreparationDeclaration::new(
            "slicer:layer-preparation/prepare@1.0.0",
            vec!["fixture_extra".into()],
            vec!["view".into()],
        ));
        let mut ungranted = PreparedHostState::for_binding(ordinary(), owner.clone(), &binding);
        let input = ungranted
            .ordinary
            .table
            .push(fixture())
            .expect("ungranted input");
        assert_eq!(
            cap::HostPrintPreparationInput::read_fixture(&mut ungranted, input, 0, 2)
                .expect("ungranted boundary"),
            Err(cap::PreparationError::AccessDenied)
        );
    }

    #[cfg(feature = "preparation-test-fixtures")]
    #[test]
    fn ordinary_ir_reads_do_not_grant_preparation_access() {
        use slicer_scheduler::{manifest::LoadedModuleBuilder, preparation::PreparationManifest};

        // Freeze/context APIs do not ingest ordinary read grants. Exercise the
        // real manifest representation, not an IR-looking preparation string.
        let manifest = PreparationManifest::new(
            LoadedModuleBuilder::new(
                "test.module",
                Default::default(),
                "Layer::Perimeters",
                "perimeters-module",
                "unused.wasm",
            )
            .ir_reads(vec!["SliceIR.regions.polygons".into()])
            .build(),
            Some(slicer_schema::preparation::PreparationDeclaration::new(
                "slicer:layer-preparation/prepare@1.0.0",
                vec![],
                vec!["view".into()],
            )),
        );
        assert_eq!(manifest.module().ir_reads(), &["SliceIR.regions.polygons"]);
        let mut binding = test_binding();
        binding.declaration = manifest.preparation().cloned();
        assert!(binding
            .declaration()
            .expect("declaration")
            .input_reads()
            .is_empty());
        let store = PreparedOwnerStore::new();
        let owner = store.create_owner(&binding).expect("owner");
        let ordinary = || HostExecutionContextBuilder::new("test.module", 0.0, 0.2).build();
        let fixture =
            || PreparationInputData::controlled_fixture(&owner, std::sync::Arc::from([3, 5]));

        let mut ordinary_read = PreparedHostState::for_binding(ordinary(), owner.clone(), &binding);
        assert!(!ordinary_read.fixture_permission);
        assert_eq!(fixture().read(&owner, 0, 2), Ok(vec![3, 5]));
        let input = ordinary_read
            .ordinary
            .table
            .push(fixture())
            .expect("ordinary-read input");
        assert_eq!(
            cap::HostPrintPreparationInput::read_fixture(&mut ordinary_read, input, 0, 2)
                .expect("ordinary-read boundary"),
            Err(cap::PreparationError::AccessDenied)
        );

        binding.declaration = Some(slicer_schema::preparation::PreparationDeclaration::new(
            "slicer:layer-preparation/prepare@1.0.0",
            vec!["fixture".into()],
            vec!["view".into()],
        ));
        let mut fixture_grant = PreparedHostState::for_binding(ordinary(), owner.clone(), &binding);
        let input = fixture_grant
            .ordinary
            .table
            .push(fixture())
            .expect("fixture-granted input");
        assert_eq!(
            cap::HostPrintPreparationInput::read_fixture(&mut fixture_grant, input, 0, 2)
                .expect("fixture-granted boundary"),
            Ok(vec![3, 5])
        );
    }

    #[test]
    fn read_fixture_denied_in_production_mode() {
        let mut binding = test_binding();
        binding.declaration = Some(slicer_schema::preparation::PreparationDeclaration::new(
            "slicer:layer-preparation/prepare@1.0.0",
            vec!["fixture".into()],
            vec!["view".into()],
        ));
        let store = PreparedOwnerStore::new();
        let owner = store.create_owner(&binding).expect("owner");
        let input = PreparationInputData::for_owner(&owner);
        let mut state = PreparedHostState::for_binding(
            HostExecutionContextBuilder::new("test.module", 0.0, 0.2).build(),
            owner,
            &binding,
        );
        assert!(
            state.fixture_permission,
            "production denial must not be a missing grant"
        );
        let resource = state.ordinary.table.push(input).expect("input");
        assert_eq!(
            cap::HostPrintPreparationInput::read_fixture(&mut state, resource, 0, 1)
                .expect("host boundary"),
            Err(cap::PreparationError::AccessDenied)
        );
        assert_eq!(
            boundary(slicer_schema::preparation::PreparationTransportError::AccessDenied),
            cap::PreparationError::AccessDenied
        );
    }

    #[test]
    fn foreign_owner_output_denied() {
        let binding = test_binding();
        let store = PreparedOwnerStore::new();
        let owner = store.create_owner(&binding).expect("owner");
        let foreign = store.create_owner(&binding).expect("foreign");
        let mut state = state(owner.clone());
        let own = state
            .ordinary
            .table
            .push(PreparationOutputData {
                transaction: store.begin(&owner).expect("own transaction"),
                owner: owner.clone(),
            })
            .expect("own resource");
        assert_eq!(
            cap::HostPreparedPlanOutput::put(
                &mut state,
                Resource::new_borrow(own.rep()),
                "view".into(),
                vec![3]
            )
            .expect("own host call"),
            Ok(())
        );
        let other = state
            .ordinary
            .table
            .push(PreparationOutputData {
                transaction: store.begin(&foreign).expect("foreign transaction"),
                owner: foreign.clone(),
            })
            .expect("foreign resource");
        assert_eq!(
            cap::HostPreparedPlanOutput::put(&mut state, other, "view".into(), vec![5])
                .expect("foreign host call"),
            Err(cap::PreparationError::AccessDenied)
        );
        assert_eq!(owner.metadata().put_calls(), 1);
        assert_eq!(foreign.metadata().put_calls(), 0);
    }

    #[test]
    fn piece_length_survives_disposal_and_reads_are_denied() {
        let binding = test_binding();
        let store = PreparedOwnerStore::new();
        let owner = store.create_owner(&binding).expect("owner");
        let mut transaction = store.begin(&owner).expect("transaction");
        transaction.put("view", &[3, 5]).expect("put");
        transaction.finish(Ok(()), |_| Ok(())).expect("publish");
        let mut state = state(owner.clone());
        let plan = plan_access::Host::current(&mut state)
            .expect("host current")
            .expect("ready");
        let piece = cap::HostPreparedPlanView::open(&mut state, plan, "view".into())
            .expect("host open")
            .expect("authorized")
            .expect("piece");
        assert_eq!(
            cap::HostPreparedPiece::read(&mut state, Resource::new_borrow(piece.rep()), 0, 2)
                .expect("read before disposal"),
            Ok(vec![3, 5])
        );
        store.dispose_owner(&owner).expect("dispose");
        assert_eq!(
            cap::HostPreparedPiece::len(&mut state, Resource::new_borrow(piece.rep()))
                .expect("captured length"),
            2
        );
        assert_eq!(
            cap::HostPreparedPiece::read(&mut state, piece, 0, 2).expect("host read"),
            Err(cap::PreparationError::AccessDenied)
        );
        assert_eq!(owner.metadata().retained_payload_bytes(), 0);
    }

    #[test]
    fn projected_linker_registers_shared_imports_once() {
        let engine = wasmtime::Engine::default();
        let stages: Vec<_> = slicer_schema::STAGES
            .iter()
            .filter(|stage| {
                stage.tier_id == slicer_schema::TIER_LAYER && !stage.wit_package.is_empty()
            })
            .collect();
        assert!(!stages.is_empty(), "no schema-derived Layer worlds");
        for stage in stages {
            let mut linker = PreparedHostState::linker(&engine, stage.stage_id)
                .unwrap_or_else(|error| panic!("{} failed registration: {error}", stage.stage_id));
            let duplicate =
                crate::host::layer_perimeters::slicer::common::host_services::add_to_linker::<
                    _,
                    HasSelf<HostExecutionContext>,
                >(&mut linker, |state: &mut PreparedHostState| {
                    &mut state.ordinary
                });
            let error = duplicate.expect_err("duplicate shared interface must be rejected");
            assert!(
                error.to_string().contains("defined"),
                "unexpected duplicate failure: {error}"
            );
        }
    }
}
