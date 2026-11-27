//! Registry-backed discovery, subprocess execution, and completion evidence.

use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

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

#[derive(Debug)]
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
    /// Whole-process deadline. Required for a SCAD property's execution child,
    /// whose generation and shrink rendering happen outside any callback.
    timeout_ms: Option<u64>,
}

/// The captured result of one child, including whether supervision ended it.
struct ChildOutput {
    status: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    timed_out: bool,
    stdout_complete: bool,
    stderr_complete: bool,
}

impl ChildOutput {
    fn success(&self) -> bool {
        self.status == Some(0) && !self.timed_out
    }

    fn from_output(output: std::process::Output) -> Self {
        Self {
            status: output.status.code(),
            stdout: output.stdout,
            stderr: output.stderr,
            timed_out: false,
            stdout_complete: true,
            stderr_complete: true,
        }
    }
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

    fn child(
        &mut self,
        launcher: &impl ChildLauncher,
        plan: &ChildPlan,
    ) -> io::Result<ChildOutput> {
        let mut cmd = launcher.build(plan)?;
        configure_child(&mut cmd, plan);
        writeln!(
            self.file,
            "\n=== property {} {:?} ({}/{}, artifacts {}) ===\ncommand: {:?}\ntimeout: {:?}",
            plan.property_id,
            plan.kind,
            plan.package,
            plan.target_name,
            plan.artifact_dir.display(),
            cmd,
            plan.timeout_ms
        )?;
        let output = match plan.timeout_ms {
            Some(timeout_ms) => {
                let output = supervise(&mut cmd, timeout_ms)?;
                if output.timed_out {
                    let message = format!(
                        "property_process_timeout: {} {:?} exceeded {timeout_ms} ms; process tree terminated",
                        plan.property_id, plan.kind
                    );
                    self.message(&message)?;
                }
                output
            }
            None => ChildOutput::from_output(cmd.output()?),
        };
        self.file.write_all(&output.stdout)?;
        self.file.write_all(&output.stderr)?;
        writeln!(
            self.file,
            "\n=== property {} {:?} status: {:?} (timed_out: {}) ===",
            plan.property_id, plan.kind, output.status, output.timed_out
        )?;
        if !output.stdout_complete || !output.stderr_complete {
            let message = format!(
                "property_child_output_incomplete: {} {:?} (stdout_complete: {}, stderr_complete: {})",
                plan.property_id, plan.kind, output.stdout_complete, output.stderr_complete
            );
            self.message(&message)?;
        }
        self.file.flush()?;
        if !self.summary {
            io::stdout().write_all(&output.stdout)?;
            io::stderr().write_all(&output.stderr)?;
        }
        Ok(output)
    }
}

/// A Windows Job Object that contains the execution child and every descendant.
///
/// The job is created with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, so closing the
/// handle terminates any process still in the job. That covers the case the
/// deadline path cannot: the controller exits abnormally and leaves a worker
/// descendant behind. Assignment can fail when the runner already placed this
/// process in a non-nestable job; the supervisor kills and reaps the child and
/// refuses the run rather than executing it uncontained.
#[cfg(windows)]
struct ProcessJob(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl ProcessJob {
    fn create() -> Option<Self> {
        use windows_sys::Win32::System::JobObjects::{
            CreateJobObjectW, JobObjectExtendedLimitInformation, SetInformationJobObject,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        };
        // SAFETY: null attributes/name ask Windows for an unnamed job object
        // owned by this process; the returned handle is checked before use.
        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() {
                return None;
            }
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let configured = SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            if configured == 0 {
                let _ = windows_sys::Win32::Foundation::CloseHandle(job);
                return None;
            }
            Some(Self(job))
        }
    }

    fn assign(&self, pid: u32) -> bool {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::JobObjects::AssignProcessToJobObject;
        use windows_sys::Win32::System::Threading::{
            OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
        };
        // SAFETY: the handle is closed on every path; the job handle is owned by
        // `self` and outlives the call.
        unsafe {
            let process = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, pid);
            if process.is_null() {
                return false;
            }
            let assigned = AssignProcessToJobObject(self.0, process);
            let _ = CloseHandle(process);
            assigned != 0
        }
    }
}

