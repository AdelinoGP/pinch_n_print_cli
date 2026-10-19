//! The controlled compiler driver used by accelerated perimeter builds.
//!
//! The driver is intentionally kept in `xtask` rather than in a library crate.
//! Cargo invokes it as a compiler wrapper, so every invocation is a new xtask
//! process.  The small marker file below is consequently the session-wide
//! rejection latch.

use std::collections::HashSet;
use std::env;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) const ALLOWED_RELEASE: &str = "1.96.0";
pub(crate) const ALLOWED_COMMIT_HASH: &str = "ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96";
pub(crate) const ALLOWED_HOST: &str = "x86_64-pc-windows-msvc";
pub(crate) const ALLOWED_LLVM_VERSION: &str = "22.1.2";
pub(crate) const ALLOWED_WASM_TARGET: &str = "wasm32-unknown-unknown";
pub(crate) const RESERVED_CFG: &str = "pnp_perimeter_spatial_accelerated";

/// Environment variable used by the later build/test entry points to opt into
/// this policy.  With no value, the wrapper is an ordinary, non-mutating
/// compiler wrapper.
pub(crate) const ACCELERATED_ENV: &str = "PNP_ACCELERATED";
/// Optional profile selector.  `auto` (the default for an accelerated build)
/// only requires the optimization property that is observable in the rustc
/// argv; the explicit profiles additionally validate their profile-specific
/// values.
pub(crate) const ACCELERATED_MODE_ENV: &str = "PNP_ACCELERATED_MODE";

const DRIVER_DIR_NAME: &str = "accelerated-driver";
const LATCH_FILE_NAME: &str = "latch";
const RUSTC_PATH_FILE_NAME: &str = "rustc-path";
const SHIM_FILE_STEM: &str = "rustc-shim";
const SHIM_LAUNCHER_SOURCE_FILE_NAME: &str = "shim_launcher.rs";
pub(crate) const PNP_XTASK_EXE_ENV: &str = "PNP_XTASK_EXE";

const SHIM_LAUNCHER_SRC: &str = r#"use std::process::{exit, Command};

