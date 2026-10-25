//! Four executable acceptance checks for TASK-576 Gate B.
#[path = "lightning_portability_support.rs"]
mod support;

use support::*;

#[test]
fn planned_graph_native_and_wasm() {
    let runs = run_all();
    assert_eq!(runs.len(), 5);
    for run in &runs {
        let planning_layers: std::collections::BTreeSet<_> = run
            .case
            .input
            .planning_layers
            .iter()
            .map(|l| u64::from(l.global_layer_index))
            .collect();
        let planning_max = *planning_layers.last().expect("recorded planning layers");
        let canonical_layers = run.case.reference["replay"]["production"]["graph"]
            .as_array()
            .expect("canonical replay layers");
        let mut node_layers = Vec::new();
        for layer in canonical_layers {
            let layer_index = layer["layer_index"].as_u64().unwrap();
            for node in layer["nodes"].as_array().unwrap() {
                assert!(
                    planning_layers.contains(&layer_index),
                    "{} canonical node {} layer_index {layer_index} has no recorded planning layer",
                    run.case.id,
                    node["node_id"]
                );
                assert!(layer_index <= planning_max,
                    "{} canonical node {} layer_index {layer_index} exceeds planning horizon {planning_max}",
                    run.case.id, node["node_id"]);
                node_layers.push(layer_index);
            }
        }
        let node_max = node_layers.into_iter().max().expect("canonical nodes");
        let replay_max = canonical_layers
            .iter()
            .map(|l| l["layer_index"].as_u64().unwrap())
            .max()
            .unwrap();
        eprintln!("{} horizon: highest recorded planning layer={planning_max}, highest canonical node layer={node_max}, highest replay layer={replay_max}, replay layer count={}",
            run.case.id, canonical_layers.len());
        for arms in [&run.native, &run.wasm] {
            let total = arm_counts(arms);
            for key in ["nodes", "edges", "roots"] {
                assert!(total[key].as_u64().unwrap() > 0);
            }
            assert_eq!(arms.len(), run.case.input.planning_layers.len());
            for arm in arms {
                validate(&arm.layer, run.case.input.canonical_region_order.len()).unwrap();
                let input_layer = run
                    .case
                    .input
                    .planning_layers
                    .iter()
                    .find(|l| l.global_layer_index == arm.layer.global_layer_index)
                    .unwrap();
                assert_eq!(
                    arm.layer.provenance.source_catalog,
                    input_layer.source_catalog
                );
                assert_eq!(
                    arm.layer.provenance.boundary_provenance,
                    input_layer.boundary_provenance
                );
                assert_eq!(arm.layer.provenance.operations, input_layer.operations);
                if input_layer.internal_domains.is_empty() {
                    assert!(arm.layer.nodes.is_empty() && arm.layer.edges.is_empty());
                } else {
                    assert!(!arm.layer.nodes.is_empty());
                    assert!(!arm.layer.edges.is_empty());
                    assert!(arm.layer.nodes.iter().any(|n| n.parent.is_none()));
                }
                assert!(arm
                    .layer
                    .nodes
                    .iter()
                    .all(|n| n.id < 1 << 24 && n.point.iter().all(|v| v.is_finite())));
                if let Err(mismatch) = compare(&run.case, &arm.layer) {
                    eprintln!("{}: {mismatch}", run.case.id);
                }
            }
        }
        if run.case.id == "anchor_bridge_classic" {
            for arms in [&run.native, &run.wasm] {
                assert_eq!(
                    arms.iter()
                        .map(|a| a.layer.global_layer_index)
                        .collect::<Vec<_>>(),
                    vec![1, 2],
                    "anchor_bridge_classic records exactly both recorded layers"
                );
                let second = &arms[1].layer;
                assert!(
                    second.nodes.is_empty() && second.edges.is_empty(),
                    "anchor_bridge_classic second layer is empty despite two temporary anchors"
                );
                let input = &run.case.input.planning_layers[1];
                assert_eq!(input.temporary_anchors.len(), 2);
                assert_eq!(
                    second.provenance.source_catalog, input.source_catalog,
                    "anchor_bridge_classic second-layer provenance identical"
                );
                assert_eq!(
                    second.provenance.boundary_provenance,
                    input.boundary_provenance
                );
                assert_eq!(second.provenance.operations, input.operations);
            }
        }
        assert_eq!(run.decision["status"], "fail");
        assert!(run.decision["mismatches"].as_array().unwrap().iter().all(|m| m.as_str().unwrap().contains("missing/reversed/disconnected node (point,parent point,root point,is_root) entry 0")));
        let p = &run.case.input.parameters;
        let g = lightning_feasibility_kernel::kernel::generator_from_parameters(Vec::new(), p)
            .expect("generator");
        assert_eq!(
            (
                g.supporting_radius,
                g.wall_supporting_radius,
                g.prune_length,
                g.straightening_max_distance
            ),
            (30_000, 2_000, 175, 175)
        );
        let mut zero = p.clone();
        zero.density_fraction = 0.0;
        zero.supporting_radius_mm = None;
        assert_eq!(
            lightning_feasibility_kernel::kernel::generator_from_parameters(Vec::new(), &zero)
                .unwrap()
                .supporting_radius,
            0
        );
        zero.supporting_radius_mm = Some(3.0);
        assert!(
            lightning_feasibility_kernel::kernel::generator_from_parameters(Vec::new(), &zero)
                .is_err(),
            "Some/zero-density must be a named error"
        );
        let mut absent = p.clone();
        absent.supporting_radius_mm = None;
        assert!(
            lightning_feasibility_kernel::kernel::generator_from_parameters(Vec::new(), &absent)
                .is_err(),
            "None/nonzero-density must be a named error"
        );
    }
}

