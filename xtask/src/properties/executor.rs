//! Registry-backed discovery, subprocess execution, and completion evidence.

use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

use super::{
    build_property_context, merge_features, parse_registry, resolve_seed, run_token, Metadata,
    PropertyRecord, Registry, TargetKind, Tier,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PropertySelection {
    pub(crate) tier: Tier,
    pub(crate) ids: Vec<String>,
    pub(crate) seed: Option<u64>,
    pub(crate) with_scad: bool,
}

pub(crate) struct SelectionError {
    pub(crate) code: i32,
    message: String,
}

impl std::fmt::Display for SelectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

pub(crate) struct PreparedRegistry {
    input: String,
}

fn selection_error(code: i32, message: impl Into<String>) -> SelectionError {
    SelectionError {
        code,
        message: message.into(),
    }
}

fn select<'a>(
    registry: &'a Registry,
    selection: &PropertySelection,
) -> Result<Vec<&'a PropertyRecord>, SelectionError> {
    for id in &selection.ids {
        let Some(record) = registry.property.iter().find(|record| record.id == *id) else {
            return Err(selection_error(2, format!("unknown_property_id: {id}")));
        };
        // A tier with no profile for an explicitly requested property must be a
        // usage error, never a silently smaller (or empty) run.
        if record.profile(selection.tier).is_err() {
            return Err(selection_error(
                2,
                format!(
                    "missing_tier_profile: {id} has no {:?} profile",
                    selection.tier
                ),
            ));
        }
    }
    let records: Vec<_> = registry
        .property
        .iter()
        .filter(|record| {
            (selection.ids.is_empty() || selection.ids.contains(&record.id))
                && (selection.with_scad || !record.requires_scad)
                && record.profile(selection.tier).is_ok()
        })
        .collect();
    if selection.with_scad && !records.iter().any(|record| record.requires_scad) {
        return Err(selection_error(2, "no_eligible_scad_properties"));
    }
    if records.is_empty() {
        return Err(selection_error(2, "empty_property_selection"));
    }
    Ok(records)
}

