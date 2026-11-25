//! Registry context, Proptest execution, and process-tagged evidence.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;

use proptest::strategy::Strategy;
use proptest::test_runner::{
    Config, RngAlgorithm, RngSeed, TestCaseError, TestCaseResult, TestError, TestRunner,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::{take_case_logs, CaseGuard};

const CONTEXT_ENV: &str = "PNP_PROPERTY_CONTEXT";
const PANIC_MARKER: &str = "PNP_PROPERTY_CALLBACK_PANIC: ";
const HARNESS_MARKER: &str = "PNP_PROPERTY_HARNESS_ERROR: ";
const OVERRIDES: &[&str] = &[
    "PROPTEST_CASES",
    "PROPTEST_MAX_LOCAL_REJECTS",
    "PROPTEST_MAX_GLOBAL_REJECTS",
    "PROPTEST_MAX_FLAT_MAP_REGENS",
    "PROPTEST_MAX_SHRINK_TIME",
    "PROPTEST_MAX_SHRINK_ITERS",
    "PROPTEST_MAX_DEFAULT_SIZE_RANGE",
    "PROPTEST_FORK",
    "PROPTEST_TIMEOUT",
    "PROPTEST_RNG_ALGORITHM",
    "PROPTEST_RNG_SEED",
    "PROPTEST_DISABLE_FAILURE_PERSISTENCE",
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Context {
    schema_version: u32,
    run_token: String,
    property_id: String,
    tier: Tier,
    seed: u64,
    package: String,
    target_kind: TargetKind,
    target_name: String,
    test_name: String,
    features: Vec<String>,
    profile: Profile,
    artifact_dir: String,
    proptest_version: String,
    generator_version: u64,
    source_revision: String,
    dirty_paths: Vec<String>,
    source_state_digest: String,
    cargo_target_dir: Option<String>,
    completion_eligible: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Tier {
    Routine,
    Extended,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TargetKind {
    Test,
    Lib,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Isolation {
    InProcess,
    Fork,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Algorithm {
    Chacha,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Profile {
    cases: u32,
    max_local_rejects: u32,
    max_global_rejects: u32,
    max_flat_map_regens: u32,
    max_default_size_range: usize,
    max_shrink_iters: u32,
    max_shrink_time_ms: u32,
    rng_algorithm: Algorithm,
    isolation: Isolation,
    timeout_ms: u32,
    bounds: BTreeMap<String, u64>,
}

/// Invalid property context or unsuccessful property execution.
#[derive(Debug)]
pub struct PropertyRunError {
    kind: &'static str,
    message: String,
}

impl PropertyRunError {
    fn new(kind: &'static str, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Stable machine-readable classification for callers and exit accounting.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        self.kind
    }
}

impl fmt::Display for PropertyRunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

impl std::error::Error for PropertyRunError {}

/// Registry-selected property execution context.
pub struct PropertyRun {
    context: Context,
    raw_context: Value,
    context_json: String,
    config: Config,
}

impl PropertyRun {
    /// Load and validate the registry-selected property context.
    pub fn new(
        property_id: &'static str,
        test_name: &'static str,
    ) -> Result<Self, PropertyRunError> {
        let context_json = std::env::var(CONTEXT_ENV).map_err(|_| {
            PropertyRunError::new(
                "missing_property_context",
                format!("set by the registry-backed command: cargo xtask test --summary --properties --property {property_id} --tier routine"),
            )
        })?;
        let raw_context: Value = serde_json::from_str(&context_json)
            .map_err(|e| PropertyRunError::new("invalid_property_context", e.to_string()))?;
        if !raw_context
            .as_object()
            .is_some_and(|object| object.contains_key("cargo_target_dir"))
        {
            return Err(PropertyRunError::new(
                "invalid_property_context",
                "missing cargo_target_dir",
            ));
        }
        let context: Context = serde_json::from_value(raw_context.clone())
            .map_err(|e| PropertyRunError::new("invalid_property_context", e.to_string()))?;
        if context.schema_version != 1 {
            return Err(PropertyRunError::new(
                "invalid_property_context",
                "unsupported schema_version",
            ));
        }
        if context.property_id != property_id || context.test_name != test_name {
            return Err(PropertyRunError::new(
                "context_identity_mismatch",
                format!(
                    "expected {property_id}/{test_name}, received {}/{}",
                    context.property_id, context.test_name
                ),
            ));
        }
        let profile = &context.profile;
        if profile.cases == 0
            || profile.max_default_size_range == 0
            || profile.max_shrink_iters == 0
            || context.generator_version == 0
        {
            return Err(PropertyRunError::new("invalid_property_context", "cases, max_default_size_range, max_shrink_iters and generator_version must be positive"));
        }
        if matches!(profile.isolation, Isolation::InProcess) && profile.timeout_ms != 0 {
            return Err(PropertyRunError::new(
                "invalid_property_context",
                "a nonzero timeout requires fork isolation",
            ));
        }
        if context.run_token.is_empty()
            || context.property_id.is_empty()
            || context.target_name.is_empty()
            || context.test_name.is_empty()
            || context.artifact_dir.is_empty()
        {
            return Err(PropertyRunError::new(
                "invalid_property_context",
                "run identity and artifact_dir must be nonempty",
            ));
        }
        // Proptest's fork API requires a static, module_path!-style test name.
        // The context is per test process, so leaking this one name is necessary
        // to pass a dynamically selected target/name to its static API.
        let name: &'static str =
            Box::leak(format!("{}::{}", context.target_name, context.test_name).into_boxed_str());
        let config = Config {
            cases: profile.cases,
            max_local_rejects: profile.max_local_rejects,
            max_global_rejects: profile.max_global_rejects,
            max_flat_map_regens: profile.max_flat_map_regens,
            failure_persistence: None,
            source_file: None,
            test_name: Some(name),
            fork: matches!(profile.isolation, Isolation::Fork),
            // Proptest's timeout is in milliseconds; in-process timeout is forbidden.
            timeout: profile.timeout_ms,
            max_shrink_time: profile.max_shrink_time_ms,
            max_shrink_iters: profile.max_shrink_iters,
            max_default_size_range: profile.max_default_size_range,
            rng_algorithm: match profile.rng_algorithm {
                Algorithm::Chacha => RngAlgorithm::ChaCha,
            },
            rng_seed: RngSeed::Fixed(context.seed),
            // Set at check time rather than relying on the cached env-based default.
            verbose: 0,
            ..Config::default()
        };
        // Validate provenance fields via their typed deserialization, while preserving
        // the exact original object in context.json and failure.json.
        let _provenance = (
            &context.proptest_version,
            &context.source_revision,
            &context.dirty_paths,
            &context.source_state_digest,
            &context.cargo_target_dir,
        );
        #[cfg(all(feature = "property_testing_scad", not(target_arch = "wasm32")))]
        crate::test_support::property_testing::scripted::set_attempt_root(
            Some(std::path::PathBuf::from(&context.artifact_dir)),
            Some(context.generator_version),
        );
        Ok(Self {
            context,
            raw_context,
            context_json,
            config,
        })
    }

    /// Read a required positive named generator bound from the selected profile.
    pub fn bound(&self, name: &str) -> Result<u64, PropertyRunError> {
        match self.context.profile.bounds.get(name) {
            None => Err(PropertyRunError::new(
                "unknown_bound",
                format!("no bound named {name}"),
            )),
            Some(0) => Err(PropertyRunError::new(
                "invalid_bound",
                format!("bound {name} must be positive"),
            )),
            Some(value) => Ok(*value),
        }
    }

    /// Check a strategy with a fresh guard for every generated or shrink call.
    pub fn check<S, F>(&self, strategy: &S, test: F) -> Result<(), PropertyRunError>
    where
        S: Strategy,
        F: Fn(S::Value, &mut CaseGuard) -> TestCaseResult,
    {
        // Proptest 1.11 sets this private signal for fork re-entry. This is a
        // read-only dependency on its internals, not a public Proptest API.
        let worker = std::env::var_os("_PROPTEST_FORKFILE").is_some();
        let role = if worker { "worker" } else { "controller" };
        self.event(json!({"phase": "begin", "process_role": role}))?;
        let verbose =
            std::env::var_os("PROPTEST_VERBOSE").map(|v| v.to_string_lossy().into_owned());
        let mut details = RunDetails::default();
        #[cfg(all(feature = "property_testing_scad", not(target_arch = "wasm32")))]
        if !worker {
            crate::test_support::property_testing::scripted::reset_generator_telemetry();
        }
        let result = self.execute(strategy, &test, worker, verbose.as_deref(), &mut details);
        if worker {
            // rusty-fork exits the worker inside TestRunner::run; if it returns
            // early instead, a worker end still cannot impersonate a controller.
            self.emit_end(role, &result, &details, verbose.as_deref())?;
            return result;
        }
        if let Err(error) = &result {
            if let Err(e) = self.write_failure(error, &details) {
                details.artifact_write_error = Some(e.to_string());
            }
        }
        self.emit_end(role, &result, &details, verbose.as_deref())?;
        result
    }

    fn execute<S, F>(
        &self,
        strategy: &S,
        test: &F,
        worker: bool,
        verbose: Option<&str>,
        details: &mut RunDetails,
    ) -> Result<(), PropertyRunError>
    where
        S: Strategy,
        F: Fn(S::Value, &mut CaseGuard) -> TestCaseResult,
    {
        if let Some(name) = OVERRIDES
            .iter()
            .find(|name| std::env::var_os(name).is_some())
        {
            return Err(PropertyRunError::new(
                "profile_override_conflict",
                format!("{name} conflicts with the measured property profile"),
            ));
        }
        if !self.context.completion_eligible {
            return Err(PropertyRunError::new(
                "harness",
                "context is not eligible for measured completion",
            ));
        }
        let dir = Path::new(&self.context.artifact_dir);
        if !worker {
            fs::create_dir_all(dir)
                .and_then(|()| fs::write(dir.join("context.json"), &self.context_json))
                .and_then(|()| fs::write(dir.join("case.txt"), b""))
                .and_then(|()| fs::write(dir.join("observations.jsonl"), b""))
                .map_err(|e| {
                    PropertyRunError::new("harness", format!("could not initialize artifacts: {e}"))
                })?;
            // A reused artifact directory must not retain an older failure.
            match fs::remove_file(dir.join("failure.json")) {
                Ok(()) => (),
                Err(e) if e.kind() == io::ErrorKind::NotFound => (),
                Err(e) => {
                    return Err(PropertyRunError::new(
                        "harness",
                        format!("could not clear previous failure: {e}"),
                    ))
                }
            }
        }
        let prior = read_observations(dir).map_err(|e| {
            PropertyRunError::new("harness", format!("could not read observations: {e}"))
        })?;
        let next = Cell::new(
            prior
                .iter()
                .map(|o| o.attempt_index)
                .max()
                .map_or(0, |n| n + 1),
        );
        let shrinking = Cell::new(
            prior
                .iter()
                .any(|o| matches!(o.result.as_str(), "failed" | "panic"))
                || incomplete(&prior),
        );
        let local_error = RefCell::new(None::<String>);
        let mut config = self.config.clone();
        config.verbose = verbose.and_then(|v| v.parse().ok()).unwrap_or(0);
        let mut runner = TestRunner::new(config);
        details.profile_applied = true;
        let run_result = catch_unwind(AssertUnwindSafe(|| {
            runner.run(strategy, |value| {
                if let Some(message) = local_error.borrow().as_ref() {
                    return Err(TestCaseError::fail(format!("{HARNESS_MARKER}{message}")));
                }
                let index = next.get();
                next.set(index + 1);
                let phase = if shrinking.get() { "shrink" } else { "initial" };
                let input = format!("{value:?}");
                let mut observation = Observation {
                    pid: std::process::id(),
                    process_role: if worker { "worker" } else { "controller" }.to_string(),
                    attempt_index: index,
                    phase: phase.to_string(),
                    result: "started".to_string(),
                    input: input.clone(),
                    reason: None,
                    logs: Vec::new(),
                };
                let before =
                    append_json(&dir.join("observations.jsonl"), &observation).and_then(|()| {
                        append_text(
                            &dir.join("case.txt"),
                            &format!("--- attempt {index} phase {phase} ---\n{input}\n"),
                        )
                    });
                if let Err(e) = before {
                    let message = format!("saving input before callback: {e}");
                    *local_error.borrow_mut() = Some(message.clone());
                    return Err(TestCaseError::fail(format!("{HARNESS_MARKER}{message}")));
                }
                // Drain any older retained logs, then retain exactly this attempt's
                // logs after the guard drops on both normal return and unwind.
                let _ = take_case_logs();
                let callback = catch_unwind(AssertUnwindSafe(|| {
                    let mut guard = CaseGuard::new();
                    test(value, &mut guard)
                }));
                let (result, panicked) = match callback {
                    Ok(result) => (result, false),
                    Err(payload) => {
                        let message = payload
                            .downcast_ref::<String>()
                            .map(String::as_str)
                            .or_else(|| payload.downcast_ref::<&str>().copied())
                            .unwrap_or("non-string panic payload");
                        (
                            Err(TestCaseError::fail(format!("{PANIC_MARKER}{message}"))),
                            true,
                        )
                    }
                };
                observation.logs = take_case_logs()
                    .into_iter()
                    .map(|(level, message)| LogRecord {
                        level: level.as_str().to_string(),
                        message,
                    })
                    .collect();
                observation.result = match &result {
                    Ok(()) => "passed",
                    Err(TestCaseError::Reject(_)) => "rejected",
                    Err(TestCaseError::Fail(_)) if panicked => "panic",
                    Err(TestCaseError::Fail(_)) => "failed",
                }
                .to_string();
                observation.reason = match &result {
                    Ok(()) => None,
                    Err(TestCaseError::Reject(reason) | TestCaseError::Fail(reason)) => {
                        Some(reason.message().to_string())
                    }
                };
                if matches!(observation.result.as_str(), "panic" | "failed") {
                    shrinking.set(true);
                }
                if let Err(e) = append_json(&dir.join("observations.jsonl"), &observation) {
                    let message = format!("saving callback observation: {e}");
                    *local_error.borrow_mut() = Some(message.clone());
                    return Err(TestCaseError::fail(format!("{HARNESS_MARKER}{message}")));
                }
                result
            })
        }));
        details.runner_statistics = format!("{runner}");
        let observations = read_observations(dir).map_err(|e| {
            PropertyRunError::new("harness", format!("could not collect observations: {e}"))
        })?;
        details.accepted_test_calls = observations
            .iter()
            .filter(|o| matches!(o.result.as_str(), "passed" | "failed" | "panic"))
            .count() as u64;
        details.rejected_test_calls = observations
            .iter()
            .filter(|o| o.result == "rejected")
            .count() as u64;
        details.call_counts_complete = !incomplete(&observations);
        details.observations = observations;
        if let Some(message) = local_error.into_inner() {
            return Err(PropertyRunError::new("harness", message));
        }
        // A fatal scripted generator/adapter/artifact error outside any callback
        // must fail the run even though Proptest only saw a strategy value.
        #[cfg(all(feature = "property_testing_scad", not(target_arch = "wasm32")))]
        if let Some(fault) =
            crate::test_support::property_testing::scripted::take_generator_fault()
        {
            return Err(PropertyRunError::new("generator_or_oracle", fault.message));
        }
        let outcome = match run_result {
            Ok(result) => result,
            Err(_) => {
                return Err(PropertyRunError::new(
                    if self.config.fork {
                        "process_failure"
                    } else {
                        "harness"
                    },
                    "Proptest runner panicked before completion",
                ))
            }
        };
        match outcome {
            Ok(()) if !details.call_counts_complete => Err(PropertyRunError::new(
                "process_failure",
                "callback started without a recorded result",
            )),
            Ok(()) if details.accepted_test_calls < u64::from(self.context.profile.cases) => {
                Err(PropertyRunError::new(
                    "harness",
                    "fewer observed accepted calls than required cases",
                ))
            }
            Ok(()) => {
                // With no persistence/cache and a passing run, no shrink calls
                // occur: non-rejected callback calls equal accepted cases.
                details.accepted_cases = Some(details.accepted_test_calls);
                Ok(())
            }
            Err(TestError::Abort(reason)) => {
                let message = reason.message();
                let kind = if (self.config.fork && is_process_failure(message))
                    || !details.call_counts_complete
                {
                    "process_failure"
                } else if message.contains("Too many") && message.contains("reject") {
                    "exhausted"
                } else {
                    "generator_or_oracle"
                };
                Err(PropertyRunError::new(kind, message))
            }
            Err(TestError::Fail(reason, value)) => {
                let input = format!("{value:?}");
                let observed_panic =
                    details.observations.iter().rev().any(|observation| {
                        observation.result == "panic" && observation.input == input
                    });
                let message = reason.message();
                let callback_reported_reason = details.observations.iter().any(|observation| {
                    observation.result == "failed" && observation.reason.as_deref() == Some(message)
                });
                details.concrete_input = Some(input);
                let kind = if message.starts_with(HARNESS_MARKER) && !callback_reported_reason {
                    "harness"
                } else if !details.call_counts_complete
                    || (self.config.fork
                        && is_process_failure(message)
                        && !callback_reported_reason)
                {
                    "process_failure"
                } else if observed_panic && message.starts_with(PANIC_MARKER) {
                    "panic"
                } else {
                    "sut_failure"
                };
                Err(PropertyRunError::new(kind, message))
            }
        }
    }

    fn event(&self, mut event: Value) -> Result<(), PropertyRunError> {
        let object = event.as_object_mut().expect("event object");
        object.insert("schema_version".into(), json!(1));
        object.insert("run_token".into(), json!(self.context.run_token));
        object.insert("property_id".into(), json!(self.context.property_id));
        object.insert("pid".into(), json!(std::process::id()));
        let line = format!("PNP_PROPERTY_EVENT {event}\n");
        io::stdout().lock().write_all(line.as_bytes()).map_err(|e| {
            PropertyRunError::new("harness", format!("could not emit property event: {e}"))
        })
    }

    fn emit_end(
        &self,
        role: &str,
        result: &Result<(), PropertyRunError>,
        details: &RunDetails,
        verbose: Option<&str>,
    ) -> Result<(), PropertyRunError> {
        let outcome = match result {
            Ok(()) => "passed",
            Err(e) if e.kind == "exhausted" => "exhausted",
            Err(_) => "failed",
        };
        self.event(json!({
            "phase": "end", "process_role": role, "outcome": outcome,
            "accepted_cases": details.accepted_cases, "accepted_test_calls": details.accepted_test_calls,
            "rejected_test_calls": details.rejected_test_calls, "call_counts_complete": details.call_counts_complete,
            "runner_statistics": details.runner_statistics, "shrink_completed": shrink_state(),
            "completion_eligible": self.context.completion_eligible && details.profile_applied,
            "error_kind": result.as_ref().err().map(PropertyRunError::kind),
            "proptest_verbose_override": verbose, "artifact_write_error": details.artifact_write_error,
        }))
    }

    fn write_failure(&self, error: &PropertyRunError, details: &RunDetails) -> io::Result<()> {
        let dir = Path::new(&self.context.artifact_dir);
        fs::create_dir_all(dir)?;
        // A failure before execution still retains the exact selected context.
        fs::write(dir.join("context.json"), &self.context_json)?;
        let selected = details
            .observations
            .iter()
            .rev()
            .find(|o| {
                !matches!(o.result.as_str(), "started" | "rejected" | "passed")
                    && details
                        .concrete_input
                        .as_ref()
                        .is_none_or(|input| input == &o.input)
            })
            .or_else(|| {
                details
                    .observations
                    .iter()
                    .rev()
                    .find(|o| o.result != "started")
            })
            .or_else(|| details.observations.last());
        let concrete = details
            .concrete_input
            .as_deref()
            .or_else(|| selected.map(|o| o.input.as_str()));
        let classification = match error.kind {
            "sut_failure" => "sut_or_contract",
            "panic" => "panic",
            "exhausted" => "exhausted",
            "process_failure" => "process_failure",
            "generator_or_oracle" => "generator_or_oracle",
            _ => "harness",
        };
        let tier = match self.context.tier {
            Tier::Routine => "routine",
            Tier::Extended => "extended",
        };
        let mut args = vec![
            "test".to_owned(),
            "-p".to_owned(),
            self.context.package.clone(),
        ];
        if !self.context.features.is_empty() {
            args.extend(["--features".to_owned(), self.context.features.join(",")]);
        }
        match self.context.target_kind {
            TargetKind::Test => {
                args.extend(["--test".to_owned(), self.context.target_name.clone()]);
            }
            TargetKind::Lib => args.push("--lib".to_owned()),
        }
        args.extend([
            self.context.test_name.clone(),
            "--".to_owned(),
            "--exact".to_owned(),
            "--nocapture".to_owned(),
        ]);
        // A scripted record is only selectable with `--with-scad`; a
        // reproduction command without it would fail as unavailable.
        let scad = self
            .context
            .features
            .iter()
            .any(|feature| feature.contains("property_testing_scad"));
        let scad_flag = if scad { " --with-scad" } else { "" };
        // Link the retained attempt journals so a reader can open the exact
        // inputs that existed before the render that failed.
        let mut retained_attempts: Vec<String> = fs::read_dir(dir.join("attempts"))
            .map(|entries| {
                entries
                    .filter_map(|entry| entry.ok())
                    .filter(|entry| entry.path().is_dir())
                    .map(|entry| entry.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        retained_attempts.sort();
        let report = json!({
            "classification": classification, "error_kind": error.kind, "message": error.message,
            "context": self.raw_context, "concrete_input": concrete,
            "retained_attempts": retained_attempts,
            "reproduction_command": format!("cargo xtask test --summary --properties --property {} --tier {}{} --seed {}", self.context.property_id, tier, scad_flag, self.context.seed),
            "reproduction_direct": {
                "program": "cargo", "args": args,
                "env": { "PNP_PROPERTY_CONTEXT": self.context_json },
            },
            "captured_sdk_logs": selected.map(|o| &o.logs),
            "shrink_completed": shrink_state(), "shrink_status_reason": shrink_status_reason(),
        });
        fs::write(
            dir.join("failure.json"),
            serde_json::to_vec_pretty(&report).map_err(io::Error::other)?,
        )
    }
}

#[derive(Default)]
struct RunDetails {
    accepted_cases: Option<u64>,
    accepted_test_calls: u64,
    rejected_test_calls: u64,
    runner_statistics: String,
    call_counts_complete: bool,
    profile_applied: bool,
    concrete_input: Option<String>,
    observations: Vec<Observation>,
    artifact_write_error: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct LogRecord {
    level: String,
    message: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Observation {
    pid: u32,
    process_role: String,
    attempt_index: u64,
    phase: String,
    result: String,
    input: String,
    reason: Option<String>,
    logs: Vec<LogRecord>,
}

fn append_text(path: &Path, text: &str) -> io::Result<()> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?
        .write_all(text.as_bytes())
}

fn append_json(path: &Path, value: &Observation) -> io::Result<()> {
    let mut line = serde_json::to_string(value).map_err(io::Error::other)?;
    line.push('\n');
    append_text(path, &line)
}

fn read_observations(dir: &Path) -> io::Result<Vec<Observation>> {
    fs::read_to_string(dir.join("observations.jsonl"))?
        .lines()
        .map(|line| serde_json::from_str(line).map_err(io::Error::other))
        .collect()
}

fn incomplete(observations: &[Observation]) -> bool {
    let mut active = BTreeSet::new();
    for observation in observations {
        if observation.result == "started" {
            active.insert(observation.attempt_index);
        } else {
            active.remove(&observation.attempt_index);
        }
    }
    !active.is_empty()
}

fn is_process_failure(message: &str) -> bool {
    message.starts_with("Child process ")
        || message.starts_with("Timed out waiting for child process")
        || message.starts_with("Giving up after ")
}

/// The scripted generator's own shrink-completion state, when it has one.
///
/// `Null` means "no scripted generator took part in this run"; Proptest 1.11
/// does not expose whether a purely Proptest shrink reached a limit.
fn shrink_state() -> Value {
    #[cfg(all(feature = "property_testing_scad", not(target_arch = "wasm32")))]
    {
        if let Some(completed) =
            crate::test_support::property_testing::scripted::last_shrink_completed()
        {
            return json!(completed);
        }
    }
    Value::Null
}

fn shrink_status_reason() -> &'static str {
    if shrink_state().is_null() {
        "Proptest 1.11 does not expose whether shrinking reached a limit"
    } else {
        "the scripted generator recorded whether its bounded shrink search completed"
    }
}
