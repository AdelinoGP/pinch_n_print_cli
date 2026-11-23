//! `geometry_controls_owned_shrink_witnesses`: audit the owned geometry trees.
//!
//! This drives the **actual** `GeometryValueTree` returned by
//! `domains::geometry_cases` — the same tree the `H-BOOL-SET` property consumes
//! through its generated operands — so the audit is about the tree the
//! properties really use, not a reconstruction. It also drives the two composed
//! trees the geometry charters consume: the paired-geometry input tree
//! (`boolean::strategy`, `H-BOOL-SET`) and the mesh scene tree
//! (`mesh_slice::scene_strategy`, `H-MESH-SLICE`).
//!
//! Four trees are audited, each over its own seeded walks:
//!
//! * the `Holed` catalog domain, where every exposed state keeps its hole
//!   witness;
//! * the `Disconnected` catalog domain with `min_components = 2`, where every
//!   exposed state keeps its disconnected-pair witness;
//! * the paired-geometry input tree, where every exposed state must satisfy the
//!   generator's own `boolean::realize` preconditions, keep both rectangle
//!   rings valid, and never grow the documented class/vertex key under
//!   `simplify`;
//! * the mesh scene tree, where every exposed state must build a closed mesh
//!   with strictly increasing planes and its whole-millimetre volume never
//!   grows under `simplify`.
//!
//! Operational definition of "the witness survived the shrink": a deliberately
//! wrong result that violates the domain's minimum hole and minimum component
//! witnesses must be rejected by `validate_geometry` at the nonminimal start
//! **and after every successful simplification**. A tree that quietly dropped
//! its hole witness while shrinking would make the mutant start passing, and
//! this control would fail. The composed trees are held to the same shape: a
//! deliberately wrong input (a destroyed class precondition) must be rejected
//! by `realize`, and a deliberately wrong mesh (one triangle re-wound) must be
//! rejected by `validate_indexed_mesh` at every exposed state.

use proptest::strategy::{Strategy, ValueTree};
use proptest::test_runner::{Config, RngSeed, TestRunner};
use slicer_ir::{ExPolygon, Point2, Polygon};
use slicer_sdk::test_support::property_testing::domains::{
    geometry_cases, geometry_complexity, BoundaryRule, GeometryBounds, GeometryCase,
    GeometryComplexity, GeometryDomain, TopologyClass,
};
use slicer_sdk::test_support::property_testing::oracles::{
    signed_area2, validate_geometry, validate_indexed_mesh, validate_ring,
};

use super::boolean;
use super::bounds::GeometryLimits;
use super::mesh_slice::{self, RawSliceBounds};

/// Absolute coordinate bound for the audited domain, in canonical units.
pub const MAX_ABS_UNITS: i64 = 40_000;
/// Total vertices across all rings.
pub const MAX_VERTICES: usize = 64;
/// Holes allowed in one case.
pub const MAX_HOLES: usize = 4;
/// Outer components allowed in one case.
pub const MAX_COMPONENTS: usize = 3;
/// Candidate budget of the value tree.
pub const MAX_SHRINK_CANDIDATES: u32 = 64;
/// Minimum holes the witness requires.
pub const MIN_HOLES: usize = 1;
/// Minimum components the witness requires.
pub const MIN_COMPONENTS: usize = 1;
/// Seeded trees to search before concluding no nonminimal tree exists.
pub const TREE_ATTEMPTS: u64 = 8;

/// Minimum components the disconnected-pair witness requires.
pub const MIN_DISCONNECTED_COMPONENTS: usize = 2;
/// Largest solid extent the mesh-scene audit allows, whole millimetres.
pub const SCENE_MAX_EXTENT_MM: u64 = 8;
/// Largest absolute origin the mesh-scene audit allows, whole millimetres.
pub const SCENE_MAX_TRANSLATION_MM: u64 = 4;
/// Plane budget of the mesh-scene audit.
pub const SCENE_MAX_PLANES: u64 = 5;
/// Seed base of the disconnected-pair audits.
pub const DISCONNECTED_SEED_BASE: u64 = 0xd15c_0000;
/// Seed base of the paired-geometry audits.
pub const BOOLEAN_SEED_BASE: u64 = 0xb001_0000;
/// Seed base of the mesh-scene audits.
pub const SCENE_SEED_BASE: u64 = 0x5ce7_0000;

