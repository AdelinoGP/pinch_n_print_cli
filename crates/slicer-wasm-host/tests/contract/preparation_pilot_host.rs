//! Test-only preparation storage and composed typed drivers. No production storage changes.
#![allow(dead_code)]

use slicer_wasm_host::host::{self, HostExecutionContext, HostExecutionContextBuilder};
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use wasmtime::component::{Component, HasSelf, Linker, Resource, ResourceTable};
use wasmtime::{Engine, Store};

pub struct PreparationInputData {
    owner: String,
    bytes: Arc<[u8]>,
}
pub struct PreparationOutputData {
    owner: String,
}
pub struct PreparedPlanViewData {
    owner: String,
    pieces: Arc<BTreeMap<String, Arc<[u8]>>>,
}
pub struct PreparedPieceData {
    owner: String,
    bytes: Arc<[u8]>,
}

pub mod infill {
    wasmtime::component::bindgen!({
        path: "../slicer-schema/wit",
        world: "slicer:layer-preparation/prepared-infill-module",
        imports: { default: trappable },
        with: {
            "slicer:types/geometry": host::layer_perimeters::slicer::types::geometry,
            "slicer:config/config-types": host::layer_perimeters::slicer::config::config_types,
            "slicer:common/host-services": host::layer_perimeters::slicer::common::host_services,
            "slicer:common/profiling": host::layer_perimeters::slicer::common::profiling,
            "slicer:common/module-errors": host::layer_perimeters::slicer::common::module_errors,
            "slicer:ir-handles/ir-handles@1.0.0": host::layer_perimeters::slicer::ir_handles::ir_handles,
            "slicer:support-queries/support-queries@1.0.0": host::layer_infill::slicer::support_queries::support_queries,
            "slicer:lightning-queries/lightning-queries@1.0.0": host::layer_infill::slicer::lightning_queries::lightning_queries,
            "slicer:layer-preparation/preparation-types@1.0.0.print-preparation-input": super::PreparationInputData,
            "slicer:layer-preparation/preparation-types@1.0.0.prepared-plan-output": super::PreparationOutputData,
            "slicer:layer-preparation/preparation-types@1.0.0.prepared-plan-view": super::PreparedPlanViewData,
            "slicer:layer-preparation/preparation-types@1.0.0.prepared-piece": super::PreparedPieceData,
        },
    });
    use super::host;
}

pub mod path {
    wasmtime::component::bindgen!({
        path: "../slicer-schema/wit",
        world: "slicer:layer-preparation/prepared-path-optimization-module",
        imports: { default: trappable },
        with: {
            "slicer:types/geometry": host::layer_perimeters::slicer::types::geometry,
            "slicer:config/config-types": host::layer_perimeters::slicer::config::config_types,
            "slicer:common/host-services": host::layer_perimeters::slicer::common::host_services,
            "slicer:common/profiling": host::layer_perimeters::slicer::common::profiling,
            "slicer:common/module-errors": host::layer_perimeters::slicer::common::module_errors,
            "slicer:ir-handles/ir-handles@1.0.0": host::layer_perimeters::slicer::ir_handles::ir_handles,
            "slicer:layer-preparation/preparation-types@1.0.0": super::infill::slicer::layer_preparation::preparation_types,
            "slicer:layer-preparation/plan-access@1.0.0": super::infill::slicer::layer_preparation::plan_access,
        },
    });
    use super::host;
}

pub mod perimeters {
    wasmtime::component::bindgen!({
        path: "../slicer-schema/wit",
        world: "slicer:layer-preparation/prepared-perimeters-module",
        imports: { default: trappable },
        with: {
            "slicer:types/geometry": host::layer_perimeters::slicer::types::geometry,
            "slicer:config/config-types": host::layer_perimeters::slicer::config::config_types,
            "slicer:common/host-services": host::layer_perimeters::slicer::common::host_services,
            "slicer:common/profiling": host::layer_perimeters::slicer::common::profiling,
            "slicer:common/module-errors": host::layer_perimeters::slicer::common::module_errors,
            "slicer:ir-handles/ir-handles@1.0.0": host::layer_perimeters::slicer::ir_handles::ir_handles,
            "slicer:layer-preparation/preparation-types@1.0.0": super::infill::slicer::layer_preparation::preparation_types,
            "slicer:layer-preparation/plan-access@1.0.0": super::infill::slicer::layer_preparation::plan_access,
        },
    });
    use super::host;
}

