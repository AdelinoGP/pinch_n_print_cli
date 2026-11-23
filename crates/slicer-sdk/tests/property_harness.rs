//! Compile witnesses and real-wrapper coverage of per-invocation SDK isolation.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Barrier};
use std::thread;

use proptest::strategy::Just;
use proptest::test_runner::{TestCaseError, TestCaseResult};
use slicer_ir::{ExtrusionRole, LayerEntityIdGen, Point3};
use slicer_sdk::host::{self, LogLevel};
use slicer_sdk::test_support::capture::{
    InfillOutputCapture, PerimeterOutputCapture, SupportOutputCapture,
};
use slicer_sdk::test_support::fixtures::extrusion_path3d_base;
use slicer_sdk::test_support::mock_host::MockHost;
use slicer_sdk::test_support::mock_host_setup;
use slicer_sdk::test_support::property_testing::{
    take_case_logs, CaseGuard, PropertyRun, PropertyRunError,
};

// test-quality: compile witnesses pin the property-testing public handoff signatures.
const _: fn(&'static str, &'static str) -> Result<PropertyRun, PropertyRunError> = PropertyRun::new;
const _: fn(&PropertyRun, &str) -> Result<u64, PropertyRunError> = PropertyRun::bound;
const _: fn(
    &PropertyRun,
    &Just<u64>,
    fn(u64, &mut CaseGuard) -> TestCaseResult,
) -> Result<(), PropertyRunError> =
    PropertyRun::check::<Just<u64>, fn(u64, &mut CaseGuard) -> TestCaseResult>;
const _: fn() -> CaseGuard = CaseGuard::new;
const _: fn(&mut CaseGuard, MockHost) = CaseGuard::install_mock;
const _: fn(&CaseGuard) -> &LayerEntityIdGen = CaseGuard::entity_ids;
const _: fn(&mut CaseGuard) -> &mut InfillOutputCapture = CaseGuard::infill;
const _: fn(&mut CaseGuard) -> &mut PerimeterOutputCapture = CaseGuard::perimeters;
const _: fn(&mut CaseGuard) -> &mut SupportOutputCapture = CaseGuard::support;
const _: fn() -> Vec<(LogLevel, String)> = take_case_logs;

// test-quality: compile witness pins PropertyRunError's promised error traits.
#[allow(dead_code)]
fn error_trait_witness<T: std::fmt::Debug + std::fmt::Display + std::error::Error>() {}
const _: fn() = error_trait_witness::<PropertyRunError>;

fn raycast() -> Option<f32> {
    host::raycast_z_down("case-object", 0.0, 0.0, 100.0)
}

fn assert_empty_captures(guard: &mut CaseGuard) {
    assert!(guard.infill().sparse_paths().is_empty());
    assert!(guard.infill().solid_paths().is_empty());
    assert!(guard.infill().ironing_paths().is_empty());
    assert!(guard.perimeters().wall_loops().is_empty());
    assert!(guard.perimeters().infill_areas().is_empty());
    assert!(guard.perimeters().seam_candidates().is_empty());
    assert!(guard.support().support_paths().is_empty());
    assert!(guard.support().interface_paths().is_empty());
    assert!(guard.support().raft_paths().is_empty());
    assert!(guard.support().anchored_proposal().is_none());
}

fn assert_fresh(guard: &mut CaseGuard) {
    assert!(host::test_support::take_log_messages().is_empty());
    assert_eq!(raycast(), None);
    assert_empty_captures(guard);
    // The allocator contract starts at 1, independent of the previous case.
    assert_eq!(guard.entity_ids().next(), 1);
    // Taking logs uninstalls the real SDK sink, so restore it after inspection.
    mock_host_setup();
}

fn dirty_case(guard: &mut CaseGuard, marker: &str, hit: f32) -> u64 {
    host::log_warn(marker);
    guard.install_mock(MockHost::new().with_raycast_hit(Some(hit)));
    guard.infill().push_sparse_path(slicer_ir::ExtrusionPath3D {
        speed_factor: hit,
        ..extrusion_path3d_base(ExtrusionRole::SparseInfill)
    });
    guard.perimeters().push_seam_candidate(
        Point3 {
            x: hit,
            y: 0.0,
            z: 0.0,
        },
        hit,
    );
    guard
        .support()
        .push_support_path(slicer_ir::ExtrusionPath3D {
            speed_factor: hit,
            ..extrusion_path3d_base(ExtrusionRole::SupportMaterial)
        });
    guard.entity_ids().next()
}

