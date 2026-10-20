//! Gate A oracle TDD: the five exact tests that turn the recorded Classic
//! fixtures and the isolated kernel into a falsifiable portability verdict.
//!
//! The oracle is the fixture. A construction is never compared to another
//! construction, and a reference value is never fed back into construction.
//! Missing evidence is `inconclusive`, a measured difference is `fail`, and only
//! full equality is `pass`; a `fail` is a result, not a test failure.

#![allow(clippy::result_large_err)]

#[path = "lightning_oracle_support.rs"]
mod support;

use std::collections::{BTreeMap, BTreeSet};

use lightning_feasibility_kernel::{
    BoundaryAddress, BoundaryContribution, ContributionRelation, DerivedRegionPolicy, DomainKind,
    FillPolicyInput, GateAConstruction, GeometryOperation, KernelError, LightningParameters,
    PlanningLayer, SourceSurface,
};
use serde_json::Value;
use slicer_ir::{ExPolygon, Point2};
use support::{
    aggregate_status, compare_case, status_label, CaseStatus, GroupVerdict, WallStrategy,
    COORD_T_TOLERANCE_MM, EXPECTED_CASE_IDS, SCALAR_TOLERANCE_MM,
};

// -----------------------------------------------------------------------------
// AC-1: fixture provenance
// -----------------------------------------------------------------------------

#[test]
fn fixture_provenance() {
    let fixtures = support::load_fixtures()
        .unwrap_or_else(|error| panic!("checked-in lightning_gate fixtures do not load: {error}"));
    support::validate_population(&fixtures).unwrap_or_else(|error| {
        panic!("fixture population is not the manifest population: {error}")
    });

    // A missing fixture directory is an error, never a skip.
    let absent_dir = support::fixture_dir().join("absent-fixture-dir");
    let missing = support::load_fixtures_from(&absent_dir);
    assert!(
        missing.is_err(),
        "loading a fixture directory that does not exist must fail, not produce an empty oracle"
    );

    // Every manifest case resolves a strict input and oracle envelope.
    for case_id in EXPECTED_CASE_IDS {
        let input = support::input_case(&fixtures, case_id)
            .unwrap_or_else(|error| panic!("inputs case `{case_id}`: {error}"));
        let oracle = support::oracle_case(&fixtures, case_id)
            .unwrap_or_else(|error| panic!("oracle case `{case_id}`: {error}"));
        let manifest_case = support::manifest_case(&fixtures, case_id)
            .unwrap_or_else(|error| panic!("manifest case `{case_id}`: {error}"));
        assert_eq!(input.case_id, case_id);
        assert_eq!(oracle.case_id, case_id);
        assert_eq!(input.wall_strategy, WallStrategy::Classic);
        assert_eq!(oracle.wall_strategy, WallStrategy::Classic);

        // Both envelopes are strict: an unknown key is a decode error.
        let mut input_view = support::case_view(input).expect("input case view serializes");
        input_view
            .as_object_mut()
            .expect("input case is an object")
            .insert("unexpected_envelope_key".to_string(), serde_json::json!(true));
        assert!(
            support::GateInputCase::from_value(&input_view, 0).is_err(),
            "inputs case `{case_id}` accepted an unknown envelope key"
        );
        let mut oracle_view = support::oracle_case_to_value(oracle);
        oracle_view
            .as_object_mut()
            .expect("oracle case is an object")
            .insert("unexpected_envelope_key".to_string(), serde_json::json!(true));
        assert!(
            support::GateOracleCase::from_value(&oracle_view, 0).is_err(),
            "oracle case `{case_id}` accepted an unknown envelope key"
        );

        // All five evidence hashes are present as a digest (none is "absent"):
        // the recorder published every subtree for this capture.
        for (slot, recorded) in [
            ("a_source_sha256", manifest_case.a_source_sha256.as_str()),
            ("a_raw_config_sha256", manifest_case.a_raw_config_sha256.as_str()),
            (
                "a_earlier_provenance_sha256",
                manifest_case.a_earlier_provenance_sha256.as_str(),
            ),
            ("b_input_sha256", manifest_case.b_input_sha256.as_str()),
            ("reference_sha256", manifest_case.reference_sha256.as_str()),
        ] {
            assert!(
                support::is_sha256_hex(recorded),
                "manifest case `{case_id}` slot `{slot}` is not a 64-hex digest: {recorded:?}"
            );
        }

        // Every present hash matches the canonical JSON of its own subtree.
        let recomputed = support::recompute_hashes_for(&fixtures, case_id)
            .unwrap_or_else(|error| panic!("case `{case_id}` hashes cannot be recomputed: {error}"));
        let verification = support::verify_case_hashes(manifest_case, &recomputed);
        assert!(
            verification.mismatches.is_empty(),
            "hash mismatches for `{case_id}`: {:?}",
            verification.mismatches
        );
        assert!(
            verification.absent.is_empty(),
            "case `{case_id}` declares absent hashes although every subtree was published: {:?}",
            verification.absent
        );
        for slot in [
            "a_source_sha256",
            "a_raw_config_sha256",
            "a_earlier_provenance_sha256",
            "b_input_sha256",
            "reference_sha256",
        ] {
            assert!(
                verification.verified.contains(&slot),
                "case `{case_id}` did not verify present slot `{slot}`"
            );
        }

        // Negative control (AC-N1): a tampered evidence digest is detected, so
        // the verification above is not a success-only assertion.
        let mut tampered = manifest_case.clone();
        let mut tampered_digest = tampered.a_source_sha256.clone().into_bytes();
        tampered_digest[0] = if tampered_digest[0] == b'0' { b'1' } else { b'0' };
        tampered.a_source_sha256 =
            String::from_utf8(tampered_digest).expect("a hex digest is ASCII");
        let tampered_verification = support::verify_case_hashes(&tampered, &recomputed);
        assert!(
            tampered_verification
                .mismatches
                .iter()
                .any(|mismatch| mismatch.contains("a_source_sha256")),
            "case `{case_id}` did not detect a tampered `a_source_sha256`: {:?}",
            tampered_verification.mismatches
        );
        assert!(
            !tampered_verification
                .verified
                .contains(&"a_source_sha256"),
            "case `{case_id}` reported a tampered `a_source_sha256` as verified"
        );

        // Any population the capture could not transcribe is declared here with
        // a reason. The test does not require a specific population to stay
        // unavailable, so a later capture that transcribes one does not fail it;
        // AC-2 checks that every reported unavailable population is declared.
        for (population, reason) in &manifest_case.absent {
            assert!(
                !reason.trim().is_empty(),
                "case `{case_id}` declares `{population}` absent without a reason"
            );
        }
    }

    // The manifest identifies the canonical revision and the frozen recorder patch.
    let capture = &fixtures.manifest.capture;
    assert_eq!(fixtures.manifest.gate, "A");
    assert!(!fixtures.manifest.status.is_empty());
    assert!(
        capture.canonical_revision.len() == 40
            && capture.canonical_revision.bytes().all(|b| b.is_ascii_hexdigit()),
        "canonical_revision is not a 40-hex commit: {:?}",
        capture.canonical_revision
    );
    assert!(
        capture.canonical_build_revision.len() == 40
            && capture
                .canonical_build_revision
                .bytes()
                .all(|b| b.is_ascii_hexdigit()),
        "canonical_build_revision is not a 40-hex commit: {:?}",
        capture.canonical_build_revision
    );
    assert!(capture.recorder_patch_applied);
    assert!(
        support::is_sha256_hex(&capture.recorder_patch_sha256),
        "recorder_patch_sha256 is not a digest: {:?}",
        capture.recorder_patch_sha256
    );
    assert!(
        support::is_sha256_hex(&capture.manifest_sha256),
        "manifest_sha256 is not a digest: {:?}",
        capture.manifest_sha256
    );
    assert!(
        capture.canonical_build_dirty,
        "the recorder patch build is expected dirty; a clean build would mean the patch did not apply"
    );
    assert_eq!(fixtures.manifest.units.pnp_units_per_canonical_unit, 100);
    assert_eq!(fixtures.manifest.units.transport.tolerance_pnp_units, 1);
    assert_eq!(fixtures.manifest.units.reference, "canonical_units_1nm");
    assert_eq!(fixtures.manifest.units.pnp, "pnp_units_100nm");

    // A missing case cannot be skipped silently: the remaining four files still
    // decode, but the population check names the dropped case.
    let dropped = EXPECTED_CASE_IDS[2];
    let mut reduced = fixtures.inputs.clone();
    reduced.cases.retain(|case| case.case_id != dropped);
    let population = support::validate_population_values(&reduced, &fixtures.oracle, &fixtures.manifest);
    let error = population.expect_err("a missing inputs case must be a named population error");
    assert!(
        error.contains(dropped),
        "the missing-case error must name `{dropped}`: {error}"
    );

    // The oracle side must also name a dropped case.
    let mut reduced_oracle = fixtures.oracle.clone();
    reduced_oracle.cases.retain(|case| case.case_id != dropped);
    let population =
        support::validate_population_values(&fixtures.inputs, &reduced_oracle, &fixtures.manifest);
    let error = population.expect_err("a missing oracle case must be a named population error");
    assert!(
        error.contains(dropped),
        "the missing-oracle-case error must name `{dropped}`: {error}"
    );

    eprintln!(
        "fixture_provenance: {} cases, canonical_revision {}, recorder_patch {}",
        EXPECTED_CASE_IDS.len(),
        capture.canonical_revision,
        capture.recorder_patch_sha256
    );
}

// -----------------------------------------------------------------------------
// AC-2: the Gate A decision
// -----------------------------------------------------------------------------