/// Typed prepared load with the requested capability as diagnostic context.
pub fn load_prepared_perimeters(engine: &Engine, component: &Component) -> wasmtime::Result<()> {
    let linker = pilot_linker(engine, false)?;
    let mut store = Store::new(engine, PilotHostContext::new("prepared-load", None));
    // Establish that the ordinary stage really loads before attributing any
    // combined-world error to the additional preparation capability.
    host::LayerModule::instantiate(&mut store, component, &linker)?;
    perimeters::PreparedPerimetersModule::instantiate(&mut store, component, &linker)
        .map(|_| ())
        .map_err(|error| {
            let detail = format!("{error:#}");
            let identity = slicer_schema::PREPARATION_QUALIFIED_EXPORT;
            wasmtime::Error::msg(format!("{identity}: {detail}"))
        })
}

pub use cap::PreparationError;
use infill::slicer::layer_preparation::{plan_access, preparation_types as cap};

#[derive(Clone, Debug)]
pub struct ReadyPlan {
    pub owner: String,
    pub print_id: String,
    pub pieces: Arc<BTreeMap<String, Arc<[u8]>>>,
}

#[derive(Clone, Debug)]
pub struct TransferMetadata {
    pub direction: &'static str,
    pub bytes: u64,
    pub count: u64,
}

/// An explicit payload-free projection, never a serialization of host storage.
#[derive(Clone, Debug)]
pub struct PilotMetadata {
    pub owner: String,
    pub phase: &'static str,
    pub preparation_lifecycle: &'static str,
    pub ready: bool,
    pub pieces: Vec<(String, u64)>,
    pub payload_bytes: u64,
    pub name_utf8_bytes: u64,
    pub transfers: Vec<TransferMetadata>,
    pub prepare_duration_ns: Option<u64>,
    pub read_duration_ns: Vec<u64>,
    instrumented: bool,
    preparation_started: Option<std::time::Instant>,
}

impl PilotMetadata {
    fn new(owner: &str, ready: Option<&ReadyPlan>) -> Self {
        let mut metadata = Self {
            owner: owner.into(),
            phase: "preparation",
            preparation_lifecycle: "not_started",
            ready: ready.is_some(),
            pieces: Vec::new(),
            payload_bytes: 0,
            name_utf8_bytes: 0,
            transfers: Vec::new(),
            prepare_duration_ns: None,
            read_duration_ns: Vec::new(),
            instrumented: false,
            preparation_started: None,
        };
        if let Some(ready) = ready {
            metadata.phase = "layer";
            metadata.preparation_lifecycle = "complete";
            for (name, bytes) in ready.pieces.iter() {
                metadata
                    .retain(name, bytes.len())
                    .expect("ready accounting must fit u64");
            }
        }
        metadata
    }
    fn retain(&mut self, name: &str, len: usize) -> Result<(), PreparationError> {
        let payload = u64::try_from(len).map_err(|_| PreparationError::ArithmeticOverflow)?;
        let names = u64::try_from(name.len()).map_err(|_| PreparationError::ArithmeticOverflow)?;
        let payload_total = self
            .payload_bytes
            .checked_add(payload)
            .ok_or(PreparationError::ArithmeticOverflow)?;
        let name_total = self
            .name_utf8_bytes
            .checked_add(names)
            .ok_or(PreparationError::ArithmeticOverflow)?;
        self.payload_bytes = payload_total;
        self.name_utf8_bytes = name_total;
        self.pieces.push((name.into(), payload));
        Ok(())
    }
    fn transfer(&mut self, direction: &'static str, len: usize) -> Result<(), PreparationError> {
        self.transfers.push(TransferMetadata {
            direction,
            bytes: u64::try_from(len).map_err(|_| PreparationError::ArithmeticOverflow)?,
            count: 1,
        });
        Ok(())
    }
    pub fn set_instrumented(&mut self, instrumented: bool) {
        self.instrumented = instrumented;
    }
    pub fn start_preparation(&mut self) {
        self.phase = "preparation";
        self.preparation_lifecycle = "running";
        self.preparation_started = self.instrumented.then(std::time::Instant::now);
    }
    pub fn complete_preparation(&mut self) {
        self.prepare_duration_ns = self.preparation_started.take().map(elapsed_ns);
    }
    pub fn to_toml(&self) -> toml::Value {
        fn integer(value: u64) -> toml::Value {
            toml::Value::Integer(i64::try_from(value).expect("observed metadata fits TOML integer"))
        }
        let mut table = toml::Table::new();
        for (key, value) in [
            ("owner", self.owner.as_str()),
            ("phase", self.phase),
            ("preparation_lifecycle", self.preparation_lifecycle),
        ] {
            table.insert(key.into(), toml::Value::String(value.into()));
        }
        table.insert("ready".into(), toml::Value::Boolean(self.ready));
        table.insert("payload_bytes".into(), integer(self.payload_bytes));
        table.insert("name_utf8_bytes".into(), integer(self.name_utf8_bytes));
        table.insert(
            "pieces".into(),
            toml::Value::Array(
                self.pieces
                    .iter()
                    .map(|(name, len)| {
                        toml::Value::Table(toml::Table::from_iter([
                            ("name".into(), toml::Value::String(name.clone())),
                            ("length".into(), integer(*len)),
                        ]))
                    })
                    .collect(),
            ),
        );
        table.insert(
            "transfers".into(),
            toml::Value::Array(
                self.transfers
                    .iter()
                    .map(|transfer| {
                        toml::Value::Table(toml::Table::from_iter([
                            (
                                "direction".into(),
                                toml::Value::String(transfer.direction.into()),
                            ),
                            ("bytes".into(), integer(transfer.bytes)),
                            ("count".into(), integer(transfer.count)),
                        ]))
                    })
                    .collect(),
            ),
        );
        if let Some(duration) = self.prepare_duration_ns {
            table.insert("prepare_duration_ns".into(), integer(duration));
        }
        if !self.read_duration_ns.is_empty() {
            table.insert(
                "read_duration_ns".into(),
                toml::Value::Array(self.read_duration_ns.iter().copied().map(integer).collect()),
            );
        }
        toml::Value::Table(table)
    }
}

