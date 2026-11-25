//! `geometry_controls_seeded_fault_is_minimised`: a SUT failure, minimised.
//!
//! The owned-shrink-witness controls (`controls_shrink`) prove the geometry
//! value trees keep their witnesses while they shrink, but they never see a
//! failing SUT. This control closes that gap end to end: it runs the **exact**
//! `H-BOOL-SET` property body (`boolean::check_input_with`) and strategy
//! (`boolean::strategy`) through the real [`PropertyRun`] harness, against a
//! deliberately broken SUT, and audits what the harness retained.
//!
//! ## The seeded fault
//!
//! `union` silently drops every hole of its result; the other three wrappers
//! are the real production ones. Two rectangles can never union to a holed
//! region, so the fault is invisible to the rectangle family and only a
//! generated *geometry* operand can expose it — the failure, and therefore the
//! shrink, run through the owned `GeometryValueTree`.
//!
//! ## How the run is executed
//!
//! For each tier, the parent re-executes this test binary for this one test
//! with a synthetic `PNP_PROPERTY_CONTEXT` carrying `H-BOOL-SET`'s **registry
//! profile for that tier**, read from `test-properties.toml` (so the control
//! cannot drift from the registry), under a control-only property ID that is
//! deliberately not a registry record, and with every Proptest override
//! variable cleared. The child's `PropertyRun::check` must fail; the parent then
//! reads `failure.json` and `observations.jsonl`.
//!
//! ## What is asserted, from the retained artifacts only
//!
//! At both tiers:
//!
//! * the run failed as `sut_or_contract`, and the message names `Union`, the
//!   faulted operation, so an unrelated failure cannot satisfy the control;
//! * the first failing callback ran in the `initial` phase;
//! * at least one shrink-phase callback **failed with a different input**:
//!   Proptest accepted a simplification, so a SUT failure really was minimised;
//! * the reported `concrete_input` is an input a callback actually ran and saw
//!   fail (not necessarily the last one; see `audit_tier`), and it
//!   differs from the initial failure.
//!
//! At the extended tier additionally: the number of shrink-phase callbacks is
//! **strictly below** the profile's `max_shrink_iters` with
//! `max_shrink_time_ms = 0`. In Proptest 1.11 `TestRunner::shrink` makes exactly
//! one test call per iteration (the harness installs no result cache) and bails
//! only when the iteration count reaches the cap, so fewer calls than the cap
//! means shrinking stopped because the value tree could neither simplify nor
//! complicate further: the counterexample is a local minimum, not a budget
//! cut-off.
//!
//! The routine tier is deliberately **not** held to that: its 64-iteration
//! budget is measured to be too small for this fault (every measured seed
//! still needed 110–167 shrink calls), so a routine failure is minimised but
//! truncated. That is recorded in `docs/testing/proptest-geometry-evidence.md`
//! as a profile finding, not hidden by this control.

use std::path::{Path, PathBuf};
use std::process::Command;

use proptest::test_runner::TestCaseError;
use serde_json::{json, Value};
use slicer_core::polygon_ops::{difference, intersection, union, xor};
use slicer_ir::ExPolygon;
use slicer_sdk::test_support::property_testing::PropertyRun;

use super::boolean::{check_input_with, strategy, validate_limits, BoolOutputs, PROPERTY_ID};
use super::bounds::RawGeometryBounds;

/// Control-only property ID; not a registry record.
const CONTROL_ID: &str = "H-BOOL-SET-SEEDED-FAULT";
/// The `#[test]` in `property_geometry_controls.rs` that runs this control.
pub const TEST_NAME: &str = "geometry_controls_seeded_fault_is_minimised";
/// Marks the re-executed child process.
const CHILD_ENV: &str = "PNP_GEOMETRY_SEEDED_FAULT_CHILD";
/// Fixed seed of both runs.
const SEED: u64 = 37;
/// Proptest environment overrides `PropertyRun` refuses; cleared for the child.
const PROPTEST_OVERRIDES: [&str; 13] = [
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
    "PROPTEST_VERBOSE",
];

/// The seeded fault: real wrappers, except that `union` loses its holes.
fn union_drops_holes(a: &[ExPolygon], b: &[ExPolygon]) -> BoolOutputs {
    let mut faulted = union(a, b);
    for polygon in &mut faulted {
        polygon.holes.clear();
    }
    BoolOutputs {
        union: faulted,
        intersection: intersection(a, b),
        difference: difference(a, b),
        xor: xor(a, b),
    }
}

/// `H-BOOL-SET`'s `[property.<tier>]` table from the live registry, as JSON.
fn registry_profile(tier: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-properties.toml");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let registry: toml::Table = text
        .parse()
        .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
    let record = registry["property"]
        .as_array()
        .expect("registry [[property]] array")
        .iter()
        .find(|record| record.get("id").and_then(toml::Value::as_str) == Some(PROPERTY_ID))
        .unwrap_or_else(|| panic!("{PROPERTY_ID} is not in the registry"));
    let profile = record
        .get(tier)
        .unwrap_or_else(|| panic!("{PROPERTY_ID} has no {tier} profile"));
    serde_json::to_value(profile).expect("profile as JSON")
}