#[test]
fn gate_a_decision() {
    let fixtures = support::load_fixtures()
        .unwrap_or_else(|error| panic!("checked-in lightning_gate fixtures do not load: {error}"));
    support::validate_population(&fixtures)
        .unwrap_or_else(|error| panic!("fixture population is not the manifest population: {error}"));

    let mut attempted = BTreeSet::new();
    let mut decisions = Vec::new();
    for case_id in EXPECTED_CASE_IDS {
        let input = support::input_case(&fixtures, case_id)
            .unwrap_or_else(|error| panic!("inputs case `{case_id}`: {error}"));
        let oracle = support::oracle_case(&fixtures, case_id)
            .unwrap_or_else(|error| panic!("oracle case `{case_id}`: {error}"));
        let manifest_case = support::manifest_case(&fixtures, case_id)
            .unwrap_or_else(|error| panic!("manifest case `{case_id}`: {error}"));

        let view = support::case_view(input)
            .unwrap_or_else(|error| panic!("case `{case_id}` cannot be viewed: {error}"));
        let execution = support::execute_case(&view);
        attempted.insert(case_id);

        // The projector hands only the five restricted A fields to the
        // extractor: no `b_input`/`reference` value can reach construction.
        let projected: serde_json::Value =
            serde_json::from_slice(&execution.projected).expect("projected bytes are JSON");
        let projected_keys: BTreeSet<&str> = projected
            .as_object()
            .expect("projected a_source is an object")
            .keys()
            .map(String::as_str)
            .collect();
        let expected_keys: BTreeSet<&str> = [
            "object_id",
            "layers",
            "canonical_region_order",
            "object_config",
            "provenance",
        ]
        .into_iter()
        .collect();
        assert_eq!(
            projected_keys, expected_keys,
            "case `{case_id}` projection carried a non-A key"
        );
        let projected_text = String::from_utf8(execution.projected.clone()).expect("bytes are UTF-8");
        assert!(!projected_text.contains("\"b_input\""));
        assert!(!projected_text.contains("\"reference\""));

        // Negative control: poisoning the B/reference sibling keys in the same
        // view cannot change the projected bytes or the construction.
        let mut poisoned_view = view.clone();
        poisoned_view["b_input"] = serde_json::json!({
            "input": {"outlines": [{"poison": true}]},
            "contributor_evidence": [{"dispatch_token": "poison", "status": "Unavailable", "reason": "poison"}],
        });
        poisoned_view["reference"] = serde_json::json!({
            "planning_layers": [],
            "policies": [],
            "parameters": {"poison": true},
        });
        let poisoned = support::execute_case(&poisoned_view);
        assert_eq!(
            poisoned.projected, execution.projected,
            "case `{case_id}` projection changed when B/reference siblings were poisoned"
        );
        assert_eq!(
            poisoned.construction, execution.construction,
            "case `{case_id}` construction changed when B/reference siblings were poisoned"
        );

        let decision = compare_case(
            input,
            oracle,
            manifest_case,
            execution.construction.as_ref(),
            execution.error.as_ref(),
        );

        // Every case is attempted and names either its construction or its error.
        assert!(
            execution.construction.is_some() || execution.error.is_some(),
            "case `{case_id}` produced neither construction nor error"
        );

        // Every unavailable population the comparator reports must be declared
        // by the manifest with the recorded reason: availability is never
        // inferred, and an undeclared absence is a fixture defect.
        for reason in &decision.unavailable {
            assert!(
                manifest_case
                    .absent
                    .iter()
                    .any(|(population, declared)| reason.contains(population.as_str())
                        && reason.contains(declared.as_str())),
                "case `{case_id}` reports an unavailable population the manifest does not declare: {reason}"
            );
        }

        match decision.status {
            CaseStatus::Pass => {
                assert!(
                    decision.mismatches.is_empty() && decision.unavailable.is_empty(),
                    "case `{case_id}` is pass but carries named evidence: {:?}",
                    decision.named_evidence()
                );
            }
            CaseStatus::Fail => {
                assert!(
                    !decision.mismatches.is_empty(),
                    "case `{case_id}` is fail without a named mismatch"
                );
            }
            CaseStatus::Inconclusive => {
                assert!(
                    !decision.unavailable.is_empty(),
                    "case `{case_id}` is inconclusive without a named unavailable population"
                );
            }
        }

        // The comparator's status must equal a genuinely independent derivation:
        // this path reads the recorded reference directly, builds its own group
        // verdicts, and applies the documented precedence itself. It never calls
        // `compare_case` or `recompute_status`.
        match execution.construction.as_ref() {
            Some(construction) => {
                let independent = independently_derive_case(construction, &oracle.reference);
                assert_eq!(
                    decision.groups.len(),
                    independent.len(),
                    "case `{case_id}` comparator group count differs from the independent derivation"
                );
                let independent_status_value = independent_status(&independent);
                assert_eq!(
                    decision.status,
                    independent_status_value,
                    "case `{case_id}` comparator status {} does not equal the independently derived status {} (independent: {independent:?}; comparator: {:?})",
                    status_label(decision.status),
                    status_label(independent_status_value),
                    decision.groups
                );
                for group in &independent {
                    let comparator_group = decision.group(group.group).unwrap_or_else(|| {
                        panic!(
                            "case `{case_id}` comparator has no `{}` group",
                            group.group
                        )
                    });
                    assert_eq!(
                        independent_verdict_label(group.verdict),
                        comparator_verdict_label(comparator_group.verdict),
                        "case `{case_id}` `{}`: independent verdict {} (mismatches {:?}, unavailable {:?}) differs from comparator verdict {} (mismatches {:?}, unavailable {:?})",
                        group.group,
                        independent_verdict_label(group.verdict),
                        group.mismatches,
                        group.unavailable,
                        comparator_verdict_label(comparator_group.verdict),
                        comparator_group.mismatches,
                        comparator_group.unavailable
                    );
                }
            }
            None => {
                // A case that produced no construction is classified from the
                // observed error alone: a measured input mismatch is `fail`,
                // missing source evidence is `inconclusive`.
                let error = execution
                    .error
                    .as_ref()
                    .unwrap_or_else(|| panic!("case `{case_id}` produced neither construction nor error"));
                let measured = matches!(
                    error,
                    KernelError::SourceProvenanceMismatch { .. }
                        | KernelError::RequiredSourceInputMismatch { .. }
                        | KernelError::InvalidInput { .. }
                );
                let independent_status_value = if measured {
                    CaseStatus::Fail
                } else {
                    CaseStatus::Inconclusive
                };
                assert_eq!(
                    decision.status, independent_status_value,
                    "case `{case_id}` comparator status {} does not match the error classification {}",
                    status_label(decision.status),
                    status_label(independent_status_value)
                );
            }
        }

        eprintln!(
            "gate_a_decision: {case_id} -> {} ({} named evidence entries)",
            status_label(decision.status),
            decision.named_evidence().len()
        );
        for evidence in decision.named_evidence() {
            eprintln!("  {evidence}");
        }
        decisions.push(decision);
    }

    // Negative control (AC-N1): emptying the recorded planning population can
    // never yield `pass`; the comparator must name the population unavailable.
    let anchor_input = support::input_case(&fixtures, "anchor_bridge_classic")
        .unwrap_or_else(|error| panic!("anchor_bridge_classic inputs: {error}"));
    let anchor_oracle = support::oracle_case(&fixtures, "anchor_bridge_classic")
        .unwrap_or_else(|error| panic!("anchor_bridge_classic oracle: {error}"));
    let anchor_manifest = support::manifest_case(&fixtures, "anchor_bridge_classic")
        .unwrap_or_else(|error| panic!("anchor_bridge_classic manifest: {error}"));
    let anchor_view = support::case_view(anchor_input).expect("anchor case view serializes");
    let anchor_execution = support::execute_case(&anchor_view);
    let mut emptied = anchor_oracle.clone();
    emptied.reference.planning_layers.clear();
    let emptied_decision = compare_case(
        anchor_input,
        &emptied,
        anchor_manifest,
        anchor_execution.construction.as_ref(),
        anchor_execution.error.as_ref(),
    );
    assert_eq!(
        emptied_decision.status,
        CaseStatus::Inconclusive,
        "an emptied reference.planning_layers must be inconclusive, got {}: {:?}",
        status_label(emptied_decision.status),
        emptied_decision.named_evidence()
    );
    let emptied_population = emptied_decision
        .group("planning_layers.population")
        .expect("the emptied reference still forms a population group");
    assert_eq!(
        emptied_population.verdict,
        GroupVerdict::Inconclusive,
        "an emptied reference population must not be judged"
    );
    assert!(
        emptied_population
            .unavailable
            .iter()
            .any(|reason| reason.contains("no planning layer")),
        "the emptied-population verdict must name the missing planning population: {:?}",
        emptied_population.unavailable
    );

    // The independent derivation must be able to fail and to be inconclusive, so
    // the status equality asserted above is not a tautology.
    let anchor_construction = anchor_execution
        .construction
        .as_ref()
        .expect("anchor_bridge_classic constructs");
    let emptied_groups = independently_derive_case(anchor_construction, &emptied.reference);
    assert_eq!(
        independent_status(&emptied_groups),
        CaseStatus::Inconclusive,
        "the independent derivation must report an emptied reference population as inconclusive"
    );
    assert!(
        emptied_groups
            .iter()
            .find(|group| group.group == "planning_layers.population")
            .is_some_and(|group| group.verdict == IndependentVerdict::Inconclusive
                && group.unavailable.iter().any(|reason| reason.contains("no planning layer"))),
        "the independent derivation must name the emptied planning population: {emptied_groups:?}"
    );
    assert!(
        !anchor_oracle.reference.planning_layers.is_empty()
            && !anchor_oracle.reference.planning_layers[0].internal_domains.is_empty(),
        "the anchor reference must carry an internal domain for the perturbation control"
    );
    let mut perturbed = anchor_oracle.clone();
    perturbed.reference.planning_layers[0].internal_domains[0]
        .contour
        .points[0]
        .x += 100_000;
    let perturbed_groups = independently_derive_case(anchor_construction, &perturbed.reference);
    assert_eq!(
        independent_status(&perturbed_groups),
        CaseStatus::Fail,
        "a 0.1 mm recorded-domain shift must be an independent fail: {:?}",
        perturbed_groups
    );

    // Negative control (F6): a constructed boundary edge with no recorded
    // counterpart is a named mismatch in both the comparator and the independent
    // derivation, not a count hidden in the group detail.
    let forged_layer_index = recorded_layer_map(&anchor_oracle.reference)
        .keys()
        .copied()
        .find(|index| {
            anchor_construction
                .planning_layers
                .iter()
                .any(|layer| {
                    layer.global_layer_index == *index && !layer.boundary_provenance.is_empty()
                })
        })
        .expect("anchor_bridge_classic records a planned layer with boundary provenance");
    let forged_position = anchor_construction
        .planning_layers
        .iter()
        .position(|layer| layer.global_layer_index == forged_layer_index)
        .expect("the forged layer exists in the construction");
    let mut forged = anchor_construction.clone();
    let duplicate = forged.planning_layers[forged_position].boundary_provenance[0].clone();
    forged.planning_layers[forged_position]
        .boundary_provenance
        .push(duplicate);
    let forged_decision = compare_case(
        anchor_input,
        anchor_oracle,
        anchor_manifest,
        Some(&forged),
        None,
    );
    let forged_boundary = forged_decision
        .group("planning_layers.boundary_provenance")
        .expect("the forged construction still forms a boundary group");
    assert_eq!(
        forged_boundary.verdict,
        GroupVerdict::Fail,
        "an unrecorded derived boundary edge must fail the boundary group: {:?}",
        forged_boundary.mismatches
    );
    assert!(
        forged_boundary
            .mismatches
            .iter()
            .any(|mismatch| mismatch.contains("derived boundary edge has no recorded counterpart")),
        "the unrecorded derived edge must be named: {:?}",
        forged_boundary.mismatches
    );
    assert_eq!(
        independent_status(&independently_derive_case(&forged, &anchor_oracle.reference)),
        CaseStatus::Fail,
        "the independent derivation must also reject an unrecorded derived boundary edge"
    );

    let expected_attempted: BTreeSet<&str> = fixtures
        .manifest
        .case_ids
        .iter()
        .map(String::as_str)
        .collect();
    assert_eq!(
        attempted, expected_attempted,
        "the attempted population is not the manifest population"
    );
    assert_eq!(
        decisions.len(),
        attempted.len(),
        "a case produced more than one decision; the population is not one decision per case"
    );
    eprintln!(
        "gate_a_decision aggregate: {}",
        status_label(aggregate_status(&decisions))
    );
}

// -----------------------------------------------------------------------------
// AC-3: a wrong final domain is rejected
// -----------------------------------------------------------------------------

