//! Concrete owner-private backing types for composed preparation resources.

use super::preparation_bindings::input;
use super::preparation_bindings::perimeters::slicer::layer_preparation::{
    plan_access, preparation_types as cap,
};
use super::preparation_input::OwnerPreparationInput;
pub use super::preparation_native::PreparationInputData;
use super::preparation_native::{PreparationInputReader, PREPARATION_MESH_HELPERS};
use super::{PreparationFailure, PreparationInputError, PreparationOwner};
use super::{PreparationTransaction, PreparedPieceHandle, PreparedPlanHandle};
use crate::host::layer_perimeters::slicer::{common::host_services as hs, types::geometry as geo};
use crate::host::HostExecutionContext;
use slicer_sdk::preparation::{
    PreparationMeshQuery, PreparationMeshQueryResult, PrintPreparationRead,
};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use wasmtime::component::{HasSelf, Linker, Resource};

#[cfg(feature = "preparation-test-fixtures")]
thread_local! {
    static WASM_PREPARATION_LOG_MESSAGES: std::cell::RefCell<Vec<(String, String)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

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
    input_reads: Vec<String>,
    input_failure: Option<PreparationInputError>,
    // Owned guest resource destruction must not abort call finalization.
    dropped_output: Option<PreparationOutputData>,
}

impl PreparedHostState {
    /// Exercise the actual WIT input handler, retaining binding and owner checks.
    /// Used to read a retained input after real owner disposal in contract tests.
    #[cfg(feature = "preparation-test-fixtures")]
    #[doc(hidden)]
    pub fn controlled_layer_count(
        owner: &PreparationOwner,
        binding: &super::FrozenPreparationBinding,
        input: PreparationInputData,
    ) -> Result<u64, PreparationInputError> {
        let ordinary =
            crate::host::HostExecutionContextBuilder::new(binding.module_id().as_str(), 0.0, 0.0)
                .build();
        let mut state = Self::for_binding(ordinary, owner.clone(), binding);
        let resource = state
            .ordinary
            .table
            .push(input)
            .expect("controlled input resource");
        cap::HostPrintPreparationInput::layer_count(&mut state, resource)
            .expect("real input handler must not trap")
            .map_err(|error| match error {
                cap::PreparationInputError::AccessDenied => PreparationInputError::AccessDenied,
                cap::PreparationInputError::RangeError => PreparationInputError::RangeError,
                cap::PreparationInputError::ArithmeticOverflow => {
                    PreparationInputError::ArithmeticOverflow
                }
                cap::PreparationInputError::MissingRequired(read) => {
                    PreparationInputError::MissingRequired { read }
                }
                cap::PreparationInputError::UnsupportedRead(read) => {
                    PreparationInputError::UnsupportedRead { read }
                }
                cap::PreparationInputError::InvalidInput(detail) => {
                    PreparationInputError::InvalidInput { detail }
                }
                cap::PreparationInputError::UnknownObject(object_id) => {
                    PreparationInputError::UnknownObject { object_id }
                }
                cap::PreparationInputError::SelectionRejected(detail) => {
                    PreparationInputError::SelectionRejected { detail }
                }
            })
    }

    /// Wrap ordinary execution state with an explicit preparation owner.
    pub fn new(ordinary: HostExecutionContext, owner: PreparationOwner) -> Self {
        Self {
            ordinary,
            owner,
            fixture_permission: false,
            input_reads: Vec::new(),
            input_failure: None,
            dropped_output: None,
        }
    }

    pub(super) fn for_binding(
        ordinary: HostExecutionContext,
        owner: PreparationOwner,
        binding: &super::FrozenPreparationBinding,
    ) -> Self {
        let mut state = Self::new(ordinary, owner);
        state.input_reads = binding
            .declaration()
            .map(|declaration| declaration.input_reads().to_vec())
            .unwrap_or_default();
        state.fixture_permission = state.input_reads.iter().any(|read| read == "fixture");
        state
    }

