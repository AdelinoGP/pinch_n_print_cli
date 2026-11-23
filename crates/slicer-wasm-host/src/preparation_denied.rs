//! Ordinary-dispatch denial for the preparation imports.
//!
//! A preparation-capable module ships one component artifact that imports
//! `slicer:layer-preparation/preparation-types@1.0.0` and `plan-access`, even
//! when the call is an ordinary stage dispatch. Wasmtime resolves every import
//! at instantiation, so the ordinary linker must provide those two interfaces
//! or the stage cannot run at all.
//!
//! Ordinary dispatch has no preparation owner, binding, grants or committed
//! input, so every host call here is denied. The module's ordinary export must
//! never reach them; a successful ordinary slice is the guard that it does not.
//! The one interface function whose WIT return cannot carry an error
//! (`prepared-piece.len`) traps instead, and resource destructors delete from
//! the same table the preparation host uses.

use crate::host::layer_perimeters::slicer::types::geometry as geo;
use crate::host::HostExecutionContext;
use crate::preparation::{
    plan_access, preparation_types as cap, PreparationInputData, PreparationOutputData,
    PreparationProjectionOutputData, PreparedPieceData, PreparedPlanViewData,
};
use wasmtime::component::Resource;

impl cap::Host for HostExecutionContext {}

impl cap::HostPrintPreparationInput for HostExecutionContext {
    fn layer_count(
        &mut self,
        _resource: Resource<PreparationInputData>,
    ) -> wasmtime::Result<Result<u64, cap::PreparationInputError>> {
        Ok(Err(cap::PreparationInputError::AccessDenied))
    }

    fn layers(
        &mut self,
        _resource: Resource<PreparationInputData>,
        _offset: u64,
        _max_items: u64,
    ) -> wasmtime::Result<Result<Vec<cap::PreparationLayer>, cap::PreparationInputError>> {
        Ok(Err(cap::PreparationInputError::AccessDenied))
    }

    fn regions(
        &mut self,
        _resource: Resource<PreparationInputData>,
        _layer_index: u32,
        _offset: u64,
        _max_items: u64,
    ) -> wasmtime::Result<Result<Vec<cap::PreparationRegion>, cap::PreparationInputError>> {
        Ok(Err(cap::PreparationInputError::AccessDenied))
    }

    fn region_configs(
        &mut self,
        _resource: Resource<PreparationInputData>,
        _layer_index: u32,
        _offset: u64,
        _max_items: u64,
    ) -> wasmtime::Result<Result<Vec<cap::PreparationRegionConfig>, cap::PreparationInputError>>
    {
        Ok(Err(cap::PreparationInputError::AccessDenied))
    }

    fn surface_objects(
        &mut self,
        _resource: Resource<PreparationInputData>,
        _offset: u64,
        _max_items: u64,
    ) -> wasmtime::Result<
        Result<Option<Vec<cap::PreparationSurfaceObject>>, cap::PreparationInputError>,
    > {
        Ok(Err(cap::PreparationInputError::AccessDenied))
    }

    fn surface_layers(
        &mut self,
        _resource: Resource<PreparationInputData>,
        _layer_index: u32,
        _offset: u64,
        _max_items: u64,
    ) -> wasmtime::Result<
        Result<Option<Vec<cap::PreparationSurfaceLayer>>, cap::PreparationInputError>,
    > {
        Ok(Err(cap::PreparationInputError::AccessDenied))
    }

    fn mesh_objects(
        &mut self,
        _resource: Resource<PreparationInputData>,
        _offset: u64,
        _max_items: u64,
    ) -> wasmtime::Result<Result<Vec<cap::PreparationMeshObject>, cap::PreparationInputError>> {
        Ok(Err(cap::PreparationInputError::AccessDenied))
    }

    fn mesh_vertices(
        &mut self,
        _resource: Resource<PreparationInputData>,
        _object_id: String,
        _offset: u64,
        _max_items: u64,
    ) -> wasmtime::Result<Result<Vec<geo::Point3>, cap::PreparationInputError>> {
        Ok(Err(cap::PreparationInputError::AccessDenied))
    }

    fn mesh_triangles(
        &mut self,
        _resource: Resource<PreparationInputData>,
        _object_id: String,
        _offset: u64,
        _max_items: u64,
    ) -> wasmtime::Result<Result<Vec<(u32, u32, u32)>, cap::PreparationInputError>> {
        Ok(Err(cap::PreparationInputError::AccessDenied))
    }

    fn mesh_queries(
        &mut self,
        _resource: Resource<PreparationInputData>,
        _requests: Vec<cap::PreparationMeshQuery>,
    ) -> wasmtime::Result<Result<Vec<cap::PreparationMeshQueryResult>, cap::PreparationInputError>>
    {
        Ok(Err(cap::PreparationInputError::AccessDenied))
    }

    fn seams(
        &mut self,
        _resource: Resource<PreparationInputData>,
        _layer_index: u32,
        _offset: u64,
        _max_items: u64,
    ) -> wasmtime::Result<Result<Option<Vec<cap::PreparationSeam>>, cap::PreparationInputError>>
    {
        Ok(Err(cap::PreparationInputError::AccessDenied))
    }

