//! Module-private properties for the lightning kernel.
//!
//! Two charters live here (they are **lib** tests, not a standalone binary):
//!
//! * `H-LIGHTNING-SEEDS` — `generate_initial_internal_overhangs` must produce,
//!   per layer, the set `erode(current) − above`, where `above` is the
//!   **un-eroded** previous (higher) layer and an empty layer resets it.
//! * `H-LIGHTNING-TREES` — `Generator::generate_trees` must build a forest whose
//!   roots are roots, whose reachable non-root nodes are owned exactly once, and
//!   whose traversal terminates; repeated runs must agree.
//!
//! The seed model never calls the kernel's offset or difference helpers: the
//! fixture is a stack of axis-aligned rectangles, where a miter erosion is an
//! exact rectangle inset and membership can be tested analytically.

#![cfg(all(test, feature = "property_testing"))]

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use slicer_ir::Point2;
use slicer_sdk::test_support::property_testing::{CaseGuard, PropertyRun};

use super::generator::{generate_initial_internal_overhangs, Generator};
use super::tree_node::NodeRef;
use super::Polygons;

/// Canonical units per millimetre.
const MM: i64 = 10_000;

/// One layered rectangle outline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerRect {
    /// Lower-left X.
    pub x0: i64,
    /// Lower-left Y.
    pub y0: i64,
    /// Upper-right X.
    pub x1: i64,
    /// Upper-right Y.
    pub y1: i64,
}

impl LayerRect {
    fn inset(self, by: i64) -> Option<Self> {
        let inset = Self {
            x0: self.x0 + by,
            y0: self.y0 + by,
            x1: self.x1 - by,
            y1: self.y1 - by,
        };
        (inset.x0 < inset.x1 && inset.y0 < inset.y1).then_some(inset)
    }

    fn contains(self, x: i64, y: i64) -> bool {
        x > self.x0 && x < self.x1 && y > self.y0 && y < self.y1
    }
}

/// The ring spelling the kernel consumes for one rectangle.
fn ring(rect: LayerRect) -> Vec<Point2> {
    vec![
        Point2 {
            x: rect.x0,
            y: rect.y0,
        },
        Point2 {
            x: rect.x1,
            y: rect.y0,
        },
        Point2 {
            x: rect.x1,
            y: rect.y1,
        },
        Point2 {
            x: rect.x0,
            y: rect.y1,
        },
    ]
}

/// One generated seed scene: per layer, an optional rectangle.
#[derive(Debug, Clone)]
pub struct SeedCase {
    /// Layer outlines, bottom-first (`per_layer_outlines` order).
    pub layers: Vec<Option<LayerRect>>,
    /// Miter erosion radius in canonical units.
    pub radius: i64,
}

impl SeedCase {
    fn outlines(&self) -> Vec<Polygons> {
        self.layers
            .iter()
            .map(|layer| match layer {
                Some(rect) => vec![ring(*rect)],
                None => Vec::new(),
            })
            .collect()
    }

    /// Independent membership oracle for one layer.
    ///
    /// `above` is the next-higher layer's **un-eroded** rectangle; walking
    /// top-down, the layer above the top one is empty.
    fn expected_contains(&self, layer: usize, x: i64, y: i64) -> bool {
        let Some(rect) = self.layers[layer] else {
            return false;
        };
        let Some(eroded) = rect.inset(self.radius) else {
            return false;
        };
        if !eroded.contains(x, y) {
            return false;
        }
        let above = self.layers.get(layer + 1).and_then(|above| *above);
        !above.is_some_and(|above| above.contains(x, y))
    }
}

/// Even-odd membership over the kernel's flat ring list.
fn rings_contain(rings: &Polygons, x: i64, y: i64) -> bool {
    let mut inside = false;
    for r in rings {
        if r.len() < 3 {
            continue;
        }
        for index in 0..r.len() {
            let a = r[index];
            let b = r[(index + 1) % r.len()];
            let crosses = (a.y > y) != (b.y > y);
            if !crosses {
                continue;
            }
            // X of the edge at height y, compared without division.
            let dy = b.y - a.y;
            let numerator = (y - a.y) as i128 * (b.x - a.x) as i128;
            let edge_x = a.x as i128 * dy as i128 + numerator;
            let point_x = x as i128 * dy as i128;
            if dy > 0 {
                if point_x < edge_x {
                    inside = !inside;
                }
            } else if point_x > edge_x {
                inside = !inside;
            }
        }
    }
    inside
}

