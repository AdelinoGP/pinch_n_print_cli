//! Real preparation components and the same fixture sources compiled natively.

use super::preparation_pilot_host as driver;
use driver::infill::slicer::layer_preparation::{plan_access, preparation_types as cap};
use preparation_pilot_infill::PreparationPilotInfill;
use slicer_ir::{ConfigView, ExtrusionPath3D, ExtrusionRole};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::native::{NativeLayerRequest, NativeStageEntry};
use slicer_sdk::preparation::{
    with_prepared_plan, PreparationError, PreparedPieceCallbacks, PreparedPlanCallbacks,
    PreparedPlanOutput, PrintPreparationInput,
};
use slicer_wasm_host::host::ConfigValueStorage;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use std::sync::Arc;
use wasmtime::component::Component;
use wasmtime::component::Resource;
use wasmtime::Engine;

fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}

fn process_output(mut command: std::process::Command, label: &str) -> (bool, String) {
    let output = command
        .output()
        .unwrap_or_else(|error| panic!("{label}: cannot start {command:?}: {error}"));
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let log_dir = workspace_root().join("target/preparation-pilot/command-logs");
    std::fs::create_dir_all(&log_dir).unwrap();
    let name: String = label
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    std::fs::write(log_dir.join(format!("{name}.log")), &combined).unwrap();
    eprintln!("{label}: {}", output.status);
    (output.status.success(), combined)
}

fn successful_process(command: std::process::Command, label: &str) -> String {
    let (success, output) = process_output(command, label);
    assert!(success, "{label}: subprocess failed:\n{output}");
    output
}

fn verifier_executable() -> &'static std::path::PathBuf {
    static VERIFIER: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    VERIFIER.get_or_init(|| {
        let root = workspace_root();
        let scratch = root.join("target/preparation-pilot/verifier");
        std::fs::create_dir_all(scratch.join("src")).unwrap();
        let schema = root.join("crates/slicer-schema").to_string_lossy().replace('\\', "/");
        std::fs::write(scratch.join("Cargo.toml"), format!(
            "[package]\nname = \"preparation-pilot-verifier\"\nversion = \"0.0.0\"\nedition = \"2021\"\n[workspace]\n[dependencies]\nslicer-schema = {{ path = {schema:?} }}\nwit-parser = \"0.247\"\n"
        )).unwrap();
        let verifier_source = root.join("xtask/src/wit_verify.rs").to_string_lossy().replace('\\', "/");
        // Compile the production verifier, not a test-local copy of its rules.
        std::fs::write(scratch.join("src/main.rs"), format!(r##"
#[path = {verifier_source:?}]
mod wit_verify;
fn main() {{
    let args: Vec<_> = std::env::args().collect();
    let embedded = wit_verify::embedded_world_model(std::path::Path::new(&args[2])).unwrap();
    let canonical = wit_verify::canonical_world_model_memoizable(std::path::Path::new(&args[1])).unwrap();
    let stage = wit_verify::resolve_stage_from_world(&embedded).unwrap();
    let identity = slicer_schema::PREPARATION_QUALIFIED_EXPORT;
    let package = identity.split('/').next().unwrap();
    let has_capability = embedded.packages.keys().any(|key| key.split('@').next() == Some(package));
    if args[3] == "prepared" && !has_capability {{
        panic!("{{identity}}: missing requested preparation export");
    }}
    let drifts = wit_verify::compare_worlds(&embedded, &canonical, Some(&stage));
    if !drifts.is_empty() {{
        panic!("{{identity}}: artifact drift: {{drifts:?}}");
    }}
    println!("stage={{}}", stage.stage_id);
}}
"##)).unwrap();
        let mut command = std::process::Command::new("cargo");
        command.args(["build", "--offline", "--manifest-path"]).arg(scratch.join("Cargo.toml"))
            .arg("--target-dir").arg(root.join("target"));
        successful_process(command, "compile production artifact verifier");
        root.join("target/debug").join(format!("preparation-pilot-verifier{}", std::env::consts::EXE_SUFFIX))
    })
}

fn verify_artifact(artifact: &std::path::Path, prepared: bool) -> (bool, String) {
    let mut command = std::process::Command::new(verifier_executable());
    command
        .arg(workspace_root())
        .arg(artifact)
        .arg(if prepared { "prepared" } else { "plain" });
    process_output(
        command,
        &format!(
            "verify {} prepared={prepared}",
            artifact.file_name().unwrap().to_string_lossy()
        ),
    )
}

fn archived_plain() -> std::path::PathBuf {
    const BEFORE: &str = "6499a7d809a1812bc581adab2bc394ca660f209b0f38b9ce459e9c47c43e5c5c";
    let archive = workspace_root().join("target/preparation-pilot/plain-before.component.wasm");
    let record = std::fs::read_to_string(archive.with_file_name("plain-before.sha256"))
        .expect("pre-edit archive hash record is mandatory; never substitute the live guest");
    assert_eq!(record.split_whitespace().next().unwrap(), BEFORE);
    assert!(
        archive.is_file(),
        "pre-edit plain archive is mandatory: {}",
        archive.display()
    );
    let mut command = std::process::Command::new("python");
    command.args(["-c", "import hashlib,pathlib,sys; print(hashlib.sha256(pathlib.Path(sys.argv[1]).read_bytes()).hexdigest())"]).arg(&archive);
    let observed = successful_process(command, "hash archived plain component");
    assert_eq!(
        observed.trim(),
        BEFORE,
        "archived bytes must equal the pre-edit hash"
    );
    archive
}

fn shape_parameters(method: &str) -> &'static str {
    match method {
        "run_slice_postprocess" => "_layer: u32, _regions: &[SliceRegionView], _paint: &PaintRegionLayerView, _output: &mut SlicePostprocessBuilder, _config: &slicer_ir::ConfigView",
        "run_perimeters" => "_layer: u32, _regions: &[SliceRegionView], _paint: &PaintRegionLayerView, _output: &mut PerimeterOutputBuilder, _config: &slicer_ir::ConfigView",
        "run_wall_postprocess" => "_layer: u32, _regions: &[PerimeterRegionView], _output: &mut PerimeterOutputBuilder, _config: &slicer_ir::ConfigView",
        "run_infill" => "_layer: u32, _regions: &[SliceRegionView], _paint: &PaintRegionLayerView, _output: &mut InfillOutputBuilder, _config: &slicer_ir::ConfigView",
        "run_infill_postprocess" => "_layer: u32, _regions: &[PerimeterRegionView], _infill: &[slicer_ir::InfillRegion], _output: &mut InfillOutputBuilder, _config: &slicer_ir::ConfigView",
        "run_support" => "_layer: u32, _regions: &[SliceRegionView], _paint: &PaintRegionLayerView, _output: &mut SupportOutputBuilder, _collection: &mut LayerCollectionBuilder, _config: &slicer_ir::ConfigView",
        "run_support_postprocess" => "_layer: u32, _regions: &[SliceRegionView], _output: &mut SupportOutputBuilder, _config: &slicer_ir::ConfigView",
        "run_anchored_events" => "_layer: u32, _regions: &[SliceRegionView], _output: &mut LayerCollectionBuilder, _config: &slicer_ir::ConfigView",
        "run_path_optimization" => "_layer: u32, _regions: &[PerimeterRegionView], _output: &mut GcodeOutputBuilder, _collection: &mut LayerCollectionBuilder, _config: &slicer_ir::ConfigView",
        other => panic!("new authority-derived Layer method has no compile shape: {other}"),
    }
}