#[test]
fn wrong_domain_is_rejected() {
    let fixtures = support::load_fixtures()
        .unwrap_or_else(|error| panic!("checked-in lightning_gate fixtures do not load: {error}"));
    let oracle = support::oracle_case(&fixtures, "anchor_bridge_classic")
        .unwrap_or_else(|error| panic!("anchor_bridge_classic oracle: {error}"));
    let replay = &oracle.reference.replay;

    // Both graphs must be present; a missing one is the named field, never a
    // weakened assertion.
    assert!(
        !replay.faithful.graph.is_empty(),
        "replay.faithful.graph is empty: the faithful replay graph is missing"
    );
    assert!(
        !replay.production.graph.is_empty(),
        "replay.production.graph is empty: the independently recorded production graph is missing"
    );
    assert!(
        !replay.wrong.graph.is_empty(),
        "replay.wrong.graph is empty: the wrong-domain replay graph is missing"
    );

    for (label, graph) in [
        ("faithful", &replay.faithful),
        ("production", &replay.production),
        ("wrong", &replay.wrong),
    ] {
        let nodes: u64 = graph.graph.iter().map(|layer| layer.nodes.len() as u64).sum();
        let edges: u64 = graph.graph.iter().map(|layer| layer.edges.len() as u64).sum();
        let roots: u64 = graph.graph.iter().map(|layer| layer.root_count).sum();
        assert_eq!(graph.node_count, nodes, "{label} node_count is not its graph");
        assert_eq!(graph.edge_count, edges, "{label} edge_count is not its graph");
        assert_eq!(graph.root_count, roots, "{label} root_count is not its graph");
        for layer in &graph.graph {
            for node in &layer.nodes {
                let expected = [node.point[0] as f64 / 1_000_000.0, node.point[1] as f64 / 1_000_000.0];
                assert!(
                    (node.point_mm[0] - expected[0]).abs() <= 1e-9
                        && (node.point_mm[1] - expected[1]).abs() <= 1e-9,
                    "node {} point_mm is not its canonical nm point/1e6: {node:?}",
                    node.node_id
                );
            }
        }
    }

    // The faithful graph matches the independently recorded production graph.
    let faithful_matches_production = replay.faithful.graph == replay.production.graph
        && replay.faithful.node_count == replay.production.node_count
        && replay.faithful.edge_count == replay.production.edge_count
        && replay.faithful.root_count == replay.production.root_count;
    assert!(
        faithful_matches_production,
        "the faithful replay graph does not match the independently recorded production graph"
    );
    assert!(
        replay.faithful_matches_production,
        "the recorder's faithful_matches_production flag disagrees with the graph comparison"
    );

    // The wrong final domain changes the graph.
    let wrong_differs = replay.faithful.graph != replay.wrong.graph;
    assert!(
        wrong_differs,
        "the wrong-domain replay graph is identical to the faithful graph; the discriminator is dead"
    );
    assert!(
        replay.topology_differs,
        "the recorder's topology_differs flag disagrees with the graph comparison"
    );
    assert!(
        replay.wrong.node_count > replay.faithful.node_count,
        "the wrong-domain graph is not larger: {} vs {}",
        replay.wrong.node_count,
        replay.faithful.node_count
    );

    // Aligned per-layer symmetric-difference area: positive and equal to the
    // recorded capture value. Canonical mm², recorded to 1e-11.
    let area = replay.domain_symmetric_difference_area;
    assert!(
        area.is_finite() && area > 0.0,
        "the aligned symmetric-difference area must be positive, got {area}"
    );
    assert!(
        (area - support::ANCHOR_BRIDGE_SYMMETRIC_DIFFERENCE_MM2).abs()
            <= support::REPLAY_AREA_TOLERANCE_MM2,
        "aligned symmetric-difference area is {area} mm², expected {} ± {} mm²",
        support::ANCHOR_BRIDGE_SYMMETRIC_DIFFERENCE_MM2,
        support::REPLAY_AREA_TOLERANCE_MM2
    );
    assert_ne!(
        replay.faithful_domain_polygon_count, replay.wrong_domain_polygon_count,
        "the faithful and wrong domain populations did not differ"
    );
    assert!(replay.wrong_nonempty, "the wrong-domain replay populated no polygons");
    assert!(replay.scalars_resolved, "the replay did not resolve its scalar inputs");
    assert!(
        replay.snapshot_unchanged && replay.snapshot_before == replay.snapshot_after,
        "the replay mutated its pre-anchor snapshot: {} -> {}",
        replay.snapshot_before,
        replay.snapshot_after
    );

    eprintln!(
        "wrong_domain_is_rejected: faithful {} nodes/{} edges, wrong {} nodes/{} edges, area {area} mm², domain polygons {} vs {}",
        replay.faithful.node_count,
        replay.faithful.edge_count,
        replay.wrong.node_count,
        replay.wrong.edge_count,
        replay.faithful_domain_polygon_count,
        replay.wrong_domain_polygon_count
    );
}

// -----------------------------------------------------------------------------
// AC-4: source-only projection is isolated
// -----------------------------------------------------------------------------

