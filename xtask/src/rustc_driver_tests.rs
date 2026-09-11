use super::build_guests::enforce_accelerated_latch_gate;
use super::rustc_driver::{
    clear_latch, is_latched, latch_path, latch_policy_rejection, parse_rustc_identity,
    prepare_rustc_invocation, shim_wrapper_path, validate_rustc_invocation, AccelerationProfile,
    DriverMode, InvocationKind, ALLOWED_COMMIT_HASH, ALLOWED_HOST, ALLOWED_LLVM_VERSION,
    ALLOWED_RELEASE, ALLOWED_WASM_TARGET, RESERVED_CFG,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const REAL_RUSTC_VV: &str = "rustc 1.96.0 (ac68faa20 2026-05-25)\nbinary: rustc\ncommit-hash: ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96\ncommit-date: 2026-05-25\nhost: x86_64-pc-windows-msvc\nrelease: 1.96.0\nLLVM version: 22.1.2\n";
const AUDITED_RUSTC_VV: &str = REAL_RUSTC_VV;

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn cargo_unit_signature() -> Vec<String> {
    args(&[
        "--error-format=json",
        "-C",
        "metadata=unit-test",
        "-C",
        "extra-filename=-unit-test",
    ])
}

fn test_root(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before Unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "pnp-rustc-driver-{label}-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create test workspace");
    root
}

fn remove_test_root(root: &PathBuf) {
    fs::remove_dir_all(root).expect("remove test workspace");
}

#[test]
fn shim_wrapper_is_a_native_executable_path() {
    let root = test_root("native-launcher-path");
    let wrapper = shim_wrapper_path(&root);
    assert_eq!(
        wrapper.extension().and_then(|value| value.to_str()),
        Some("exe")
    );
    assert!(wrapper.ends_with(PathBuf::from("target/accelerated-driver/rustc-shim.exe")));
    remove_test_root(&root);
}

#[test]
fn exact_real_rustc_probe_is_accepted_and_matches_allowlist() {
    let identity = parse_rustc_identity(REAL_RUSTC_VV).expect("real rustc identity parses");
    assert_eq!(identity.release, ALLOWED_RELEASE);
    assert_eq!(identity.commit_hash, ALLOWED_COMMIT_HASH);
    assert_eq!(identity.host, ALLOWED_HOST);
    assert_eq!(identity.llvm_version, ALLOWED_LLVM_VERSION);

    let probe = validate_rustc_invocation(REAL_RUSTC_VV, &args(&["-vV"]), DriverMode::Ordinary)
        .expect("real rustc version probe is accepted");
    assert_eq!(probe.kind, InvocationKind::Probe);
    assert_eq!(probe.target, ALLOWED_HOST);
}

#[test]
fn cargo_target_information_probe_accepts_stdin_source_operand() {
    let invocation = args(&[
        "-",
        "--crate-name",
        "___",
        "--print=file-names",
        "--crate-type",
        "bin",
        "--crate-type",
        "rlib",
        "--crate-type",
        "dylib",
        "--crate-type",
        "cdylib",
        "--crate-type",
        "staticlib",
        "--crate-type",
        "proc-macro",
        "--print=sysroot",
        "--print=split-debuginfo",
        "--print=crate-name",
        "--print=cfg",
        "-Wwarnings",
    ]);
    let probe = validate_rustc_invocation(REAL_RUSTC_VV, &invocation, DriverMode::Ordinary)
        .expect("Cargo target-information probe is accepted");
    assert_eq!(probe.kind, InvocationKind::Probe);
}

#[test]
fn autocfg_llvm_probe_is_accepted_only_with_stdin_source() {
    let probe = args(&[
        "--crate-name",
        "autocfg_probe",
        "--crate-type=lib",
        "--out-dir",
        "target/autocfg-probe",
        "--emit=llvm-ir",
        "-",
    ]);
    let info = validate_rustc_invocation(REAL_RUSTC_VV, &probe, DriverMode::Ordinary)
        .expect("autocfg's stdin LLVM probe is accepted");
    assert_eq!(info.kind, InvocationKind::Probe);

    let non_probe = args(&[
        "--crate-name",
        "autocfg_probe",
        "--crate-type=lib",
        "--emit=llvm-ir",
        "src/lib.rs",
    ]);
    assert!(
        validate_rustc_invocation(REAL_RUSTC_VV, &non_probe, DriverMode::Ordinary).is_err(),
        "LLVM emission for a source-file compilation must remain rejected"
    );
}