    fn support_analysis(
        &mut self,
        _resource: Resource<PreparationInputData>,
        _offset: u64,
        _max_items: u64,
    ) -> wasmtime::Result<
        Result<Option<cap::PreparationSupportAnalysisBatch>, cap::PreparationInputError>,
    > {
        Ok(Err(cap::PreparationInputError::AccessDenied))
    }

    fn support_geometry(
        &mut self,
        _resource: Resource<PreparationInputData>,
        _offset: u64,
        _max_items: u64,
    ) -> wasmtime::Result<
        Result<Option<cap::PreparationSupportGeometryBatch>, cap::PreparationInputError>,
    > {
        Ok(Err(cap::PreparationInputError::AccessDenied))
    }

    fn support_plan(
        &mut self,
        _resource: Resource<PreparationInputData>,
        _offset: u64,
        _max_items: u64,
    ) -> wasmtime::Result<
        Result<Option<cap::PreparationSupportPlanBatch>, cap::PreparationInputError>,
    > {
        Ok(Err(cap::PreparationInputError::AccessDenied))
    }

    fn selected_targets(
        &mut self,
        _resource: Resource<PreparationInputData>,
        _layer_index: u32,
        _offset: u64,
        _max_items: u64,
    ) -> wasmtime::Result<Result<Vec<cap::PreparationSelectedTarget>, cap::PreparationInputError>>
    {
        Ok(Err(cap::PreparationInputError::AccessDenied))
    }

    fn read_fixture(
        &mut self,
        _resource: Resource<PreparationInputData>,
        _offset: u64,
        _max_bytes: u64,
    ) -> wasmtime::Result<Result<Vec<u8>, cap::PreparationError>> {
        Ok(Err(cap::PreparationError::AccessDenied))
    }

    fn drop(&mut self, resource: Resource<PreparationInputData>) -> wasmtime::Result<()> {
        self.table.delete(resource)?;
        Ok(())
    }
}

impl cap::HostPreparedPlanOutput for HostExecutionContext {
    fn projection(
        &mut self,
        _resource: Resource<PreparationOutputData>,
    ) -> wasmtime::Result<
        Result<Option<Resource<PreparationProjectionOutputData>>, cap::PreparationProjectionError>,
    > {
        Ok(Err(cap::PreparationProjectionError::AccessDenied))
    }

    fn put(
        &mut self,
        _resource: Resource<PreparationOutputData>,
        _name: String,
        _bytes: Vec<u8>,
    ) -> wasmtime::Result<Result<(), cap::PreparationError>> {
        Ok(Err(cap::PreparationError::AccessDenied))
    }

    fn drop(&mut self, resource: Resource<PreparationOutputData>) -> wasmtime::Result<()> {
        self.table.delete(resource)?;
        Ok(())
    }
}

impl cap::HostPreparationProjectionOutput for HostExecutionContext {
    fn requested_views(
        &mut self,
        _resource: Resource<PreparationProjectionOutputData>,
    ) -> wasmtime::Result<Result<Vec<String>, cap::PreparationProjectionError>> {
        Ok(Err(cap::PreparationProjectionError::AccessDenied))
    }

    fn publish(
        &mut self,
        _resource: Resource<PreparationProjectionOutputData>,
        _scene: cap::PreparationProjectionScene,
    ) -> wasmtime::Result<Result<(), cap::PreparationProjectionError>> {
        Ok(Err(cap::PreparationProjectionError::AccessDenied))
    }

    fn drop(
        &mut self,
        resource: Resource<PreparationProjectionOutputData>,
    ) -> wasmtime::Result<()> {
        self.table.delete(resource)?;
        Ok(())
    }
}

impl cap::HostPreparedPlanView for HostExecutionContext {
    fn open(
        &mut self,
        _resource: Resource<PreparedPlanViewData>,
        _name: String,
    ) -> wasmtime::Result<Result<Option<Resource<PreparedPieceData>>, cap::PreparationError>> {
        Ok(Err(cap::PreparationError::AccessDenied))
    }

    fn drop(&mut self, resource: Resource<PreparedPlanViewData>) -> wasmtime::Result<()> {
        self.table.delete(resource)?;
        Ok(())
    }
}

impl cap::HostPreparedPiece for HostExecutionContext {
    fn len(&mut self, _resource: Resource<PreparedPieceData>) -> wasmtime::Result<u64> {
        // The WIT return is a bare u64, so this call cannot carry a typed
        // denial. Reaching it means ordinary dispatch called a preparation
        // export; fail loudly instead of returning a fabricated length.
        wasmtime::bail!("prepared piece length requested on ordinary dispatch")
    }

    fn read(
        &mut self,
        _resource: Resource<PreparedPieceData>,
        _offset: u64,
        _max_bytes: u64,
    ) -> wasmtime::Result<Result<Vec<u8>, cap::PreparationError>> {
        Ok(Err(cap::PreparationError::AccessDenied))
    }

    fn drop(&mut self, resource: Resource<PreparedPieceData>) -> wasmtime::Result<()> {
        self.table.delete(resource)?;
        Ok(())
    }
}

impl plan_access::Host for HostExecutionContext {
    fn current(
        &mut self,
    ) -> wasmtime::Result<Result<Resource<PreparedPlanViewData>, cap::PreparationError>> {
        Ok(Err(cap::PreparationError::AccessDenied))
    }
}
