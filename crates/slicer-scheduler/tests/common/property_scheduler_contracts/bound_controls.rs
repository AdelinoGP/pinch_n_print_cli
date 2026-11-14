//! Deterministic AC-N3 control: scheduler safety ceilings and foundation-harness
//! refusals.
//!
//! The first half calls the target-private checked bound builders exported by
//! `crates/slicer-scheduler/tests/common/property_scheduler_{dag,claims,registry}.rs`
//! directly, so an unsafe, narrowing or empty generation request is rejected
//! before any strategy allocates, and then executes each charter's shrink audit
//! through its `run_shrink_audit` entry point at both accepted ceiling extremes.
//! The second half drives the real foundation
//! harness (`slicer_sdk::test_support::property_testing::PropertyRun`,
//! `crates/slicer-sdk/src/test_support/property_testing/run.rs`) with in-process
//! `PNP_PROPERTY_CONTEXT` JSON and proves that missing, invalid,
//! identity-mismatched and ineligible contexts, unknown/zero bounds, exhaustion
//! and a Proptest override conflict all fail with their named error kinds, while
//! a two-case passing run writes the promised evidence.
//!
//! `PNP_PROPERTY_CONTEXT` and the Proptest override variables are process-global,
//! so **every** context-mutating assertion lives in the single
//! `#[test] fn scheduler_property_bound_and_shrink_rejections` below. The names
//! this test touches are captured and restored by a `Drop` guard even when an
//! assertion panics. No registry property is executed here.

// The target-private model files are mounted once by this binary's root. The
// checked bound builders are the exported surface under test; copying them here
// would replace the production oracle with a test-local mirror.
use crate::{claim_models, dag_models, registry_models};

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use proptest::strategy::Just;
use proptest::test_runner::{TestCaseError, TestCaseResult};
use serde_json::{json, Value};
use slicer_sdk::test_support::property_testing::PropertyRun;

/// Identity used for every synthetic context. It need not exist in the live
/// registry: `PropertyRun::new` only compares the context's fields to these
/// arguments.
const PROPERTY_ID: &str = "SCTL-BOUND";
const TEST_NAME: &str = "scheduler_property_bound_and_shrink_rejections";
const CONTEXT_ENV: &str = "PNP_PROPERTY_CONTEXT";
const OVERRIDE_CASES_ENV: &str = "PROPTEST_CASES";