    fn read_input<T>(
        &mut self,
        resource: &Resource<PreparationInputData>,
        permission: &str,
        project: impl FnOnce(&OwnerPreparationInput) -> Result<T, PreparationInputError>,
    ) -> Result<T, cap::PreparationInputError> {
        let result = (|| {
            let data = self
                .ordinary
                .table
                .get(resource)
                .map_err(|_| PreparationInputError::AccessDenied)?;
            let backing = data
                .committed
                .as_deref()
                .ok_or(PreparationInputError::AccessDenied)?;
            // Match PreparationInputReader: use the backing's normalized grants,
            // bind the complete frozen declaration, then check owner/liveness.
            // Product availability and range checks only happen in project.
            if !backing.permits(permission) || self.input_reads != backing.owner.input_reads() {
                return Err(PreparationInputError::AccessDenied);
            }
            if !self.owner.same_owner(&backing.owner) {
                return Err(PreparationInputError::AccessDenied);
            }
            let _guard = self
                .owner
                .active_guard()
                .map_err(|_| PreparationInputError::AccessDenied)?;
            project(backing)
        })();
        if let Err(error) = &result {
            self.input_failure = Some(error.clone());
        }
        result.map_err(Into::into)
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

    fn preparation_linker(
        engine: &wasmtime::Engine,
        stage_id: &str,
        input: Arc<PreparationInputReader>,
    ) -> wasmtime::Result<Linker<Self>> {
        let mut linker = Self::linker(engine, stage_id)?;
        linker.allow_shadowing(true);
        {
            // Re-register the complete canonical interface before replacing only
            // its mesh functions. Creating a linker instance shadows the whole
            // interface, not individual functions; non-mesh services must keep
            // their ordinary forwarding unchanged.
            let mut services = linker.instance("slicer:common/host-services")?;
            hs::add_to_linker_instance::<_, HasSelf<HostExecutionContext>>(
                &mut services,
                |state: &mut Self| state.ordinary_mut(),
            )?;
            let reader = input.clone();
            services.func_wrap(
                "raycast-z-down",
                move |_store, (object_id, x, y, start_z): (String, f32, f32, f32)| {
                    let results = reader
                        .mesh_queries(&[PreparationMeshQuery::RaycastZDown {
                            object_id,
                            x,
                            y,
                            start_z,
                        }])
                        .map_err(wasmtime::Error::new)?;
                    let mut results = results.into_iter().map(preparation_raycast_result);
                    Ok((results.next().expect("one result per mesh query"),))
                },
            )?;
            let reader = input.clone();
            services.func_wrap(
                "surface-normal-at",
                move |_store, (object_id, x, y, z): (String, f32, f32, f32)| {
                    let results = reader
                        .mesh_queries(&[PreparationMeshQuery::SurfaceNormalAt {
                            object_id,
                            x,
                            y,
                            z,
                        }])
                        .map_err(wasmtime::Error::new)?;
                    let mut results = results.into_iter().map(preparation_normal_result);
                    Ok((results.next().expect("one result per mesh query"),))
                },
            )?;
            let reader = input.clone();
            services.func_wrap("object-bounds", move |_store, (object_id,): (String,)| {
                let results = reader
                    .mesh_queries(&[PreparationMeshQuery::ObjectBounds { object_id }])
                    .map_err(wasmtime::Error::new)?;
                let Some(PreparationMeshQueryResult::ObjectBounds(bounds)) =
                    results.into_iter().next()
                else {
                    unreachable!("bounds projection preserves mesh query kind and count");
                };
                Ok((geo::BoundingBox3 {
                    min: preparation_point(bounds.min),
                    max: preparation_point(bounds.max),
                },))
            })?;
            let reader = input.clone();
            services.func_wrap(
                "raycast-z-down-batch",
                move |_store, (requests,): (Vec<hs::RaycastRequest>,)| {
                    let requests = requests
                        .into_iter()
                        .map(|request| PreparationMeshQuery::RaycastZDown {
                            object_id: request.object_id,
                            x: request.x,
                            y: request.y,
                            start_z: request.start_z,
                        })
                        .collect::<Vec<_>>();
                    let results = reader
                        .mesh_queries(&requests)
                        .map_err(wasmtime::Error::new)?;
                    Ok((results
                        .into_iter()
                        .map(preparation_raycast_result)
                        .collect::<Vec<_>>(),))
                },
            )?;
            services.func_wrap(
                "surface-normal-at-batch",
                move |_store, (requests,): (Vec<hs::SurfaceNormalRequest>,)| {
                    let requests = requests
                        .into_iter()
                        .map(|request| PreparationMeshQuery::SurfaceNormalAt {
                            object_id: request.object_id,
                            x: request.x,
                            y: request.y,
                            z: request.z,
                        })
                        .collect::<Vec<_>>();
                    let results = input
                        .mesh_queries(&requests)
                        .map_err(wasmtime::Error::new)?;
                    Ok((results
                        .into_iter()
                        .map(preparation_normal_result)
                        .collect::<Vec<_>>(),))
                },
            )?;
        }
        linker.allow_shadowing(false);
        Ok(linker)
    }

    /// Read ordinary G-code output before the owner-bound call's store is dropped.
    pub fn gcode_output(&self) -> &crate::host::GcodeOutputCollected {
        self.ordinary.gcode_output()
    }

    pub(super) fn ordinary_mut(&mut self) -> &mut HostExecutionContext {
        &mut self.ordinary
    }
}

fn preparation_point(point: slicer_ir::Point3) -> geo::Point3 {
    geo::Point3 {
        x: point.x,
        y: point.y,
        z: point.z,
    }
}

fn preparation_raycast_result(result: PreparationMeshQueryResult) -> Option<f32> {
    match result {
        PreparationMeshQueryResult::RaycastZDown(hit) => hit,
        _ => unreachable!("raycast projection preserves mesh query kind"),
    }
}

fn preparation_normal_result(result: PreparationMeshQueryResult) -> Option<geo::Point3> {
    match result {
        PreparationMeshQueryResult::SurfaceNormalAt(normal) => normal.map(preparation_point),
        _ => unreachable!("normal projection preserves mesh query kind"),
    }
}

impl super::FrozenPreparationBinding {
    /// Drain the most recent WASM preparation call's logs on this thread.
    /// Every call clears the previous capture, including calls rejected early.
    #[cfg(feature = "preparation-test-fixtures")]
    pub fn take_wasm_log_messages() -> Vec<(String, String)> {
        WASM_PREPARATION_LOG_MESSAGES.with(|messages| std::mem::take(&mut *messages.borrow_mut()))
    }

