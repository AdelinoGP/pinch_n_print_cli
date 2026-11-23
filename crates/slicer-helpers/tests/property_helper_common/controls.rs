//! Deterministic controls for `H-HELPERS-SPLIT`.
//!
//! Both entry points run under a plain
//! `cargo test -p slicer-helpers --features property_testing --test property_helper_controls`.
//! They call the **same** [`check_split`] oracle and the **same**
//! [`SplitSceneValueTree`] the randomized property uses: one arm proves the real
//! `split_connected_components` output passes, the other proves a named mutation
//! of that output is rejected for the expected reason. No control here
//! substitutes for a registry execution, and no synthetic mutation claims a real
//! SUT failure.

use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::{Config, RngSeed, TestRunner};
use slicer_helpers::split_connected_components;
use slicer_ir::IndexedTriangleSet;

use super::bounds::HelperLimits;
use super::mesh::{
    face_count, make_scene, split_scene_strategy, validate_scene, SceneKind, SceneParams,
    SplitSceneValueTree, EVERY_KIND,
};
use super::split::check_split;

/// Fixed limits for the controls.
///
/// These are the packet's witness floors, not a measured profile: three
/// components (`ThreeDisjointSolids`), twelve faces and eight millimetres.
fn control_limits() -> HelperLimits {
    HelperLimits {
        max_components: 3,
        max_faces: 12,
        max_abs_mm: 8.0,
    }
}

/// Split every face into its own component, carrying the whole vertex table.
fn split_into_singletons(mesh: &IndexedTriangleSet) -> Vec<IndexedTriangleSet> {
    (0..face_count(mesh))
        .map(|face| IndexedTriangleSet {
            vertices: mesh.vertices.clone(),
            indices: mesh.indices[face * 3..face * 3 + 3].to_vec(),
        })
        .collect()
}

/// Join every component into one, offsetting each component's indices.
fn join_components(outputs: &[IndexedTriangleSet]) -> Vec<IndexedTriangleSet> {
    let mut combined = IndexedTriangleSet::default();
    for component in outputs {
        let offset = combined.vertices.len() as u32;
        combined.vertices.extend(component.vertices.iter().copied());
        combined
            .indices
            .extend(component.indices.iter().map(|&index| index + offset));
    }
    vec![combined]
}

/// Drop every one-face component.
fn drop_singletons(outputs: &[IndexedTriangleSet]) -> Vec<IndexedTriangleSet> {
    outputs
        .iter()
        .filter(|component| face_count(component) != 1)
        .cloned()
        .collect()
}

/// Reverse the emitted component order.
fn reverse_components(outputs: &[IndexedTriangleSet]) -> Vec<IndexedTriangleSet> {
    outputs.iter().rev().cloned().collect()
}

/// Duplicate the first face of the first component.
fn duplicate_first_face(outputs: &[IndexedTriangleSet]) -> Vec<IndexedTriangleSet> {
    let mut mutated = outputs.to_vec();
    let first = mutated
        .first_mut()
        .expect("the control scene has at least one component");
    let face = first.indices[0..3].to_vec();
    first.indices.extend(face);
    mutated
}

/// Remove the first face of the first component.
fn remove_first_face(outputs: &[IndexedTriangleSet]) -> Vec<IndexedTriangleSet> {
    let mut mutated = outputs.to_vec();
    let first = mutated
        .first_mut()
        .expect("the control scene has at least one component");
    first.indices.drain(0..3);
    mutated
}

/// Largest absolute vertex coordinate in the mesh, in millimetres.
fn max_abs_coordinate(mesh: &IndexedTriangleSet) -> f32 {
    mesh.vertices
        .iter()
        .map(|vertex| vertex.x.abs().max(vertex.y.abs()).max(vertex.z.abs()))
        .fold(0.0_f32, f32::max)
}

/// Largest vertex X coordinate in the mesh, in millimetres.
fn max_x(mesh: &IndexedTriangleSet) -> f32 {
    mesh.vertices
        .iter()
        .map(|vertex| vertex.x)
        .fold(f32::MIN, f32::max)
}

fn expect_pass(mesh: &IndexedTriangleSet, outputs: &[IndexedTriangleSet], kind: SceneKind) {
    if let Err(error) = check_split(mesh, outputs, kind) {
        panic!("the correct split_connected_components output for {kind:?} must pass the oracle: {error}");
    }
}

