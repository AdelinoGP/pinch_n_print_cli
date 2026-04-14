//! Python text-postprocess bridge (TASK-104).
//!
//! Implements the documented Python bridge for `PostPass::TextPostProcess`
//! modules (docs/05_module_sdk.md §"Python Bridge (TextPostProcess tier)").
//!
//! A module manifest with a `[python]` section declares a Python script and
//! an entry function. The host invokes the function as
//! `entry(text, config_dict)` and uses the returned string as the
//! replacement G-code text.
//!
//! ### Interpreter backend
//!
//! The interpreter is embedded via [`pyo3`] (workspace-pinned to `0.28.3`,
//! `auto-initialize`). `Py_Initialize` fires on the first call into
//! [`Python::with_gil`]; subsequent calls share the interpreter across the
//! host process. User scripts are loaded with `importlib.util`
//! (`spec_from_file_location` → `module_from_spec` → `exec_module`) and the
//! declared entry is invoked as a normal Python function. The Rust surface
//! (`PythonBinding`, `PythonBridge`, `PythonPostpassRunner`) is unchanged
//! from the previous subprocess-based backend.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use slicer_ir::{ConfigValue, ConfigView, GCodeIR, ModuleId, StageId};

use crate::postpass::{PostpassError, PostpassOutput, PostpassStageRunner};
use crate::{Blackboard, CompiledModule};

/// Manifest-declared Python entry point for a text-postprocess module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonBinding {
    /// Absolute path to the Python script (`[python].script`, resolved
    /// against the manifest directory at load time).
    pub script_path: PathBuf,
    /// Entry function name to call inside the script (`[python].entry`).
    pub entry: String,
}

/// Structured Python-bridge diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonBridgeError {
    /// Module whose Python bridge failed.
    pub module_id: ModuleId,
    /// Stage that was being executed.
    pub stage_id: StageId,
    /// Phase of the bridge at which the failure occurred.
    pub phase: PythonBridgePhase,
    /// Stable human-readable detail.
    pub message: String,
}

/// Where in the bridge the failure originated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PythonBridgePhase {
    /// Script path referenced by the manifest is missing on disk.
    MissingScript,
    /// Marshaling the host `ConfigView` into a Python dict failed.
    ConfigEncoding,
    /// Interpreter initialization / `importlib` setup failed.
    Init,
    /// Importing / executing the user script or calling the entry raised.
    ScriptError,
    /// The entry returned a value that is not a Python `str`.
    OutputEncoding,
}

impl std::fmt::Display for PythonBridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "python bridge failure in stage '{}' for module '{}' (phase={:?}): {}",
            self.stage_id, self.module_id, self.phase, self.message
        )
    }
}

impl std::error::Error for PythonBridgeError {}

/// Runtime-wide Python-bridge configuration.
///
/// Fields are retained for API stability with the previous subprocess
/// backend, but with the embedded PyO3 backend they are advisory: the
/// interpreter is linked into the host process so `interpreter` is not
/// spawned, and `timeout` is not enforced because there is no subprocess
/// to kill. Long-running scripts should bound themselves internally.
#[derive(Debug, Clone)]
pub struct PythonBridge {
    /// Advisory interpreter path (ignored by the embedded backend).
    pub interpreter: PathBuf,
    /// Advisory time budget (not enforced by the embedded backend).
    pub timeout: Option<Duration>,
}

impl Default for PythonBridge {
    fn default() -> Self {
        Self {
            interpreter: PathBuf::from("python3"),
            timeout: None,
        }
    }
}

impl PythonBridge {
    /// Invoke a Python text-postprocess module.
    pub fn run_text(
        &self,
        binding: &PythonBinding,
        config: &ConfigView,
        text: &str,
        module_id: &ModuleId,
        stage_id: &StageId,
    ) -> Result<String, PythonBridgeError> {
        if !binding.script_path.exists() {
            return Err(PythonBridgeError {
                module_id: module_id.clone(),
                stage_id: stage_id.clone(),
                phase: PythonBridgePhase::MissingScript,
                message: format!("script not found: {}", binding.script_path.display()),
            });
        }

        let script_path = absolutize(&binding.script_path);
        let script_path_str = script_path.to_string_lossy().into_owned();
        let entry = binding.entry.as_str();

        let mk = |phase: PythonBridgePhase, message: String| PythonBridgeError {
            module_id: module_id.clone(),
            stage_id: stage_id.clone(),
            phase,
            message,
        };

        Python::attach(|py| -> Result<String, PythonBridgeError> {
            // --- Build the config dict from ConfigView. -----------------
            let config_dict = PyDict::new(py);
            for (key, value) in &config.fields {
                let pv = config_value_to_py(py, value)
                    .map_err(|e| mk(PythonBridgePhase::ConfigEncoding, format_pyerr(py, e)))?;
                config_dict
                    .set_item(key, pv)
                    .map_err(|e| mk(PythonBridgePhase::ConfigEncoding, format_pyerr(py, e)))?;
            }

            // --- Load the user script via importlib.util. ---------------
            let importlib_util = py.import("importlib.util").map_err(|e| {
                mk(
                    PythonBridgePhase::Init,
                    format!("import importlib.util: {}", format_pyerr(py, e)),
                )
            })?;

            let spec = importlib_util
                .call_method1(
                    "spec_from_file_location",
                    ("slicer_user_script", script_path_str.as_str()),
                )
                .map_err(|e| {
                    mk(
                        PythonBridgePhase::Init,
                        format!("spec_from_file_location: {}", format_pyerr(py, e)),
                    )
                })?;
            if spec.is_none() {
                return Err(mk(
                    PythonBridgePhase::MissingScript,
                    format!("spec_from_file_location returned None for {script_path_str}"),
                ));
            }

            let module = importlib_util
                .call_method1("module_from_spec", (&spec,))
                .map_err(|e| {
                    mk(
                        PythonBridgePhase::Init,
                        format!("module_from_spec: {}", format_pyerr(py, e)),
                    )
                })?;

            let loader = spec.getattr("loader").map_err(|e| {
                mk(
                    PythonBridgePhase::Init,
                    format!("spec.loader: {}", format_pyerr(py, e)),
                )
            })?;

            loader
                .call_method1("exec_module", (&module,))
                .map_err(|e| mk(PythonBridgePhase::ScriptError, format_pyerr(py, e)))?;

            // --- Resolve and call the entry function. -------------------
            let fn_obj = module.getattr(entry).map_err(|e| {
                mk(
                    PythonBridgePhase::Init,
                    format!("entry '{entry}' not found: {}", format_pyerr(py, e)),
                )
            })?;

            let result = fn_obj
                .call1((text, &config_dict))
                .map_err(|e| mk(PythonBridgePhase::ScriptError, format_pyerr(py, e)))?;

            // --- Extract the returned string. ---------------------------
            let type_name = result
                .get_type()
                .name()
                .map(|n| n.to_string())
                .unwrap_or_else(|_| "<unknown>".to_string());
            result.extract::<String>().map_err(|_| {
                mk(
                    PythonBridgePhase::OutputEncoding,
                    format!("entry '{entry}' must return str, got {type_name}"),
                )
            })
        })
    }
}