    pub(super) fn prepare_wasm(
        &self,
        owner: &PreparationOwner,
        input: PreparationInputData,
        validate: impl FnOnce(&super::StagedPreparation) -> Result<(), PreparationFailure>,
    ) -> Result<(), PreparationFailure> {
        #[cfg(feature = "preparation-test-fixtures")]
        WASM_PREPARATION_LOG_MESSAGES.with(|messages| messages.borrow_mut().clear());
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
        let reader = Arc::new(PreparationInputReader::new(owner, &input, self));
        let config = match input.committed.as_deref() {
            Some(backing)
                if owner.same_owner(&backing.owner)
                    && state.input_reads == backing.owner.input_reads() =>
            {
                crate::host::config_view_to_data(backing.invocation_config())
            }
            // A foreign resource may enter the call, but must expose neither
            // its configuration nor product data. read_input rejects it at the
            // actual resource boundary, identically to the native reader.
            _ => crate::host::ConfigViewData {
                fields: Default::default(),
            },
        };
        let trap =
            |error: wasmtime::Error| match error.downcast_ref::<super::PreparationInputError>() {
                Some(error) => PreparationFailure::Input(error.clone()),
                None => PreparationFailure::Trap {
                    detail: error.to_string(),
                },
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
        // Preserve input/output resource identities. This ordinary config
        // resource is call-local and is never retained beyond the fresh store.
        let config = state.ordinary.push_config_view(config).map_err(trap)?;
        let mut store = wasmtime::Store::new(&artifact.engine, state);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let linker =
                PreparedHostState::preparation_linker(&artifact.engine, &artifact.stage_id, reader)
                    .map_err(trap)?;
            let instance = linker
                .instantiate(&mut store, &artifact.component)
                .map_err(trap)?;
            let configured = artifact
                .indices
                .configure(&mut store, &instance, config)
                .map_err(trap)?;
            let result = match configured {
                Ok(()) => {
                    owner.record_prepare_invocation()?;
                    artifact
                        .indices
                        .prepare(&mut store, &instance, input, output)
                        .map_err(trap)?
                }
                Err(error) => Err(error),
            };
            result.map_err(|error| {
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
                match store.data().input_failure.clone() {
                    Some(error) => PreparationFailure::Input(error),
                    None => PreparationFailure::Module(error),
                }
            })
        }))
        .unwrap_or_else(|_| {
            Err(PreparationFailure::Trap {
                detail: "WASM preparation panic".into(),
            })
        });
        let state = store.data_mut();
        #[cfg(feature = "preparation-test-fixtures")]
        WASM_PREPARATION_LOG_MESSAGES.with(|messages| {
            *messages.borrow_mut() = std::mem::take(state.ordinary.log_messages_mut());
        });
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
    fn layer_count(
        &mut self,
        resource: Resource<PreparationInputData>,
    ) -> wasmtime::Result<Result<u64, cap::PreparationInputError>> {
        Ok(self.read_input(&resource, "layer_plan", OwnerPreparationInput::layer_count))
    }