fn write_shape(
    directory: &std::path::Path,
    name: &str,
    method: &str,
    prepared: bool,
    implement_preparation: bool,
) {
    std::fs::create_dir_all(directory.join("src")).unwrap();
    let root = workspace_root();
    let sdk = root
        .join("crates/slicer-sdk")
        .to_string_lossy()
        .replace('\\', "/");
    let schema = root
        .join("crates/slicer-schema")
        .to_string_lossy()
        .replace('\\', "/");
    let ir = root
        .join("crates/slicer-ir")
        .to_string_lossy()
        .replace('\\', "/");
    std::fs::write(directory.join("Cargo.toml"), format!(
        "[package]\nname = {name:?}\nversion = \"0.0.0\"\nedition = \"2021\"\n[lib]\ncrate-type = [\"rlib\", \"cdylib\"]\n[dependencies]\nslicer-sdk = {{ path = {sdk:?} }}\nslicer-ir = {{ path = {ir:?} }}\nslicer-schema = {{ path = {schema:?} }}\n[target.'cfg(target_arch = \"wasm32\")'.dependencies]\nwit-bindgen = \"0.60.0\"\n"
    )).unwrap();
    let attribute = if prepared {
        "#[slicer_module(preparation)]"
    } else {
        "#[slicer_module]"
    };
    let preparation = if implement_preparation {
        r#"
impl slicer_sdk::preparation::LayerPreparation for Shape {
    fn prepare_print(input: &slicer_sdk::preparation::PrintPreparationInput, output: &mut slicer_sdk::preparation::PreparedPlanOutput) -> Result<(), slicer_sdk::error::ModuleError> {
        let bytes = input.read_fixture(0, 0)?;
        output.put("compile_shape", &bytes)?;
        Ok(())
    }
}
"#
    } else {
        ""
    };
    let parameters = shape_parameters(method);
    let native_entry = if prepared {
        "pub fn native_entry() -> slicer_sdk::native::NativePreparedStageEntry { Shape::__slicer_prepared_native_entry() }"
    } else {
        "pub fn native_entry() -> slicer_sdk::native::NativeStageEntry { Shape::__slicer_native_entry() }"
    };
    std::fs::write(directory.join("src/lib.rs"), format!(r#"
#![allow(unused_imports)]
use slicer_sdk::*;
use slicer_sdk::builders::*;
use slicer_sdk::views::*;
use slicer_sdk::traits::*;
use slicer_sdk::layer_collection_builder::LayerCollectionBuilder;
use slicer_sdk::postpass_builders::GcodeOutputBuilder;
pub struct Shape;
{preparation}
{attribute}
impl LayerModule for Shape {{
    fn from_config(_: &slicer_ir::ConfigView) -> Result<Self, slicer_sdk::error::ModuleError> {{ Ok(Self) }}
    fn {method}(&self, {parameters}) -> Result<(), slicer_sdk::error::ModuleError> {{ Ok(()) }}
}}
#[cfg(not(target_arch = "wasm32"))]
{native_entry}
"#)).unwrap();
}

#[test]
fn preparation_pilot_stage_surface() {
    use std::collections::BTreeSet;
    let started = std::time::Instant::now();
    let rows: Vec<_> = slicer_schema::STAGES
        .iter()
        .filter(|row| row.tier_id == slicer_schema::TIER_LAYER && !row.wit_package.is_empty())
        .collect();
    assert!(!rows.is_empty(), "Layer stage authority must not be empty");
    eprintln!("authority-derived Layer rows={}", rows.len());
    let root = workspace_root();
    exercise_prepared_path(&root);
    let scratch = root.join("target/preparation-pilot/matrix");
    std::fs::create_dir_all(&scratch).unwrap();
    let mut members = Vec::new();
    let mut expected = BTreeSet::new();
    for row in &rows {
        for form in ["plain", "prepared"] {
            let name = format!("pilot-{}-{form}", row.wit_dir);
            write_shape(
                &scratch.join(&name),
                &name,
                row.method,
                form == "prepared",
                form == "prepared",
            );
            members.push(name);
            for target in ["native", "wasm32"] {
                expected.insert((row.stage_id.to_owned(), form.to_owned(), target.to_owned()));
            }
        }
    }
    let negative = "pilot-missing-preparation";
    write_shape(
        &scratch.join(negative),
        negative,
        rows[0].method,
        true,
        false,
    );
    members.push(negative.into());
    std::fs::write(
        scratch.join("Cargo.toml"),
        format!("[workspace]\nresolver = \"2\"\nmembers = {members:?}\n"),
    )
    .unwrap();
    let mut command = std::process::Command::new("cargo");
    command
        .args(["build", "--offline", "--manifest-path"])
        .arg(scratch.join("Cargo.toml"))
        .args(["--package", negative, "--lib", "--target-dir"])
        .arg(root.join("target"));
    let (success, failure) =
        process_output(command, "opt-in without LayerPreparation compile rejection");
    assert!(
        !success,
        "opt-in without prepare_print must fail compilation"
    );
    assert!(
        failure.contains("LayerPreparation"),
        "wrong negative-control failure: {failure}"
    );
    let mut observed = BTreeSet::new();
    for row in &rows {
        for form in ["plain", "prepared"] {
            let name = format!("pilot-{}-{form}", row.wit_dir);
            for target in ["native", "wasm32"] {
                let label = format!("{} {form} {target}", row.stage_id);
                let mut command = std::process::Command::new("cargo");
                command
                    .args(["build", "--offline", "--manifest-path"])
                    .arg(scratch.join("Cargo.toml"))
                    .args(["--package", &name, "--lib", "--target-dir"])
                    .arg(root.join("target"));
                if target == "wasm32" {
                    command.args(["--target", "wasm32-unknown-unknown", "--release"]);
                }
                successful_process(command, &label);
                if target == "wasm32" {
                    let core = root
                        .join("target/wasm32-unknown-unknown/release")
                        .join(format!("{}.wasm", name.replace('-', "_")));
                    let component_path = scratch.join(format!("{name}.component.wasm"));
                    let mut command = std::process::Command::new("wasm-tools");
                    command
                        .args(["component", "new"])
                        .arg(core)
                        .arg("-o")
                        .arg(&component_path);
                    successful_process(command, &format!("{label} componentization"));
                    let engine = crate::common::wasm_cache::shared_engine()
                        .wasmtime_engine()
                        .clone();
                    Component::from_file(&engine, &component_path)
                        .expect("compiled shape must be a valid real component");
                    let (valid, output) = verify_artifact(&component_path, form == "prepared");
                    assert!(valid, "{label}: artifact verification failed: {output}");
                    assert_eq!(output.trim(), format!("stage={}", row.stage_id));
                }
                observed.insert((row.stage_id.to_owned(), form.to_owned(), target.to_owned()));
            }
        }
    }
    assert_eq!(
        observed, expected,
        "each authority-derived row must compile in all four configurations"
    );
    eprintln!(
        "Layer matrix rows={} configurations={} elapsed={:?}",
        rows.len(),
        observed.len(),
        started.elapsed()
    );
}

fn exercise_prepared_path(root: &std::path::Path) {
    let engine = crate::common::wasm_cache::shared_engine()
        .wasmtime_engine()
        .clone();
    let component = Component::from_file(
        &engine,
        root.join("crates/slicer-wasm-host/test-guests/preparation-pilot-path.component.wasm"),
    )
    .unwrap();
    let prepared = driver::prepare(
        &engine,
        &component,
        driver::PilotFamily::Path,
        "path-pilot",
        Arc::from(b"alpha|beta".as_slice()),
    )
    .unwrap();
    assert!(prepared.failure.is_none(), "{:?}", prepared.failure);
    for (offset, max, expected) in [(0, 5, "alpha"), (6, 4, "beta")] {
        let consumed = driver::consume(
            &engine,
            &component,
            driver::PilotFamily::Path,
            "path-pilot",
            prepared.ready.clone(),
            fields(offset, max, true),
            0,
        )
        .unwrap();
        let commands = &consumed.ordinary.gcode_output().commands;
        assert_eq!(commands.len(), 1);
        assert!(
            matches!(&commands[0], slicer_wasm_host::host::GcodeCommandCollected::Comment(text) if text == expected),
            "actual builder commands: {commands:?}"
        );
    }
    eprintln!("prepared PathOptimization fresh builders: comments alpha / beta");
}

#[test]
fn preparation_pilot_plain_guest() {
    let archive = archived_plain();
    let engine = crate::common::wasm_cache::shared_engine()
        .wasmtime_engine()
        .clone();
    let component = Component::from_file(&engine, &archive).unwrap();
    let linker = driver::pilot_linker(&engine, false).unwrap();
    let mut store = wasmtime::Store::new(
        &engine,
        driver::PilotHostContext::new("archived-plain", None),
    );
    slicer_wasm_host::host::LayerModule::instantiate(&mut store, &component, &linker)
        .expect("unchanged plain artifact must load with preparation imports registered alongside ordinary imports");
    let (valid, output) = verify_artifact(&archive, false);
    assert!(
        valid,
        "capability addition must not drift the archived stage: {output}"
    );
    assert_eq!(output.trim(), "stage=Layer::Perimeters");
    let error = driver::load_prepared_perimeters(&engine, &component).unwrap_err();
    assert!(error
        .to_string()
        .contains(slicer_schema::PREPARATION_QUALIFIED_EXPORT));
}

fn copy_wit_tree(source: &std::path::Path, destination: &std::path::Path) {
    std::fs::create_dir_all(destination).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_wit_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
fn preparation_pilot_export_mismatch() {
    let archive = archived_plain();
    let engine = crate::common::wasm_cache::shared_engine()
        .wasmtime_engine()
        .clone();
    let plain = Component::from_file(&engine, &archive).unwrap();
    let error = driver::load_prepared_perimeters(&engine, &plain).unwrap_err();
    assert!(error
        .to_string()
        .contains(slicer_schema::PREPARATION_QUALIFIED_EXPORT));
    let (valid, output) = verify_artifact(&archive, true);
    assert!(
        !valid,
        "plain artifact requested as prepared must be rejected"
    );
    assert!(output.contains(slicer_schema::PREPARATION_QUALIFIED_EXPORT));
    let root = workspace_root();
    let scratch = root.join("target/preparation-pilot/mutated-signature");
    let wit = scratch.join("wit");
    copy_wit_tree(&root.join("crates/slicer-schema/wit"), &wit);
    let capability = wit.join("deps/layer-preparation/layer-preparation.wit");
    let original = std::fs::read_to_string(&capability).unwrap();
    let signature = "prepare-print: func(input: print-preparation-input, output: prepared-plan-output) -> result<_, module-error>;";
    assert_eq!(
        original.matches(signature).count(),
        1,
        "mutate exactly the capability export signature"
    );
    std::fs::write(
        capability,
        original.replace(
            signature,
            "prepare-print: func(input: u32) -> result<_, module-error>;",
        ),
    )
    .unwrap();
    let core = scratch.join("mutated.core.wasm");
    let mutated = scratch.join("mutated.component.wasm");
    let mut command = std::process::Command::new("wasm-tools");
    command
        .args(["component", "embed"])
        .arg(&wit)
        .args([
            "--world",
            "slicer:layer-preparation/prepared-perimeters-module",
            "--dummy",
            "-o",
        ])
        .arg(&core);
    successful_process(command, "embed incompatible preparation signature");
    let mut command = std::process::Command::new("wasm-tools");
    command
        .args(["component", "new"])
        .arg(core)
        .arg("-o")
        .arg(&mutated);
    successful_process(command, "componentize incompatible preparation signature");
    let component = Component::from_file(&engine, &mutated)
        .expect("mutant is a valid component, not malformed WASM");
    let error = driver::load_prepared_perimeters(&engine, &component).unwrap_err();
    assert!(error
        .to_string()
        .contains(slicer_schema::PREPARATION_QUALIFIED_EXPORT));
    let (valid, output) = verify_artifact(&mutated, true);
    assert!(
        !valid,
        "artifact verifier must reject incompatible preparation signature"
    );
    assert!(output.contains(slicer_schema::PREPARATION_QUALIFIED_EXPORT));
}

fn component() -> (Engine, Component) {
    let engine = crate::common::wasm_cache::shared_engine()
        .wasmtime_engine()
        .clone();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("test-guests/preparation-pilot-infill.component.wasm");
    let component = Component::from_file(&engine, path).expect("real infill pilot component");
    (engine, component)
}

fn fields(offset: i64, max: i64, required: bool) -> HashMap<String, ConfigValueStorage> {
    HashMap::from([
        ("pilot_range_offset".into(), ConfigValueStorage::Int(offset)),
        ("pilot_range_max_bytes".into(), ConfigValueStorage::Int(max)),
        (
            "pilot_require_numbers".into(),
            ConfigValueStorage::Bool(required),
        ),
    ])
}

fn assert_path(paths: &[ExtrusionPath3D], expected_x: [f32; 2]) {
    assert_eq!(paths.len(), 1, "exactly one real builder path");
    let path = &paths[0];
    assert_eq!(path.role, ExtrusionRole::SparseInfill);
    assert_eq!(path.speed_factor, 1.0);
    assert_eq!(path.points.len(), 2);
    assert_eq!([path.points[0].x, path.points[1].x], expected_x);
    for point in &path.points {
        assert_eq!(point.y, 0.0);
        assert_eq!(point.z, 0.2);
        assert_eq!(point.width, 0.4);
        assert_eq!(point.flow_factor, 1.0);
    }
}

fn assert_wasm_path(
    paths: &[slicer_wasm_host::host::layer_perimeters::slicer::types::geometry::ExtrusionPath3d],
    expected_x: [f32; 2],
) {
    use slicer_wasm_host::host::layer_perimeters::slicer::types::geometry;
    assert_eq!(paths.len(), 1);
    let path = &paths[0];
    assert!(matches!(path.role, geometry::ExtrusionRole::SparseInfill));
    assert_eq!(path.speed_factor, 1.0);
    assert_eq!(path.points.len(), 2);
    assert_eq!([path.points[0].x, path.points[1].x], expected_x);
    for point in &path.points {
        assert_eq!(point.y, 0.0);
        assert_eq!(point.z, 0.2);
        assert_eq!(point.width, 0.4);
        assert_eq!(point.flow_factor, 1.0);
    }
}

fn native_prepare(bytes: &[u8]) -> Result<BTreeMap<String, Arc<[u8]>>, String> {
    native_prepare_observed(bytes, false).map(|(pieces, _)| pieces)
}

fn native_prepare_observed(
    bytes: &[u8],
    instrumented: bool,
) -> Result<(BTreeMap<String, Arc<[u8]>>, driver::PilotMetadata), String> {
    native_prepare_owned(bytes, instrumented, "A")
}

fn native_prepare_owned(
    bytes: &[u8],
    instrumented: bool,
    owner: &str,
) -> Result<(BTreeMap<String, Arc<[u8]>>, driver::PilotMetadata), String> {
    let mut host = driver::PilotHostContext::new(owner, None);
    host.metadata.set_instrumented(instrumented);
    let input_resource = host.push_preparation_input(Arc::from(bytes), true).unwrap();
    let output_resource = host.push_preparation_output().unwrap();
    let input_index = input_resource.rep();
    let output_index = output_resource.rep();
    let host = Rc::new(RefCell::new(host));
    let reader = host.clone();
    let input = PrintPreparationInput::from_callback(move |offset, max| {
        cap::HostPrintPreparationInput::read_fixture(
            &mut *reader.borrow_mut(),
            Resource::new_borrow(input_index),
            offset,
            max,
        )
        .unwrap()
        .map_err(native_error)
    });
    let staged = host.clone();
    let mut output = PreparedPlanOutput::from_callback(move |name, bytes| {
        cap::HostPreparedPlanOutput::put(
            &mut *staged.borrow_mut(),
            Resource::new_borrow(output_index),
            name.into(),
            bytes.into(),
        )
        .unwrap()
        .map_err(native_error)
    });
    host.borrow_mut().metadata.start_preparation();
    let result =
        (PreparationPilotInfill::__slicer_prepared_native_entry().prepare)(&input, &mut output);
    drop(output);
    drop(input);
    cap::HostPrintPreparationInput::drop(&mut *host.borrow_mut(), input_resource).unwrap();
    cap::HostPreparedPlanOutput::drop(&mut *host.borrow_mut(), output_resource).unwrap();
    if let Err(error) = &result {
        host.borrow_mut().record_preparation_error(error.code);
    }
    let ready = host.borrow_mut().finish(result.is_ok());
    host.borrow_mut().metadata.complete_preparation();
    let metadata = host.borrow().metadata.clone();
    let poisoned = host.borrow().poisoned;
    drop(host);
    result.map_err(|e| format!("owner={}; phase={}; {e:?}", metadata.owner, metadata.phase))?;
    if poisoned {
        return Err("layer preparation: poisoned-output".into());
    }
    Ok((
        Arc::try_unwrap(ready.unwrap().pieces).expect("no native preparation retained"),
        metadata,
    ))
}

fn native_error(error: driver::PreparationError) -> PreparationError {
    match error {
        driver::PreparationError::NotReady => PreparationError::NotReady,
        driver::PreparationError::InvalidName => PreparationError::InvalidName,
        driver::PreparationError::DuplicateName => PreparationError::DuplicateName,
        driver::PreparationError::PoisonedOutput => PreparationError::PoisonedOutput,
        driver::PreparationError::RangeError => PreparationError::RangeError,
        driver::PreparationError::ArithmeticOverflow => PreparationError::ArithmeticOverflow,
        driver::PreparationError::AccessDenied => PreparationError::AccessDenied,
    }
}

fn metadata_contract(value: &toml::Value) -> Result<(), String> {
    let table = value.as_table().ok_or("metadata must be a table")?;
    // Pinned diagnostic contract, not a roster of implementation stages.
    let allowed = [
        "owner",
        "phase",
        "preparation_lifecycle",
        "ready",
        "pieces",
        "payload_bytes",
        "name_utf8_bytes",
        "transfers",
        "prepare_duration_ns",
        "read_duration_ns",
    ];
    if table.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err("non-whitelisted metadata key".into());
    }
    for key in ["owner", "phase", "preparation_lifecycle"] {
        if table.get(key).and_then(toml::Value::as_str).is_none() {
            return Err(format!("{key} must be text"));
        }
    }
    if table.get("ready").and_then(toml::Value::as_bool).is_none() {
        return Err("ready must be boolean".into());
    }
    for key in ["payload_bytes", "name_utf8_bytes"] {
        if !matches!(table.get(key).and_then(toml::Value::as_integer), Some(0..)) {
            return Err(format!("{key} must be a nonnegative scalar"));
        }
    }
    for piece in table
        .get("pieces")
        .and_then(toml::Value::as_array)
        .ok_or("pieces must be metadata records")?
    {
        let piece = piece.as_table().ok_or("piece must be a table")?;
        if piece.len() != 2
            || piece.get("name").and_then(toml::Value::as_str).is_none()
            || !matches!(
                piece.get("length").and_then(toml::Value::as_integer),
                Some(0..)
            )
        {
            return Err("piece permits only name and scalar length".into());
        }
    }
    for transfer in table
        .get("transfers")
        .and_then(toml::Value::as_array)
        .ok_or("transfers must be metadata records")?
    {
        let transfer = transfer.as_table().ok_or("transfer must be a table")?;
        if transfer.len() != 3
            || !matches!(
                transfer.get("direction").and_then(toml::Value::as_str),
                Some("host_to_module_input" | "module_to_host_put" | "host_to_module_read")
            )
            || !matches!(
                transfer.get("bytes").and_then(toml::Value::as_integer),
                Some(0..)
            )
            || transfer.get("count").and_then(toml::Value::as_integer) != Some(1)
        {
            return Err("transfer permits only direction, scalar bytes and count".into());
        }
    }
    if table
        .get("prepare_duration_ns")
        .is_some_and(|value| !matches!(value.as_integer(), Some(0..)))
    {
        return Err("preparation duration must be an observed scalar".into());
    }
    if let Some(reads) = table.get("read_duration_ns") {
        let reads = reads
            .as_array()
            .ok_or("read durations must be observed scalars")?;
        if reads.is_empty()
            || reads
                .iter()
                .any(|value| !matches!(value.as_integer(), Some(0..)))
        {
            return Err("read duration entries must exist and be scalars".into());
        }
    }
    Ok(())
}

fn contains_opaque_fixture(value: &toml::Value) -> bool {
    const FIXTURE: [u8; 4] = [3, 5, 11, 17];
    match value {
        toml::Value::String(text) => text
            .as_bytes()
            .windows(FIXTURE.len())
            .any(|bytes| bytes == FIXTURE),
        toml::Value::Array(values) => {
            values.windows(FIXTURE.len()).any(|window| {
                window
                    .iter()
                    .zip(FIXTURE)
                    .all(|(value, byte)| value.as_integer() == Some(i64::from(byte)))
            }) || values.iter().any(contains_opaque_fixture)
        }
        toml::Value::Table(table) => table.values().any(contains_opaque_fixture),
        _ => false,
    }
}

fn assert_payload_free(metadata: &driver::PilotMetadata) -> toml::Value {
    const FIXTURE: [u8; 4] = [3, 5, 11, 17];
    let projection = metadata.to_toml();
    metadata_contract(&projection).unwrap();
    let serialized = toml::to_string(projection.as_table().unwrap()).unwrap();
    assert!(!serialized
        .as_bytes()
        .windows(FIXTURE.len())
        .any(|window| window == FIXTURE));
    assert!(!serialized.contains("[3, 5, 11, 17]"));
    assert!(!serialized.replace(' ', "").contains("[3,5,11,17]"));
    let decoded: toml::Value = toml::from_str(&serialized).unwrap();
    metadata_contract(&decoded).unwrap();
    assert!(
        !contains_opaque_fixture(&decoded),
        "escaped strings and numeric arrays cannot conceal opaque payload bytes"
    );
    let mut leaked = decoded.clone();
    leaked.as_table_mut().unwrap().insert(
        "opaque_payload".into(),
        toml::Value::Array(
            FIXTURE
                .iter()
                .map(|byte| toml::Value::Integer(i64::from(*byte)))
                .collect(),
        ),
    );
    assert!(
        metadata_contract(&leaked).is_err(),
        "negative control must reject injected opaque bytes"
    );
    let mut escaped = decoded.clone();
    escaped.as_table_mut().unwrap().insert(
        "owner".into(),
        toml::Value::String(String::from_utf8(FIXTURE.to_vec()).unwrap()),
    );
    let serialized_escape = toml::to_string(escaped.as_table().unwrap()).unwrap();
    let decoded_escape: toml::Value = toml::from_str(&serialized_escape).unwrap();
    assert!(
        contains_opaque_fixture(&decoded_escape),
        "negative control detects payload concealed in a whitelisted string after serialization"
    );
    let mut disguised = decoded;
    disguised.as_table_mut().unwrap().insert(
        "payload_bytes".into(),
        toml::Value::Array(
            FIXTURE
                .iter()
                .map(|byte| toml::Value::Integer(i64::from(*byte)))
                .collect(),
        ),
    );
    assert!(
        metadata_contract(&disguised).is_err(),
        "whitelisted accounting keys cannot smuggle payload arrays"
    );
    projection
}

fn transfers(metadata: &driver::PilotMetadata) -> Vec<(&'static str, u64, u64)> {
    metadata
        .transfers
        .iter()
        .map(|transfer| (transfer.direction, transfer.bytes, transfer.count))
        .collect()
}

fn transfer_totals<'a>(
    events: impl Iterator<Item = &'a driver::TransferMetadata>,
    direction: &str,
) -> (u64, u64) {
    events.filter(|event| event.direction == direction).fold(
        (0_u64, 0_u64),
        |(count, bytes), event| {
            (
                count.checked_add(event.count).unwrap(),
                bytes.checked_add(event.bytes).unwrap(),
            )
        },
    )
}

#[test]
fn preparation_pilot_accounting() {
    const NUMBERS: [u8; 4] = [3, 5, 11, 17];
    const EMPTY: [u8; 0] = [];
    const NUMBER_NAME: &[u8] = b"numbers";
    const PI_NAME_UTF8: [u8; 2] = [0xcf, 0x80];
    let expected_payload = u64::try_from(NUMBERS.len() + EMPTY.len()).unwrap();
    let expected_names = u64::try_from(NUMBER_NAME.len() + PI_NAME_UTF8.len()).unwrap();
    assert_eq!(NUMBERS.len(), 4);
    assert_eq!("π".as_bytes(), &PI_NAME_UTF8);
    assert_eq!(PI_NAME_UTF8.len(), 2);
    assert_eq!(expected_payload, 4);
    assert_eq!(expected_names, 9);
    let (engine, component) = component();
    let mut projections = Vec::new();
    let mut observed_preparation_ns = Vec::new();
    let mut observed_read_ns = Vec::new();
    for native in [false, true] {
        for instrumented in [false, true] {
            let (ready, preparation) = if native {
                let (pieces, metadata) = native_prepare_observed(&NUMBERS, instrumented).unwrap();
                (
                    driver::ReadyPlan {
                        owner: "A".into(),
                        print_id: "pilot-print".into(),
                        pieces: Arc::new(pieces),
                    },
                    metadata,
                )
            } else {
                let prepared = driver::prepare_observed(
                    &engine,
                    &component,
                    driver::PilotFamily::Infill,
                    "A",
                    Arc::from(NUMBERS),
                    true,
                    "pilot-print",
                    instrumented,
                )
                .unwrap();
                assert!(prepared.failure.is_none(), "{:?}", prepared.failure);
                assert!(prepared.store_destroyed);
                (prepared.ready.unwrap(), prepared.metadata)
            };
            assert!(preparation.ready);
            assert_eq!(preparation.preparation_lifecycle, "complete");
            assert_eq!(preparation.payload_bytes, expected_payload);
            assert_eq!(preparation.name_utf8_bytes, expected_names);
            assert_eq!(
                preparation.pieces,
                vec![("numbers".into(), 4), ("π".into(), 0)]
            );
            assert_eq!(
                transfers(&preparation),
                vec![
                    ("host_to_module_input", 4, 1),
                    ("module_to_host_put", 4, 1),
                    ("module_to_host_put", 0, 1)
                ]
            );
            assert_eq!(preparation.prepare_duration_ns.is_some(), instrumented);
            assert!(preparation.read_duration_ns.is_empty());
            if let Some(duration) = preparation.prepare_duration_ns {
                observed_preparation_ns.push(duration);
            }
            let projection = assert_payload_free(&preparation);
            assert_eq!(projection["payload_bytes"].as_integer(), Some(4));
            assert_eq!(projection["name_utf8_bytes"].as_integer(), Some(9));
            assert_eq!(
                projection.get("prepare_duration_ns").is_some(),
                instrumented
            );
            assert!(projection.get("read_duration_ns").is_none());
            projections.push(projection);
            let weak_store = Arc::downgrade(&ready.pieces);
            let weak_payload = Arc::downgrade(&ready.pieces["numbers"]);
            let observations = Rc::new(RefCell::new(Vec::new()));
            let capture = observations.clone();
            let weak_in_flight = weak_store.clone();
            let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                // Move the print's last owner into the consumer frame. Metadata
                // captured outside this frame has no payload-bearing references.
                let ready = ready;
                for (offset, expected_x) in [(0, [3.0, 5.0]), (2, [11.0, 17.0])] {
                    let metadata = if native {
                        let (result, output, metadata) = native_consume_observed(
                            Some(ready.clone()),
                            "A",
                            "pilot-print",
                            offset,
                            2,
                            true,
                            instrumented,
                        );
                        result.unwrap();
                        assert_path(output.sparse_paths(), expected_x);
                        metadata
                    } else {
                        let mut config = fields(offset, 2, true);
                        config.insert(
                            "pilot_host_instrumented".into(),
                            ConfigValueStorage::Bool(instrumented),
                        );
                        let consumed = driver::consume(
                            &engine,
                            &component,
                            driver::PilotFamily::Infill,
                            "A",
                            Some(ready.clone()),
                            config,
                            0,
                        )
                        .unwrap();
                        assert_wasm_path(
                            &consumed.ordinary.infill_output().sparse_paths,
                            expected_x,
                        );
                        consumed.metadata.clone()
                    };
                    capture.borrow_mut().push(metadata);
                }
                assert!(
                    weak_in_flight.upgrade().is_some(),
                    "store is retained while consumer frame owns it"
                );
                panic!("controlled pilot consumer unwind");
            }));
            let panic = unwind.unwrap_err();
            assert_eq!(
                panic.downcast_ref::<&str>().copied(),
                Some("controlled pilot consumer unwind")
            );
            assert!(
                weak_store.upgrade().is_none(),
                "print store released after consumer unwinding"
            );
            assert!(
                weak_payload.upgrade().is_none(),
                "opaque payload released after consumer unwinding"
            );
            let observations = observations.borrow();
            assert_eq!(observations.len(), 2);
            for read in observations.iter() {
                assert_eq!(read.payload_bytes, expected_payload);
                assert_eq!(read.name_utf8_bytes, expected_names);
                assert_eq!(transfers(read), vec![("host_to_module_read", 2, 1)]);
                assert!(read.prepare_duration_ns.is_none());
                assert_eq!(read.read_duration_ns.len(), usize::from(instrumented));
                observed_read_ns.extend_from_slice(&read.read_duration_ns);
                let projection = assert_payload_free(read);
                assert_eq!(projection["payload_bytes"].as_integer(), Some(4));
                assert_eq!(projection["name_utf8_bytes"].as_integer(), Some(9));
                assert!(projection.get("prepare_duration_ns").is_none());
                assert_eq!(projection.get("read_duration_ns").is_some(), instrumented);
                projections.push(projection);
            }
            let events: Vec<_> = preparation
                .transfers
                .iter()
                .chain(
                    observations
                        .iter()
                        .flat_map(|metadata| metadata.transfers.iter()),
                )
                .collect();
            let input_total = transfer_totals(events.iter().copied(), "host_to_module_input");
            let put_total = transfer_totals(events.iter().copied(), "module_to_host_put");
            let read_total = transfer_totals(events.iter().copied(), "host_to_module_read");
            eprintln!("ACCOUNTING native={native} instrumented={instrumented} payload_bytes={} name_utf8_bytes={} input_count={} input_bytes={} put_count={} put_bytes={} read_count={} read_bytes={} prepare_duration_ns={:?} read_duration_ns={:?}",
                preparation.payload_bytes, preparation.name_utf8_bytes, input_total.0, input_total.1,
                put_total.0, put_total.1, read_total.0, read_total.1, preparation.prepare_duration_ns,
                observations.iter().flat_map(|metadata| metadata.read_duration_ns.iter()).collect::<Vec<_>>());
            let (empty_ready, empty_metadata) = if native {
                let (pieces, metadata) = native_prepare_observed(&EMPTY, instrumented).unwrap();
                (
                    driver::ReadyPlan {
                        owner: "A".into(),
                        print_id: "pilot-print".into(),
                        pieces: Arc::new(pieces),
                    },
                    metadata,
                )
            } else {
                let prepared = driver::prepare_observed(
                    &engine,
                    &component,
                    driver::PilotFamily::Infill,
                    "A",
                    Arc::from(EMPTY),
                    true,
                    "pilot-print",
                    instrumented,
                )
                .unwrap();
                assert!(prepared.failure.is_none());
                (prepared.ready.unwrap(), prepared.metadata)
            };
            assert!(empty_metadata.ready);
            assert!(empty_metadata.pieces.is_empty());
            assert_eq!(empty_metadata.payload_bytes, 0);
            assert_eq!(empty_metadata.name_utf8_bytes, 0);
            assert_eq!(
                transfers(&empty_metadata),
                vec![("host_to_module_input", 0, 1)]
            );
            assert_eq!(empty_metadata.prepare_duration_ns.is_some(), instrumented);
            if let Some(duration) = empty_metadata.prepare_duration_ns {
                observed_preparation_ns.push(duration);
            }
            let projection = assert_payload_free(&empty_metadata);
            assert_eq!(projection["payload_bytes"].as_integer(), Some(0));
            assert_eq!(projection["name_utf8_bytes"].as_integer(), Some(0));
            assert_eq!(
                projection.get("prepare_duration_ns").is_some(),
                instrumented
            );
            assert!(projection.get("read_duration_ns").is_none());
            projections.push(projection);
            let optional_read = if native {
                let (result, output, metadata) = native_consume_observed(
                    Some(empty_ready),
                    "A",
                    "pilot-print",
                    0,
                    2,
                    false,
                    instrumented,
                );
                result.unwrap();
                assert!(output.sparse_paths().is_empty());
                metadata
            } else {
                let mut config = fields(0, 2, false);
                config.insert(
                    "pilot_host_instrumented".into(),
                    ConfigValueStorage::Bool(instrumented),
                );
                let consumed = driver::consume(
                    &engine,
                    &component,
                    driver::PilotFamily::Infill,
                    "A",
                    Some(empty_ready),
                    config,
                    0,
                )
                .unwrap();
                assert!(consumed.ordinary.infill_output().sparse_paths.is_empty());
                consumed.metadata.clone()
            };
            assert!(optional_read.ready);
            assert!(optional_read.transfers.is_empty());
            assert!(
                optional_read.read_duration_ns.is_empty(),
                "no fabricated read timing for missing optional piece"
            );
            let projection = assert_payload_free(&optional_read);
            assert!(projection.get("prepare_duration_ns").is_none());
            assert!(projection.get("read_duration_ns").is_none());
            projections.push(projection);
            let empty_events: Vec<_> = empty_metadata
                .transfers
                .iter()
                .chain(optional_read.transfers.iter())
                .collect();
            let input_total = transfer_totals(empty_events.iter().copied(), "host_to_module_input");
            let put_total = transfer_totals(empty_events.iter().copied(), "module_to_host_put");
            let read_total = transfer_totals(empty_events.iter().copied(), "host_to_module_read");
            eprintln!("READY_EMPTY native={native} instrumented={instrumented} payload_bytes={} name_utf8_bytes={} input_count={} input_bytes={} put_count={} read_count={} prepare_duration_ns={:?}",
                empty_metadata.payload_bytes, empty_metadata.name_utf8_bytes, input_total.0, input_total.1, put_total.0, read_total.0, empty_metadata.prepare_duration_ns);
        }
    }
    assert_eq!(observed_preparation_ns.len(), 4);
    assert_eq!(observed_read_ns.len(), 4);
    let evidence = toml::Table::from_iter([
        (
            "allocator_process_overhead".into(),
            toml::Value::String("UNMEASURED".into()),
        ),
        (
            "process_peak_memory".into(),
            toml::Value::String("UNMEASURED".into()),
        ),
        ("observations".into(), toml::Value::Array(projections)),
    ]);
    let serialized = toml::to_string(&evidence).unwrap();
    assert_eq!(
        evidence["allocator_process_overhead"].as_str(),
        Some("UNMEASURED")
    );
    assert_eq!(evidence["process_peak_memory"].as_str(), Some("UNMEASURED"));
    assert!(!serialized.contains("threshold"));
    eprintln!("MEASURED prepare_duration_ns_range={}..{} read_duration_ns_range={}..{} allocator_process_overhead=UNMEASURED process_peak_memory=UNMEASURED",
        observed_preparation_ns.iter().min().unwrap(), observed_preparation_ns.iter().max().unwrap(),
        observed_read_ns.iter().min().unwrap(), observed_read_ns.iter().max().unwrap());
}