#[test]
fn source_only_projection_is_isolated() {
    let fixtures = support::load_fixtures()
        .unwrap_or_else(|error| panic!("checked-in lightning_gate fixtures do not load: {error}"));
    let input = support::input_case(&fixtures, "anchor_bridge_classic")
        .unwrap_or_else(|error| panic!("anchor_bridge_classic inputs: {error}"));
    let oracle = support::oracle_case(&fixtures, "anchor_bridge_classic")
        .unwrap_or_else(|error| panic!("anchor_bridge_classic oracle: {error}"));
    let manifest_case = support::manifest_case(&fixtures, "anchor_bridge_classic")
        .unwrap_or_else(|error| panic!("anchor_bridge_classic manifest: {error}"));

    let view = support::case_view(input).expect("case view serializes");
    let baseline = support::execute_case(&view);
    let baseline_construction = baseline
        .construction
        .clone()
        .expect("the anchor_bridge_classic source subtree must construct");
    let baseline_source = support::extract_gate_a_source(&baseline.projected)
        .expect("the projected bytes decode strictly");
    let baseline_hashes = support::source_hashes(&baseline_source);
    let recorded_hashes = support::recompute_hashes_for(&fixtures, "anchor_bridge_classic")
        .expect("recorded hashes recompute");
    assert_eq!(recorded_hashes.a_source_sha256, manifest_case.a_source_sha256);
    assert_eq!(
        recorded_hashes.a_raw_config_sha256,
        manifest_case.a_raw_config_sha256
    );
    assert_eq!(
        recorded_hashes.a_earlier_provenance_sha256,
        manifest_case.a_earlier_provenance_sha256
    );

    // Independently remove, replace, and poison the B/reference siblings.
    let mut variants: Vec<(&str, serde_json::Value)> = Vec::new();

    let mut removed_b = view.clone();
    removed_b
        .as_object_mut()
        .expect("case view is an object")
        .remove("b_input");
    variants.push(("b_input removed", removed_b));

    let mut null_b = view.clone();
    null_b["b_input"] = serde_json::Value::Null;
    variants.push(("b_input replaced with null", null_b));

    let mut poisoned_b = view.clone();
    poisoned_b["b_input"] = serde_json::json!({
        "input": {
            "object_id": "poisoned-object",
            "layers": [{"poison": true}],
            "planning_layers": [{"internal_domains": []}],
            "canonical_region_order": [],
            "scalar_source_region": {"object_id": "poisoned-object", "region_id": 9, "variant_chain": []},
            "parameters": {"density_fraction": 99.0},
        },
        "contributor_evidence": [{"dispatch_token": "poison", "status": "Unavailable", "reason": "poison"}],
    });
    variants.push(("b_input poisoned", poisoned_b));

    let mut poisoned_reference = view.clone();
    poisoned_reference["reference"] = serde_json::json!({
        "poison": true,
        "planning_layers": [],
        "policies": [],
    });
    variants.push(("reference poisoned", poisoned_reference));

    let mut replaced_reference = view.clone();
    replaced_reference["reference"] = support::reference_to_value(&oracle.reference);
    variants.push(("reference replaced with the recorded reference", replaced_reference));

    for (label, variant) in &variants {
        let execution = support::execute_case(variant);
        assert_eq!(
            execution.projected, baseline.projected,
            "projected a_source bytes changed when {label}"
        );
        let source = support::extract_gate_a_source(&execution.projected)
            .unwrap_or_else(|error| panic!("projected bytes do not decode after {label}: {error}"));
        let hashes = support::source_hashes(&source);
        assert_eq!(
            hashes, baseline_hashes,
            "source/config/provenance hashes changed when {label}"
        );
        assert_eq!(
            execution.construction.as_ref(),
            Some(&baseline_construction),
            "the complete GateAConstruction changed when {label}"
        );
        eprintln!(
            "source_only_projection_is_isolated: {label} -> typed_a_source_sha256 {} unchanged (typed re-encode; distinct from the raw-subtree `a_source_sha256` recorded in the manifest and outcome.json)",
            hashes.typed_a_source_sha256
        );
    }

    // A discriminating reference-only change alters only the comparator verdict.
    let baseline_decision = compare_case(
        input,
        oracle,
        manifest_case,
        Some(&baseline_construction),
        None,
    );
    assert_ne!(baseline_decision.status, CaseStatus::Pass);

    let mut policy_poisoned = oracle.clone();
    policy_poisoned.reference.policies = Some(Vec::new());
    let policy_decision = compare_case(
        input,
        &policy_poisoned,
        manifest_case,
        Some(&baseline_construction),
        None,
    );
    assert_eq!(
        baseline_decision
            .group("policies")
            .expect("baseline has a policies group")
            .verdict,
        GroupVerdict::Inconclusive,
        "the recorded reference must leave policies unavailable"
    );
    assert_eq!(
        policy_decision
            .group("policies")
            .expect("poisoned has a policies group")
            .verdict,
        GroupVerdict::Fail,
        "an empty reference policy population must be a measured mismatch"
    );
    assert_ne!(
        policy_decision.named_evidence(),
        baseline_decision.named_evidence(),
        "poisoning the reference policies did not change the comparator verdict"
    );

    // The independent derivation carries its own policy-content comparison, and
    // the recorded cases never exercise it: every `reference.policies` in
    // canonical.json is null, so `independent_policies` returns at its null
    // branch before any policy content is compared. Feed the same policy-poisoned
    // reference through the independent path so that branch is measured, not
    // merely compiled.
    assert!(
        !baseline_construction.policies.is_empty(),
        "the anchor construction must derive at least one policy for the independent policy control; \
         an empty derived population would make the poisoned comparison vacuous"
    );
    let independent_baseline_groups =
        independently_derive_case(&baseline_construction, &oracle.reference);
    assert_eq!(
        independent_baseline_groups
            .iter()
            .find(|group| group.group == "policies")
            .expect("the independent derivation always forms a policies group")
            .verdict,
        IndependentVerdict::Inconclusive,
        "the recorded reference must leave the independent policies group unavailable"
    );
    let independent_policy_groups =
        independently_derive_case(&baseline_construction, &policy_poisoned.reference);
    let independent_policy_group = independent_policy_groups
        .iter()
        .find(|group| group.group == "policies")
        .expect("the independent derivation always forms a policies group");
    assert_eq!(
        independent_policy_group.verdict,
        IndependentVerdict::Fail,
        "an empty reference policy population must be a measured mismatch in the independent derivation: \
         mismatches {:?}, unavailable {:?}",
        independent_policy_group.mismatches,
        independent_policy_group.unavailable
    );
    assert!(
        !independent_policy_group.mismatches.is_empty(),
        "the independent policies group must name the poisoned mismatch, not fail silently"
    );
    assert_eq!(
        independent_status(&independent_policy_groups),
        CaseStatus::Fail,
        "the independent derivation must derive `fail` from the poisoned policy population: {:?}",
        independent_policy_groups
    );
    let independent_baseline_verdict = independent_baseline_groups
        .iter()
        .find(|group| group.group == "policies")
        .expect("the independent derivation always forms a policies group")
        .verdict;
    eprintln!(
        "source_only_projection_is_isolated: independent policies group {} -> {} ({} named mismatch(es), first: {})",
        independent_verdict_label(independent_baseline_verdict),
        independent_verdict_label(independent_policy_group.verdict),
        independent_policy_group.mismatches.len(),
        independent_policy_group
            .mismatches
            .first()
            .map(String::as_str)
            .unwrap_or("<none>")
    );

    // The two controls above only reach `independent_policies`' null branch and
    // its population loop: no recorded case carries a `policies` array, so the
    // field-by-field comparator (`independent_compare_fill_policy`) never ran.
    // Feed it a reference policy list cloned from the construction's own derived
    // policies. Cloning from the construction is acceptable here **only** because
    // the clone is an *input operand to the comparator*, never an expectation
    // compared to itself: the comparator's own left-hand side is the construction,
    // and a self-referential comparison would report `pass` for the mutated case
    // too. The single-field mutation flipping the group to `fail` is therefore the
    // signal that a matched policy key is genuinely compared field-by-field.
    let cloned_policies: Vec<DerivedRegionPolicy> = baseline_construction.policies.clone();
    assert!(
        !cloned_policies.is_empty(),
        "the anchor construction must derive at least one policy for the matched-key control; \
         an empty clone would make the field-by-field comparison vacuous"
    );
    let mut synthetic_reference = oracle.reference.clone();
    synthetic_reference.policies = Some(cloned_policies);
    let unchanged_policy_groups =
        independently_derive_case(&baseline_construction, &synthetic_reference);
    let unchanged_policy_group = unchanged_policy_groups
        .iter()
        .find(|group| group.group == "policies")
        .expect("the independent derivation always forms a policies group");
    assert_eq!(
        unchanged_policy_group.verdict,
        IndependentVerdict::Pass,
        "a reference policy list cloned unchanged from the construction must be compared \
         field-by-field and agree: mismatches {:?}, unavailable {:?}",
        unchanged_policy_group.mismatches,
        unchanged_policy_group.unavailable
    );
    assert!(
        unchanged_policy_group.mismatches.is_empty()
            && unchanged_policy_group.unavailable.is_empty(),
        "a pass for a matched policy key must carry no named evidence: mismatches {:?}, unavailable {:?}",
        unchanged_policy_group.mismatches,
        unchanged_policy_group.unavailable
    );
    let unchanged_synthetic_policy_count = synthetic_reference
        .policies
        .as_ref()
        .map(Vec::len)
        .unwrap_or(0);
    assert_eq!(
        unchanged_synthetic_policy_count,
        baseline_construction.policies.len(),
        "the synthetic reference must carry every derived policy so every key is matched"
    );

    // Mutate exactly ONE field the comparator inspects: `fixed_angle`, which
    // `independent_compare_fill_policy` compares as `{key}.fixed_angle`.
    let mut mutated_reference = synthetic_reference.clone();
    let mutated_policies = mutated_reference
        .policies
        .as_mut()
        .expect("the synthetic reference carries the cloned policy list");
    let mutated_policy = mutated_policies
        .first_mut()
        .expect("the synthetic policy list is non-empty");
    mutated_policy.policy.fixed_angle = !mutated_policy.policy.fixed_angle;
    let mutated_policy_groups =
        independently_derive_case(&baseline_construction, &mutated_reference);
    let mutated_policy_group = mutated_policy_groups
        .iter()
        .find(|group| group.group == "policies")
        .expect("the independent derivation always forms a policies group");
    assert_eq!(
        mutated_policy_group.verdict,
        IndependentVerdict::Fail,
        "a single mutated `fixed_angle` must fail the matched policy key: mismatches {:?}, unavailable {:?}",
        mutated_policy_group.mismatches,
        mutated_policy_group.unavailable
    );
    assert!(
        mutated_policy_group
            .mismatches
            .iter()
            .any(|mismatch| mismatch.contains(".fixed_angle")),
        "the single-field mutation must be named by the field comparator as `.fixed_angle`: {:?}",
        mutated_policy_group.mismatches
    );
    assert_eq!(
        independent_status(&mutated_policy_groups),
        CaseStatus::Fail,
        "the independent derivation must derive `fail` from the one-field policy mutation: {:?}",
        mutated_policy_groups
    );
    // The unchanged clone and the mutated clone differ in exactly one input field,
    // so exactly one group may change verdict: anything else would mean the
    // mutation leaked into an unrelated comparison, and the `fail` above would no
    // longer be attributable to the policy field.
    let changed_groups: Vec<&str> = unchanged_policy_groups
        .iter()
        .zip(mutated_policy_groups.iter())
        .filter(|(unchanged, mutated)| unchanged.verdict != mutated.verdict)
        .map(|(unchanged, _)| unchanged.group)
        .collect();
    assert_eq!(
        changed_groups,
        vec!["policies"],
        "the one-field policy mutation must change only the policies group verdict"
    );
    eprintln!(
        "source_only_projection_is_isolated: independent policies group (matched key, unchanged clone) -> {} ({} named mismatch(es))",
        independent_verdict_label(unchanged_policy_group.verdict),
        unchanged_policy_group.mismatches.len()
    );
    eprintln!(
        "source_only_projection_is_isolated: independent policies group (single-field mutation `fixed_angle`) -> {} ({} named mismatch(es), first: {})",
        independent_verdict_label(mutated_policy_group.verdict),
        mutated_policy_group.mismatches.len(),
        mutated_policy_group
            .mismatches
            .first()
            .map(String::as_str)
            .unwrap_or("<none>")
    );

    let mut geometry_poisoned = oracle.clone();
    geometry_poisoned.reference.planning_layers[0].internal_domains[0]
        .contour
        .points[0]
        .x += 100_000;
    let geometry_decision = compare_case(
        input,
        &geometry_poisoned,
        manifest_case,
        Some(&baseline_construction),
        None,
    );
    assert_eq!(
        geometry_decision
            .group("planning_layers.domains")
            .expect("geometry decision has a domains group")
            .verdict,
        GroupVerdict::Fail,
        "a 0.1 mm reference-domain shift (100 000 canonical units, 1 unit = 1 nm) must be a measured mismatch"
    );
    assert_ne!(
        geometry_decision.named_evidence(),
        baseline_decision.named_evidence(),
        "a perturbed reference domain did not change the named comparator evidence"
    );
    assert_eq!(
        support::source_hashes(&baseline_source),
        baseline_hashes,
        "reference-only changes altered the A source hashes"
    );

    // Unknown reference-like keys inside `a_source` fail the whitelist.
    let mut unknown_top = view.clone();
    unknown_top["a_source"]["planning_layers"] = serde_json::json!([]);
    let error = support::project_gate_a(&unknown_top)
        .expect_err("a reference-like key inside a_source must fail the whitelist");
    assert!(
        error.to_string().contains("unknown field"),
        "the whitelist error must name the unknown field: {error}"
    );

    let mut unknown_nested = view.clone();
    unknown_nested["a_source"]["object_config"]["resolved_width_mm"] = serde_json::json!(0.45);
    let error = support::project_gate_a(&unknown_nested)
        .expect_err("an unknown nested config key must fail the whitelist");
    assert!(
        error.to_string().contains("unknown field"),
        "the nested whitelist error must name the unknown field: {error}"
    );

    eprintln!(
        "source_only_projection_is_isolated: {} B/reference variants isolated, policies {} -> {}, domains {}",
        variants.len(),
        status_label(baseline_decision.status),
        status_label(policy_decision.status),
        status_label(geometry_decision.status)
    );
}

// -----------------------------------------------------------------------------
// AC-5: the outcome records Gate A truthfully
// -----------------------------------------------------------------------------