    fn layers(
        &mut self,
        resource: Resource<PreparationInputData>,
        offset: u64,
        max_items: u64,
    ) -> wasmtime::Result<Result<Vec<cap::PreparationLayer>, cap::PreparationInputError>> {
        let layers = match self.read_input(&resource, "layer_plan", |input| {
            input.layers(offset, max_items)
        }) {
            Ok(layers) => layers,
            Err(error) => return Ok(Err(error)),
        };
        Ok(Ok(layers
            .into_iter()
            .map(|layer| input::layer(&mut self.ordinary, layer))
            .collect::<wasmtime::Result<_>>()?))
    }

    fn regions(
        &mut self,
        resource: Resource<PreparationInputData>,
        layer_index: u32,
        offset: u64,
        max_items: u64,
    ) -> wasmtime::Result<Result<Vec<cap::PreparationRegion>, cap::PreparationInputError>> {
        Ok(self
            .read_input(&resource, "slices", |input| {
                input.regions(layer_index, offset, max_items)
            })
            .map(|regions| regions.into_iter().map(input::region).collect()))
    }

    fn region_configs(
        &mut self,
        resource: Resource<PreparationInputData>,
        layer_index: u32,
        offset: u64,
        max_items: u64,
    ) -> wasmtime::Result<Result<Vec<cap::PreparationRegionConfig>, cap::PreparationInputError>>
    {
        let configs = match self.read_input(&resource, "region_map", |input| {
            input.region_configs(layer_index, offset, max_items)
        }) {
            Ok(configs) => configs,
            Err(error) => return Ok(Err(error)),
        };
        Ok(Ok(configs
            .into_iter()
            .map(|config| input::region_config(&mut self.ordinary, config))
            .collect::<wasmtime::Result<_>>()?))
    }

    fn surface_objects(
        &mut self,
        resource: Resource<PreparationInputData>,
        offset: u64,
        max_items: u64,
    ) -> wasmtime::Result<
        Result<Option<Vec<cap::PreparationSurfaceObject>>, cap::PreparationInputError>,
    > {
        Ok(self
            .read_input(&resource, "surface_classification", |input| {
                input.surface_objects(offset, max_items)
            })
            .map(|objects| {
                objects.map(|objects| objects.into_iter().map(input::surface_object).collect())
            }))
    }

    fn surface_layers(
        &mut self,
        resource: Resource<PreparationInputData>,
        layer_index: u32,
        offset: u64,
        max_items: u64,
    ) -> wasmtime::Result<
        Result<Option<Vec<cap::PreparationSurfaceLayer>>, cap::PreparationInputError>,
    > {
        Ok(self
            .read_input(&resource, "surface_classification", |input| {
                input.surface_layers(layer_index, offset, max_items)
            })
            .map(|layers| {
                layers.map(|layers| layers.into_iter().map(input::surface_layer).collect())
            }))
    }

    fn mesh_objects(
        &mut self,
        resource: Resource<PreparationInputData>,
        offset: u64,
        max_items: u64,
    ) -> wasmtime::Result<Result<Vec<cap::PreparationMeshObject>, cap::PreparationInputError>> {
        Ok(self
            .read_input(&resource, "mesh", |input| {
                input.mesh_objects(offset, max_items, &PREPARATION_MESH_HELPERS)
            })
            .map(|objects| objects.into_iter().map(input::mesh_object).collect()))
    }

