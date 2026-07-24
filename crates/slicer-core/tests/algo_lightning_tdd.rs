//! TDD coverage for Lightning data structures.

use std::{collections::BTreeSet, rc::Rc};

use slicer_core::algos::lightning::distance_field::DistanceField;
use slicer_core::algos::lightning::generate_initial_internal_overhangs;
use slicer_core::algos::lightning::generator::Generator;
use slicer_core::algos::lightning::layer::Layer;
use slicer_core::algos::lightning::tree_node::Node;
use slicer_core::{difference, offset, OffsetJoinType};
use slicer_ir::{
    mm_to_units, slice_ir::BoundingBox2, units_to_mm, ExPolygon, Point2, Polygon, ResolvedConfig,
    SliceIR, SlicedRegion,
};

fn square(size_mm: f32) -> Vec<Point2> {
    vec![
        Point2::from_mm(0.0, 0.0),
        Point2::from_mm(size_mm, 0.0),
        Point2::from_mm(size_mm, size_mm),
        Point2::from_mm(0.0, size_mm),
        Point2::from_mm(0.0, 0.0),
    ]
}

fn translated_square(size_mm: f32, x_offset_mm: f32) -> Vec<Point2> {
    square(size_mm)
        .into_iter()
        .map(|point| Point2 {
            x: point.x + mm_to_units(x_offset_mm),
            y: point.y,
        })
        .collect()
}

fn expolygon(points: Vec<Point2>) -> ExPolygon {
    ExPolygon {
        contour: Polygon { points },
        holes: Vec::new(),
    }
}

fn flatten_expolygons(polygons: &[ExPolygon]) -> Vec<Point2> {
    let mut points = Vec::new();
    for polygon in polygons {
        for ring in std::iter::once(&polygon.contour).chain(polygon.holes.iter()) {
            points.extend_from_slice(&ring.points);
            if ring.points.last() != ring.points.first() {
                points.push(ring.points[0]);
            }
        }
    }
    points
}

fn square_bbox(size_mm: f32) -> BoundingBox2 {
    BoundingBox2 {
        min: Point2::from_mm(0.0, 0.0),
        max: Point2::from_mm(size_mm, size_mm),
    }
}

// Orca ref: DistanceField::update (OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/DistanceField.cpp)
#[test]
fn lightning_distance_field() {
    let outline = square(4.0);
    let overhang = outline.clone();
    let mut field = DistanceField::new(mm_to_units(6.0), &outline, square_bbox(4.0), &overhang);

    assert_eq!(field.unsupported_count(), 16);
    let next = field.try_get_next_point().expect("the overhang has cells");
    assert!(square_bbox(4.0).contains_point(next));

    field.update(outline[0], Point2::from_mm(2.0, 2.0));
    assert_eq!(field.unsupported_count(), 0);
    assert_eq!(field.try_get_next_point(), None);
}

// Orca ref: DistanceField::update (OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/DistanceField.cpp)
#[test]
fn lightning_distance_field_rectangle_consumes_past_circle() {
    let outline = square(1.0);
    let supporting_radius = mm_to_units(1.0);
    let overhang = outline.clone();
    let to_node = Point2::from_mm(0.0, 0.0);
    let added_leaf = Point2::from_mm(0.0, 1.0);

    let mut circle_only =
        DistanceField::new(supporting_radius, &outline, square_bbox(1.0), &overhang);
    let mut segment_support =
        DistanceField::new(supporting_radius, &outline, square_bbox(1.0), &overhang);
    let cell_size = supporting_radius / 6;
    let rectangle_only_cell = Point2 {
        x: cell_size / 2 + cell_size * 4,
        y: cell_size / 2,
    };
    let dx = rectangle_only_cell.x - added_leaf.x;
    let dy = rectangle_only_cell.y - added_leaf.y;
    assert!(dx * dx + dy * dy > supporting_radius * supporting_radius);

    circle_only.update(added_leaf, added_leaf);
    segment_support.update(to_node, added_leaf);

    assert!(circle_only.unsupported_count() > segment_support.unsupported_count());
}