fn native_consume(
    pieces: Option<BTreeMap<String, Arc<[u8]>>>,
    offset: i64,
    max: i64,
    required: bool,
) -> (Result<(), String>, InfillOutputBuilder) {
    let ready = pieces.map(|pieces| driver::ReadyPlan {
        owner: "A".into(),
        print_id: "pilot-print".into(),
        pieces: Arc::new(pieces),
    });
    native_consume_scoped(ready, "A", "pilot-print", offset, max, required)
}

fn native_consume_scoped(
    ready: Option<driver::ReadyPlan>,
    owner: &str,
    print_id: &str,
    offset: i64,
    max: i64,
    required: bool,
) -> (Result<(), String>, InfillOutputBuilder) {
    let (result, output, _) =
        native_consume_observed(ready, owner, print_id, offset, max, required, false);
    (result, output)
}

fn native_consume_observed(
    ready: Option<driver::ReadyPlan>,
    owner: &str,
    print_id: &str,
    offset: i64,
    max: i64,
    required: bool,
    instrumented: bool,
) -> (
    Result<(), String>,
    InfillOutputBuilder,
    driver::PilotMetadata,
) {
    let config = ConfigView::from_map(HashMap::from([
        (
            "pilot_range_offset".into(),
            slicer_ir::ConfigValue::Int(offset),
        ),
        (
            "pilot_range_max_bytes".into(),
            slicer_ir::ConfigValue::Int(max),
        ),
        (
            "pilot_require_numbers".into(),
            slicer_ir::ConfigValue::Bool(required),
        ),
    ]));
    let mut host = driver::PilotHostContext::new(owner, ready);
    host.bind_print(print_id);
    host.metadata.phase = "layer";
    host.metadata.set_instrumented(instrumented);
    let host = Rc::new(RefCell::new(host));
    let retained_host = host.clone();
    let result = with_prepared_plan(
        move || {
            let view = plan_access::Host::current(&mut *host.borrow_mut())
                .unwrap()
                .map_err(|error| match error {
                    // A foreign owner's current plan is unavailable to the
                    // native facade, but remains present in the host control.
                    driver::PreparationError::AccessDenied => PreparationError::NotReady,
                    other => native_error(other),
                })?;
            let view_index = view.rep();
            let host = host.clone();
            Ok(PreparedPlanCallbacks::new(move |name| {
                let piece = cap::HostPreparedPlanView::open(
                    &mut *host.borrow_mut(),
                    Resource::new_borrow(view_index),
                    name.into(),
                )
                .unwrap()
                .map_err(native_error)?;
                let Some(piece) = piece else {
                    return Ok(None);
                };
                let piece_index = piece.rep();
                let length_host = host.clone();
                let read_host = host.clone();
                Ok(Some(PreparedPieceCallbacks::new(
                    move || {
                        cap::HostPreparedPiece::len(
                            &mut *length_host.borrow_mut(),
                            Resource::new_borrow(piece_index),
                        )
                        .unwrap()
                    },
                    move |offset, max| {
                        cap::HostPreparedPiece::read(
                            &mut *read_host.borrow_mut(),
                            Resource::new_borrow(piece_index),
                            offset,
                            max,
                        )
                        .unwrap()
                        .map_err(native_error)
                    },
                )))
            }))
        },
        || {
            let NativeStageEntry::Layer(dispatch) =
                PreparationPilotInfill::__slicer_prepared_native_entry().stage
            else {
                panic!("infill must have a generated native Layer dispatch")
            };
            // exhaustive: generated infill dispatch request fixes all stage inputs
            dispatch(&NativeLayerRequest {
                layer_index: 0,
                regions: Vec::new(),
                perimeter_regions: None,
                paint: Some(slicer_sdk::PaintRegionLayerView::new(0)),
                prior_infill: None,
                config,
                stage_export: "Layer::Infill",
                ordered_entities: Vec::new(),
            })
        },
    );
    let metadata = retained_host.borrow().metadata.clone();
    drop(retained_host);
    match result {
        Ok(response) => (
            Ok(()),
            response.infill.expect("generated native infill builder"),
            metadata,
        ),
        Err(error) => (
            Err(format!("{error:?}")),
            InfillOutputBuilder::new(),
            metadata,
        ),
    }
}

