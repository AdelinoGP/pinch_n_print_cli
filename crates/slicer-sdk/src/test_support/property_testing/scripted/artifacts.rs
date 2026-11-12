//! Durable scripted attempt artifacts and concrete replay.
//!
//! An attempt directory holds the exact inputs that existed **before** the
//! engine ran, so an abort, timeout or kill still leaves a complete and
//! replayable record. Nothing here regenerates a seed: the recorded inputs are
//! re-emitted, compared byte for byte, and evaluated with the retained options.

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use slicer_scad::{ScadEngineInfo, ScadEvalOptions, ScadEvalOutput, ScadGeometry, ScadSource};

use super::strategy::rejection_reasons;
use super::{
    emit_scad, evaluate_recipe, last_shrink_completed, RecipeBounds, ScadRecipe, ScriptedCase,
    ScriptedError,
};

/// The compiled scripted generator version recorded in every artifact.
pub(crate) const GENERATOR_VERSION: u64 = 1;

const RECIPE_FILE: &str = "recipe.json";
const BOUNDS_FILE: &str = "recipe-bounds.json";
const SOURCE_FILE: &str = "source.scad";
const OPTIONS_FILE: &str = "options.json";
const ENGINE_FILE: &str = "engine.json";
const STATE_FILE: &str = "state.json";
const GEOMETRY_FILE: &str = "geometry.json";
const DIAGNOSTICS_FILE: &str = "diagnostics.json";
const OUTPUT_FILE: &str = "output.json";

/// Hard cap on any single artifact read during replay.
const MAX_ARTIFACT_BYTES: u64 = 16 * 1024 * 1024;

/// Versioned, local attempt-state record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StateRecord {
    schema_version: u32,
    stage: String,
    generator_version: u64,
    source_digest: String,
    input_digest: String,
    reason: Option<String>,
    shrink_completed: Option<bool>,
}

fn io_error(context: &str, error: io::Error) -> ScriptedError {
    ScriptedError::Artifact(format!("{context}: {error}"))
}