// Orca ref: DistanceField::update and Node::propagateToNextLayer (OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/DistanceField.cpp; TreeNode.cpp)
#[test]
fn lightning_empty_inputs_no_panic() {
    let mut field = DistanceField::new(mm_to_units(6.0), &[], square_bbox(4.0), &[]);

    assert_eq!(field.unsupported_count(), 0);
    assert_eq!(field.try_get_next_point(), None);
    field.update(Point2::default(), Point2::default());
    assert_eq!(field.try_get_next_point(), None);

    let tree = Node::new(Point2::default());
    assert!(tree.borrow().is_root());
    assert!(tree.borrow().children().is_empty());
    assert_eq!(tree.borrow().prune(5), 0);
    tree.borrow().straighten(3, 0);
    // Canonical `Node::realign` returns false immediately when the next layer's
    // outlines are empty, and `propagateToNextLayer` only emits the copy when
    // realign accepted it. So an empty outline set propagates nothing; the
    // intent under test is that this is a clean no-op rather than a panic.
    let propagated = tree.borrow().propagate_to_next_layer(&[], 4, 0, 3, 0);
    assert!(propagated.is_empty());
}

/// AC-1: the first layer has no predecessor and layer N is outline N minus the dilated outline N-1.
#[test]
fn lightning_generator_overhangs() {
    let layer_zero = square(10.0);
    let layer_one = square(20.0);
    let wall_supporting_radius = mm_to_units(0.5);

    let overhangs = generate_initial_internal_overhangs(
        &[layer_zero.clone(), layer_one.clone()],
        wall_supporting_radius,
    );
    let dilated_previous = offset(
        &[expolygon(layer_zero)],
        units_to_mm(wall_supporting_radius),
        OffsetJoinType::Miter,
        0.0,
    );
    let expected = difference(&[expolygon(layer_one)], &dilated_previous);

    assert_eq!(overhangs.len(), 2);
    assert!(overhangs[0].is_empty());
    assert_eq!(overhangs[1], flatten_expolygons(&expected));
}

// Orca ref: DistanceField::update and Node::straighten (OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/DistanceField.cpp; TreeNode.cpp)
#[test]
fn lightning_primitives_are_deterministic() {
    let outline = square(2.0);
    let overhang = outline.clone();
    let mut field_one = DistanceField::new(mm_to_units(6.0), &outline, square_bbox(2.0), &overhang);
    let mut field_two = DistanceField::new(mm_to_units(6.0), &outline, square_bbox(2.0), &overhang);

    assert_eq!(field_one.unsupported_count(), 4);
    assert_eq!(field_one.unsupported_count(), field_two.unsupported_count());
    assert_eq!(
        field_one.try_get_next_point(),
        field_two.try_get_next_point()
    );

    let tree_one = Node::new(Point2 { x: 0, y: 0 });
    let middle_one = tree_one.borrow().add_child(Point2 { x: 5, y: 5 });
    middle_one.borrow().add_child(Point2 { x: 10, y: 0 });
    let tree_two = Node::new(Point2 { x: 0, y: 0 });
    let middle_two = tree_two.borrow().add_child(Point2 { x: 5, y: 5 });
    middle_two.borrow().add_child(Point2 { x: 10, y: 0 });

    let max_pruned_one = tree_one.borrow().prune(5);
    let max_pruned_two = tree_two.borrow().prune(5);
    assert_eq!(max_pruned_one, 5);
    assert_eq!(max_pruned_one, max_pruned_two);

    tree_one.borrow().straighten(3, 0);
    tree_two.borrow().straighten(3, 0);
    assert_eq!(
        middle_one.borrow().location(),
        middle_two.borrow().location()
    );

    field_one.update(Point2::default(), Point2::default());
    field_two.update(Point2::default(), Point2::default());
    assert_eq!(field_one.unsupported_count(), field_two.unsupported_count());
}

// Orca ref: Node::prune (OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/TreeNode.cpp)
#[test]
fn lightning_tree_node_prune() {
    let root = Node::new(Point2 { x: 0, y: 0 });
    root.borrow().add_child(Point2 { x: 3, y: 0 });
    root.borrow().add_child(Point2 { x: 10, y: 0 });

    assert_eq!(root.borrow().prune(5), 5);
    let children = root.borrow().children();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].borrow().location(), Point2 { x: 5, y: 0 });
}