fn elapsed_ns(started: std::time::Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos())
        .expect("observed call duration fits u64 nanoseconds")
}

pub struct PilotHostContext {
    pub ordinary: HostExecutionContext,
    pub table: ResourceTable,
    pub owner: String,
    pub print_id: String,
    pub fixture_permission: Option<String>,
    pub staged: BTreeMap<String, Arc<[u8]>>,
    pub poisoned: bool,
    pub ready: Option<ReadyPlan>,
    pub piece_reads: Vec<(u64, u64, Result<Vec<u8>, PreparationError>)>,
    pub piece_read_accounting: Option<u64>,
    pub metadata: PilotMetadata,
    lifetime: Option<StoreLifetime>,
}

struct StoreLifetime(Arc<AtomicBool>);
impl Drop for StoreLifetime {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

impl PilotHostContext {
    pub fn new(owner: &str, ready: Option<ReadyPlan>) -> Self {
        let metadata = PilotMetadata::new(
            owner,
            ready
                .as_ref()
                .filter(|plan| plan.owner == owner && plan.print_id == "pilot-print"),
        );
        Self {
            ordinary: HostExecutionContextBuilder::new(owner, 0.2, 0.2).build(),
            table: ResourceTable::new(),
            owner: owner.into(),
            print_id: "pilot-print".into(),
            fixture_permission: None,
            staged: BTreeMap::new(),
            poisoned: false,
            ready,
            piece_reads: Vec::new(),
            piece_read_accounting: None,
            metadata,
            lifetime: None,
        }
    }
    pub fn bind_print(&mut self, print_id: &str) {
        self.print_id = print_id.into();
        // A foreign plan is not ready for this call and must not contribute
        // another owner's/print's piece names or retention to its diagnostics.
        self.metadata = PilotMetadata::new(
            &self.owner,
            self.ready
                .as_ref()
                .filter(|plan| plan.owner == self.owner && plan.print_id == self.print_id),
        );
    }
    fn authorized(&self, owner: &str) -> Result<(), PreparationError> {
        if owner == self.owner {
            Ok(())
        } else {
            Err(PreparationError::AccessDenied)
        }
    }
    /// Any failed preparation invalidates its staging, including SDK-local
    /// rejected puts followed by an unchanged module error. Never classify
    /// publication safety by replacing the module's original error identity.
    pub fn record_preparation_error(&mut self, _code: u32) {
        self.poisoned = true;
    }