#[test]
fn outcome_records_gate_a() {
    let fixtures = support::load_fixtures()
        .unwrap_or_else(|error| panic!("checked-in lightning_gate fixtures do not load: {error}"));
    support::validate_population(&fixtures)
        .unwrap_or_else(|error| panic!("fixture population is not the manifest population: {error}"));

    // Recompute the decision independently of any written record.
    let mut decisions = Vec::new();
    let mut hashes = BTreeMap::new();
    for case_id in EXPECTED_CASE_IDS {
        let input = support::input_case(&fixtures, case_id)
            .unwrap_or_else(|error| panic!("inputs case `{case_id}`: {error}"));
        let oracle = support::oracle_case(&fixtures, case_id)
            .unwrap_or_else(|error| panic!("oracle case `{case_id}`: {error}"));
        let manifest_case = support::manifest_case(&fixtures, case_id)
            .unwrap_or_else(|error| panic!("manifest case `{case_id}`: {error}"));
        let view = support::case_view(input).expect("case view serializes");
        let execution = support::execute_case(&view);
        let decision = compare_case(
            input,
            oracle,
            manifest_case,
            execution.construction.as_ref(),
            execution.error.as_ref(),
        );
        hashes.insert(
            case_id.to_string(),
            support::recompute_hashes_for(&fixtures, case_id)
                .unwrap_or_else(|error| panic!("case `{case_id}` hashes cannot be recomputed: {error}")),
        );
        decisions.push(decision);
    }
    let aggregate = aggregate_status(&decisions);

    // Observe real exits for the four sibling AC commands in child processes.
    let sibling_filters = [
        "lightning_oracle_tdd::fixture_provenance",
        "lightning_oracle_tdd::gate_a_decision",
        "lightning_oracle_tdd::wrong_domain_is_rejected",
        "lightning_oracle_tdd::source_only_projection_is_isolated",
    ];
    let mut commands: Vec<support::CommandRecord> = sibling_filters
        .iter()
        .map(|filter| support::run_sibling_test(filter))
        .collect();
    commands.push(support::self_command_record("lightning_oracle_tdd::outcome_records_gate_a"));

    let outcome = support::build_outcome(&fixtures, &decisions, &hashes, commands.clone())
        .unwrap_or_else(|error| panic!("outcome cannot be built: {error}"));
    let path = support::write_outcome(&outcome)
        .unwrap_or_else(|error| panic!("outcome cannot be written: {error}"));
    let recorded = support::read_outcome()
        .unwrap_or_else(|error| panic!("outcome cannot be read back strictly: {error}"));

    // Record identity and execution.
    let recorded_object = recorded
        .as_object()
        .unwrap_or_else(|| panic!("the outcome root is not an object: {recorded}"));
    let outcome_str = |name: &str| {
        recorded_object
            .get(name)
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| panic!("outcome.{name} is missing or not a string: {recorded}"))
    };
    let outcome_bool = |name: &str| {
        recorded_object
            .get(name)
            .and_then(Value::as_bool)
            .unwrap_or_else(|| panic!("outcome.{name} is missing or not a boolean: {recorded}"))
    };
    assert_eq!(outcome_str("gate"), "A");
    assert!(outcome_bool("executed"), "the outcome must record that A executed");
    assert_eq!(outcome_str("task_id"), "TASK-576");
    assert_eq!(
        outcome_str("packet"),
        "layer-module-preparation_07_lightning-geometry-portability-gate"
    );
    assert_eq!(
        recorded["schema_version"].as_u64(),
        Some(support::OUTCOME_SCHEMA_VERSION)
    );
    assert!(path.ends_with("target/lightning-oracle/outcome.json"));

    // Per-case statuses equal the independent recomputation, and every case is
    // present exactly once.
    let recorded_cases = recorded["cases"]
        .as_array()
        .unwrap_or_else(|| panic!("outcome.cases is not an array: {recorded}"));
    let recorded_ids: BTreeSet<&str> = recorded_cases
        .iter()
        .map(|case| {
            case.get("case_id")
                .and_then(Value::as_str)
                .unwrap_or_else(|| panic!("outcome case has no case_id: {case}"))
        })
        .collect();
    let recomputed_ids: BTreeSet<&str> = decisions.iter().map(|case| case.case_id.as_str()).collect();
    assert_eq!(recorded_ids, recomputed_ids, "outcome cases are not the decision population");
    for decision in &decisions {
        let case = recorded_cases
            .iter()
            .find(|case| case.get("case_id").and_then(Value::as_str) == Some(decision.case_id.as_str()))
            .unwrap_or_else(|| panic!("outcome has no case `{}`", decision.case_id));
        let status = support::parse_case_status(&case["status"], "outcome case status")
            .unwrap_or_else(|error| panic!("case `{}`: {error}", decision.case_id));
        assert_eq!(
            status, decision.status,
            "outcome status for `{}` is not the recomputed status",
            decision.case_id
        );
        let mismatches = case["mismatches"]
            .as_array()
            .unwrap_or_else(|| panic!("case `{}` mismatches are not an array", decision.case_id));
        let unavailable = case["unavailable"]
            .as_array()
            .unwrap_or_else(|| panic!("case `{}` unavailable is not an array", decision.case_id));
        match status {
            CaseStatus::Pass => {
                assert!(mismatches.is_empty() && unavailable.is_empty());
            }
            CaseStatus::Fail => assert!(!mismatches.is_empty()),
            CaseStatus::Inconclusive => assert!(!unavailable.is_empty()),
        }
        let expected_hashes = hashes
            .get(&decision.case_id)
            .expect("recomputed evidence hashes exist");
        for (field, expected) in [
            ("a_source_sha256", &expected_hashes.a_source_sha256),
            ("a_raw_config_sha256", &expected_hashes.a_raw_config_sha256),
            (
                "a_earlier_provenance_sha256",
                &expected_hashes.a_earlier_provenance_sha256,
            ),
            ("reference_sha256", &expected_hashes.reference_sha256),
        ] {
            let recorded_hash = case[field]
                .as_str()
                .unwrap_or_else(|| panic!("case `{}` has no `{field}`", decision.case_id));
            assert_eq!(&recorded_hash, expected, "case `{}` `{field}`", decision.case_id);
            assert!(support::is_sha256_hex(recorded_hash));
        }
    }

    // Fixture and provenance hashes.
    let fixture_dir = support::fixture_dir();
    let inputs_sha256 =
        support::sha256_file(&fixture_dir.join(support::INPUTS_FILE)).expect("inputs hash");
    let canonical_sha256 =
        support::sha256_file(&fixture_dir.join(support::CANONICAL_FILE)).expect("canonical hash");
    let manifest_sha256 =
        support::sha256_file(&fixture_dir.join(support::MANIFEST_FILE)).expect("manifest hash");
    let out_fixtures = &recorded["fixtures"];
    assert_eq!(out_fixtures["inputs_sha256"].as_str(), Some(inputs_sha256.as_str()));
    assert_eq!(
        out_fixtures["canonical_sha256"].as_str(),
        Some(canonical_sha256.as_str())
    );
    assert_eq!(
        out_fixtures["manifest_sha256"].as_str(),
        Some(manifest_sha256.as_str())
    );
    assert_eq!(
        out_fixtures["manifest_recorded_sha256"].as_str(),
        Some(fixtures.manifest.capture.manifest_sha256.as_str())
    );
    let out_capture = &recorded["capture"];
    assert_eq!(
        out_capture["canonical_revision"].as_str(),
        Some(fixtures.manifest.capture.canonical_revision.as_str())
    );
    assert_eq!(
        out_capture["recorder_patch_sha256"].as_str(),
        Some(fixtures.manifest.capture.recorder_patch_sha256.as_str())
    );
    assert_eq!(
        out_capture["canonical_build_dirty"].as_bool(),
        Some(fixtures.manifest.capture.canonical_build_dirty)
    );

    // Commands and exits actually observed: each sibling filter was executed in
    // a child process whose real argv is recorded (`<test-binary> <filter>
    // --exact --nocapture`); the Cargo line is recorded separately as
    // `equivalent` and was never spawned. This invocation names why its own
    // exit is not observable in-process.
    let recorded_commands = recorded["commands"]
        .as_array()
        .unwrap_or_else(|| panic!("outcome.commands is not an array: {recorded}"));
    assert!(!recorded_commands.is_empty(), "the outcome records no command");
    for filter in sibling_filters {
        let command = recorded_commands
            .iter()
            .find(|command| {
                command["command"].as_str().is_some_and(|text| text.contains(filter))
                    && command["observed_by"]
                        .as_str()
                        .is_some_and(|text| text.contains("child"))
            })
            .unwrap_or_else(|| panic!("the outcome does not record the `{filter}` execution"));
        assert_eq!(
            command["exit"].as_i64(),
            Some(0),
            "`{filter}` did not exit zero as a child process: {command}"
        );
        let argv = command["command"]
            .as_str()
            .unwrap_or_else(|| panic!("`{filter}` records no argv: {command}"));
        assert!(
            argv.contains(filter),
            "`{filter}` argv does not name its own filter: {argv}"
        );
        assert!(
            argv.ends_with("--exact --nocapture"),
            "`{filter}` argv is not `<test-binary> <filter> --exact --nocapture`: {argv}"
        );
        assert!(
            !argv.contains("cargo"),
            "the recorded command must be the argv that ran, not a Cargo line: {argv}"
        );
        let equivalent = command["equivalent"]
            .as_str()
            .unwrap_or_else(|| panic!("`{filter}` records no equivalent Cargo line: {command}"));
        assert!(
            equivalent.starts_with("cargo test -p slicer-wasm-host --test integration ")
                && equivalent.contains(filter)
                && equivalent.ends_with("-- --exact --nocapture"),
            "`{filter}` equivalent Cargo line is not the documented reproduction command: {equivalent}"
        );
    }
    let self_command = recorded_commands
        .iter()
        .find(|command| {
            command["command"]
                .as_str()
                .is_some_and(|text| text.contains("outcome_records_gate_a"))
        })
        .expect("the outcome records its own invocation");
    assert!(
        self_command["exit"].is_null()
            && self_command["exit_reason"].as_str().is_some_and(|reason| !reason.is_empty()),
        "the still-running invocation must name why its exit is unobserved"
    );

    // `executed` is derived from the observed exits, not asserted: the command
    // records above must prove it, and a fabricated failure must flip it.
    assert!(
        support::commands_executed(&commands),
        "the recorded sibling executions do not prove that Gate A executed"
    );
    let mut failed_sibling = commands.clone();
    failed_sibling[0].exit = Some(1);
    let failed_outcome =
        support::build_outcome(&fixtures, &decisions, &hashes, failed_sibling)
            .unwrap_or_else(|error| panic!("outcome cannot be built for a failed sibling: {error}"));
    assert_eq!(
        failed_outcome["executed"].as_bool(),
        Some(false),
        "a sibling that exited non-zero must never be recorded as executed"
    );
    let mut unobserved_sibling = commands.clone();
    unobserved_sibling[0].exit = None;
    unobserved_sibling[0].exit_reason = Some("the child exit could not be observed".to_string());
    let unobserved_outcome =
        support::build_outcome(&fixtures, &decisions, &hashes, unobserved_sibling)
            .unwrap_or_else(|error| panic!("outcome cannot be built for an unobserved sibling: {error}"));
    assert_eq!(
        unobserved_outcome["executed"].as_bool(),
        Some(false),
        "an unobservable sibling exit must never be recorded as executed"
    );

    // Named blockers and migration eligibility: eligibility is derived from the
    // blocker list, so a named Gate B/framework blocker can never coexist with
    // `true`, and a fabricated empty list would flip it.
    let recorded_blockers = recorded["blockers"]
        .as_array()
        .unwrap_or_else(|| panic!("outcome.blockers is not an array: {recorded}"));
    assert_eq!(
        recorded["migration_eligible"].as_bool(),
        Some(recorded_blockers.is_empty()),
        "migration eligibility must be derived from the named blocker list"
    );
    assert_eq!(
        recorded["migration_eligible"].as_bool(),
        Some(false),
        "Gate A alone must never be migration eligible"
    );
    assert!(
        !recorded_blockers.is_empty(),
        "the Gate A outcome must name its migration blockers"
    );
    let gate_b = recorded_blockers
        .iter()
        .find(|blocker| blocker["code"].as_str() == Some(support::GATE_B_BLOCKER))
        .expect("the Gate B blocker must be named");
    assert!(gate_b["detail"]
        .as_str()
        .is_some_and(|detail| detail.contains("Gate B")));
    let framework = recorded_blockers
        .iter()
        .find(|blocker| blocker["code"].as_str() == Some(support::FRAMEWORK_BLOCKER))
        .expect("the framework blocker must be named");
    assert!(framework["detail"]
        .as_str()
        .is_some_and(|detail| detail.contains("ADR-0066")));
    if aggregate != CaseStatus::Pass {
        assert!(
            recorded_blockers
                .iter()
                .any(|blocker| blocker["code"].as_str() == Some(support::GATE_A_BLOCKER)),
            "a non-pass A record must name the Gate A blocker"
        );
    }

    // The source-plan outcome section and the existing TASK-576 item.
    let plan = std::fs::read_to_string(
        support::workspace_root().join("docs/specs/layer-module-preparation-plan.md"),
    )
    .expect("the source plan is readable");
    assert!(
        plan.lines()
            .any(|line| line.trim_end() == "## Lightning geometry/portability gate outcomes"),
        "docs/specs/layer-module-preparation-plan.md has no `## Lightning geometry/portability gate outcomes` section"
    );

    let backlog =
        std::fs::read_to_string(support::workspace_root().join("docs/07_implementation_status.md"))
            .expect("the implementation status file is readable");
    assert!(
        backlog.lines().any(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with("- [") && trimmed.contains("**TASK-576**")
        }),
        "docs/07_implementation_status.md has no TASK-576 checkbox item"
    );
    let outcome_lines: Vec<&str> = backlog
        .lines()
        .filter(|line| line.starts_with("  - TASK-576 Gate A outcome: "))
        .collect();
    let [outcome_line] = outcome_lines.as_slice() else {
        panic!(
            "docs/07_implementation_status.md must carry exactly one `  - TASK-576 Gate A outcome: (pass|fail|inconclusive)` line, found {outcome_lines:?}"
        )
    };
    let recorded_outcome = outcome_line
        .trim_start_matches("  - TASK-576 Gate A outcome: ")
        .trim();
    assert!(
        matches!(recorded_outcome, "pass" | "fail" | "inconclusive"),
        "TASK-576 Gate A outcome must be pass|fail|inconclusive, got {recorded_outcome:?}"
    );
    assert_eq!(
        recorded_outcome,
        status_label(aggregate),
        "the plan's TASK-576 Gate A outcome does not equal the recorded aggregate"
    );

    eprintln!(
        "outcome_records_gate_a: wrote {} with aggregate {}, {} cases, {} commands, {} blockers",
        path.display(),
        status_label(aggregate),
        recorded_cases.len(),
        recorded_commands.len(),
        recorded_blockers.len()
    );
}