// Orca ref: Node::straighten (OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/TreeNode.cpp)
#[test]
fn lightning_tree_node_straighten() {
    let root = Node::new(Point2 { x: 0, y: 0 });
    let middle = root.borrow().add_child(Point2 { x: 5, y: 5 });
    let leaf = middle.borrow().add_child(Point2 { x: 10, y: 0 });
    let original_middle = middle.borrow().location();

    root.borrow().straighten(3, 0);

    let new_middle = middle.borrow().location();
    let moved_x = i128::from(new_middle.x - original_middle.x);
    let moved_y = i128::from(new_middle.y - original_middle.y);
    assert!(moved_x * moved_x + moved_y * moved_y <= 9);
    assert_eq!(root.borrow().location(), Point2 { x: 0, y: 0 });
    assert_eq!(leaf.borrow().location(), Point2 { x: 10, y: 0 });

    let old_path = (50.0_f64).sqrt() * 2.0;
    let new_path = {
        let root_location = root.borrow().location();
        let leaf_location = leaf.borrow().location();
        let middle_location = middle.borrow().location();
        distance(root_location, middle_location) + distance(middle_location, leaf_location)
    };
    assert!(new_path < old_path);
}

// Orca ref: Node::straightenRecursive (OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/TreeNode.cpp)
#[test]
fn lightning_tree_node_straighten_does_not_remove_off_axis_node() {
    let root = Node::new(Point2 { x: 0, y: 0 });
    let middle = root.borrow().add_child(Point2 { x: 5, y: 5 });
    let leaf = middle.borrow().add_child(Point2 { x: 10, y: 0 });
    let original_middle = middle.borrow().location();

    root.borrow().straighten(3, 100);

    let root_children = root.borrow().children();
    assert_eq!(root_children.len(), 1);
    assert_eq!(
        root_children[0].borrow().location(),
        middle.borrow().location()
    );
    assert_eq!(middle.borrow().children().len(), 1);
    assert_eq!(
        middle.borrow().children()[0].borrow().location(),
        leaf.borrow().location()
    );

    let new_middle = middle.borrow().location();
    let moved_x = i128::from(new_middle.x - original_middle.x);
    let moved_y = i128::from(new_middle.y - original_middle.y);
    assert!(moved_x * moved_x + moved_y * moved_y <= 9);
}

// Orca ref: Node::propagateToNextLayer (OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/TreeNode.cpp)
#[test]
fn lightning_tree_node_propagate() {
    let root = Node::new(Point2 { x: 0, y: 0 });
    let middle = root.borrow().add_child(Point2 { x: 5, y: 5 });
    middle.borrow().add_child(Point2 { x: 10, y: 0 });
    let original_middle = middle.borrow().location();

    // The next layer's outline must actually contain the tree: canonical
    // `Node::realign` drops any node that falls outside it. This box clears
    // every node by a wide margin, so realign keeps the tree intact and the
    // prune/straighten/structure behaviour is what the assertions below see.
    let next_outline = vec![
        Point2 { x: -1000, y: -1000 },
        Point2 { x: 1000, y: -1000 },
        Point2 { x: 1000, y: 1000 },
        Point2 { x: -1000, y: 1000 },
        Point2 { x: -1000, y: -1000 },
    ];
    let mut propagated_roots = root
        .borrow()
        .propagate_to_next_layer(&next_outline, 4, 0, 4, 0);
    assert_eq!(propagated_roots.len(), 1);
    let propagated = propagated_roots.pop().expect("the unpruned tree propagates");
    let propagated_children = propagated.borrow().children();
    let propagated_middle = propagated_children[0].clone();
    let propagated_leaf = propagated_middle.borrow().children()[0].clone();
    let new_middle = propagated_middle.borrow().location();
    let moved_x = i128::from(new_middle.x - original_middle.x);
    let moved_y = i128::from(new_middle.y - original_middle.y);

    assert!(propagated.borrow().is_root());
    assert_eq!(propagated_children.len(), 1);
    assert_eq!(propagated_middle.borrow().children().len(), 1);
    assert!(moved_x * moved_x + moved_y * moved_y <= 16);
    assert_eq!(propagated.borrow().location(), Point2 { x: 0, y: 0 });
    assert_eq!(propagated_leaf.borrow().location(), Point2 { x: 10, y: 0 });
}