#[test]
fn preparation_pilot_fresh_wasm() {
    let (engine, component) = component();
    let prepared = driver::prepare(
        &engine,
        &component,
        driver::PilotFamily::Infill,
        "A",
        Arc::from([3, 5, 11, 17]),
    )
    .unwrap();
    assert!(prepared.failure.is_none(), "{:?}", prepared.failure);
    assert!(
        prepared.store_destroyed,
        "preparation Store must be destroyed before Layer"
    );
    let ready = prepared.ready.unwrap();
    assert_eq!(
        Arc::strong_count(&ready.pieces),
        1,
        "preparation owns no retained plan"
    );
    assert_eq!(ready.pieces["numbers"].as_ref(), &[3, 5, 11, 17]);
    for (offset, expected) in [(0, [3.0, 5.0]), (2, [11.0, 17.0])] {
        let consumed = driver::consume(
            &engine,
            &component,
            driver::PilotFamily::Infill,
            "A",
            Some(ready.clone()),
            fields(offset, 2, true),
            0,
        )
        .unwrap();
        assert_wasm_path(&consumed.ordinary.infill_output().sparse_paths, expected);
        assert!(
            consumed
                .table
                .get(&wasmtime::component::Resource::<driver::PreparationInputData>::new_borrow(0))
                .is_err(),
            "preparation input table index cannot survive in a fresh Layer table"
        );
    }
}