#[test]
fn accelerated_guest_release_accepts_workspace_strip_profile() {
    let mut invocation = args(&[
        "--target",
        ALLOWED_WASM_TARGET,
        "--crate-name",
        "guest_dependency",
        "--crate-type",
        "lib",
        "--emit=dep-info,metadata,link",
        "-C",
        "opt-level=3",
        "-C",
        "strip=symbols",
        "src/lib.rs",
    ]);
    invocation.extend(cargo_unit_signature());
    let info = validate_rustc_invocation(
        REAL_RUSTC_VV,
        &invocation,
        DriverMode::Accelerated(AccelerationProfile::GuestRelease),
    )
    .expect("workspace guest release profile is accepted");
    assert_eq!(info.kind, InvocationKind::Compilation);
}

#[test]
fn accelerated_guest_release_accepts_own_workspace_debuginfo_strip() {
    let mut invocation = args(&[
        "--target",
        ALLOWED_WASM_TARGET,
        "--crate-name",
        "guest_dependency",
        "--crate-type",
        "lib",
        "--emit=dep-info,metadata,link",
        "-C",
        "opt-level=3",
        "-C",
        "strip=debuginfo",
        "src/lib.rs",
    ]);
    invocation.extend(cargo_unit_signature());
    validate_rustc_invocation(
        REAL_RUSTC_VV,
        &invocation,
        DriverMode::Accelerated(AccelerationProfile::GuestRelease),
    )
    .expect("own-workspace guest release profile is accepted");
}

#[test]
fn accelerated_guest_release_rejects_none_strip() {
    let mut invocation = args(&[
        "--target",
        ALLOWED_WASM_TARGET,
        "--crate-name",
        "guest_dependency",
        "--crate-type",
        "lib",
        "--emit=dep-info,metadata,link",
        "-C",
        "opt-level=3",
        "-C",
        "strip=none",
        "src/lib.rs",
    ]);
    invocation.extend(cargo_unit_signature());
    let error = validate_rustc_invocation(
        REAL_RUSTC_VV,
        &invocation,
        DriverMode::Accelerated(AccelerationProfile::GuestRelease),
    )
    .expect_err("guest release must reject unstripped output");
    assert!(error.to_string().contains("strip=symbols"));
    assert!(error.to_string().contains("strip=debuginfo"));
}

#[test]
fn accelerated_guest_release_rejects_missing_strip() {
    let mut invocation = args(&[
        "--target",
        ALLOWED_WASM_TARGET,
        "--crate-name",
        "guest_dependency",
        "--crate-type",
        "lib",
        "--emit=dep-info,metadata,link",
        "-C",
        "opt-level=3",
        "src/lib.rs",
    ]);
    invocation.extend(cargo_unit_signature());
    let error = validate_rustc_invocation(
        REAL_RUSTC_VV,
        &invocation,
        DriverMode::Accelerated(AccelerationProfile::GuestRelease),
    )
    .expect_err("guest release must require Cargo's strip setting");
    assert!(error.to_string().contains("strip=symbols"));
    assert!(error.to_string().contains("strip=debuginfo"));
}

#[test]
fn accelerated_anyhow_build_script_probe_is_accepted_without_latch_or_cfg() {
    let root = test_root("anyhow-probe");
    let invocation = args(&[
        "--cfg=anyhow_build_probe",
        "--edition=2018",
        "--crate-name=anyhow",
        "--crate-type=lib",
        "--cap-lints=allow",
        "--emit=dep-info,metadata",
        "--out-dir",
        "target/anyhow-probe",
        "src/nightly.rs",
        "--target",
        ALLOWED_WASM_TARGET,
    ]);
    assert!(!is_latched(&root));
    let prepared = prepare_rustc_invocation(
        REAL_RUSTC_VV,
        &invocation,
        DriverMode::Accelerated(AccelerationProfile::GuestRelease),
    )
    .expect("anyhow's build-script probe is structurally valid");
    assert_eq!(prepared.info.kind, InvocationKind::Probe);
    assert!(!prepared.info.inject_accelerated_cfg);
    assert_eq!(prepared.args, invocation);
    assert!(!is_latched(&root));
    remove_test_root(&root);
}