// Orca ref: Node::realign (OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/TreeNode.cpp)
//
// The "outside" branch: a node that falls outside the next layer's outline dies,
// but each surviving descendant is lifted out as an independent root and takes
// the *dead parent's* position as its grounding location.
#[test]
fn lightning_tree_node_propagate_reroots_descendants_of_an_outside_node() {
    let outline = vec![
        Point2 { x: 0, y: 0 },
        Point2 { x: 1000, y: 0 },
        Point2 { x: 1000, y: 1000 },
        Point2 { x: 0, y: 1000 },
        Point2 { x: 0, y: 0 },
    ];
    let dead_root_location = Point2 { x: 2000, y: 2000 };
    let root = Node::new(dead_root_location);
    root.borrow().add_child(Point2 { x: 500, y: 500 });

    // Locator resolution 0 disables the boundary-crossing test, isolating the
    // inside/outside decision.
    let propagated = root.borrow().propagate_to_next_layer(&outline, 0, 0, 0, 0);

    assert_eq!(propagated.len(), 1, "the outside root itself must not survive");
    let rerooted = &propagated[0];
    assert_eq!(rerooted.borrow().location(), Point2 { x: 500, y: 500 });
    assert!(rerooted.borrow().is_root());
    assert_eq!(
        rerooted.borrow().get_last_grounding_location(),
        Some(dead_root_location)
    );
}

// Orca ref: Node::realign and lineSegmentPolygonsIntersection
// (OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/TreeNode.cpp)
//
// The "inside" branch: both nodes survive, but the edge between them leaves the
// outline, so the child is detached into its own root and both ends have their
// grounding location cleared.
#[test]
fn lightning_tree_node_propagate_detaches_a_child_across_a_notch() {
    // A "U": the notch spans x in [400, 600] above y = 200.
    let outline = vec![
        Point2 { x: 0, y: 0 },
        Point2 { x: 1000, y: 0 },
        Point2 { x: 1000, y: 1000 },
        Point2 { x: 600, y: 1000 },
        Point2 { x: 600, y: 200 },
        Point2 { x: 400, y: 200 },
        Point2 { x: 400, y: 1000 },
        Point2 { x: 0, y: 1000 },
        Point2 { x: 0, y: 0 },
    ];
    let root_location = Point2 { x: 200, y: 600 };
    let child_location = Point2 { x: 800, y: 600 };
    let root = Node::new_with_grounding_location(root_location, Some(Point2 { x: 0, y: 600 }));
    root.borrow().add_child(child_location);

    // Resolution 250 => crossing tolerance 500, which covers the nearest
    // notch wall at x = 400 (200 units from the parent).
    let propagated = root.borrow().propagate_to_next_layer(&outline, 250, 0, 0, 0);

    assert_eq!(propagated.len(), 2);
    // Canonical appends rerooted parts before the main copy.
    let detached = &propagated[0];
    let kept = &propagated[1];
    assert_eq!(detached.borrow().location(), child_location);
    assert!(detached.borrow().is_root());
    assert_eq!(detached.borrow().get_last_grounding_location(), None);
    assert_eq!(kept.borrow().location(), root_location);
    assert!(kept.borrow().children().is_empty());
    assert_eq!(
        kept.borrow().get_last_grounding_location(),
        None,
        "canonical clears the grounding location of a node that had to reground"
    );
}

// Orca ref: Node::reroot (OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/TreeNode.cpp)
#[test]
fn lightning_tree_node_reroot_with_new_parent() {
    let root_a = Node::new(Point2 { x: 0, y: 0 });
    let node_b = root_a.borrow().add_child(Point2 { x: 5, y: 0 });
    let root_x = Node::new(Point2 { x: 10, y: 0 });

    node_b.borrow().reroot(Some(Rc::clone(&root_x)));

    let a_children = root_a.borrow().children();
    assert!(a_children.iter().all(|child| !Rc::ptr_eq(child, &node_b)));
    let x_children = root_x.borrow().children();
    assert!(x_children.iter().any(|child| Rc::ptr_eq(child, &node_b)));
    assert!(!node_b.borrow().is_root());
}

