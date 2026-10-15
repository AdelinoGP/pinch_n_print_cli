//! Shared preparation contract driver using production adapters and resources.
#![allow(dead_code)]

use std::path::PathBuf;

use slicer_schema::preparation::PreparationDeclaration;
use slicer_wasm_host::{
    host::{HostExecutionContext, HostExecutionContextBuilder},
    preparation::{
        freeze_native, freeze_wasm, FrozenPreparationBinding, NativePreparationArtifact,
        PreparationFailure, PreparationInputData, PreparationOwner, PreparedHostState,
        PreparedOwnerStore, PreparedPieceHandle, PreparedPlanHandle, StagedPreparation,
    },
};
use wasmtime::{
    component::{Component, HasSelf, Linker},
    Engine,
};

pub fn required_artifact(relative: &str) -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest
        .parent()
        .and_then(|dir| dir.parent())
        .expect("preparation transport driver: manifest has no workspace root");
    let path = root.join(relative);
    assert!(path.is_file(),
        "preparation transport driver: required artifact missing: {} (no skip or replacement permitted)",
        path.display());
    path
}

pub fn fixture_path() -> PathBuf {
    required_artifact(
        "crates/slicer-wasm-host/test-guests/preparation-transport-path.component.wasm",
    )
}

pub fn old_artifact_path() -> PathBuf {
    required_artifact("target/preparation-pilot/plain-before.component.wasm")
}

pub fn engine() -> Engine {
    crate::common::wasm_cache::shared_engine()
        .wasmtime_engine()
        .clone()
}

pub fn fixture_component(engine: &Engine) -> Component {
    Component::from_file(engine, fixture_path())
        .expect("preparation transport driver: cannot load staged transport fixture")
}

#[cfg(feature = "preparation-test-fixtures")]
pub fn fixture_component_text() -> String {
    let output = std::process::Command::new("wasm-tools")
        .arg("print")
        .arg(fixture_path())
        .output()
        .expect("print preparation fixture with the freshness gate's wasm-tools");
    assert!(
        output.status.success(),
        "print preparation fixture: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("component text is UTF-8")
}

#[cfg(feature = "preparation-test-fixtures")]
pub fn component_from_text(engine: &Engine, text: &str) -> Component {
    let bytes = wat::parse_str(text).expect("malformed witness must remain valid component WAT");
    Component::new(engine, bytes).expect("malformed witness must compile before production freeze")
}

/// Preserve all ordinary exports, changing only the resource-provider identity.
#[cfg(feature = "preparation-test-fixtures")]
pub fn wrong_resource_component(engine: &Engine, text: &str) -> Component {
    let canonical = "\n  (import \"slicer:layer-preparation/preparation-types@1.0.0\"";
    assert_eq!(text.matches(canonical).count(), 1);
    component_from_text(
        engine,
        &text.replacen(
            canonical,
            "\n  (import \"slicer:layer-preparation/preparation-typez@1.0.0\"",
            1,
        ),
    )
}

/// Preserve imports and ordinary exports, but export prepare-print: func() -> ().
#[cfg(feature = "preparation-test-fixtures")]
pub fn wrong_signature_component(engine: &Engine, text: &str) -> Component {
    let export = text
        .lines()
        .find(|line| {
            line.starts_with("  (export ")
                && line.contains("\"slicer:layer-preparation/prepare@1.0.0\"")
        })
        .expect("fixture has a top-level preparation capability export");
    assert_eq!(text.matches(export).count(), 1);
    let replacement = r#"
  (core module $wrong-signature-module (func (export "prepare")))
  (core instance $wrong-signature-core (instantiate $wrong-signature-module))
  (alias core export $wrong-signature-core "prepare" (core func $wrong-signature-core-func))
  (type $wrong-signature-type (func))
  (func $wrong-signature-prepare (type $wrong-signature-type) (canon lift (core func $wrong-signature-core-func)))
  (instance $wrong-signature-export (export "prepare-print" (func $wrong-signature-prepare)))
  (export "slicer:layer-preparation/prepare@1.0.0" (instance $wrong-signature-export))"#;
    // Append after existing indexed types: inserting a new type in the middle
    // would shift the printer's numeric references in the ordinary exports.
    let mut witness = text.replacen(export, "", 1);
    let end = witness.rfind(')').expect("component closing delimiter");
    witness.insert_str(end, replacement);
    component_from_text(engine, &witness)
}

pub fn old_component(engine: &Engine) -> Component {
    Component::from_file(engine, old_artifact_path())
        .expect("preparation transport driver: cannot load old provenance artifact")
}

pub fn declaration(input_reads: Vec<String>, views: Vec<String>) -> PreparationDeclaration {
    PreparationDeclaration::new("slicer:layer-preparation/prepare@1.0.0", input_reads, views)
}

/// The caller supplies the actual fixture's compiled pair and export schema.
pub fn native_binding(
    module_id: &str,
    declaration: Option<PreparationDeclaration>,
    artifact: NativePreparationArtifact,
) -> Result<FrozenPreparationBinding, PreparationFailure> {
    freeze_native(module_id.into(), declaration, artifact)
}

pub fn native_schema() -> slicer_schema::PreparationExportSchema {
    slicer_schema::PreparationExportSchema {
        wit_package: "slicer:layer-preparation@1.0.0",
        ordinary_world: "perimeters-module",
        wit_world: "prepared-perimeters-module",
        qualified_export: slicer_schema::PREPARATION_QUALIFIED_EXPORT,
    }
}

pub fn native_ordinary(
    _: &slicer_sdk::native::NativeLayerRequest,
) -> Result<slicer_sdk::native::NativeLayerResponse, slicer_sdk::error::ModuleError> {
    Err(slicer_sdk::error::ModuleError::fatal(
        57403,
        "ordinary entry must not run during freeze",
    ))
}

pub fn native_artifact(
    prepare: fn(
        &slicer_sdk::preparation::PrintPreparationInput,
        &mut slicer_sdk::preparation::PreparedPlanOutput,
    ) -> Result<(), slicer_sdk::error::ModuleError>,
    schema: slicer_schema::PreparationExportSchema,
) -> NativePreparationArtifact {
    NativePreparationArtifact::Prepared {
        entry: slicer_sdk::native::NativePreparedStageEntry {
            stage: slicer_sdk::native::NativeStageEntry::Layer(native_ordinary),
            prepare,
        },
        schema,
    }
}

pub fn path_stage_id() -> &'static str {
    slicer_schema::STAGES
        .iter()
        .find(|stage| stage.wit_world == "path-optimization-module")
        .expect("preparation transport driver: path optimization stage missing")
        .stage_id
}