#[test]
fn preparation_pilot_native() {
    let pieces = native_prepare(&[3, 5, 11, 17]).unwrap();
    for (offset, expected) in [(0, [3.0, 5.0]), (2, [11.0, 17.0])] {
        let (result, output) = native_consume(Some(pieces.clone()), offset, 2, true);
        result.unwrap();
        assert_path(output.sparse_paths(), expected);
    }
    let numbers = pieces["numbers"].clone();
    let escaped_piece = with_prepared_plan(
        move || {
            let numbers = numbers.clone();
            Ok(PreparedPlanCallbacks::new(move |_| {
                let numbers = numbers.clone();
                Ok(Some(PreparedPieceCallbacks::new(
                    || 4,
                    move |offset, max| {
                        driver::bounded_read(&numbers, offset, max).map_err(native_error)
                    },
                )))
            }))
        },
        || {
            slicer_sdk::preparation::PreparedPlanView::current()
                .unwrap()
                .open("numbers")
                .unwrap()
                .unwrap()
        },
    );
    assert_eq!(
        escaped_piece.read(0, 2).unwrap_err(),
        PreparationError::NotReady,
        "no native resource may outlive its call-local adapter scope"
    );
    preparation_pilot_fresh_wasm();
}

#[test]
fn preparation_pilot_isolation() {
    let (engine, component) = component();
    let first_print = driver::prepare_in_print(
        &engine,
        &component,
        driver::PilotFamily::Infill,
        "A",
        Arc::from([3, 5, 11, 17]),
        true,
        "print-one",
    )
    .unwrap()
    .ready;
    let (other_print, failure) = driver::consume_in_print(
        &engine,
        &component,
        driver::PilotFamily::Infill,
        "A",
        first_print.clone(),
        fields(0, 2, true),
        0,
        "print-two",
    )
    .unwrap();
    assert!(failure.unwrap().contains("not-ready"));
    assert!(other_print.ordinary.infill_output().sparse_paths.is_empty());
    let (failure, output) = native_consume_scoped(first_print, "A", "print-two", 0, 2, true);
    assert!(failure.unwrap_err().contains("not-ready"));
    assert!(output.sparse_paths().is_empty());
    let owner_a = driver::prepare(
        &engine,
        &component,
        driver::PilotFamily::Infill,
        "A",
        Arc::from([3, 5]),
    )
    .unwrap()
    .ready;
    let (wrong_owner, failure) = driver::consume_outcome(
        &engine,
        &component,
        driver::PilotFamily::Infill,
        "B",
        owner_a.clone(),
        fields(0, 2, true),
        0,
    )
    .unwrap();
    assert!(failure.unwrap().contains("access-denied"));
    assert!(wrong_owner.ordinary.infill_output().sparse_paths.is_empty());
    let mut native_host = driver::PilotHostContext::new("B", owner_a.clone());
    assert!(
        native_host.ready.is_some(),
        "foreign plan must reach the host"
    );
    assert!(
        matches!(
            plan_access::Host::current(&mut native_host).unwrap(),
            Err(driver::PreparationError::AccessDenied)
        ),
        "current itself must reject the foreign owner before open can mask it"
    );
    let (failure, output) = native_consume_scoped(owner_a, "B", "pilot-print", 0, 2, true);
    assert!(failure.unwrap_err().contains("not-ready"));
    assert!(output.sparse_paths().is_empty());
    for (owner, bytes, expected) in [("A", [3, 5], [3.0, 5.0]), ("B", [11, 17], [11.0, 17.0])] {
        let prepared = driver::prepare(
            &engine,
            &component,
            driver::PilotFamily::Infill,
            owner,
            Arc::from(bytes),
        )
        .unwrap();
        let consumed = driver::consume(
            &engine,
            &component,
            driver::PilotFamily::Infill,
            owner,
            prepared.ready,
            fields(0, 2, true),
            0,
        )
        .unwrap();
        assert_wasm_path(&consumed.ordinary.infill_output().sparse_paths, expected);
        let ready = driver::ReadyPlan {
            owner: owner.into(),
            print_id: "pilot-print".into(),
            pieces: Arc::new(native_prepare_owned(&bytes, false, owner).unwrap().0),
        };
        let (result, output) = native_consume_scoped(Some(ready), owner, "pilot-print", 0, 2, true);
        result.unwrap();
        assert_path(output.sparse_paths(), expected);
    }
    let ready = driver::prepare(
        &engine,
        &component,
        driver::PilotFamily::Infill,
        "A",
        Arc::from([]),
    )
    .unwrap()
    .ready;
    let optional = driver::consume(
        &engine,
        &component,
        driver::PilotFamily::Infill,
        "A",
        ready.clone(),
        fields(0, 2, false),
        0,
    )
    .unwrap();
    assert!(optional.ordinary.infill_output().sparse_paths.is_empty());
    let (failed, error) = driver::consume_outcome(
        &engine,
        &component,
        driver::PilotFamily::Infill,
        "A",
        ready,
        fields(0, 2, true),
        0,
    )
    .unwrap();
    assert!(error.unwrap().contains("required piece numbers missing"));
    assert!(failed.ordinary.infill_output().sparse_paths.is_empty());
    let error = driver::consume(
        &engine,
        &component,
        driver::PilotFamily::Infill,
        "A",
        None,
        fields(0, 2, false),
        0,
    )
    .err()
    .unwrap();
    assert!(error.to_string().contains("not-ready"));
    let (result, output) = native_consume(Some(BTreeMap::new()), 0, 2, false);
    result.unwrap();
    assert!(output.sparse_paths().is_empty());
    let (result, output) = native_consume(Some(BTreeMap::new()), 0, 2, true);
    assert!(result
        .unwrap_err()
        .contains("required piece numbers missing"));
    assert!(output.sparse_paths().is_empty());
    let (result, output) = native_consume(None, 0, 2, false);
    assert!(result.unwrap_err().contains("not-ready"));
    assert!(output.sparse_paths().is_empty());
}