// Orca ref: Node constructor and getLastGroundingLocation (TreeNode.cpp)
#[test]
fn lightning_tree_node_grounding_location() {
    let grounding = Point2 { x: 10, y: 20 };
    let node = Node::new_with_grounding_location(Point2 { x: 30, y: 40 }, Some(grounding));

    assert_eq!(node.borrow().get_last_grounding_location(), Some(grounding));
    node.borrow_mut().set_last_grounding_location(None);
    assert_eq!(node.borrow().get_last_grounding_location(), None);
}

// Orca ref: Node::hasOffspring (TreeNode.cpp)
#[test]
fn lightning_tree_node_has_offspring() {
    let root = Node::new(Point2 { x: 0, y: 0 });
    let child = root.borrow().add_child(Point2 { x: 10, y: 0 });
    let grandchild = child.borrow().add_child(Point2 { x: 20, y: 0 });
    let unrelated = Node::new(Point2 { x: 30, y: 0 });

    assert!(root.borrow().has_offspring(Rc::clone(&root)));
    assert!(root.borrow().has_offspring(Rc::clone(&grandchild)));
    assert!(child.borrow().has_offspring(Rc::clone(&grandchild)));
    assert!(!child.borrow().has_offspring(unrelated));
}

// Orca ref: Node::closestNode (TreeNode.cpp)
#[test]
fn lightning_tree_node_closest_node() {
    let root = Node::new(Point2 { x: 0, y: 0 });
    let child = root.borrow().add_child(Point2 { x: 100, y: 0 });
    let grandchild = child.borrow().add_child(Point2 { x: 200, y: 0 });
    root.borrow().add_child(Point2 { x: 500, y: 0 });

    let closest = root.borrow().closest_node(Point2 { x: 190, y: 0 });
    assert!(Rc::ptr_eq(&closest, &grandchild));
}

// Orca ref: Node::getWeightedDistance (TreeNode.cpp)
#[test]
fn lightning_tree_node_weighted_distance() {
    let root = Node::new(Point2 { x: 0, y: 0 });
    let branch = root.borrow().add_child(Point2 { x: 0, y: 10 });
    for x in [1, 2, 3, 4] {
        branch.borrow().add_child(Point2 { x, y: 10 });
    }
    let supporting_radius = 10;

    assert_eq!(
        root.borrow()
            .get_weighted_distance(Point2 { x: 100, y: 0 }, supporting_radius),
        60
    );
    assert_eq!(
        branch
            .borrow()
            .get_weighted_distance(Point2 { x: 100, y: 10 }, supporting_radius),
        100
    );
}

// Orca ref: Node::convertToPolylines and removeJunctionOverlap (TreeNode.cpp)
#[test]
fn lightning_tree_node_convert_to_polylines() {
    let root = Node::new(Point2 { x: 0, y: 0 });
    root.borrow().add_child(Point2 { x: 100, y: 0 });
    let mut lines = Vec::new();

    root.borrow().convert_to_polylines(&mut lines, 10);

    assert_eq!(
        lines,
        vec![vec![Point2 { x: 100, y: 0 }, Point2 { x: 10, y: 0 }]]
    );
}

// Orca ref: Node::visitNodes (TreeNode.cpp)
#[test]
fn lightning_tree_node_visit_nodes() {
    let root = Node::new(Point2 { x: 0, y: 0 });
    let child = root.borrow().add_child(Point2 { x: 10, y: 0 });
    child.borrow().add_child(Point2 { x: 20, y: 0 });
    root.borrow().add_child(Point2 { x: 30, y: 0 });
    let mut visited = Vec::new();

    root.borrow()
        .visit_nodes(|node| visited.push(node.borrow().location()));

    assert_eq!(
        visited,
        vec![
            Point2 { x: 0, y: 0 },
            Point2 { x: 10, y: 0 },
            Point2 { x: 20, y: 0 },
            Point2 { x: 30, y: 0 },
        ]
    );
}