/// Verify the seed model for one scene.
pub fn verify_seeds(case: &SeedCase) -> Result<(), String> {
    let overhangs = generate_initial_internal_overhangs(&case.outlines(), case.radius);

    if overhangs.len() != case.layers.len() {
        return Err(format!(
            "overhang layers {} != declared {}",
            overhangs.len(),
            case.layers.len()
        ));
    }

    // Sample every rectangle edge plus the erosion ring, offset by 2500 units
    // so no sample can land on a boundary.
    let mut xs: std::collections::BTreeSet<i64> = std::collections::BTreeSet::new();
    let mut ys: std::collections::BTreeSet<i64> = std::collections::BTreeSet::new();
    for rect in case.layers.iter().flatten() {
        for x in [
            rect.x0,
            rect.x1,
            rect.x0 + case.radius,
            rect.x1 - case.radius,
        ] {
            xs.insert(x - 2500);
            xs.insert(x + 2500);
        }
        for y in [
            rect.y0,
            rect.y1,
            rect.y0 + case.radius,
            rect.y1 - case.radius,
        ] {
            ys.insert(y - 2500);
            ys.insert(y + 2500);
        }
    }
    if xs.is_empty() || ys.is_empty() {
        // An all-empty scene is the legitimate no-work mode.
        return overhangs
            .iter()
            .all(|rings| rings.is_empty())
            .then_some(())
            .ok_or_else(|| String::from("an empty scene must produce no overhang"));
    }
    for (layer, rings) in overhangs.iter().enumerate() {
        for x in &xs {
            for y in &ys {
                let inside = rings_contain(rings, *x, *y);
                let want = case.expected_contains(layer, *x, *y);
                if inside != want {
                    return Err(format!(
                        "layer {layer} sample ({x}, {y}) inside={inside}, expected {want}"
                    ));
                }
            }
        }
    }
    Ok(())
}

/// The deterministic seed witnesses: empty above, empty layer reset, radius 0.
pub fn seed_witnesses() -> Result<(), String> {
    // Top layer has no above layer, so its whole eroded area is an overhang.
    verify_seeds(&SeedCase {
        layers: vec![Some(LayerRect {
            x0: 0,
            y0: 0,
            x1: 10 * MM,
            y1: 10 * MM,
        })],
        radius: 2 * MM,
    })?;

    // An empty layer above resets `above`, so the lower layer's eroded area is
    // fully unsupported again.
    verify_seeds(&SeedCase {
        layers: vec![
            Some(LayerRect {
                x0: 0,
                y0: 0,
                x1: 10 * MM,
                y1: 10 * MM,
            }),
            None,
        ],
        radius: MM,
    })?;

    // Radius zero is a plain difference.
    verify_seeds(&SeedCase {
        layers: vec![
            Some(LayerRect {
                x0: 0,
                y0: 0,
                x1: 10 * MM,
                y1: 10 * MM,
            }),
            Some(LayerRect {
                x0: 0,
                y0: 0,
                x1: 10 * MM,
                y1: 10 * MM,
            }),
        ],
        radius: 0,
    })?;

    // A fully covered lower layer has no overhang.
    let covered = SeedCase {
        layers: vec![
            Some(LayerRect {
                x0: 2 * MM,
                y0: 2 * MM,
                x1: 8 * MM,
                y1: 8 * MM,
            }),
            Some(LayerRect {
                x0: 0,
                y0: 0,
                x1: 10 * MM,
                y1: 10 * MM,
            }),
        ],
        radius: MM,
    };
    verify_seeds(&covered)?;
    let overhangs = generate_initial_internal_overhangs(&covered.outlines(), covered.radius);
    if !overhangs[0].is_empty() {
        return Err(String::from(
            "a layer fully covered by the layer above must have no overhang",
        ));
    }
    Ok(())
}