#[cfg(windows)]
impl Drop for ProcessJob {
    fn drop(&mut self) {
        // KILL_ON_JOB_CLOSE terminates every remaining member as a side effect.
        // SAFETY: `self.0` is a live handle owned by this value.
        unsafe {
            let _ = windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

// Test-only delay between spawning a suspended child and assigning it to the
// containment job. Widening the startup window on purpose lets a regression
// prove the child cannot execute before assignment.
#[cfg(all(test, windows))]
thread_local! {
    static TEST_ASSIGN_DELAY_MS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Resume the primary thread of a child created with `CREATE_SUSPENDED`.
///
/// A freshly created process has exactly one thread. Enumerate it rather than
/// relying on std, which does not expose the thread handle. Fail closed: the
/// caller kills and reaps the child if this returns `false`.
#[cfg(windows)]
fn resume_main_thread(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
    };
    use windows_sys::Win32::System::Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME};
    // SAFETY: the snapshot and thread handles are closed on every path; the
    // entry buffer is initialized before the first API call.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return false;
        }
        let mut entry: THREADENTRY32 = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
        let mut resumed = false;
        if Thread32First(snapshot, &mut entry) != 0 {
            loop {
                if entry.th32OwnerProcessID == pid {
                    let thread = OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID);
                    if !thread.is_null() {
                        let previous = ResumeThread(thread);
                        let _ = CloseHandle(thread);
                        resumed = previous != u32::MAX;
                    }
                    break;
                }
                if Thread32Next(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);
        resumed
    }
}

/// Drain one child pipe into a shared buffer and signal completion.
///
/// A descendant that leaks the pipe's inherited write handle can keep a plain
/// `read_to_end` blocked long after the controller exits, so `finish` waits for
/// a short grace period and then returns the bytes read so far instead of
/// joining indefinitely. The reader thread then detaches; it exits when the last
/// write handle closes.
struct PipeDrain {
    bytes: Arc<Mutex<Vec<u8>>>,
    finished: mpsc::Receiver<bool>,
}

impl PipeDrain {
    fn spawn(pipe: Option<impl Read + Send + 'static>) -> Self {
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let shared = Arc::clone(&bytes);
        let (finished, done) = mpsc::channel();
        std::thread::spawn(move || {
            let mut pipe = pipe;
            let mut chunk = [0u8; 8192];
            let mut complete = true;
            loop {
                match pipe.as_mut().map(|pipe| pipe.read(&mut chunk)) {
                    Some(Ok(0)) | None => break,
                    Some(Ok(read)) => {
                        if let Ok(mut buffer) = shared.lock() {
                            buffer.extend_from_slice(&chunk[..read]);
                        }
                    }
                    Some(Err(error)) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Some(Err(_)) => {
                        // A terminal read error is not EOF: the tail may be
                        // missing, so the caller must not report complete capture.
                        complete = false;
                        break;
                    }
                }
            }
            let _ = finished.send(complete);
        });
        Self {
            bytes,
            finished: done,
        }
    }

    fn finish(self) -> (Vec<u8>, bool) {
        // The child has already exited (or was killed), so a well-behaved pipe
        // reaches EOF immediately. The bound covers only leaked handles; if it
        // expires, or the reader reports a terminal error, the caller records
        // incomplete capture instead of treating a truncated tail as complete.
        let complete = self
            .finished
            .recv_timeout(Duration::from_secs(2))
            .unwrap_or_default();
        let bytes = self
            .bytes
            .lock()
            .map(|bytes| bytes.clone())
            .unwrap_or_default();
        (bytes, complete)
    }
}

/// Run one execution child under a whole-process deadline.
///
/// Both pipes are drained concurrently, and a deadline kill terminates the
/// **process tree**, not just the controller: a SCAD property renders and
/// shrinks in Proptest fork descendants that outlive a killed parent.
fn supervise(cmd: &mut Command, timeout_ms: u64) -> io::Result<ChildOutput> {
    // On Windows, containment is established before execution is accepted: an
    // uncontained controller could exit abnormally and leave descendants
    // running, which the deadline branch cannot recover. Other platforms have no
    // job object; they supervise through the deadline kill below.
    #[cfg(windows)]
    let job = ProcessJob::create().ok_or_else(|| {
        io::Error::other("could not create the Windows containment job for a property child")
    })?;
    // Create the child suspended on Windows so it cannot execute a single
    // instruction before it belongs to the containment job. Assigning a running
    // child leaves a startup window in which a descendant can be created
    // outside the job and survive an abnormal controller exit.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(windows_sys::Win32::System::Threading::CREATE_SUSPENDED);
    }
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    #[cfg(windows)]
    {
        #[cfg(test)]
        {
            let delay = TEST_ASSIGN_DELAY_MS.with(std::cell::Cell::get);
            if delay > 0 {
                std::thread::sleep(Duration::from_millis(delay));
            }
        }
        if !job.assign(child.id()) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::other(
                "could not assign the property child to its containment job; refusing to run it uncontained",
            ));
        }
        if !resume_main_thread(child.id()) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::other(
                "could not resume the suspended property child after job assignment; refusing to run it uncontained",
            ));
        }
    }
    let stdout = PipeDrain::spawn(child.stdout.take());
    let stderr = PipeDrain::spawn(child.stderr.take());
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status.code();
        }
        if Instant::now() >= deadline {
            timed_out = true;
            #[cfg(windows)]
            terminate_tree(child.id());
            // On every platform, kill the direct child before waiting so a
            // blocked fixture cannot make `wait` outlive the deadline.
            let _ = child.kill();
            let status = child.wait()?;
            break status.code();
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    if timed_out {
        // A descendant that ignored the tree kill must not keep the pipes open.
        let _ = child.kill();
    }
    // Dropping the job closes its handle; with KILL_ON_JOB_CLOSE any descendant
    // the controller left behind is terminated, including after an abnormal
    // controller exit that never reached the deadline branch.
    #[cfg(windows)]
    drop(job);
    let (stdout, stdout_complete) = stdout.finish();
    let (stderr, stderr_complete) = stderr.finish();
    Ok(ChildOutput {
        status,
        stdout,
        stderr,
        timed_out,
        stdout_complete,
        stderr_complete,
    })
}

/// Terminate a child and its descendants.
///
/// `taskkill /T` is the supported Windows tree termination; on other platforms
/// the direct child is killed and its descendants are expected to exit with it.
fn terminate_tree(pid: u32) {
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/T", "/F", "/PID", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
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
        timeout_ms: None,
    }
}

fn discovered(output: &ChildOutput) -> BTreeSet<String> {
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
) -> Result<ChildOutput, i32> {
    let output = log.child(launcher, plan).map_err(|error| {
        let _ = log.message(&format!(
            "property_child_or_log_failure: {} {:?}: {error}",
            plan.property_id, plan.kind
        ));
        1
    })?;
    let code = normalize_status(if output.timed_out {
        None
    } else {
        output.status
    });
    if code != 0 || !output.success() {
        return Err(if code == 0 { 1 } else { code });
    }
    Ok(output)
}