#[test]
fn preparation_pilot_publication_failures() {
    let (engine, component) = component();
    // Prove the fixture itself returns genuine success after catching duplicate,
    // independently of the generated completion wrapper exercised below.
    let input = PrintPreparationInput::from_callback(|_, _| Ok(vec![252]));
    let calls = Rc::new(RefCell::new(0));
    let capture = calls.clone();
    let mut output = PreparedPlanOutput::from_callback(move |_, _| {
        *capture.borrow_mut() += 1;
        Ok(())
    });
    <PreparationPilotInfill as slicer_sdk::preparation::LayerPreparation>::prepare_print(
        &input,
        &mut output,
    )
    .unwrap();
    assert_eq!(
        *calls.borrow(),
        1,
        "rejected duplicate never reaches the callback"
    );
    let completion = output.complete(Ok(())).unwrap_err();
    assert_eq!(
        completion.code,
        slicer_sdk::preparation::POISONED_PREPARATION_CODE
    );
    assert!(completion.message.contains("duplicate-name"));
    let mut host = driver::PilotHostContext::new("discriminator-owner", None);
    host.record_preparation_error(completion.code);
    assert!(
        host.poisoned,
        "classification does not inspect the diagnostic message"
    );
    assert!(host.finish(true).is_none());
    for invalid in ["", "bad\nname"] {
        let mut host = driver::PilotHostContext::new("invalid-owner", None);
        let output = host.push_preparation_output().unwrap();
        cap::HostPreparedPlanOutput::put(
            &mut host,
            Resource::new_borrow(output.rep()),
            "numbers".into(),
            vec![3, 5],
        )
        .unwrap()
        .unwrap();
        let error = cap::HostPreparedPlanOutput::put(
            &mut host,
            Resource::new_borrow(output.rep()),
            invalid.into(),
            vec![11, 17],
        )
        .unwrap()
        .unwrap_err();
        assert!(matches!(error, driver::PreparationError::InvalidName));
        assert!(host.poisoned);
        let error = cap::HostPreparedPlanOutput::put(
            &mut host,
            Resource::new_borrow(output.rep()),
            "legitimate".into(),
            vec![11, 17],
        )
        .unwrap()
        .unwrap_err();
        assert!(matches!(error, driver::PreparationError::PoisonedOutput));
        assert!(host.finish(true).is_none());
        assert!(host.staged.is_empty());
    }
    let mut output = PreparedPlanOutput::from_callback(|_, _| Ok(()));
    assert_eq!(
        output.put("", &[3, 5]).unwrap_err(),
        PreparationError::InvalidName
    );
    assert_eq!(
        output.put("numbers", &[11, 17]).unwrap_err(),
        PreparationError::PoisonedOutput
    );
    let mut unexpected = Vec::new();
    for (sentinel, detail) in [
        (255, "duplicate-name"),
        (254, "pilot preparation trap"),
        (253, "pilot preparation failed"),
        (252, "poisoned-output"),
    ] {
        let (prepared, consumers) = driver::prepare_then_consume(
            &engine,
            &component,
            driver::PilotFamily::Infill,
            "failure-owner",
            Arc::from([sentinel]),
            fields(0, 2, true),
            &[0],
        )
        .unwrap();
        assert_eq!(prepared.owner, "failure-owner");
        assert_eq!(prepared.phase, "preparation");
        assert!(prepared.store_destroyed);
        if sentinel == 253 {
            let error = prepared.module_error.as_ref().unwrap();
            assert_eq!(error.code, 1);
            assert_eq!(error.message, "pilot preparation failed");
            assert!(
                !error.fatal,
                "non-fatal preparation error still blocks its dependency"
            );
        }
        let failure = prepared.failure.unwrap();
        if sentinel == 252 {
            assert!(
                failure.contains("duplicate-name"),
                "original error must survive glue: {failure}"
            );
            assert!(failure.contains("owner=failure-owner; phase=preparation"));
        }
        assert!(prepared.ready.is_none());
        assert!(
            consumers.is_empty(),
            "no Layer execution after preparation failure"
        );
        if !failure.contains(detail) {
            unexpected.push(format!(
                "sentinel={sentinel}: expected {detail}; observed {failure}"
            ));
        }
        if matches!(sentinel, 255 | 252) && !prepared.poisoned {
            unexpected.push(format!(
                "sentinel={sentinel}: duplicate error did not poison host staging"
            ));
        }
        if sentinel != 254 {
            let error = native_prepare(&[sentinel]).unwrap_err();
            assert!(
                error.contains(detail),
                "native sentinel={sentinel}: {error}"
            );
        } else {
            let panic = std::panic::catch_unwind(|| native_prepare(&[254])).unwrap_err();
            assert_eq!(
                panic.downcast_ref::<&str>().copied(),
                Some("pilot preparation trap")
            );
        }
    }
    assert!(unexpected.is_empty(), "{}", unexpected.join("\n"));
}