#[test]
fn lightning_layer_seed_inside_overhang() {
    let outline = square(20.0);
    let overhang = square(10.0);
    let mut layer = Layer::new(Vec::new());

    layer.generate_new_trees(
        &overhang,
        &outline,
        mm_to_units(6.0),
        mm_to_units(0.2),
        &|| {},
    );

    assert!(!layer.tree_roots.is_empty());
    assert!(layer.tree_roots[0]
        .borrow()
        .get_last_grounding_location()
        .is_some());
    let mut node_count = 0;
    for root in &layer.tree_roots {
        root.borrow().visit_nodes(|_| node_count += 1);
    }
    assert!(node_count > layer.tree_roots.len());
}

#[test]
fn lightning_layer_wall_supporting_radius() {
    let outline = vec![
        Point2 { x: 0, y: 0 },
        Point2 { x: 100, y: 0 },
        Point2 { x: 100, y: 100 },
        Point2 { x: 0, y: 100 },
        Point2 { x: 0, y: 0 },
    ];
    let tree = Node::new(Point2 { x: 5, y: 50 });
    let mut layer = Layer::new(vec![Rc::clone(&tree)]);
    let overhang = vec![
        Point2 { x: 5, y: 49 },
        Point2 { x: 6, y: 49 },
        Point2 { x: 6, y: 50 },
        Point2 { x: 5, y: 50 },
        Point2 { x: 5, y: 49 },
    ];

    layer.generate_new_trees(&overhang, &outline, 6, 1000, &|| {});

    assert_eq!(layer.tree_roots.len(), 1);
    assert!(tree.borrow().children().is_empty());
}

#[test]
fn lightning_layer_reconnect_to_outline() {
    let outline = square(20.0);
    let root = Node::new(Point2::from_mm(10.0, 10.0));
    let mut layer = Layer::new(vec![Rc::clone(&root)]);

    layer.reconnect_roots(
        vec![Rc::clone(&root)],
        &outline,
        mm_to_units(6.0),
        mm_to_units(0.2),
    );

    assert_eq!(layer.tree_roots.len(), 1);
    assert_ne!(
        layer.tree_roots[0].borrow().location(),
        root.borrow().location()
    );
    assert!(layer.tree_roots[0]
        .borrow()
        .children()
        .iter()
        .any(|child| Rc::ptr_eq(child, &root)));
}

#[test]
fn lightning_layer_convert_to_lines() {
    let outline = square(20.0);
    let root = Node::new(Point2::from_mm(10.0, 10.0));
    root.borrow().add_child(Point2::from_mm(12.0, 10.0));
    let layer = Layer::new(vec![root]);

    let lines = layer.convert_to_lines(&outline, 0);

    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].len(), 2);
}

fn lightning_prism_outlines() -> Vec<Vec<Point2>> {
    vec![square(10.0), square(10.0), square(10.0), square(12.0)]
}

fn lightning_generator(outlines: Vec<Vec<Point2>>) -> Generator {
    Generator::new(
        outlines,
        0.2,
        mm_to_units(0.4),
        1,
        mm_to_units(0.2),
        45.0,
        5.0,
        0.0,
    )
}

fn tree_endpoints(
    root: &Rc<std::cell::RefCell<slicer_core::algos::lightning::tree_node::Node>>,
) -> Vec<Point2> {
    let (location, is_root, children) = {
        let node = root.borrow();
        (node.location(), node.is_root(), node.children())
    };
    let mut endpoints = Vec::new();
    if is_root || children.is_empty() {
        endpoints.push(location);
    }
    for child in children {
        endpoints.extend(tree_endpoints(&child));
    }
    endpoints
}

fn tree_nodes(
    root: &Rc<std::cell::RefCell<slicer_core::algos::lightning::tree_node::Node>>,
) -> Vec<Point2> {
    let mut nodes = Vec::new();
    root.borrow()
        .visit_nodes(|node| nodes.push(node.borrow().location()));
    nodes
}

