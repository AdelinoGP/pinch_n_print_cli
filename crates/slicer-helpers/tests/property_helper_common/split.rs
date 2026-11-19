//! Charter `H-HELPERS-SPLIT`: an independent oriented face partition for
//! `slicer_helpers::split_connected_components`.
//!
//! The production seam splits an `IndexedTriangleSet` into connected components
//! whose adjacency is **index topology only**, and only through a shared
//! *reversed* directed edge: face A carries `(a->b)` and face B carries `(b->a)`.
//! Vertex-only contact does not join, and a shared edge traversed in the same
//! direction does not join either.
//!
//! The reference model here is deliberately not the production algorithm: it
//! tests every unordered face pair with two plain nested loops over the directed
//! edges, records an adjacency matrix, and groups faces with a union-find. It
//! never builds the production `(from, to) -> faces` map and never runs the
//! production stack DFS.
//!
//! Faces are compared as **oriented** triangles of vertex *positions*: each face
//! becomes its three position bit patterns reduced to the lexicographically
//! smallest cyclic rotation (rotation keeps winding; reversal is never applied).
//! Because every constructed scene has globally distinct positions, position
//! identity equals vertex identity, so each output face resolves through its own
//! component's local vertex table back to exactly one input face. That is why
//! this charter asserts a face partition and never a vertex layout: per-component
//! vertex compaction and unreferenced-vertex dropping are production
//! implementation choices, not claimed contract.
//!
//! Deterministic witnesses run every [`SceneKind`] at two fixed parameter sets
//! before the generative run, so no scene class depends on RNG frequency.

use std::collections::BTreeMap;

use proptest::test_runner::{TestCaseError, TestCaseResult};
use slicer_helpers::split_connected_components;
use slicer_ir::IndexedTriangleSet;
use slicer_sdk::test_support::property_testing::PropertyRun;

use super::bounds::{checked_limits, HelperLimits};
use super::mesh::{
    canonical_face, directed_edges, face_count, face_ids, face_multiset, face_positions,
    make_scene, split_scene_strategy, validate_scene, Position, SceneKind, SceneParams, SplitScene,
    EVERY_KIND,
};

/// Registry identity of this charter.
pub const PROPERTY_ID: &str = "H-HELPERS-SPLIT";
/// Exact registry test name.
pub const TEST_NAME: &str = "proptest_helpers_split_components";

/// Whether two faces share one directed edge in opposite directions.
fn faces_join(mesh: &IndexedTriangleSet, first: usize, second: usize) -> bool {
    for &(a0, a1) in &directed_edges(mesh, first) {
        for &(b0, b1) in &directed_edges(mesh, second) {
            if a0 == b1 && a1 == b0 {
                return true;
            }
        }
    }
    false
}

/// Pairwise face reachability: one boolean per unordered face pair.
#[must_use]
pub fn adjacency_matrix(mesh: &IndexedTriangleSet) -> Vec<Vec<bool>> {
    let faces = face_count(mesh);
    let mut adjacent = vec![vec![false; faces]; faces];
    for first in 0..faces {
        for second in (first + 1)..faces {
            if faces_join(mesh, first, second) {
                adjacent[first][second] = true;
                adjacent[second][first] = true;
            }
        }
    }
    adjacent
}

fn find(parent: &mut [usize], node: usize) -> usize {
    let mut cursor = node;
    while parent[cursor] != cursor {
        parent[cursor] = parent[parent[cursor]];
        cursor = parent[cursor];
    }
    cursor
}

/// Group face ids into connected components, ordered by ascending first face.
#[must_use]
pub fn component_groups(adjacency: &[Vec<bool>]) -> Vec<Vec<usize>> {
    let faces = adjacency.len();
    let mut parent: Vec<usize> = (0..faces).collect();
    for first in 0..faces {
        for second in (first + 1)..faces {
            if adjacency[first][second] {
                let left = find(&mut parent, first);
                let right = find(&mut parent, second);
                if left != right {
                    parent[right] = left;
                }
            }
        }
    }
    let mut grouped: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for face in 0..faces {
        let root = find(&mut parent, face);
        grouped.entry(root).or_default().push(face);
    }
    let mut groups: Vec<Vec<usize>> = grouped.into_values().collect();
    // Union by first encounter leaves arbitrary roots; production seeds by
    // ascending face index, so order the groups by their smallest face.
    groups.sort_by_key(|group| group[0]);
    groups
}

/// Expected oriented-face multiset for each component, in production's seed order.
pub fn expected_face_groups(mesh: &IndexedTriangleSet) -> Result<Vec<Vec<[Position; 3]>>, String> {
    let groups = component_groups(&adjacency_matrix(mesh));
    groups
        .iter()
        .map(|group| face_multiset(mesh, group))
        .collect()
}

/// Describe the first face whose multiplicity differs between the two multisets.
fn multiset_difference(observed: &[[Position; 3]], expected: &[[Position; 3]]) -> String {
    let mut balance: BTreeMap<[Position; 3], i64> = BTreeMap::new();
    for face in observed {
        *balance.entry(*face).or_default() += 1;
    }
    for face in expected {
        *balance.entry(*face).or_default() -= 1;
    }
    for (face, count) in balance {
        if count != 0 {
            return format!("face {face:?} has multiplicity offset {count:+}");
        }
    }
    "the multisets hold the same faces in a different order".to_string()
}