/// Reject a mutated result and prove *why* it was rejected.
fn expect_reject(
    mesh: &IndexedTriangleSet,
    outputs: &[IndexedTriangleSet],
    kind: SceneKind,
    mutation: &str,
    expected_fragment: &str,
) {
    match check_split(mesh, outputs, kind) {
        Ok(()) => panic!("the partition oracle accepted {mutation}"),
        Err(message) => assert!(
            message.contains(expected_fragment),
            "the oracle rejected {mutation} for the wrong reason: expected a message containing \
             {expected_fragment:?}, got {message:?}"
        ),
    }
}

/// Vertex-adjacency, same-direction-edge, dropped-singleton and face mutations.
pub fn edge_adjacency_and_singleton_mutations() {
    let limits = control_limits();

    // Every plan kind passes the oracle when the real SUT produced the output.
    for kind in EVERY_KIND {
        let scene = make_scene(kind, SceneParams::WITNESS);
        validate_scene(&scene, &limits).unwrap_or_else(|error| {
            panic!("control witness {kind:?} violates its preconditions: {error}")
        });
        let outputs = split_connected_components(&scene.mesh);
        expect_pass(&scene.mesh, &outputs, kind);
        assert_eq!(
            outputs.len(),
            kind.expected_components(),
            "control witness {kind:?} must produce {} components",
            kind.expected_components()
        );
    }

    // An opposite-direction shared edge must join into one two-face component.
    let opposite = make_scene(SceneKind::OppositeEdgePair, SceneParams::WITNESS);
    let joined = split_connected_components(&opposite.mesh);
    assert_eq!(joined.len(), 1, "a reversed shared edge must join");
    assert_eq!(
        face_count(&joined[0]),
        2,
        "the joined pair must keep both faces"
    );

    // A same-direction shared edge must not join: two one-face components.
    let same_direction = make_scene(SceneKind::SameDirectionEdgePair, SceneParams::WITNESS);
    let separated = split_connected_components(&same_direction.mesh);
    assert_eq!(
        separated.len(),
        2,
        "a same-direction shared edge must not join"
    );
    assert!(separated.iter().all(|component| face_count(component) == 1));

    // A vertex-only shared index must not join: two four-face components.
    let vertex_only = make_scene(SceneKind::VertexOnlyContact, SceneParams::WITNESS);
    let touching = split_connected_components(&vertex_only.mesh);
    assert_eq!(touching.len(), 2, "a vertex-only contact must not join");
    assert!(touching.iter().all(|component| face_count(component) == 4));

    // A lone one-face fragment survives beside a closed solid.
    let fragment = make_scene(SceneKind::SolidWithFragment, SceneParams::WITNESS);
    let fragments = split_connected_components(&fragment.mesh);
    assert_eq!(fragments.len(), 2, "the fragment must be its own component");
    assert_eq!(
        fragments
            .iter()
            .filter(|component| face_count(component) == 1)
            .count(),
        1,
        "exactly one component must be the one-face fragment"
    );
    let tiny = fragments
        .iter()
        .find(|component| face_count(component) == 1)
        .expect("the fragment component exists");
    assert_eq!(
        tiny.vertices.len(),
        3,
        "the fragment keeps its three distinct positions"
    );

    // (1) Vertex-adjacency mutant, splitting faces that must join.
    let tetrahedron = make_scene(SceneKind::Tetrahedron, SceneParams::WITNESS);
    let tetrahedron_out = split_connected_components(&tetrahedron.mesh);
    assert_eq!(tetrahedron_out.len(), 1);
    expect_reject(
        &tetrahedron.mesh,
        &split_into_singletons(&tetrahedron.mesh),
        SceneKind::Tetrahedron,
        "a tetrahedron split into four single-face components",
        "components",
    );
    expect_reject(
        &opposite.mesh,
        &split_into_singletons(&opposite.mesh),
        SceneKind::OppositeEdgePair,
        "a joined edge pair split into two single-face components",
        "components",
    );

    // (2) Vertex-adjacency mutant, joining faces that only share a vertex.
    expect_reject(
        &vertex_only.mesh,
        &join_components(&touching),
        SceneKind::VertexOnlyContact,
        "two vertex-only-touching solids joined into one component",
        "components",
    );

    // (3) Same-direction-edge mutant, joining a pair production must keep apart.
    expect_reject(
        &same_direction.mesh,
        &join_components(&separated),
        SceneKind::SameDirectionEdgePair,
        "a same-direction edge pair joined into one component",
        "components",
    );

    // (4) Dropped singleton component.
    expect_reject(
        &fragment.mesh,
        &drop_singletons(&fragments),
        SceneKind::SolidWithFragment,
        "the dropped one-face fragment component",
        "components",
    );

    // (5) Wrong component order.
    let two_solids = make_scene(SceneKind::TwoDisjointSolids, SceneParams::WITNESS);
    let two_solids_out = split_connected_components(&two_solids.mesh);
    assert_eq!(two_solids_out.len(), 2);
    assert_ne!(
        two_solids_out[0], two_solids_out[1],
        "the two solids must be distinguishable, or a swap would be invisible"
    );
    expect_reject(
        &two_solids.mesh,
        &reverse_components(&two_solids_out),
        SceneKind::TwoDisjointSolids,
        "the reversed component order",
        "ascending",
    );

    // (6) Extra and missing faces.
    expect_reject(
        &tetrahedron.mesh,
        &duplicate_first_face(&tetrahedron_out),
        SceneKind::Tetrahedron,
        "a duplicated face",
        "exactly once",
    );
    expect_reject(
        &tetrahedron.mesh,
        &remove_first_face(&tetrahedron_out),
        SceneKind::Tetrahedron,
        "a removed face",
        "exactly once",
    );
}