fn encode_error(context: &str, error: serde_json::Error) -> ScriptedError {
    ScriptedError::Artifact(format!("{context}: {error}"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// Unambiguous input framing: a version tag, then each fixed input as a
/// filename / byte-length / content record, then the generator version.
fn input_digest(inputs: &[(&str, &[u8])], generator_version: u64) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"pnp-scripted-inputs-v1\n");
    for (name, bytes) in inputs {
        hasher.update(name.as_bytes());
        hasher.update(b"\n");
        hasher.update(bytes.len().to_string().as_bytes());
        hasher.update(b"\n");
        hasher.update(bytes);
        hasher.update(b"\n");
    }
    hasher.update(b"generator_version\n");
    hasher.update(generator_version.to_string().as_bytes());
    hasher.update(b"\n");
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// Effective generator version: the active `PropertyRun`'s value when one is
/// present on this thread, otherwise the compiled constant.
fn effective_generator_version() -> u64 {
    super::strategy::attempt_generator_version()
        .filter(|version| *version > 0)
        .unwrap_or(GENERATOR_VERSION)
}

fn engine_identity() -> Result<ScadEngineInfo, ScriptedError> {
    slicer_scad::engine_info().ok_or_else(|| {
        ScriptedError::Artifact(
            "no engine identity is compiled in; scripted artifacts need the openrscad feature"
                .into(),
        )
    })
}

/// Atomically replace `directory/name` with `bytes`.
fn write_atomic(directory: &Path, name: &str, bytes: &[u8]) -> Result<(), ScriptedError> {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let temporary = directory.join(format!(
        "{name}.tmp-{}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = File::create(&temporary)
        .map_err(|error| io_error(&format!("create {}", temporary.display()), error))?;
    file.write_all(bytes)
        .and_then(|()| file.flush())
        .map_err(|error| io_error(&format!("write {}", temporary.display()), error))?;
    drop(file);
    fs::rename(&temporary, directory.join(name))
        .map_err(|error| io_error(&format!("replace {name}"), error))
}

fn encode_json<T: Serialize>(context: &str, value: &T) -> Result<Vec<u8>, ScriptedError> {
    serde_json::to_vec_pretty(value).map_err(|error| encode_error(context, error))
}

/// The five pre-execution inputs plus the state record, framed for the digest.
#[derive(Debug)]
struct Inputs {
    recipe: Vec<u8>,
    bounds: Vec<u8>,
    source: Vec<u8>,
    options: Vec<u8>,
    engine: Vec<u8>,
    digest: String,
    source_digest: String,
}

fn encode_inputs(
    recipe: &ScadRecipe,
    bounds: &RecipeBounds,
    options: &ScadEvalOptions,
    source_text: &str,
) -> Result<Inputs, ScriptedError> {
    let recipe_bytes = encode_json("recipe", recipe)?;
    let bounds_bytes = encode_json("recipe bounds", bounds)?;
    let options_bytes = encode_json("options", options)?;
    let engine_bytes = encode_json("engine", &engine_identity()?)?;
    let source_bytes = source_text.as_bytes().to_vec();
    let version = effective_generator_version();
    let digest = input_digest(
        &[
            (RECIPE_FILE, &recipe_bytes),
            (BOUNDS_FILE, &bounds_bytes),
            (SOURCE_FILE, &source_bytes),
            (OPTIONS_FILE, &options_bytes),
            (ENGINE_FILE, &engine_bytes),
        ],
        version,
    );
    Ok(Inputs {
        recipe: recipe_bytes,
        bounds: bounds_bytes,
        source: source_bytes,
        options: options_bytes,
        engine: engine_bytes,
        digest,
        source_digest: sha256_hex(source_text.as_bytes()),
    })
}

fn state_bytes(
    stage: &str,
    inputs: &Inputs,
    reason: Option<&str>,
    shrink_completed: Option<bool>,
) -> Result<Vec<u8>, ScriptedError> {
    encode_json(
        "state",
        &StateRecord {
            schema_version: 1,
            stage: stage.to_string(),
            generator_version: effective_generator_version(),
            source_digest: inputs.source_digest.clone(),
            input_digest: inputs.digest.clone(),
            reason: reason.map(str::to_owned),
            shrink_completed,
        },
    )
}

fn write_prepared(directory: &Path, inputs: &Inputs) -> Result<(), ScriptedError> {
    fs::create_dir_all(directory)
        .map_err(|error| io_error(&format!("create {}", directory.display()), error))?;
    write_atomic(directory, RECIPE_FILE, &inputs.recipe)?;
    write_atomic(directory, BOUNDS_FILE, &inputs.bounds)?;
    // The source is saved after the recipe, before any execution, so an
    // emission failure still leaves the recipe and an honest failure state.
    write_atomic(directory, SOURCE_FILE, &inputs.source)?;
    write_atomic(directory, OPTIONS_FILE, &inputs.options)?;
    write_atomic(directory, ENGINE_FILE, &inputs.engine)?;
    write_atomic(
        directory,
        STATE_FILE,
        &state_bytes("prepared", inputs, None, None)?,
    )
}

/// A prepared attempt directory that a caller can complete or fail honestly.
#[derive(Debug)]
pub(crate) struct Attempt {
    directory: PathBuf,
    inputs: Inputs,
    recipe: ScadRecipe,
    bounds: RecipeBounds,
    options: ScadEvalOptions,
}

impl Attempt {
    /// Record the emitted source, promoting the attempt to `prepared`.
    pub(crate) fn record_source(&mut self, source: &ScadSource) -> Result<(), ScriptedError> {
        let inputs = encode_inputs(&self.recipe, &self.bounds, &self.options, &source.text)?;
        write_atomic(&self.directory, SOURCE_FILE, &inputs.source)?;
        write_atomic(
            &self.directory,
            STATE_FILE,
            &state_bytes("prepared", &inputs, None, None)?,
        )?;
        self.inputs = inputs;
        Ok(())
    }

    /// Record that the adapter produced checked geometry.
    pub(crate) fn finish(mut self, output: &ScadEvalOutput) -> Result<(), ScriptedError> {
        write_evaluated(
            &self.directory,
            &mut self.inputs,
            output,
            last_shrink_completed(),
        )
    }

    /// Record a failure stage without claiming a completed evaluation.
    pub(crate) fn fail(self, stage: &str, reason: &str) -> Result<(), ScriptedError> {
        let state = state_bytes(stage, &self.inputs, Some(reason), Some(false))?;
        write_atomic(&self.directory, STATE_FILE, &state)
    }
}

/// Prepare the attempt directory for one candidate when a registry context
/// exists. Direct library use has no attempt root and returns `None`.
///
/// The directory is written before emission and before any adapter call, and it
/// starts in the honest `failed` stage: an emission that never completes leaves
/// that record rather than a falsely complete `prepared` state.
pub(crate) fn prepare_attempt(
    recipe: &ScadRecipe,
    bounds: &RecipeBounds,
    options: &ScadEvalOptions,
) -> Result<Option<Attempt>, ScriptedError> {
    let Some(root) = super::strategy::attempt_root() else {
        return Ok(None);
    };
    let role = if std::env::var_os("_PROPTEST_FORKFILE").is_some() {
        "worker"
    } else {
        "controller"
    };
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let directory = root
        .join("attempts")
        .join(format!(
            "{role}-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
    let inputs = encode_inputs(recipe, bounds, options, "")?;
    fs::create_dir_all(&directory)
        .map_err(|error| io_error(&format!("create {}", directory.display()), error))?;
    write_atomic(&directory, RECIPE_FILE, &inputs.recipe)?;
    write_atomic(&directory, BOUNDS_FILE, &inputs.bounds)?;
    // An empty source exists as a file so the digest framing is unambiguous
    // even when emission never produces text.
    write_atomic(&directory, SOURCE_FILE, &inputs.source)?;
    write_atomic(&directory, OPTIONS_FILE, &inputs.options)?;
    write_atomic(&directory, ENGINE_FILE, &inputs.engine)?;
    write_atomic(
        &directory,
        STATE_FILE,
        &state_bytes(
            "failed",
            &inputs,
            Some("emission has not produced a source yet"),
            Some(false),
        )?,
    )?;
    Ok(Some(Attempt {
        directory,
        inputs,
        recipe: recipe.clone(),
        bounds: *bounds,
        options: options.clone(),
    }))
}

fn write_evaluated(
    directory: &Path,
    inputs: &mut Inputs,
    output: &ScadEvalOutput,
    shrink_completed: Option<bool>,
) -> Result<(), ScriptedError> {
    let geometry = match &output.geometry {
        ScadGeometry::Empty => Vec::new(),
        geometry => encode_json("geometry", geometry)?,
    };
    if !geometry.is_empty() {
        write_atomic(directory, GEOMETRY_FILE, &geometry)?;
    }
    write_atomic(
        directory,
        DIAGNOSTICS_FILE,
        &encode_json("diagnostics", &output.diagnostics)?,
    )?;
    write_atomic(directory, OUTPUT_FILE, &encode_json("output", output)?)?;
    write_atomic(
        directory,
        STATE_FILE,
        &state_bytes("evaluated", inputs, None, shrink_completed)?,
    )?;
    // The rejection reasons of this attempt are recorded next to its inputs so
    // a rejection-heavy generation is diagnosable after the fact.
    let reasons = rejection_reasons();
    if !reasons.is_empty() {
        write_atomic(
            directory,
            "reasons.json",
            &encode_json("reasons", &reasons)?,
        )?;
    }
    Ok(())
}

/// Write one complete evaluated case to `directory` using the fixed artifact
/// filenames and the safe atomic/error handling replay relies on.
///
/// # Errors
/// Returns [`ScriptedError::Artifact`] when the directory cannot be created or
/// a file cannot be written, and never leaves a partially complete `prepared`
/// state behind a successful return.
pub fn write_scripted_artifacts(
    directory: &Path,
    case: &ScriptedCase,
) -> Result<(), ScriptedError> {
    let mut inputs = encode_inputs(
        &case.recipe,
        &case.recipe_bounds,
        &case.options,
        &case.source.text,
    )?;
    fs::create_dir_all(directory)
        .map_err(|error| io_error(&format!("create {}", directory.display()), error))?;
    write_prepared(directory, &inputs)?;
    write_evaluated(directory, &mut inputs, &case.output, last_shrink_completed())
}

/// Read one fixed artifact file, rejecting oversize, non-regular and escaping
/// paths before loading any bytes.
fn read_checked(directory: &Path, name: &str) -> Result<Vec<u8>, ScriptedError> {
    let path = directory.join(name);
    let link = fs::symlink_metadata(&path)
        .map_err(|error| io_error(&format!("inspect {name}"), error))?;
    if link.file_type().is_symlink() {
        return Err(ScriptedError::Artifact(format!(
            "refusing to read {name}: symlink artifact"
        )));
    }
    if !link.file_type().is_file() {
        return Err(ScriptedError::Artifact(format!(
            "refusing to read {name}: not a regular file"
        )));
    }
    if link.len() > MAX_ARTIFACT_BYTES {
        return Err(ScriptedError::Artifact(format!(
            "refusing to read {name}: {} bytes exceeds the artifact bound",
            link.len()
        )));
    }
    let canonical = fs::canonicalize(&path)
        .map_err(|error| io_error(&format!("canonicalize {name}"), error))?;
    if !canonical.starts_with(directory) {
        return Err(ScriptedError::Artifact(format!(
            "refusing to read {name}: path escapes the replay directory"
        )));
    }
    let mut file = File::open(&canonical).map_err(|error| io_error(&format!("open {name}"), error))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|error| io_error(&format!("read {name}"), error))?;
    Ok(bytes)
}

fn parse_json<T: for<'de> Deserialize<'de>>(
    name: &str,
    bytes: &[u8],
    context: &str,
) -> Result<T, ScriptedError> {
    serde_json::from_slice(bytes).map_err(|error| {
        ScriptedError::Artifact(format!("{context} in {name}: {error}"))
    })
}

/// Read one retained attempt directory, verify its provenance, and evaluate the
/// retained recipe with the retained options and bounds.
///
/// # Errors
/// Returns [`ScriptedError::Artifact`] for missing, damaged or unsafe files,
/// [`ScriptedError::ReplayMismatch`] when the recorded source, inputs, generator
/// version or engine identity no longer agree with this build, and
/// [`ScriptedError::Adapter`] when the re-evaluation itself fails.
pub fn replay_scripted_case(directory: &Path) -> Result<ScriptedCase, ScriptedError> {
    let directory = fs::canonicalize(directory).map_err(|error| {
        io_error(
            &format!("canonicalize {}", directory.display()),
            error,
        )
    })?;
    if !directory.is_dir() {
        return Err(ScriptedError::Artifact(format!(
            "{} is not a directory",
            directory.display()
        )));
    }

    let recipe_bytes = read_checked(&directory, RECIPE_FILE)?;
    let bounds_bytes = read_checked(&directory, BOUNDS_FILE)?;
    let source_bytes = read_checked(&directory, SOURCE_FILE)?;
    let options_bytes = read_checked(&directory, OPTIONS_FILE)?;
    let engine_bytes = read_checked(&directory, ENGINE_FILE)?;
    let state: StateRecord =
        parse_json(STATE_FILE, &read_checked(&directory, STATE_FILE)?, "state")?;

    if state.schema_version != 1 {
        return Err(ScriptedError::Artifact(format!(
            "unsupported state schema_version {}",
            state.schema_version
        )));
    }
    if !matches!(state.stage.as_str(), "prepared" | "evaluated") {
        return Err(ScriptedError::Artifact(format!(
            "attempt stage {:?} did not retain a complete evaluation",
            state.stage
        )));
    }
    let version = effective_generator_version();
    if state.generator_version != version {
        return Err(ScriptedError::ReplayMismatch(format!(
            "recorded generator version {} does not match {}",
            state.generator_version, version
        )));
    }

    let recorded_source_digest = sha256_hex(&source_bytes);
    if state.source_digest != recorded_source_digest {
        return Err(ScriptedError::ReplayMismatch(
            "recorded source_digest does not cover the retained source bytes".into(),
        ));
    }
    let recorded_input_digest = input_digest(
        &[
            (RECIPE_FILE, &recipe_bytes),
            (BOUNDS_FILE, &bounds_bytes),
            (SOURCE_FILE, &source_bytes),
            (OPTIONS_FILE, &options_bytes),
            (ENGINE_FILE, &engine_bytes),
        ],
        state.generator_version,
    );
    if state.input_digest != recorded_input_digest {
        return Err(ScriptedError::ReplayMismatch(
            "recorded input_digest does not cover the retained inputs".into(),
        ));
    }

    let recipe: ScadRecipe = parse_json(RECIPE_FILE, &recipe_bytes, "recipe")?;
    let bounds: RecipeBounds = parse_json(BOUNDS_FILE, &bounds_bytes, "recipe bounds")?;
    let options: ScadEvalOptions = parse_json(OPTIONS_FILE, &options_bytes, "options")?;
    let recorded_engine: ScadEngineInfo = parse_json(ENGINE_FILE, &engine_bytes, "engine")?;

    let active_engine = engine_identity()?;
    if recorded_engine != active_engine {
        return Err(ScriptedError::ReplayMismatch(format!(
            "recorded engine {recorded_engine:?} differs from the active {active_engine:?}"
        )));
    }

    let reemitted = emit_scad(&recipe, &bounds)?;
    if reemitted.text.as_bytes() != source_bytes.as_slice() {
        return Err(ScriptedError::ReplayMismatch(
            "re-emitted source differs from the retained source bytes".into(),
        ));
    }

    let output = evaluate_recipe(&recipe, &options, &bounds)?;
    Ok(ScriptedCase {
        recipe,
        recipe_bounds: bounds,
        source: reemitted,
        options,
        output,
    })
}

