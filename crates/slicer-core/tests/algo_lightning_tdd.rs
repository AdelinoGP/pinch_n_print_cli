//! TDD coverage for Lightning data structures.

use std::rc::Rc;

use slicer_core::algos::lightning::distance_field::DistanceField;
use slicer_core::algos::lightning::tree_node::Node;
use slicer_ir::{mm_to_units, slice_ir::BoundingBox2, Point2};

fn square(size_mm: f32) -> Vec<Point2> {
    vec![
        Point2::from_mm(0.0, 0.0),
        Point2::from_mm(size_mm, 0.0),
        Point2::from_mm(size_mm, size_mm),
        Point2::from_mm(0.0, size_mm),
        Point2::from_mm(0.0, 0.0),
    ]
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
    let propagated = tree
        .borrow()
        .propagate_to_next_layer(&[], 4, 0, 3, 0)
        .expect("an empty tree remains a root node");
    assert_eq!(propagated.borrow().location(), Point2::default());
    assert!(propagated.borrow().children().is_empty());
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

    let propagated = root
        .borrow()
        .propagate_to_next_layer(&[], 4, 0, 4, 0)
        .expect("the unpruned tree propagates");
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

fn distance(first: Point2, second: Point2) -> f64 {
    let dx = (first.x - second.x) as f64;
    let dy = (first.y - second.y) as f64;
    dx.hypot(dy)
}