/// Seeded trees walked by the shrink control.
const SEEDED_TREES: usize = 16;

/// Drive the local value tree through `current`, `simplify` and `complicate`.
pub fn owned_shrink_witnesses() {
    let limits = control_limits();
    let kind = SceneKind::SolidWithFragment;
    let mut tree = SplitSceneValueTree::with_params(limits, kind, SceneParams::WITNESS);

    let initial = tree.current();
    assert_eq!(initial.params, SceneParams::WITNESS);
    validate_scene(&initial, &limits).unwrap_or_else(|error| {
        panic!("the nonminimal starting scene violates its preconditions: {error}")
    });
    let initial_outputs = split_connected_components(&initial.mesh);
    expect_pass(&initial.mesh, &initial_outputs, kind);
    assert_eq!(
        initial_outputs.len(),
        2,
        "the disconnected-pair witness must hold at the start"
    );
    assert_eq!(
        initial_outputs
            .iter()
            .filter(|component| face_count(component) == 1)
            .count(),
        1,
        "the singleton-fragment witness must hold at the start"
    );
    assert!(
        initial_outputs
            .iter()
            .map(|component| component.vertices.len())
            .all(|vertices| vertices > 0),
        "no exposed component may be empty"
    );

    // The known-failing input for this arm: the same output with its singleton
    // component dropped. Every shrink state must keep rejecting it, which is what
    // "the witness survived the shrink" means operationally.
    expect_reject(
        &initial.mesh,
        &drop_singletons(&initial_outputs),
        kind,
        "the dropped singleton at the nonminimal start",
        "components",
    );

    let initial_max_x = max_x(&initial.mesh);
    let initial_max_abs = max_abs_coordinate(&initial.mesh);
    let mut simplifications = 0_usize;
    let mut walk = vec![initial.params];
    while tree.simplify() {
        simplifications += 1;
        let current = tree.current();
        let previous = *walk.last().expect("the walk always has a head");
        assert_ne!(
            current.params, previous,
            "a successful simplify must move at least one parameter"
        );
        assert!(
            current.params.offset_eighths <= previous.offset_eighths
                && current.params.height_eighths <= previous.height_eighths,
            "shrinking may only reduce parameters: {:?} -> {:?}",
            previous,
            current.params
        );
        assert!(
            current.params.simplification_weight() < previous.simplification_weight(),
            "the documented simplification weight must strictly decrease: {:?} -> {:?}",
            previous,
            current.params
        );
        validate_scene(&current, &limits).unwrap_or_else(|error| {
            panic!(
                "a shrunk scene violates its preconditions at {:?}: {error}",
                current.params
            )
        });
        let outputs = split_connected_components(&current.mesh);
        expect_pass(&current.mesh, &outputs, kind);
        assert_eq!(
            outputs.len(),
            2,
            "the disconnected-pair witness must survive shrinking at {:?}",
            current.params
        );
        assert_eq!(
            outputs
                .iter()
                .filter(|component| face_count(component) == 1)
                .count(),
            1,
            "the singleton-fragment witness must survive shrinking at {:?}",
            current.params
        );
        expect_reject(
            &current.mesh,
            &drop_singletons(&outputs),
            kind,
            "the dropped singleton after shrinking",
            "components",
        );
        walk.push(current.params);
    }
    assert!(
        simplifications >= 2,
        "the nonminimal start must permit more than one shrink step, got {simplifications}"
    );
    assert!(
        !tree.simplify(),
        "a tree already at its minimum must report false rather than loop"
    );

    let minimal = tree.current();
    assert_eq!(
        minimal.params,
        SceneParams::MINIMAL,
        "the shrink walk must reach the minimal permitted parameters"
    );
    let minimal_max_x = max_x(&minimal.mesh);
    let minimal_max_abs = max_abs_coordinate(&minimal.mesh);
    assert!(
        minimal_max_abs < initial_max_abs,
        "the scene must be structurally smaller after shrinking: {initial_max_abs} mm -> {minimal_max_abs} mm"
    );
    // The offset is the only X-affecting parameter and it moved by exactly four
    // eighths, so the X extent must drop by exactly half a millimetre.
    assert_eq!(
        initial_max_x - minimal_max_x,
        0.5,
        "the offset reduction must shrink the X extent by exactly the offset difference"
    );

    // Repeated complicate must invert every simplify step exactly, and each
    // restored state must still satisfy the preconditions and both oracle arms.
    let mut restored = 0_usize;
    for expected in walk.iter().rev().skip(1) {
        assert!(tree.complicate(), "complicate must restore {expected:?}");
        restored += 1;
        let state = tree.current();
        assert_eq!(
            state.params, *expected,
            "complicate must expose the exact parameters the matching simplify replaced"
        );
        validate_scene(&state, &limits).unwrap_or_else(|error| {
            panic!("a complicate-restored scene violates its preconditions: {error}")
        });
        let outputs = split_connected_components(&state.mesh);
        expect_pass(&state.mesh, &outputs, kind);
        assert_eq!(
            outputs.len(),
            2,
            "the disconnected-pair witness must survive complicating"
        );
        expect_reject(
            &state.mesh,
            &drop_singletons(&outputs),
            kind,
            "the dropped singleton after complicating",
            "components",
        );
    }
    assert!(
        restored >= 2,
        "the walk must complicate repeatedly, restored {restored} states"
    );
    assert!(
        !tree.complicate(),
        "an exhausted history must report false rather than fabricate a state"
    );
    assert_eq!(
        tree.current().params,
        SceneParams::WITNESS,
        "complicate exhausts the history back to the start"
    );

    // The strategy really hands this tree type to Proptest: walk seeded trees
    // through the same preconditions, both oracle arms and the same simplify and
    // complicate contract.
    let strategy = split_scene_strategy(&limits);
    let mut runner = TestRunner::new(Config {
        cases: 1,
        rng_seed: RngSeed::Fixed(0x5eed_c0de),
        failure_persistence: None,
        ..Config::default()
    });
    for _ in 0..SEEDED_TREES {
        let mut seeded = strategy.new_tree(&mut runner).expect("a seeded split tree");
        let scene = seeded.current();
        validate_scene(&scene, &limits).expect("a strategy-constructed scene is valid");
        let outputs = split_connected_components(&scene.mesh);
        expect_pass(&scene.mesh, &outputs, scene.kind);
        let before = scene.params;
        if seeded.simplify() {
            let after = seeded.current();
            assert_eq!(after.kind, scene.kind, "shrinking must not change the plan");
            assert!(
                after.params.simplification_weight() < before.simplification_weight(),
                "a strategy tree must only shrink: {before:?} -> {:?}",
                after.params
            );
            validate_scene(&after, &limits).expect("a shrunk strategy scene is valid");
            let shrunk_outputs = split_connected_components(&after.mesh);
            expect_pass(&after.mesh, &shrunk_outputs, after.kind);
            assert!(seeded.complicate(), "a shrunk tree must complicate back");
            assert_eq!(seeded.current().params, before);
        }
    }
}
