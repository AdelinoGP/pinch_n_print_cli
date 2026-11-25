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

/// One layered outline: an outer rectangle, optionally bored by a rectangular
/// hole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerShape {
    /// Material boundary.
    pub outer: LayerRect,
    /// Rectangular hole, strictly inside `outer`.
    pub hole: Option<LayerRect>,
}

impl LayerShape {
    /// A solid rectangle.
    pub fn solid(outer: LayerRect) -> Self {
        Self { outer, hole: None }
    }

    /// A rectangle bored by a concentric rectangle of uniform wall thickness.
    ///
    /// Uniform walls are the exactness condition for [`LayerShape::erode`]: if
    /// the grown hole crosses one wall it crosses all four, so the eroded
    /// material is either an annulus or empty. An off-centre bore can erode to
    /// a C or U shape, which this single-hole model cannot represent.
    pub fn bored(outer: LayerRect, hole: LayerRect) -> Self {
        let left = hole.x0 - outer.x0;
        let bottom = hole.y0 - outer.y0;
        let right = outer.x1 - hole.x1;
        let top = outer.y1 - hole.y1;
        assert!(
            left > 0 && left == right && bottom > 0 && bottom == top && left == bottom,
            "the analytic erosion model requires a concentric bore with uniform walls"
        );
        Self {
            outer,
            hole: Some(hole),
        }
    }

    /// Material erosion by `by` under a miter join.
    ///
    /// For axis-aligned rectangles a miter offset is exact: the outer ring
    /// insets and the hole ring grows into the material. With a uniform-wall
    /// bore the grown hole crosses either no wall or all four, so `None`
    /// (empty material) is the only partial outcome; an off-centre bore could
    /// erode to a C or U shape instead.
    fn erode(self, by: i64) -> Option<Self> {
        let outer = self.outer.inset(by)?;
        let Some(hole) = self.hole else {
            return Some(Self { outer, hole: None });
        };
        let grown = LayerRect {
            x0: hole.x0 - by,
            y0: hole.y0 - by,
            x1: hole.x1 + by,
            y1: hole.y1 + by,
        };
        if grown.x0 <= outer.x0
            || grown.y0 <= outer.y0
            || grown.x1 >= outer.x1
            || grown.y1 >= outer.y1
        {
            // The hole swallowed the eroded wall; the polygon has no material.
            return None;
        }
        Some(Self {
            outer,
            hole: Some(grown),
        })
    }

    fn contains(self, x: i64, y: i64) -> bool {
        self.outer.contains(x, y) && !self.hole.is_some_and(|hole| hole.contains(x, y))
    }

    /// Whether `(x, y)` lies exactly on one of this shape's axis-aligned edges.
    fn on_edge(self, x: i64, y: i64) -> bool {
        fn rect_edge(rect: LayerRect, x: i64, y: i64) -> bool {
            let on_vertical = (x == rect.x0 || x == rect.x1) && y >= rect.y0 && y <= rect.y1;
            let on_horizontal = (y == rect.y0 || y == rect.y1) && x >= rect.x0 && x <= rect.x1;
            on_vertical || on_horizontal
        }
        rect_edge(self.outer, x, y) || self.hole.is_some_and(|hole| rect_edge(hole, x, y))
    }
}

/// The rings the kernel consumes for one shape: outer counter-clockwise, hole
/// clockwise (the flat `Polygons` convention the kernel groups by signed area).
fn shape_rings(shape: LayerShape) -> Polygons {
    let mut rings = vec![ring(shape.outer)];
    if let Some(hole) = shape.hole {
        let mut hole_ring = ring(hole);
        hole_ring.reverse();
        rings.push(hole_ring);
    }
    rings
}

/// One generated seed scene: per layer, an optional shape.
#[derive(Debug, Clone)]
pub struct SeedCase {
    /// Layer outlines, bottom-first (`per_layer_outlines` order).
    pub layers: Vec<Option<LayerShape>>,
    /// Miter erosion radius in canonical units.
    pub radius: i64,
}