// -----------------------------------------------------------------------------
// Independent re-derivation of the Gate A verdict.
//
// This path is test-local: it never calls `support::compare_case`, never calls
// `support::recompute_status`, and never reads the construction to form an
// expectation. Every expected value is read straight from the recorded
// reference, and every verdict is produced by this file's own comparison code.
// The shared support helpers it does use are encodings, not verdicts: the
// documented canonical-nm -> PnP transport rule (`canonical_to_pnp`) and
// canonical JSON (used to key region identities).
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IndependentVerdict {
    Pass,
    Fail,
    Inconclusive,
}

#[derive(Debug)]
struct IndependentGroup {
    group: &'static str,
    verdict: IndependentVerdict,
    mismatches: Vec<String>,
    unavailable: Vec<String>,
}

impl IndependentGroup {
    fn new(group: &'static str) -> Self {
        Self {
            group,
            verdict: IndependentVerdict::Pass,
            mismatches: Vec::new(),
            unavailable: Vec::new(),
        }
    }

    fn mismatch(&mut self, text: String) {
        self.mismatches.push(text);
    }

    fn finish(mut self) -> Self {
        self.verdict = if !self.mismatches.is_empty() {
            IndependentVerdict::Fail
        } else if !self.unavailable.is_empty() {
            IndependentVerdict::Inconclusive
        } else {
            IndependentVerdict::Pass
        };
        self
    }
}

fn independent_verdict_label(verdict: IndependentVerdict) -> &'static str {
    match verdict {
        IndependentVerdict::Pass => "pass",
        IndependentVerdict::Fail => "fail",
        IndependentVerdict::Inconclusive => "inconclusive",
    }
}

fn comparator_verdict_label(verdict: GroupVerdict) -> &'static str {
    match verdict {
        GroupVerdict::Pass => "pass",
        GroupVerdict::Fail => "fail",
        GroupVerdict::Inconclusive => "inconclusive",
    }
}

/// Status precedence, applied here without the support comparator's
/// `recompute_status`: a measured mismatch is `fail` even when another group is
/// unavailable; otherwise an unavailable required group is `inconclusive`;
/// otherwise `pass`.
fn independent_status(groups: &[IndependentGroup]) -> CaseStatus {
    let mut measured_mismatch = false;
    let mut unavailable = false;
    for group in groups {
        match group.verdict {
            IndependentVerdict::Fail => measured_mismatch = true,
            IndependentVerdict::Inconclusive => unavailable = true,
            IndependentVerdict::Pass => {}
        }
    }
    if measured_mismatch {
        CaseStatus::Fail
    } else if unavailable {
        CaseStatus::Inconclusive
    } else {
        CaseStatus::Pass
    }
}

fn independently_derive_case(
    construction: &GateAConstruction,
    reference: &support::GateReference,
) -> Vec<IndependentGroup> {
    let recorded = recorded_layer_map(reference);
    let constructed = constructed_layer_map(construction);
    vec![
        independent_layer_identity(&recorded, &constructed),
        independent_layer_population(&recorded, &constructed),
        independent_domain_geometry(&recorded, &constructed),
        independent_source_regions(&recorded, &constructed),
        independent_source_catalog(&recorded, &constructed),
        independent_boundary_provenance(&recorded, &constructed),
        independent_operations(&recorded, &constructed),
        independent_parameters(&construction.parameters, &reference.parameters),
        independent_scalar_source(construction, reference),
        independent_policies(construction, reference),
    ]
}

fn independent_layer_identity(
    recorded: &BTreeMap<u32, &PlanningLayer>,
    constructed: &BTreeMap<u32, &PlanningLayer>,
) -> IndependentGroup {
    let mut group = IndependentGroup::new("planning_layers.identity");
    for (index, recorded_layer) in recorded {
        let Some(derived_layer) = constructed.get(index) else {
            continue;
        };
        if derived_layer.object_id != recorded_layer.object_id {
            group.mismatch(format!(
                "layer g{index} object_id: recorded {}, derived {}",
                recorded_layer.object_id, derived_layer.object_id
            ));
        }
        if derived_layer.local_layer_index != recorded_layer.local_layer_index {
            group.mismatch(format!(
                "layer g{index} local_layer_index: recorded {}, derived {}",
                recorded_layer.local_layer_index, derived_layer.local_layer_index
            ));
        }
        if (derived_layer.z_mm - recorded_layer.z_mm).abs() > SCALAR_TOLERANCE_MM {
            group.mismatch(format!(
                "layer g{index} z_mm: recorded {}, derived {}",
                recorded_layer.z_mm, derived_layer.z_mm
            ));
        }
    }
    group.finish()
}

fn independent_layer_population(
    recorded: &BTreeMap<u32, &PlanningLayer>,
    constructed: &BTreeMap<u32, &PlanningLayer>,
) -> IndependentGroup {
    let mut group = IndependentGroup::new("planning_layers.population");
    if recorded.is_empty() {
        group
            .unavailable
            .push("the reference records no planning layer".to_string());
        return group.finish();
    }
    for index in recorded.keys() {
        if !constructed.contains_key(index) {
            group.mismatch(format!(
                "the reference records planning layer g{index} but the construction has none"
            ));
        }
    }
    for (index, layer) in constructed {
        if recorded.contains_key(index) {
            continue;
        }
        if !layer.temporary_anchors.is_empty() || !layer.operations.is_empty() {
            group.mismatch(format!(
                "the construction plans unrecorded layer g{index} ({} temporary anchors, {} operations)",
                layer.temporary_anchors.len(),
                layer.operations.len()
            ));
        }
    }
    group.finish()
}

fn independent_domain_geometry(
    recorded: &BTreeMap<u32, &PlanningLayer>,
    constructed: &BTreeMap<u32, &PlanningLayer>,
) -> IndependentGroup {
    let mut group = IndependentGroup::new("planning_layers.domains");
    if recorded.is_empty() {
        group
            .unavailable
            .push("the reference records no planning layer".to_string());
        return group.finish();
    }
    for (index, recorded_layer) in recorded {
        let Some(derived_layer) = constructed.get(index) else {
            group.unavailable.push(format!(
                "planning layer g{index} is recorded but absent from the construction, so its domains are not comparable"
            ));
            continue;
        };
        for (label, derived_set, recorded_set) in [
            ("internal", &derived_layer.internal_domains, &recorded_layer.internal_domains),
            ("void", &derived_layer.void_domains, &recorded_layer.void_domains),
            (
                "temporary_anchor",
                &derived_layer.temporary_anchors,
                &recorded_layer.temporary_anchors,
            ),
        ] {
            independent_compare_polygon_multiset(
                &mut group,
                &format!("g{index}.{label}"),
                derived_set,
                recorded_set,
            );
        }
    }
    group.finish()
}

fn independent_source_regions(
    recorded: &BTreeMap<u32, &PlanningLayer>,
    constructed: &BTreeMap<u32, &PlanningLayer>,
) -> IndependentGroup {
    let mut group = IndependentGroup::new("planning_layers.source_regions");
    for (index, recorded_layer) in recorded {
        let Some(derived_layer) = constructed.get(index) else {
            continue;
        };
        if derived_layer.source_regions != recorded_layer.source_regions {
            group.mismatch(format!("g{index} source_regions differ"));
        }
    }
    group.finish()
}

fn independent_source_catalog(
    recorded: &BTreeMap<u32, &PlanningLayer>,
    constructed: &BTreeMap<u32, &PlanningLayer>,
) -> IndependentGroup {
    let mut group = IndependentGroup::new("planning_layers.source_catalog");
    for (index, recorded_layer) in recorded {
        let Some(derived_layer) = constructed.get(index) else {
            continue;
        };
        let derived_by_token: BTreeMap<&str, &SourceSurface> = derived_layer
            .source_catalog
            .iter()
            .map(|surface| (surface.id.surface_token.as_str(), surface))
            .collect();
        let recorded_by_token: BTreeMap<&str, &SourceSurface> = recorded_layer
            .source_catalog
            .iter()
            .map(|surface| (surface.id.surface_token.as_str(), surface))
            .collect();
        for (token, recorded_surface) in &recorded_by_token {
            let Some(derived_surface) = derived_by_token.get(token) else {
                group.mismatch(format!(
                    "g{index} source_catalog is missing recorded surface `{token}`"
                ));
                continue;
            };
            if derived_surface.surface_kind != recorded_surface.surface_kind {
                group.mismatch(format!(
                    "g{index}.catalog[{token}] surface_kind: recorded {}, derived {}",
                    recorded_surface.surface_kind, derived_surface.surface_kind
                ));
            }
            independent_compare_angle_option(
                &mut group,
                &format!("g{index}.catalog[{token}].solid_angle_rad"),
                derived_surface.solid_angle_rad,
                recorded_surface.solid_angle_rad,
            );
            independent_compare_angle_option(
                &mut group,
                &format!("g{index}.catalog[{token}].bridge_angle_rad"),
                derived_surface.bridge_angle_rad,
                recorded_surface.bridge_angle_rad,
            );
            independent_compare_polygon_multiset(
                &mut group,
                &format!("g{index}.catalog[{token}].geometry"),
                std::slice::from_ref(&derived_surface.geometry),
                std::slice::from_ref(&recorded_surface.geometry),
            );
        }
        for token in derived_by_token.keys() {
            if !recorded_by_token.contains_key(token) {
                group.mismatch(format!(
                    "g{index} source_catalog carries unrecorded surface `{token}`"
                ));
            }
        }
    }
    group.finish()
}

/// One contributed source occurrence. Content, not serialized equality: the
/// source identity (object/region/variant chain), its layer indices and
/// checkpoint, the ring/segment locator, and the exact relation (overlap
/// interval bounds or the generating operation id).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct IndependentContributor {
    identity: String,
    local_layer_index: u32,
    global_layer_index: u32,
    checkpoint: String,
    surface_token: String,
    ring_id: Option<u32>,
    segment_id: Option<u32>,
    relation: String,
}

fn independent_contributor(contributor: &BoundaryContribution) -> IndependentContributor {
    let surface = &contributor.source.surface;
    IndependentContributor {
        identity: support::canonical_json(
            &serde_json::to_value(&surface.identity).expect("identity serializes"),
        ),
        local_layer_index: surface.local_layer_index,
        global_layer_index: surface.global_layer_index,
        checkpoint: format!("{:?}", surface.checkpoint),
        surface_token: surface.surface_token.clone(),
        ring_id: contributor.source.ring_id,
        segment_id: contributor.source.segment_id,
        relation: match &contributor.relation {
            ContributionRelation::Overlap { start, end } => format!(
                "overlap {}/{}..{}/{}",
                start.numerator, start.denominator, end.numerator, end.denominator
            ),
            ContributionRelation::Generated { operation_id } => {
                format!("generated op {operation_id}")
            }
        },
    }
}