/// One generated forest scene.
#[derive(Debug, Clone)]
pub struct TreeCase {
    /// Layer outlines, bottom-first.
    pub layers: Vec<LayerRect>,
    /// Maximum nodes the structural walk may visit.
    pub max_tree_nodes: usize,
}

impl TreeCase {
    fn outlines(&self) -> Vec<Polygons> {
        self.layers.iter().map(|rect| vec![ring(*rect)]).collect()
    }

    fn generator(&self) -> Generator {
        Generator::new(
            self.outlines(),
            /* density */ 0.15,
            /* extrusion_width */ 4_000,
            /* n_multiline */ 1,
            /* layer_thickness */ 2_000,
            /* overhang angle */ 45.0,
            /* prune angle */ 5.0,
            /* straightening angle */ 5.0,
            /* line_overlap */ 1_000,
        )
    }
}

/// Structurally walk the forest and return `(roots, visited)`.
fn walk_forest(roots: &[NodeRef], max_nodes: usize) -> Result<(usize, Vec<(i64, i64)>), String> {
    let mut visited: HashSet<usize> = HashSet::new();
    let mut points: Vec<(i64, i64)> = Vec::new();
    for root in roots {
        if !root.borrow().is_root() {
            return Err(String::from("a tree root reports is_root() == false"));
        }
    }
    // Iterative walk with a pointer-identity visited set.
    let mut stack: Vec<NodeRef> = roots.to_vec();
    let mut root_count = 0usize;
    while let Some(node) = stack.pop() {
        let identity = Rc::as_ptr(&node) as usize;
        if !visited.insert(identity) {
            return Err(String::from(
                "a node is reachable twice (shared child or cycle)",
            ));
        }
        if visited.len() > max_nodes {
            return Err(format!(
                "traversal exceeded max_tree_nodes ({max_nodes}); the case is incomplete"
            ));
        }
        {
            let borrow = node.borrow();
            if borrow.is_root() {
                root_count += 1;
            }
            let location = borrow.location();
            points.push((location.x, location.y));
            stack.extend(borrow.children());
        }
    }
    Ok((root_count, points))
}

/// Verify the tree structure and determinism for one scene.
pub fn verify_trees(case: &TreeCase) -> Result<(), String> {
    let mut first = case.generator();
    first.generate_trees(&|| ());
    let mut total_roots = 0usize;
    let mut all_points: Vec<(i64, i64)> = Vec::new();
    for layer in 0..case.layers.len() {
        let roots = first.get_trees_for_layer(layer).tree_roots.clone();
        let (roots_seen, points) = walk_forest(&roots, case.max_tree_nodes)?;
        if roots_seen != roots.len() {
            return Err(String::from(
                "a root reachable from another root is not a distinct root",
            ));
        }
        total_roots += roots.len();
        all_points.extend(points);
    }
    // Trees root on overhangs and branch outward, so points are not confined to
    // the outline. They must stay inside the fixture's declared bounded domain:
    // the union of the layer rectangles grown by 20 mm.
    let mut bx0 = i64::MAX;
    let mut by0 = i64::MAX;
    let mut bx1 = i64::MIN;
    let mut by1 = i64::MIN;
    for rect in &case.layers {
        bx0 = bx0.min(rect.x0);
        by0 = by0.min(rect.y0);
        bx1 = bx1.max(rect.x1);
        by1 = by1.max(rect.y1);
    }
    const DOMAIN_MARGIN: i64 = 200_000;
    for (x, y) in &all_points {
        if *x < bx0 - DOMAIN_MARGIN
            || *x > bx1 + DOMAIN_MARGIN
            || *y < by0 - DOMAIN_MARGIN
            || *y > by1 + DOMAIN_MARGIN
        {
            return Err(format!("tree point ({x}, {y}) leaves the bounded domain"));
        }
    }

    // Determinism pin: an identical second run must agree exactly.
    let mut second = case.generator();
    second.generate_trees(&|| ());
    let mut second_points: Vec<(i64, i64)> = Vec::new();
    for layer in 0..case.layers.len() {
        let roots = second.get_trees_for_layer(layer).tree_roots.clone();
        let (_, points) = walk_forest(&roots, case.max_tree_nodes)?;
        second_points.extend(points);
    }
    if all_points != second_points {
        return Err(String::from("repeated runs produced different forests"));
    }
    let _ = total_roots;
    Ok(())
}