/// Check usage without invoking Cargo or any preflight. Full metadata validation
/// happens in `run`, after the ordinary test gates have completed.
pub(crate) fn read_and_select(
    root: &Path,
    selection: &PropertySelection,
) -> Result<PreparedRegistry, SelectionError> {
    let input = fs::read_to_string(root.join("test-properties.toml"))
        .map_err(|e| selection_error(1, format!("property_registry_read_failed: {e}")))?;
    let registry: Registry = toml::from_str(&input)
        .map_err(|e| selection_error(1, format!("property_registry_invalid: {e}")))?;
    select(&registry, selection)?;
    Ok(PreparedRegistry { input })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChildKind {
    ListAll,
    ListIgnored,
    Run,
}

#[derive(Debug)]
struct ChildPlan {
    property_id: String,
    package: String,
    target_kind: TargetKind,
    target_name: String,
    features: Vec<String>,
    test_name: String,
    kind: ChildKind,
    context_json: Option<String>,
    artifact_dir: PathBuf,
}

trait ChildLauncher {
    fn build(&self, plan: &ChildPlan) -> io::Result<Command>;
}

struct CargoLauncher {
    ws_root: PathBuf,
}

impl ChildLauncher for CargoLauncher {
    fn build(&self, plan: &ChildPlan) -> io::Result<Command> {
        let mut cmd = Command::new("cargo");
        cmd.current_dir(&self.ws_root)
            .args(["test", "-p", &plan.package]);
        if !plan.features.is_empty() {
            cmd.arg("--features").arg(plan.features.join(","));
        }
        match plan.target_kind {
            TargetKind::Test => {
                cmd.args(["--test", &plan.target_name]);
            }
            TargetKind::Lib => {
                cmd.arg("--lib");
            }
        }
        cmd.arg("--");
        match plan.kind {
            ChildKind::ListAll => {
                cmd.arg("--list");
            }
            ChildKind::ListIgnored => {
                cmd.args(["--list", "--ignored"]);
            }
            ChildKind::Run => {
                cmd.args(["--exact", &plan.test_name, "--nocapture"]);
            }
        }
        Ok(cmd)
    }
}

// These controls conflict with a measured profile. PROPTEST_VERBOSE must reach
// the SDK so it can record the logging override.
const CONFLICTING_ENVIRONMENT_VARIABLES: &[&str] = &[
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
    "_PROPTEST_FORKFILE",
];

fn is_conflicting_override(name: &OsStr) -> bool {
    name.to_str().is_some_and(|name| {
        CONFLICTING_ENVIRONMENT_VARIABLES.iter().any(|conflict| {
            if cfg!(windows) {
                name.eq_ignore_ascii_case(conflict)
            } else {
                name == *conflict
            }
        })
    })
}

/// Reject caller controls before registry selection or any test preflight.
pub(crate) fn reject_environment_overrides() -> Result<(), OsString> {
    for (name, _) in std::env::vars_os() {
        if is_conflicting_override(&name) {
            return Err(name);
        }
    }
    Ok(())
}

// Scrub launcher-added conflicts as well as inherited ones; the property
// entry point rejects inherited conflicts before any children are spawned.
fn configure_child(cmd: &mut Command, plan: &ChildPlan) {
    // Inspect both the inherited environment and additions made by a launcher.
    // Windows environment names are case-insensitive, unlike Unix names.
    let names: Vec<_> = std::env::vars_os()
        .map(|(name, _)| name)
        .chain(cmd.get_envs().map(|(name, _)| name.to_os_string()))
        .filter(|name| is_conflicting_override(name))
        .collect();
    for name in names {
        cmd.env_remove(name);
    }
    if let Some(context) = &plan.context_json {
        cmd.env("PNP_PROPERTY_CONTEXT", context);
    } else {
        cmd.env_remove("PNP_PROPERTY_CONTEXT");
    }
}

fn normalize_status(code: Option<i32>) -> i32 {
    match code {
        Some(0) => 0,
        Some(code @ 1..=255) => code,
        _ => 1,
    }
}

struct ExecutionLog {
    file: File,
    summary: bool,
}

impl ExecutionLog {
    fn message(&mut self, message: &str) -> io::Result<()> {
        writeln!(self.file, "{message}")?;
        eprintln!("{message}");
        Ok(())
    }

    fn child(&mut self, launcher: &impl ChildLauncher, plan: &ChildPlan) -> io::Result<Output> {
        let mut cmd = launcher.build(plan)?;
        configure_child(&mut cmd, plan);
        writeln!(
            self.file,
            "\n=== property {} {:?} ({}/{}, artifacts {}) ===\ncommand: {:?}",
            plan.property_id,
            plan.kind,
            plan.package,
            plan.target_name,
            plan.artifact_dir.display(),
            cmd
        )?;
        let output = cmd.output()?;
        self.file.write_all(&output.stdout)?;
        self.file.write_all(&output.stderr)?;
        writeln!(
            self.file,
            "\n=== property {} {:?} status: {:?} ===",
            plan.property_id, plan.kind, output.status
        )?;
        self.file.flush()?;
        if !self.summary {
            io::stdout().write_all(&output.stdout)?;
            io::stderr().write_all(&output.stderr)?;
        }
        Ok(output)
    }
}

fn make_plan(
    record: &PropertyRecord,
    features: &[String],
    kind: ChildKind,
    artifact_dir: &Path,
) -> ChildPlan {
    ChildPlan {
        property_id: record.id.clone(),
        package: record.package.clone(),
        target_kind: record.target_kind,
        target_name: record.target_name.clone(),
        features: features.to_vec(),
        test_name: record.test_name.clone(),
        kind,
        context_json: None,
        artifact_dir: artifact_dir.to_path_buf(),
    }
}

fn discovered(output: &Output) -> BTreeSet<String> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.strip_suffix(": test").map(str::to_owned))
        .collect()
}