fn assert_dirty(guard: &mut CaseGuard, hit: f32) {
    assert_eq!(raycast(), Some(hit));
    assert_eq!(guard.infill().sparse_paths().len(), 1);
    assert_eq!(guard.infill().sparse_paths()[0].speed_factor, hit);
    assert_eq!(guard.perimeters().seam_candidates().len(), 1);
    assert_eq!(guard.perimeters().seam_candidates()[0].0.x, hit);
    assert_eq!(guard.perimeters().seam_candidates()[0].1, hit);
    assert_eq!(guard.support().support_paths().len(), 1);
    assert_eq!(guard.support().support_paths()[0].speed_factor, hit);
}

fn assert_torn_down(marker: &str) {
    // Observe teardown before constructing the next guard: entry reset must
    // not mask a broken Drop, especially on the unwind path.
    assert_eq!(raycast(), None);
    assert!(host::test_support::take_log_messages().is_empty());
    assert_eq!(take_case_logs(), vec![(LogLevel::Warn, marker.to_string())]);
    assert!(take_case_logs().is_empty());
}

#[test]
fn case_guard_cleans_success_failure_and_unwind() {
    // Dirty the same thread outside a guard so Drop cannot mask a missing
    // entry reset. The real mesh wrapper witnesses the dirty precondition.
    mock_host_setup();
    host::log_warn("pre-existing-log");
    MockHost::new().with_raycast_hit(Some(99.0)).install();
    assert_eq!(raycast(), Some(99.0));

    {
        let mut guard = CaseGuard::new();
        assert_fresh(&mut guard);
        assert_eq!(dirty_case(&mut guard, "success-case", 10.0), 2);
        assert_dirty(&mut guard, 10.0);
    }
    assert_torn_down("success-case");

    let returned_failure_case = || -> TestCaseResult {
        let mut guard = CaseGuard::new();
        assert_fresh(&mut guard);
        assert_eq!(dirty_case(&mut guard, "returned-failure", 20.0), 2);
        assert_dirty(&mut guard, 20.0);
        Err(TestCaseError::fail("returned-failure"))
    };
    let failure = returned_failure_case();
    match failure {
        Err(TestCaseError::Fail(reason)) => assert_eq!(reason.to_string(), "returned-failure"),
        other => panic!("expected the deliberate returned failure, got {other:?}"),
    }
    assert_torn_down("returned-failure");

    let panic = catch_unwind(|| {
        let mut guard = CaseGuard::new();
        assert_fresh(&mut guard);
        assert_eq!(dirty_case(&mut guard, "unwound-case", 30.0), 2);
        assert_dirty(&mut guard, 30.0);
        panic!("deliberate case unwind");
    })
    .expect_err("the deliberate case panic must unwind");
    assert_eq!(
        panic.downcast_ref::<&str>(),
        Some(&"deliberate case unwind")
    );
    assert_torn_down("unwound-case");

    let mut next = CaseGuard::new();
    // Observe the constructor-installed sink before assert_fresh drains it;
    // the test's own sink restoration must not hide a missing setup call.
    host::log_warn("fresh-case-sink");
    assert_eq!(
        host::test_support::take_log_messages(),
        vec![(LogLevel::Warn, "fresh-case-sink".to_string())]
    );
    assert_fresh(&mut next);
}

#[test]
fn case_guard_isolates_concurrent_threads() {
    let barrier = Arc::new(Barrier::new(4));
    let threads: Vec<_> = (0..4)
        .map(|index| {
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let marker = format!("thread-{index}");
                let hit = (index + 1) as f32;
                let mut guard = CaseGuard::new();
                // Delay assertions until after the barrier so a broken seam
                // cannot panic early and strand the other test threads.
                let entry_logs = host::test_support::take_log_messages();
                let entry_hit = raycast();
                let first_id = guard.entity_ids().next();
                mock_host_setup();
                let second_id = dirty_case(&mut guard, &marker, hit);
                barrier.wait();

                assert!(entry_logs.is_empty());
                assert_eq!(entry_hit, None);
                assert_eq!(first_id, 1);
                assert_eq!(second_id, 2);
                assert_dirty(&mut guard, hit);
                assert_eq!(guard.entity_ids().next(), 3);
                assert_eq!(
                    host::test_support::take_log_messages(),
                    vec![(LogLevel::Warn, marker.clone())]
                );
                mock_host_setup();
                host::log_warn(&marker);
                drop(guard);
                assert_torn_down(&marker);
                let mut next = CaseGuard::new();
                assert_fresh(&mut next);
            })
        })
        .collect();

    for thread in threads {
        thread
            .join()
            .expect("case-local state leaked between threads");
    }
}