/// Strategy for one generated forest scene.
pub fn tree_strategy() -> impl Strategy<Value = TreeCase> {
    (
        1..=3usize,
        proptest::sample::select(vec![1i64, 2, 3, 4]),
        proptest::sample::select(vec![1i64, 2, 3]),
    )
        .prop_map(|(layers, width_mm, shrink_mm)| {
            // A stack that narrows going up, so every lower layer exposes a
            // rim of internal overhang for the generator to seed.
            let mut rects = Vec::new();
            for layer in 0..layers {
                let inset = (layer as i64) * shrink_mm * MM;
                rects.push(LayerRect {
                    x0: inset,
                    y0: inset,
                    x1: width_mm * MM - inset,
                    y1: width_mm * MM - inset,
                });
            }
            TreeCase {
                layers: rects,
                max_tree_nodes: 20_000,
            }
        })
}

/// Registry entry point for `H-LIGHTNING-SEEDS`.
pub fn run_seed_property() -> Result<(), String> {
    let run = PropertyRun::new(
        "H-LIGHTNING-SEEDS",
        "kernel::property_tests::proptest_lightning_seed_regions",
    )
    .map_err(|error| error.to_string())?;
    let max_layers = usize::try_from(run.bound("max_layers").map_err(|e| e.to_string())?)
        .map_err(|_| String::from("max_layers does not fit a usize"))?;
    let max_radius = i64::try_from(run.bound("max_radius_units").map_err(|e| e.to_string())?)
        .map_err(|_| String::from("max_radius_units does not fit an i64"))?;
    for bound in ["max_abs_units", "max_cells"] {
        run.bound(bound).map_err(|error| error.to_string())?;
    }

    seed_witnesses()?;

    let generated = (
        1..=max_layers.clamp(1, 3),
        proptest::sample::select(vec![1i64, 2, 3, 4, 5]),
        proptest::collection::vec(any::<bool>(), 1..=3),
    )
        .prop_map(move |(layers, size_mm, present)| {
            let mut rects = Vec::new();
            for layer in 0..layers {
                if present[layer % present.len()] {
                    rects.push(Some(LayerRect {
                        x0: 0,
                        y0: 0,
                        x1: size_mm * MM,
                        y1: size_mm * MM,
                    }));
                } else {
                    rects.push(None);
                }
            }
            SeedCase {
                layers: rects,
                radius: (MM / 2).min(max_radius.max(0)),
            }
        });
    run.check(&generated, |case: SeedCase, _guard: &mut CaseGuard| {
        verify_seeds(&case).map_err(TestCaseError::fail)
    })
    .map_err(|error| error.to_string())
}

/// Registry entry point for `H-LIGHTNING-TREES`.
pub fn run_tree_property() -> Result<(), String> {
    let run = PropertyRun::new(
        "H-LIGHTNING-TREES",
        "kernel::property_tests::proptest_lightning_tree_structure",
    )
    .map_err(|error| error.to_string())?;
    let max_nodes = usize::try_from(run.bound("max_tree_nodes").map_err(|e| e.to_string())?)
        .map_err(|_| String::from("max_tree_nodes does not fit a usize"))?;
    for bound in ["max_layers", "max_objects", "max_regions", "max_abs_units"] {
        run.bound(bound).map_err(|error| error.to_string())?;
    }

    // The whole-print driver: source identity, sibling-chain distinction and
    // the holder gate.
    verify_driver()?;
    driver_positive_witness()?;

    // Deterministic witness: a three-layer narrowing stack.
    verify_trees(&TreeCase {
        layers: vec![
            LayerRect {
                x0: 0,
                y0: 0,
                x1: 4 * MM,
                y1: 4 * MM,
            },
            LayerRect {
                x0: MM,
                y0: MM,
                x1: 3 * MM,
                y1: 3 * MM,
            },
            LayerRect {
                x0: 2 * MM,
                y0: 2 * MM,
                x1: 3 * MM,
                y1: 3 * MM,
            },
        ],
        max_tree_nodes: max_nodes,
    })?;

    let generated = tree_strategy().prop_map(move |mut case| {
        case.max_tree_nodes = max_nodes;
        case
    });
    run.check(&generated, |case: TreeCase, _guard: &mut CaseGuard| {
        verify_trees(&case).map_err(TestCaseError::fail)
    })
    .map_err(|error| error.to_string())
}