/// Check all registry identities in the selected target against this exact
/// feature-enabled listing, and reject ignored tests before executing any.
fn discover(
    launcher: &impl ChildLauncher,
    log: &mut ExecutionLog,
    registry: &Registry,
    record: &PropertyRecord,
    features: &[String],
    artifact_dir: &Path,
) -> Result<(), i32> {
    let all = launch_checked(
        launcher,
        log,
        &make_plan(record, features, ChildKind::ListAll, artifact_dir),
    )?;
    let ignored = launch_checked(
        launcher,
        log,
        &make_plan(record, features, ChildKind::ListIgnored, artifact_dir),
    )?;
    let all = discovered(&all);
    let ignored = discovered(&ignored);
    let registered: BTreeSet<_> = registry
        .property
        .iter()
        .filter(|r| {
            r.package == record.package
                && r.target_name == record.target_name
                && r.target_kind == record.target_kind
        })
        .map(|r| r.test_name.clone())
        .collect();
    let properties: BTreeSet<_> = all
        .iter()
        .filter(|name| {
            name.rsplit("::")
                .next()
                .is_some_and(|part| part.starts_with("proptest_"))
        })
        .cloned()
        .collect();
    let mut mismatch = false;
    for name in registered.difference(&properties) {
        let _ = log.message(&format!(
            "missing_property_execution: registered test {name} not discovered"
        ));
        mismatch = true;
    }
    for name in properties.difference(&registered) {
        let _ = log.message(&format!("unregistered_property_test: {name}"));
        mismatch = true;
    }
    if mismatch {
        return Err(1);
    }
    if !all.contains(&record.test_name) || ignored.contains(&record.test_name) {
        let _ = log.message(&format!(
            "missing_property_execution: test {} is absent or ignored",
            record.test_name
        ));
        return Err(1);
    }
    Ok(())
}

fn launch_checked(
    launcher: &impl ChildLauncher,
    log: &mut ExecutionLog,
    plan: &ChildPlan,
) -> Result<Output, i32> {
    let output = log.child(launcher, plan).map_err(|error| {
        let _ = log.message(&format!(
            "property_child_or_log_failure: {} {:?}: {error}",
            plan.property_id, plan.kind
        ));
        1
    })?;
    let code = normalize_status(output.status.code());
    if code != 0 || !output.status.success() {
        return Err(if code == 0 { 1 } else { code });
    }
    Ok(output)
}

fn valid_evidence(output: &Output, record: &PropertyRecord, context: &Value, tier: Tier) -> bool {
    let text = String::from_utf8_lossy(&output.stdout);
    let expected_line = format!("test {} ... ok", record.test_name);
    if !text.lines().any(|line| line.trim() == expected_line) {
        return false;
    }
    let mut begin = Vec::new();
    let mut end = Vec::new();
    for line in text.lines() {
        if let Some(json) = line.strip_prefix("PNP_PROPERTY_EVENT ") {
            let Ok(event) = serde_json::from_str::<Value>(json) else {
                return false;
            };
            if event["process_role"] != "controller" {
                continue;
            }
            match event["phase"].as_str() {
                Some("begin") => begin.push(event),
                Some("end") => end.push(event),
                _ => return false,
            }
        }
    }
    if begin.len() != 1 || end.len() != 1 {
        return false;
    }
    let matches = |event: &Value| {
        event["schema_version"] == 1
            && event["run_token"] == context["run_token"]
            && event["property_id"] == record.id
    };
    matches(&begin[0])
        && matches(&end[0])
        && end[0]["outcome"] == "passed"
        && end[0]["completion_eligible"] == true
        && end[0]["accepted_cases"].as_u64().is_some_and(|cases| {
            cases >= u64::from(record.profile(tier).expect("selected profile").cases)
        })
}

fn proptest_version(root: &Path) -> Result<String, String> {
    let input =
        fs::read_to_string(root.join("Cargo.lock")).map_err(|e| format!("Cargo.lock: {e}"))?;
    let lock: toml::Value = toml::from_str(&input).map_err(|e| format!("Cargo.lock: {e}"))?;
    let versions: Vec<_> = lock
        .get("package")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter(|p| p.get("name").and_then(toml::Value::as_str) == Some("proptest"))
        .filter_map(|p| p.get("version").and_then(toml::Value::as_str))
        .collect();
    if versions.len() != 1 {
        return Err(format!(
            "expected one locked proptest version, found {}",
            versions.len()
        ));
    }
    Ok(versions[0].to_owned())
}

pub(crate) fn run(
    root: &Path,
    prepared: &PreparedRegistry,
    selection: &PropertySelection,
    log_path: &Path,
    summary: bool,
) -> i32 {
    let result =
        Metadata::load(root).and_then(|metadata| parse_registry(&prepared.input, &metadata));
    let registry = match result {
        Ok(registry) => registry,
        Err(error) => {
            eprintln!("xtask test: property registry: {error}");
            return 1;
        }
    };
    let selected = match select(&registry, selection) {
        Ok(selected) => selected,
        Err(error) => {
            eprintln!("xtask test: {error}");
            return error.code;
        }
    };
    execute(
        root,
        &registry,
        &selected,
        selection,
        log_path,
        summary,
        &CargoLauncher {
            ws_root: root.to_path_buf(),
        },
    )
}