fn distance_to_outline(point: Point2, outline: &[Point2]) -> f64 {
    outline
        .iter()
        .copied()
        .zip(outline.iter().copied().cycle().skip(1))
        .take(outline.len())
        .map(|(start, end)| {
            let dx = (end.x - start.x) as f64;
            let dy = (end.y - start.y) as f64;
            let length_squared = dx * dx + dy * dy;
            let t = if length_squared == 0.0 {
                0.0
            } else {
                (((point.x - start.x) as f64 * dx + (point.y - start.y) as f64 * dy)
                    / length_squared)
                    .clamp(0.0, 1.0)
            };
            let closest_x = start.x as f64 + t * dx;
            let closest_y = start.y as f64 + t * dy;
            ((point.x as f64 - closest_x).powi(2) + (point.y as f64 - closest_y).powi(2)).sqrt()
        })
        .fold(f64::INFINITY, f64::min)
}

// Orca ref: Generator::generateTrees and Node::realign
// (OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Generator.cpp; TreeNode.cpp)
//
// Structure of the `[10, 10, 10, 12]` prism fixture, once `Node::realign` is
// ported (before the port, `propagate_to_next_layer` discarded `next_outlines`
// entirely and every tree was copied down unconditionally):
//
//  * `generate_initial_internal_overhangs` gives layer N the area of layer N
//    minus the *dilated outline of layer N-1*, so layers 0..=2 — three
//    identical 10 mm squares — have an empty overhang and seed nothing.
//  * Layer 3 (12 mm) has an overhang band that begins outside the 10 mm square
//    dilated by `wall_supporting_radius`, so every node layer 3 seeds sits
//    outside layer 2's outline.
//  * Propagating layer 3 downward therefore takes canonical `realign`'s
//    "outside" branch at the root and at every descendant, the whole tree is
//    dropped, and layers 0..=2 stay empty. The trees that the pre-realign port
//    left on those layers were floating outside their own layer's outline.
//
// The cross-layer continuity that this fixture used to claim is consequently
// vacuous for it; what the fixture still pins precisely is grounding and
// containment, both of which the pre-realign port violated.
#[test]
fn lightning_generator_tree_continuity() {
    let outlines = lightning_prism_outlines();
    let top_layer = outlines.len() - 1;
    let mut generator = lightning_generator(outlines.clone());
    generator.generate_trees(&|| {});

    let layers_with_trees: Vec<usize> = (0..outlines.len())
        .filter(|layer_id| {
            !generator
                .get_trees_for_layer(*layer_id)
                .tree_roots
                .is_empty()
        })
        .collect();
    assert_eq!(
        layers_with_trees,
        vec![top_layer],
        "only the layer whose outline exceeds the layer below has an overhang, \
         and realign keeps its tree from propagating into the smaller layer"
    );

    let below_outline_max = mm_to_units(10.0);
    let own_outline_max = mm_to_units(12.0);
    for root in &generator.get_trees_for_layer(top_layer).tree_roots {
        assert!(root.borrow().is_root());
        // Every tree is grounded on its own layer's wall: `attach` creates the
        // root at the closest point of the current outline.
        assert!(
            distance_to_outline(root.borrow().location(), &outlines[top_layer]) < 1.0,
            "root {:?} is not grounded on its own outline",
            root.borrow().location()
        );

        for node in tree_nodes(root) {
            // Containment: realign guarantees no node survives outside the
            // outline of the layer that owns it.
            assert!(
                node.x >= 0 && node.x <= own_outline_max,
                "node {node:?} escapes its own layer outline"
            );
            assert!(
                node.y >= 0 && node.y <= own_outline_max,
                "node {node:?} escapes its own layer outline"
            );
            // ...and every node is outside the layer below, which is exactly
            // why nothing propagates down out of this fixture.
            assert!(
                node.x > below_outline_max || node.y > below_outline_max,
                "node {node:?} is inside the layer below, so realign should have \
                 propagated it downward"
            );
        }

        // A grounded root always carries at least one overhang node.
        assert!(!tree_endpoints(root).is_empty());
    }
}