struct MixedFailurePreparation;

impl slicer_sdk::preparation::LayerPreparation for MixedFailurePreparation {
    fn prepare_print(
        _: &PrintPreparationInput,
        output: &mut PreparedPlanOutput,
    ) -> Result<(), slicer_sdk::error::ModuleError> {
        output.put("numbers", &[3, 5])?;
        assert_eq!(
            output.put("numbers", &[11, 17]),
            Err(PreparationError::DuplicateName)
        );
        Err(slicer_sdk::error::ModuleError::non_fatal(
            731,
            "distinct mixed preparation failure",
        ))
    }
}

#[slicer_sdk::slicer_module(preparation)]
impl slicer_sdk::traits::LayerModule for MixedFailurePreparation {
    fn from_config(_: &ConfigView) -> Result<Self, slicer_sdk::error::ModuleError> {
        Ok(Self)
    }

    fn run_infill(
        &self,
        _: u32,
        _: &[slicer_sdk::views::SliceRegionView],
        _: &slicer_sdk::PaintRegionLayerView,
        _: &mut InfillOutputBuilder,
        _: &ConfigView,
    ) -> Result<(), slicer_sdk::error::ModuleError> {
        Ok(())
    }
}

#[test]
fn preparation_pilot_poison_preserves_original_non_fatal_error() {
    let host = Rc::new(RefCell::new(driver::PilotHostContext::new(
        "mixed-owner",
        None,
    )));
    let resource = host.borrow_mut().push_preparation_output().unwrap();
    let index = resource.rep();
    let staged = host.clone();
    let mut output = PreparedPlanOutput::from_callback(move |name, bytes| {
        cap::HostPreparedPlanOutput::put(
            &mut *staged.borrow_mut(),
            Resource::new_borrow(index),
            name.into(),
            bytes.into(),
        )
        .unwrap()
        .map_err(native_error)
    });
    let input = PrintPreparationInput::from_callback(|_, _| Ok(Vec::new()));
    host.borrow_mut().metadata.start_preparation();
    let result =
        (MixedFailurePreparation::__slicer_prepared_native_entry().prepare)(&input, &mut output);
    assert_eq!(host.borrow().staged["numbers"].as_ref(), &[3, 5]);
    let error = result.as_ref().unwrap_err();
    assert_eq!(error.code, 731);
    assert!(!error.fatal);
    assert_eq!(error.message, "distinct mixed preparation failure");
    assert_eq!(host.borrow().metadata.owner, "mixed-owner");
    assert_eq!(host.borrow().metadata.phase, "preparation");
    assert!(host.borrow_mut().finish(result.is_ok()).is_none());
    assert!(host.borrow().staged.is_empty());
    assert!(!host.borrow().metadata.ready);
    drop(output);
    cap::HostPreparedPlanOutput::drop(&mut *host.borrow_mut(), resource).unwrap();

    // Cross the real component boundary as well as the generated native entry.
    let root = workspace_root();
    let scratch = root.join("target/preparation-pilot/mixed-failure");
    write_shape(&scratch, "pilot-mixed-failure", "run_infill", true, true);
    let manifest = scratch.join("Cargo.toml");
    let manifest_text = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(&manifest, format!("{manifest_text}\n[workspace]\n")).unwrap();
    let source = scratch.join("src/lib.rs");
    let original = std::fs::read_to_string(&source).unwrap();
    let body = "let bytes = input.read_fixture(0, 0)?;\n        output.put(\"compile_shape\", &bytes)?;\n        Ok(())";
    assert_eq!(original.matches(body).count(), 1);
    std::fs::write(&source, original.replace(body, r#"
        let _ = input;
        output.put("numbers", &[3, 5])?;
        assert_eq!(output.put("numbers", &[11, 17]), Err(slicer_sdk::preparation::PreparationError::DuplicateName));
        Err(slicer_sdk::error::ModuleError::non_fatal(731, "distinct mixed preparation failure"))
    "#)).unwrap();
    let mut command = std::process::Command::new("cargo");
    command
        .args(["build", "--offline", "--manifest-path"])
        .arg(manifest)
        .args([
            "--target",
            "wasm32-unknown-unknown",
            "--release",
            "--target-dir",
        ])
        .arg(root.join("target"));
    successful_process(command, "compile mixed preparation failure component");
    let component_path = scratch.join("mixed-failure.component.wasm");
    let mut command = std::process::Command::new("wasm-tools");
    command
        .args(["component", "new"])
        .arg(root.join("target/wasm32-unknown-unknown/release/pilot_mixed_failure.wasm"))
        .arg("-o")
        .arg(&component_path);
    successful_process(command, "componentize mixed preparation failure");
    let engine = crate::common::wasm_cache::shared_engine()
        .wasmtime_engine()
        .clone();
    let component = Component::from_file(&engine, component_path).unwrap();
    let prepared = driver::prepare(
        &engine,
        &component,
        driver::PilotFamily::Infill,
        "mixed-owner",
        Arc::from([]),
    )
    .unwrap();
    let error = prepared.module_error.unwrap();
    assert_eq!(error.code, 731);
    assert!(!error.fatal);
    assert_eq!(error.message, "distinct mixed preparation failure");
    assert_eq!(prepared.owner, "mixed-owner");
    assert_eq!(prepared.phase, "preparation");
    assert!(prepared
        .failure
        .unwrap()
        .contains("owner=mixed-owner; phase=preparation"));
    assert!(prepared.ready.is_none());
    assert!(!prepared.metadata.ready);
    assert!(prepared.metadata.pieces.is_empty());
    assert!(prepared.store_destroyed);
}

#[test]
fn preparation_pilot_ranges() {
    let (engine, component) = component();
    let ready = driver::prepare(
        &engine,
        &component,
        driver::PilotFamily::Infill,
        "max-offset",
        Arc::from([3, 5, 11, 17]),
    )
    .unwrap()
    .ready;
    let mut probe_fields = fields(0, 2, true);
    probe_fields.insert(
        "pilot_max_offset_probe".into(),
        ConfigValueStorage::Bool(true),
    );
    let (probe, failure) = driver::consume_outcome(
        &engine,
        &component,
        driver::PilotFamily::Infill,
        "max-offset",
        ready,
        probe_fields,
        0,
    )
    .unwrap();
    let failure = failure.unwrap();
    assert!(failure.contains("range-error"), "{failure}");
    assert!(probe.ordinary.infill_output().sparse_paths.is_empty());
    assert_eq!(probe.piece_reads.len(), 1);
    assert_eq!(
        (probe.piece_reads[0].0, probe.piece_reads[0].1),
        (u64::MAX, 1)
    );
    assert!(matches!(
        probe.piece_reads[0].2,
        Err(driver::PreparationError::RangeError)
    ));
    // These independent expectations exercise unmodified arguments through
    // the real WIT import, not PreparedPiece::read's SDK guards/truncation.
    for (offset, max, expected) in [
        (4, 8, Ok(vec![])),
        (5, 1, Err(driver::PreparationError::RangeError)),
        (3, 8, Ok(vec![17])),
        (0, 0, Ok(vec![])),
    ] {
        let ready = driver::prepare(
            &engine,
            &component,
            driver::PilotFamily::Infill,
            "raw-ranges",
            Arc::from([3, 5, 11, 17]),
        )
        .unwrap()
        .ready;
        let mut raw_fields = fields(offset, max, true);
        raw_fields.insert(
            "pilot_raw_read_probe".into(),
            ConfigValueStorage::Bool(true),
        );
        let (consumed, failure) = driver::consume_outcome(
            &engine,
            &component,
            driver::PilotFamily::Infill,
            "raw-ranges",
            ready,
            raw_fields,
            0,
        )
        .unwrap();
        assert_eq!(
            consumed.piece_reads,
            vec![(
                u64::try_from(offset).unwrap(),
                u64::try_from(max).unwrap(),
                expected.clone(),
            )]
        );
        match expected {
            Ok(_) => assert!(failure.is_none(), "{failure:?}"),
            Err(driver::PreparationError::RangeError) => {
                let failure = failure.unwrap();
                assert!(failure.contains("range-error"), "{failure}");
            }
            Err(_) => unreachable!(),
        }
        assert!(consumed.ordinary.infill_output().sparse_paths.is_empty());
    }
    let denied = driver::prepare_with_permission(
        &engine,
        &component,
        driver::PilotFamily::Infill,
        "denied",
        Arc::from([3, 5, 11, 17]),
        false,
    )
    .unwrap();
    assert!(denied.ready.is_none());
    let denied_error = denied.module_error.unwrap();
    assert_eq!(denied_error.message, "layer preparation: access-denied");
    let input = PrintPreparationInput::from_callback(|_, _| Err(PreparationError::AccessDenied));
    assert_eq!(
        input.read_fixture(0, 4).unwrap_err(),
        PreparationError::AccessDenied
    );
    for (offset, max, detail) in [
        (4, 8, "expected two endpoints"),
        (5, 1, "range-error"),
        (3, 8, "expected two endpoints"),
        (0, 0, "expected two endpoints"),
    ] {
        let ready = driver::prepare(
            &engine,
            &component,
            driver::PilotFamily::Infill,
            "ranges",
            Arc::from([3, 5, 11, 17]),
        )
        .unwrap()
        .ready;
        let (consumed, failure) = driver::consume_outcome(
            &engine,
            &component,
            driver::PilotFamily::Infill,
            "ranges",
            ready,
            fields(offset, max, true),
            0,
        )
        .unwrap();
        let failure = failure.unwrap();
        assert!(failure.contains(detail), "{failure}");
        assert!(consumed.ordinary.infill_output().sparse_paths.is_empty());
        match offset {
            4 | 0 => {
                assert_eq!(consumed.piece_reads.len(), 1);
                assert_eq!(
                    consumed.piece_reads[0].2.as_ref().unwrap(),
                    &Vec::<u8>::new()
                );
            }
            3 => {
                assert_eq!(consumed.piece_reads.len(), 1);
                assert_eq!(consumed.piece_reads[0].2.as_ref().unwrap(), &vec![17]);
            }
            5 => assert!(
                consumed.piece_reads.is_empty(),
                "SDK rejects past-EOF before import"
            ),
            _ => unreachable!(),
        }
        let (result, output) = native_consume(
            Some(native_prepare(&[3, 5, 11, 17]).unwrap()),
            offset,
            max,
            true,
        );
        assert!(result.unwrap_err().contains(detail));
        assert!(output.sparse_paths().is_empty());
    }
    let pieces = BTreeMap::from([("numbers".into(), Arc::<[u8]>::from([252, 5]))]);
    let ready = driver::ReadyPlan {
        owner: "malformed".into(),
        print_id: "pilot-print".into(),
        pieces: Arc::new(pieces.clone()),
    };
    let (failed, failure) = driver::consume_outcome(
        &engine,
        &component,
        driver::PilotFamily::Infill,
        "malformed",
        Some(ready),
        fields(0, 2, true),
        0,
    )
    .unwrap();
    assert!(failure.unwrap().contains("malformed pilot numbers"));
    assert!(failed.ordinary.infill_output().sparse_paths.is_empty());
    let (result, output) = native_consume(Some(pieces), 0, 2, true);
    assert!(result.unwrap_err().contains("malformed pilot numbers"));
    assert!(output.sparse_paths().is_empty());
    let ready = driver::prepare(
        &engine,
        &component,
        driver::PilotFamily::Infill,
        "accounting",
        Arc::from([3, 5, 11, 17]),
    )
    .unwrap()
    .ready;
    let mut accounting_fields = fields(0, 2, true);
    accounting_fields.insert(
        "pilot_host_accounting_overflow".into(),
        ConfigValueStorage::Bool(true),
    );
    let (failed, failure) = driver::consume_outcome(
        &engine,
        &component,
        driver::PilotFamily::Infill,
        "accounting",
        ready,
        accounting_fields,
        0,
    )
    .unwrap();
    assert!(failure.unwrap().contains("arithmetic-overflow"));
    assert!(failed.ordinary.infill_output().sparse_paths.is_empty());
    let accounting = Rc::new(RefCell::new(None));
    let read_accounting = accounting.clone();
    with_prepared_plan(
        move || {
            let read_accounting = read_accounting.clone();
            Ok(PreparedPlanCallbacks::new(move |_| {
                let read_accounting = read_accounting.clone();
                Ok(Some(PreparedPieceCallbacks::new(
                    || 4,
                    move |offset, max| {
                        driver::accounted_read(
                            &[3, 5, 11, 17],
                            offset,
                            max,
                            &mut read_accounting.borrow_mut(),
                        )
                        .map_err(native_error)
                    },
                )))
            }))
        },
        || {
            let piece = slicer_sdk::preparation::PreparedPlanView::current()
                .unwrap()
                .open("numbers")
                .unwrap()
                .unwrap();
            assert_eq!(piece.read(4, 8).unwrap(), Vec::<u8>::new());
            assert_eq!(piece.read(5, 1).unwrap_err(), PreparationError::RangeError);
            assert_eq!(piece.read(3, 8).unwrap(), vec![17]);
            assert_eq!(piece.read(0, 0).unwrap(), Vec::<u8>::new());
            assert_eq!(piece.read(1, u64::MAX).unwrap(), vec![5, 11, 17]);
            *accounting.borrow_mut() = Some(u64::MAX);
            assert_eq!(
                piece.read(1, u64::MAX).unwrap_err(),
                PreparationError::ArithmeticOverflow
            );
            assert_eq!(
                piece.read(u64::MAX, 1).unwrap_err(),
                PreparationError::RangeError,
                "offset beyond EOF must be range-error, not accounting overflow"
            );
        },
    );
}

#[test]
fn preparation_pilot_interleaved_fresh_consumers() {
    let (engine, component) = component();
    let ready = driver::prepare(
        &engine,
        &component,
        driver::PilotFamily::Infill,
        "interleaved",
        Arc::from([3, 5, 11, 17]),
    )
    .unwrap()
    .ready
    .unwrap();
    let bytes = ready.pieces["numbers"].clone();
    // Interleave on one thread: native wrappers stay live across fresh guest
    // stores, all bound to the same immutable published allocation.
    with_prepared_plan(
        move || {
            let bytes = bytes.clone();
            Ok(PreparedPlanCallbacks::new(move |_| {
                let bytes = bytes.clone();
                Ok(Some(PreparedPieceCallbacks::new(
                    || 4,
                    move |offset, max| {
                        driver::bounded_read(&bytes, offset, max).map_err(native_error)
                    },
                )))
            }))
        },
        || {
            let first = slicer_sdk::preparation::PreparedPlanView::current()
                .unwrap()
                .open("numbers")
                .unwrap()
                .unwrap();
            let second = slicer_sdk::preparation::PreparedPlanView::current()
                .unwrap()
                .open("numbers")
                .unwrap()
                .unwrap();
            assert_eq!(first.read(0, 2).unwrap(), vec![3, 5]);
            let guest = driver::consume(
                &engine,
                &component,
                driver::PilotFamily::Infill,
                "interleaved",
                Some(ready.clone()),
                fields(2, 2, true),
                0,
            )
            .unwrap();
            assert_wasm_path(&guest.ordinary.infill_output().sparse_paths, [11.0, 17.0]);
            assert_eq!(second.read(2, 2).unwrap(), vec![11, 17]);
            assert_eq!(first.read(0, 2).unwrap(), vec![3, 5]);
            let guest = driver::consume(
                &engine,
                &component,
                driver::PilotFamily::Infill,
                "interleaved",
                Some(ready.clone()),
                fields(0, 2, true),
                0,
            )
            .unwrap();
            assert_wasm_path(&guest.ordinary.infill_output().sparse_paths, [3.0, 5.0]);
            assert_eq!(second.read(2, 2).unwrap(), vec![11, 17]);
        },
    );
}
