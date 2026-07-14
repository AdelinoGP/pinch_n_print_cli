#![allow(missing_docs)]

use slicer_core::arachne::region_order::{
    get_region_order, reorder_by_region_order, topological_walk,
};
use slicer_ir::{ExtrusionJunction, ExtrusionLine, Point3WithWidth};

fn junction(x: f32, y: f32, width: f32) -> ExtrusionJunction {
    ExtrusionJunction {
        p: Point3WithWidth {
            x,
            y,
            z: 0.2,
            width,
            flow_factor: 1.0,
            overhang_quartile: None,
        },
        perimeter_index: 0,
    }
}

fn line(points: &[(f32, f32)], inset_idx: u32, is_odd: bool) -> ExtrusionLine {
    ExtrusionLine {
        junctions: points.iter().map(|&(x, y)| junction(x, y, 0.4)).collect(),
        inset_idx,
        is_odd,
        is_closed: false,
    }
}

#[test]
fn region_order_get_matches_canonical_pair_guards() {
    // The four first points form a complete bipartite neighborhood: the two
    // outer points are 1.0 mm apart, while both inner points are within 0.76
    // mm of both outers. Remaining points are isolated test noise.
    let isolated = |x: f32| {
        [
            (x, 0.0),
            (x + 2.0, 0.0),
            (x + 4.0, 0.0),
            (x + 6.0, 0.0),
            (x + 8.0, 0.0),
            (x + 10.0, 0.0),
            (x + 12.0, 0.0),
        ]
    };
    let mut outer_0 = vec![(0.0, 0.0)];
    outer_0.extend(isolated(100.0));
    let mut outer_1 = vec![(0.0, 1.0)];
    outer_1.extend(isolated(120.0));
    let mut inner_0 = vec![(0.35, 0.45)];
    inner_0.extend(isolated(140.0));
    let mut inner_1 = vec![(0.35, 0.55)];
    inner_1.extend(isolated(160.0));

    let input = vec![
        line(&outer_0, 0, false),
        line(&outer_1, 0, false),
        line(&inner_0, 1, true),
        line(&inner_1, 1, true),
    ];
    let expected = vec![(0, 2), (0, 3), (1, 2), (1, 3)];

    assert_eq!(get_region_order(&input, true), expected);
    assert_eq!(get_region_order(&input, false), expected);
}

#[test]
fn region_order_excludes_same_line_same_inset_and_non_adjacent_insets() {
    let input = vec![
        line(&[(0.0, 0.0), (0.1, 0.0)], 0, false),
        line(&[(0.2, 0.0)], 0, false),
        line(&[(0.3, 0.0)], 2, false),
        line(&[(0.4, 0.0)], 1, true),
    ];

    // Orca rejects same-line, same-inset, and inset gaps greater than one
    // before evaluating the odd/even precedence predicate.
    assert_eq!(get_region_order(&input, true), vec![(0, 3), (1, 3)]);
}

#[test]
fn region_order_deduplicates_constraints_from_multiple_junction_pairs() {
    let input = vec![
        line(&[(0.0, 0.0), (0.1, 0.0)], 0, false),
        line(&[(0.2, 0.0), (0.3, 0.0)], 1, true),
    ];

    assert_eq!(get_region_order(&input, true), vec![(0, 1)]);
}

#[test]
fn region_order_empty_input_returns_empty() {
    assert_eq!(get_region_order(&[], false), Vec::<(usize, usize)>::new());
}

#[test]
fn region_order_single_line_preserved() {
    let input = vec![line(&[(0.0, 0.0)], 0, false)];
    assert_eq!(get_region_order(&input, false), vec![]);
}

#[test]
fn region_order_no_adjacency_falls_back_to_nearest_neighbor() {
    let input = vec![
        line(&[(0.0, 0.0)], 0, false),
        line(&[(100.0, 100.0)], 1, false),
    ];
    assert_eq!(get_region_order(&input, false), vec![]);
}

#[test]
fn region_order_zero_max_line_width_returns_no_constraints() {
    let input = vec![line(&[(0.0, 0.0)], 0, false), line(&[(0.1, 0.1)], 1, true)];
    let input = input
        .into_iter()
        .map(|mut line| {
            line.junctions[0].p.width = 0.0;
            line
        })
        .collect::<Vec<_>>();
    assert_eq!(get_region_order(&input, false), vec![]);
}

#[test]
fn region_order_topological_walk_matches_canonical_open_line_cursor() {
    let input = vec![
        line(&[(0.0, 0.0), (5.0, 0.0)], 0, false),
        line(&[(10.0, 0.0)], 1, false),
        line(&[(20.0, 0.0)], 2, false),
        line(&[(30.0, 0.0)], 3, false),
    ];
    let result = topological_walk(&input, &[(0, 2), (0, 3), (1, 2), (1, 3)]);

    let pos0 = result.iter().position(|&i| i == 0).unwrap();
    let pos1 = result.iter().position(|&i| i == 1).unwrap();
    let pos2 = result.iter().position(|&i| i == 2).unwrap();
    let pos3 = result.iter().position(|&i| i == 3).unwrap();
    assert!(pos0 < pos2);
    assert!(pos1 < pos2);
    assert!(pos0 < pos3);
    assert!(pos1 < pos3);
}

#[test]
fn region_order_topological_walk_with_extra_constraints_4_lines() {
    let input = vec![
        line(&[(0.0, 0.0)], 0, false),
        line(&[(1.0, 0.0)], 1, false),
        line(&[(2.0, 0.0)], 2, false),
        line(&[(3.0, 0.0)], 3, false),
    ];
    let result = topological_walk(&input, &[(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]);

    assert_eq!(result, vec![0, 1, 2, 3]);
}

#[test]
fn reorder_by_region_order_preserves_length_and_permutes_in_place() {
    let mut input = vec![
        line(&[(0.0, 0.0)], 10, false),
        line(&[(100.0, 0.0)], 20, false),
        line(&[(200.0, 0.0)], 30, false),
        line(&[(300.0, 0.0)], 20, false),
    ];
    let expected = input.iter().map(|line| line.inset_idx).collect::<Vec<_>>();

    reorder_by_region_order(&mut input, false);

    let mut actual = input.iter().map(|line| line.inset_idx).collect::<Vec<_>>();
    let mut expected = expected;
    actual.sort_unstable();
    expected.sort_unstable();
    assert_eq!(input.len(), 4);
    assert_eq!(actual, expected);
}