#[test]
fn accelerated_heapless_build_script_probe_is_accepted_without_latch_or_cfg() {
    let root = test_root("heapless-probe");
    let invocation = args(&[
        "--edition=2018",
        "--crate-name=probe",
        "--crate-type=lib",
        "--out-dir",
        "target/heapless-probe",
        "target/heapless-probe/probe.rs",
        "--target",
        ALLOWED_WASM_TARGET,
    ]);
    assert!(!is_latched(&root));
    let prepared = prepare_rustc_invocation(
        REAL_RUSTC_VV,
        &invocation,
        DriverMode::Accelerated(AccelerationProfile::GuestRelease),
    )
    .expect("heapless's build-script probe is structurally valid");
    assert_eq!(prepared.info.kind, InvocationKind::Probe);
    assert!(!prepared.info.inject_accelerated_cfg);
    assert_eq!(prepared.args, invocation);
    assert!(!is_latched(&root));
    remove_test_root(&root);
}

#[test]
fn accelerated_structurally_invalid_probe_rejects_and_latches() {
    let root = test_root("invalid-probe");
    let invocation = args(&[
        "--edition=2018",
        "--crate-name=probe",
        "--crate-type=lib",
        "--out-dir",
        "target/probe",
        "target/probe.rs",
        "--target",
        ALLOWED_WASM_TARGET,
        "-C",
        "llvm-args=--unsafe-pass",
    ]);
    let error = validate_rustc_invocation(
        REAL_RUSTC_VV,
        &invocation,
        DriverMode::Accelerated(AccelerationProfile::GuestRelease),
    )
    .expect_err("structurally invalid probes must reject");
    assert!(error.to_string().contains("llvm-args"));
    assert!(!is_latched(&root));
    latch_policy_rejection(&root, &error.to_string()).expect("persist probe rejection latch");
    assert!(is_latched(&root));
    clear_latch(&root).expect("clear test latch");
    remove_test_root(&root);
}

#[test]
fn accelerated_policy_rejection_latch_accepts_audited_identity_and_targets() {
    let identity = parse_rustc_identity(AUDITED_RUSTC_VV).expect("audited identity parses");
    assert_eq!(identity.release, ALLOWED_RELEASE);
    assert_eq!(identity.commit_hash, ALLOWED_COMMIT_HASH);
    assert_eq!(identity.host, ALLOWED_HOST);
    assert_eq!(identity.llvm_version, ALLOWED_LLVM_VERSION);

    for target in [None, Some(ALLOWED_HOST), Some(ALLOWED_WASM_TARGET)] {
        let mut invocation = args(&["--crate-name", "helper", "--emit=metadata"]);
        invocation.extend(cargo_unit_signature());
        if let Some(target) = target {
            invocation.extend(args(&["--target", target]));
        }
        let prepared = prepare_rustc_invocation(
            AUDITED_RUSTC_VV,
            &invocation,
            DriverMode::Accelerated(AccelerationProfile::Auto),
        )
        .expect("audited target is accepted");
        assert_eq!(prepared.args, invocation);
        assert_eq!(prepared.info.kind, InvocationKind::Compilation);
        assert!(!prepared.info.inject_accelerated_cfg);
    }
}

#[test]
fn accelerated_policy_rejection_latch_accepts_repeated_discovery_and_link_flags() {
    let invocation = args(&[
        "--print",
        "sysroot",
        "--print=target-list",
        "--cfg",
        "feature=\"example\"",
        "--cfg=other",
        "--check-cfg",
        "cfg(feature, values(\"example\"))",
        "-L",
        "dependency=target/debug/deps",
        "-Ldependency=target/release/deps",
        "--extern",
        "foo=target/debug/deps/libfoo.rlib",
        "--extern=bar=target/debug/deps/libbar.rlib",
        "-A",
        "warnings",
        "-Dunused_variables",
        "-l",
        "static=helper",
        "-lframework=System",
    ]);
    let prepared = prepare_rustc_invocation(AUDITED_RUSTC_VV, &invocation, DriverMode::Ordinary)
        .expect("repeated non-keyed controls are accepted");
    assert_eq!(prepared.args, invocation);
    assert_eq!(prepared.info.kind, InvocationKind::Probe);
}

#[test]
fn accelerated_policy_rejection_latch_rejects_duplicate_keyed_controls() {
    let duplicate_cases = [
        args(&["-C", "opt-level=3", "-C", "opt-level=3"]),
        args(&["--emit=link", "--emit=metadata"]),
        args(&["--target", ALLOWED_HOST, "--target", ALLOWED_WASM_TARGET]),
        args(&["-C", "debuginfo=2", "-C", "debuginfo=0"]),
    ];
    for invocation in duplicate_cases {
        assert!(
            validate_rustc_invocation(AUDITED_RUSTC_VV, &invocation, DriverMode::Ordinary).is_err(),
            "duplicate keyed controls must reject: {invocation:?}"
        );
    }
}