    fn mesh_vertices(
        &mut self,
        resource: Resource<PreparationInputData>,
        object_id: String,
        offset: u64,
        max_items: u64,
    ) -> wasmtime::Result<Result<Vec<geo::Point3>, cap::PreparationInputError>> {
        Ok(self
            .read_input(&resource, "mesh", |input| {
                input.mesh_vertices(&object_id, offset, max_items, &PREPARATION_MESH_HELPERS)
            })
            .map(|vertices| vertices.into_iter().map(preparation_point).collect()))
    }

    fn mesh_triangles(
        &mut self,
        resource: Resource<PreparationInputData>,
        object_id: String,
        offset: u64,
        max_items: u64,
    ) -> wasmtime::Result<Result<Vec<(u32, u32, u32)>, cap::PreparationInputError>> {
        Ok(self.read_input(&resource, "mesh", |input| {
            input.mesh_triangles(&object_id, offset, max_items)
        }))
    }

    fn mesh_queries(
        &mut self,
        resource: Resource<PreparationInputData>,
        requests: Vec<cap::PreparationMeshQuery>,
    ) -> wasmtime::Result<Result<Vec<cap::PreparationMeshQueryResult>, cap::PreparationInputError>>
    {
        // Lower only inside the authorized projection, as with native queries.
        Ok(self
            .read_input(&resource, "mesh", |backing| {
                let requests = requests
                    .into_iter()
                    .map(input::mesh_query)
                    .collect::<Vec<_>>();
                backing.mesh_queries(&requests, &PREPARATION_MESH_HELPERS)
            })
            .map(|results| results.into_iter().map(input::mesh_query_result).collect()))
    }

    fn seams(
        &mut self,
        resource: Resource<PreparationInputData>,
        layer_index: u32,
        offset: u64,
        max_items: u64,
    ) -> wasmtime::Result<Result<Option<Vec<cap::PreparationSeam>>, cap::PreparationInputError>>
    {
        Ok(self
            .read_input(&resource, "seam_plan", |input| {
                input.seams(layer_index, offset, max_items)
            })
            .map(|seams| seams.map(|seams| seams.into_iter().map(input::seam).collect())))
    }

    fn support_analysis(
        &mut self,
        resource: Resource<PreparationInputData>,
        offset: u64,
        max_items: u64,
    ) -> wasmtime::Result<
        Result<Option<cap::PreparationSupportAnalysisBatch>, cap::PreparationInputError>,
    > {
        Ok(self
            .read_input(&resource, "support_analysis", |input| {
                input.support_analysis(offset, max_items)
            })
            .map(|batch| batch.map(input::support_analysis)))
    }

    fn support_geometry(
        &mut self,
        resource: Resource<PreparationInputData>,
        offset: u64,
        max_items: u64,
    ) -> wasmtime::Result<
        Result<Option<cap::PreparationSupportGeometryBatch>, cap::PreparationInputError>,
    > {
        Ok(self
            .read_input(&resource, "support_geometry", |input| {
                input.support_geometry(offset, max_items)
            })
            .map(|batch| batch.map(input::support_geometry)))
    }

    fn support_plan(
        &mut self,
        resource: Resource<PreparationInputData>,
        offset: u64,
        max_items: u64,
    ) -> wasmtime::Result<
        Result<Option<cap::PreparationSupportPlanBatch>, cap::PreparationInputError>,
    > {
        Ok(self
            .read_input(&resource, "support_plan", |input| {
                input.support_plan(offset, max_items)
            })
            .map(|batch| batch.map(input::support_plan)))
    }

    fn selected_targets(
        &mut self,
        resource: Resource<PreparationInputData>,
        layer_index: u32,
        offset: u64,
        max_items: u64,
    ) -> wasmtime::Result<Result<Vec<cap::PreparationSelectedTarget>, cap::PreparationInputError>>
    {
        let targets = match self.read_input(&resource, "selected_targets", |input| {
            input.selected_targets(layer_index, offset, max_items)
        }) {
            Ok(targets) => targets,
            Err(error) => return Ok(Err(error)),
        };
        Ok(Ok(targets
            .into_iter()
            .map(|target| input::selected_target(&mut self.ordinary, target))
            .collect::<wasmtime::Result<_>>()?))
    }

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