    pub fn finish(&mut self, success: bool) -> Option<ReadyPlan> {
        self.ready = if success && !self.poisoned {
            Some(ReadyPlan {
                owner: self.owner.clone(),
                print_id: self.print_id.clone(),
                pieces: Arc::new(std::mem::take(&mut self.staged)),
            })
        } else {
            self.staged.clear();
            None
        };
        self.metadata.ready = self.ready.is_some();
        self.metadata.preparation_lifecycle = if self.ready.is_some() {
            "complete"
        } else {
            "failed"
        };
        if self.ready.is_none() {
            self.metadata.payload_bytes = 0;
            self.metadata.name_utf8_bytes = 0;
            self.metadata.pieces.clear();
        }
        self.ready.clone()
    }
    pub fn push_preparation_input(
        &mut self,
        bytes: Arc<[u8]>,
        allowed: bool,
    ) -> wasmtime::Result<Resource<PreparationInputData>> {
        self.fixture_permission = allowed.then(|| self.owner.clone());
        Ok(self.table.push(PreparationInputData {
            owner: self.owner.clone(),
            bytes,
        })?)
    }
    pub fn push_preparation_output(&mut self) -> wasmtime::Result<Resource<PreparationOutputData>> {
        Ok(self.table.push(PreparationOutputData {
            owner: self.owner.clone(),
        })?)
    }
}

pub fn bounded_read(bytes: &[u8], offset: u64, max: u64) -> Result<Vec<u8>, PreparationError> {
    let len = u64::try_from(bytes.len()).map_err(|_| PreparationError::ArithmeticOverflow)?;
    let remaining = len
        .checked_sub(offset)
        .ok_or(PreparationError::RangeError)?;
    let start = usize::try_from(offset).map_err(|_| PreparationError::ArithmeticOverflow)?;
    let count =
        usize::try_from(remaining.min(max)).map_err(|_| PreparationError::ArithmeticOverflow)?;
    // count <= bytes.len() - start, so this addition is checked independently.
    let end = start
        .checked_add(count)
        .ok_or(PreparationError::ArithmeticOverflow)?;
    Ok(bytes[start..end].to_vec())
}

/// A synthetic accounting budget, independent of range clipping or allocation.
pub fn accounted_read(
    bytes: &[u8],
    offset: u64,
    max: u64,
    accounting: &mut Option<u64>,
) -> Result<Vec<u8>, PreparationError> {
    let len = u64::try_from(bytes.len()).map_err(|_| PreparationError::ArithmeticOverflow)?;
    let remaining = len
        .checked_sub(offset)
        .ok_or(PreparationError::RangeError)?;
    if let Some(total) = accounting {
        *total = total
            .checked_add(remaining.min(max))
            .ok_or(PreparationError::ArithmeticOverflow)?;
    }
    bounded_read(bytes, offset, max)
}

impl cap::Host for PilotHostContext {}
impl cap::HostPrintPreparationInput for PilotHostContext {
    fn layer_count(
        &mut self,
        resource: Resource<PreparationInputData>,
    ) -> wasmtime::Result<Result<u64, cap::PreparationInputError>> {
        let _ = resource;
        Ok(Err(cap::PreparationInputError::AccessDenied))
    }
    fn layers(
        &mut self,
        resource: Resource<PreparationInputData>,
        offset: u64,
        max_items: u64,
    ) -> wasmtime::Result<Result<Vec<cap::PreparationLayer>, cap::PreparationInputError>> {
        let _ = (resource, offset, max_items);
        Ok(Err(cap::PreparationInputError::AccessDenied))
    }
    fn regions(
        &mut self,
        resource: Resource<PreparationInputData>,
        layer_index: u32,
        offset: u64,
        max_items: u64,
    ) -> wasmtime::Result<Result<Vec<cap::PreparationRegion>, cap::PreparationInputError>> {
        let _ = (resource, layer_index, offset, max_items);
        Ok(Err(cap::PreparationInputError::AccessDenied))
    }
    fn region_configs(
        &mut self,
        resource: Resource<PreparationInputData>,
        layer_index: u32,
        offset: u64,
        max_items: u64,
    ) -> wasmtime::Result<Result<Vec<cap::PreparationRegionConfig>, cap::PreparationInputError>>
    {
        let _ = (resource, layer_index, offset, max_items);
        Ok(Err(cap::PreparationInputError::AccessDenied))
    }
    fn surface_objects(
        &mut self,
        resource: Resource<PreparationInputData>,
        offset: u64,
        max_items: u64,
    ) -> wasmtime::Result<
        Result<Option<Vec<cap::PreparationSurfaceObject>>, cap::PreparationInputError>,
    > {
        let _ = (resource, offset, max_items);
        Ok(Err(cap::PreparationInputError::AccessDenied))
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
        let _ = (resource, layer_index, offset, max_items);
        Ok(Err(cap::PreparationInputError::AccessDenied))
    }
    fn mesh_objects(
        &mut self,
        resource: Resource<PreparationInputData>,
        offset: u64,
        max_items: u64,
    ) -> wasmtime::Result<Result<Vec<cap::PreparationMeshObject>, cap::PreparationInputError>> {
        let _ = (resource, offset, max_items);
        Ok(Err(cap::PreparationInputError::AccessDenied))
    }
    fn mesh_vertices(
        &mut self,
        resource: Resource<PreparationInputData>,
        object_id: String,
        offset: u64,
        max_items: u64,
    ) -> wasmtime::Result<
        Result<
            Vec<host::layer_perimeters::slicer::types::geometry::Point3>,
            cap::PreparationInputError,
        >,
    > {
        let _ = (resource, object_id, offset, max_items);
        Ok(Err(cap::PreparationInputError::AccessDenied))
    }
    fn mesh_triangles(
        &mut self,
        resource: Resource<PreparationInputData>,
        object_id: String,
        offset: u64,
        max_items: u64,
    ) -> wasmtime::Result<Result<Vec<(u32, u32, u32)>, cap::PreparationInputError>> {
        let _ = (resource, object_id, offset, max_items);
        Ok(Err(cap::PreparationInputError::AccessDenied))
    }
    fn mesh_queries(
        &mut self,
        resource: Resource<PreparationInputData>,
        requests: Vec<cap::PreparationMeshQuery>,
    ) -> wasmtime::Result<Result<Vec<cap::PreparationMeshQueryResult>, cap::PreparationInputError>>
    {
        let _ = (resource, requests);
        Ok(Err(cap::PreparationInputError::AccessDenied))
    }
    fn seams(
        &mut self,
        resource: Resource<PreparationInputData>,
        layer_index: u32,
        offset: u64,
        max_items: u64,
    ) -> wasmtime::Result<Result<Option<Vec<cap::PreparationSeam>>, cap::PreparationInputError>>
    {
        let _ = (resource, layer_index, offset, max_items);
        Ok(Err(cap::PreparationInputError::AccessDenied))
    }
    fn support_analysis(
        &mut self,
        resource: Resource<PreparationInputData>,
        offset: u64,
        max_items: u64,
    ) -> wasmtime::Result<
        Result<Option<cap::PreparationSupportAnalysisBatch>, cap::PreparationInputError>,
    > {
        let _ = (resource, offset, max_items);
        Ok(Err(cap::PreparationInputError::AccessDenied))
    }
    fn support_geometry(
        &mut self,
        resource: Resource<PreparationInputData>,
        offset: u64,
        max_items: u64,
    ) -> wasmtime::Result<
        Result<Option<cap::PreparationSupportGeometryBatch>, cap::PreparationInputError>,
    > {
        let _ = (resource, offset, max_items);
        Ok(Err(cap::PreparationInputError::AccessDenied))
    }
    fn support_plan(
        &mut self,
        resource: Resource<PreparationInputData>,
        offset: u64,
        max_items: u64,
    ) -> wasmtime::Result<
        Result<Option<cap::PreparationSupportPlanBatch>, cap::PreparationInputError>,
    > {
        let _ = (resource, offset, max_items);
        Ok(Err(cap::PreparationInputError::AccessDenied))
    }
    fn selected_targets(
        &mut self,
        resource: Resource<PreparationInputData>,
        layer_index: u32,
        offset: u64,
        max_items: u64,
    ) -> wasmtime::Result<Result<Vec<cap::PreparationSelectedTarget>, cap::PreparationInputError>>
    {
        let _ = (resource, layer_index, offset, max_items);
        Ok(Err(cap::PreparationInputError::AccessDenied))
    }
    fn read_fixture(
        &mut self,
        r: Resource<PreparationInputData>,
        offset: u64,
        max: u64,
    ) -> wasmtime::Result<Result<Vec<u8>, PreparationError>> {
        let data = self.table.get(&r)?;
        if self.authorized(&data.owner).is_err()
            || self.fixture_permission.as_deref() != Some(data.owner.as_str())
        {
            return Ok(Err(PreparationError::AccessDenied));
        }
        let result = bounded_read(&data.bytes, offset, max);
        if let Ok(bytes) = &result {
            self.metadata
                .transfer("host_to_module_input", bytes.len())?;
        }
        Ok(result)
    }
    fn drop(&mut self, r: Resource<PreparationInputData>) -> wasmtime::Result<()> {
        self.table.delete(r)?;
        Ok(())
    }
}
impl cap::HostPreparedPlanOutput for PilotHostContext {
    fn put(
        &mut self,
        r: Resource<PreparationOutputData>,
        name: String,
        bytes: Vec<u8>,
    ) -> wasmtime::Result<Result<(), PreparationError>> {
        let data = self.table.get(&r)?;
        if let Err(e) = self.authorized(&data.owner) {
            self.poisoned = true;
            return Ok(Err(e));
        }
        if self.poisoned {
            return Ok(Err(PreparationError::PoisonedOutput));
        }
        if name.is_empty() || name.chars().any(char::is_control) {
            self.poisoned = true;
            return Ok(Err(PreparationError::InvalidName));
        }
        if self.staged.contains_key(&name) {
            self.poisoned = true;
            return Ok(Err(PreparationError::DuplicateName));
        }
        if let Err(error) = self.metadata.retain(&name, bytes.len()) {
            self.poisoned = true;
            return Ok(Err(error));
        }
        self.metadata.transfer("module_to_host_put", bytes.len())?;
        self.staged.insert(name, Arc::from(bytes));
        Ok(Ok(()))
    }
    fn drop(&mut self, r: Resource<PreparationOutputData>) -> wasmtime::Result<()> {
        self.table.delete(r)?;
        Ok(())
    }
}
impl plan_access::Host for PilotHostContext {
    fn current(
        &mut self,
    ) -> wasmtime::Result<Result<Resource<PreparedPlanViewData>, PreparationError>> {
        let Some(plan) = &self.ready else {
            return Ok(Err(PreparationError::NotReady));
        };
        if plan.print_id != self.print_id {
            return Ok(Err(PreparationError::NotReady));
        }
        if let Err(e) = self.authorized(&plan.owner) {
            return Ok(Err(e));
        }
        Ok(Ok(self.table.push(PreparedPlanViewData {
            owner: plan.owner.clone(),
            pieces: plan.pieces.clone(),
        })?))
    }
}
impl cap::HostPreparedPlanView for PilotHostContext {
    fn open(
        &mut self,
        r: Resource<PreparedPlanViewData>,
        name: String,
    ) -> wasmtime::Result<Result<Option<Resource<PreparedPieceData>>, PreparationError>> {
        let data = self.table.get(&r)?;
        if let Err(e) = self.authorized(&data.owner) {
            return Ok(Err(e));
        }
        let piece = data.pieces.get(&name).map(|bytes| PreparedPieceData {
            owner: data.owner.clone(),
            bytes: bytes.clone(),
        });
        Ok(Ok(piece.map(|p| self.table.push(p)).transpose()?))
    }
    fn drop(&mut self, r: Resource<PreparedPlanViewData>) -> wasmtime::Result<()> {
        self.table.delete(r)?;
        Ok(())
    }
}
impl cap::HostPreparedPiece for PilotHostContext {
    fn len(&mut self, r: Resource<PreparedPieceData>) -> wasmtime::Result<u64> {
        let data = self.table.get(&r)?;
        self.authorized(&data.owner)
            .map_err(|e| wasmtime::Error::msg(format!("{e:?}")))?;
        Ok(u64::try_from(data.bytes.len())?)
    }
    fn read(
        &mut self,
        r: Resource<PreparedPieceData>,
        offset: u64,
        max: u64,
    ) -> wasmtime::Result<Result<Vec<u8>, PreparationError>> {
        let started = self.metadata.instrumented.then(std::time::Instant::now);
        let data = self.table.get(&r)?;
        if let Err(e) = self.authorized(&data.owner) {
            return Ok(Err(e));
        }
        let result = accounted_read(&data.bytes, offset, max, &mut self.piece_read_accounting);
        if let Ok(bytes) = &result {
            self.metadata.transfer("host_to_module_read", bytes.len())?;
        }
        if let Some(started) = started {
            self.metadata.read_duration_ns.push(elapsed_ns(started));
        }
        self.piece_reads.push((offset, max, result.clone()));
        Ok(result)
    }
    fn drop(&mut self, r: Resource<PreparedPieceData>) -> wasmtime::Result<()> {
        self.table.delete(r)?;
        Ok(())
    }
}

pub fn pilot_linker(
    engine: &Engine,
    with_infill: bool,
) -> wasmtime::Result<Linker<PilotHostContext>> {
    use host::layer_perimeters::slicer::{common, config, ir_handles};
    let mut linker = Linker::new(engine);
    common::host_services::add_to_linker::<_, HasSelf<HostExecutionContext>>(
        &mut linker,
        |c: &mut PilotHostContext| &mut c.ordinary,
    )?;
    common::profiling::add_to_linker::<_, HasSelf<HostExecutionContext>>(
        &mut linker,
        |c: &mut PilotHostContext| &mut c.ordinary,
    )?;
    config::config_types::add_to_linker::<_, HasSelf<HostExecutionContext>>(
        &mut linker,
        |c: &mut PilotHostContext| &mut c.ordinary,
    )?;
    ir_handles::ir_handles::add_to_linker::<_, HasSelf<HostExecutionContext>>(
        &mut linker,
        |c: &mut PilotHostContext| &mut c.ordinary,
    )?;
    if with_infill {
        host::layer_infill::slicer::support_queries::support_queries::add_to_linker::<
            _,
            HasSelf<HostExecutionContext>,
        >(&mut linker, |c: &mut PilotHostContext| &mut c.ordinary)?;
        host::layer_infill::slicer::lightning_queries::lightning_queries::add_to_linker::<
            _,
            HasSelf<HostExecutionContext>,
        >(&mut linker, |c: &mut PilotHostContext| &mut c.ordinary)?;
    }
    cap::add_to_linker::<_, HasSelf<PilotHostContext>>(&mut linker, |c| c)?;
    plan_access::add_to_linker::<_, HasSelf<PilotHostContext>>(&mut linker, |c| c)?;
    Ok(linker)
}

#[derive(Clone, Copy)]
pub enum PilotFamily {
    Infill,
    Path,
}
pub struct PreparationOutcome {
    pub ready: Option<ReadyPlan>,
    pub failure: Option<String>,
    pub module_error: Option<host::layer_perimeters::slicer::common::module_errors::ModuleError>,
    pub poisoned: bool,
    pub owner: String,
    pub phase: &'static str,
    pub store_destroyed: bool,
    pub metadata: PilotMetadata,
}

pub fn prepare(
    engine: &Engine,
    component: &Component,
    family: PilotFamily,
    owner: &str,
    fixture: Arc<[u8]>,
) -> wasmtime::Result<PreparationOutcome> {
    prepare_with_permission(engine, component, family, owner, fixture, true)
}

/// Allow a real guest's fixture import to witness denied read permission.
pub fn prepare_with_permission(
    engine: &Engine,
    component: &Component,
    family: PilotFamily,
    owner: &str,
    fixture: Arc<[u8]>,
    allowed: bool,
) -> wasmtime::Result<PreparationOutcome> {
    prepare_in_print(
        engine,
        component,
        family,
        owner,
        fixture,
        allowed,
        "pilot-print",
    )
}

pub fn prepare_in_print(
    engine: &Engine,
    component: &Component,
    family: PilotFamily,
    owner: &str,
    fixture: Arc<[u8]>,
    allowed: bool,
    print_id: &str,
) -> wasmtime::Result<PreparationOutcome> {
    prepare_observed(
        engine, component, family, owner, fixture, allowed, print_id, false,
    )
}

pub fn prepare_observed(
    engine: &Engine,
    component: &Component,
    family: PilotFamily,
    owner: &str,
    fixture: Arc<[u8]>,
    allowed: bool,
    print_id: &str,
    instrumented: bool,
) -> wasmtime::Result<PreparationOutcome> {
    let linker = pilot_linker(engine, matches!(family, PilotFamily::Infill))?;
    let mut ctx = PilotHostContext::new(owner, None);
    ctx.bind_print(print_id);
    ctx.metadata.set_instrumented(instrumented);
    let destroyed = Arc::new(AtomicBool::new(false));
    ctx.lifetime = Some(StoreLifetime(destroyed.clone()));
    ctx.fixture_permission = allowed.then(|| owner.into());
    let input = ctx.table.push(PreparationInputData {
        owner: owner.into(),
        bytes: fixture,
    })?;
    let output = ctx.table.push(PreparationOutputData {
        owner: owner.into(),
    })?;
    let mut store = Store::new(engine, ctx);
    let result = match family {
        PilotFamily::Infill => {
            let module = infill::PreparedInfillModule::instantiate(&mut store, component, &linker)?;
            store.data_mut().metadata.start_preparation();
            module
                .slicer_layer_preparation_prepare()
                .call_prepare_print(&mut store, input, output)
        }
        PilotFamily::Path => {
            let module =
                path::PreparedPathOptimizationModule::instantiate(&mut store, component, &linker)?;
            store.data_mut().metadata.start_preparation();
            module
                .slicer_layer_preparation_prepare()
                .call_prepare_print(&mut store, input, output)
        }
    };
    let (mut failure, module_error) = match result {
        Ok(Ok(())) => (None, None),
        Ok(Err(e)) => {
            store.data_mut().record_preparation_error(e.code);
            (
                Some(format!("owner={owner}; phase=preparation; {e:?}")),
                Some(e),
            )
        }
        Err(e) => {
            let details = store
                .data()
                .ordinary
                .log_messages()
                .iter()
                .filter(|(level, _)| level == "error")
                .map(|(_, detail)| detail.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            (
                Some(format!(
                    "owner={owner}; phase=preparation; {e:#}; {details}"
                )),
                None,
            )
        }
    };
    if failure.is_none() && store.data().poisoned {
        failure = Some("layer preparation: poisoned-output".into());
    }
    let ready = store.data_mut().finish(failure.is_none());
    store.data_mut().metadata.complete_preparation();
    let metadata = store.data().metadata.clone();
    let poisoned = store.data().poisoned;
    // Guest-owned imported handles are deleted by the typed resource destructors;
    // any trap survivors die with this call's table. No instance survives publication.
    drop(store);
    Ok(PreparationOutcome {
        ready,
        failure,
        module_error,
        poisoned,
        owner: owner.into(),
        phase: "preparation",
        store_destroyed: destroyed.load(Ordering::SeqCst),
        metadata,
    })
}

pub fn consume(
    engine: &Engine,
    component: &Component,
    family: PilotFamily,
    owner: &str,
    ready: Option<ReadyPlan>,
    fields: HashMap<String, host::ConfigValueStorage>,
    layer: u32,
) -> wasmtime::Result<PilotHostContext> {
    let (ctx, failure) = consume_outcome(engine, component, family, owner, ready, fields, layer)?;
    if let Some(failure) = failure {
        return Err(wasmtime::Error::msg(failure));
    }
    Ok(ctx)
}

/// Preserve the actual builder even when the typed module returns failure.
pub fn consume_outcome(
    engine: &Engine,
    component: &Component,
    family: PilotFamily,
    owner: &str,
    ready: Option<ReadyPlan>,
    fields: HashMap<String, host::ConfigValueStorage>,
    layer: u32,
) -> wasmtime::Result<(PilotHostContext, Option<String>)> {
    consume_in_print(
        engine,
        component,
        family,
        owner,
        ready,
        fields,
        layer,
        "pilot-print",
    )
}

pub fn consume_in_print(
    engine: &Engine,
    component: &Component,
    family: PilotFamily,
    owner: &str,
    ready: Option<ReadyPlan>,
    fields: HashMap<String, host::ConfigValueStorage>,
    layer: u32,
    print_id: &str,
) -> wasmtime::Result<(PilotHostContext, Option<String>)> {
    let linker = pilot_linker(engine, matches!(family, PilotFamily::Infill))?;
    let mut ctx = PilotHostContext::new(owner, ready);
    ctx.bind_print(print_id);
    ctx.metadata.phase = "layer";
    ctx.metadata.set_instrumented(matches!(
        fields.get("pilot_host_instrumented"),
        Some(host::ConfigValueStorage::Bool(true))
    ));
    if matches!(
        fields.get("pilot_host_accounting_overflow"),
        Some(host::ConfigValueStorage::Bool(true))
    ) {
        ctx.piece_read_accounting = Some(u64::MAX);
    }
    let config = ctx
        .ordinary
        .push_config_view(host::ConfigViewData { fields })?;
    let mut store = Store::new(engine, ctx);
    let result = match family {
        PilotFamily::Infill => {
            let paint = store.data_mut().ordinary.push_paint_region_layer_view(
                host::PaintRegionLayerData {
                    layer_index: layer,
                    ..Default::default()
                },
            )?;
            let support = store
                .data_mut()
                .ordinary
                .push_support_query_view(host::SupportQueryViewData::default())?;
            let lightning = store
                .data_mut()
                .ordinary
                .push_lightning_query_view(host::LightningQueryViewData::default())?;
            let output = store.data_mut().ordinary.push_infill_output_builder()?;
            infill::PreparedInfillModule::instantiate(&mut store, component, &linker)?
                .slicer_layer_infill_infill()
                .call_run(
                    &mut store,
                    i32::try_from(layer)?,
                    &[],
                    paint,
                    support,
                    lightning,
                    output,
                    config,
                )?
        }
        PilotFamily::Path => {
            let output = store.data_mut().ordinary.push_gcode_output_builder()?;
            let collection = store
                .data_mut()
                .ordinary
                .push_layer_collection_builder(Vec::new())?;
            path::PreparedPathOptimizationModule::instantiate(&mut store, component, &linker)?
                .slicer_layer_path_optimization_path_optimization()
                .call_run(
                    &mut store,
                    i32::try_from(layer)?,
                    &[],
                    output,
                    collection,
                    config,
                )?
        }
    };
    let failure = result.err().map(|e| format!("{e:?}"));
    Ok((store.into_data(), failure))
}

pub fn prepare_then_consume(
    engine: &Engine,
    component: &Component,
    family: PilotFamily,
    owner: &str,
    fixture: Arc<[u8]>,
    fields: HashMap<String, host::ConfigValueStorage>,
    layers: &[u32],
) -> wasmtime::Result<(PreparationOutcome, Vec<PilotHostContext>)> {
    let prepared = prepare(engine, component, family, owner, fixture)?;
    let mut consumers = Vec::new();
    if prepared.failure.is_none() && !prepared.poisoned {
        for &layer in layers {
            consumers.push(consume(
                engine,
                component,
                family,
                owner,
                prepared.ready.clone(),
                fields.clone(),
                layer,
            )?);
        }
    }
    Ok((prepared, consumers))
}
