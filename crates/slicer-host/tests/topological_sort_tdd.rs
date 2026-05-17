//! Red tests for TASK-023 topological sorting.

use slicer_host::{topological_sort, ModuleNode};

// TASK-023 contract notes:
// - Explicit cycle reporting avoids silently dropping work from cyclic graphs.
// - Deterministic queue behavior is locked down here rather than relying on
//   any abandoned upstream topological-sort heuristic.

#[test]
fn empty_input_returns_empty_order() {
    let nodes = Vec::new();

    let order = topological_sort(&nodes).expect("empty DAG should sort successfully");

    assert!(order.is_empty());
}

#[test]
fn linear_dag_preserves_dependency_order() {
    let nodes = vec![
        node("com.example.a", &["com.example.b"]),
        node("com.example.b", &["com.example.c"]),
        node("com.example.c", &[]),
    ];

    let order = topological_sort(&nodes).expect("linear DAG should sort successfully");

    assert_eq!(
        order,
        ids(&["com.example.a", "com.example.b", "com.example.c"])
    );
}

#[test]
fn zero_in_degree_roots_are_emitted_in_lexical_module_id_order() {
    let nodes = vec![
        node("com.example.zeta", &["com.example.shared"]),
        node("com.example.alpha", &["com.example.shared"]),
        node("com.example.shared", &[]),
    ];

    let order = topological_sort(&nodes).expect("acyclic DAG should sort successfully");

    assert_eq!(
        order,
        ids(&[
            "com.example.alpha",
            "com.example.zeta",
            "com.example.shared",
        ])
    );
}

#[test]
fn downstream_node_waits_until_all_predecessors_are_emitted() {
    let nodes = vec![
        node("com.example.alpha", &["com.example.join"]),
        node("com.example.beta", &["com.example.join"]),
        node("com.example.gamma", &["com.example.beta"]),
        node("com.example.join", &[]),
    ];

    let order = topological_sort(&nodes).expect("acyclic DAG should sort successfully");

    assert_eq!(
        order,
        ids(&[
            "com.example.alpha",
            "com.example.gamma",
            "com.example.beta",
            "com.example.join",
        ])
    );
}

#[test]
fn duplicate_or_presorted_edge_lists_do_not_change_output() {
    let nodes = vec![
        node(
            "com.example.root",
            &[
                "com.example.beta",
                "com.example.alpha",
                "com.example.alpha",
                "com.example.beta",
            ],
        ),
        node("com.example.alpha", &["com.example.leaf"]),
        node("com.example.beta", &["com.example.leaf"]),
        node("com.example.leaf", &[]),
    ];

    let order = topological_sort(&nodes).expect("acyclic DAG should sort successfully");

    assert_eq!(
        order,
        ids(&[
            "com.example.root",
            "com.example.alpha",
            "com.example.beta",
            "com.example.leaf",
        ])
    );
}

#[test]
fn cycle_returns_remaining_unsorted_module_ids_in_deterministic_order() {
    let nodes = vec![
        node("com.example.alpha", &["com.example.beta"]),
        node("com.example.beta", &["com.example.alpha"]),
        node("com.example.zeta", &[]),
    ];

    let cycle = topological_sort(&nodes).expect_err("cycle should be reported");

    assert_eq!(cycle, ids(&["com.example.alpha", "com.example.beta"]));
}

#[test]
fn disconnected_components_follow_kahn_queue_ordering_deterministically() {
    let nodes = vec![
        node("com.example.alpha", &["com.example.delta"]),
        node("com.example.beta", &["com.example.gamma"]),
        node("com.example.delta", &[]),
        node("com.example.gamma", &[]),
    ];

    let order = topological_sort(&nodes).expect("disconnected DAG should sort successfully");

    assert_eq!(
        order,
        ids(&[
            "com.example.alpha",
            "com.example.beta",
            "com.example.delta",
            "com.example.gamma",
        ])
    );
}

fn node(module_id: &str, edges_to: &[&str]) -> ModuleNode {
    use slicer_host::instrumentation::EdgeReason;
    use slicer_host::EdgeTo;
    ModuleNode {
        module_id: module_id.to_string(),
        ir_reads: Vec::new(),
        ir_writes: Vec::new(),
        edges_to: edges_to
            .iter()
            .map(|to| EdgeTo {
                to: (*to).to_string(),
                reasons: vec![EdgeReason::ExplicitRequires],
            })
            .collect(),
    }
}

#[allow(dead_code)]
fn ids(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}