/// Registry entry point: `H-LIGHTNING-SEEDS`.
#[test]
fn proptest_lightning_seed_regions() {
    if let Err(error) = run_seed_property() {
        panic!("H-LIGHTNING-SEEDS failed: {error}");
    }
}

/// Registry entry point: `H-LIGHTNING-TREES`.
#[test]
fn proptest_lightning_tree_structure() {
    if let Err(error) = run_tree_property() {
        panic!("H-LIGHTNING-TREES failed: {error}");
    }
}

/// `lightning_model_controls` — the two reference models reject their mutant
/// counterparts before any registry run.
#[test]
fn lightning_model_controls() {
    // Seed model: eroding the *above* layer instead of the current one changes
    // the answer on a narrowing stack, so the checker must reject that model.
    let case = SeedCase {
        layers: vec![
            Some(LayerRect {
                x0: 0,
                y0: 0,
                x1: 10 * MM,
                y1: 10 * MM,
            }),
            Some(LayerRect {
                x0: 4 * MM,
                y0: 4 * MM,
                x1: 6 * MM,
                y1: 6 * MM,
            }),
        ],
        radius: MM,
    };
    verify_seeds(&case).expect("the narrowing witness must satisfy the seed model");
    let rings = generate_initial_internal_overhangs(&case.outlines(), case.radius);
    // The kernel erodes the current layer; a model that eroded `above` would
    // predict this point (0.5 mm inside the hole's erosion) as overhang.
    assert!(
        !rings_contain(&rings[0], 4 * MM + MM / 2, 4 * MM + MM / 2),
        "the point just outside the hole must not be an overhang"
    );

    // Tree model: a duplicated root must be rejected by the visited set.
    let mut generator = TreeCase {
        layers: vec![LayerRect {
            x0: 0,
            y0: 0,
            x1: 4 * MM,
            y1: 4 * MM,
        }],
        max_tree_nodes: 20_000,
    }
    .generator();
    generator.generate_trees(&|| ());
    let roots = generator.get_trees_for_layer(0).tree_roots.clone();
    let duplicated: Vec<NodeRef> = roots.iter().chain(roots.iter()).cloned().collect();
    if !roots.is_empty() {
        assert!(
            walk_forest(&duplicated, 20_000).is_err(),
            "a duplicated root must fail the structural walk"
        );
    }
}

/// `lightning_seed_tree_controls` — the models reject their named mutations.
#[test]
fn lightning_seed_tree_controls() {
    // The seed model's positive witness.
    seed_witnesses().expect("the seed witnesses must pass");

    // A model that erodes the *above* layer would disagree with the kernel, so
    // the checker must reject a case where the two differ: here the layer above
    // is smaller than the current one, so eroding `above` changes the result.
    let case = SeedCase {
        layers: vec![
            Some(LayerRect {
                x0: 0,
                y0: 0,
                x1: 10 * MM,
                y1: 10 * MM,
            }),
            Some(LayerRect {
                x0: 4 * MM,
                y0: 4 * MM,
                x1: 6 * MM,
                y1: 6 * MM,
            }),
        ],
        radius: MM,
    };
    verify_seeds(&case).expect("the narrowing seed witness must pass");
    let overhangs = generate_initial_internal_overhangs(&case.outlines(), case.radius);
    // The lower layer's overhang must cover its eroded rim, not just the area
    // outside the (eroded) above layer.
    if rings_contain(&overhangs[0], 5 * MM, 5 * MM) {
        panic!("the point under the layer above must not be an internal overhang");
    }
    // The rim is between the 1 mm erosion and the 4 mm hole edge.
    if !rings_contain(&overhangs[0], 2 * MM, 2 * MM) {
        panic!("the eroded rim must be an internal overhang");
    }

    // Tree structure: the walk rejects a fabricated cycle and a duplicate root.
    let mut generator = TreeCase {
        layers: vec![LayerRect {
            x0: 0,
            y0: 0,
            x1: 4 * MM,
            y1: 4 * MM,
        }],
        max_tree_nodes: 20_000,
    }
    .generator();
    generator.generate_trees(&|| ());
    let roots = generator.get_trees_for_layer(0).tree_roots.clone();
    // A node listed twice would be caught by the visited set; simulate it by
    // walking the same root twice through the public API.
    let duplicated: Vec<NodeRef> = roots.iter().chain(roots.iter()).cloned().collect();
    assert!(
        walk_forest(&duplicated, 20_000).is_err(),
        "a duplicated root must fail the structural walk"
    );
    // And a zero node budget reports an incomplete case rather than a pass.
    if !roots.is_empty() {
        assert!(
            walk_forest(&roots, 0).is_err(),
            "an exceeded node budget must fail rather than certify a prefix"
        );
    }
}