#[test]
fn accelerated_policy_rejection_latch_rejects_response_llvm_and_sysroot_controls() {
    let rejected_cases = [
        args(&["@rustc.rsp"]),
        args(&["-C", "llvm-args=--some-pass"]),
        args(&["-C", "target-cpu=native"]),
        args(&["-C", "target-feature=+avx2"]),
        args(&["--sysroot", "C:/untrusted/sysroot"]),
        args(&["-Z", "unstable-options"]),
        args(&["-C", "link-arg=/untrusted/linker-flag"]),
        args(&["--emit=llvm-ir"]),
    ];
    for invocation in rejected_cases {
        assert!(
            validate_rustc_invocation(AUDITED_RUSTC_VV, &invocation, DriverMode::Ordinary).is_err(),
            "unsafe policy input must reject: {invocation:?}"
        );
    }
}

#[test]
fn accelerated_policy_rejection_latch_injects_cfg_only_for_canonical_core() {
    let mut core = args(&[
        "--crate-name",
        "slicer_core",
        "--crate-type",
        "lib",
        "-C",
        "opt-level=3",
    ]);
    core.extend(cargo_unit_signature());
    let core_plan = prepare_rustc_invocation(
        AUDITED_RUSTC_VV,
        &core,
        DriverMode::Accelerated(AccelerationProfile::Auto),
    )
    .expect("optimized canonical core is accepted");
    assert!(core_plan.info.canonical_core);
    assert!(core_plan.info.inject_accelerated_cfg);
    assert_eq!(
        core_plan.args.last().map(String::as_str),
        Some(RESERVED_CFG)
    );
    assert_eq!(
        core_plan
            .args
            .iter()
            .filter(|value| value.as_str() == RESERVED_CFG)
            .count(),
        1
    );

    let mut unit_test = args(&["--test", "--crate-name", "slicer_core", "-C", "opt-level=3"]);
    unit_test.extend(cargo_unit_signature());
    let unit_plan = prepare_rustc_invocation(
        AUDITED_RUSTC_VV,
        &unit_test,
        DriverMode::Accelerated(AccelerationProfile::Auto),
    )
    .expect("optimized canonical unit test is accepted");
    assert!(unit_plan.info.inject_accelerated_cfg);

    let mut wasm_other = args(&[
        "--target",
        ALLOWED_WASM_TARGET,
        "--crate-name",
        "some_other",
        "--crate-type",
        "lib",
        "-C",
        "opt-level=3",
    ]);
    wasm_other.extend(cargo_unit_signature());
    let other_plan = prepare_rustc_invocation(
        AUDITED_RUSTC_VV,
        &wasm_other,
        DriverMode::Accelerated(AccelerationProfile::Auto),
    )
    .expect("non-canonical wasm crate is accepted");
    assert!(!other_plan.info.inject_accelerated_cfg);

    let mut binary = args(&[
        "--crate-name",
        "slicer_core",
        "--crate-type",
        "bin",
        "-C",
        "opt-level=3",
    ]);
    binary.extend(cargo_unit_signature());
    let binary_plan = prepare_rustc_invocation(
        AUDITED_RUSTC_VV,
        &binary,
        DriverMode::Accelerated(AccelerationProfile::Auto),
    )
    .expect("canonical-name binary is accepted without acceleration");
    assert!(!binary_plan.info.canonical_core);
    assert!(!binary_plan.info.inject_accelerated_cfg);

    let mut wasm_core = args(&[
        "--target",
        ALLOWED_WASM_TARGET,
        "--crate-name",
        "slicer_core",
        "--crate-type",
        "lib",
        "-C",
        "opt-level=3",
    ]);
    wasm_core.extend(cargo_unit_signature());
    let wasm_plan = prepare_rustc_invocation(
        AUDITED_RUSTC_VV,
        &wasm_core,
        DriverMode::Accelerated(AccelerationProfile::Auto),
    )
    .expect("canonical wasm core compilation is accelerated");
    assert!(wasm_plan.info.canonical_core);
    assert!(wasm_plan.info.inject_accelerated_cfg);
    assert_eq!(
        wasm_plan.args.last().map(String::as_str),
        Some(RESERVED_CFG)
    );
}

#[test]
fn accelerated_policy_rejection_latch_rejects_unoptimized_canonical_core() {
    let mut unoptimized = args(&["--crate-name", "slicer_core", "-C", "opt-level=0"]);
    unoptimized.extend(cargo_unit_signature());
    let error = validate_rustc_invocation(
        AUDITED_RUSTC_VV,
        &unoptimized,
        DriverMode::Accelerated(AccelerationProfile::Auto),
    )
    .expect_err("unoptimized accelerated core must fail closed");
    assert!(error.to_string().contains("opt-level=3"));
}