#[test]
fn gate_b_decision_is_truthful() {
    let runs = run_all();
    if !outcome_path().exists() {
        write_outcome(&runs, false);
    }
    let recorded = read_json(&outcome_path());
    for run in &runs {
        let case = recorded["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["case_id"] == run.case.id)
            .unwrap();
        assert_eq!(case["status"], run.decision["status"]);
        for arm in run.native.iter().chain(&run.wasm) {
            let recomputed = compare(&run.case, &arm.layer);
            if case["status"] == "pass" {
                assert!(recomputed.is_ok());
                assert!(case["mismatches"].as_array().unwrap().is_empty());
            }
        }
        assert_eq!(case["mismatches"], run.decision["mismatches"]);
    }
    assert_eq!(recorded["status"], aggregate(&runs));
}

#[test]
fn rejects_canned_echo_and_fallback() {
    let (case, graph) = synthetic_baseline(0);
    let (other, other_graph) = synthetic_baseline(100_000);
    for source in ["Canned", "HostFallback"] {
        assert!(drive_source(&case, source, 1)
            .err()
            .unwrap()
            .contains("missing producer"));
    }
    for producer in [Producer::Native, Producer::Wasm] {
        let payload = encode_plan(&graph, producer);
        assert_eq!(
            compare_arm(&case, &payload, 1),
            Ok(()),
            "valid nonempty SYNTHETIC baseline {producer:?}"
        );
        let layer = decode(&payload, 1).unwrap();
        assert!(!layer.nodes.is_empty() && !layer.edges.is_empty());
        let arm = DecodedArm { payload, layer };
        for (named_failure, payload) in corruption_records(&arm, &case) {
            let error = compare_arm(&case, &payload, 1).expect_err(named_failure);
            assert!(
                error.contains(named_failure),
                "{producer:?}: expected {named_failure}, got {error}"
            );
        }
        assert_eq!(
            compare_arm(&other, &arm.payload, 1).unwrap_err(),
            "case-identity/layer-provenance mismatch",
            "cached/canned case A against B"
        );
        let mut mixed = arm.payload.clone();
        mixed.records.retain(|r| r[0][0] != 3.0);
        mixed.records.extend(
            encode_plan(&other_graph, producer)
                .records
                .into_iter()
                .filter(|r| r[0][0] == 3.0),
        );
        assert_eq!(
            compare_arm(&case, &mixed, 1).unwrap_err(),
            "case-identity/layer-provenance mismatch",
            "case A nodes with case B provenance"
        );
    }
}

#[test]
fn outcome_records_gate_b() {
    let runs = run_all();
    let written = write_outcome(&runs, true);
    let outcome = read_json(&outcome_path());
    assert_eq!(outcome, written);
    assert_eq!(outcome["gate"], "B");
    assert_eq!(outcome["packet"], PACKET);
    assert_eq!(outcome["task_id"], "TASK-576");
    assert_eq!(outcome["status"], aggregate(&runs));
    assert_eq!(outcome["executed"], true);
    assert_eq!(outcome["build_guests_check_exit"], 0);
    for include in outcome["includes"].as_array().unwrap() {
        assert_eq!(
            include["sha256"],
            sha256_file(&root().join(include["path"].as_str().unwrap()))
        );
    }
    assert_eq!(outcome["includes"].as_array().unwrap().len(), 4);
    let guest = &outcome["guest_artifact"];
    assert_eq!(
        guest["sha256"],
        sha256_file(&root().join(guest["path"].as_str().unwrap()))
    );
    for ((record, run), count) in outcome["cases"]
        .as_array()
        .unwrap()
        .iter()
        .zip(&runs)
        .zip([13, 2, 2, 4, 6])
    {
        assert_eq!(record["planned"]["status"], run.decision["status"]);
        assert_eq!(record["native"], arm_counts(&run.native));
        assert_eq!(record["wasm"], arm_counts(&run.wasm));
        assert_eq!(
            record["non_comparable_records"]["final_fill_contributor_attribution"],
            contributor_record(&run.case)
        );
        assert_eq!(record["clipped"]["status"], "inconclusive");
        assert_eq!(
            record["clipped"]["reason"],
            run.case.absent["reference.final_fills"]
        );
        assert_eq!(record["clipped"]["fill_counts"], count);
        assert_eq!(
            record["clipped"]["absent_population"],
            "reference.final_fills"
        );
    }
    for population in NON_COMPARABLE {
        assert!(outcome["non_comparable_populations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["population"] == population));
    }
    assert_eq!(outcome["commands"].as_array().unwrap().len(), 5);
    assert_eq!(outcome["log_hashes"].as_array().unwrap().len(), 4);
    assert_eq!(outcome["migration_eligible"], false);
    use FrameworkAcceptance::{Unverified, Verified};
    use GateOutcome::{Fail, Inconclusive, Pass};
    for a in [Pass, Fail, Inconclusive] {
        for b in [Pass, Fail, Inconclusive] {
            for framework in [Verified, Unverified] {
                for executed in [true, false] {
                    assert_eq!(
                        migration_eligible(a, b, framework, executed),
                        a == Pass && b == Pass && framework == Verified && executed
                    );
                }
            }
        }
    }
    assert_eq!(outcome["framework_acceptance"], "Unverified");
    assert_eq!(outcome["gate_a_status"], "fail");
    assert_eq!(outcome["status"], "fail");
    assert!(!migration_eligible(
        GateOutcome::Fail,
        GateOutcome::Fail,
        FrameworkAcceptance::Unverified,
        true
    ));
    let blockers = outcome["blockers"].as_array().unwrap();
    assert!(blockers.iter().any(|b| b == "MIGRATION_BLOCKED_GATE_A"));
    assert!(blockers.iter().any(|b| b == "MIGRATION_BLOCKED_FRAMEWORK"));
    assert!(blockers.iter().any(|b| b == "MIGRATION_BLOCKED_GATE_B"));
    assert_eq!(blockers.len(), 3);
    let unexecuted = write_outcome(&runs, false);
    assert_eq!(unexecuted["executed"], false);
    assert_eq!(unexecuted["migration_eligible"], false);
    assert!(unexecuted["blockers"]
        .as_array()
        .unwrap()
        .iter()
        .any(|b| b == "MIGRATION_BLOCKED_UNEXECUTED"));
    // Restore the fully observed record after checking the unexecuted branch.
    assert_eq!(write_outcome(&runs, true)["executed"], true);
    assert_eq!(
        blockers.iter().any(|b| b == "MIGRATION_BLOCKED_GATE_B"),
        aggregate(&runs) != "pass"
    );
    let status_doc =
        std::fs::read_to_string(root().join("docs/07_implementation_status.md")).unwrap();
    let line = status_doc
        .lines()
        .find(|line| line.contains("TASK-576 Gate B outcome:"))
        .expect("Step 6 docs pending: TASK-576 Gate B outcome:");
    assert!(
        line.contains(aggregate(&runs)),
        "Gate B status doc disagrees"
    );
    let plan = std::fs::read_to_string(root().join("docs/specs/layer-module-preparation-plan.md"))
        .unwrap();
    let section = plan
        .split("### Gate B record")
        .nth(1)
        .expect("Step 6 docs pending: ### Gate B record");
    assert!(
        section
            .split("\n### ")
            .next()
            .unwrap()
            .contains(aggregate(&runs)),
        "Gate B record status disagrees"
    );
}
