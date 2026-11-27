//! Closure controls for the fail-closed registry census.
//!
//! Each test drives the real census (`census_scoped`, the same census every
//! property run executes) with real Cargo metadata, the real Cargo launcher
//! and actual feature-enabled `--list` discovery. A passing control on the
//! live registry precedes each negative so a census that fails for an
//! unrelated reason cannot pass the negative by accident.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::properties::executor::census_scoped;
use crate::properties::{parse_registry, run_token, Metadata, Registry};

fn root() -> PathBuf {
    crate::build_guests::workspace_root()
}

fn live_registry(root: &Path) -> Registry {
    let metadata = Metadata::load(root).expect("workspace Cargo metadata");
    let input = fs::read_to_string(root.join("test-properties.toml")).expect("live registry");
    parse_registry(&input, &metadata).expect("valid live registry")
}

fn log_dir(root: &Path) -> PathBuf {
    let dir = root
        .join("target/property-testing/closure-census")
        .join(run_token());
    fs::create_dir_all(&dir).expect("create census log directory");
    dir
}

fn census(root: &Path, registry: &Registry, package: &str, log: &Path) -> (i32, String) {
    let scope = BTreeSet::from([package.to_owned()]);
    let code = census_scoped(root, registry, false, &scope, log);
    (code, fs::read_to_string(log).expect("read census log"))
}

#[test]
fn closure_registry_census_rejects_unregistered_target() {
    let root = root();
    let live = live_registry(&root);
    let dir = log_dir(&root);
    let owned: Vec<_> = live
        .property
        .iter()
        .filter(|record| record.package == "slicer-helpers")
        .cloned()
        .collect();
    assert_eq!(
        owned.len(),
        1,
        "the fixture removes the only record of an eligible target"
    );
    let record = &owned[0];
    let listing = format!("{}: test", record.test_name);

    let (code, text) = census(
        &root,
        &live,
        "slicer-helpers",
        &dir.join("unregistered-control.log"),
    );
    assert_eq!(code, 0, "live registry census must pass:\n{text}");
    assert!(
        text.contains(&listing),
        "the control must really list {listing}:\n{text}"
    );

    let mut bad = live.clone();
    bad.property.retain(|candidate| candidate.id != record.id);
    let (code, text) = census(
        &root,
        &bad,
        "slicer-helpers",
        &dir.join("unregistered-negative.log"),
    );
    assert_ne!(code, 0, "a target without records must fail:\n{text}");
    assert!(
        text.contains(&listing),
        "the unregistered target must still be discovered:\n{text}"
    );
    let expected = format!(
        "unregistered_property_test: slicer-helpers/test/{}: {}",
        record.target_name, record.test_name
    );
    assert!(
        text.lines().any(|line| line == expected),
        "expected `{expected}`:\n{text}"
    );
}

#[test]
fn closure_registry_census_rejects_missing_registered_property() {
    let root = root();
    let live = live_registry(&root);
    let dir = log_dir(&root);

    let (code, text) = census(&root, &live, "slicer-ir", &dir.join("missing-control.log"));
    assert_eq!(code, 0, "live registry census must pass:\n{text}");
    let anchor = live
        .property
        .iter()
        .find(|record| record.id == "H-IR-ANCHORS")
        .expect("live IR anchor record");
    assert!(
        text.contains(&format!("{}: test", anchor.test_name)),
        "the control must really list the anchor test:\n{text}"
    );

    let mut bad = live.clone();
    bad.property
        .iter_mut()
        .find(|record| record.id == "H-IR-ANCHORS")
        .expect("live IR anchor record")
        .test_name = "proptest_absent_fixture".to_owned();
    let (code, text) = census(&root, &bad, "slicer-ir", &dir.join("missing-negative.log"));
    assert_ne!(
        code, 0,
        "an undiscoverable registered test must fail:\n{text}"
    );
    assert!(
        text.lines().any(|line| {
            line
            == "missing_property_execution: registered test proptest_absent_fixture not discovered"
        }),
        "expected missing_property_execution for proptest_absent_fixture:\n{text}"
    );
}