#[test]
fn accelerated_host_release_accepts_proc_macro_build_script_without_opt_level() {
    let root = test_root("proc-macro2-build-script");
    let invocation = args(&[
        "--crate-name",
        "build_script_build",
        "--edition=2021",
        r"C:\Users\agpen\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\proc-macro2-1.0.106\build.rs",
        "--error-format=json",
        "--json=diagnostic-rendered-ansi,artifacts,future-incompat",
        "--crate-type",
        "bin",
        "--emit=dep-info,link",
        "-C",
        "embed-bitcode=no",
        "-C",
        "debug-assertions=off",
        "--cfg",
        "feature=\"default\"",
        "--cfg",
        "feature=\"proc-macro\"",
        "--check-cfg",
        "cfg(docsrs)",
        "--check-cfg",
        "cfg(feature, values(\"default\", \"proc-macro\"))",
        "-C",
        "metadata=d2b7ccaab28556d3",
        "-C",
        "extra-filename=-50a349f8e7e10525",
        "--out-dir",
        r"target\release\build\proc-macro2-50a349f8e7e10525",
        "-C",
        "strip=symbols",
        "-L",
        r"dependency=target\release\deps",
        "--cap-lints",
        "allow",
    ]);

    let prepared = prepare_rustc_invocation(
        REAL_RUSTC_VV,
        &invocation,
        DriverMode::Accelerated(AccelerationProfile::HostRelease),
    )
    .expect("dependency build scripts may omit host-release opt-level");
    assert_eq!(prepared.info.kind, InvocationKind::Compilation);
    assert_eq!(
        prepared.info.crate_name.as_deref(),
        Some("build_script_build")
    );
    assert!(!prepared.info.canonical_core);
    assert!(!prepared.info.inject_accelerated_cfg);
    assert_eq!(prepared.args, invocation);
    assert!(!prepared.args.iter().any(|value| value == RESERVED_CFG));
    assert!(!is_latched(&root));
    remove_test_root(&root);
}

#[test]
fn accelerated_host_release_accepts_optimized_canonical_core() {
    let mut invocation = args(&[
        "--crate-name",
        "slicer_core",
        "--crate-type",
        "lib",
        "-C",
        "opt-level=3",
    ]);
    invocation.extend(cargo_unit_signature());

    let prepared = prepare_rustc_invocation(
        REAL_RUSTC_VV,
        &invocation,
        DriverMode::Accelerated(AccelerationProfile::HostRelease),
    )
    .expect("optimized canonical core must be accepted in host release");
    assert_eq!(prepared.info.kind, InvocationKind::Compilation);
    assert!(prepared.info.canonical_core);
    assert!(prepared.info.inject_accelerated_cfg);
    assert_eq!(prepared.args.last().map(String::as_str), Some(RESERVED_CFG));
}

#[test]
fn accelerated_policy_rejection_latch_persists_after_a_swallowed_child_failure() {
    let root = test_root("swallowed-child");
    assert!(!is_latched(&root));
    latch_policy_rejection(
        &root,
        "a child rejected policy and its parent swallowed exit 1",
    )
    .expect("write persistent latch");
    assert!(is_latched(&root));
    assert!(latch_path(&root).is_file());
    assert_eq!(
        enforce_accelerated_latch_gate(&root, 0),
        1,
        "a swallowed child rejection must fail the accelerated post-build gate"
    );
    clear_latch(&root).expect("clear test latch");
    assert!(!is_latched(&root));
    assert_eq!(
        enforce_accelerated_latch_gate(&root, 0),
        0,
        "a clean accelerated session must pass the post-build gate"
    );
    remove_test_root(&root);
}

#[test]
fn accelerated_policy_rejection_latch_keeps_ordinary_mode_linear_without_cfg() {
    let invocation = args(&[
        "--crate-name",
        "slicer_core",
        "--crate-type",
        "lib",
        "-C",
        "opt-level=0",
        "--cfg",
        "feature=\"fallback\"",
    ]);
    let prepared = prepare_rustc_invocation(AUDITED_RUSTC_VV, &invocation, DriverMode::Ordinary)
        .expect("ordinary pass-through accepts ordinary core profile");
    assert_eq!(prepared.args, invocation);
    assert!(!prepared.info.inject_accelerated_cfg);
    assert!(!prepared.args.iter().any(|value| value == RESERVED_CFG));
}