#[test]
fn case_guard_rejects_nested_use_before_mutation() {
    let mut outer = CaseGuard::new();
    assert_fresh(&mut outer);
    assert_eq!(dirty_case(&mut outer, "outer-case", 42.0), 2);

    let panic = catch_unwind(AssertUnwindSafe(CaseGuard::new))
        .err()
        .expect("nested case guards must panic");
    let message = panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().copied());
    assert_eq!(message, Some("nested CaseGuard on the same thread"));

    assert_dirty(&mut outer, 42.0);
    assert_eq!(outer.entity_ids().next(), 3);
    assert_eq!(
        host::test_support::take_log_messages(),
        vec![(LogLevel::Warn, "outer-case".to_string())]
    );
    // The outer guard remains writable after the rejected nested attempt.
    mock_host_setup();
    assert_eq!(dirty_case(&mut outer, "outer-still-usable", 42.0), 4);
    assert_eq!(outer.infill().sparse_paths().len(), 2);
    assert_eq!(outer.perimeters().seam_candidates().len(), 2);
    assert_eq!(outer.support().support_paths().len(), 2);
    drop(outer);
    assert_torn_down("outer-still-usable");

    let mut next = CaseGuard::new();
    assert_fresh(&mut next);
}

// All environment changes below are made on child Commands, never in the
// multi-threaded libtest process. The same test is re-entered by rusty-fork.
mod property_run_tests {
    use std::path::PathBuf;
    use std::process::{Command, Output};
    use std::sync::atomic::{AtomicU64, Ordering};

    use proptest::strategy::Just;
    use proptest::test_runner::TestCaseError;
    use serde_json::{json, Value};
    use slicer_sdk::host;
    use slicer_sdk::test_support::mock_host::MockHost;
    use slicer_sdk::test_support::property_testing::PropertyRun;

    static NEXT: AtomicU64 = AtomicU64::new(0);
    const PROPERTY: &str = "PBT-TEST";
    const MODE: &str = "PNP_SDK_PROPERTY_TEST_MODE";
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

    fn fixture(test_name: &str, label: &str) -> (Value, PathBuf) {
        let token = format!(
            "test-{}-{}-{label}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/property-testing")
            .join(&token)
            .join(PROPERTY);
        let context = json!({
            "schema_version": 1, "run_token": token, "property_id": PROPERTY,
            "tier": "routine", "seed": 37, "package": "slicer-sdk", "target_kind": "test",
            "target_name": "property_harness", "test_name": test_name,
            "features": ["slicer-sdk/property_testing"],
            "profile": {
                "cases": 8, "max_local_rejects": 16, "max_global_rejects": 3,
                "max_flat_map_regens": 100, "max_default_size_range": 32,
                "max_shrink_iters": 64, "max_shrink_time_ms": 0,
                "rng_algorithm": "chacha", "isolation": "in_process", "timeout_ms": 0,
                "bounds": {"positive": 7, "zero": 0}
            },
            "artifact_dir": dir.to_str().expect("UTF-8 test path"),
            "proptest_version": "1.11.0", "generator_version": 1,
            "source_revision": "test-revision", "dirty_paths": [], "source_state_digest": "test-digest",
            "cargo_target_dir": null, "completion_eligible": true
        });
        (context, dir)
    }