fn valid_evidence(
    output: &ChildOutput,
    record: &PropertyRecord,
    context: &Value,
    tier: Tier,
) -> bool {
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
    let result = Metadata::load(root).and_then(|metadata| {
        parse_registry(&prepared.input, &metadata).map(|registry| (metadata, registry))
    });
    let (metadata, registry) = match result {
        Ok(loaded) => loaded,
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
        // Every real property run audits the whole lane, not just the
        // selected records, so a deleted record cannot hide its target.
        Some(&CensusScope {
            metadata: &metadata,
            with_scad: selection.with_scad,
            packages: None,
        }),
    )
}

/// `census` is `None` only for executor unit fixtures whose synthetic
/// registry and launcher exercise record execution in isolation; `run`, the
/// only production caller, always supplies the lane-wide census.
#[allow(clippy::too_many_arguments)]
fn execute(
    root: &Path,
    registry: &Registry,
    selected: &[&PropertyRecord],
    selection: &PropertySelection,
    log_path: &Path,
    summary: bool,
    launcher: &impl ChildLauncher,
    census_scope: Option<&CensusScope<'_>>,
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
    // A census failure is retained as the first failure, but the selected
    // records still execute so their own evidence and status are logged.
    let mut first_failure = match census_scope {
        Some(scope) => census(root, &mut log, registry, scope, launcher),
        None => 0,
    };
    let version = match proptest_version(root) {
        Ok(version) => version,
        Err(error) => {
            let _ = log.message(&format!("property_lock_failure: {error}"));
            return 1;
        }
    };
    let token = run_token();
    let cargo_target_dir = std::env::var_os("CARGO_TARGET_DIR").map(PathBuf::from);
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
            if record.requires_scad {
                // Required by registry validation; read it from the selected
                // profile so each tier's deadline is honoured.
                plan.timeout_ms = record
                    .profile(selection.tier)
                    .map_err(|_| 1)?
                    .bounds
                    .get("test_process_timeout_ms")
                    .copied()
                    .filter(|value| *value > 0);
            }
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

// ---------------------------------------------------------------------------
// Fail-closed registry census.
//
// The census derives the eligible property owners and targets from Cargo
// metadata and source, never from the registry alone, and lists every eligible
// target through the same child path as `discover`. A target whose last record
// was deleted therefore still appears, as does any discovered `proptest_` test
// the registry does not claim.

/// Marks the following `fn proptest_*` as a deliberate harness fixture that is
/// intentionally not a registered property (it must still be discovered).
const CENSUS_EXEMPTION: &str = "// property-census: harness-fixture";
const CENSUS_LABEL: &str = "CENSUS";
const APPROVED_MATRIX: &str = "docs/testing/properties.md";

/// What the census audits: one lane, optionally narrowed to some packages.
pub(crate) struct CensusScope<'a> {
    pub(crate) metadata: &'a Metadata,
    pub(crate) with_scad: bool,
    /// `None` audits every eligible workspace owner.
    pub(crate) packages: Option<&'a BTreeSet<String>>,
}

struct CensusTarget {
    package: String,
    kind: TargetKind,
    name: String,
    features: Vec<String>,
    /// The target's own source defines at least one `fn proptest_`.
    source_expected: bool,
    registered: Vec<String>,
    exemptions: BTreeSet<String>,
}

impl CensusTarget {
    fn label(&self) -> String {
        format!("{}/{}/{}", self.package, kind_name(self.kind), self.name)
    }
}

fn kind_name(kind: TargetKind) -> &'static str {
    match kind {
        TargetKind::Test => "test",
        TargetKind::Lib => "lib",
    }
}

fn is_lib_kind(kind: &str) -> bool {
    matches!(kind, "lib" | "rlib" | "dylib" | "cdylib" | "staticlib")
}

fn is_property_test(name: &str) -> bool {
    name.rsplit("::")
        .next()
        .is_some_and(|part| part.starts_with("proptest_"))
}

/// Approved IDs are the matrix's exact `## <PROPERTY-ID>` headings.
fn approved_ids(root: &Path) -> io::Result<BTreeSet<String>> {
    let text = fs::read_to_string(root.join(APPROVED_MATRIX))?;
    Ok(text
        .lines()
        .filter_map(|line| line.strip_prefix("## "))
        .filter(|id| {
            let mut chars = id.chars();
            chars.next().is_some_and(|c| c.is_ascii_uppercase())
                && chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')
        })
        .map(str::to_owned)
        .collect())
}

/// Read a target's sources: a test target's own file, or a library's whole
/// source directory (unit-test modules live in submodule files).
fn target_source(kind: TargetKind, src_path: &Path) -> io::Result<String> {
    match kind {
        TargetKind::Test => fs::read_to_string(src_path),
        TargetKind::Lib => {
            let dir = src_path
                .parent()
                .ok_or_else(|| io::Error::other("library source has no parent directory"))?;
            let mut text = String::new();
            let mut files: Vec<_> = walkdir::WalkDir::new(dir)
                .into_iter()
                .collect::<Result<Vec<_>, _>>()
                .map_err(io::Error::other)?
                .into_iter()
                .filter(|entry| {
                    entry.file_type().is_file()
                        && entry.path().extension().is_some_and(|ext| ext == "rs")
                })
                .map(walkdir::DirEntry::into_path)
                .collect();
            files.sort();
            for file in files {
                text.push_str(&fs::read_to_string(file)?);
                text.push('\n');
            }
            Ok(text)
        }
    }
}