fn independent_contributor_multiset(
    contributors: &[BoundaryContribution],
) -> BTreeMap<IndependentContributor, usize> {
    let mut multiset = BTreeMap::new();
    for contributor in contributors {
        *multiset.entry(independent_contributor(contributor)).or_insert(0) += 1;
    }
    multiset
}

fn independent_contributor_summary(contributors: &[BoundaryContribution]) -> Vec<String> {
    contributors
        .iter()
        .map(|contributor| {
            let key = independent_contributor(contributor);
            match &contributor.relation {
                ContributionRelation::Overlap { start, end } => format!(
                    "overlap({} ring {:?} seg {:?} {}/{}-{}/{})",
                    key.surface_token,
                    key.ring_id,
                    key.segment_id,
                    start.numerator,
                    start.denominator,
                    end.numerator,
                    end.denominator
                ),
                ContributionRelation::Generated { operation_id } => format!(
                    "generated({} op {operation_id})",
                    key.surface_token
                ),
            }
        })
        .collect()
}

fn independent_boundary_provenance(
    recorded: &BTreeMap<u32, &PlanningLayer>,
    constructed: &BTreeMap<u32, &PlanningLayer>,
) -> IndependentGroup {
    let mut group = IndependentGroup::new("planning_layers.boundary_provenance");
    for (index, recorded_layer) in recorded {
        let Some(derived_layer) = constructed.get(index) else {
            group.unavailable.push(format!(
                "planning layer g{index} is recorded but absent from the construction, so its boundary provenance is not comparable"
            ));
            continue;
        };
        independent_compare_boundary_layer(&mut group, *index, derived_layer, recorded_layer);
    }
    group.finish()
}

fn independent_compare_boundary_layer(
    group: &mut IndependentGroup,
    layer_index: u32,
    derived: &PlanningLayer,
    recorded: &PlanningLayer,
) {
    let mut used = vec![false; derived.boundary_provenance.len()];
    let mut claimed = vec![false; derived.boundary_provenance.len()];
    for recorded_entry in &recorded.boundary_provenance {
        let recorded_start = independent_point(recorded_entry.endpoints[0]);
        let recorded_end = independent_point(recorded_entry.endpoints[1]);
        let recorded_contributors =
            independent_contributor_multiset(&recorded_entry.contributors);
        let candidates: Vec<usize> = derived
            .boundary_provenance
            .iter()
            .enumerate()
            .filter(|(index, derived_entry)| {
                !used[*index]
                    && !claimed[*index]
                    && derived_entry.address.domain == recorded_entry.address.domain
                    && independent_points_close(derived_entry.endpoints[0], recorded_start)
                    && independent_points_close(derived_entry.endpoints[1], recorded_end)
            })
            .map(|(index, _)| index)
            .collect();
        let exact_match = candidates.iter().copied().find(|index| {
            independent_contributor_multiset(&derived.boundary_provenance[*index].contributors)
                == recorded_contributors
        });
        match (exact_match, candidates.first().copied()) {
            (Some(index), _) => used[index] = true,
            (None, Some(index)) => {
                claimed[index] = true;
                let derived_entry = &derived.boundary_provenance[index];
                group.mismatch(format!(
                    "g{layer_index} {:?} edge {:?}->{:?} contributor content differs: recorded {} {:?}, derived {} {:?}",
                    recorded_entry.address.domain,
                    recorded_entry.endpoints[0],
                    recorded_entry.endpoints[1],
                    recorded_entry.contributors.len(),
                    independent_contributor_summary(&recorded_entry.contributors),
                    derived_entry.contributors.len(),
                    independent_contributor_summary(&derived_entry.contributors)
                ));
            }
            (None, None) => group.mismatch(format!(
                "g{layer_index} {:?} edge {:?}->{:?} has no derived counterpart ({} recorded contributors)",
                recorded_entry.address.domain,
                recorded_entry.endpoints[0],
                recorded_entry.endpoints[1],
                recorded_entry.contributors.len()
            )),
        }
    }
    for (index, derived_entry) in derived.boundary_provenance.iter().enumerate() {
        if used[index] || claimed[index] {
            continue;
        }
        group.mismatch(format!(
            "g{layer_index} derived {:?} edge {:?}->{:?} has no recorded counterpart ({} derived contributors)",
            derived_entry.address.domain,
            derived_entry.endpoints[0],
            derived_entry.endpoints[1],
            derived_entry.contributors.len()
        ));
    }
}

fn independent_operations(
    recorded: &BTreeMap<u32, &PlanningLayer>,
    constructed: &BTreeMap<u32, &PlanningLayer>,
) -> IndependentGroup {
    let mut group = IndependentGroup::new("planning_layers.operations");
    for (index, recorded_layer) in recorded {
        let Some(derived_layer) = constructed.get(index) else {
            group.unavailable.push(format!(
                "planning layer g{index} is recorded but absent from the construction, so its operations are not comparable"
            ));
            continue;
        };
        let derived_by_id: BTreeMap<u32, &GeometryOperation> = derived_layer
            .operations
            .iter()
            .map(|operation| (operation.operation_id, operation))
            .collect();
        let recorded_by_id: BTreeMap<u32, &GeometryOperation> = recorded_layer
            .operations
            .iter()
            .map(|operation| (operation.operation_id, operation))
            .collect();
        for (id, recorded_operation) in &recorded_by_id {
            match derived_by_id.get(id) {
                None => group.mismatch(format!(
                    "g{index} recorded operation {id} ({:?}) is absent from the construction",
                    recorded_operation.kind
                )),
                Some(derived_operation) => independent_compare_operation(
                    &mut group,
                    *index,
                    derived_operation,
                    recorded_operation,
                ),
            }
        }
        for id in derived_by_id.keys() {
            if !recorded_by_id.contains_key(id) {
                group.mismatch(format!(
                    "g{index} derived operation {id} has no recorded counterpart"
                ));
            }
        }
    }
    group.finish()
}

fn independent_compare_operation(
    group: &mut IndependentGroup,
    layer_index: u32,
    derived: &GeometryOperation,
    recorded: &GeometryOperation,
) {
    let label = format!("g{layer_index} operation {}", recorded.operation_id);
    if derived.kind != recorded.kind {
        group.mismatch(format!(
            "{label} kind: recorded {:?}, derived {:?}",
            recorded.kind, derived.kind
        ));
    }
    if derived.checkpoint != recorded.checkpoint {
        group.mismatch(format!(
            "{label} checkpoint: recorded {:?}, derived {:?}",
            recorded.checkpoint, derived.checkpoint
        ));
    }
    if derived.inputs != recorded.inputs {
        group.mismatch(format!("{label} inputs differ from the recorded lineage"));
    }
    if derived.parent_operations != recorded.parent_operations {
        group.mismatch(format!(
            "{label} parent_operations: recorded {:?}, derived {:?}",
            recorded.parent_operations, derived.parent_operations
        ));
    }
    if derived.operands != recorded.operands {
        group.mismatch(format!("{label} operands differ from the recorded lineage"));
    }
    if derived.join != recorded.join {
        group.mismatch(format!(
            "{label} join: recorded {:?}, derived {:?}",
            recorded.join, derived.join
        ));
    }
    if derived.offset_units != recorded.offset_units.map(support::canonical_to_pnp) {
        group.mismatch(format!(
            "{label} offset_units: recorded {:?} (canonical), derived {:?} (PnP)",
            recorded.offset_units, derived.offset_units
        ));
    }
    let derived_outputs: BTreeSet<(u8, u32, u32, u32)> =
        derived.outputs.iter().map(independent_address_key).collect();
    let recorded_outputs: BTreeSet<(u8, u32, u32, u32)> =
        recorded.outputs.iter().map(independent_address_key).collect();
    if derived_outputs != recorded_outputs {
        group.mismatch(format!(
            "{label} outputs: recorded {} addresses, derived {} addresses",
            recorded_outputs.len(),
            derived_outputs.len()
        ));
    }
    independent_compare_polygon_multiset(
        group,
        &format!("{label} result_geometry"),
        &derived.result_geometry,
        &recorded.result_geometry,
    );
}

fn independent_address_key(address: &BoundaryAddress) -> (u8, u32, u32, u32) {
    let domain = match address.domain {
        DomainKind::Internal => 0,
        DomainKind::Void => 1,
        DomainKind::TemporaryAnchor => 2,
    };
    (domain, address.surface_id, address.ring_id, address.segment_id)
}

fn independent_parameters(
    construction: &LightningParameters,
    reference: &LightningParameters,
) -> IndependentGroup {
    let mut group = IndependentGroup::new("parameters");
    for mismatch in independent_parameter_mismatches(construction, reference) {
        group.mismatch(mismatch);
    }
    group.finish()
}

fn independent_scalar_source(
    construction: &GateAConstruction,
    reference: &support::GateReference,
) -> IndependentGroup {
    let mut group = IndependentGroup::new("scalar_source_region");
    if construction.scalar_source_region != reference.scalar_source_region {
        group.mismatch(format!(
            "scalar_source_region: recorded {:?}, derived {:?}",
            reference.scalar_source_region, construction.scalar_source_region
        ));
    }
    group.finish()
}

fn independent_policies(
    construction: &GateAConstruction,
    reference: &support::GateReference,
) -> IndependentGroup {
    let mut group = IndependentGroup::new("policies");
    let Some(recorded_policies) = &reference.policies else {
        group.unavailable.push(
            "reference.policies is unavailable, so no derived policy population is comparable"
                .to_string(),
        );
        return group.finish();
    };
    let policy_key = |policy: &DerivedRegionPolicy| {
        format!(
            "{}@g{}",
            support::canonical_json(
                &serde_json::to_value(&policy.identity).expect("identity serializes")
            ),
            policy.global_layer_index
        )
    };
    let derived_by_key: BTreeMap<String, &DerivedRegionPolicy> = construction
        .policies
        .iter()
        .map(|policy| (policy_key(policy), policy))
        .collect();
    let recorded_by_key: BTreeMap<String, &DerivedRegionPolicy> = recorded_policies
        .iter()
        .map(|policy| (policy_key(policy), policy))
        .collect();
    for (key, recorded_policy) in &recorded_by_key {
        match derived_by_key.get(key) {
            None => group.mismatch(format!("recorded policy {key} has no derived occurrence")),
            Some(derived_policy) => independent_compare_fill_policy(
                &mut group,
                key,
                &derived_policy.policy,
                &recorded_policy.policy,
            ),
        }
    }
    for key in derived_by_key.keys() {
        if !recorded_by_key.contains_key(key) {
            group.mismatch(format!("derived policy {key} has no recorded occurrence"));
        }
    }
    group.finish()
}

