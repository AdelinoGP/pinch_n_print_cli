//! Native replay of ordered integer WASM captures; no optimized candidate.
use super::*;

fn number(tokens: &mut std::str::SplitWhitespace<'_>) -> i64 {
    tokens
        .next()
        .expect("truncated capture")
        .parse()
        .expect("integer coordinate")
}

fn polygons(tokens: &mut std::str::SplitWhitespace<'_>) -> Vec<ExPolygon> {
    (0..number(tokens))
        .map(|_| {
            let rings: Vec<Polygon> = (0..number(tokens))
                .map(|_| {
                    let points = (0..number(tokens))
                        .map(|_| slicer_ir::Point2 {
                            x: number(tokens),
                            y: number(tokens),
                        })
                        .collect();
                    Polygon { points }
                })
                .collect();
            let mut rings = rings.into_iter();
            ExPolygon {
                contour: rings.next().expect("contour"),
                holes: rings.collect(),
            }
        })
        .collect()
}

#[test]
fn t53_current_chain_native_replays_exact_ordered_wasm_intermediates() {
    let mut tokens = include_str!("replay.txt").split_whitespace();
    let count = number(&mut tokens);
    assert!(count > 0, "real captures required");
    let mut multi = 0;
    for index in 0..count {
        let tolerance: f64 = tokens.next().unwrap().parse().unwrap();
        let input = polygons(&mut tokens);
        let collision = polygons(&mut tokens);
        let expected_union = polygons(&mut tokens);
        let expected_simplify = polygons(&mut tokens);
        let expected_final = polygons(&mut tokens);
        multi += usize::from(input.len() > 1);
        let actual_union = union_expolys(input);
        assert_eq!(actual_union, expected_union, "union capture {index}");
        let simplified = if tolerance < 0.0 {
            actual_union
        } else {
            expolygons_simplify(&actual_union, tolerance)
        };
        assert_eq!(simplified, expected_simplify, "simplify capture {index}");
        let final_regions = if collision.is_empty() {
            simplified
        } else {
            host::clip_polygons(&simplified, &collision, ClipOperation::Difference)
        };
        assert_eq!(
            final_regions, expected_final,
            "final difference capture {index}"
        );
    }
    assert!(multi > 0, "multi-region host union must execute");
    assert!(
        tokens.next().is_none(),
        "capture framing consumed completely"
    );
}