fn absolutize(p: &Path) -> PathBuf {
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(p))
            .unwrap_or_else(|_| p.to_path_buf())
    }
}

/// Render a `PyErr` into a stable, single-line string. Includes the
/// exception type and value (e.g. `ValueError: synthetic failure`). The
/// traceback is intentionally omitted — the structured `PythonBridgeError`
/// carries the stage/module/phase context the host needs; the trailing
/// human message stays short enough for progress-event surfaces.
fn format_pyerr(py: Python<'_>, err: PyErr) -> String {
    let value = err.value(py);
    let type_name = value
        .get_type()
        .name()
        .map(|n| n.to_string())
        .unwrap_or_else(|_| "Exception".to_string());
    match value.str() {
        Ok(s) => format!("{type_name}: {s}"),
        Err(_) => type_name,
    }
}

fn config_value_to_py<'py>(
    py: Python<'py>,
    value: &ConfigValue,
) -> PyResult<Bound<'py, pyo3::PyAny>> {
    match value {
        ConfigValue::Bool(b) => Ok(b.into_pyobject(py)?.to_owned().into_any()),
        ConfigValue::Int(i) => Ok(i.into_pyobject(py)?.into_any()),
        ConfigValue::Float(f) => Ok(f.into_pyobject(py)?.into_any()),
        ConfigValue::String(s) => Ok(s.into_pyobject(py)?.into_any()),
        ConfigValue::List(items) => {
            let list = PyList::empty(py);
            for item in items {
                list.append(config_value_to_py(py, item)?)?;
            }
            Ok(list.into_any())
        }
    }
}

/// `PostpassStageRunner` that dispatches text-postprocess modules to Python
/// scripts declared in their manifests.
///
/// Modules without a binding in this map return a structured fatal error —
/// this runner is constructed only with Python modules, and the scheduler
/// is expected to route non-Python modules through a WASM runner.
#[derive(Debug, Clone)]
pub struct PythonPostpassRunner {
    /// Runtime bridge used to invoke the interpreter.
    pub bridge: Arc<PythonBridge>,
    /// Python binding per module id.
    pub bindings: HashMap<ModuleId, PythonBinding>,
}

impl PythonPostpassRunner {
    /// Construct a runner with the default bridge configuration.
    pub fn new(bindings: HashMap<ModuleId, PythonBinding>) -> Self {
        Self {
            bridge: Arc::new(PythonBridge::default()),
            bindings,
        }
    }

    /// Construct a runner with a caller-supplied bridge — used by tests
    /// and by integrators that want to override the advisory fields.
    pub fn with_bridge(
        bridge: Arc<PythonBridge>,
        bindings: HashMap<ModuleId, PythonBinding>,
    ) -> Self {
        Self { bridge, bindings }
    }
}

impl PostpassStageRunner for PythonPostpassRunner {
    fn run_gcode_postprocess(
        &self,
        stage_id: &StageId,
        module: &CompiledModule,
        _blackboard: &Blackboard,
        _gcode_ir: &mut GCodeIR,
    ) -> Result<PostpassOutput, PostpassError> {
        Err(PostpassError::FatalModule {
            stage_id: stage_id.clone(),
            module_id: module.module_id.clone(),
            message: "python bridge does not support GCodePostProcess — modules in this tier must be WASM"
                .to_string(),
        })
    }

    fn run_text_postprocess(
        &self,
        stage_id: &StageId,
        module: &CompiledModule,
        _blackboard: &Blackboard,
        text: String,
    ) -> Result<PostpassOutput, PostpassError> {
        let binding =
            self.bindings
                .get(&module.module_id)
                .ok_or_else(|| PostpassError::FatalModule {
                    stage_id: stage_id.clone(),
                    module_id: module.module_id.clone(),
                    message: "no python binding registered for this module".to_string(),
                })?;

        match self.bridge.run_text(
            binding,
            module.config_view.as_ref(),
            &text,
            &module.module_id,
            stage_id,
        ) {
            Ok(new_text) => Ok(PostpassOutput::TextSuccess { text: new_text }),
            Err(e) => Err(PostpassError::FatalModule {
                stage_id: stage_id.clone(),
                module_id: module.module_id.clone(),
                message: e.to_string(),
            }),
        }
    }
}