fn main() {
    let xtask = match std::env::var_os("PNP_XTASK_EXE") {
        Some(path) => path,
        None => {
            eprintln!("pnp rustc shim launcher: PNP_XTASK_EXE is not set");
            exit(1);
        }
    };

    let status = Command::new(xtask)
        .arg("rustc-shim")
        .args(std::env::args_os().skip(1))
        .status();
    match status {
        Ok(status) => exit(status.code().unwrap_or(1)),
        Err(error) => {
            eprintln!("pnp rustc shim launcher: failed to spawn xtask rustc-shim: {error}");
            exit(1);
        }
    }
}
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustcIdentity {
    pub(crate) release: String,
    pub(crate) commit_hash: String,
    pub(crate) host: String,
    pub(crate) llvm_version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AccelerationProfile {
    /// Infer whether a rustc invocation is a host or guest compilation from
    /// its target.  This is the default for the boolean acceleration switch.
    Auto,
    HostDebug,
    HostRelease,
    GuestRelease,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DriverMode {
    Ordinary,
    Accelerated(AccelerationProfile),
}

impl DriverMode {
    fn is_accelerated(self) -> bool {
        matches!(self, Self::Accelerated(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InvocationKind {
    Compilation,
    Probe,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InvocationInfo {
    pub(crate) kind: InvocationKind,
    pub(crate) target: String,
    pub(crate) crate_name: Option<String>,
    pub(crate) canonical_core: bool,
    pub(crate) inject_accelerated_cfg: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedInvocation {
    pub(crate) args: Vec<String>,
    pub(crate) info: InvocationInfo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PolicyError {
    reason: String,
}

impl PolicyError {
    fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl fmt::Display for PolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.reason)
    }
}

pub(crate) fn driver_dir(ws_root: &Path) -> PathBuf {
    ws_root.join("target").join(DRIVER_DIR_NAME)
}

/// Windows' `canonicalize` may return a verbatim path (`\\?\...`).  Cargo can
/// pass that spelling to a command successfully in some contexts but not in
/// all executable-launch contexts, so keep the wrapper-facing paths in the
/// ordinary drive-letter form.
fn strip_verbatim_prefix(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(&text))
}

pub(crate) fn latch_path(ws_root: &Path) -> PathBuf {
    driver_dir(ws_root).join(LATCH_FILE_NAME)
}

pub(crate) fn rustc_path_file(ws_root: &Path) -> PathBuf {
    driver_dir(ws_root).join(RUSTC_PATH_FILE_NAME)
}

#[allow(dead_code)]
pub(crate) fn shim_wrapper_path(ws_root: &Path) -> PathBuf {
    strip_verbatim_prefix(&driver_dir(ws_root).join(format!("{SHIM_FILE_STEM}.exe")))
}

pub(crate) fn is_latched(ws_root: &Path) -> bool {
    latch_path(ws_root).exists()
}

/// Remove a previous session's rejection marker.  Controlled entry points
/// call this before starting a new accelerated build; it is also the seam used
/// by the policy tests.
#[allow(dead_code)]
pub(crate) fn clear_latch(ws_root: &Path) -> io::Result<()> {
    match fs::remove_file(latch_path(ws_root)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

/// Persist a policy rejection.  This is deliberately separate from the child
/// compiler's exit status: a build script can swallow that status, but it
/// cannot make the marker disappear before the next compiler process starts.
pub(crate) fn latch_policy_rejection(ws_root: &Path, reason: &str) -> io::Result<()> {
    let dir = driver_dir(ws_root);
    fs::create_dir_all(&dir)?;
    fs::write(latch_path(ws_root), format!("policy rejection: {reason}\n"))
}

/// Parse and validate the exact `rustc -vV` identity required by the audited
/// arithmetic configuration.
pub(crate) fn parse_rustc_identity(output: &str) -> Result<RustcIdentity, PolicyError> {
    let mut release = None;
    let mut commit_hash = None;
    let mut host = None;
    let mut llvm_version = None;

    for line in output.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "release" => set_identity_field(&mut release, value, "release")?,
            "commit-hash" => set_identity_field(&mut commit_hash, value, "commit-hash")?,
            "host" => set_identity_field(&mut host, value, "host")?,
            "LLVM version" => set_identity_field(&mut llvm_version, value, "LLVM version")?,
            _ => {}
        }
    }

    let identity = RustcIdentity {
        release: release.ok_or_else(|| PolicyError::new("rustc -vV omitted release"))?,
        commit_hash: commit_hash
            .ok_or_else(|| PolicyError::new("rustc -vV omitted commit-hash"))?,
        host: host.ok_or_else(|| PolicyError::new("rustc -vV omitted host"))?,
        llvm_version: llvm_version
            .ok_or_else(|| PolicyError::new("rustc -vV omitted LLVM version"))?,
    };

    if identity.release != ALLOWED_RELEASE
        || identity.commit_hash != ALLOWED_COMMIT_HASH
        || identity.host != ALLOWED_HOST
        || identity.llvm_version != ALLOWED_LLVM_VERSION
    {
        return Err(PolicyError::new(format!(
            "unsupported rustc identity (release={}, commit-hash={}, host={}, LLVM version={})",
            identity.release, identity.commit_hash, identity.host, identity.llvm_version
        )));
    }

    Ok(identity)
}

fn set_identity_field(
    slot: &mut Option<String>,
    value: &str,
    field: &str,
) -> Result<(), PolicyError> {
    if slot.is_some() {
        return Err(PolicyError::new(format!(
            "rustc -vV repeated identity field '{field}'"
        )));
    }
    *slot = Some(value.to_string());
    Ok(())
}

pub(crate) fn validate_rustc_identity(output: &str) -> Result<(), PolicyError> {
    parse_rustc_identity(output).map(|_| ())
}

/// Convert the environment contract used by the later build plumbing into a
/// closed set of modes.  Unknown explicit values reject rather than silently
/// selecting ordinary compilation.
pub(crate) fn driver_mode_from_env() -> Result<DriverMode, PolicyError> {
    let accelerated = env::var(ACCELERATED_ENV).ok();
    let profile = env::var(ACCELERATED_MODE_ENV).ok();
    parse_driver_mode(accelerated.as_deref(), profile.as_deref())
}

pub(crate) fn parse_driver_mode(
    accelerated: Option<&str>,
    profile: Option<&str>,
) -> Result<DriverMode, PolicyError> {
    let accelerated = match accelerated.map(|value| value.trim().to_ascii_lowercase()) {
        None => false,
        Some(value) if matches!(value.as_str(), "1" | "true" | "yes" | "on") => true,
        Some(value) if matches!(value.as_str(), "0" | "false" | "no" | "off" | "ordinary") => false,
        Some(value) => {
            return Err(PolicyError::new(format!(
                "unsupported {ACCELERATED_ENV} value '{value}'"
            )));
        }
    };

    let Some(profile) = profile else {
        return Ok(if accelerated {
            DriverMode::Accelerated(AccelerationProfile::Auto)
        } else {
            DriverMode::Ordinary
        });
    };

    let profile = profile.trim().to_ascii_lowercase();
    let parsed = match profile.as_str() {
        "" | "ordinary" | "off" | "false" | "0" => DriverMode::Ordinary,
        "auto" | "accelerated" => DriverMode::Accelerated(AccelerationProfile::Auto),
        "host-debug" | "host_debug" | "debug" => {
            DriverMode::Accelerated(AccelerationProfile::HostDebug)
        }
        "host-release" | "host_release" => {
            DriverMode::Accelerated(AccelerationProfile::HostRelease)
        }
        "guest-release" | "guest_release" | "wasm-release" | "wasm_release" => {
            DriverMode::Accelerated(AccelerationProfile::GuestRelease)
        }
        _ => {
            return Err(PolicyError::new(format!(
                "unsupported {ACCELERATED_MODE_ENV} value '{profile}'"
            )));
        }
    };

    if accelerated != parsed.is_accelerated() {
        return Err(PolicyError::new(format!(
            "{ACCELERATED_ENV} and {ACCELERATED_MODE_ENV} disagree about acceleration"
        )));
    }
    Ok(parsed)
}

/// Validate actual rustc arguments without executing a compiler.  This is the
/// policy core used both by the process wrapper and by the deterministic unit
/// tests.
pub(crate) fn validate_rustc_invocation(
    rustc_vv: &str,
    args: &[String],
    mode: DriverMode,
) -> Result<InvocationInfo, PolicyError> {
    validate_rustc_identity(rustc_vv)?;

    let mut parser = ArgParser::default();
    parser.parse(args)?;

    let target = parser
        .target
        .clone()
        .unwrap_or_else(|| ALLOWED_HOST.to_string());
    if target != ALLOWED_HOST && target != ALLOWED_WASM_TARGET {
        return Err(PolicyError::new(format!(
            "unsupported rustc target '{target}'"
        )));
    }

    let kind = if parser.is_probe {
        InvocationKind::Probe
    } else {
        InvocationKind::Compilation
    };
    let canonical_core = parser.crate_name.as_deref() == Some("slicer_core")
        && (parser.is_test || parser.crate_types.is_empty() || parser.is_library_crate());
    let accelerated_core =
        canonical_core && mode.is_accelerated() && kind == InvocationKind::Compilation;

    validate_profile(
        mode,
        target.as_str(),
        kind,
        accelerated_core,
        &parser.codegen,
    )?;

    Ok(InvocationInfo {
        kind,
        target,
        crate_name: parser.crate_name,
        canonical_core,
        inject_accelerated_cfg: accelerated_core,
    })
}

/// Validate and, only for canonical accelerated core compilations, add the
/// reserved cfg.  The input vector is never modified in place.
pub(crate) fn prepare_rustc_invocation(
    rustc_vv: &str,
    args: &[String],
    mode: DriverMode,
) -> Result<PreparedInvocation, PolicyError> {
    let info = validate_rustc_invocation(rustc_vv, args, mode)?;
    let mut prepared = args.to_vec();
    if info.inject_accelerated_cfg {
        prepared.push("--cfg".to_string());
        prepared.push(RESERVED_CFG.to_string());
    }
    Ok(PreparedInvocation {
        args: prepared,
        info,
    })
}

#[derive(Default)]
struct ArgParser {
    target: Option<String>,
    crate_name: Option<String>,
    crate_types: Vec<String>,
    codegen: CodegenState,
    keyed_controls: HashSet<String>,
    cargo_error_format_json: bool,
    llvm_probe: bool,
    is_probe: bool,
    is_test: bool,
    source_is_stdin: bool,
    after_options: bool,
}

#[derive(Default)]
struct CodegenState {
    opt_level: Option<String>,
    debuginfo: Option<String>,
    debug_assertions: Option<bool>,
    strip: Option<String>,
    values: HashSet<String>,
}

impl ArgParser {
    fn parse(&mut self, args: &[String]) -> Result<(), PolicyError> {
        let mut index = 0;
        while index < args.len() {
            let token = &args[index];
            if token.starts_with('@') {
                return Err(PolicyError::new(format!(
                    "response files are not allowed ('{token}')"
                )));
            }

            if self.after_options {
                validate_path_value(token, "source")?;
                index += 1;
                continue;
            }
            if token == "--" {
                self.after_options = true;
                index += 1;
                continue;
            }

            if token == "-vV" || token == "-V" || token == "--version" {
                self.is_probe = true;
                index += 1;
                continue;
            }
            if token == "--verbose" || token == "-v" {
                index += 1;
                continue;
            }
            if token == "--explain" {
                let _ = next_value(args, &mut index, token)?;
                self.is_probe = true;
                index += 1;
                continue;
            }
            if token.starts_with("--print=") {
                let value = token.trim_start_matches("--print=");
                validate_nonempty(value, "--print")?;
                self.is_probe = true;
                index += 1;
                continue;
            }
            if token == "--print" {
                let value = next_value(args, &mut index, token)?;
                validate_nonempty(value, "--print")?;
                self.is_probe = true;
                index += 1;
                continue;
            }

            if token == "-C" || token.starts_with("-C=") || token.starts_with("-C") {
                let value = if token == "-C" {
                    next_value(args, &mut index, token)?.to_string()
                } else {
                    let value = token.strip_prefix("-C").unwrap_or_default();
                    value.strip_prefix('=').unwrap_or(value).to_string()
                };
                self.parse_codegen(&value, token)?;
                index += 1;
                continue;
            }
            if token == "--codegen" || token.starts_with("--codegen=") {
                let value = if token == "--codegen" {
                    next_value(args, &mut index, token)?.to_string()
                } else {
                    token
                        .strip_prefix("--codegen=")
                        .unwrap_or_default()
                        .to_string()
                };
                self.parse_codegen(&value, token)?;
                index += 1;
                continue;
            }
            if token == "-O" {
                self.codegen.insert("opt-level", Some("2"), token)?;
                index += 1;
                continue;
            }
            if token == "-g" {
                self.codegen.insert("debuginfo", Some("2"), token)?;
                index += 1;
                continue;
            }
            if token == "-Z" || token.starts_with("-Z") || token == "--unstable-options" {
                return Err(PolicyError::new(format!(
                    "unstable rustc option is not allowed ('{token}')"
                )));
            }
            if token == "--sysroot" || token.starts_with("--sysroot=") {
                let value = if token == "--sysroot" {
                    next_value(args, &mut index, token)?
                } else {
                    token.trim_start_matches("--sysroot=")
                };
                validate_nonempty(value, "--sysroot")?;
                self.insert_keyed("sysroot", token)?;
                return Err(PolicyError::new(
                    "sysroot overrides are not allowed in the controlled driver",
                ));
            }
            if token == "--codegen-backend" || token.starts_with("--codegen-backend=") {
                if token == "--codegen-backend" {
                    let _ = next_value(args, &mut index, token)?;
                }
                return Err(PolicyError::new(format!(
                    "codegen-backend overrides are not allowed ('{token}')"
                )));
            }
            if token == "--llvm-args" || token.starts_with("--llvm-args=") {
                if token == "--llvm-args" {
                    let _ = next_value(args, &mut index, token)?;
                }
                return Err(PolicyError::new(format!(
                    "raw LLVM arguments are not allowed ('{token}')"
                )));
            }

            if token == "--target" || token.starts_with("--target=") {
                let value = if token == "--target" {
                    next_value(args, &mut index, token)?
                } else {
                    token.trim_start_matches("--target=")
                };
                validate_nonempty(value, "--target")?;
                self.insert_keyed("target", token)?;
                self.target = Some(value.to_string());
                index += 1;
                continue;
            }
            if token == "--emit" || token.starts_with("--emit=") {
                let value = if token == "--emit" {
                    next_value(args, &mut index, token)?
                } else {
                    token.trim_start_matches("--emit=")
                };
                if validate_emit(value)? {
                    self.llvm_probe = true;
                }
                self.insert_keyed("emit", token)?;
                index += 1;
                continue;
            }

            if token == "--cfg" || token.starts_with("--cfg=") {
                let value = if token == "--cfg" {
                    next_value(args, &mut index, token)?
                } else {
                    token.trim_start_matches("--cfg=")
                };
                validate_cfg(value)?;
                index += 1;
                continue;
            }
            if token == "--check-cfg" || token.starts_with("--check-cfg=") {
                let value = if token == "--check-cfg" {
                    next_value(args, &mut index, token)?
                } else {
                    token.trim_start_matches("--check-cfg=")
                };
                validate_nonempty(value, "--check-cfg")?;
                index += 1;
                continue;
            }

            if token == "-L" || (token.starts_with("-L") && token.len() > 2) {
                let value = if token == "-L" {
                    next_value(args, &mut index, token)?
                } else {
                    token.strip_prefix("-L").unwrap_or_default()
                };
                validate_path_value(value, "-L")?;
                index += 1;
                continue;
            }
            if token == "--library-path" || token.starts_with("--library-path=") {
                let value = if token == "--library-path" {
                    next_value(args, &mut index, token)?
                } else {
                    token.trim_start_matches("--library-path=")
                };
                validate_path_value(value, "--library-path")?;
                index += 1;
                continue;
            }
            if token == "--extern" || token.starts_with("--extern=") {
                let value = if token == "--extern" {
                    next_value(args, &mut index, token)?
                } else {
                    token.trim_start_matches("--extern=")
                };
                validate_extern(value)?;
                index += 1;
                continue;
            }
            if token == "-l" || (token.starts_with("-l") && token.len() > 2) {
                let value = if token == "-l" {
                    next_value(args, &mut index, token)?
                } else {
                    token.strip_prefix("-l").unwrap_or_default()
                };
                validate_link(value)?;
                index += 1;
                continue;
            }

            if token == "--crate-name" || token.starts_with("--crate-name=") {
                let value = if token == "--crate-name" {
                    next_value(args, &mut index, token)?
                } else {
                    token.trim_start_matches("--crate-name=")
                };
                validate_nonempty(value, "--crate-name")?;
                self.crate_name = Some(value.to_string());
                index += 1;
                continue;
            }
            if token == "--crate-type" || token.starts_with("--crate-type=") {
                let value = if token == "--crate-type" {
                    next_value(args, &mut index, token)?
                } else {
                    token.trim_start_matches("--crate-type=")
                };
                validate_nonempty(value, "--crate-type")?;
                self.crate_types
                    .extend(value.split(',').map(ToString::to_string));
                index += 1;
                continue;
            }
            if token == "--edition" || token.starts_with("--edition=") {
                let value = if token == "--edition" {
                    next_value(args, &mut index, token)?
                } else {
                    token.trim_start_matches("--edition=")
                };
                if !matches!(value, "2015" | "2018" | "2021" | "2024") {
                    return Err(PolicyError::new(format!("unsupported edition '{value}'")));
                }
                index += 1;
                continue;
            }

            if is_lint_option(token) {
                if lint_value_is_separate(token) {
                    let value = next_value(args, &mut index, token)?;
                    validate_nonempty(value, "lint option")?;
                } else if let Some((_, value)) = token.split_once('=') {
                    validate_nonempty(value, "lint option")?;
                } else if token.starts_with('-') && token.len() > 2 {
                    validate_nonempty(&token[2..], "lint option")?;
                }
                index += 1;
                continue;
            }
            if token == "--cap-lints" || token.starts_with("--cap-lints=") {
                let value = if token == "--cap-lints" {
                    next_value(args, &mut index, token)?
                } else {
                    token.trim_start_matches("--cap-lints=")
                };
                if !matches!(value, "allow" | "warn" | "deny" | "forbid") {
                    return Err(PolicyError::new(format!(
                        "invalid --cap-lints value '{value}'"
                    )));
                }
                index += 1;
                continue;
            }

            if let Some((option, value)) = token.split_once('=') {
                if matches!(
                    option,
                    "--out-dir"
                        | "--error-format"
                        | "--json"
                        | "--color"
                        | "--diagnostic-width"
                        | "--remap-path-prefix"
                        | "--crate-version"
                ) {
                    validate_generic_value(value, option)?;
                    if option == "--error-format" && value == "json" {
                        self.cargo_error_format_json = true;
                    }
                    index += 1;
                    continue;
                }
            }
            if token == "--test" {
                self.is_test = true;
                index += 1;
                continue;
            }
            if has_separate_value(token) {
                let value = next_value(args, &mut index, token)?;
                validate_generic_value(value, token)?;
                if token == "--error-format" && value == "json" {
                    self.cargo_error_format_json = true;
                }
                index += 1;
                continue;
            }
            if token == "-" {
                // Cargo uses stdin as the source operand for its target
                // information probe (`rustc - --print=...`).
                validate_path_value(token, "source")?;
                self.source_is_stdin = true;
                index += 1;
                continue;
            }
            if is_bare_allowed_option(token) {
                index += 1;
                continue;
            }
            if token.starts_with('-') {
                return Err(PolicyError::new(format!(
                    "unsupported rustc option '{token}'"
                )));
            }

            validate_path_value(token, "source")?;
            index += 1;
        }
        if self.llvm_probe {
            if !self.source_is_stdin {
                return Err(PolicyError::new(
                    "LLVM emission is only allowed for stdin compiler probes",
                ));
            }
            self.is_probe = true;
        }
        // Build-script compiler probes do not carry Cargo's unit-compilation
        // markers.  Their target/profile arguments still went through the
        // structural grammar above, but they must not be held to a release
        // profile that Cargo never supplied.  A single marker is sufficient:
        // Cargo unit invocations use `--error-format=json`, `-C metadata`,
        // and/or `-C extra-filename`.
        self.is_probe = !self.has_cargo_unit_signature();
        Ok(())
    }

    fn has_cargo_unit_signature(&self) -> bool {
        self.cargo_error_format_json
            || self.codegen.values.contains("metadata")
            || self.codegen.values.contains("extra-filename")
    }

    fn is_library_crate(&self) -> bool {
        self.crate_types.iter().any(|crate_type| {
            matches!(
                crate_type.as_str(),
                "lib" | "rlib" | "dylib" | "cdylib" | "staticlib" | "proc-macro"
            )
        }) && !self
            .crate_types
            .iter()
            .any(|crate_type| crate_type == "bin")
    }

    fn insert_keyed(&mut self, key: &str, token: &str) -> Result<(), PolicyError> {
        if !self.keyed_controls.insert(key.to_string()) {
            return Err(PolicyError::new(format!(
                "duplicate keyed rustc control '{key}' (at '{token}')"
            )));
        }
        Ok(())
    }

    fn parse_codegen(&mut self, raw: &str, token: &str) -> Result<(), PolicyError> {
        let raw = raw.trim();
        validate_nonempty(raw, "-C")?;
        let (key, value) = match raw.split_once('=') {
            Some((key, value)) => (key.trim(), Some(value.trim())),
            None => (raw, None),
        };
        validate_nonempty(key, "-C key")?;
        self.codegen.insert(key, value, token)
    }
}

impl CodegenState {
    fn insert(&mut self, key: &str, value: Option<&str>, token: &str) -> Result<(), PolicyError> {
        if !self.values.insert(key.to_string()) {
            return Err(PolicyError::new(format!(
                "duplicate keyed codegen control '{key}' (at '{token}')"
            )));
        }

        match key {
            "opt-level" => {
                let value =
                    value.ok_or_else(|| PolicyError::new("-C opt-level requires a value"))?;
                if !matches!(value, "0" | "1" | "2" | "3" | "s" | "z") {
                    return Err(PolicyError::new(format!(
                        "invalid -C opt-level value '{value}'"
                    )));
                }
                self.opt_level = Some(value.to_string());
            }
            "debuginfo" => {
                let value =
                    value.ok_or_else(|| PolicyError::new("-C debuginfo requires a value"))?;
                if !matches!(value, "0" | "1" | "2" | "none" | "limited" | "full") {
                    return Err(PolicyError::new(format!(
                        "invalid -C debuginfo value '{value}'"
                    )));
                }
                self.debuginfo = Some(value.to_string());
            }
            "debug-assertions" => {
                let value = value
                    .ok_or_else(|| PolicyError::new("-C debug-assertions requires a value"))?;
                self.debug_assertions = Some(parse_bool_codegen(value, key)?);
            }
            "overflow-checks" => {
                let value =
                    value.ok_or_else(|| PolicyError::new("-C overflow-checks requires a value"))?;
                let _ = parse_bool_codegen(value, key)?;
            }
            "panic" => {
                let value = value.ok_or_else(|| PolicyError::new("-C panic requires a value"))?;
                if !matches!(value, "abort" | "unwind") {
                    return Err(PolicyError::new(format!(
                        "invalid -C panic value '{value}'"
                    )));
                }
            }
            "lto" => {
                let value = value.ok_or_else(|| PolicyError::new("-C lto requires a value"))?;
                if !matches!(value, "off" | "thin" | "fat" | "true" | "false") {
                    return Err(PolicyError::new(format!("invalid -C lto value '{value}'")));
                }
            }
            "embed-bitcode" => {
                let value =
                    value.ok_or_else(|| PolicyError::new("-C embed-bitcode requires a value"))?;
                let _ = parse_bool_codegen(value, key)?;
            }
            "strip" => {
                let value = value.ok_or_else(|| PolicyError::new("-C strip requires a value"))?;
                if !matches!(value, "none" | "debuginfo" | "symbols") {
                    return Err(PolicyError::new(format!(
                        "invalid -C strip value '{value}'"
                    )));
                }
                self.strip = Some(value.to_string());
            }
            "codegen-units" => {
                let value =
                    value.ok_or_else(|| PolicyError::new("-C codegen-units requires a value"))?;
                if value
                    .parse::<u32>()
                    .ok()
                    .filter(|units| *units > 0)
                    .is_none()
                {
                    return Err(PolicyError::new(format!(
                        "invalid -C codegen-units value '{value}'"
                    )));
                }
            }
            "incremental" => {
                let value =
                    value.ok_or_else(|| PolicyError::new("-C incremental requires a path"))?;
                validate_path_value(value, "-C incremental")?;
            }
            "metadata" | "extra-filename" => {
                validate_nonempty(value.unwrap_or_default(), key)?;
            }
            "linker-plugin-lto" | "prefer-dynamic" => {
                if value.is_some() {
                    return Err(PolicyError::new(format!(
                        "{key} must be a bare controlled codegen flag"
                    )));
                }
            }
            "rpath" | "force-unwind-tables" => {
                if let Some(value) = value {
                    let _ = parse_bool_codegen(value, key)?;
                }
            }
            "split-debuginfo" => {
                let value =
                    value.ok_or_else(|| PolicyError::new("-C split-debuginfo requires a value"))?;
                if !matches!(value, "off" | "packed" | "unpacked") {
                    return Err(PolicyError::new(format!(
                        "invalid -C split-debuginfo value '{value}'"
                    )));
                }
            }
            "symbol-mangling-version" => {
                let value = value.ok_or_else(|| {
                    PolicyError::new("-C symbol-mangling-version requires a value")
                })?;
                if !matches!(value, "legacy" | "v0") {
                    return Err(PolicyError::new(format!(
                        "invalid -C symbol-mangling-version value '{value}'"
                    )));
                }
            }
            "llvm-args"
            | "target-cpu"
            | "target-feature"
            | "codegen-backend"
            | "sysroot"
            | "linker"
            | "link-arg"
            | "link-args"
            | "link-self-contained"
            | "linker-flavor"
            | "passes" => {
                return Err(PolicyError::new(format!(
                    "unaudited or unsafe codegen option '-C {key}' is not allowed"
                )));
            }
            _ => {
                return Err(PolicyError::new(format!(
                    "unsupported codegen option '-C {key}'"
                )));
            }
        }
        Ok(())
    }
}

fn validate_profile(
    mode: DriverMode,
    target: &str,
    kind: InvocationKind,
    accelerated_core: bool,
    codegen: &CodegenState,
) -> Result<(), PolicyError> {
    if accelerated_core {
        if codegen.opt_level.as_deref() != Some("3") {
            return Err(PolicyError::new(
                "accelerated slicer-core compilation must use -C opt-level=3",
            ));
        }

        match mode {
            DriverMode::Accelerated(AccelerationProfile::HostDebug) => {
                if codegen.debuginfo.as_deref() != Some("2") {
                    return Err(PolicyError::new(
                        "accelerated host-debug slicer-core must use -C debuginfo=2",
                    ));
                }
                if codegen.debug_assertions != Some(true) {
                    return Err(PolicyError::new(
                        "accelerated host-debug slicer-core must enable debug assertions",
                    ));
                }
            }
            DriverMode::Accelerated(AccelerationProfile::HostRelease)
            | DriverMode::Accelerated(AccelerationProfile::GuestRelease)
            | DriverMode::Accelerated(AccelerationProfile::Auto)
            | DriverMode::Ordinary => {}
        }
    }

    if kind == InvocationKind::Compilation {
        match mode {
            DriverMode::Accelerated(AccelerationProfile::GuestRelease)
                if target == ALLOWED_WASM_TARGET && codegen.opt_level.as_deref() != Some("3") =>
            {
                return Err(PolicyError::new(
                    "accelerated guest-release compilation must use -C opt-level=3",
                ));
            }
            _ => {}
        }

        if let DriverMode::Accelerated(profile) = mode {
            match (profile, target) {
                (AccelerationProfile::HostRelease, ALLOWED_HOST) => {
                    if let Some(strip) = codegen.strip.as_deref() {
                        if strip != "symbols" {
                            return Err(PolicyError::new(
                                "accelerated profile requires -C strip=symbols",
                            ));
                        }
                    }
                }
                // Root-member guests inherit `strip = true`, which Cargo
                // lowers to `symbols`; own-workspace guests use Cargo's
                // release default, `debuginfo`.
                (AccelerationProfile::GuestRelease, ALLOWED_WASM_TARGET)
                    if !matches!(codegen.strip.as_deref(), Some("symbols" | "debuginfo")) =>
                {
                    return Err(PolicyError::new(
                        "accelerated guest-release profile requires -C strip=symbols or -C strip=debuginfo",
                    ));
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn parse_bool_codegen(value: &str, key: &str) -> Result<bool, PolicyError> {
    match value {
        "on" | "yes" | "true" => Ok(true),
        "off" | "no" | "false" => Ok(false),
        _ => Err(PolicyError::new(format!(
            "invalid -C {key} value '{value}'"
        ))),
    }
}

fn next_value<'a>(
    args: &'a [String],
    index: &mut usize,
    option: &str,
) -> Result<&'a str, PolicyError> {
    let value = args.get(*index + 1).ok_or_else(|| {
        PolicyError::new(format!(
            "rustc option '{option}' requires a following value"
        ))
    })?;
    if value.starts_with('@') {
        return Err(PolicyError::new(format!(
            "response files are not allowed ('{value}')"
        )));
    }
    *index += 1;
    Ok(value)
}

fn validate_nonempty(value: &str, option: &str) -> Result<(), PolicyError> {
    if value.is_empty() || value.contains('\0') {
        return Err(PolicyError::new(format!(
            "invalid empty value for {option}"
        )));
    }
    Ok(())
}

fn validate_path_value(value: &str, option: &str) -> Result<(), PolicyError> {
    validate_nonempty(value, option)?;
    if value.starts_with('@') || value.contains('\0') {
        return Err(PolicyError::new(format!(
            "response-file or NUL path is not allowed for {option}"
        )));
    }
    Ok(())
}

fn validate_cfg(value: &str) -> Result<(), PolicyError> {
    validate_nonempty(value, "--cfg")?;
    let key = value.split_once('=').map_or(value, |(key, _)| key).trim();
    if key == RESERVED_CFG {
        return Err(PolicyError::new(
            "the reserved acceleration cfg may only be injected by the driver",
        ));
    }
    Ok(())
}

fn validate_extern(value: &str) -> Result<(), PolicyError> {
    validate_nonempty(value, "--extern")?;
    let (_, path) = value.split_once('=').unwrap_or((value, ""));
    if !path.is_empty() {
        validate_path_value(path, "--extern")?;
    }
    Ok(())
}

fn validate_emit(value: &str) -> Result<bool, PolicyError> {
    validate_nonempty(value, "--emit")?;
    let mut has_llvm_output = false;
    for output in value.split(',') {
        let output = output.trim();
        validate_nonempty(output, "--emit")?;
        if matches!(output, "llvm-bc" | "llvm-ir" | "llvm-bitcode") {
            has_llvm_output = true;
        }
    }
    if has_llvm_output
        && value
            .split(',')
            .map(str::trim)
            .any(|output| !matches!(output, "llvm-bc" | "llvm-ir" | "llvm-bitcode"))
    {
        return Err(PolicyError::new(
            "LLVM emission cannot be combined with another --emit output",
        ));
    }
    Ok(has_llvm_output)
}

fn validate_link(value: &str) -> Result<(), PolicyError> {
    validate_nonempty(value, "-l")?;
    let (kind, name) = value
        .split_once('=')
        .map_or((None, value), |(kind, name)| (Some(kind), name));
    if let Some(kind) = kind {
        if !matches!(
            kind,
            "static" | "dylib" | "framework" | "raw-dylib" | "static-nobundle" | "verbatim"
        ) {
            return Err(PolicyError::new(format!(
                "invalid structural -l kind '{kind}'"
            )));
        }
    }
    if name.is_empty() || name.contains(' ') || name.starts_with('-') {
        return Err(PolicyError::new(format!(
            "invalid structural -l value '{value}'"
        )));
    }
    Ok(())
}

fn is_lint_option(token: &str) -> bool {
    ["--allow", "--warn", "--deny", "--forbid", "--force-warn"]
        .iter()
        .any(|prefix| token == *prefix || token.starts_with(&format!("{prefix}=")))
        || (token.len() >= 2
            && matches!(token.as_bytes()[0], b'-')
            && matches!(token.as_bytes()[1], b'A' | b'W' | b'D' | b'F'))
}

fn lint_value_is_separate(token: &str) -> bool {
    matches!(
        token,
        "--allow" | "--warn" | "--deny" | "--forbid" | "--force-warn" | "-A" | "-W" | "-D" | "-F"
    )
}

fn has_separate_value(token: &str) -> bool {
    matches!(
        token,
        "--out-dir"
            | "-o"
            | "--error-format"
            | "--json"
            | "--color"
            | "--diagnostic-width"
            | "--remap-path-prefix"
            | "--crate-version"
    )
}

fn is_bare_allowed_option(token: &str) -> bool {
    matches!(
        token,
        "--test"
            | "--bench"
            | "--verbose"
            | "--color=auto"
            | "--color=always"
            | "--color=never"
            | "--error-format=human"
            | "--error-format=json"
            | "--json=diagnostic-rendered-ansi"
            | "--json=artifacts"
            | "--remap-path-scope"
    )
}

fn validate_generic_value(value: &str, option: &str) -> Result<(), PolicyError> {
    validate_nonempty(value, option)?;
    if value.starts_with('@') {
        return Err(PolicyError::new(format!(
            "response files are not allowed ('{value}')"
        )));
    }
    Ok(())
}

fn resolve_program(name: &str) -> io::Result<PathBuf> {
    let path = env::var_os("PATH").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "PATH is not set while resolving rustc",
        )
    })?;
    let candidates = env::split_paths(&path).flat_map(|directory| {
        let plain = directory.join(name);
        let with_suffix = if cfg!(windows) {
            directory.join(format!("{name}.exe"))
        } else {
            plain.clone()
        };
        [plain, with_suffix]
    });
    for candidate in candidates {
        if candidate.is_file() {
            // Do not canonicalize this path: on Windows the rustc launcher in
            // Cargo's bin directory is commonly a symlink to rustup.exe.  The
            // rustc.exe spelling selects rustup's compiler-proxy behavior,
            // while the canonical rustup.exe spelling invokes rustup itself.
            return Ok(strip_verbatim_prefix(&candidate));
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("could not resolve '{name}' through PATH"),
    ))
}

fn resolve_real_rustc(ws_root: &Path) -> io::Result<PathBuf> {
    if let Some(configured) = env::var_os("RUSTC") {
        let configured = strip_verbatim_prefix(&PathBuf::from(configured));
        let candidate = if configured.is_absolute() {
            configured
        } else {
            resolve_program(configured.to_string_lossy().as_ref())?
        };
        if !candidate.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("configured rustc is not a file: {}", candidate.display()),
            ));
        }
        let current = env::current_exe()
            .ok()
            .and_then(|path| fs::canonicalize(path).ok())
            .map(|path| strip_verbatim_prefix(&path));
        if current.as_ref() != Some(&candidate)
            && candidate != shim_wrapper_path(ws_root)
            && !candidate
                .file_stem()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case(SHIM_FILE_STEM))
        {
            return Ok(candidate);
        }
    }

    resolve_program("rustc")
}

fn read_saved_real_rustc(ws_root: &Path) -> io::Result<PathBuf> {
    let contents = fs::read_to_string(rustc_path_file(ws_root))?;
    let path = strip_verbatim_prefix(&PathBuf::from(contents.trim()));
    if !path.is_absolute() || !path.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "saved real rustc path is not an absolute file",
        ));
    }
    Ok(path)
}