    fn child(
        test_name: &str,
        mode: &str,
        context: Option<&Value>,
        override_var: Option<&str>,
    ) -> Output {
        let mut command = Command::new(std::env::current_exe().expect("current test binary"));
        command
            .args(["--exact", test_name, "--nocapture"])
            .env(MODE, mode)
            .env_remove("PNP_PROPERTY_CONTEXT")
            .env_remove("_PROPTEST_FORKFILE")
            .env_remove("PROPTEST_VERBOSE");
        for name in OVERRIDES {
            command.env_remove(name);
        }
        if let Some(context) = context {
            command.env("PNP_PROPERTY_CONTEXT", context.to_string());
        }
        if let Some(name) = override_var {
            command.env(name, "1");
        }
        let output = command.output().expect("launch isolated property test");
        assert!(
            output.status.success(),
            "child {mode} failed: {}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    fn mode() -> Option<String> {
        std::env::var(MODE).ok()
    }

    fn json_file(dir: &std::path::Path, file: &str) -> Value {
        serde_json::from_slice(&std::fs::read(dir.join(file)).expect("artifact must exist"))
            .expect("valid artifact JSON")
    }

    fn events(output: &Output) -> Vec<Value> {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                line.find("PNP_PROPERTY_EVENT ").map(|index| {
                    assert_eq!(index, 0, "property events must start on a new line: {line}");
                    serde_json::from_str(&line[index + "PNP_PROPERTY_EVENT ".len()..])
                        .expect("valid property event")
                })
            })
            .collect()
    }

    fn controller_events(output: &Output) -> Vec<Value> {
        events(output)
            .into_iter()
            .filter(|event| event["process_role"] == "controller")
            .collect()
    }

    fn assert_controller_end(output: &Output, outcome: &str) -> Value {
        let controller = controller_events(output);
        assert_eq!(
            controller.len(),
            2,
            "expected one controller begin/end: {controller:?}"
        );
        assert_eq!(controller[0]["phase"], "begin");
        assert_eq!(controller[1]["phase"], "end");
        assert_eq!(controller[0]["schema_version"], 1);
        assert_eq!(controller[1]["schema_version"], 1);
        assert_eq!(controller[0]["property_id"], PROPERTY);
        assert_eq!(controller[1]["property_id"], PROPERTY);
        assert_eq!(controller[0]["run_token"], controller[1]["run_token"]);
        assert_eq!(controller[1]["outcome"], outcome);
        assert_eq!(controller[1]["shrink_completed"], Value::Null);
        controller[1].clone()
    }

    #[test]
    fn property_run_rejects_missing_mismatched_context_and_bounds() {
        const NAME: &str =
            "property_run_tests::property_run_rejects_missing_mismatched_context_and_bounds";
        if let Some(mode) = mode() {
            match mode.as_str() {
                "missing" => {
                    let error = PropertyRun::new(PROPERTY, NAME)
                        .err()
                        .expect("missing context must fail");
                    assert_eq!(error.kind(), "missing_property_context");
                    assert!(error.to_string().contains("cargo xtask test --summary --properties --property PBT-TEST --tier routine"));
                }
                "identity" => assert_eq!(
                    PropertyRun::new(PROPERTY, NAME)
                        .err()
                        .expect("identity mismatch")
                        .kind(),
                    "context_identity_mismatch"
                ),
                "bounds" => {
                    let run = PropertyRun::new(PROPERTY, NAME).expect("valid context");
                    assert_eq!(run.bound("positive").expect("positive bound"), 7);
                    assert_eq!(
                        run.bound("absent").expect_err("missing bound").kind(),
                        "unknown_bound"
                    );
                    assert_eq!(
                        run.bound("zero").expect_err("zero bound").kind(),
                        "invalid_bound"
                    );
                }
                "invalid" => assert_eq!(
                    PropertyRun::new(PROPERTY, NAME)
                        .err()
                        .expect("invalid profile")
                        .kind(),
                    "invalid_property_context"
                ),
                _ => panic!("unexpected test mode {mode}"),
            }
            return;
        }
        child(NAME, "missing", None, None);
        let (mut context, _) = fixture(NAME, "identity");
        context["property_id"] = json!("WRONG");
        child(NAME, "identity", Some(&context), None);
        context["property_id"] = json!(PROPERTY);
        context["test_name"] = json!("wrong_test");
        child(NAME, "identity", Some(&context), None);
        context["test_name"] = json!(NAME);
        child(NAME, "bounds", Some(&context), None);
        context["profile"]["cases"] = json!(0);
        child(NAME, "invalid", Some(&context), None);
        context["profile"]["cases"] = json!(8);
        context["profile"]["timeout_ms"] = json!(1);
        child(NAME, "invalid", Some(&context), None);
        context["profile"]["timeout_ms"] = json!(0);
        context["extra_key"] = json!(true);
        child(NAME, "invalid", Some(&context), None);
    }

    #[test]
    fn property_run_rejects_profile_overrides() {
        const NAME: &str = "property_run_tests::property_run_rejects_profile_overrides";
        if mode().is_some() {
            let run = PropertyRun::new(PROPERTY, NAME).expect("valid context");
            let called = std::cell::Cell::new(false);
            let error = run
                .check(&Just(1_u32), |_, _| {
                    called.set(true);
                    Ok(())
                })
                .expect_err("override must fail");
            assert_eq!(error.kind(), "profile_override_conflict");
            assert!(
                !called.get(),
                "conflicting profile must not execute a callback"
            );
            return;
        }
        for name in OVERRIDES {
            let (context, dir) = fixture(NAME, name);
            let output = child(NAME, "override", Some(&context), Some(name));
            let end = assert_controller_end(&output, "failed");
            assert_eq!(end["error_kind"], "profile_override_conflict");
            assert_eq!(end["completion_eligible"], false);
            assert_eq!(json_file(&dir, "failure.json")["classification"], "harness");
        }
    }

    #[test]
    fn property_run_passing_verbose_and_ineligible_context() {
        const NAME: &str =
            "property_run_tests::property_run_passing_verbose_and_ineligible_context";
        if let Some(mode) = mode() {
            let run = PropertyRun::new(PROPERTY, NAME).expect("valid context");
            let called = std::cell::Cell::new(0_u32);
            let result = run.check(&Just(11_u32), |value, _| {
                assert_eq!(value, 11);
                called.set(called.get() + 1);
                Ok(())
            });
            if mode == "ineligible" {
                assert_eq!(result.expect_err("ineligible run").kind(), "harness");
                assert_eq!(called.get(), 0);
            } else {
                result.expect("measured property must pass");
                assert_eq!(called.get(), 8);
            }
            return;
        }
        let (mut context, dir) = fixture(NAME, "passing");
        let output = child(NAME, "passing", Some(&context), Some("PROPTEST_VERBOSE"));
        let end = assert_controller_end(&output, "passed");
        assert_eq!(end["accepted_cases"], 8);
        assert_eq!(end["accepted_test_calls"], 8);
        assert_eq!(end["rejected_test_calls"], 0);
        assert_eq!(end["completion_eligible"], true);
        assert_eq!(end["proptest_verbose_override"], "1");
        assert_eq!(json_file(&dir, "context.json"), context);
        assert!(!dir.join("failure.json").exists());
        context["completion_eligible"] = json!(false);
        let output = child(NAME, "ineligible", Some(&context), None);
        let end = assert_controller_end(&output, "failed");
        assert_eq!(end["completion_eligible"], false);
        assert_eq!(json_file(&dir, "context.json"), context);
    }

    #[test]
    fn property_run_reports_artifact_write_failure() {
        const NAME: &str = "property_run_tests::property_run_reports_artifact_write_failure";
        if mode().is_some() {
            let run = PropertyRun::new(PROPERTY, NAME).expect("valid context");
            let error = run
                .check(&Just(1_u32), |_, _| {
                    panic!("unwritable artifacts must prevent callback")
                })
                .expect_err("unwritable artifact directory");
            assert_eq!(error.kind(), "harness");
            return;
        }
        let (context, dir) = fixture(NAME, "write-failure");
        std::fs::create_dir_all(dir.parent().expect("artifact parent"))
            .expect("create test parent");
        std::fs::write(&dir, "not a directory").expect("create obstructing file");
        let output = child(NAME, "write-failure", Some(&context), None);
        let end = assert_controller_end(&output, "failed");
        assert_eq!(end["error_kind"], "harness");
        assert_eq!(end["completion_eligible"], false);
        assert!(end["artifact_write_error"]
            .as_str()
            .is_some_and(|s| !s.is_empty()));
        assert_eq!(
            std::fs::read_to_string(dir).expect("obstruction retained"),
            "not a directory"
        );
    }

    #[test]
    fn property_run_reports_failure_panic_and_exhaustion() {
        const NAME: &str = "property_run_tests::property_run_reports_failure_panic_and_exhaustion";
        if let Some(mode) = mode() {
            let run = PropertyRun::new(PROPERTY, NAME).expect("valid context");
            let result = run.check(&Just(1234_u32), |_, guard| match mode.as_str() {
                "failure" => {
                    host::log_warn("failure-log");
                    Err(TestCaseError::fail("deliberate assertion"))
                }
                "panic" => {
                    host::log_warn("panic-log");
                    guard.install_mock(MockHost::new().with_raycast_hit(Some(10.0)));
                    panic!("deliberate callback panic");
                }
                "exhaustion" => Err(TestCaseError::reject("deliberate rejection")),
                _ => panic!("unexpected mode"),
            });
            let error = result.expect_err("failing property must not pass");
            assert_eq!(
                error.kind(),
                match mode.as_str() {
                    "failure" => "sut_failure",
                    "panic" => "panic",
                    "exhaustion" => "exhausted",
                    _ => unreachable!(),
                }
            );
            assert!(host::test_support::take_log_messages().is_empty());
            assert_eq!(host::raycast_z_down("object", 0.0, 0.0, 100.0), None);
            return;
        }
        for (scenario, classification, outcome) in [
            ("failure", "sut_or_contract", "failed"),
            ("panic", "panic", "failed"),
            ("exhaustion", "exhausted", "exhausted"),
        ] {
            let (context, dir) = fixture(NAME, scenario);
            let output = child(NAME, scenario, Some(&context), None);
            let end = assert_controller_end(&output, outcome);
            assert_eq!(end["accepted_cases"], Value::Null);
            assert!(end["runner_statistics"]
                .as_str()
                .expect("stats string")
                .contains("successes:"));
            let report = json_file(&dir, "failure.json");
            assert_eq!(report["classification"], classification);
            assert_eq!(report["concrete_input"], "1234");
            assert_eq!(report["context"], context);
            assert!(std::fs::read_to_string(dir.join("case.txt"))
                .expect("case log")
                .contains("1234"));
            assert_eq!(json_file(&dir, "context.json"), context);
            if scenario == "exhaustion" {
                // Proptest aborts the fourth callback after its three allowed
                // global rejects, so observed calls exceed its stored rejects.
                assert_eq!(end["rejected_test_calls"], 4);
                assert_eq!(end["accepted_test_calls"], 0);
                assert!(end["runner_statistics"]
                    .as_str()
                    .expect("stats")
                    .contains("global rejects: 3"));
            } else {
                assert_eq!(end["accepted_test_calls"], 1);
                assert_eq!(
                    report["captured_sdk_logs"][0]["message"],
                    format!("{scenario}-log")
                );
            }
        }
    }

    #[test]
    fn property_run_reproduction_preserves_tier_target_features_and_context() {
        const NAME: &str =
            "property_run_tests::property_run_reproduction_preserves_tier_target_features_and_context";
        if mode().is_some() {
            let run = PropertyRun::new(PROPERTY, NAME).expect("valid context");
            let error = run
                .check(&Just(19_u32), |_, _| {
                    Err(TestCaseError::fail("replay probe"))
                })
                .expect_err("probe must fail");
            assert_eq!(error.kind(), "sut_failure");
            return;
        }

        for (tier, target_kind, features, expected_args) in [
            (
                "extended",
                "lib",
                json!([]),
                json!([
                    "test",
                    "-p",
                    "slicer-sdk",
                    "--lib",
                    NAME,
                    "--",
                    "--exact",
                    "--nocapture"
                ]),
            ),
            (
                "routine",
                "test",
                json!(["slicer-sdk/property_testing", "slicer-sdk/test"]),
                json!([
                    "test",
                    "-p",
                    "slicer-sdk",
                    "--features",
                    "slicer-sdk/property_testing,slicer-sdk/test",
                    "--test",
                    "property_harness",
                    NAME,
                    "--",
                    "--exact",
                    "--nocapture"
                ]),
            ),
        ] {
            let (mut context, dir) = fixture(NAME, tier);
            context["tier"] = json!(tier);
            context["target_kind"] = json!(target_kind);
            context["features"] = features;
            // The original JSON has noncanonical whitespace; replay must use
            // the exact environment string, not a reserialized approximation.
            let context_json = serde_json::to_string_pretty(&context).expect("serialize context");
            let mut command = Command::new(std::env::current_exe().expect("current test binary"));
            command
                .args(["--exact", NAME, "--nocapture"])
                .env(MODE, "reproduction")
                .env("PNP_PROPERTY_CONTEXT", &context_json)
                .env_remove("_PROPTEST_FORKFILE")
                .env_remove("PROPTEST_VERBOSE");
            for name in OVERRIDES {
                command.env_remove(name);
            }
            let output = command.output().expect("launch reproduction probe");
            assert!(
                output.status.success(),
                "reproduction probe failed: {}\n{}\n{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            let report = json_file(&dir, "failure.json");
            assert_eq!(report["classification"], "sut_or_contract");
            assert_eq!(report["concrete_input"], "19");
            assert_eq!(report["context"], context);
            assert_eq!(report["reproduction_direct"]["program"], "cargo");
            assert_eq!(report["reproduction_direct"]["args"], expected_args);
            assert_eq!(
                report["reproduction_direct"]["env"]["PNP_PROPERTY_CONTEXT"],
                context_json
            );
            assert_eq!(
                report["reproduction_command"],
                format!("cargo xtask test --summary --properties --property {PROPERTY} --tier {tier} --seed 37")
            );
        }
    }

    #[test]
    fn property_run_same_seed_case_sequence_is_reproducible() {
        const NAME: &str =
            "property_run_tests::property_run_same_seed_case_sequence_is_reproducible";
        if mode().is_some() {
            let run = PropertyRun::new(PROPERTY, NAME).expect("valid context");
            let error = run
                .check(&(0_u32..1000), |value, guard| {
                    // Every initial and shrink invocation receives fresh SDK
                    // state, rather than reusing a guard for the whole run.
                    assert_eq!(guard.entity_ids().next(), 1);
                    assert_eq!(host::raycast_z_down("object", 0.0, 0.0, 100.0), None);
                    guard.install_mock(MockHost::new().with_raycast_hit(Some(10.0)));
                    if value >= 100 {
                        Err(TestCaseError::fail("replay threshold"))
                    } else {
                        Ok(())
                    }
                })
                .expect_err("threshold must produce a failure");
            assert_eq!(error.kind(), "sut_failure");
            return;
        }
        let (context_a, dir_a) = fixture(NAME, "replay-a");
        let (context_b, dir_b) = fixture(NAME, "replay-b");
        let out_a = child(NAME, "replay", Some(&context_a), None);
        let out_b = child(NAME, "replay", Some(&context_b), None);
        assert_controller_end(&out_a, "failed");
        assert_controller_end(&out_b, "failed");
        let cases_a = std::fs::read_to_string(dir_a.join("case.txt")).expect("first case log");
        let cases_b = std::fs::read_to_string(dir_b.join("case.txt")).expect("second case log");
        assert!(cases_a.contains("phase initial"));
        assert!(cases_a.contains("phase shrink"));
        // test-quality: determinism pin compares actual pre-shrink and shrink sequences for the same seed.
        assert_eq!(cases_a, cases_b);
        assert_eq!(
            json_file(&dir_a, "failure.json")["concrete_input"],
            json_file(&dir_b, "failure.json")["concrete_input"]
        );
        assert_eq!(json_file(&dir_a, "failure.json")["concrete_input"], "100");
    }

    #[test]
    fn property_run_failure_classification_uses_observed_callback_result() {
        const NAME: &str =
            "property_run_tests::property_run_failure_classification_uses_observed_callback_result";
        if let Some(mode) = mode() {
            let run = PropertyRun::new(PROPERTY, NAME).expect("valid context");
            let message = match mode.as_str() {
                "panic-marker" => "PNP_PROPERTY_CALLBACK_PANIC: not an unwind",
                "harness-marker" => "PNP_PROPERTY_HARNESS_ERROR: user assertion",
                "process-marker" => "Child process exited with simulated assertion text",
                _ => panic!("unexpected mode"),
            };
            let error = run
                .check(&Just(19_u32), |_, _| Err(TestCaseError::fail(message)))
                .expect_err("deliberate returned failure");
            assert_eq!(error.kind(), "sut_failure");
            return;
        }
        for scenario in ["panic-marker", "harness-marker", "process-marker"] {
            let (mut context, dir) = fixture(NAME, scenario);
            if scenario == "process-marker" {
                context["profile"]["isolation"] = json!("fork");
            }
            let output = child(NAME, scenario, Some(&context), None);
            let end = assert_controller_end(&output, "failed");
            assert_eq!(end["error_kind"], "sut_failure");
            assert_eq!(
                json_file(&dir, "failure.json")["classification"],
                "sut_or_contract"
            );
        }
    }

    #[test]
    fn property_run_fork_probe_tags_workers() {
        const NAME: &str = "property_run_tests::property_run_fork_probe_tags_workers";
        if mode().is_some() {
            let run = PropertyRun::new(PROPERTY, NAME).expect("valid context");
            run.check(&Just(19_u32), |value, _| {
                assert_eq!(value, 19);
                Ok(())
            })
            .expect("fork isolation is UNPROVEN: forked run failed");
            return;
        }
        let (mut context, dir) = fixture(NAME, "fork");
        context["profile"]["isolation"] = json!("fork");
        context["profile"]["cases"] = json!(3);
        let output = child(NAME, "fork", Some(&context), None);
        let end = assert_controller_end(&output, "passed");
        assert_eq!(end["accepted_cases"], 3);
        assert_eq!(end["accepted_test_calls"], 3);
        assert_eq!(end["rejected_test_calls"], 0);
        assert_eq!(end["call_counts_complete"], true);
        let observations: Vec<Value> = std::fs::read_to_string(dir.join("observations.jsonl"))
            .expect("fork observations")
            .lines()
            .map(|line| serde_json::from_str(line).expect("observation JSON"))
            .collect();
        let complete: Vec<_> = observations
            .iter()
            .filter(|o| o["result"] == "passed")
            .collect();
        assert_eq!(
            complete.len(),
            3,
            "fork isolation is UNPROVEN: missing callbacks"
        );
        let controller_pid = end["pid"].as_u64().expect("controller PID");
        for observation in complete {
            assert_eq!(
                observation["process_role"], "worker",
                "fork isolation is UNPROVEN: callback did not run as worker"
            );
            assert_ne!(
                observation["pid"].as_u64().expect("worker PID"),
                controller_pid,
                "fork isolation is UNPROVEN: callback ran in controller"
            );
        }
    }

    #[test]
    fn property_run_fork_crash_is_not_reported_as_completed() {
        const NAME: &str =
            "property_run_tests::property_run_fork_crash_is_not_reported_as_completed";
        if mode().is_some() {
            let run = PropertyRun::new(PROPERTY, NAME).expect("valid context");
            let error = run
                .check(&Just(19_u32), |_, _| std::process::exit(17))
                .expect_err("worker crash must fail");
            assert_eq!(error.kind(), "process_failure");
            return;
        }
        let (mut context, dir) = fixture(NAME, "fork-crash");
        context["profile"]["isolation"] = json!("fork");
        let output = child(NAME, "fork-crash", Some(&context), None);
        let end = assert_controller_end(&output, "failed");
        assert_eq!(end["error_kind"], "process_failure");
        assert_eq!(end["accepted_cases"], Value::Null);
        assert_eq!(end["call_counts_complete"], false);
        let report = json_file(&dir, "failure.json");
        assert_eq!(report["classification"], "process_failure");
        assert_eq!(report["concrete_input"], "19");
        let journal =
            std::fs::read_to_string(dir.join("observations.jsonl")).expect("worker journal");
        assert!(journal
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).expect("observation"))
            .any(|observation| observation["process_role"] == "worker"
                && observation["result"] == "started"));
    }
}