pub fn wasm_binding(
    module_id: &str,
    declaration: Option<PreparationDeclaration>,
) -> Result<FrozenPreparationBinding, PreparationFailure> {
    let engine = engine();
    let component = fixture_component(&engine);
    freeze_wasm(
        module_id.into(),
        declaration,
        engine,
        component,
        path_stage_id().into(),
    )
}

pub fn owner_store(
    binding: &FrozenPreparationBinding,
) -> Result<(PreparedOwnerStore, PreparationOwner), PreparationFailure> {
    let store = PreparedOwnerStore::new();
    let owner = store.create_owner(binding)?;
    Ok((store, owner))
}

#[cfg(feature = "preparation-test-fixtures")]
pub fn fixture_input(
    owner: &PreparationOwner,
    bytes: impl Into<std::sync::Arc<[u8]>>,
) -> PreparationInputData {
    PreparationInputData::controlled_fixture(owner, bytes.into())
}

pub fn prepare(
    binding: &FrozenPreparationBinding,
    owner: &PreparationOwner,
    input: PreparationInputData,
) -> Result<(), PreparationFailure> {
    binding.prepare(owner, input)
}

pub fn prepare_with_publication(
    binding: &FrozenPreparationBinding,
    owner: &PreparationOwner,
    input: PreparationInputData,
    validate: impl FnOnce(&StagedPreparation) -> Result<(), PreparationFailure>,
) -> Result<(), PreparationFailure> {
    binding.prepare_with_publication(owner, input, validate)
}

pub fn declared_views(owner: &PreparationOwner) -> &[String] {
    owner.declared_views()
}

pub fn current(
    store: &PreparedOwnerStore,
    owner: &PreparationOwner,
) -> Result<PreparedPlanHandle, PreparationFailure> {
    store.current(owner)
}

pub fn piece(plan: &PreparedPlanHandle, name: &str) -> PreparedPieceHandle {
    plan.open(name)
        .expect("preparation transport driver: open failed")
        .unwrap_or_else(|| panic!("preparation transport driver: required piece missing: {name}"))
}

pub fn ordinary_context(module_id: &str) -> HostExecutionContext {
    HostExecutionContextBuilder::new(module_id, 0.0, 1.0).build()
}

/// Feature-off ordinary linker, also usable for old-artifact capability checks.
pub fn ordinary_linker(engine: &Engine) -> wasmtime::Result<Linker<HostExecutionContext>> {
    let mut linker = Linker::new(engine);
    slicer_wasm_host::host::layer_path_optimization::PathOptimizationModule::add_to_linker::<
        _,
        HasSelf<HostExecutionContext>,
    >(&mut linker, |ctx| ctx)?;
    Ok(linker)
}

/// Production composed linker; its Host implementations own all transport resources.
pub fn prepared_linker(engine: &Engine) -> wasmtime::Result<Linker<PreparedHostState>> {
    PreparedHostState::linker(engine, path_stage_id())
}

#[cfg(feature = "preparation-test-fixtures")]
#[test]
fn production_store_round_trips_one_piece() {
    assert_eq!(
        fixture_path().file_name().and_then(|name| name.to_str()),
        Some("preparation-transport-path.component.wasm")
    );
    let binding = wasm_binding(
        "transport-driver-smoke",
        Some(declaration(
            vec!["fixture".into()],
            vec!["driver-piece".into()],
        )),
    )
    .expect("freeze real staged component");
    let (store, owner) = owner_store(&binding).expect("create production owner");
    assert_eq!(declared_views(&owner), &["driver-piece".to_owned()]);
    let mut transaction = store.begin(&owner).expect("begin production transaction");
    transaction
        .put("driver-piece", &[7, 11])
        .expect("put production piece");
    transaction
        .finish(Ok(()), |_| Ok(()))
        .expect("publish production transaction");
    let plan = current(&store, &owner).expect("published plan");
    let piece = piece(&plan, "driver-piece");
    assert_eq!(piece.len(), 2);
    assert_eq!(piece.read(0, u64::MAX), Ok(vec![7, 11]));
    assert_eq!(plan.metadata().pieces(), &[("driver-piece".to_owned(), 2)]);
}