/// Function names marked by `CENSUS_EXEMPTION`. An annotation that does not
/// precede a `fn proptest_*` is kept verbatim so it can never match a
/// discovered test and is reported as stale.
fn census_exemptions(text: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut lines = text.lines().map(str::trim);
    while let Some(line) = lines.next() {
        if line != CENSUS_EXEMPTION {
            continue;
        }
        let mut name = None;
        for next in lines.by_ref() {
            if next.is_empty() || next.starts_with("#[") || next.starts_with("//") {
                continue;
            }
            name = next
                .strip_prefix("fn ")
                .and_then(|rest| {
                    rest.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                        .next()
                })
                .filter(|name| name.starts_with("proptest_"))
                .map(str::to_owned)
                .or_else(|| Some(format!("<annotation before `{next}`>")));
            break;
        }
        names.insert(name.unwrap_or_else(|| "<annotation at end of file>".to_owned()));
    }
    names
}

fn push_unique(features: &mut Vec<String>, feature: String) {
    if !features.contains(&feature) {
        features.push(feature);
    }
}

/// Enumerate eligible targets for the lane. Problems that need no Cargo run
/// (unreadable sources, registered targets absent from metadata) are logged
/// and reported through the returned flag.
fn census_targets(
    log: &mut ExecutionLog,
    registry: &Registry,
    scope: &CensusScope<'_>,
) -> (Vec<CensusTarget>, bool) {
    let metadata = scope.metadata;
    let in_scope = |name: &str| scope.packages.is_none_or(|set| set.contains(name));
    let lane_record = |record: &PropertyRecord| {
        in_scope(&record.package) && (scope.with_scad || !record.requires_scad)
    };
    let mut targets = Vec::new();
    let mut failed = false;
    let mut matched = BTreeSet::new();
    for package in metadata
        .packages
        .iter()
        .filter(|p| metadata.workspace_members.contains(&p.id) && in_scope(&p.name))
    {
        let has_core = package.features.contains_key("property_testing");
        let has_scad = package.features.contains_key("property_testing_scad");
        let owner = has_core || (scope.with_scad && has_scad);
        for target in &package.targets {
            let kind = if target.kind.iter().any(|k| is_lib_kind(k)) {
                TargetKind::Lib
            } else if target.kind.iter().any(|k| k == "test") {
                TargetKind::Test
            } else {
                continue;
            };
            let records: Vec<&PropertyRecord> = registry
                .property
                .iter()
                .filter(|r| {
                    r.package == package.name
                        && r.target_kind == kind
                        && r.target_name == target.name
                })
                .collect();
            // The registered target exists in metadata; whether it is listed
            // in this lane is decided below.
            for record in &records {
                matched.insert(record.id.clone());
            }
            if !owner && records.is_empty() {
                continue;
            }
            let label = format!("{}/{}/{}", package.name, kind_name(kind), target.name);
            let source = match target_source(kind, Path::new(&target.src_path)) {
                Ok(source) => source,
                Err(error) => {
                    let _ = log.message(&format!(
                        "property_census_failure: {label}: cannot read source: {error}"
                    ));
                    failed = true;
                    continue;
                }
            };
            let source_expected = source.contains("fn proptest_");
            let requires = |feature: &str| target.required_features.iter().any(|f| f == feature);
            let opportunistic = kind == TargetKind::Test
                && (source.contains("property_testing")
                    || requires("property_testing")
                    || requires("property_testing_scad"));
            if !source_expected && records.is_empty() && !opportunistic {
                continue;
            }
            let scad_only = requires("property_testing_scad")
                || records.iter().any(|r| r.requires_scad)
                || (kind == TargetKind::Test && source.contains("property_testing_scad"));
            // An engine-requiring target is impossible in the core lane:
            // exclude it rather than activate OpenRSCAD there.
            if scad_only && !scope.with_scad {
                for record in records.iter().filter(|r| !r.requires_scad) {
                    let _ = log.message(&format!(
                        "missing_property_execution: {label} needs the SCAD lane but core record {} is registered on it",
                        record.id
                    ));
                    failed = true;
                }
                continue;
            }
            let mut features = Vec::new();
            let owner_feature = if (scad_only && has_scad) || !has_core {
                has_scad.then_some("property_testing_scad")
            } else {
                Some("property_testing")
            };
            if let Some(feature) = owner_feature {
                push_unique(&mut features, format!("{}/{feature}", package.name));
            }
            for required in &target.required_features {
                let qualified = if required.contains('/') {
                    required.clone()
                } else {
                    format!("{}/{required}", package.name)
                };
                push_unique(&mut features, qualified);
            }
            if records.iter().any(|r| r.requires_host_algos)
                && metadata.can_see(&package.name, "slicer-core")
                && metadata
                    .package("slicer-core")
                    .is_some_and(|core| core.features.contains_key("host-algos"))
            {
                push_unique(&mut features, "slicer-core/host-algos".to_owned());
            }
            targets.push(CensusTarget {
                package: package.name.clone(),
                kind,
                name: target.name.clone(),
                features,
                source_expected,
                registered: records.iter().map(|r| r.test_name.clone()).collect(),
                exemptions: census_exemptions(&source),
            });
        }
    }
    for record in registry
        .property
        .iter()
        .filter(|r| lane_record(r) && !matched.contains(&r.id))
    {
        let _ = log.message(&format!(
            "missing_property_execution: registered target {}/{}/{} for {} is not an eligible workspace target",
            record.package,
            kind_name(record.target_kind),
            record.target_name,
            record.id
        ));
        failed = true;
    }
    (targets, failed)
}