fn independent_compare_fill_policy(
    group: &mut IndependentGroup,
    key: &str,
    derived: &FillPolicyInput,
    recorded: &FillPolicyInput,
) {
    for (field, derived_value, recorded_value, tolerance) in [
        (
            "infill_direction_deg",
            derived.infill_direction_deg,
            recorded.infill_direction_deg,
            SCALAR_TOLERANCE_MM,
        ),
        (
            "solid_infill_direction_deg",
            derived.solid_infill_direction_deg,
            recorded.solid_infill_direction_deg,
            SCALAR_TOLERANCE_MM,
        ),
        (
            "top_layer_direction_deg",
            derived.top_layer_direction_deg,
            recorded.top_layer_direction_deg,
            SCALAR_TOLERANCE_MM,
        ),
        (
            "bottom_layer_direction_deg",
            derived.bottom_layer_direction_deg,
            recorded.bottom_layer_direction_deg,
            SCALAR_TOLERANCE_MM,
        ),
        (
            "model_alignment_offset_rad",
            derived.model_alignment_offset_rad,
            recorded.model_alignment_offset_rad,
            support::ANGLE_TOLERANCE_RAD,
        ),
        (
            "resolved_sparse_angle_rad",
            derived.resolved_sparse_angle_rad,
            recorded.resolved_sparse_angle_rad,
            support::ANGLE_TOLERANCE_RAD,
        ),
        (
            "resolved_solid_angle_rad",
            derived.resolved_solid_angle_rad,
            recorded.resolved_solid_angle_rad,
            support::ANGLE_TOLERANCE_RAD,
        ),
    ] {
        independent_compare_scalar(
            group,
            &format!("{key}.{field}"),
            derived_value,
            recorded_value,
            tolerance,
        );
    }
    independent_compare_angle_option(
        group,
        &format!("{key}.resolved_top_angle_rad"),
        derived.resolved_top_angle_rad,
        recorded.resolved_top_angle_rad,
    );
    independent_compare_angle_option(
        group,
        &format!("{key}.resolved_bottom_angle_rad"),
        derived.resolved_bottom_angle_rad,
        recorded.resolved_bottom_angle_rad,
    );
    independent_compare_angle_option(
        group,
        &format!("{key}.bridge_angle_rad"),
        derived.bridge_angle_rad,
        recorded.bridge_angle_rad,
    );
    for (field, derived_value, recorded_value) in [
        (
            "sparse_rotate_template",
            &derived.sparse_rotate_template,
            &recorded.sparse_rotate_template,
        ),
        (
            "solid_rotate_template",
            &derived.solid_rotate_template,
            &recorded.solid_rotate_template,
        ),
    ] {
        if derived_value != recorded_value {
            group.mismatch(format!(
                "{key}.{field}: recorded {recorded_value:?}, derived {derived_value:?}"
            ));
        }
    }
    for (field, derived_value, recorded_value) in [
        ("align_to_model", derived.align_to_model, recorded.align_to_model),
        ("fixed_angle", derived.fixed_angle, recorded.fixed_angle),
        ("is_bridge", derived.is_bridge, recorded.is_bridge),
    ] {
        if derived_value != recorded_value {
            group.mismatch(format!(
                "{key}.{field}: recorded {recorded_value}, derived {derived_value}"
            ));
        }
    }
}

fn independent_compare_scalar(
    group: &mut IndependentGroup,
    label: &str,
    derived: f64,
    recorded: f64,
    tolerance: f64,
) {
    if !(derived - recorded).abs().is_finite() || (derived - recorded).abs() > tolerance {
        group.mismatch(format!(
            "{label}: recorded {recorded}, derived {derived} (tol {tolerance})"
        ));
    }
}

fn independent_compare_angle_option(
    group: &mut IndependentGroup,
    label: &str,
    derived: Option<f64>,
    recorded: Option<f64>,
) {
    match (derived, recorded) {
        (None, None) => {}
        (Some(a), Some(b)) if (a - b).abs() <= support::ANGLE_TOLERANCE_RAD => {}
        _ => group.mismatch(format!(
            "{label}: recorded {recorded:?}, derived {derived:?} (tol {})",
            support::ANGLE_TOLERANCE_RAD
        )),
    }
}

fn independent_point(point: Point2) -> Point2 {
    Point2 {
        x: support::canonical_to_pnp(point.x),
        y: support::canonical_to_pnp(point.y),
    }
}

fn independent_points_close(a: Point2, b: Point2) -> bool {
    (a.x - b.x).abs() <= support::GEOMETRY_TOLERANCE_PNP_UNITS
        && (a.y - b.y).abs() <= support::GEOMETRY_TOLERANCE_PNP_UNITS
}

fn independent_ring(points: &[Point2]) -> slicer_ir::Polygon {
    slicer_ir::Polygon {
        points: points.iter().copied().map(independent_point).collect(),
    }
}

/// Rotation-tolerant ring equality within the recorded transport tolerance. The
/// ring is compared in its recorded direction; only the starting vertex may
/// shift, exactly as the transport rule allows.
fn independent_rings_equal(a: &[Point2], b: &[Point2]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    if a.is_empty() {
        return true;
    }
    let count = a.len();
    (0..count).any(|shift| {
        (0..count).all(|index| {
            let other = b[(index + shift) % count];
            independent_points_close(a[index], other)
        })
    })
}

/// Polygon equality with holes preserved: the contour must match and the hole
/// multiset must match, so a hole dropped or added is a mismatch.
fn independent_polygons_equal(derived: &ExPolygon, recorded: &ExPolygon) -> bool {
    if !independent_rings_equal(&derived.contour.points, &recorded.contour.points) {
        return false;
    }
    if derived.holes.len() != recorded.holes.len() {
        return false;
    }
    let mut used = vec![false; recorded.holes.len()];
    for hole in &derived.holes {
        let Some(position) = recorded
            .holes
            .iter()
            .enumerate()
            .position(|(index, candidate)| {
                !used[index] && independent_rings_equal(&hole.points, &candidate.points)
            })
        else {
            return false;
        };
        used[position] = true;
    }
    true
}

fn independent_area(polygon: &ExPolygon) -> f64 {
    fn ring_area(points: &[Point2]) -> f64 {
        if points.len() < 3 {
            return 0.0;
        }
        let mut sum = 0.0f64;
        for index in 0..points.len() {
            let a = points[index];
            let b = points[(index + 1) % points.len()];
            sum += (a.x as f64) * (b.y as f64) - (b.x as f64) * (a.y as f64);
        }
        sum / 2.0
    }
    ring_area(&polygon.contour.points)
        - polygon
            .holes
            .iter()
            .map(|hole| ring_area(&hole.points).abs())
            .sum::<f64>()
}

/// Polygon multiset comparison in PnP units: the recorded side is converted with
/// the documented transport rule, then matched largest-first so duplicate
/// polygons cannot be matched more than once.
fn independent_compare_polygon_multiset(
    group: &mut IndependentGroup,
    label: &str,
    derived: &[ExPolygon],
    recorded: &[ExPolygon],
) {
    let recorded_geometry: Vec<ExPolygon> = recorded
        .iter()
        .map(|polygon| ExPolygon {
            contour: independent_ring(&polygon.contour.points),
            holes: polygon
                .holes
                .iter()
                .map(|hole| independent_ring(&hole.points))
                .collect(),
        })
        .collect();
    let mut order: Vec<usize> = (0..recorded_geometry.len()).collect();
    order.sort_by(|a, b| {
        independent_area(&recorded_geometry[*b])
            .partial_cmp(&independent_area(&recorded_geometry[*a]))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut used = vec![false; derived.len()];
    let mut matched = 0usize;
    let mut unmatched_recorded = Vec::new();
    for recorded_index in order {
        let found = derived.iter().enumerate().find(|(index, candidate)| {
            !used[*index] && independent_polygons_equal(candidate, &recorded_geometry[recorded_index])
        });
        match found {
            Some((index, _)) => {
                used[index] = true;
                matched += 1;
            }
            None => unmatched_recorded.push(recorded_index),
        }
    }
    let unmatched_derived: Vec<usize> = used
        .iter()
        .enumerate()
        .filter_map(|(index, was_used)| (!was_used).then_some(index))
        .collect();
    if !unmatched_recorded.is_empty() || !unmatched_derived.is_empty() {
        group.mismatch(format!(
            "{label}: {} derived / {} recorded polygons, {} matched; unmatched recorded {:?}, unmatched derived {:?}",
            derived.len(),
            recorded_geometry.len(),
            matched,
            unmatched_recorded,
            unmatched_derived
        ));
    }
}


fn independent_parameter_mismatches(
    construction: &LightningParameters,
    reference: &LightningParameters,
) -> Vec<String> {
    let mut mismatches = Vec::new();
    push_if_diff(
        &mut mismatches,
        "nozzle_diameter_mm",
        construction.nozzle_diameter_mm,
        reference.nozzle_diameter_mm,
        SCALAR_TOLERANCE_MM,
    );
    push_if_diff(
        &mut mismatches,
        "extrusion_width_mm",
        construction.extrusion_width_mm,
        reference.extrusion_width_mm,
        SCALAR_TOLERANCE_MM,
    );
    push_if_diff(
        &mut mismatches,
        "density_fraction",
        construction.density_fraction,
        reference.density_fraction,
        SCALAR_TOLERANCE_MM,
    );
    if construction.multiline != reference.multiline {
        mismatches.push(format!(
            "multiline: derived {}, recorded {}",
            construction.multiline, reference.multiline
        ));
    }
    push_if_diff(
        &mut mismatches,
        "layer_height_mm",
        construction.layer_height_mm,
        reference.layer_height_mm,
        SCALAR_TOLERANCE_MM,
    );
    push_if_diff(
        &mut mismatches,
        "overhang_angle_deg",
        construction.overhang_angle_deg,
        reference.overhang_angle_deg,
        SCALAR_TOLERANCE_MM,
    );
    push_if_diff(
        &mut mismatches,
        "prune_angle_deg",
        construction.prune_angle_deg,
        reference.prune_angle_deg,
        SCALAR_TOLERANCE_MM,
    );
    push_if_diff(
        &mut mismatches,
        "straightening_angle_deg",
        construction.straightening_angle_deg,
        reference.straightening_angle_deg,
        SCALAR_TOLERANCE_MM,
    );
    match (construction.supporting_radius_mm, reference.supporting_radius_mm) {
        (Some(a), Some(b)) if (a - b).abs() <= SCALAR_TOLERANCE_MM => {}
        (None, None) => {}
        _ => mismatches.push(format!(
            "supporting_radius_mm: derived {:?}, recorded {:?}",
            construction.supporting_radius_mm, reference.supporting_radius_mm
        )),
    }
    push_if_diff(
        &mut mismatches,
        "wall_supporting_radius_mm",
        construction.wall_supporting_radius_mm,
        reference.wall_supporting_radius_mm,
        SCALAR_TOLERANCE_MM,
    );
    // Canonical integer coord_t (1 nm) vs the derived exact real value.
    push_if_diff(
        &mut mismatches,
        "prune_length_mm",
        construction.prune_length_mm,
        reference.prune_length_mm,
        COORD_T_TOLERANCE_MM,
    );
    push_if_diff(
        &mut mismatches,
        "straightening_max_distance_mm",
        construction.straightening_max_distance_mm,
        reference.straightening_max_distance_mm,
        COORD_T_TOLERANCE_MM,
    );
    mismatches
}

fn push_if_diff(
    mismatches: &mut Vec<String>,
    field: &str,
    derived: f64,
    recorded: f64,
    tolerance: f64,
) {
    if (derived - recorded).abs() > tolerance {
        mismatches.push(format!(
            "{field}: derived {derived}, recorded {recorded} (tol {tolerance})"
        ));
    }
}

fn recorded_layer_map(reference: &support::GateReference) -> BTreeMap<u32, &PlanningLayer> {
    reference
        .planning_layers
        .iter()
        .map(|layer| (layer.global_layer_index, layer))
        .collect()
}

fn constructed_layer_map(construction: &GateAConstruction) -> BTreeMap<u32, &PlanningLayer> {
    construction
        .planning_layers
        .iter()
        .map(|layer| (layer.global_layer_index, layer))
        .collect()
}