/// Measured counts of one audited tree walk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuditFacts {
    /// Successful `simplify` calls.
    pub simplifications: usize,
    /// Successful `complicate` calls on the exhausted tree.
    pub complications: usize,
    /// Most successful `complicate` calls seen in an intermediate-depth replay.
    pub restored: usize,
    /// Documented reduction key at the first exposed state.
    pub start: (u128, usize),
    /// Documented reduction key at the final simplified state.
    pub end: (u128, usize),
}

/// The audited domain is pinned field by field, so a new domain field cannot
/// silently widen the witness this control audits, and a changed minimum cannot
/// be inherited from the helper default.
fn domain() -> GeometryDomain {
    // exhaustive: the audited domain is pinned field-by-field on purpose
    GeometryDomain {
        class: TopologyClass::Holed,
        min_holes: MIN_HOLES,
        min_components: MIN_COMPONENTS,
        boundary: BoundaryRule::Strict,
        // exhaustive: every audited bound is pinned explicitly
        bounds: GeometryBounds {
            max_abs_units: MAX_ABS_UNITS,
            max_vertices: MAX_VERTICES,
            max_holes: MAX_HOLES,
            max_components: MAX_COMPONENTS,
            max_shrink_candidates: MAX_SHRINK_CANDIDATES,
        },
    }
}

/// A mutant that violates the witness: every hole is dropped.
///
/// Dropping a single hole would not violate `min_holes = 1` for a case that
/// carries three, so the mutant must remove the whole hole population.
fn dropped_hole(case: &GeometryCase) -> GeometryCase {
    let mut mutant = case.clone();
    for polygon in &mut mutant.polygons {
        polygon.holes.clear();
    }
    mutant
}

/// Lexicographic reduction key: total absolute units, then vertex count.
fn reduction_key(complexity: GeometryComplexity) -> (u128, usize) {
    (complexity.3, complexity.2)
}

fn check_state(case: &GeometryCase, domain: &GeometryDomain) -> GeometryComplexity {
    validate_geometry(case, domain).unwrap_or_else(|error| {
        panic!("an exposed tree value violates its domain preconditions: {error}")
    });
    let complexity = geometry_complexity(case).expect("complexity");
    assert!(
        complexity.1 >= MIN_HOLES,
        "the hole witness must hold at every exposed state, got {complexity:?}"
    );
    assert!(
        complexity.0 >= MIN_COMPONENTS,
        "the component witness must hold at every exposed state, got {complexity:?}"
    );
    let mutant = dropped_hole(case);
    assert!(
        validate_geometry(&mutant, domain).is_err(),
        "the dropped-hole mutant must be rejected at every exposed state, but {complexity:?} accepted it"
    );
    complexity
}

/// Drive one seeded tree through current, simplify, a false simplify, and
/// repeated complicate.
fn audit_tree(seed: u64) -> Option<usize> {
    let domain = domain();
    let strategy = geometry_cases(domain.clone()).expect("H-BOOL-SET geometry domain");
    let mut runner = TestRunner::new(Config {
        cases: 1,
        rng_seed: RngSeed::Fixed(seed),
        failure_persistence: None,
        ..Config::default()
    });
    let mut tree = strategy
        .new_tree(&mut runner)
        .expect("seeded geometry tree");
    let start = tree.current().clone();
    let start_complexity = check_state(&start, &domain);

    let mut walk: Vec<GeometryComplexity> = vec![start_complexity];
    let mut simplifications = 0_usize;
    while tree.simplify() {
        simplifications += 1;
        let current = tree.current().clone();
        let previous = *walk.last().expect("walk head");
        let complexity = check_state(&current, &domain);
        assert!(
            reduction_key(complexity) < reduction_key(previous),
            "a successful simplify must strictly reduce the documented key: {previous:?} -> {complexity:?}"
        );
        walk.push(complexity);
    }
    if simplifications == 0 {
        return None;
    }
    assert!(
        !tree.simplify(),
        "a tree already at its minimum must report false rather than loop"
    );
    let final_complexity = check_state(&tree.current().clone(), &domain);
    assert!(
        reduction_key(final_complexity) < reduction_key(start_complexity),
        "the shrink walk must end strictly smaller than its start: {start_complexity:?} -> {final_complexity:?}"
    );

    let mut restored = 0_usize;
    for expected in walk.iter().rev().skip(1) {
        assert!(tree.complicate(), "complicate must restore {expected:?}");
        restored += 1;
        let state = tree.current().clone();
        let complexity = check_state(&state, &domain);
        assert_eq!(
            reduction_key(complexity),
            reduction_key(*expected),
            "complicate must restore the exact simplification key"
        );
    }
    assert!(
        restored >= 1,
        "the walk must complicate at least once, restored {restored}"
    );
    assert!(
        !tree.complicate(),
        "an exhausted history must report false rather than fabricate a state"
    );
    assert_eq!(
        tree.current().clone(),
        start,
        "complicate exhausts the history back to the first state"
    );
    Some(simplifications)
}