/// Every environment name this test manipulates: the context variable, the
/// names `PropertyRun::check` rejects as profile overrides
/// (`crates/slicer-sdk/src/test_support/property_testing/run.rs::OVERRIDES`) and
/// Proptest's fork worker marker. Clearing them makes a harness-kind assertion
/// independent of the ambient shell; nothing here treats this roster as an
/// expected set.
const HARNESS_ENV: [&str; 14] = [
    CONTEXT_ENV,
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

/// Saves every touched environment entry and restores it on drop, including
/// during unwinding.
struct EnvironmentGuard {
    originals: Vec<(&'static str, Option<OsString>)>,
}

impl EnvironmentGuard {
    fn clear(names: &[&'static str]) -> Self {
        let originals = names
            .iter()
            .map(|name| (*name, std::env::var_os(name)))
            .collect();
        for name in names {
            std::env::remove_var(name);
        }
        Self { originals }
    }

    fn set(&self, name: &str, value: &str) {
        std::env::set_var(name, value);
    }

    fn remove(&self, name: &str) {
        std::env::remove_var(name);
    }
}

impl Drop for EnvironmentGuard {
    fn drop(&mut self) {
        for (name, original) in &self.originals {
            match original {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}

static NEXT_ARTIFACT: AtomicU64 = AtomicU64::new(0);

/// One unique artifact directory under the system temp directory, removed on
/// drop.
struct TempArtifactDir {
    path: PathBuf,
}

impl TempArtifactDir {
    fn new(label: &str) -> Self {
        let sequence = NEXT_ARTIFACT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "pnp-scheduler-bound-controls-{}-{sequence}-{label}",
            std::process::id()
        ));
        // A crashed earlier run must not leak observations into this run.
        let _ = fs::remove_dir_all(&path);
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempArtifactDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// One valid registry context for the foundation harness.
///
/// The profile is deliberately tiny (`cases = 2`) so only the always-rejecting
/// case pays for more than two callbacks. `bounds` carries one positive and one
/// zero entry so `PropertyRun::bound` can be exercised on all three branches.
fn context_value(test_name: &str, artifact_dir: &Path) -> Value {
    json!({
        "schema_version": 1,
        "run_token": format!("scheduler-bound-controls-{}", std::process::id()),
        "property_id": PROPERTY_ID,
        "tier": "routine",
        "seed": 37,
        "package": "slicer-scheduler",
        "target_kind": "test",
        "target_name": "property_scheduler_contracts",
        "test_name": test_name,
        "features": ["slicer-scheduler/property_testing"],
        "profile": {
            "cases": 2,
            "max_local_rejects": 16,
            "max_global_rejects": 3,
            "max_flat_map_regens": 100,
            "max_default_size_range": 32,
            "max_shrink_iters": 64,
            "max_shrink_time_ms": 0,
            "rng_algorithm": "chacha",
            "isolation": "in_process",
            "timeout_ms": 0,
            "bounds": {"known": 7, "zero": 0}
        },
        "artifact_dir": artifact_dir
            .to_str()
            .expect("the temporary artifact directory is UTF-8"),
        "proptest_version": "1.11.0",
        "generator_version": 1,
        "source_revision": "scheduler-bound-controls-test-revision",
        "dirty_paths": [],
        "source_state_digest": "scheduler-bound-controls-test-digest",
        "cargo_target_dir": null,
        "completion_eligible": true
    })
}

/// Install `context` and assert `PropertyRun::new` refuses it with the exact
/// kind and message fragments the implementation emits.
fn expect_new_rejection(context: &Value, expected_kind: &str, fragments: &[&str]) {
    std::env::set_var(CONTEXT_ENV, context.to_string());
    let error = PropertyRun::new(PROPERTY_ID, TEST_NAME)
        .err()
        .expect("this synthetic context must not produce a usable PropertyRun");
    assert_eq!(
        error.kind(),
        expected_kind,
        "wrong rejection kind for context {context}: {error}"
    );
    let rendered = error.to_string();
    for fragment in fragments {
        assert!(
            rendered.contains(fragment),
            "the {expected_kind} message must contain {fragment:?}, got {rendered:?}"
        );
    }
}

/// Install a context that `PropertyRun::new` accepts.
fn valid_run(context: &Value) -> PropertyRun {
    std::env::set_var(CONTEXT_ENV, context.to_string());
    PropertyRun::new(PROPERTY_ID, TEST_NAME).expect("the bound-control context is valid")
}

/// Read a run's append-only observation journal.
fn observations(dir: &Path) -> Vec<Value> {
    fs::read_to_string(dir.join("observations.jsonl"))
        .expect("a run must write observations.jsonl")
        .lines()
        .map(|line| serde_json::from_str(line).expect("observation JSON line"))
        .collect()
}

/// Read the failure report the harness writes when `check` fails.
fn failure_report(dir: &Path) -> Value {
    serde_json::from_slice(
        &fs::read(dir.join("failure.json")).expect("a failing run must write failure.json"),
    )
    .expect("failure.json is valid JSON")
}

/// The checked claim bounds as one ordered tuple, for exact comparisons.
fn claim_tuple(limits: &claim_models::ClaimLimits) -> [u64; 5] {
    [
        limits.max_modules,
        limits.max_claims,
        limits.max_observations,
        limits.max_layers,
        limits.max_text_bytes,
    ]
}

/// Accepted `H-DAG` `max_nodes` extremes (`dag_models::checked_max_nodes`).
const DAG_LOW_MAX_NODES: u64 = 3;
const DAG_HIGH_MAX_NODES: u64 = 7;

/// Accepted `H-CLAIMS` bound extremes (`claim_models::checked_claim_limits`):
/// modules `3..=7`, claims `1..=8`, observations `2..=16`, layers `2..=4` and
/// variable text `36..=64` (UUID-syntax object ids).
const CLAIM_LOW: [u64; 5] = [3, 1, 2, 2, 36];
const CLAIM_HIGH: [u64; 5] = [7, 8, 16, 4, 64];

/// Accepted `H-REGISTRY` bound extremes
/// (`registry_models::checked_registry_limits`): modules `2..=7`, declarations
/// `2..=16` and text `8..=64`.
const REGISTRY_LOW: [u64; 3] = [2, 2, 8];
const REGISTRY_HIGH: [u64; 3] = [7, 16, 64];

/// Part A: every safety ceiling rejects the unsafe/narrowing request before a
/// strategy allocates and accepts exactly the documented endpoints.
fn assert_scheduler_bounds_reject_unsafe_requests() {
    // ── H-DAG `max_nodes`: `3..=7` ─────────────────────────────────────────
    for raw in [0_u64, 2, 8, u64::MAX] {
        let error = dag_models::checked_max_nodes(raw)
            .expect_err("an out-of-ceiling node bound must be rejected");
        assert!(
            error.contains("max_nodes"),
            "the rejection must name max_nodes: {error}"
        );
        assert!(
            error.contains("3..=7"),
            "the rejection must name the legal 3..=7 range: {error}"
        );
        assert!(
            error.contains(&raw.to_string()),
            "the rejection must name the offending value {raw}: {error}"
        );
    }
    for raw in [DAG_LOW_MAX_NODES, DAG_HIGH_MAX_NODES] {
        let checked = dag_models::checked_max_nodes(raw).expect("an in-ceiling node bound");
        assert_eq!(
            checked,
            u16::try_from(raw).expect("3 and 7 fit u16"),
            "the accepted node bound must be the exact u16 conversion"
        );
        assert_eq!(
            u64::from(checked),
            raw,
            "u16 narrowing must not silently change the accepted node bound"
        );
    }

    // ── H-CLAIMS: modules `3..=7`, claims `1..=8`, observations `2..=16`,
    //    layers `2..=4`, variable text `36..=64` (UUID-syntax object ids) ────
    const CLAIM_BOUNDS: [(&str, usize, u64, u64); 5] = [
        ("max_modules", 0, 3, 7),
        ("max_claims", 1, 1, 8),
        ("max_observations", 2, 2, 16),
        ("max_layers", 3, 2, 4),
        ("max_text_bytes", 4, 36, 64),
    ];

    for (name, index, low, high) in CLAIM_BOUNDS {
        for value in [low - 1, high + 1] {
            let mut requested = CLAIM_LOW;
            requested[index] = value;
            let error = claim_models::checked_claim_limits(
                requested[0],
                requested[1],
                requested[2],
                requested[3],
                requested[4],
            )
            .expect_err("a bound outside the H-CLAIMS safety ceiling must be rejected");
            assert!(
                error.contains(name),
                "the rejection must name {name}: {error}"
            );
            assert!(
                error.contains(&format!("{low}..={high}")),
                "the rejection must name the legal {low}..={high} range: {error}"
            );
            assert!(
                error.contains(&value.to_string()),
                "the rejection must name the offending value {value}: {error}"
            );
        }
    }

    for requested in [CLAIM_LOW, CLAIM_HIGH] {
        let limits = claim_models::checked_claim_limits(
            requested[0],
            requested[1],
            requested[2],
            requested[3],
            requested[4],
        )
        .expect("both documented ceiling endpoints are accepted");
        assert_eq!(
            claim_tuple(&limits),
            requested,
            "the accepted claim tuple must be the exact requested endpoints"
        );
        for value in requested {
            let narrowed = u16::try_from(value).expect("every checked claim bound fits u16");
            assert_eq!(
                u64::from(narrowed),
                value,
                "u16 narrowing must not change claim bound {value}"
            );
            let widened = usize::try_from(value).expect("every checked claim bound fits usize");
            assert_eq!(
                u64::try_from(widened).expect("usize round-trips into u64"),
                value,
                "usize widening must not change claim bound {value}"
            );
        }
    }

    // `checked_claim_limits` enforces the 36-byte UUID-syntax floor, not just the
    // profile ceiling: the generated object ids are UUID syntax, so accepting a
    // smaller `max_text_bytes` and then widening it inside the strategy would
    // make the accepted bound a lie. Both the packet's 8-byte variable-text
    // floor and the near-miss 35 must be rejected, naming the key, the accepted
    // 36..=64 range, the offending value and the reason.
    for value in [8_u64, 35] {
        let error = claim_models::checked_claim_limits(
            CLAIM_LOW[0],
            CLAIM_LOW[1],
            CLAIM_LOW[2],
            CLAIM_LOW[3],
            value,
        )
        .expect_err("a variable-text ceiling below the 36-byte UUID syntax floor must reject");
        assert!(
            error.contains("max_text_bytes"),
            "the rejection must name max_text_bytes: {error}"
        );
        assert!(
            error.contains("36..=64"),
            "the rejection must name the accepted 36..=64 range: {error}"
        );
        assert!(
            error.contains(&value.to_string()),
            "the rejection must name the offending value {value}: {error}"
        );
        assert!(
            error.contains("UUID syntax"),
            "the rejection must name the UUID-syntax reason: {error}"
        );
    }
    for value in [36_u64, 64] {
        let limits = claim_models::checked_claim_limits(
            CLAIM_LOW[0],
            CLAIM_LOW[1],
            CLAIM_LOW[2],
            CLAIM_LOW[3],
            value,
        )
        .expect("the 36-byte UUID floor and the 64-byte ceiling are both accepted");
        assert_eq!(
            limits.max_text_bytes, value,
            "the checker must preserve, not narrow, the accepted text ceiling"
        );
    }

    // ── H-REGISTRY: modules `2..=7`, declarations `2..=16`, text `8..=64` ──
    const REGISTRY_BOUNDS: [(&str, usize, u64, u64); 3] = [
        ("max_modules", 0, 2, 7),
        ("max_declarations", 1, 2, 16),
        ("max_text_bytes", 2, 8, 64),
    ];

    for (name, index, low, high) in REGISTRY_BOUNDS {
        for value in [low - 1, high + 1] {
            let mut requested = REGISTRY_LOW;
            requested[index] = value;
            let error =
                registry_models::checked_registry_limits(requested[0], requested[1], requested[2])
                    .expect_err("a bound outside the H-REGISTRY safety ceiling must be rejected");
            assert!(
                error.contains(name),
                "the rejection must name {name}: {error}"
            );
            assert!(
                error.contains(&format!("{low}..={high}")),
                "the rejection must name the legal {low}..={high} range: {error}"
            );
            assert!(
                error.contains(&value.to_string()),
                "the rejection must name the offending value {value}: {error}"
            );
        }
    }

    for requested in [REGISTRY_LOW, REGISTRY_HIGH] {
        let limits =
            registry_models::checked_registry_limits(requested[0], requested[1], requested[2])
                .expect("both documented registry ceiling endpoints are accepted");
        assert_eq!(
            u64::from(limits.max_modules),
            requested[0],
            "the accepted registry module bound must be exact"
        );
        assert_eq!(
            u64::try_from(limits.max_declarations).expect("registry declarations fit u64"),
            requested[1],
            "the accepted registry declaration bound must be exact"
        );
        assert_eq!(
            u64::try_from(limits.max_text_bytes).expect("registry text bound fits u64"),
            requested[2],
            "the accepted registry text bound must be exact"
        );
        assert_eq!(
            limits.max_modules,
            u16::try_from(requested[0]).expect("registry module bound fits u16"),
            "u16 narrowing must not silently change the accepted module bound"
        );
        assert_eq!(
            limits.declaration_capacity,
            usize::from(limits.max_modules) * (registry_models::COMMUNITY_POOL_SIZE + 2),
            "the allocation ceiling must be modules times the finite semantic pool"
        );
    }
}

/// Part A (continued): the shrink audits each charter's generative
/// `run_property` executes are reachable from this named control.
///
/// Each audit runs at both accepted safety-ceiling extremes (the `H-REGISTRY`
/// low extreme at its real 8-byte text floor and at the named 36-byte point).
/// The same entry points then reject one bound just outside each ceiling with
/// the model's own checked-bound error, so an audit cannot be reached through a
/// profile the charter has not already accepted. A shrink-audit failure is an
/// assertion panic inside the model, exactly as it is inside `run_property`;
/// `unwrap_or_else` surfaces that panic unchanged.
fn assert_shrink_audits_run_from_bound_control() {
    // ── H-DAG: both accepted node ceilings ───────────────────────────────
    for max_nodes in [DAG_LOW_MAX_NODES, DAG_HIGH_MAX_NODES] {
        dag_models::run_shrink_audit(max_nodes).unwrap_or_else(|error| {
            panic!(
                "the H-DAG shrink audit must run at the accepted max_nodes ceiling {max_nodes}: {error}"
            )
        });
    }
    let dag_outside = DAG_HIGH_MAX_NODES + 1;
    let dag_error = dag_models::run_shrink_audit(dag_outside)
        .expect_err("the H-DAG shrink audit must reject a node bound above its ceiling");
    assert!(
        dag_error.contains("max_nodes")
            && dag_error.contains("3..=7")
            && dag_error.contains(&dag_outside.to_string()),
        "the H-DAG audit rejection must be checked_max_nodes' own error, got {dag_error:?}"
    );

    // ── H-CLAIMS: the full accepted profile at both extremes ─────────────
    for [max_modules, max_claims, max_observations, max_layers, max_text_bytes] in
        [CLAIM_LOW, CLAIM_HIGH]
    {
        claim_models::run_shrink_audit(
            max_modules,
            max_claims,
            max_observations,
            max_layers,
            max_text_bytes,
        )
        .unwrap_or_else(|error| {
            panic!(
                "the H-CLAIMS shrink audit must run at the accepted ceiling ({max_modules}, {max_claims}, {max_observations}, {max_layers}, {max_text_bytes}): {error}"
            )
        });
    }
    let claim_text_floor = CLAIM_LOW[4] - 1;
    let claim_error = claim_models::run_shrink_audit(
        CLAIM_LOW[0],
        CLAIM_LOW[1],
        CLAIM_LOW[2],
        CLAIM_LOW[3],
        claim_text_floor,
    )
    .expect_err("the H-CLAIMS shrink audit must reject a text ceiling below its floor");
    assert!(
        claim_error.contains("max_text_bytes")
            && claim_error.contains("36..=64")
            && claim_error.contains(&claim_text_floor.to_string())
            && claim_error.contains("UUID syntax"),
        "the H-CLAIMS audit rejection must be checked_claim_limits' own error, got {claim_error:?}"
    );

    // ── H-REGISTRY: the full accepted profile at both extremes, plus the
    //    named 36-byte text point ────────────────────────────────────────
    for [max_modules, max_declarations, max_text_bytes] in [REGISTRY_LOW, [2, 2, 36], REGISTRY_HIGH]
    {
        registry_models::run_shrink_audit(max_modules, max_declarations, max_text_bytes)
            .unwrap_or_else(|error| {
                panic!(
                    "the H-REGISTRY shrink audit must run at the accepted ceiling ({max_modules}, {max_declarations}, {max_text_bytes}): {error}"
                )
            });
    }
    let registry_module_floor = REGISTRY_LOW[0] - 1;
    let registry_error =
        registry_models::run_shrink_audit(registry_module_floor, REGISTRY_LOW[1], REGISTRY_LOW[2])
            .expect_err("the H-REGISTRY shrink audit must reject a module bound below its floor");
    assert!(
        registry_error.contains("max_modules")
            && registry_error.contains("2..=7")
            && registry_error.contains(&registry_module_floor.to_string()),
        "the H-REGISTRY audit rejection must be checked_registry_limits' own error, got {registry_error:?}"
    );
}

/// AC-N3 Part A and Part B in one deterministic control.
///
/// Part A calls the checked-bound builders, then executes the `H-DAG`,
/// `H-CLAIMS` and `H-REGISTRY` shrink audits through each model's
/// `run_shrink_audit` at both accepted safety-ceiling extremes. The shrink audits
/// are therefore executed from this named C control, not only from the
/// generative `run_property` entrypoints. Part B drives the foundation harness.
///
/// This is the only test in this binary that touches `PNP_PROPERTY_CONTEXT` or a
/// Proptest override variable, so those process-global mutations cannot race
/// another test.
#[test]
fn scheduler_property_bound_and_shrink_rejections() {
    assert_scheduler_bounds_reject_unsafe_requests();
    assert_shrink_audits_run_from_bound_control();

    let env = EnvironmentGuard::clear(&HARNESS_ENV);
    let artifacts = TempArtifactDir::new("scheduler-bound-controls");
    let base_dir = artifacts.path().join("context-only");
    let base = context_value(TEST_NAME, &base_dir);

    // ── Missing context is its own named failure ──────────────────────────
    let missing = PropertyRun::new(PROPERTY_ID, TEST_NAME)
        .err()
        .expect("a missing PNP_PROPERTY_CONTEXT must not yield a run");
    assert_eq!(
        missing.kind(),
        "missing_property_context",
        "missing context must be distinguishable from an invalid one: {missing}"
    );
    assert!(
        missing
            .to_string()
            .contains("cargo xtask test --summary --properties"),
        "the missing-context message must name the registry-backed command: {missing}"
    );

    // ── Invalid contexts ──────────────────────────────────────────────────
    let mut wrong_schema = base.clone();
    wrong_schema["schema_version"] = json!(2);
    expect_new_rejection(
        &wrong_schema,
        "invalid_property_context",
        &["unsupported schema_version"],
    );

    let mut missing_target_dir = base.clone();
    assert!(
        missing_target_dir
            .as_object_mut()
            .expect("the context is a JSON object")
            .remove("cargo_target_dir")
            .is_some(),
        "the fixture must actually carry cargo_target_dir before removing it"
    );
    expect_new_rejection(
        &missing_target_dir,
        "invalid_property_context",
        &["missing cargo_target_dir"],
    );

    let positive_fields =
        "cases, max_default_size_range, max_shrink_iters and generator_version must be positive";
    for field in ["cases", "max_default_size_range", "max_shrink_iters"] {
        let mut zero = base.clone();
        zero["profile"][field] = json!(0);
        expect_new_rejection(&zero, "invalid_property_context", &[positive_fields]);
    }
    let mut zero_generator = base.clone();
    zero_generator["generator_version"] = json!(0);
    expect_new_rejection(
        &zero_generator,
        "invalid_property_context",
        &[positive_fields],
    );

    for empty in ["run_token", "artifact_dir"] {
        let mut context = base.clone();
        context[empty] = json!("");
        expect_new_rejection(
            &context,
            "invalid_property_context",
            &["run identity and artifact_dir must be nonempty"],
        );
    }

    let mut in_process_timeout = base.clone();
    in_process_timeout["profile"]["timeout_ms"] = json!(1);
    expect_new_rejection(
        &in_process_timeout,
        "invalid_property_context",
        &["a nonzero timeout requires fork isolation"],
    );

    // ── Identity mismatch names both identities ───────────────────────────
    let mut other_property = base.clone();
    other_property["property_id"] = json!("SCTL-OTHER");
    expect_new_rejection(
        &other_property,
        "context_identity_mismatch",
        &[PROPERTY_ID, "SCTL-OTHER"],
    );

    let mut other_test = base.clone();
    other_test["test_name"] = json!("other_property_test");
    expect_new_rejection(
        &other_test,
        "context_identity_mismatch",
        &[TEST_NAME, "other_property_test"],
    );

    // ── `bound` rejects an unknown key and a zero value ───────────────────
    let run = valid_run(&base);
    assert_eq!(
        run.bound("known").expect("the known bound is positive"),
        7,
        "a present positive bound must be returned unchanged"
    );
    let unknown = run
        .bound("absent_key")
        .expect_err("an absent bound key must not default to a value");
    assert_eq!(unknown.kind(), "unknown_bound", "{unknown}");
    assert!(
        unknown.to_string().contains("no bound named absent_key"),
        "the unknown-bound message must name the missing key: {unknown}"
    );
    let zero = run
        .bound("zero")
        .expect_err("a zero bound must not be accepted as a positive budget");
    assert_eq!(zero.kind(), "invalid_bound", "{zero}");
    assert!(
        zero.to_string().contains("bound zero must be positive"),
        "the invalid-bound message must name the zero key: {zero}"
    );

    // ── An ineligible context cannot claim measured completion ────────────
    let ineligible_dir = artifacts.path().join("ineligible");
    let mut ineligible = context_value(TEST_NAME, &ineligible_dir);
    ineligible["completion_eligible"] = json!(false);
    let run = valid_run(&ineligible);
    let error = run
        .check(&Just(0_u64), |_value, _guard| -> TestCaseResult { Ok(()) })
        .expect_err("an ineligible context must fail the run, not silently pass");
    assert_eq!(error.kind(), "harness", "{error}");
    assert!(
        error
            .to_string()
            .contains("not eligible for measured completion"),
        "the harness message must name measured completion: {error}"
    );
    assert!(
        !ineligible_dir.join("observations.jsonl").exists(),
        "the ineligible run must fail before initializing accepted-case evidence"
    );
    assert_eq!(
        failure_report(&ineligible_dir)["error_kind"],
        "harness",
        "the retained failure report must classify the ineligibility as a harness refusal"
    );

    // ── Exhaustion is a failure, never a passing run ──────────────────────
    let exhausted_dir = artifacts.path().join("exhausted");
    let global_reject_budget = 3_u64;
    let mut exhausted = context_value(TEST_NAME, &exhausted_dir);
    exhausted["profile"]["max_global_rejects"] = json!(global_reject_budget);
    let run = valid_run(&exhausted);
    let error = run
        .check(&Just(0_u64), |_value, _guard| -> TestCaseResult {
            Err(TestCaseError::reject(
                "bound control strategy always rejects",
            ))
        })
        .expect_err("an always-rejecting strategy must exhaust, not pass");
    assert_eq!(error.kind(), "exhausted", "{error}");
    assert!(
        error.to_string().contains("Too many global rejects"),
        "the exhaustion message must name the global rejection budget: {error}"
    );
    let rejected = observations(&exhausted_dir);
    assert!(
        !rejected.is_empty(),
        "an exhausted run must retain its rejection observations"
    );
    assert!(
        rejected.iter().all(|observation| {
            matches!(observation["result"].as_str(), Some("started" | "rejected"))
        }),
        "an exhausted run may only record started/rejected observations: {rejected:?}"
    );
    let rejections = rejected
        .iter()
        .filter(|observation| observation["result"] == "rejected")
        .count();
    assert!(
        u64::try_from(rejections).expect("rejection count fits u64") > global_reject_budget,
        "the run must actually exceed the {global_reject_budget} global-rejection budget, \
         observed {rejections} rejections"
    );
    assert!(
        rejected
            .iter()
            .all(|observation| observation["result"] != "passed"),
        "no acceptance may be recorded while the rejection budget is exhausted: {rejected:?}"
    );
    assert_eq!(
        failure_report(&exhausted_dir)["error_kind"],
        "exhausted",
        "the retained failure report must classify exhaustion as exhausted"
    );

    // ── A Proptest override conflicts with the measured profile ───────────
    let override_dir = artifacts.path().join("override");
    let override_context = context_value(TEST_NAME, &override_dir);
    let run = valid_run(&override_context);
    env.set(OVERRIDE_CASES_ENV, "1");
    let error = run
        .check(&Just(0_u64), |_value, _guard| -> TestCaseResult { Ok(()) })
        .expect_err("a Proptest override must not be silently applied");
    assert_eq!(error.kind(), "profile_override_conflict", "{error}");
    assert!(
        error.to_string().contains(OVERRIDE_CASES_ENV),
        "the override conflict must name the offending variable: {error}"
    );
    assert!(
        error.to_string().contains("measured property profile"),
        "the override conflict must name the measured profile: {error}"
    );
    assert!(
        !override_dir.join("observations.jsonl").exists(),
        "the override must be rejected before any case executes"
    );
    env.remove(OVERRIDE_CASES_ENV);

    // ── A passing two-case run writes the promised evidence ───────────────
    let passing_dir = artifacts.path().join("passing");
    let passing = context_value(TEST_NAME, &passing_dir);
    let passing_json = passing.to_string();
    std::env::set_var(CONTEXT_ENV, &passing_json);
    let run = PropertyRun::new(PROPERTY_ID, TEST_NAME).expect("the passing context is valid");
    run.check(&Just(7_u64), |value, _guard| -> TestCaseResult {
        assert_eq!(
            value, 7,
            "the passing strategy value must reach the callback unchanged"
        );
        Ok(())
    })
    .expect("a passing two-case run must be accepted");

    for name in ["context.json", "case.txt", "observations.jsonl"] {
        assert!(
            passing_dir.join(name).is_file(),
            "a passing run must write {name}"
        );
    }
    assert_eq!(
        fs::read_to_string(passing_dir.join("context.json")).expect("context.json"),
        passing_json,
        "context.json must retain the exact selected context"
    );
    assert!(
        !passing_dir.join("failure.json").exists(),
        "a passing run must not retain a failure report"
    );

    let cases = 2_usize;
    let accepted = observations(&passing_dir);
    assert!(
        accepted.iter().all(|observation| {
            matches!(observation["result"].as_str(), Some("started" | "passed"))
        }),
        "a passing run must not record rejections or failures: {accepted:?}"
    );
    let passed_rows: Vec<&Value> = accepted
        .iter()
        .filter(|observation| observation["result"] == "passed")
        .collect();
    let passed = passed_rows.len();
    assert!(
        passed >= cases,
        "the accepted-case gate requires at least {cases} passed calls, observed {passed}"
    );
    assert_eq!(
        passed, cases,
        "a passing two-case run must record exactly its accepted calls"
    );
    assert_eq!(
        accepted
            .iter()
            .filter(|observation| observation["result"] == "started")
            .count(),
        cases,
        "every accepted call must have its pre-callback started record"
    );
    for (index, observation) in passed_rows.iter().enumerate() {
        assert_eq!(
            observation["phase"],
            json!("initial"),
            "a passing run must not enter the shrink phase"
        );
        assert_eq!(
            observation["attempt_index"],
            json!(index),
            "accepted calls must be recorded in their execution order"
        );
    }

    let recorded_cases =
        fs::read_to_string(passing_dir.join("case.txt")).expect("case.txt is written");
    assert_eq!(
        recorded_cases.matches("--- attempt ").count(),
        cases,
        "case.txt must retain one block per accepted call, got {recorded_cases:?}"
    );
    assert!(
        recorded_cases.contains("--- attempt 0 phase initial ---"),
        "case.txt must retain the first initial attempt: {recorded_cases:?}"
    );
    assert!(
        recorded_cases.contains("--- attempt 1 phase initial ---"),
        "case.txt must retain the second initial attempt: {recorded_cases:?}"
    );

    drop(env);
}