/// List one eligible target with the real child path and compare both ways.
fn census_target(
    launcher: &impl ChildLauncher,
    log: &mut ExecutionLog,
    target: &CensusTarget,
    artifact_dir: &Path,
) -> i32 {
    let label = target.label();
    let plan = |kind| ChildPlan {
        property_id: CENSUS_LABEL.to_owned(),
        package: target.package.clone(),
        target_kind: target.kind,
        target_name: target.name.clone(),
        features: target.features.clone(),
        test_name: String::new(),
        kind,
        context_json: None,
        artifact_dir: artifact_dir.to_path_buf(),
        timeout_ms: None,
    };
    let listed = launch_checked(launcher, log, &plan(ChildKind::ListAll)).and_then(|all| {
        launch_checked(launcher, log, &plan(ChildKind::ListIgnored)).map(|ignored| (all, ignored))
    });
    let (all, ignored) = match listed {
        Ok((all, ignored)) => (discovered(&all), discovered(&ignored)),
        Err(code) => {
            let _ = log.message(&format!(
                "missing_property_execution: {label} discovery failed with status {code} (features {:?})",
                target.features
            ));
            return code;
        }
    };
    let properties: BTreeSet<_> = all
        .iter()
        .filter(|n| is_property_test(n))
        .cloned()
        .collect();
    let _ = writeln!(
        log.file,
        "property_census_target: {label} features={:?} discovered={:?} registered={:?} exempt={:?}",
        target.features, properties, target.registered, target.exemptions
    );
    let leaf = |name: &str| name.rsplit("::").next().unwrap_or(name).to_owned();
    let exempt = |name: &str| target.exemptions.contains(&leaf(name));
    let mut failed = false;
    let mut registered = BTreeSet::new();
    for name in &target.registered {
        if !registered.insert(name) {
            let _ = log.message(&format!(
                "duplicate_test_identity: {label}: {name} is registered more than once"
            ));
            failed = true;
        }
    }
    for name in &target.exemptions {
        if !properties.iter().any(|p| leaf(p) == *name) {
            let _ = log.message(&format!(
                "stale_property_census_exemption: {name} ({label}) names no discovered proptest_ test"
            ));
            failed = true;
        }
    }
    for name in registered.iter().filter(|n| !properties.contains(**n)) {
        let _ = log.message(&format!(
            "missing_property_execution: registered test {name} not discovered"
        ));
        failed = true;
    }
    for name in properties
        .iter()
        .filter(|n| !registered.contains(n) && !exempt(n))
    {
        let _ = log.message(&format!("unregistered_property_test: {label}: {name}"));
        failed = true;
    }
    for name in properties
        .iter()
        .filter(|n| ignored.contains(*n) && !exempt(n))
    {
        let _ = log.message(&format!(
            "missing_property_execution: {label}: property test {name} is ignored"
        ));
        failed = true;
    }
    if (target.source_expected || !target.registered.is_empty()) && properties.is_empty() {
        let _ = log.message(&format!(
            "missing_property_execution: {label} discovered no proptest_ tests"
        ));
        failed = true;
    }
    i32::from(failed)
}

/// Run the whole census, reporting every mismatch rather than stopping at the
/// first. Returns the first nonzero child status, else 1 on any mismatch.
fn census(
    root: &Path,
    log: &mut ExecutionLog,
    registry: &Registry,
    scope: &CensusScope<'_>,
    launcher: &impl ChildLauncher,
) -> i32 {
    let mut code = 0;
    let mut fail = |value: i32| {
        if code == 0 {
            code = value;
        }
    };
    let lane = if scope.with_scad { "scad" } else { "core" };
    let _ = log.message(&format!(
        "property_census: lane={lane} packages={:?}",
        scope.packages
    ));
    match approved_ids(root) {
        Ok(approved) => {
            let registered: BTreeSet<_> = registry.property.iter().map(|r| r.id.clone()).collect();
            for id in approved.difference(&registered) {
                let _ = log.message(&format!("missing_property_record: {id}"));
                fail(1);
            }
            for id in registered.difference(&approved) {
                let _ = log.message(&format!("unapproved_property_record: {id}"));
                fail(1);
            }
        }
        Err(error) => {
            let _ = log.message(&format!(
                "property_census_failure: cannot read approved matrix {APPROVED_MATRIX}: {error}"
            ));
            fail(1);
        }
    }
    let (targets, failed) = census_targets(log, registry, scope);
    if failed {
        fail(1);
    }
    if targets.is_empty() {
        let _ =
            log.message("missing_property_execution: property census found no eligible targets");
        fail(1);
    }
    let artifact_dir = root.join("target/property-testing/census");
    for target in &targets {
        let status = census_target(launcher, log, target, &artifact_dir);
        if status != 0 {
            fail(status);
        }
    }
    let _ = log.message(&format!(
        "property_census: lane={lane} targets={} status={code}",
        targets.len()
    ));
    code
}