// ─────────────────────────────────────────────────────────────────────────────
// Disconnected-pair audit.
// ─────────────────────────────────────────────────────────────────────────────

/// The audited `Disconnected` domain, pinned field by field like [`domain`].
fn disconnected_domain() -> GeometryDomain {
    // exhaustive: the audited domain is pinned field-by-field on purpose
    GeometryDomain {
        class: TopologyClass::Disconnected,
        min_holes: 0,
        min_components: MIN_DISCONNECTED_COMPONENTS,
        boundary: BoundaryRule::Strict,
        // exhaustive: every audited bound is pinned explicitly
        bounds: GeometryBounds {
            max_abs_units: MAX_ABS_UNITS,
            max_vertices: MAX_VERTICES,
            max_holes: MAX_HOLES,
            max_components: MAX_COMPONENTS,
            max_shrink_candidates: MAX_SHRINK_CANDIDATES,
        },
    }
}

/// A mutant that violates the disconnected-pair witness: the component
/// population is trimmed below `min_components`.
///
/// Removing a single component from a three-component case would still satisfy
/// `min_components = 2`, so — exactly as [`dropped_hole`] clears every hole
/// rather than one — this trims the population to fewer than the minimum.
fn dropped_component(case: &GeometryCase) -> GeometryCase {
    let mut mutant = case.clone();
    mutant.polygons.truncate(MIN_DISCONNECTED_COMPONENTS - 1);
    mutant
}

/// Check one exposed disconnected state against its preconditions, its
/// disconnected-pair witness, and the dropped-component mutant.
fn check_disconnected_state(case: &GeometryCase, domain: &GeometryDomain) -> GeometryComplexity {
    validate_geometry(case, domain).unwrap_or_else(|error| {
        panic!("an exposed disconnected tree value violates its domain preconditions: {error}")
    });
    let complexity = geometry_complexity(case).expect("complexity");
    assert!(
        complexity.0 >= MIN_DISCONNECTED_COMPONENTS,
        "the disconnected-pair witness must hold at every exposed state, got {complexity:?}"
    );
    let mutant = dropped_component(case);
    match validate_geometry(&mutant, domain) {
        Ok(()) => panic!(
            "the dropped-component mutant must be rejected at every exposed state, but {complexity:?} accepted it"
        ),
        Err(error) => assert!(
            error.to_string().contains("component count"),
            "the dropped-component mutant must fail the component-count precondition, got: {error}"
        ),
    }
    complexity
}