#[test]
fn lightning_generator_deterministic() {
    let first_region_outlines = lightning_prism_outlines();
    let second_region_outlines = vec![
        translated_square(10.0, 30.0),
        translated_square(10.0, 30.0),
        translated_square(10.0, 30.0),
        translated_square(12.0, 30.0),
    ];
    let slices: Vec<SliceIR> = first_region_outlines
        .into_iter()
        .zip(second_region_outlines)
        .enumerate()
        .map(
            |(global_layer_index, (first_outline, second_outline))| SliceIR {
                global_layer_index: global_layer_index as u32,
                regions: vec![
                    SlicedRegion {
                        object_id: String::from("cube"),
                        region_id: 1,
                        polygons: vec![expolygon(first_outline)],
                        ..SlicedRegion::default()
                    },
                    SlicedRegion {
                        object_id: String::from("cube"),
                        region_id: 2,
                        polygons: vec![expolygon(second_outline)],
                        ..SlicedRegion::default()
                    },
                ],
                ..SliceIR::default()
            },
        )
        .collect();
    let config = ResolvedConfig {
        sparse_fill_holder: String::from("lightning-infill"),
        ..ResolvedConfig::default()
    };
    let first_ir = slicer_core::algos::lightning::generate_lightning_trees(&slices, &config)
        .expect("first lightning IR must generate");
    let second_ir = slicer_core::algos::lightning::generate_lightning_trees(&slices, &config)
        .expect("second lightning IR must generate");
    let first_bytes = serde_json::to_vec(&*first_ir).expect("first lightning IR must serialize");
    let second_bytes = serde_json::to_vec(&*second_ir).expect("second lightning IR must serialize");
    assert_eq!(first_bytes, second_bytes);

    let region_ids = first_ir
        .entries
        .iter()
        .map(|entry| entry.region_id)
        .collect::<BTreeSet<_>>();
    assert_eq!(region_ids, BTreeSet::from([1, 2]));

    let segments_for_region = |ir: &slicer_ir::LightningTreeIR, region_id| {
        ir.entries
            .iter()
            .filter(|entry| entry.region_id == region_id)
            .flat_map(|entry| entry.tree_edge_segments.iter().copied())
            .collect::<Vec<_>>()
    };
    let first_region_one = segments_for_region(first_ir.as_ref(), 1);
    let first_region_two = segments_for_region(first_ir.as_ref(), 2);
    let second_region_one = segments_for_region(second_ir.as_ref(), 1);
    let second_region_two = segments_for_region(second_ir.as_ref(), 2);
    assert!(!first_region_one.is_empty());
    assert!(!first_region_two.is_empty());
    assert_eq!(first_region_one, second_region_one);
    assert_eq!(first_region_two, second_region_two);
    assert_ne!(first_region_one, first_region_two);
    assert!(first_region_one
        .iter()
        .flatten()
        .all(|point| point.x < mm_to_units(20.0)));
    assert!(first_region_two
        .iter()
        .flatten()
        .all(|point| point.x > mm_to_units(20.0)));
}

#[test]
fn lightning_generator_no_overhang_no_trees() {
    const LAYER_COUNT: usize = 5;
    let outlines = (0..LAYER_COUNT).map(|_| square(10.0)).collect::<Vec<_>>();
    let slices: Vec<SliceIR> = outlines
        .into_iter()
        .enumerate()
        .map(|(global_layer_index, outline)| SliceIR {
            global_layer_index: global_layer_index as u32,
            regions: vec![SlicedRegion {
                object_id: String::from("cube"),
                region_id: 1,
                polygons: vec![expolygon(outline)],
                ..SlicedRegion::default()
            }],
            ..SliceIR::default()
        })
        .collect();
    let config = ResolvedConfig {
        sparse_fill_holder: String::from("lightning-infill"),
        ..ResolvedConfig::default()
    };
    let ir = slicer_core::algos::lightning::generate_lightning_trees(&slices, &config)
        .expect("uniform prism lightning IR must generate");
    assert_eq!(
        ir.schema_version,
        slicer_ir::CURRENT_LIGHTNING_TREE_IR_SCHEMA_VERSION
    );
    assert!(
        ir.entries.is_empty(),
        "uniform prism with no internal overhangs must not produce spurious trees"
    );
}

fn distance(first: Point2, second: Point2) -> f64 {
    let dx = (first.x - second.x) as f64;
    let dy = (first.y - second.y) as f64;
    dx.hypot(dy)
}