/// Test entry point: the same census `execute` runs, with real metadata, the
/// real Cargo launcher and one combined log, narrowed to `packages`.
pub(crate) fn census_scoped(
    root: &Path,
    registry: &Registry,
    with_scad: bool,
    packages: &BTreeSet<String>,
    log_path: &Path,
) -> i32 {
    let metadata = match Metadata::load(root) {
        Ok(metadata) => metadata,
        Err(error) => {
            eprintln!("property_census_failure: {error}");
            return 1;
        }
    };
    let file = match File::create(log_path) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("property_log_failure: {}: {error}", log_path.display());
            return 1;
        }
    };
    let mut log = ExecutionLog {
        file,
        summary: true,
    };
    census(
        root,
        &mut log,
        registry,
        &CensusScope {
            metadata: &metadata,
            with_scad,
            packages: Some(packages),
        },
        &CargoLauncher {
            ws_root: root.to_path_buf(),
        },
    )
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
            None,
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
        if mode == "large_output" {
            let payload = "x".repeat(1024 * 1024);
            println!("BEGIN{payload}END");
            return;
        }
        if mode == "exit_with_descendant" {
            let child = Command::new(std::env::current_exe().expect("current exe"))
                .args([
                    "--exact",
                    "properties::executor::tests::property_child_fixture",
                    "--nocapture",
                ])
                .env("XTASK_PROPERTY_CHILD_MODE", "block_with_descendant")
                .env("XTASK_PROPERTY_DESCENDANT", "1")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("spawn the descendant fixture");
            println!("XTASK_PROPERTY_DESCENDANT_PID {}", child.id());
            // No callback, no deadline: the controller exits abnormally and the
            // job object must terminate the descendant.
            std::process::exit(9);
        }
        if mode == "exit_with_descendant_late" {
            let child = Command::new(std::env::current_exe().expect("current exe"))
                .args([
                    "--exact",
                    "properties::executor::tests::property_child_fixture",
                    "--nocapture",
                ])
                .env("XTASK_PROPERTY_CHILD_MODE", "block_with_descendant")
                .env("XTASK_PROPERTY_DESCENDANT", "1")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("spawn the descendant fixture");
            println!("XTASK_PROPERTY_DESCENDANT_PID {}", child.id());
            // Stay alive while the supervisor assigns this controller, then
            // exit abnormally. The descendant was created before assignment.
            std::thread::sleep(Duration::from_secs(1));
            std::process::exit(9);
        }
        if mode == "block_with_descendant" {
            // A live descendant proves the deadline kill contains the whole
            // tree, not just the controller this test binary runs as.
            if std::env::var_os("XTASK_PROPERTY_DESCENDANT").is_some() {
                // test-quality: the descendant exists only to be killed by the supervisor
                std::thread::sleep(Duration::from_secs(60));
            } else {
                // The descendant exists only to be killed by the supervisor.
                #[allow(clippy::zombie_processes)]
                let child = Command::new(std::env::current_exe().expect("current exe"))
                    .args([
                        "--exact",
                        "properties::executor::tests::property_child_fixture",
                        "--nocapture",
                    ])
                    .env("XTASK_PROPERTY_CHILD_MODE", "block_with_descendant")
                    .env("XTASK_PROPERTY_DESCENDANT", "1")
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .expect("spawn the descendant fixture");
                println!("XTASK_PROPERTY_DESCENDANT_PID {}", child.id());
                // test-quality: blocks before any callback so the deadline covers generation
                std::thread::sleep(Duration::from_secs(60));
            }
            return;
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
        // The expected roster is derived from the live registry, not hardcoded:
        // every record registered against this target must be discovered by the
        // real `--list` pass, and discovery must add nothing the registry does
        // not claim. A registered-but-undiscovered test fails here, which is the
        // same condition the executor reports as `missing_property_execution`.
        let mut expected: Vec<String> = registry
            .property
            .iter()
            .filter(|candidate| candidate.package == "slicer-ir")
            .filter(|candidate| candidate.target_name == "property_ir")
            .filter(|candidate| {
                candidate
                    .features
                    .iter()
                    .any(|feature| feature == "slicer-ir/property_testing")
            })
            .map(|candidate| format!("{}: test", candidate.test_name))
            .collect();
        expected.sort();
        let mut discovered: Vec<String> = names.iter().map(|name| (*name).to_owned()).collect();
        discovered.sort();
        assert_eq!(
            discovered, expected,
            "live discovery must match the registry"
        );
        assert!(
            names.contains(&"proptest_ir_travel_anchors_membership: test"),
            "the preserved anchor test must be discovered: {names:?}"
        );

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
                &launcher,
                None
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
        {
            let selection = PropertySelection {
                ids: vec!["UNKNOWN".into()],
                ..selection()
            };
            let error = read_and_select(&dir, &selection)
                .err()
                .expect("usage error");
            assert_eq!(error.code, 2);
            assert!(error.to_string().contains("unknown_property_id"));
            let args: Vec<String> = ["--properties", "--property", "UNKNOWN"]
                .iter()
                .map(|s| (*s).to_owned())
                .collect();
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
        // The honest selector is still honest for a registry with no SCAD
        // record: `--with-scad` must say so rather than run an empty lane.
        let scad_only = PropertySelection {
            with_scad: true,
            ..selection()
        };
        assert!(read_and_select(&dir, &scad_only)
            .err()
            .expect("unavailable SCAD selector")
            .to_string()
            .contains("no_eligible_scad_properties"));
        assert_eq!(
            crate::test::test_command(&dir, &["--properties".into(), "--with-scad".into()]),
            2
        );
        // Positive control: the live registry now carries the two scripted
        // records, so the same selector is eligible rather than unavailable.
        let live_dir = fixture_dir().join("live-scad");
        fs::create_dir_all(&live_dir).expect("live fixture directory");
        fs::copy(
            root().join("test-properties.toml"),
            live_dir.join("test-properties.toml"),
        )
        .expect("live registry copy");
        let prepared = read_and_select(&live_dir, &scad_only).expect("eligible SCAD selector");
        let live: Registry = toml::from_str(&prepared.input).expect("live registry");
        let scad: Vec<_> = live
            .property
            .iter()
            .filter(|record| record.requires_scad)
            .map(|record| record.id.as_str())
            .collect();
        // The live registry's SCAD-gated set. This is deliberately pinned: a
        // record that gains `requires_scad` without a supervision-capable
        // profile should not pass silently. Packet
        // `proptest-openrscad_09_pbt-geometry` added the two geometry records.
        assert_eq!(
            scad,
            ["D-SCAD-2D", "D-SCAD-3D", "H-BOOL-SCAD", "H-MESH-SCAD"]
        );
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
                &launcher,
                None
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
                &launcher,
                None
            ),
            1,
            "directory cannot be a log file"
        );
    }

    fn descendant_gone(pid: u32) -> io::Result<bool> {
        // Bound the observation by wall clock. The query itself is a bounded
        // Win32 wait, and an absence observed after the deadline is rejected:
        // a stalled query must never let a natural exit look like containment.
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let alive = process_exists(pid)?;
            if !alive {
                return Ok(Instant::now() <= deadline);
            }
            if Instant::now() >= deadline {
                return Ok(false);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn process_exists(pid: u32) -> io::Result<bool> {
        #[cfg(windows)]
        {
            use windows_sys::Win32::Foundation::{
                CloseHandle, ERROR_INVALID_PARAMETER, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT,
            };
            use windows_sys::Win32::System::Threading::{
                OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
            };
            // SAFETY: the process handle is closed on every path; a zero timeout
            // makes the wait a non-blocking liveness observation.
            unsafe {
                let process = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
                if process.is_null() {
                    let error = io::Error::last_os_error();
                    if error.raw_os_error() == Some(ERROR_INVALID_PARAMETER as i32) {
                        return Ok(false);
                    }
                    return Err(error);
                }
                let waited = WaitForSingleObject(process, 0);
                let _ = CloseHandle(process);
                match waited {
                    WAIT_OBJECT_0 => Ok(false),
                    WAIT_TIMEOUT => Ok(true),
                    WAIT_FAILED => Err(io::Error::last_os_error()),
                    _ => Err(io::Error::other(format!(
                        "unexpected process wait result {waited}"
                    ))),
                }
            }
        }
        #[cfg(not(windows))]
        {
            let _ = pid;
            Ok(false)
        }
    }

    #[test]
    fn property_scad_job_object_cleans_up_after_an_abnormal_controller_exit() {
        let dir = fixture_dir();
        let record = live_registry()
            .property
            .into_iter()
            .find(|record| record.id == "D-SCAD-3D")
            .expect("the scripted solid record must be registered");
        let log_path = dir.join("scad-job-object.log");
        let mut log = ExecutionLog {
            file: File::create(&log_path).expect("job object log"),
            summary: true,
        };
        let launcher =
            FixtureLauncher::new(std::slice::from_ref(&record), &["exit_with_descendant"]);
        let mut plan = make_plan(&record, &[], ChildKind::Run, &dir);
        // A generous deadline proves the cleanup comes from the job handle, not
        // from the deadline branch.
        plan.timeout_ms = Some(60_000);
        let output = log.child(&launcher, &plan).expect("supervised child runs");
        assert!(
            !output.timed_out,
            "the controller exits on its own; the deadline must not fire"
        );
        assert_ne!(output.status, Some(0), "the fixture exits abnormally");
        assert!(
            output.stdout_complete && output.stderr_complete,
            "job cleanup must close the descendant's inherited pipe handles"
        );
        let text = String::from_utf8_lossy(&output.stdout);
        let descendant = text
            .lines()
            .find_map(|line| line.strip_prefix("XTASK_PROPERTY_DESCENDANT_PID "))
            .and_then(|value| value.trim().parse::<u32>().ok())
            .expect("the fixture reports its descendant pid");
        #[cfg(windows)]
        {
            assert!(
                descendant_gone(descendant).expect("observe the descendant process"),
                "a descendant of an abnormally exited controller must be terminated"
            );
        }
        #[cfg(not(windows))]
        {
            println!("descendant liveness check is Windows-only; pid {descendant}");
        }
    }

    #[test]
    fn property_supervision_captures_finite_child_output_without_truncation() {
        let dir = fixture_dir();
        let record = live_registry()
            .property
            .into_iter()
            .find(|record| record.id == "D-SCAD-3D")
            .expect("the scripted solid record must be registered");
        let log_path = dir.join("finite-output.log");
        let mut log = ExecutionLog {
            file: File::create(&log_path).expect("finite output log"),
            summary: true,
        };
        let launcher = FixtureLauncher::new(std::slice::from_ref(&record), &["large_output"]);
        let mut plan = make_plan(&record, &[], ChildKind::Run, &dir);
        plan.timeout_ms = Some(60_000);
        let output = log.child(&launcher, &plan).expect("supervised child runs");
        assert_eq!(
            output.status,
            Some(0),
            "the finite-output fixture exits cleanly"
        );
        assert!(!output.timed_out, "the deadline must not fire");
        assert!(
            output.stdout_complete,
            "the full finite stdout must be captured"
        );
        assert!(
            output.stderr_complete,
            "the full finite stderr must be captured"
        );
        let text = String::from_utf8_lossy(&output.stdout);
        assert!(text.contains("BEGIN"), "the output head must be retained");
        assert!(
            text.contains("END"),
            "the output tail must be retained, got {} bytes",
            output.stdout.len()
        );
        assert!(
            output.stdout.len() >= 1024 * 1024,
            "the full payload must be captured, got {} bytes",
            output.stdout.len()
        );
    }

    #[test]
    fn property_pipe_drain_reports_terminal_read_errors_as_incomplete() {
        struct FailingPipe {
            sent_prefix: bool,
        }
        impl Read for FailingPipe {
            fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
                if !self.sent_prefix {
                    self.sent_prefix = true;
                    buffer[..4].copy_from_slice(b"head");
                    return Ok(4);
                }
                Err(io::Error::other("injected terminal read failure"))
            }
        }
        let (bytes, complete) = PipeDrain::spawn(Some(FailingPipe { sent_prefix: false })).finish();
        assert_eq!(bytes, b"head");
        assert!(
            !complete,
            "a terminal read error must be reported as incomplete capture"
        );
    }

    /// Deliberately widen the supervisor's spawn-to-assignment window.
    #[cfg(windows)]
    struct AssignDelay;

    #[cfg(windows)]
    impl AssignDelay {
        fn new(delay: Duration) -> Self {
            TEST_ASSIGN_DELAY_MS.with(|cell| cell.set(delay.as_millis() as u64));
            Self
        }
    }

    #[cfg(windows)]
    impl Drop for AssignDelay {
        fn drop(&mut self) {
            TEST_ASSIGN_DELAY_MS.with(|cell| cell.set(0));
        }
    }

    #[test]
    fn property_scad_job_object_contains_a_descendant_spawned_before_assignment() {
        #[cfg(not(windows))]
        {
            println!("pre-assignment containment is Windows-only");
        }
        #[cfg(windows)]
        {
            let dir = fixture_dir();
            let record = live_registry()
                .property
                .into_iter()
                .find(|record| record.id == "D-SCAD-3D")
                .expect("the scripted solid record must be registered");
            let log_path = dir.join("scad-job-object-pre-assignment.log");
            let mut log = ExecutionLog {
                file: File::create(&log_path).expect("pre-assignment log"),
                summary: true,
            };
            let launcher = FixtureLauncher::new(
                std::slice::from_ref(&record),
                &["exit_with_descendant_late"],
            );
            let mut plan = make_plan(&record, &[], ChildKind::Run, &dir);
            plan.timeout_ms = Some(60_000);
            // The child is suspended for this delay. Without that suspension the
            // fixture would spawn its descendant before job assignment, and the
            // descendant would outlive the controller's abnormal exit.
            let _delay = AssignDelay::new(Duration::from_millis(500));
            let output = log.child(&launcher, &plan).expect("supervised child runs");
            assert!(
                !output.timed_out,
                "the controller exits on its own; the deadline must not fire"
            );
            assert_ne!(output.status, Some(0), "the fixture exits abnormally");
            let text = String::from_utf8_lossy(&output.stdout);
            let descendant = text
                .lines()
                .find_map(|line| line.strip_prefix("XTASK_PROPERTY_DESCENDANT_PID "))
                .and_then(|value| value.trim().parse::<u32>().ok())
                .expect("the fixture reports its descendant pid");
            let gone = descendant_gone(descendant).expect("observe the descendant process");
            assert!(
                gone,
                "a descendant created after assignment must stay inside the job"
            );
        }
    }

    #[test]
    fn property_scad_supervision_covers_generation_shrink_and_descendants() {
        let dir = fixture_dir();
        let record = live_registry()
            .property
            .into_iter()
            .find(|record| record.id == "D-SCAD-3D")
            .expect("the scripted solid record must be registered");
        let log_path = dir.join("scad-supervision.log");
        let mut log = ExecutionLog {
            file: File::create(&log_path).expect("supervision log"),
            summary: true,
        };

        // A child that blocks before any callback and keeps a live descendant
        // must be ended by the whole-process deadline, and the descendant with
        // it: a controller-only kill would leave the worker running.
        let launcher =
            FixtureLauncher::new(std::slice::from_ref(&record), &["block_with_descendant"]);
        let mut plan = make_plan(&record, &[], ChildKind::Run, &dir);
        plan.timeout_ms = Some(1500);
        let start = Instant::now();
        let output = log.child(&launcher, &plan).expect("supervised child runs");
        assert!(
            output.timed_out,
            "the deadline must end a child blocked before its callback"
        );
        assert!(
            output.stdout_complete && output.stderr_complete,
            "the tree kill must close the descendant's inherited pipe handles"
        );
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "the deadline kill must not wait for the blocked fixture: {:?}",
            start.elapsed()
        );
        let text = String::from_utf8_lossy(&output.stdout);
        let descendant = text
            .lines()
            .find_map(|line| line.strip_prefix("XTASK_PROPERTY_DESCENDANT_PID "))
            .and_then(|value| value.trim().parse::<u32>().ok())
            .expect("the fixture reports its descendant pid");
        #[cfg(windows)]
        {
            assert!(
                descendant_gone(descendant).expect("observe the descendant process"),
                "descendant {descendant} survived the process-tree kill"
            );
        }
        #[cfg(not(windows))]
        {
            // `process_exists` cannot answer on this platform, so the descendant
            // check is not claimed here; the deadline assertion above still holds.
            println!("descendant liveness check is Windows-only; pid {descendant}");
        }
        let log_text = fs::read_to_string(&log_path).expect("supervision log text");
        assert!(
            log_text.contains("property_process_timeout"),
            "a deadline kill must be retained in the combined log:
{log_text}"
        );
        assert!(log_text.contains("timed_out: true"));

        // A real bounded adapter-generated property passes under the same
        // supervised path, so the test is not only a negative control.
        let selection = PropertySelection {
            tier: Tier::Routine,
            ids: vec!["D-SCAD-3D".to_owned()],
            seed: Some(7),
            with_scad: true,
        };
        let prepared = read_and_select(&root(), &selection).expect("select D-SCAD-3D");
        let real_log = dir.join("scad-supervision-real.log");
        let code = run(&root(), &prepared, &selection, &real_log, true);
        let real_text = fs::read_to_string(&real_log).expect("real supervision log");
        assert_eq!(
            code, 0,
            "a bounded adapter-generated property must pass under supervision:
{real_text}"
        );
        assert!(
            real_text.contains("status: Some(0) (timed_out: false)"),
            "the real property child must report a normal status:
{real_text}"
        );
        assert!(real_text.contains("timeout: Some("));
    }
}