/// A `PNP_PROPERTY_CONTEXT` for one tier, under the control-only ID.
fn context(tier: &str, profile: &Value) -> (Value, PathBuf) {
    let token = format!("seeded-fault-{}-{tier}-{SEED}", std::process::id());
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/property-testing")
        .join(&token)
        .join(CONTROL_ID);
    let context = json!({
        "schema_version": 1, "run_token": token, "property_id": CONTROL_ID,
        "tier": tier, "seed": SEED, "package": "slicer-core", "target_kind": "test",
        "target_name": "property_geometry_controls", "test_name": TEST_NAME,
        "features": ["slicer-core/property_testing", "slicer-core/host-algos"],
        "profile": profile,
        "artifact_dir": dir.to_str().expect("UTF-8 artifact path"),
        "proptest_version": "1.11.0", "generator_version": 1,
        "source_revision": "seeded-fault-control", "dirty_paths": [],
        "source_state_digest": "seeded-fault-control",
        "cargo_target_dir": null, "completion_eligible": true
    });
    (context, dir)
}

/// Child side: run the property body against the seeded fault. It must fail.
fn child() {
    let run = PropertyRun::new(CONTROL_ID, TEST_NAME).expect("seeded-fault context");
    let raw = RawGeometryBounds::read(&run).expect("seeded-fault bounds");
    let limits = validate_limits(raw).expect("seeded-fault profile");
    let strategy = strategy(&limits).expect("H-BOOL-SET strategy");
    let error = run
        .check(&strategy, |input, _guard| {
            check_input_with(&input, &limits, &union_drops_holes).map_err(TestCaseError::fail)
        })
        .expect_err("a union that drops holes must fail the H-BOOL-SET property body");
    assert_eq!(
        error.kind(),
        "sut_failure",
        "unexpected failure kind: {error}"
    );
}

fn read_json(path: PathBuf) -> Value {
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// What one seeded run retained, reduced to the audited quantities.
struct Audit {
    shrink_calls: u64,
    max_shrink_iters: u64,
}

/// Run one tier in a child process and audit its artifacts.
fn audit_tier(tier: &str) -> Audit {
    let profile = registry_profile(tier);
    let max_shrink_iters = profile["max_shrink_iters"]
        .as_u64()
        .expect("max_shrink_iters");
    assert_eq!(
        profile["max_shrink_time_ms"], 0,
        "{tier}: a shrink time limit would make the local-minimum argument unsound"
    );
    let (context, dir) = context(tier, &profile);
    let mut command = Command::new(std::env::current_exe().expect("current test binary"));
    command
        .args(["--exact", TEST_NAME, "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, "1")
        .env("PNP_PROPERTY_CONTEXT", context.to_string());
    for name in PROPTEST_OVERRIDES {
        command.env_remove(name);
    }
    let output = command.output().expect("launch the seeded-fault child");
    assert!(
        output.status.success(),
        "{tier}: seeded-fault child failed: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let report = read_json(dir.join("failure.json"));
    assert_eq!(
        report["classification"], "sut_or_contract",
        "{tier}: {report}"
    );
    let message = report["message"].as_str().expect("failure message");
    assert!(
        message.contains("Union"),
        "{tier}: the failure must name the faulted operation, got: {message}"
    );
    let concrete = report["concrete_input"]
        .as_str()
        .expect("failure.json must retain the minimised input");

    let journal =
        std::fs::read_to_string(dir.join("observations.jsonl")).expect("observations journal");
    let finished: Vec<Value> = journal
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("observation JSON"))
        .filter(|observation| observation["result"] != "started")
        .collect();
    let failed: Vec<&Value> = finished
        .iter()
        .filter(|observation| observation["result"] == "failed")
        .collect();
    let first = *failed.first().expect("at least one failing callback");
    assert_eq!(
        first["phase"], "initial",
        "{tier}: the first failure must come from generation, not shrinking"
    );
    let initial_input = first["input"].as_str().expect("initial input");
    let shrink_calls = finished
        .iter()
        .filter(|observation| observation["phase"] == "shrink")
        .count() as u64;
    let accepted = failed
        .iter()
        .filter(|observation| {
            observation["phase"] == "shrink" && observation["input"] != first["input"]
        })
        .count();
    let last = *failed.last().expect("a last failing callback");
    println!(
        "seeded fault {tier} seed={SEED}: {shrink_calls} shrink call(s) of a \
         {max_shrink_iters}-iteration budget, {accepted} accepted simplification(s), \
         initial input {} bytes, minimised input {} bytes",
        initial_input.len(),
        concrete.len()
    );
    assert!(
        accepted >= 1,
        "{tier}: no simplification of the failing input was accepted, so nothing was \
         minimised ({shrink_calls} shrink call(s))"
    );
    // The reported counterexample must be an input the harness actually ran and
    // saw fail. It need not be the *last* failure: when the iteration cap is
    // reached Proptest backs out with `complicate()` until it returns false,
    // which on a compound tree can retreat past several failing steps.
    let reported = failed
        .iter()
        .find(|observation| observation["input"].as_str() == Some(concrete))
        .unwrap_or_else(|| {
            panic!("{tier}: failure.json reports an input no callback ever failed on: {concrete}")
        });
    println!(
        "seeded fault {tier}: reported counterexample is attempt {} ({}), last failure is \
         attempt {}",
        reported["attempt_index"], reported["phase"], last["attempt_index"]
    );
    assert_ne!(
        concrete, initial_input,
        "{tier}: the reported counterexample must differ from the initial failure"
    );
    Audit {
        shrink_calls,
        max_shrink_iters,
    }
}

/// Parent side: audit both registry tiers; child side: run the seeded fault.
pub fn run() {
    if std::env::var_os(CHILD_ENV).is_some() {
        child();
        return;
    }
    audit_tier("routine");
    let extended = audit_tier("extended");
    assert!(
        extended.shrink_calls < extended.max_shrink_iters,
        "extended: shrinking used {} call(s), reaching the {}-iteration cap, so the \
         counterexample is not proven to be a local minimum",
        extended.shrink_calls,
        extended.max_shrink_iters
    );
}
