//! Declaration-validated native pairs and explicit owner-bound invocation scopes.

use super::preparation_input::{OwnerPreparationInput, PreparationMeshHelpers};
use super::*;
use slicer_schema::{PreparationExportSchema, STAGES, TIER_LAYER};
use slicer_sdk::native::{NativePreparedStageEntry, NativeStageEntry};
use slicer_sdk::preparation::{
    with_prepared_plan_view, PreparationLayer, PreparationMeshObject, PreparationMeshQuery,
    PreparationMeshQueryResult, PreparationRegion, PreparationRegionConfig, PreparationSeam,
    PreparationSelectedTarget, PreparationSupportAnalysisBatch, PreparationSupportGeometryBatch,
    PreparationSupportPlanBatch, PreparationSurfaceLayer, PreparationSurfaceObject,
    PreparedPieceRead, PreparedPlanOutput, PreparedPlanRead, PrintPreparationInput,
    PrintPreparationRead,
};
use std::sync::Mutex;

/// The ordinary entry and preparation function come from one compiled artifact.
pub enum NativePreparationArtifact {
    /// Ordinary artifact without a preparation capability.
    Plain(NativeStageEntry),
    /// Compiled ordinary/preparation pair with its capability metadata.
    Prepared {
        /// Both function pointers from the selected artifact.
        entry: NativePreparedStageEntry,
        /// Compiled export metadata belonging to the pair.
        schema: PreparationExportSchema,
    },
}

/// Opaque input; production defaults deny every read.
#[derive(Default)]
pub struct PreparationInputData {
    fixture: Option<(PreparationOwner, Arc<[u8]>)>,
    pub(super) committed: Option<Arc<super::preparation_input::OwnerPreparationInput>>,
}

impl PreparationInputData {
    /// Construct production input without granting access to pilot fixture bytes.
    pub fn for_owner(_owner: &PreparationOwner) -> Self {
        Self::default()
    }

    /// Install opaque pilot bytes for exactly this owner, never a provider lookup.
    #[cfg(feature = "preparation-test-fixtures")]
    pub fn controlled_fixture(owner: &PreparationOwner, bytes: Arc<[u8]>) -> Self {
        Self {
            fixture: Some((owner.clone(), bytes)),
            committed: None,
        }
    }

    /// Observe typed DTOs through the production reader with its owner and grants intact.
    #[cfg(feature = "preparation-test-fixtures")]
    #[doc(hidden)]
    pub fn controlled_native_input(
        &self,
        owner: &PreparationOwner,
        binding: &FrozenPreparationBinding,
    ) -> Result<PrintPreparationInput, PreparationFailure> {
        owner.originating_store()?.validate_owner(owner, binding)?;
        Ok(PrintPreparationInput::from_host_input(Arc::new(
            PreparationInputReader::new(owner, self, binding),
        )))
    }

    pub(crate) fn read(
        &self,
        owner: &PreparationOwner,
        offset: u64,
        max_bytes: u64,
    ) -> Result<Vec<u8>, PreparationTransportError> {
        let (fixture_owner, bytes) = self
            .fixture
            .as_ref()
            .ok_or(PreparationTransportError::AccessDenied)?;
        if !owner.same_owner(fixture_owner) {
            return Err(PreparationTransportError::AccessDenied);
        }
        let _guard = owner
            .active_guard()
            .map_err(|_| PreparationTransportError::AccessDenied)?;
        let len = u64::try_from(bytes.len())
            .map_err(|_| PreparationTransportError::ArithmeticOverflow)?;
        if offset > len {
            return Err(PreparationTransportError::RangeError);
        }
        let start =
            usize::try_from(offset).map_err(|_| PreparationTransportError::ArithmeticOverflow)?;
        let take = usize::try_from(max_bytes.min(len - offset))
            .map_err(|_| PreparationTransportError::ArithmeticOverflow)?;
        let end = start
            .checked_add(take)
            .ok_or(PreparationTransportError::ArithmeticOverflow)?;
        Ok(bytes
            .get(start..end)
            .ok_or(PreparationTransportError::RangeError)?
            .to_vec())
    }
}