fn execute(
    root: &Path,
    registry: &Registry,
    selected: &[&PropertyRecord],
    selection: &PropertySelection,
    log_path: &Path,
    summary: bool,
    launcher: &impl ChildLauncher,
) -> i32 {
    let file = match File::create(log_path) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("property_log_failure: {}: {error}", log_path.display());
            return 1;
        }
    };
    let mut log = ExecutionLog { file, summary };
    if selected.is_empty() {
        let _ = log.message("missing_property_execution: no selected properties");
        return 1;
    }
    let version = match proptest_version(root) {
        Ok(version) => version,
        Err(error) => {
            let _ = log.message(&format!("property_lock_failure: {error}"));
            return 1;
        }
    };
    let token = run_token();
    let cargo_target_dir = std::env::var_os("CARGO_TARGET_DIR").map(PathBuf::from);
    let mut first_failure = 0;
    for record in selected {
        let dir = root
            .join("target/property-testing")
            .join(&token)
            .join(&record.id);
        let result = (|| -> Result<(), i32> {
            let features = merge_features(record).map_err(|error| {
                let _ = log.message(&format!("property_feature_failure: {error}"));
                1
            })?;
            discover(launcher, &mut log, registry, record, &features, &dir)?;
            let context = build_property_context(
                record,
                selection.tier,
                resolve_seed(selection.seed),
                &dir,
                &version,
                cargo_target_dir.as_deref(),
            )
            .map_err(|error| {
                let _ = log.message(&format!("property_context_failure: {error}"));
                1
            })?;
            let json = serde_json::to_string(&context).map_err(|error| {
                let _ = log.message(&format!("property_context_failure: {error}"));
                1
            })?;
            fs::write(dir.join("context.json"), &json).map_err(|error| {
                let _ = log.message(&format!("property_context_failure: {error}"));
                1
            })?;
            let mut plan = make_plan(record, &features, ChildKind::Run, &dir);
            plan.context_json = Some(json);
            let output = launch_checked(launcher, &mut log, &plan)?;
            if !valid_evidence(&output, record, &context, selection.tier) {
                let _ = log.message(&format!(
                    "missing_property_execution: {} lacks matching successful completion evidence",
                    record.id
                ));
                return Err(1);
            }
            Ok(())
        })();
        if let Err(code) = result {
            if first_failure == 0 {
                first_failure = code;
            }
        }
    }
    first_failure
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::sync::OnceLock;

    // Independently pin the profile contract's prohibited environment names.
    const CONTRACT_CONFLICTS: &[&str] = &[
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
        "_PROPTEST_FORKFILE",
    ];

    fn root() -> PathBuf {
        crate::build_guests::workspace_root()
    }

    fn fixture_dir() -> PathBuf {
        let path = root()
            .join("target/property-testing/fixtures")
            .join(run_token());
        fs::create_dir_all(&path).expect("create fixture directory");
        path
    }

    fn metadata() -> &'static Metadata {
        static METADATA: OnceLock<Metadata> = OnceLock::new();
        METADATA.get_or_init(|| Metadata::load(&root()).expect("workspace metadata"))
    }

    fn live_registry() -> Registry {
        let input = fs::read_to_string(root().join("test-properties.toml")).expect("live registry");
        parse_registry(&input, metadata()).expect("valid live registry")
    }

    fn selection() -> PropertySelection {
        PropertySelection {
            tier: Tier::Routine,
            ids: Vec::new(),
            seed: Some(42),
            with_scad: false,
        }
    }

    fn fixture_record(id: &str, name: &str) -> PropertyRecord {
        let mut record = live_registry().property.remove(0);
        record.id = id.to_owned();
        record.test_name = name.to_owned();
        record
    }

    struct FixtureLauncher {
        names: Vec<String>,
        modes: BTreeMap<String, String>,
    }

    impl FixtureLauncher {
        fn new(records: &[PropertyRecord], modes: &[&str]) -> Self {
            assert_eq!(records.len(), modes.len());
            Self {
                names: records.iter().map(|r| r.test_name.clone()).collect(),
                modes: records
                    .iter()
                    .zip(modes)
                    .map(|(r, mode)| (r.id.clone(), (*mode).to_owned()))
                    .collect(),
            }
        }
    }

    impl ChildLauncher for FixtureLauncher {
        fn build(&self, plan: &ChildPlan) -> io::Result<Command> {
            let mut cmd = Command::new(std::env::current_exe()?);
            cmd.args([
                "--exact",
                "properties::executor::tests::property_child_fixture",
                "--nocapture",
            ])
            .env("XTASK_PROPERTY_CHILD_MODE", &self.modes[&plan.property_id])
            .env("XTASK_PROPERTY_CHILD_PHASE", format!("{:?}", plan.kind))
            .env(
                "XTASK_PROPERTY_CHILD_NAMES",
                serde_json::to_string(&self.names).expect("fixture names"),
            )
            .env("XTASK_PROPERTY_CHILD_NAME", &plan.test_name)
            // Positive controls for scrubbing and logging/unknown passthrough.
            .env("PROPTEST_FUTURE_OVERRIDE", "invalid")
            .env("PROPTEST_VERBOSE", "1");
            for name in CONTRACT_CONFLICTS {
                cmd.env(name, "1");
            }
            Ok(cmd)
        }
    }

    fn run_fixture(records: Vec<PropertyRecord>, modes: &[&str]) -> (i32, String) {
        let launcher = FixtureLauncher::new(&records, modes);
        let registry = Registry {
            schema_version: 1,
            property: records,
        };
        let selected: Vec<_> = registry.property.iter().collect();
        let log = fixture_dir().join("test-output.log");
        let code = execute(
            &root(),
            &registry,
            &selected,
            &selection(),
            &log,
            true,
            &launcher,
        );
        (code, fs::read_to_string(log).expect("read fixture log"))
    }

    // This is the only subprocess fixture. Its own libtest identity is distinct
    // from the synthetic test output it supplies to the real executor parser.
    #[test]
    fn property_child_fixture() {
        let Ok(mode) = std::env::var("XTASK_PROPERTY_CHILD_MODE") else {
            return;
        };
        if mode == "driver_override" {
            let args: Vec<String> = serde_json::from_str(
                &std::env::var("XTASK_PROPERTY_DRIVER_ARGS").expect("driver arguments"),
            )
            .expect("driver arguments JSON");
            std::process::exit(crate::test::test_command(&root(), &args));
        }
        for name in CONTRACT_CONFLICTS {
            assert!(
                std::env::var_os(name).is_none(),
                "{name} must not reach a property child"
            );
        }
        assert_eq!(std::env::var("PROPTEST_VERBOSE").as_deref(), Ok("1"));
        assert_eq!(
            std::env::var("PROPTEST_FUTURE_OVERRIDE").as_deref(),
            Ok("invalid")
        );
        let phase = std::env::var("XTASK_PROPERTY_CHILD_PHASE").expect("fixture phase");
        let names: Vec<String> = serde_json::from_str(
            &std::env::var("XTASK_PROPERTY_CHILD_NAMES").expect("fixture names"),
        )
        .expect("names JSON");
        let name = std::env::var("XTASK_PROPERTY_CHILD_NAME").expect("fixture name");
        if phase == "ListAll" {
            if mode != "absent" {
                for name in names {
                    println!("{name}: test");
                }
            }
            return;
        }
        if phase == "ListIgnored" {
            if mode == "ignored" {
                println!("{name}: test");
            }
            return;
        }
        println!("fixture-output-{name}-{mode}");
        if mode == "exit17" {
            std::process::exit(17);
        }
        if mode == "exit_minus_one" {
            std::process::exit(-1);
        }
        if mode == "zero_tests" {
            println!("running 0 tests");
            return;
        }
        if mode == "passing_unrelated" {
            println!("test unrelated ... ok");
            return;
        }
        let context: Value =
            serde_json::from_str(&std::env::var("PNP_PROPERTY_CONTEXT").expect("fixture context"))
                .expect("context JSON");
        if mode != "no_callback" {
            let begin = serde_json::json!({"schema_version": 1, "phase": "begin", "process_role": "controller", "run_token": context["run_token"], "property_id": context["property_id"]});
            println!("PNP_PROPERTY_EVENT {begin}");
            if mode != "begin_only" {
                let token = if mode == "wrong_token" {
                    Value::String("wrong".into())
                } else {
                    context["run_token"].clone()
                };
                let id = if mode == "wrong_id" {
                    Value::String("WRONG".into())
                } else {
                    context["property_id"].clone()
                };
                let cases = if mode == "zero_cases" {
                    0
                } else {
                    context["profile"]["cases"].as_u64().expect("cases")
                };
                let end = serde_json::json!({"schema_version": 1, "phase": "end", "process_role": "controller", "run_token": token, "property_id": id, "outcome": "passed", "accepted_cases": cases, "completion_eligible": true});
                println!("PNP_PROPERTY_EVENT {end}");
            }
        }
        println!("test {name} ... ok");
    }

    #[test]
    fn property_executor_rejects_environment_override() {
        let valid_args = r#"["--properties","--property","H-IR-ANCHORS","--seed","42"]"#;
        let unknown_args = r#"["--properties","--property","UNKNOWN","--seed","42"]"#;
        for (name, args) in CONTRACT_CONFLICTS
            .iter()
            .copied()
            .map(|name| (name, valid_args))
            .chain(std::iter::once(("PROPTEST_CASES", unknown_args)))
            .chain(cfg!(windows).then_some(("proptest_cases", valid_args)))
        {
            let mut cmd = Command::new(std::env::current_exe().expect("fixture executable"));
            // Isolate each subprocess from any inherited Proptest controls.
            for (inherited, _) in std::env::vars_os() {
                let key = inherited.to_string_lossy().to_ascii_uppercase();
                if key.starts_with("PROPTEST_") || key == "_PROPTEST_FORKFILE" {
                    cmd.env_remove(inherited);
                }
            }
            let output = cmd
                .args([
                    "--exact",
                    "properties::executor::tests::property_child_fixture",
                    "--nocapture",
                ])
                .env("XTASK_PROPERTY_CHILD_MODE", "driver_override")
                .env("XTASK_PROPERTY_DRIVER_ARGS", args)
                .env(name, "1")
                .output()
                .expect("spawn property driver fixture");
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert_eq!(output.status.code(), Some(1), "{name}, {args}: {stderr}");
            assert!(
                stderr.contains("profile_override_conflict"),
                "{name}: {stderr}"
            );
            assert!(
                stderr
                    .to_ascii_uppercase()
                    .contains(&name.to_ascii_uppercase()),
                "{name}: {stderr}"
            );
            assert!(!stderr.contains("unknown_property_id"), "{name}: {stderr}");
        }
    }

    #[test]
    fn property_registry_validates_and_discovers_exact_tests() {
        let registry = live_registry();
        let record = registry
            .property
            .iter()
            .find(|r| r.id == "H-IR-ANCHORS")
            .expect("live anchor");
        assert_eq!(
            (&*record.package, &*record.target_name, &*record.test_name),
            (
                "slicer-ir",
                "property_ir",
                "proptest_ir_travel_anchors_membership"
            )
        );
        let dir = fixture_dir();
        let launcher = CargoLauncher { ws_root: root() };
        let mut log = ExecutionLog {
            file: File::create(dir.join("live.log")).expect("live log"),
            summary: true,
        };
        let features = merge_features(record).expect("IR features");
        discover(&launcher, &mut log, &registry, record, &features, &dir)
            .expect("live exact discovery");
        let live_log = fs::read_to_string(dir.join("live.log")).expect("live discovery log");
        let names: Vec<_> = live_log
            .lines()
            .filter(|line| line.ends_with(": test"))
            .collect();
        assert_eq!(names, ["proptest_ir_travel_anchors_membership: test"]);

        let input = fs::read_to_string(root().join("test-properties.toml"))
            .expect("live fixture source")
            .replace("H-IR-ANCHORS", "SDK-FIXTURE")
            .replace("slicer-ir", "slicer-sdk")
            .replace("property_ir", "property_harness")
            .replace(
                "proptest_ir_travel_anchors_membership",
                "proptest_fixture_context_child",
            );
        let fixture = dir.join("sdk-registry.toml");
        fs::write(&fixture, &input).expect("fixture registry");
        let sdk = parse_registry(
            &fs::read_to_string(fixture).expect("fixture registry"),
            metadata(),
        )
        .expect("valid SDK fixture");
        let record = &sdk.property[0];
        let mut log = ExecutionLog {
            file: File::create(dir.join("sdk.log")).expect("SDK log"),
            summary: true,
        };
        assert_eq!(
            discover(
                &launcher,
                &mut log,
                &sdk,
                record,
                &merge_features(record).expect("SDK features"),
                &dir
            ),
            Err(1)
        );
        assert!(fs::read_to_string(dir.join("sdk.log"))
            .expect("SDK discovery")
            .contains("unregistered_property_test: proptest_fixture_all_rejecting"));

        let absent_input = fs::read_to_string(root().join("test-properties.toml"))
            .expect("live fixture source")
            .replace(
                "proptest_ir_travel_anchors_membership",
                "proptest_absent_fixture",
            );
        let absent_path = dir.join("absent-registry.toml");
        fs::write(&absent_path, absent_input).expect("absent registry fixture");
        let absent = parse_registry(
            &fs::read_to_string(absent_path).expect("absent fixture registry"),
            metadata(),
        )
        .expect("valid registry with an absent test");
        let absent_log = dir.join("absent.log");
        assert_eq!(
            execute(
                &root(),
                &absent,
                &[&absent.property[0]],
                &selection(),
                &absent_log,
                true,
                &launcher
            ),
            1
        );
        let absent_log = fs::read_to_string(absent_log).expect("absent discovery log");
        assert!(absent_log.contains(
            "missing_property_execution: registered test proptest_absent_fixture not discovered"
        ));
        assert!(absent_log
            .contains("unregistered_property_test: proptest_ir_travel_anchors_membership"));

        for mode in ["absent", "ignored"] {
            let (code, log) =
                run_fixture(vec![fixture_record("FIXTURE", "proptest_fixture")], &[mode]);
            assert_eq!(code, 1, "{mode}");
            assert!(log.contains("missing_property_execution"), "{mode}: {log}");
        }
    }

    #[test]
    fn property_selector_rejects_conflicts_and_unknown_ids() {
        let dir = fixture_dir();
        fs::copy(
            root().join("test-properties.toml"),
            dir.join("test-properties.toml"),
        )
        .expect("fixture registry");
        let cases: &[(&[&str], &str)] = &[
            (&["--properties", "--accelerated"], "--accelerated"),
            (&["--properties", "--summary-from", "-"], "--summary-from"),
            (&["--properties", "--package", "slicer-ir"], "--package"),
            (&["--properties", "-p", "slicer-ir"], "-p"),
            (&["--properties", "--test", "property_ir"], "--test"),
            (&["--properties", "--no-run"], "--no-run"),
            (&["--properties", "--list"], "--list"),
            (&["--properties", "filter"], "filter"),
            (&["--properties", "--", "--list"], "--"),
            (
                &["--properties", "--tier", "unknown"],
                "invalid_property_tier",
            ),
            (&["--properties", "--seed", "no"], "invalid_property_seed"),
            (
                &["--properties", "--seed", "18446744073709551616"],
                "invalid_property_seed",
            ),
            (
                &["--tier", "routine"],
                "property_selector_requires_properties",
            ),
            (
                &["--property", "H-IR-ANCHORS"],
                "property_selector_requires_properties",
            ),
            (&["--seed", "42"], "property_selector_requires_properties"),
            (&["--with-scad"], "property_selector_requires_properties"),
        ];
        for (args, token) in cases {
            let args: Vec<_> = args.iter().map(|s| (*s).to_owned()).collect();
            let error = crate::test::parse_test_options(&args).expect_err("selector rejected");
            assert!(error.contains(token), "{args:?}: {error}");
            assert_eq!(crate::test::test_command(&dir, &args), 2, "{args:?}");
        }
        for (args, token) in [
            (
                vec!["--properties", "--property", "UNKNOWN"],
                "unknown_property_id",
            ),
            (
                vec!["--properties", "--with-scad"],
                "no_eligible_scad_properties",
            ),
        ] {
            let selection = if token == "unknown_property_id" {
                PropertySelection {
                    ids: vec!["UNKNOWN".into()],
                    ..selection()
                }
            } else {
                PropertySelection {
                    with_scad: true,
                    ..selection()
                }
            };
            let error = read_and_select(&dir, &selection)
                .err()
                .expect("usage error");
            assert_eq!(error.code, 2);
            assert!(error.to_string().contains(token));
            let args: Vec<_> = args.iter().map(|s| (*s).to_owned()).collect();
            assert_eq!(crate::test::test_command(&dir, &args), 2);
        }
        fs::write(
            dir.join("test-properties.toml"),
            "schema_version = 1\nproperty = []\n",
        )
        .expect("empty fixture registry");
        assert!(read_and_select(&dir, &selection())
            .err()
            .expect("empty selection")
            .to_string()
            .contains("empty_property_selection"));
        assert_eq!(crate::test::test_command(&dir, &["--properties".into()]), 2);
        let mut tier_only = fixture_record("TIER-ONLY", "proptest_fixture");
        tier_only.extended = None;
        let registry = Registry {
            schema_version: 1,
            property: vec![tier_only],
        };
        let error = select(
            &registry,
            &PropertySelection {
                tier: Tier::Extended,
                ids: vec!["TIER-ONLY".into()],
                ..selection()
            },
        )
        .expect_err("named record without the selected tier profile");
        assert_eq!(error.code, 2);
        assert!(error.to_string().contains("missing_tier_profile"));
        assert!(
            !dir.join("target").exists(),
            "usage errors must not create a log or run gates"
        );
    }

    #[test]
    fn property_executor_rejects_missing_zero_ignored_and_mismatched_evidence() {
        for mode in [
            "zero_tests",
            "ignored",
            "passing_unrelated",
            "begin_only",
            "wrong_token",
            "wrong_id",
            "zero_cases",
            "no_callback",
        ] {
            let (code, log) =
                run_fixture(vec![fixture_record("FIXTURE", "proptest_fixture")], &[mode]);
            assert_eq!(code, 1, "{mode}: {log}");
            assert!(log.contains("missing_property_execution"), "{mode}: {log}");
            assert!(
                log.contains(&format!(
                    "property FIXTURE {:?}",
                    if mode == "ignored" {
                        ChildKind::ListIgnored
                    } else {
                        ChildKind::Run
                    }
                )),
                "{mode}: {log}"
            );
        }
        let dir = fixture_dir();
        let registry = live_registry();
        let launcher = FixtureLauncher::new(&[], &[]);
        let log_path = dir.join("empty.log");
        assert_eq!(
            execute(
                &root(),
                &registry,
                &[],
                &selection(),
                &log_path,
                true,
                &launcher
            ),
            1
        );
        assert!(fs::read_to_string(log_path)
            .expect("empty selection log")
            .contains("missing_property_execution"));
    }

    #[test]
    fn property_executor_appends_logs_and_preserves_child_status() {
        for (status, expected) in [
            (None, 1),
            (Some(0), 0),
            (Some(1), 1),
            (Some(17), 17),
            (Some(255), 255),
            (Some(256), 1),
            (Some(-1), 1),
        ] {
            assert_eq!(normalize_status(status), expected, "status {status:?}");
        }
        let records = vec![
            fixture_record("FIXTURE-A", "proptest_first"),
            fixture_record("FIXTURE-B", "proptest_second"),
        ];
        let (code, log) = run_fixture(records, &["pass", "exit17"]);
        assert_eq!(code, 17, "{log}");
        let first = log
            .find("fixture-output-proptest_first-pass")
            .expect("first child output");
        let second = log
            .find("fixture-output-proptest_second-exit17")
            .expect("second child output");
        assert!(first < second, "earlier child output must survive");

        let records = vec![
            fixture_record("FIXTURE-A", "proptest_first"),
            fixture_record("FIXTURE-B", "proptest_second"),
        ];
        let (code, log) = run_fixture(records, &["exit17", "pass"]);
        assert_eq!(code, 17, "first child status must be retained: {log}");
        assert!(
            log.contains("fixture-output-proptest_second-pass"),
            "later properties must run after a failure: {log}"
        );

        let (code, log) = run_fixture(
            vec![fixture_record("FIXTURE", "proptest_fixture")],
            &["exit_minus_one"],
        );
        #[cfg(windows)]
        assert_eq!(code, 1, "Windows -1 must not wrap to success: {log}");
        #[cfg(unix)]
        assert_eq!(code, 255, "Unix reports exit(-1) as its native 255: {log}");

        let dir = fixture_dir();
        let registry = live_registry();
        // The live registry grows as packet properties land; the fixture launcher
        // must cover every record rather than assume a single-entry roster.
        let modes = vec!["pass"; registry.property.len()];
        let launcher = FixtureLauncher::new(&registry.property, &modes);
        let selected: Vec<_> = registry.property.iter().collect();
        assert_eq!(
            execute(
                &root(),
                &registry,
                &selected,
                &selection(),
                &dir,
                true,
                &launcher
            ),
            1,
            "directory cannot be a log file"
        );
    }
}