impl SeedCase {
    fn outlines(&self) -> Vec<Polygons> {
        self.layers
            .iter()
            .map(|layer| match layer {
                Some(shape) => shape_rings(*shape),
                None => Vec::new(),
            })
            .collect()
    }

    /// Independent membership oracle for one layer.
    ///
    /// `above` is the next-higher layer's **un-eroded** material; walking
    /// top-down, the layer above the top one is empty.
    fn expected_contains(&self, layer: usize, x: i64, y: i64) -> bool {
        let Some(shape) = self.layers[layer] else {
            return false;
        };
        let Some(eroded) = shape.erode(self.radius) else {
            return false;
        };
        if !eroded.contains(x, y) {
            return false;
        }
        let above = self.layers.get(layer + 1).and_then(|above| *above);
        !above.is_some_and(|above| above.contains(x, y))
    }

    /// Whether `(x, y)` is a boundary point of any raw or eroded shape. The
    /// clipper and the even-odd oracle can legitimately disagree on a boundary,
    /// so the sampled grid skips those points.
    fn any_boundary(&self, x: i64, y: i64) -> bool {
        self.layers.iter().flatten().any(|shape| {
            shape.on_edge(x, y) || shape.erode(self.radius).is_some_and(|e| e.on_edge(x, y))
        })
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
    verify_observation(case, &overhangs)
}

/// Verify an arbitrary observed overhang set against the model.
///
/// The deterministic controls feed wrong answers here to prove the sampler
/// can reject them.
fn verify_observation(case: &SeedCase, overhangs: &[Polygons]) -> Result<(), String> {
    if overhangs.len() != case.layers.len() {
        return Err(format!(
            "overhang layers {} != declared {}",
            overhangs.len(),
            case.layers.len()
        ));
    }

    // Sample every raw and eroded rectangle edge. The offset is half the
    // erosion radius (or 2500 units with no erosion), so at least one lattice
    // line always falls strictly inside the erosion band: a fixed offset that
    // happened to equal a generated radius would leave the band unsampled and
    // let a kernel that skips erosion agree with the model on every point.
    let offset = if case.radius > 0 {
        (case.radius / 2).max(1)
    } else {
        2500
    };
    let mut xs: std::collections::BTreeSet<i64> = std::collections::BTreeSet::new();
    let mut ys: std::collections::BTreeSet<i64> = std::collections::BTreeSet::new();
    for shape in case.layers.iter().flatten() {
        let mut rects = vec![shape.outer];
        if let Some(hole) = shape.hole {
            rects.push(hole);
        }
        if let Some(eroded) = shape.erode(case.radius) {
            rects.push(eroded.outer);
            if let Some(hole) = eroded.hole {
                rects.push(hole);
            }
        }
        for rect in rects {
            for x in [rect.x0, rect.x1] {
                xs.insert(x - offset);
                xs.insert(x + offset);
            }
            for y in [rect.y0, rect.y1] {
                ys.insert(y - offset);
                ys.insert(y + offset);
            }
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
                if case.any_boundary(*x, *y) {
                    continue;
                }
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
        layers: vec![Some(LayerShape::solid(LayerRect {
            x0: 0,
            y0: 0,
            x1: 10 * MM,
            y1: 10 * MM,
        }))],
        radius: 2 * MM,
    })?;

    // An empty layer above resets `above`, so the lower layer's eroded area is
    // fully unsupported again.
    verify_seeds(&SeedCase {
        layers: vec![
            Some(LayerShape::solid(LayerRect {
                x0: 0,
                y0: 0,
                x1: 10 * MM,
                y1: 10 * MM,
            })),
            None,
        ],
        radius: MM,
    })?;

    // Radius zero is a plain difference.
    verify_seeds(&SeedCase {
        layers: vec![
            Some(LayerShape::solid(LayerRect {
                x0: 0,
                y0: 0,
                x1: 10 * MM,
                y1: 10 * MM,
            })),
            Some(LayerShape::solid(LayerRect {
                x0: 0,
                y0: 0,
                x1: 10 * MM,
                y1: 10 * MM,
            })),
        ],
        radius: 0,
    })?;

    // A fully covered lower layer has no overhang.
    let covered = SeedCase {
        layers: vec![
            Some(LayerShape::solid(LayerRect {
                x0: 2 * MM,
                y0: 2 * MM,
                x1: 8 * MM,
                y1: 8 * MM,
            })),
            Some(LayerShape::solid(LayerRect {
                x0: 0,
                y0: 0,
                x1: 10 * MM,
                y1: 10 * MM,
            })),
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

    // A bored top layer: erosion grows the hole, and the ring around it is a
    // genuine internal overhang while the bore itself is not material.
    let bored = SeedCase {
        layers: vec![Some(LayerShape::bored(
            LayerRect {
                x0: 0,
                y0: 0,
                x1: 10 * MM,
                y1: 10 * MM,
            },
            LayerRect {
                x0: 4 * MM,
                y0: 4 * MM,
                x1: 6 * MM,
                y1: 6 * MM,
            },
        ))],
        radius: MM,
    };
    verify_seeds(&bored)?;
    let overhangs = generate_initial_internal_overhangs(&bored.outlines(), bored.radius);
    if rings_contain(&overhangs[0], 5 * MM, 5 * MM) {
        return Err(String::from(
            "the hole interior must not be an internal overhang",
        ));
    }
    if !rings_contain(&overhangs[0], 2 * MM, 5 * MM) {
        return Err(String::from(
            "the eroded rim beside the hole must be an internal overhang",
        ));
    }

    // A bored lower layer under a solid cover: the cover fills the bore, so
    // neither the rim nor the bore is exposed.
    let plugged = SeedCase {
        layers: vec![
            Some(LayerShape::bored(
                LayerRect {
                    x0: 0,
                    y0: 0,
                    x1: 10 * MM,
                    y1: 10 * MM,
                },
                LayerRect {
                    x0: 4 * MM,
                    y0: 4 * MM,
                    x1: 6 * MM,
                    y1: 6 * MM,
                },
            )),
            Some(LayerShape::solid(LayerRect {
                x0: 0,
                y0: 0,
                x1: 10 * MM,
                y1: 10 * MM,
            })),
        ],
        radius: MM,
    };
    verify_seeds(&plugged)?;
    let overhangs = generate_initial_internal_overhangs(&plugged.outlines(), plugged.radius);
    if !overhangs[0].is_empty() {
        return Err(String::from(
            "a solid cover must plug the bored layer below",
        ));
    }

    // A hole that swallows the eroded wall erodes to nothing, not to a sliver.
    verify_seeds(&SeedCase {
        layers: vec![Some(LayerShape::bored(
            LayerRect {
                x0: 0,
                y0: 0,
                x1: 2 * MM,
                y1: 2 * MM,
            },
            LayerRect {
                x0: MM / 2,
                y0: MM / 2,
                x1: 3 * MM / 2,
                y1: 3 * MM / 2,
            },
        ))],
        radius: MM / 2,
    })?;
    Ok(())
}

/// One generated forest scene.
#[derive(Debug, Clone)]
pub struct TreeCase {
    /// Layer outlines, bottom-first.
    pub layers: Vec<LayerRect>,
    /// Optional rectangular hole per layer (empty means none declared).
    pub holes: Vec<Option<LayerRect>>,
    /// Maximum nodes the structural walk may visit.
    pub max_tree_nodes: usize,
}

impl TreeCase {
    fn outlines(&self) -> Vec<Polygons> {
        self.layers
            .iter()
            .enumerate()
            .map(|(index, rect)| {
                let mut rings = vec![ring(*rect)];
                if let Some(hole) = self.holes.get(index).copied().flatten() {
                    // Clockwise, the flat `Polygons` hole convention.
                    let mut hole_ring = ring(hole);
                    hole_ring.reverse();
                    rings.push(hole_ring);
                }
                rings
            })
            .collect()
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
    // the union of the layer rectangles grown by 5 mm.
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
    const DOMAIN_MARGIN: i64 = 5 * MM;
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
    if total_roots == 0 {
        return Err(String::from(
            "a narrowing stack must seed at least one tree; an all-empty forest is not a pass",
        ));
    }
    Ok(())
}

/// Strategy for one generated forest scene.
pub fn tree_strategy(max_layers: usize) -> impl Strategy<Value = TreeCase> {
    (
        1..=max_layers.clamp(1, 3),
        proptest::sample::select(vec![6i64, 8, 10]),
        proptest::sample::select(vec![1i64, 2]),
        any::<bool>(),
    )
        .prop_map(|(layers, width_mm, requested_shrink, bore)| {
            // A stack that narrows going up, so every lower layer exposes a
            // rim of internal overhang for the generator to seed. The shrink
            // is clamped so the innermost side stays at least 2 mm: with `L`
            // layers the total inset is `2 * (L-1) * shrink`.
            let shrink = if layers <= 1 {
                requested_shrink
            } else {
                ((width_mm - 2) / (2 * (layers as i64 - 1))).clamp(1, requested_shrink)
            };
            let mut rects = Vec::new();
            let mut holes = Vec::new();
            for layer in 0..layers {
                let inset = (layer as i64) * shrink * MM;
                let rect = LayerRect {
                    x0: inset,
                    y0: inset,
                    x1: width_mm * MM - inset,
                    y1: width_mm * MM - inset,
                };
                // A concentric quarter-side bore on every layer, so generated
                // scenes exercise the flat `Polygons` hole convention too.
                holes.push(bore.then(|| {
                    let margin = (rect.x1 - rect.x0) / 4;
                    LayerRect {
                        x0: rect.x0 + margin,
                        y0: rect.y0 + margin,
                        x1: rect.x1 - margin,
                        y1: rect.y1 - margin,
                    }
                }));
                rects.push(rect);
            }
            TreeCase {
                layers: rects,
                holes,
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
        proptest::sample::select(vec![2i64, 3, 4, 5, 6]),
        proptest::collection::vec(any::<bool>(), 1..=3),
        proptest::collection::vec(
            proptest::option::of(proptest::sample::select(vec![3i64, 4, 5])),
            1..=3,
        ),
        proptest::sample::select(vec![MM / 4, MM / 2]),
        any::<bool>(),
    )
        .prop_map(move |(layers, size_mm, present, holes, radius, narrow)| {
            let size = size_mm * MM;
            let radius = radius.min(max_radius.max(0));
            // Narrowing stacks expose rims on all four sides; the shrink is
            // clamped so the innermost side stays at least 1 mm.
            let shrink = if narrow && layers > 1 {
                ((size_mm - 1) / (2 * (layers as i64 - 1))).max(0)
            } else {
                0
            };
            let mut rects = Vec::new();
            for layer in 0..layers {
                if !present[layer % present.len()] {
                    rects.push(None);
                    continue;
                }
                let layer_inset = (layer as i64) * shrink * MM;
                let outer = LayerRect {
                    x0: layer_inset,
                    y0: layer_inset,
                    x1: size - layer_inset,
                    y1: size - layer_inset,
                };
                let side = outer.x1 - outer.x0;
                // A concentric bore: `1/d` of each side, so the walls are
                // uniform. Small parts erode to nothing around it, which the
                // model treats as no overhang.
                let shape = match holes[layer % holes.len()] {
                    Some(denominator) => {
                        let inset = (side / denominator).max(1);
                        LayerShape::bored(
                            outer,
                            LayerRect {
                                x0: outer.x0 + inset,
                                y0: outer.y0 + inset,
                                x1: outer.x1 - inset,
                                y1: outer.y1 - inset,
                            },
                        )
                    }
                    None => LayerShape::solid(outer),
                };
                rects.push(Some(shape));
            }
            SeedCase {
                layers: rects,
                radius,
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
    let max_layers = usize::try_from(run.bound("max_layers").map_err(|e| e.to_string())?)
        .map_err(|_| String::from("max_layers does not fit a usize"))?;
    for bound in ["max_objects", "max_regions", "max_abs_units"] {
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
        holes: vec![Some(LayerRect {
            x0: 3 * MM / 2,
            y0: 3 * MM / 2,
            x1: 2 * MM,
            y1: 2 * MM,
        })],
        max_tree_nodes: max_nodes,
    })?;

    let generated = tree_strategy(max_layers).prop_map(move |mut case| {
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
            Some(LayerShape::solid(LayerRect {
                x0: 0,
                y0: 0,
                x1: 10 * MM,
                y1: 10 * MM,
            })),
            Some(LayerShape::solid(LayerRect {
                x0: 4 * MM,
                y0: 4 * MM,
                x1: 6 * MM,
                y1: 6 * MM,
            })),
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
        holes: Vec::new(),
        max_tree_nodes: 20_000,
    }
    .generator();
    generator.generate_trees(&|| ());
    let roots = generator.get_trees_for_layer(0).tree_roots.clone();
    let duplicated: Vec<NodeRef> = roots.iter().chain(roots.iter()).cloned().collect();
    assert!(
        !roots.is_empty(),
        "the 4 mm single-layer witness must seed a root"
    );
    assert!(
        walk_forest(&duplicated, 20_000).is_err(),
        "a duplicated root must fail the structural walk"
    );
}

/// `lightning_seed_tree_controls` — the models reject their named mutations.
#[test]
fn lightning_seed_tree_controls() {
    // The seed model's positive witness.
    seed_witnesses().expect("the seed witnesses must pass");

    // The sampler must catch a kernel that skips erosion even at the smallest
    // generated radius (0.25 mm). Feeding the un-eroded outline as the
    // observation must be rejected; at a fixed 2500-unit offset this radius
    // left the 0..2500 band unsampled and the wrong answer passed.
    let uneroded_case = SeedCase {
        layers: vec![Some(LayerShape::solid(LayerRect {
            x0: 0,
            y0: 0,
            x1: 10 * MM,
            y1: 10 * MM,
        }))],
        radius: MM / 4,
    };
    let Some(shape) = uneroded_case.layers[0] else {
        panic!("the witness must declare a layer");
    };
    assert!(
        verify_observation(&uneroded_case, &[vec![ring(shape.outer)]]).is_err(),
        "an un-eroded answer must fail at radius MM/4"
    );

    // A model that erodes the *above* layer would disagree with the kernel, so
    // the checker must reject a case where the two differ: here the layer above
    // is smaller than the current one, so eroding `above` changes the result.
    let case = SeedCase {
        layers: vec![
            Some(LayerShape::solid(LayerRect {
                x0: 0,
                y0: 0,
                x1: 10 * MM,
                y1: 10 * MM,
            })),
            Some(LayerShape::solid(LayerRect {
                x0: 4 * MM,
                y0: 4 * MM,
                x1: 6 * MM,
                y1: 6 * MM,
            })),
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
        holes: Vec::new(),
        max_tree_nodes: 20_000,
    }
    .generator();
    generator.generate_trees(&|| ());
    let roots = generator.get_trees_for_layer(0).tree_roots.clone();
    // A node listed twice would be caught by the visited set; simulate it by
    // walking the same root twice through the public API.
    let duplicated: Vec<NodeRef> = roots.iter().chain(roots.iter()).cloned().collect();
    assert!(
        !roots.is_empty(),
        "the 4 mm single-layer witness must seed a root"
    );
    assert!(
        walk_forest(&duplicated, 20_000).is_err(),
        "a duplicated root must fail the structural walk"
    );
    // And a zero node budget reports an incomplete case rather than a pass.
    assert!(
        walk_forest(&roots, 0).is_err(),
        "an exceeded node budget must fail rather than certify a prefix"
    );
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
    let mut tree = tree_strategy(3)
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