/// The production native reader and preparation mesh-service guard share one
/// owner/declaration check and the committed backing's canonical projections.
pub(super) struct PreparationInputReader {
    owner: PreparationOwner,
    backing: Option<Arc<OwnerPreparationInput>>,
    input_reads: Vec<String>,
    read_failure: Mutex<Option<PreparationInputError>>,
}

impl PreparationInputReader {
    pub(super) fn new(
        owner: &PreparationOwner,
        input: &PreparationInputData,
        binding: &FrozenPreparationBinding,
    ) -> Self {
        Self {
            owner: owner.clone(),
            backing: input.committed.clone(),
            input_reads: binding
                .declaration()
                .map(|declaration| declaration.input_reads().to_vec())
                .unwrap_or_default(),
            read_failure: Mutex::new(None),
        }
    }

    fn read<T>(
        &self,
        permission: &str,
        project: impl FnOnce(&OwnerPreparationInput) -> Result<T, PreparationInputError>,
    ) -> Result<T, PreparationInputError> {
        let result = (|| {
            let backing = self
                .backing
                .as_deref()
                .ok_or(PreparationInputError::AccessDenied)?;
            // The backing normalizes optional grants and excludes fixture reads.
            // Bind those grants to the invocation's frozen declaration too; do
            // not interpret a second permission vocabulary in this adapter.
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
            *self
                .read_failure
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(error.clone());
        }
        result
    }

    fn failure(&self) -> Option<PreparationInputError> {
        self.read_failure
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

pub(super) const PREPARATION_MESH_HELPERS: PreparationMeshHelpers = PreparationMeshHelpers {
    transform: crate::host::transform_mesh_point,
    bounds: crate::host::object_bounds_against,
    raycast: crate::host::raycast_z_down_against,
    normal: crate::host::surface_normal_at_against,
};

impl PrintPreparationRead for PreparationInputReader {
    fn layer_count(&self) -> Result<u64, PreparationInputError> {
        self.read("layer_plan", OwnerPreparationInput::layer_count)
    }

    fn layers(
        &self,
        offset: u64,
        max_items: u64,
    ) -> Result<Vec<PreparationLayer>, PreparationInputError> {
        self.read("layer_plan", |input| input.layers(offset, max_items))
    }

    fn regions(
        &self,
        layer_index: u32,
        offset: u64,
        max_items: u64,
    ) -> Result<Vec<PreparationRegion>, PreparationInputError> {
        self.read("slices", |input| {
            input.regions(layer_index, offset, max_items)
        })
    }

    fn region_configs(
        &self,
        layer_index: u32,
        offset: u64,
        max_items: u64,
    ) -> Result<Vec<PreparationRegionConfig>, PreparationInputError> {
        self.read("region_map", |input| {
            input.region_configs(layer_index, offset, max_items)
        })
    }

    fn surface_objects(
        &self,
        offset: u64,
        max_items: u64,
    ) -> Result<Option<Vec<PreparationSurfaceObject>>, PreparationInputError> {
        self.read("surface_classification", |input| {
            input.surface_objects(offset, max_items)
        })
    }

    fn surface_layers(
        &self,
        layer_index: u32,
        offset: u64,
        max_items: u64,
    ) -> Result<Option<Vec<PreparationSurfaceLayer>>, PreparationInputError> {
        self.read("surface_classification", |input| {
            input.surface_layers(layer_index, offset, max_items)
        })
    }

    fn mesh_objects(
        &self,
        offset: u64,
        max_items: u64,
    ) -> Result<Vec<PreparationMeshObject>, PreparationInputError> {
        self.read("mesh", |input| {
            input.mesh_objects(offset, max_items, &PREPARATION_MESH_HELPERS)
        })
    }

    fn mesh_vertices(
        &self,
        object_id: &str,
        offset: u64,
        max_items: u64,
    ) -> Result<Vec<slicer_ir::Point3>, PreparationInputError> {
        self.read("mesh", |input| {
            input.mesh_vertices(object_id, offset, max_items, &PREPARATION_MESH_HELPERS)
        })
    }

    fn mesh_triangles(
        &self,
        object_id: &str,
        offset: u64,
        max_items: u64,
    ) -> Result<Vec<(u32, u32, u32)>, PreparationInputError> {
        self.read("mesh", |input| {
            input.mesh_triangles(object_id, offset, max_items)
        })
    }

    fn mesh_queries(
        &self,
        requests: &[PreparationMeshQuery],
    ) -> Result<Vec<PreparationMeshQueryResult>, PreparationInputError> {
        self.read("mesh", |input| {
            input.mesh_queries(requests, &PREPARATION_MESH_HELPERS)
        })
    }

    fn seams(
        &self,
        layer_index: u32,
        offset: u64,
        max_items: u64,
    ) -> Result<Option<Vec<PreparationSeam>>, PreparationInputError> {
        self.read("seam_plan", |input| {
            input.seams(layer_index, offset, max_items)
        })
    }

    fn support_analysis(
        &self,
        offset: u64,
        max_items: u64,
    ) -> Result<Option<PreparationSupportAnalysisBatch>, PreparationInputError> {
        self.read("support_analysis", |input| {
            input.support_analysis(offset, max_items)
        })
    }

    fn support_geometry(
        &self,
        offset: u64,
        max_items: u64,
    ) -> Result<Option<PreparationSupportGeometryBatch>, PreparationInputError> {
        self.read("support_geometry", |input| {
            input.support_geometry(offset, max_items)
        })
    }

    fn support_plan(
        &self,
        offset: u64,
        max_items: u64,
    ) -> Result<Option<PreparationSupportPlanBatch>, PreparationInputError> {
        self.read("support_plan", |input| {
            input.support_plan(offset, max_items)
        })
    }

    fn selected_targets(
        &self,
        layer_index: u32,
        offset: u64,
        max_items: u64,
    ) -> Result<Vec<PreparationSelectedTarget>, PreparationInputError> {
        self.read("selected_targets", |input| {
            input.selected_targets(layer_index, offset, max_items)
        })
    }
}

/// Freeze the paired source after validating both declaration/export directions.
pub fn freeze_native(
    module_id: ModuleId,
    declaration: Option<PreparationDeclaration>,
    artifact: NativePreparationArtifact,
) -> Result<FrozenPreparationBinding, PreparationFailure> {
    use PreparationDeclarationError as D;
    let reject = |error| PreparationFailure::Declaration(error);
    let (stage_id, native, native_plain) = match artifact {
        NativePreparationArtifact::Plain(entry) => {
            if declaration.is_some() {
                return Err(reject(D::MissingExport(module_id)));
            }
            (String::new(), None, Some(entry))
        }
        NativePreparationArtifact::Prepared { entry, schema } => {
            let declared = declaration
                .as_ref()
                .ok_or_else(|| reject(D::UndeclaredExport(module_id.clone())))?;
            if format!("{}#prepare-print", declared.interface()) != schema.qualified_export() {
                return Err(reject(D::IncompatibleExport(module_id)));
            }
            let stage = STAGES
                .iter()
                .find(|stage| stage.wit_world == schema.ordinary_world)
                .filter(|stage| stage.tier_id == TIER_LAYER)
                .filter(|_| matches!(entry.stage, NativeStageEntry::Layer(_)))
                .ok_or_else(|| reject(D::UnsupportedStage(module_id.clone())))?;
            (stage.stage_id.to_owned(), Some(entry), None)
        }
    };
    Ok(FrozenPreparationBinding {
        module_id,
        stage_id,
        artifact_kind: "native".into(),
        artifact_identity: Arc::new(()),
        declaration,
        native,
        native_plain,
        wasm: None,
    })
}

struct NativePlan(PreparationOwner);
impl PreparedPlanRead for NativePlan {
    fn check_ready(&self) -> Result<(), PreparationTransportError> {
        self.0
            .originating_store()
            .and_then(|store| store.current(&self.0))
            .map(|_| ())
            .map_err(|failure| match failure {
                PreparationFailure::Transport(error) => error,
                _ => PreparationTransportError::AccessDenied,
            })
    }

    fn open(
        &self,
        name: &str,
    ) -> Result<Option<Arc<dyn PreparedPieceRead + Send + Sync>>, PreparationTransportError> {
        let store = self
            .0
            .originating_store()
            .map_err(|_| PreparationTransportError::AccessDenied)?;
        let plan = store.current(&self.0).map_err(|failure| match failure {
            PreparationFailure::Transport(error) => error,
            _ => PreparationTransportError::AccessDenied,
        })?;
        plan.open(name).map(|piece| {
            piece.map(|piece| Arc::new(piece) as Arc<dyn PreparedPieceRead + Send + Sync>)
        })
    }
}
impl PreparedPieceRead for PreparedPieceHandle {
    fn len(&self) -> u64 {
        PreparedPieceHandle::len(self)
    }
    fn read(&self, offset: u64, max_bytes: u64) -> Result<Vec<u8>, PreparationTransportError> {
        PreparedPieceHandle::read(self, offset, max_bytes)
    }
}

impl FrozenPreparationBinding {
    /// Prepare and atomically publish without an additional validation hook.
    pub fn prepare(
        &self,
        owner: &PreparationOwner,
        input: PreparationInputData,
    ) -> Result<(), PreparationFailure> {
        self.prepare_with_publication(owner, input, |_| Ok(()))
    }

    /// Invoke preparation and validate immutable staging before atomic publication.
    pub fn prepare_with_publication(
        &self,
        owner: &PreparationOwner,
        input: PreparationInputData,
        validate: impl FnOnce(&StagedPreparation) -> Result<(), PreparationFailure>,
    ) -> Result<(), PreparationFailure> {
        if self.wasm.is_some() {
            return self.prepare_wasm(owner, input, validate);
        }
        let store = owner.originating_store()?;
        store.validate_owner(owner, self)?;
        let _guard = owner.active_guard()?;
        let entry = self.native.ok_or_else(|| {
            PreparationFailure::Declaration(PreparationDeclarationError::MissingExport(
                self.module_id.clone(),
            ))
        })?;
        let transaction = Arc::new(Mutex::new(store.begin(owner)?));
        let result = {
            let reader = Arc::new(PreparationInputReader::new(owner, &input, self));
            let input = if input.fixture.is_some() {
                // Only the feature-gated controlled constructor (or in-module
                // unit fixtures) can install these bytes. A declaration token
                // alone never switches committed production input to this path.
                let reader_owner = owner.clone();
                let fixture_permission = reader.input_reads.iter().any(|read| read == "fixture");
                PrintPreparationInput::from_host_reader(Arc::new(move |offset, max_bytes| {
                    if !fixture_permission {
                        return Err(PreparationTransportError::AccessDenied);
                    }
                    input.read(&reader_owner, offset, max_bytes)
                }))
            } else {
                PrintPreparationInput::from_host_input(reader.clone())
            };
            let writer = Arc::clone(&transaction);
            let mut output = PreparedPlanOutput::from_host_writer(Arc::new(move |name, bytes| {
                writer
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .put(name, bytes)
            }));
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                owner.record_prepare_invocation()?;
                let result = (entry.prepare)(&input, &mut output);
                match result {
                    Ok(()) if output.writer_fault().is_some() => Err(
                        PreparationFailure::Transport(PreparationTransportError::PoisonedOutput),
                    ),
                    Err(error)
                        if output.writer_fault().is_some()
                            && output.synthesized_completion()
                                == Some((error.code, error.message.clone(), error.fatal)) =>
                    {
                        Err(PreparationFailure::Transport(
                            PreparationTransportError::PoisonedOutput,
                        ))
                    }
                    Err(error) => Err(match reader.failure() {
                        Some(error) => PreparationFailure::Input(error),
                        None => PreparationFailure::Module(error),
                    }),
                    Ok(()) => Ok(()),
                }
            }))
            .unwrap_or_else(|panic| {
                Err(PreparationFailure::Trap {
                    detail: panic
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| panic.downcast_ref::<&str>().map(|s| (*s).to_owned()))
                        .unwrap_or_else(|| "native preparation panic".into()),
                })
            });
            if let Some(error) = output.writer_fault() {
                transaction
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .record_writer_fault(error);
            }
            result
        }; // Drop the call-local input and output (including writer Arc) before finish.
        let transaction = Arc::try_unwrap(transaction)
            .unwrap_or_else(|_| unreachable!("call-local writer was dropped"))
            .into_inner()
            .unwrap_or_else(|error| error.into_inner());
        transaction
            .finish(result, |_| validate(&owner.staged_description()?))
            .map(|_| ())
    }

    /// Invoke the paired ordinary entry under a fresh owner-bound read scope.
    pub fn with_native_plan<R>(
        &self,
        owner: &PreparationOwner,
        call: impl FnOnce(NativeStageEntry) -> Result<R, ModuleError>,
    ) -> Result<R, PreparationFailure> {
        let store = owner.originating_store()?;
        store.validate_owner(owner, self)?;
        let _guard = owner.active_guard()?;
        let entry = self
            .native
            .map(|entry| entry.stage)
            .or(self.native_plain)
            .ok_or(PreparationFailure::InvalidOwner)?;
        with_prepared_plan_view(Arc::new(NativePlan(owner.clone())), || call(entry))
            .map_err(PreparationFailure::Module)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_sdk::native::{
        NativeFinalizationRequest, NativeFinalizationResponse, NativeLayerRequest,
        NativeLayerResponse,
    };
    use slicer_sdk::preparation::PreparedPlanView;

    fn ordinary(_: &NativeLayerRequest) -> Result<NativeLayerResponse, ModuleError> {
        Err(ModuleError::fatal(57403, "paired ordinary"))
    }
    fn finalization(
        _: &NativeFinalizationRequest,
    ) -> Result<NativeFinalizationResponse, ModuleError> {
        Err(ModuleError::fatal(57404, "wrong family"))
    }
    fn prepare(
        _: &PrintPreparationInput,
        output: &mut PreparedPlanOutput,
    ) -> Result<(), ModuleError> {
        output.put("view", &[3, 5])?;
        Ok(())
    }
    fn declaration() -> PreparationDeclaration {
        PreparationDeclaration::new(
            "slicer:layer-preparation/prepare@1.0.0",
            vec![],
            vec!["view".into()],
        )
    }
    fn schema() -> PreparationExportSchema {
        PreparationExportSchema {
            wit_package: "slicer:layer-preparation@1.0.0",
            ordinary_world: "perimeters-module",
            wit_world: "prepared-perimeters-module",
            qualified_export: slicer_schema::PREPARATION_QUALIFIED_EXPORT,
        }
    }
    fn artifact() -> NativePreparationArtifact {
        NativePreparationArtifact::Prepared {
            entry: NativePreparedStageEntry {
                stage: NativeStageEntry::Layer(ordinary),
                prepare,
            },
            schema: schema(),
        }
    }
    fn binding() -> FrozenPreparationBinding {
        freeze_native("fixture".into(), Some(declaration()), artifact()).expect("valid pair")
    }
    #[test]
    fn native_current_distinguishes_uncalled_and_ready_empty() {
        let binding = binding();
        let store = PreparedOwnerStore::new();
        let owner = store.create_owner(&binding).expect("owner");
        binding
            .with_native_plan(&owner, |_| {
                assert!(matches!(
                    PreparedPlanView::current(),
                    Err(slicer_sdk::preparation::PreparationError::NotReady)
                ));
                Ok(())
            })
            .expect("uncalled scope");
        store
            .begin(&owner)
            .expect("begin")
            .finish(Ok(()), |_| Ok(()))
            .expect("empty publish");
        binding
            .with_native_plan(&owner, |_| {
                assert!(PreparedPlanView::current()?.open("missing")?.is_none());
                Ok(())
            })
            .expect("ready-empty scope");
    }

    #[test]
    fn freeze_native_pairs_entry_with_schema() {
        let binding = binding();
        assert_eq!(binding.stage_id(), "Layer::Perimeters");
        let store = PreparedOwnerStore::new();
        let owner = store.create_owner(&binding).expect("owner");
        binding
            .prepare(&owner, PreparationInputData::default())
            .expect("prepare");
        assert_eq!(owner.metadata().prepare_invocations(), 1);
        println!("native completed prepare_invocations=1");
        let pointer = binding
            .with_native_plan(&owner, |entry| {
                let NativeStageEntry::Layer(pointer) = entry else {
                    panic!("paired family");
                };
                let piece = PreparedPlanView::current()?.require("view")?;
                assert_eq!(piece.read(0, u64::MAX).expect("piece"), vec![3, 5]);
                assert!(matches!(
                    store.dispose_owner(&owner),
                    Err(PreparationFailure::Busy)
                ));
                Ok(pointer as usize)
            })
            .expect("native scope");
        assert_eq!(pointer, ordinary as *const () as usize);
    }
    #[test]
    fn freeze_native_missing_export() {
        assert!(matches!(
            freeze_native(
                "fixture".into(),
                Some(declaration()),
                NativePreparationArtifact::Plain(NativeStageEntry::Layer(ordinary))
            ),
            Err(PreparationFailure::Declaration(
                PreparationDeclarationError::MissingExport(_)
            ))
        ));
    }
    #[test]
    fn freeze_native_undeclared_export() {
        assert!(matches!(
            freeze_native("fixture".into(), None, artifact()),
            Err(PreparationFailure::Declaration(
                PreparationDeclarationError::UndeclaredExport(_)
            ))
        ));
    }
    #[test]
    fn freeze_native_incompatible_export() {
        let declaration = PreparationDeclaration::new("wrong", vec![], vec![]);
        assert!(matches!(
            freeze_native("fixture".into(), Some(declaration), artifact()),
            Err(PreparationFailure::Declaration(
                PreparationDeclarationError::IncompatibleExport(_)
            ))
        ));
    }
    #[test]
    fn freeze_native_unsupported_stage() {
        for (entry, world) in [
            (
                NativeStageEntry::Finalization(finalization),
                "perimeters-module",
            ),
            (NativeStageEntry::Layer(ordinary), "not-a-layer-world"),
        ] {
            let mut schema = schema();
            schema.ordinary_world = world;
            let artifact = NativePreparationArtifact::Prepared {
                entry: NativePreparedStageEntry {
                    stage: entry,
                    prepare,
                },
                schema,
            };
            assert!(matches!(
                freeze_native("fixture".into(), Some(declaration()), artifact),
                Err(PreparationFailure::Declaration(
                    PreparationDeclarationError::UnsupportedStage(_)
                ))
            ));
        }
    }
    #[test]
    fn caught_writer_faults_are_transport_and_original_failures_win() {
        fn fixture(
            input: &PrintPreparationInput,
            output: &mut PreparedPlanOutput,
        ) -> Result<(), ModuleError> {
            let mode = input.read_fixture(0, 1)?[0];
            output.put("numbers", &[3, 5, 11, 17])?;
            let error = output.put(if mode == 252 { "numbers" } else { "" }, &[]);
            assert!(error.is_err());
            match mode {
                253 => Err(ModuleError::non_fatal(57402, "transport fixture failure")),
                254 => panic!("transport fixture trap"),
                _ => Ok(()),
            }
        }
        for mode in [251, 252, 253, 254] {
            let binding = freeze_native(
                "fixture".into(),
                Some(PreparationDeclaration::new(
                    "slicer:layer-preparation/prepare@1.0.0",
                    vec!["fixture".into()],
                    vec!["numbers".into()],
                )),
                NativePreparationArtifact::Prepared {
                    entry: NativePreparedStageEntry {
                        stage: NativeStageEntry::Layer(ordinary),
                        prepare: fixture,
                    },
                    schema: schema(),
                },
            )
            .expect("pair");
            let store = PreparedOwnerStore::new();
            let owner = store.create_owner(&binding).expect("owner");
            let result = binding.prepare(
                &owner,
                PreparationInputData {
                    fixture: Some((owner.clone(), Arc::from([mode]))),
                    committed: None,
                },
            );
            match (mode, result) {
                (
                    251 | 252,
                    Err(PreparationFailure::Transport(PreparationTransportError::PoisonedOutput)),
                ) => {}
                (253, Err(PreparationFailure::Module(error))) => {
                    assert_eq!(error.code, 57402);
                    assert_eq!(error.message, "transport fixture failure");
                    assert!(!error.fatal);
                }
                (254, Err(PreparationFailure::Trap { detail })) => {
                    assert_eq!(detail, "transport fixture trap")
                }
                (_, other) => panic!("mode {mode}: {other:?}"),
            }
            assert_eq!(
                owner.writer_fault(),
                Some(if mode == 252 {
                    PreparationTransportError::DuplicateName
                } else {
                    PreparationTransportError::InvalidName
                })
            );
            assert!(!owner.metadata().ready());
            assert_eq!(owner.metadata().retained_payload_bytes(), 0);
            assert_eq!(owner.metadata().staged_payload_bytes(), 0);
            assert_eq!(owner.metadata().prepare_invocations(), 1);
            assert!(matches!(
                store.current(&owner),
                Err(PreparationFailure::Transport(
                    PreparationTransportError::NotReady
                ))
            ));
            println!("native mode={mode} classification verified; current=NotReady retained=0");
        }
    }

    #[test]
    fn prepare_preserves_nonfatal_module_error_fields() {
        fn failure(
            _: &PrintPreparationInput,
            output: &mut PreparedPlanOutput,
        ) -> Result<(), ModuleError> {
            output.put("view", &[9])?;
            Err(ModuleError::non_fatal(57402, "transport fixture failure"))
        }
        let artifact = NativePreparationArtifact::Prepared {
            entry: NativePreparedStageEntry {
                stage: NativeStageEntry::Layer(ordinary),
                prepare: failure,
            },
            schema: schema(),
        };
        let binding =
            freeze_native("fixture".into(), Some(declaration()), artifact).expect("freeze");
        let store = PreparedOwnerStore::new();
        let owner = store.create_owner(&binding).expect("owner");
        match binding.prepare(&owner, PreparationInputData::default()) {
            Err(PreparationFailure::Module(error)) => {
                assert_eq!(error.code, 57402);
                assert_eq!(error.message, "transport fixture failure");
                assert!(!error.fatal);
            }
            other => panic!("expected original module failure, got {other:?}"),
        }
        assert!(!owner.metadata().ready());
        assert_eq!(owner.metadata().prepare_invocations(), 1);
        assert_eq!(owner.metadata().retained_payload_bytes(), 0);
        assert!(matches!(
            store.current(&owner),
            Err(PreparationFailure::Transport(
                PreparationTransportError::NotReady
            ))
        ));
    }
    #[test]
    fn with_native_plan_rejects_foreign_owner() {
        let binding = binding();
        let foreign_binding = self::binding();
        let other_store = PreparedOwnerStore::new();
        let foreign_owner = other_store
            .create_owner(&foreign_binding)
            .expect("foreign owner");
        let mut invoked = false;
        assert!(matches!(
            binding.with_native_plan(&foreign_owner, |_| {
                invoked = true;
                Ok(())
            }),
            Err(PreparationFailure::InvalidOwner)
        ));
        assert!(!invoked);
    }
    #[test]
    fn exact_control_character_names_match_sdk_and_store() {
        const NAME: &str = "line\n\tpath/../e\u{301}\0";
        fn prepare(
            _: &PrintPreparationInput,
            output: &mut PreparedPlanOutput,
        ) -> Result<(), ModuleError> {
            output.put(NAME, &[3, 5])?;
            Ok(())
        }
        let binding = freeze_native(
            "names".into(),
            Some(declaration()),
            NativePreparationArtifact::Prepared {
                entry: NativePreparedStageEntry {
                    stage: NativeStageEntry::Layer(ordinary),
                    prepare,
                },
                schema: schema(),
            },
        )
        .expect("pair");
        let store = PreparedOwnerStore::new();
        let sdk_owner = store.create_owner(&binding).expect("SDK owner");
        let raw_owner = store.create_owner(&binding).expect("raw owner");
        binding
            .prepare(&sdk_owner, PreparationInputData::for_owner(&sdk_owner))
            .expect("SDK put");
        let mut transaction = store.begin(&raw_owner).expect("begin");
        transaction.put(NAME, &[3, 5]).expect("raw put");
        let raw_plan = transaction.finish(Ok(()), |_| Ok(())).expect("publish");
        assert_eq!(sdk_owner.metadata().pieces(), raw_owner.metadata().pieces());
        assert_eq!(
            raw_plan
                .open(NAME)
                .expect("raw open")
                .expect("piece")
                .read(0, 2),
            Ok(vec![3, 5])
        );
        binding
            .with_native_plan(&sdk_owner, |_| {
                assert_eq!(
                    PreparedPlanView::current()?.require(NAME)?.read(0, 2)?,
                    vec![3, 5]
                );
                Ok(())
            })
            .expect("SDK open");
    }

    #[test]
    fn input_denies_default_and_wrong_owner_and_clips_fixture() {
        let binding = binding();
        let store = PreparedOwnerStore::new();
        let owner = store.create_owner(&binding).expect("owner");
        let other = store.create_owner(&binding).expect("other");
        assert_eq!(
            PreparationInputData::default().read(&owner, 0, 1),
            Err(PreparationTransportError::AccessDenied)
        );
        let input = PreparationInputData {
            fixture: Some((owner.clone(), Arc::from([3, 5]))),
            committed: None,
        };
        assert_eq!(
            input.read(&other, 0, 1),
            Err(PreparationTransportError::AccessDenied)
        );
        assert_eq!(input.read(&owner, 1, u64::MAX), Ok(vec![5]));
        assert_eq!(
            input.read(&owner, 3, 1),
            Err(PreparationTransportError::RangeError)
        );
    }

    #[test]
    fn prepare_reads_bound_fixture_and_rejects_publication_atomically() {
        fn copy(
            input: &PrintPreparationInput,
            output: &mut PreparedPlanOutput,
        ) -> Result<(), ModuleError> {
            output.put("view", &input.read_fixture(0, u64::MAX)?)?;
            Ok(())
        }
        let binding = freeze_native(
            "fixture".into(),
            Some(PreparationDeclaration::new(
                "slicer:layer-preparation/prepare@1.0.0",
                vec!["fixture".into()],
                vec!["view".into()],
            )),
            NativePreparationArtifact::Prepared {
                entry: NativePreparedStageEntry {
                    stage: NativeStageEntry::Layer(ordinary),
                    prepare: copy,
                },
                schema: schema(),
            },
        )
        .expect("pair");
        let store = PreparedOwnerStore::new();
        let owner = store.create_owner(&binding).expect("owner");
        let fixture = || PreparationInputData {
            fixture: Some((owner.clone(), Arc::from([7, 11]))),
            committed: None,
        };
        assert!(
            matches!(binding.prepare_with_publication(&owner, fixture(), |staged| {
            assert!(staged.owner().same_owner(&owner));
            assert_eq!(staged.pieces(), &[("view".to_owned(), 2)]);
            assert!(!owner.metadata().ready());
            assert!(matches!(store.dispose_owner(&owner), Err(PreparationFailure::Busy)));
            Err(PreparationFailure::PublicationRejected { detail: "veto".into() })
        }), Err(PreparationFailure::PublicationRejected { detail }) if detail == "veto")
        );
        assert!(!owner.metadata().ready());
        binding.prepare(&owner, fixture()).expect("retry publishes");
        assert_eq!(
            store
                .current(&owner)
                .expect("ready")
                .open("view")
                .expect("open")
                .expect("piece")
                .read(0, 2),
            Ok(vec![7, 11])
        );
    }
}