/// Drive one seeded `Disconnected` tree through current, simplify, a false
/// simplify, and repeated complicate.
fn audit_disconnected_tree(seed: u64) -> Option<AuditFacts> {
    let domain = disconnected_domain();
    let strategy = geometry_cases(domain.clone()).expect("H-BOOL-SET disconnected domain");
    let mut runner = TestRunner::new(Config {
        cases: 1,
        rng_seed: RngSeed::Fixed(seed),
        failure_persistence: None,
        ..Config::default()
    });
    let mut tree = strategy
        .new_tree(&mut runner)
        .expect("seeded disconnected tree");
    let start = tree.current().clone();
    let start_complexity = check_disconnected_state(&start, &domain);

    let mut walk: Vec<GeometryComplexity> = vec![start_complexity];
    let mut simplifications = 0_usize;
    while tree.simplify() {
        simplifications += 1;
        let current = tree.current().clone();
        let previous = *walk.last().expect("walk head");
        let complexity = check_disconnected_state(&current, &domain);
        assert!(
            reduction_key(complexity) < reduction_key(previous),
            "a successful simplify must strictly reduce the documented key: {previous:?} -> {complexity:?}"
        );
        walk.push(complexity);
    }
    if simplifications == 0 {
        return None;
    }
    assert!(
        !tree.simplify(),
        "a tree already at its minimum must report false rather than loop"
    );
    let final_complexity = check_disconnected_state(&tree.current().clone(), &domain);
    assert!(
        reduction_key(final_complexity) < reduction_key(start_complexity),
        "the shrink walk must end strictly smaller than its start: {start_complexity:?} -> {final_complexity:?}"
    );

    let mut complications = 0_usize;
    for expected in walk.iter().rev().skip(1) {
        assert!(tree.complicate(), "complicate must restore {expected:?}");
        complications += 1;
        let state = tree.current().clone();
        let complexity = check_disconnected_state(&state, &domain);
        assert_eq!(
            reduction_key(complexity),
            reduction_key(*expected),
            "complicate must restore the exact simplification key"
        );
    }
    assert!(
        complications >= 1,
        "the walk must complicate at least once, restored {complications}"
    );
    assert!(
        !tree.complicate(),
        "an exhausted history must report false rather than fabricate a state"
    );
    assert_eq!(
        tree.current().clone(),
        start,
        "complicate exhausts the history back to the first state"
    );
    Some(AuditFacts {
        simplifications,
        complications,
        restored: complications,
        start: reduction_key(start_complexity),
        end: reduction_key(final_complexity),
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Paired-geometry audit (`H-BOOL-SET` input tree).
// ─────────────────────────────────────────────────────────────────────────────

/// The reference `H-BOOL-SET` profile the control drives.
///
/// This is the same reference profile `bounds::controls_run` validates, so the
/// control and the profile controls cannot drift apart.
fn boolean_limits() -> GeometryLimits {
    boolean::validate_limits(boolean::reference_raw_bounds()).expect("H-BOOL-SET reference profile")
}

/// Total vertices across every contour and hole of one operand.
fn count_vertices(polygons: &[ExPolygon]) -> usize {
    polygons
        .iter()
        .map(|polygon| {
            std::iter::once(&polygon.contour)
                .chain(polygon.holes.iter())
                .map(|ring| ring.points.len())
                .sum::<usize>()
        })
        .sum()
}

/// Rank of a class in the paired tree's `Union` order.
///
/// Proptest's `Union` shrinks a picked option to exhaustion and then moves to
/// an **earlier** option, so this rank is non-increasing under `simplify`.
fn class_rank(class: TopologyClass) -> usize {
    boolean::CLASSES
        .iter()
        .position(|candidate| *candidate == class)
        .unwrap_or_else(|| panic!("{class:?} is not one of the paired tree's classes"))
}

/// The paired-tree reduction key: the two operands' summed `Union` rank, then
/// the total number of realised vertices.
///
/// Why this is non-increasing under a successful `simplify`:
///
/// * a shrink **inside** a picked class only removes vertices (the #2
///   `GeometryValueTree` never adds one), so the vertex total falls while the
///   ranks stay put;
/// * a shrink that exhausts a class moves to an **earlier** `Union` option, so
///   the rank sum falls strictly even though the freshly generated
///   earlier-class value may carry a different vertex count;
/// * the rectangle, relative-placement, copy and base fields change neither a
///   class nor a vertex count, so the key is unchanged.
///
/// The rank component therefore always covers the case the vertex count cannot:
/// a class switch, which is a genuine permitted reduction but not a
/// vertex-count reduction.
fn boolean_key(input: &boolean::BoolInput, realized: &boolean::Realized) -> (u128, usize) {
    let ranks = class_rank(input.a.class) + class_rank(input.b.class);
    let vertices = count_vertices(&realized.a) + count_vertices(&realized.b);
    (
        u128::try_from(ranks).expect("bounded class rank sum"),
        vertices,
    )
}

/// A valid counterclockwise unit square, used only to break an `Empty` case.
fn unit_square() -> Polygon {
    Polygon {
        points: vec![
            Point2 { x: 0, y: 0 },
            Point2 { x: 1, y: 0 },
            Point2 { x: 1, y: 1 },
            Point2 { x: 0, y: 1 },
        ],
    }
}

/// A deliberately wrong paired input plus the diagnostic fragment that proves
/// `realize` rejected it for the destroyed class precondition.
fn broken_input(input: &boolean::BoolInput) -> (boolean::BoolInput, &'static str) {
    let mut broken = input.clone();
    let fragment = match input.a.class {
        TopologyClass::Empty => {
            broken.a.case.polygons.push(ExPolygon {
                contour: unit_square(),
                holes: Vec::new(),
            });
            "empty geometry has components"
        }
        TopologyClass::Holed => {
            for polygon in &mut broken.a.case.polygons {
                polygon.holes.clear();
            }
            "geometry lacks its requested topology witness"
        }
        TopologyClass::Disconnected => {
            broken
                .a
                .case
                .polygons
                .truncate(MIN_DISCONNECTED_COMPONENTS - 1);
            // `boolean::realised_domain` pins `min_components = 1` for every
            // non-Empty class, so a truncated pair fails the class **witness**
            // rather than the count.
            "geometry lacks its requested topology witness"
        }
        _ => {
            broken.a.case.polygons.clear();
            "component count is outside the domain"
        }
    };
    (broken, fragment)
}

/// Check one exposed paired-tree state against the generator's preconditions,
/// the deliberately broken input, and the documented reduction key.
///
/// `boolean::realize` validates the three *tagged* operands only, so the
/// rectangle pair — the other input family `H-BOOL-SET` consumes — is checked
/// here against the same ring oracle the property uses: a strictly positive,
/// counter-clockwise rectangle at every exposed state.
fn check_boolean_state(input: &boolean::BoolInput, limits: &GeometryLimits) -> (u128, usize) {
    for (label, rect) in [("A", input.rects.0), ("B", input.rects.1)] {
        let ring = rect.ring();
        validate_ring(&ring).unwrap_or_else(|error| {
            panic!("the {label} rectangle ring violates its own precondition: {error}")
        });
        assert!(
            signed_area2(&ring).expect("rectangle double area") > 0,
            "the {label} rectangle must keep positive area and counter-clockwise winding at every exposed state: {rect:?}"
        );
    }
    let realized = boolean::realize(input, limits).unwrap_or_else(|error| {
        panic!("an exposed paired-tree value violates the generator preconditions: {error}")
    });
    let expected_vertices = geometry_complexity(&input.a.case).expect("a complexity").2
        + geometry_complexity(&input.b.case).expect("b complexity").2;
    assert_eq!(
        count_vertices(&realized.a) + count_vertices(&realized.b),
        expected_vertices,
        "normalise, scale and translate must preserve the operands' vertex total"
    );
    let (broken, fragment) = broken_input(input);
    match boolean::realize(&broken, limits) {
        Ok(_) => panic!(
            "the deliberately broken paired input ({fragment}) must be rejected at every exposed state"
        ),
        Err(error) => assert!(
            error.contains(fragment),
            "the broken paired input must be rejected for `{fragment}`, got: {error}"
        ),
    }
    boolean_key(input, &realized)
}

/// Drive one seeded paired-geometry tree through current, simplify, a false
/// simplify, and repeated complicate.
fn audit_boolean_tree(seed: u64) -> Option<AuditFacts> {
    let limits = boolean_limits();
    let strategy = boolean::strategy(&limits).expect("H-BOOL-SET strategy");
    let mut runner = TestRunner::new(Config {
        cases: 1,
        rng_seed: RngSeed::Fixed(seed),
        failure_persistence: None,
        ..Config::default()
    });
    let mut tree = strategy.new_tree(&mut runner).expect("seeded paired tree");
    let start_key = check_boolean_state(&tree.current(), &limits);

    let mut walk = vec![start_key];
    let mut simplifications = 0_usize;
    let mut reductions = 0_usize;
    while tree.simplify() {
        simplifications += 1;
        let previous = *walk.last().expect("walk head");
        let key = check_boolean_state(&tree.current(), &limits);
        assert!(
            key <= previous,
            "a successful simplify must never enlarge the paired-tree key: {previous:?} -> {key:?}"
        );
        if key < previous {
            reductions += 1;
        }
        walk.push(key);
    }
    if simplifications == 0 || reductions == 0 {
        // Parameter-only shrinking of the rectangle/placement fields proves no
        // structural reduction, so this seed is not evidence.
        return None;
    }
    assert!(
        !tree.simplify(),
        "a tree already at its minimum must report false rather than loop"
    );
    let end_key = check_boolean_state(&tree.current(), &limits);
    assert!(
        end_key < start_key,
        "the paired-tree walk must end strictly smaller than its start: {start_key:?} -> {end_key:?}"
    );

    let mut complications = 0_usize;
    let mut last = end_key;
    while tree.complicate() {
        complications += 1;
        let key = check_boolean_state(&tree.current(), &limits);
        assert!(
            key >= last,
            "a successful complicate must restore a larger-or-equal paired-tree key: {last:?} -> {key:?}"
        );
        last = key;
    }
    assert!(
        last <= start_key,
        "complicate can only restore states the walk already exposed: {last:?} > {start_key:?}"
    );
    assert!(
        !tree.complicate(),
        "an exhausted restore history must report false rather than fabricate a state"
    );
    let restored = complicate_round_trip(&strategy, seed, |value| {
        check_boolean_state(value, &limits);
    });
    Some(AuditFacts {
        simplifications,
        complications,
        restored,
        start: start_key,
        end: end_key,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Mesh-scene audit (`H-MESH-SLICE` scene tree).
// ─────────────────────────────────────────────────────────────────────────────

/// The reference `H-MESH-SLICE` profile the control drives.
///
/// A small measured-by-construction profile: it is validated by the same
/// `checked_limits` the property uses, and it leaves the tree room to shrink.
fn scene_limits() -> mesh_slice::SliceLimits {
    mesh_slice::checked_limits(RawSliceBounds {
        max_extent_mm: SCENE_MAX_EXTENT_MM,
        max_translation_mm: SCENE_MAX_TRANSLATION_MM,
        max_planes: SCENE_MAX_PLANES,
    })
    .expect("H-MESH-SLICE reference scene profile")
}

/// The mesh-scene reduction key: the solid's whole-millimetre volume, then the
/// plane count.
///
/// Why this is non-increasing under a successful `simplify`: the scene tree is
/// a ten-field tuple, so one successful `simplify` shrinks exactly one field.
/// The extents binary-search **down** to `MIN_EXTENT_MM`, the plane count down
/// to one, the origin/base fields toward zero and `gap` toward one. Every
/// extent is at least `MIN_EXTENT_MM = 2 > 0`, so shrinking one factor can
/// never enlarge the product, and the plane count only falls. A shrink of an
/// origin, base or gap field changes neither component.
fn scene_key(scene: &mesh_slice::SliceScene) -> (u128, usize) {
    let solid = scene.solid;
    let volume = solid
        .w
        .checked_mul(solid.d)
        .and_then(|area| area.checked_mul(solid.h))
        .expect("bounded whole-mm solid volume");
    (
        u128::try_from(volume).expect("a whole-mm volume is positive"),
        scene.planes_mm.len(),
    )
}

/// Check one exposed scene value against its own preconditions, the
/// deliberately re-wound mesh, and the documented reduction key.
fn check_scene_state(scene: &mesh_slice::SliceScene) -> (u128, usize) {
    let floor = i64::try_from(mesh_slice::MIN_EXTENT_MM).expect("documented extent floor");
    let solid = scene.solid;
    assert!(
        solid.w >= floor && solid.d >= floor && solid.h >= floor,
        "every generated extent must stay at or above the documented floor: {solid:?}"
    );
    let mesh = solid.mesh();
    validate_indexed_mesh(&mesh, true).unwrap_or_else(|error| {
        panic!("an exposed scene value violates its closed-mesh precondition: {error}")
    });
    assert!(
        !scene.planes_mm.is_empty() && scene.planes_mm.windows(2).all(|pair| pair[0] < pair[1]),
        "the scene's planes must be nonempty and strictly increasing at every exposed state: {:?}",
        scene.planes_mm
    );
    let mut wrong = mesh;
    wrong.indices.swap(1, 2);
    assert!(
        validate_indexed_mesh(&wrong, true).is_err(),
        "a mesh with one triangle re-wound must be rejected at every exposed state"
    );
    scene_key(scene)
}

/// Drive one seeded mesh-scene tree through current, simplify, a false
/// simplify, and repeated complicate.
fn audit_scene_tree(seed: u64) -> Option<AuditFacts> {
    let limits = scene_limits();
    let strategy = mesh_slice::scene_strategy(limits);
    let mut runner = TestRunner::new(Config {
        cases: 1,
        rng_seed: RngSeed::Fixed(seed),
        failure_persistence: None,
        ..Config::default()
    });
    let mut tree = strategy.new_tree(&mut runner).expect("seeded scene tree");
    let start_key = check_scene_state(&tree.current());

    let mut walk = vec![start_key];
    let mut simplifications = 0_usize;
    let mut reductions = 0_usize;
    while tree.simplify() {
        simplifications += 1;
        let previous = *walk.last().expect("walk head");
        let key = check_scene_state(&tree.current());
        assert!(
            key <= previous,
            "a successful simplify must never enlarge the scene key: {previous:?} -> {key:?}"
        );
        if key < previous {
            reductions += 1;
        }
        walk.push(key);
    }
    if simplifications == 0 || reductions == 0 {
        // Placement-only shrinking proves no structural reduction, so this seed
        // is not evidence and the caller searches another seed.
        return None;
    }
    assert!(
        !tree.simplify(),
        "a tree already at its minimum must report false rather than loop"
    );
    let end_key = check_scene_state(&tree.current());
    assert!(
        end_key < start_key,
        "the scene walk must end strictly smaller than its start: {start_key:?} -> {end_key:?}"
    );

    let mut complications = 0_usize;
    let mut last = end_key;
    while tree.complicate() {
        complications += 1;
        let key = check_scene_state(&tree.current());
        assert!(
            key >= last,
            "a successful complicate must restore a larger-or-equal scene key: {last:?} -> {key:?}"
        );
        last = key;
    }
    assert!(
        last <= start_key,
        "complicate can only restore states the walk already exposed: {last:?} > {start_key:?}"
    );
    assert!(
        !tree.complicate(),
        "an exhausted restore history must report false rather than fabricate a state"
    );
    let restored = complicate_round_trip(&strategy, seed, |value| {
        check_scene_state(value);
    });
    Some(AuditFacts {
        simplifications,
        complications,
        restored,
        start: start_key,
        end: end_key,
    })
}

/// Simplification depths replayed when demonstrating complicate restoration.
pub const ROUND_TRIP_DEPTHS: std::ops::RangeInclusive<usize> = 1..=6;

/// Replay `seed` to an intermediate simplification depth and drive repeated
/// `complicate` from there, validating every restored state.
///
/// Why a replay is needed: a **fully** walked composed tree cannot complicate.
/// Proptest's integer `BinarySearch` exhausts a field on the very `simplify`
/// that returns `false` (it sets `hi = curr` and then cannot reposition), and a
/// tuple keeps only its **last** successful field's history — which is exactly
/// that exhausted field. Restoration is therefore reachable only before the
/// walk is driven to its minimum, so this replays the same deterministic seed
/// to a bounded depth and restores there. The exhausted tree is still driven by
/// the caller and must report `false` rather than fabricate a state.
fn complicate_round_trip<S>(strategy: &S, seed: u64, check: impl Fn(&S::Value)) -> usize
where
    S: Strategy,
{
    let mut best = 0_usize;
    for depth in ROUND_TRIP_DEPTHS {
        let mut runner = TestRunner::new(Config {
            cases: 1,
            rng_seed: RngSeed::Fixed(seed),
            failure_persistence: None,
            ..Config::default()
        });
        let mut tree = strategy.new_tree(&mut runner).expect("replayed tree");
        let mut applied = 0_usize;
        while applied < depth {
            if !tree.simplify() {
                break;
            }
            applied += 1;
            check(&tree.current());
        }
        if applied == 0 {
            continue;
        }
        let mut restored = 0_usize;
        while tree.complicate() {
            restored += 1;
            check(&tree.current());
        }
        best = best.max(restored);
    }
    best
}

/// Walk one audit's seeds and keep the trees that demonstrated a reduction.
fn walk_seeds(audit: impl Fn(u64) -> Option<AuditFacts>, seed_base: u64) -> Vec<AuditFacts> {
    (0..TREE_ATTEMPTS)
        .filter_map(|attempt| audit(seed_base + attempt))
        .collect()
}

/// Print the measured counts of one audit's seeding run.
fn report(label: &str, trees: &[AuditFacts]) {
    let simplifications: usize = trees.iter().map(|facts| facts.simplifications).sum();
    let complications: usize = trees.iter().map(|facts| facts.complications).sum();
    let restored = trees.iter().map(|facts| facts.restored).max().unwrap_or(0);
    for (index, facts) in trees.iter().enumerate() {
        println!(
            "geometry_controls_owned_shrink_witnesses: {label} tree[{index}] simplifications={} complications={} restored={} start={:?} end={:?}",
            facts.simplifications, facts.complications, facts.restored, facts.start, facts.end
        );
    }
    println!(
        "geometry_controls_owned_shrink_witnesses: {label} seeds={}/{TREE_ATTEMPTS} simplifications={simplifications} complications={complications} best_restored={restored}",
        trees.len()
    );
}

/// Run the owned shrink-witness audit over seeded trees.
pub fn run() {
    let mut total = 0_usize;
    let mut trees = 0_usize;
    for attempt in 0..TREE_ATTEMPTS {
        if let Some(simplifications) = audit_tree(0x5eed_c0de + attempt) {
            total += simplifications;
            trees += 1;
        }
    }
    assert!(
        trees > 0,
        "no seeded tree permitted a simplification in {TREE_ATTEMPTS} attempts: an initially minimal probe is not evidence"
    );
    assert!(
        total >= 2,
        "the audited trees must demonstrate more than one permitted reduction, got {total}"
    );
    println!(
        "geometry_controls_owned_shrink_witnesses: holed seeds={trees}/{TREE_ATTEMPTS} simplifications={total}"
    );

    let disconnected = walk_seeds(audit_disconnected_tree, DISCONNECTED_SEED_BASE);
    report("disconnected", &disconnected);
    let disconnected_simplifications: usize =
        disconnected.iter().map(|facts| facts.simplifications).sum();
    assert!(
        disconnected_simplifications >= 2,
        "the disconnected trees must demonstrate more than one permitted reduction in {TREE_ATTEMPTS} attempts, got {disconnected_simplifications}"
    );

    let paired = walk_seeds(audit_boolean_tree, BOOLEAN_SEED_BASE);
    report("paired", &paired);
    assert!(
        !paired.is_empty(),
        "the paired-geometry tree must demonstrate a structural reduction in {TREE_ATTEMPTS} attempts: an initially minimal or parameter-only probe is not evidence"
    );
    let paired_restored = paired.iter().map(|facts| facts.restored).max().unwrap_or(0);
    assert!(
        paired_restored >= 2,
        "the paired-geometry tree must demonstrate repeated complicate restoration, best was {paired_restored}"
    );

    let scenes = walk_seeds(audit_scene_tree, SCENE_SEED_BASE);
    report("scene", &scenes);
    let scene_reductions: usize = scenes
        .iter()
        .map(|facts| facts.simplifications)
        .sum::<usize>();
    assert!(
        scene_reductions >= 2,
        "the mesh-scene trees must demonstrate more than one permitted reduction in {TREE_ATTEMPTS} attempts, got {scene_reductions}"
    );
    let scene_restored = scenes.iter().map(|facts| facts.restored).max().unwrap_or(0);
    assert!(
        scene_restored >= 2,
        "the mesh-scene tree must demonstrate repeated complicate restoration, best was {scene_restored}"
    );
}