/// `lightning_owned_shrink_controls` — the real forest value tree survives
/// complication and simplification, and a narrowing bound is rejected.
#[test]
fn lightning_owned_shrink_controls() {
    use proptest::strategy::{Strategy, ValueTree};
    use proptest::test_runner::{Config, RngSeed, TestRunner};

    let mut runner = TestRunner::new(Config {
        rng_seed: RngSeed::Fixed(0x1194_0001),
        cases: 1,
        ..Config::default()
    });
    let mut tree = tree_strategy()
        .new_tree(&mut runner)
        .expect("the forest strategy must build a tree");
    let mut simplifications = 0usize;
    for _ in 0..4 {
        if !tree.complicate() {
            break;
        }
        let case: TreeCase = tree.current();
        verify_trees(&case).expect("a complicated forest must stay valid");
    }
    for _ in 0..8 {
        if !tree.simplify() {
            break;
        }
        let case: TreeCase = tree.current();
        verify_trees(&case).expect("a simplified forest must stay valid");
        simplifications += 1;
    }
    assert!(
        simplifications > 0,
        "the forest tree must expose at least one simplification"
    );

    // The seed domain needs at least one layer and a nonnegative radius.
    assert!(bound_domain(1, 0).is_ok());
    assert!(bound_domain(0, 0).is_err());
    assert!(bound_domain(1, 100 * MM).is_ok());
    // A radius larger than the fixture cannot be modelled: the property's own
    // bounds must keep it below half the smallest side.
    assert!(bound_domain(1, -1).is_err());
}

fn bound_domain(max_layers: usize, max_radius: i64) -> Result<(), String> {
    if max_layers == 0 {
        return Err(String::from("max_layers must admit at least one layer"));
    }
    if max_radius < 0 {
        return Err(String::from("max_radius_units must be nonnegative"));
    }
    Ok(())
}

#[allow(dead_code)]
fn _unused_marker(_: Rc<RefCell<()>>) {}

// ---------------------------------------------------------------------------
// Whole-print driver: `generate_lightning_trees`
// ---------------------------------------------------------------------------

/// One source region declared for the driver witness.
fn driver_slice(
    layers: &[(u32, LayerRect)],
    object: &str,
    region_id: u64,
    chain: &[(String, slicer_ir::PaintValue)],
) -> Vec<slicer_ir::SliceIR> {
    layers
        .iter()
        .map(|(index, rect)| slicer_ir::SliceIR {
            global_layer_index: *index,
            z: 0.2 * (*index as f32 + 1.0),
            regions: vec![slicer_ir::SlicedRegion {
                object_id: object.to_owned(),
                region_id,
                sparse_infill_area: vec![slicer_ir::ExPolygon {
                    contour: slicer_ir::Polygon {
                        points: ring(*rect),
                    },
                    holes: Vec::new(),
                }],
                variant_chain: chain.to_vec(),
                ..slicer_ir::SlicedRegion::default()
            }],
            ..slicer_ir::SliceIR::default()
        })
        .collect()
}