/// Compare the real SUT output against the independent pairwise-reachability model.
///
/// Rejects, in this order: an incomplete or out-of-range output index; a scene
/// whose own contact witness the model disagrees with; a wrong component count; a
/// face that is missing, duplicated or foreign; a component order that is not
/// strictly ascending by first input face; and a component whose face multiset is
/// not the expected group.
///
/// Singleton survival is enforced by the count and partition checks rather than a
/// separate predicate: dropping the one-face component changes the count, and
/// moving its face elsewhere changes that component's multiset.
pub fn check_split(
    mesh: &IndexedTriangleSet,
    outputs: &[IndexedTriangleSet],
    kind: SceneKind,
) -> Result<(), String> {
    for (index, component) in outputs.iter().enumerate() {
        if !component.indices.len().is_multiple_of(3) {
            return Err(format!(
                "component {index} carries {} indices, which is not 3 * its face count: a truncated triangle reached the output",
                component.indices.len()
            ));
        }
        if let Some(&vertex) = component
            .indices
            .iter()
            .find(|&&vertex| vertex as usize >= component.vertices.len())
        {
            return Err(format!(
                "component {index} index {vertex} is outside its {} vertices",
                component.vertices.len()
            ));
        }
    }

    let expected = expected_face_groups(mesh)?;
    let plan_components = kind.expected_components();
    if expected.len() != plan_components {
        return Err(format!(
            "the independent model found {} components for {kind:?} but the plan requires {plan_components}: the generated scene lost its contact witness",
            expected.len()
        ));
    }
    if outputs.len() != expected.len() {
        return Err(format!(
            "split_connected_components returned {} components for {kind:?}, the model requires {}",
            outputs.len(),
            expected.len()
        ));
    }

    let mut observed_all: Vec<[Position; 3]> = Vec::new();
    for component in outputs {
        observed_all.extend(face_multiset(component, &face_ids(component))?);
    }
    observed_all.sort();
    let expected_all = face_multiset(mesh, &face_ids(mesh))?;
    if observed_all != expected_all {
        return Err(format!(
            "the outputs do not reproduce every input face exactly once ({kind:?}): {}",
            multiset_difference(&observed_all, &expected_all)
        ));
    }

    let mut position_of_face: Vec<[Position; 3]> = Vec::with_capacity(face_count(mesh));
    for face in 0..face_count(mesh) {
        position_of_face.push(canonical_face(face_positions(mesh, face)?));
    }
    let mut minima = Vec::with_capacity(outputs.len());
    for (index, component) in outputs.iter().enumerate() {
        let mut minimum = usize::MAX;
        for face in 0..face_count(component) {
            let signature = canonical_face(face_positions(component, face)?);
            let source = position_of_face
                .iter()
                .position(|candidate| *candidate == signature)
                .ok_or_else(|| {
                    format!("component {index} face {face} is not an input face of this scene")
                })?;
            minimum = minimum.min(source);
        }
        minima.push(minimum);
    }
    for pair in minima.windows(2) {
        if pair[0] >= pair[1] {
            return Err(format!(
                "component first-face indices {minima:?} are not strictly ascending ({kind:?})"
            ));
        }
    }

    for (index, group) in expected.iter().enumerate() {
        let observed = face_multiset(&outputs[index], &face_ids(&outputs[index]))?;
        if &observed != group {
            return Err(format!(
                "component {index} face partition differs ({kind:?}): {}",
                multiset_difference(&observed, group)
            ));
        }
    }
    Ok(())
}

/// Check one generated scene end to end: preconditions, real SUT, partition oracle.
pub fn check_scene(scene: &SplitScene, limits: &HelperLimits) -> TestCaseResult {
    if let Err(error) = validate_scene(scene, limits) {
        return Err(TestCaseError::fail(format!(
            "the generated scene violates its own preconditions ({:?} {:?}): {error}",
            scene.kind, scene.params
        )));
    }
    let outputs = split_connected_components(&scene.mesh);
    if let Err(error) = check_split(&scene.mesh, &outputs, scene.kind) {
        return Err(TestCaseError::fail(error));
    }
    Ok(())
}

/// Run every scene kind at two fixed parameter sets, independent of the RNG.
///
/// The witness pass covers the widest and simplest parameter combinations, so
/// the bound floors are proved to admit each named witness rather than assumed.
pub fn deterministic_witnesses(limits: HelperLimits) {
    for params in [SceneParams::WITNESS, SceneParams::MINIMAL] {
        for kind in EVERY_KIND {
            let scene = make_scene(kind, params);
            validate_scene(&scene, &limits).unwrap_or_else(|error| {
                panic!("H-HELPERS-SPLIT witness {kind:?} {params:?} violates its preconditions: {error}")
            });
            let outputs = split_connected_components(&scene.mesh);
            check_split(&scene.mesh, &outputs, kind).unwrap_or_else(|error| {
                panic!("H-HELPERS-SPLIT witness {kind:?} {params:?} failed the partition oracle: {error}")
            });
            assert_eq!(
                outputs.len(),
                kind.expected_components(),
                "H-HELPERS-SPLIT witness {kind:?} {params:?} must produce {} components",
                kind.expected_components()
            );
        }
    }
}

/// Run the registry-selected `H-HELPERS-SPLIT` property.
pub fn run_property() {
    let run = PropertyRun::new(PROPERTY_ID, TEST_NAME).expect("H-HELPERS-SPLIT registry context");
    let limits = checked_limits(&run).expect("H-HELPERS-SPLIT checked profile bounds");
    deterministic_witnesses(limits);
    let strategy = split_scene_strategy(&limits);
    run.check(&strategy, |scene, _guard| check_scene(&scene, &limits))
        .expect("H-HELPERS-SPLIT oriented face partition");
}