// These fixtures are intentionally not in the live registry. The xtask
// boundary tests launch them with a real, subprocess-local context.
#[test]
fn proptest_fixture_context_child() {
    const NAME: &str = "proptest_fixture_context_child";
    let run = PropertyRun::new("PBT-FIXTURE", NAME);
    if std::env::var_os("PNP_PROPERTY_CONTEXT").is_none() {
        assert_eq!(
            run.err().expect("missing context").kind(),
            "missing_property_context"
        );
        return;
    }
    let run = run.unwrap_or_else(|error| panic!("{error}"));
    run.check(&(0_u32..1000), |value, guard| {
        assert_eq!(guard.entity_ids().next(), 1);
        if value >= 100 {
            Err(TestCaseError::fail("fixture threshold"))
        } else {
            Ok(())
        }
    })
    .unwrap_or_else(|error| panic!("{error}"));
}

#[test]
fn proptest_fixture_all_rejecting() {
    const NAME: &str = "proptest_fixture_all_rejecting";
    let run = PropertyRun::new("PBT-FIXTURE", NAME);
    if std::env::var_os("PNP_PROPERTY_CONTEXT").is_none() {
        assert_eq!(
            run.err().expect("missing context").kind(),
            "missing_property_context"
        );
        return;
    }
    let run = run.unwrap_or_else(|error| panic!("{error}"));
    run.check(&Just(1234_u32), |_, _| {
        Err(TestCaseError::reject("fixture rejection"))
    })
    .unwrap_or_else(|error| panic!("{error}"));
}