#[allow(dead_code)]
fn write_saved_rustc_path(ws_root: &Path, path: &Path) -> io::Result<()> {
    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "real rustc path must be absolute",
        ));
    }
    fs::create_dir_all(driver_dir(ws_root))?;
    let path = strip_verbatim_prefix(path);
    fs::write(rustc_path_file(ws_root), format!("{}\n", path.display()))
}

/// Create the executable wrapper consumed by Cargo's `RUSTC` setting and save
/// the real compiler path before returning it.  The returned path is absolute
/// whenever the workspace root is absolute (as it is for production xtask
/// entry points).
#[allow(dead_code)]
pub(crate) fn create_shim_wrapper(ws_root: &Path) -> io::Result<PathBuf> {
    let root = if ws_root.is_absolute() {
        ws_root.to_path_buf()
    } else {
        fs::canonicalize(ws_root)?
    };
    let rustc = resolve_real_rustc(&root)?;
    let state_dir = driver_dir(&root);
    fs::create_dir_all(&state_dir)?;
    write_saved_rustc_path(&root, &rustc)?;

    let wrapper = shim_wrapper_path(&root);
    let source = state_dir.join(SHIM_LAUNCHER_SOURCE_FILE_NAME);
    fs::write(&source, SHIM_LAUNCHER_SRC)?;

    let status = Command::new(&rustc)
        .args(["--edition", "2021", "--crate-name", "pnp_rustc_shim", "-O"])
        .arg(&source)
        .arg("-o")
        .arg(&wrapper)
        .status()?;
    if !status.success() {
        return Err(io::Error::other(format!(
            "failed to compile rustc shim launcher with {}: {status}",
            rustc.display()
        )));
    }

    Ok(wrapper)
}

