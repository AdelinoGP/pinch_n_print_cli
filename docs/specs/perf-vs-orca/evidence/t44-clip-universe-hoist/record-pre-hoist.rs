//! Pre-hoist recorder; output is a pinned regression reference.
use clipper2_rust::{inflate_paths_64, EndType, JoinType, Point64};
use slicer_core::polygon_ops::clip_polylines;
use slicer_ir::{ExPolygon, Point2, Polygon};

fn ring(points: &[(i64, i64)]) -> Polygon {
    Polygon {
        points: points.iter().map(|&(x, y)| Point2 { x, y }).collect(),
    }
}

fn main() {
    let outer = ring(&[(0, 0), (100000, 0), (100000, 100000), (0, 100000)]);
    let hole = ring(&[
        (40000, 40000),
        (40000, 60000),
        (60000, 60000),
        (60000, 40000),
    ]);
    let square = ExPolygon {
        contour: outer.clone(),
        holes: vec![],
    };
    let annulus = ExPolygon {
        contour: outer,
        holes: vec![hole],
    };
    let island = ExPolygon {
        contour: ring(&[
            (45000, 45000),
            (55000, 45000),
            (55000, 55000),
            (45000, 55000),
        ]),
        holes: vec![],
    };
    let concave = ExPolygon {
        contour: ring(&[
            (0, 0),
            (100000, 0),
            (100000, 100000),
            (60000, 100000),
            (60000, 40000),
            (40000, 40000),
            (40000, 100000),
            (0, 100000),
        ]),
        holes: vec![],
    };
    let degenerates = ExPolygon {
        contour: ring(&[(10, 10), (10, 10)]),
        holes: vec![],
    };
    let mut dense = square.clone();
    dense.contour.points = (0..256)
        .map(|i| {
            let a = i as f64 * std::f64::consts::TAU / 256.0;
            let radius = if i % 2 == 0 { 50000.0 } else { 48000.0 };
            Point2 {
                x: 50000 + (radius * a.cos()).round() as i64,
                y: 50000 + (radius * a.sin()).round() as i64,
            }
        })
        .collect();
    let universes = vec![
        ("square", vec![square.clone()]),
        ("hole_and_island", vec![annulus, island]),
        ("concave_and_degenerate", vec![concave, degenerates]),
        ("dense_ring", vec![dense]),
        ("empty", vec![]),
    ];
    let lines: Vec<Vec<Point2>> = vec![
        vec![(20000, 20000), (50000, 30000), (80000, 80000)],
        vec![(50000, 50000), (150000, 50000)],
        vec![(20000, 50000), (40000, 150000), (60000, 50000)],
        vec![(-10000, 50000), (110000, 50000)],
        vec![(20000, 0), (80000, 0)],
        vec![(100000, 20000), (100000, 80000)],
        vec![(20000, 100000), (80000, 100000)],
        vec![(0, 20000), (0, 80000)],
        vec![(40000, 40000), (40000, 60000)],
        vec![(200000, 200000), (300000, 300000)],
        vec![],
        vec![(50000, 50000)],
    ]
    .into_iter()
    .map(|points| points.into_iter().map(|(x, y)| Point2 { x, y }).collect())
    .collect();
    let mut cases = Vec::new();
    for (name, clip) in universes {
        // Same flatten order and inflate parameters as pre-hoist clip_polylines.
        let raw: Vec<Vec<Point64>> = clip
            .iter()
            .flat_map(|poly| {
                std::iter::once(&poly.contour)
                    .chain(poly.holes.iter())
                    .map(|ring| {
                        ring.points
                            .iter()
                            .map(|p| Point64 { x: p.x, y: p.y })
                            .collect()
                    })
            })
            .collect();
        let inflated = inflate_paths_64(&raw, 1.0, JoinType::Miter, EndType::Polygon, 2.0, 0.0);
        let coords = |paths: &[Vec<Point64>]| {
            paths
                .iter()
                .map(|path| path.iter().map(|p| [p.x, p.y]).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        };
        let mut inputs: Vec<Vec<Vec<Point2>>> =
            lines.iter().map(|line| vec![line.clone()]).collect();
        inputs.push(lines.clone());
        inputs.push(vec![]);
        let calls: Vec<_> = inputs
            .into_iter()
            .map(|input| {
                let expected = clip_polylines(&input, &clip);
                serde_json::json!({ "input": input, "expected": expected })
            })
            .collect();
        cases.push(serde_json::json!({ "name": name, "clip": clip, "raw_paths": coords(&raw), "inflated_paths": coords(&inflated), "calls": calls }));
    }
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "provenance": "Captured before the ticket-44 hoist from clip_polylines and its exact flatten/inflate parameters; integer units are 100 nm. This is a representation pin, not a canonical geometry oracle.",
        "cases": cases
    })).unwrap());
}