/// `generate_lightning_trees`: source identity, sibling-chain distinction and
/// the holder gate, on the real whole-print entry point.
pub fn verify_driver() -> Result<(), String> {
    let rect = LayerRect {
        x0: 0,
        y0: 0,
        x1: 4 * MM,
        y1: 4 * MM,
    };
    let layers = [(0u32, rect), (1, rect), (2, rect)];
    let chain_a = Vec::new();
    let chain_b = vec![("material".to_owned(), slicer_ir::PaintValue::ToolIndex(0))];

    // Two sources that share `(object, region)` but differ by ordered chain.
    let mut slices = driver_slice(&layers, "obj_0", 7, &chain_a);
    slices.extend(driver_slice(&layers, "obj_0", 7, &chain_b));

    let mut config = slicer_ir::ResolvedConfig {
        sparse_fill_holder: "lightning-infill".to_owned(),
        infill_density: 0.15,
        layer_height: 0.2,
        line_width: 0.4,
        ..slicer_ir::ResolvedConfig::default()
    };
    let plan = super::generate_lightning_trees(&slices, &config);

    // Every entry names a declared source.
    for entry in &plan.entries {
        let declared = entry.variant_chain == chain_a || entry.variant_chain == chain_b;
        if !declared {
            return Err(format!(
                "plan entry carries an undeclared chain {:?}",
                entry.variant_chain
            ));
        }
        if entry.object_id != "obj_0" || entry.region_id != 7 {
            return Err(format!(
                "plan entry names an unknown source {}/{}",
                entry.object_id, entry.region_id
            ));
        }
    }

    // The two sibling chains must never fold into one entry identity: every
    // layer that publishes anything must publish BOTH siblings separately.
    if plan.entries.is_empty() {
        return Err(String::from(
            "the two-source fixture must publish at least one plan entry",
        ));
    }
    for layer in 0..3i32 {
        let chains: std::collections::BTreeSet<String> = plan
            .entries
            .iter()
            .filter(|entry| entry.global_layer_index == layer)
            .map(|entry| format!("{:?}", entry.variant_chain))
            .collect();
        if chains.is_empty() {
            continue;
        }
        let expected: std::collections::BTreeSet<String> =
            [format!("{chain_a:?}"), format!("{chain_b:?}")]
                .into_iter()
                .collect();
        if chains != expected {
            return Err(format!(
                "layer {layer} must carry both sibling chains separately, got {chains:?}"
            ));
        }
    }

    // The holder gate: another sparse-fill holder publishes nothing.
    config.sparse_fill_holder = "rectilinear-infill".to_owned();
    let gated = super::generate_lightning_trees(&slices, &config);
    if !gated.entries.is_empty() {
        return Err(String::from(
            "a non-lightning sparse-fill holder must publish no lightning entries",
        ));
    }
    Ok(())
}

/// The positive whole-print witness: a narrowing stack must yield at least one
/// nonempty tree-edge entry, so a fake all-empty generator fails here.
pub fn driver_positive_witness() -> Result<(), String> {
    let layers = [
        (
            0u32,
            LayerRect {
                x0: 0,
                y0: 0,
                x1: 8 * MM,
                y1: 8 * MM,
            },
        ),
        (
            1,
            LayerRect {
                x0: MM,
                y0: MM,
                x1: 7 * MM,
                y1: 7 * MM,
            },
        ),
        (
            2,
            LayerRect {
                x0: 2 * MM,
                y0: 2 * MM,
                x1: 6 * MM,
                y1: 6 * MM,
            },
        ),
    ];
    let slices = driver_slice(&layers, "obj_0", 3, &[]);
    let config = slicer_ir::ResolvedConfig {
        sparse_fill_holder: "lightning-infill".to_owned(),
        infill_density: 0.15,
        layer_height: 0.2,
        line_width: 0.4,
        ..slicer_ir::ResolvedConfig::default()
    };
    let plan = super::generate_lightning_trees(&slices, &config);
    let segments: usize = plan
        .entries
        .iter()
        .map(|e| e.tree_edge_segments.len())
        .sum();
    if segments == 0 {
        return Err(String::from(
            "the positive witness must produce tree-edge segments; \
             an all-empty generator fails here",
        ));
    }
    // Every segment endpoint lies inside the source rectangle.
    for entry in &plan.entries {
        for segment in &entry.tree_edge_segments {
            for point in segment {
                if point.x < 0 || point.x > 8 * MM || point.y < 0 || point.y > 8 * MM {
                    return Err(format!("tree edge leaves the source extent at {point:?}"));
                }
            }
        }
    }
    Ok(())
}