/// Entry point for `cargo xtask rustc-shim ...`.
pub(crate) fn run_shim_mode(args: &[String]) -> i32 {
    let ws_root = crate::build_guests::workspace_root();
    if is_latched(&ws_root) {
        eprintln!("xtask rustc-shim: accelerated-driver session is latched; refusing invocation");
        return 1;
    }

    let mode = match driver_mode_from_env() {
        Ok(mode) => mode,
        Err(error) => return reject(&ws_root, error),
    };
    let rustc = match read_saved_real_rustc(&ws_root) {
        Ok(path) => path,
        Err(error) => {
            return reject(
                &ws_root,
                PolicyError::new(format!("could not resolve saved real rustc: {error}")),
            );
        }
    };
    if !rustc.is_absolute() || !rustc.is_file() {
        return reject(
            &ws_root,
            PolicyError::new("saved real rustc path is not an absolute file"),
        );
    }

    let version = match Command::new(&rustc).arg("-vV").output() {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).into_owned()
        }
        Ok(output) => {
            return reject(
                &ws_root,
                PolicyError::new(format!(
                    "real rustc -vV probe failed with {}",
                    output.status
                )),
            );
        }
        Err(error) => {
            return reject(
                &ws_root,
                PolicyError::new(format!("failed to execute real rustc -vV: {error}")),
            );
        }
    };

    let prepared = match prepare_rustc_invocation(&version, args, mode) {
        Ok(prepared) => prepared,
        Err(error) => return reject(&ws_root, error),
    };

    let status = Command::new(&rustc)
        .args(&prepared.args)
        .env_remove("RUSTC_WRAPPER")
        .env_remove("RUSTC_WORKSPACE_WRAPPER")
        .status();
    match status {
        Ok(status) => status.code().unwrap_or(1),
        Err(error) => {
            eprintln!("xtask rustc-shim: failed to execute real rustc: {error}");
            1
        }
    }
}

fn reject(ws_root: &Path, error: PolicyError) -> i32 {
    eprintln!("xtask rustc-shim: policy rejection: {error}");
    if let Err(latch_error) = latch_policy_rejection(ws_root, &error.to_string()) {
        eprintln!("xtask rustc-shim: failed to persist rejection latch: {latch_error}");
    }
    1
}
