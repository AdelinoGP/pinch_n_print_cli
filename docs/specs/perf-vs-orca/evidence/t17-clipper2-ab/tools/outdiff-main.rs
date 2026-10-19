//! Wayfinder ticket 17 output-diff harness (scratch, gitignored).
//!
//! Runs a fixed deterministic corpus through `slicer-core::polygon_ops` — the
//! exact clipper2 entry points the production slice path uses — and dumps each
//! result as canonical JSON. Run once per clipper2-rust version (from two trees
//! that differ only in the pinned dependency); byte-compare the dumps.
//!
//! Usage: t17-outdiff --out <dir> --stl <path/to/wedge.stl>

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use slicer_core::polygon_ops::{
    clip_polylines, closing_ex, difference, intersection, offset, offset2_ex, opening, union,
    union_ex, validate_polygon_simplicity, xor, OffsetJoinType,
};
use slicer_ir::{ExPolygon, Point2, Polygon};

/// CCW square centred at (cx, cy), side `side_units` (1 unit = 100 nm).
fn square(cx: i64, cy: i64, side_units: i64) -> ExPolygon {
    let h = side_units / 2;
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 {
                    x: cx - h,
                    y: cy - h,
                },
                Point2 {
                    x: cx + h,
                    y: cy - h,
                },
                Point2 {
                    x: cx + h,
                    y: cy + h,
                },
                Point2 {
                    x: cx - h,
                    y: cy + h,
                },
            ],
        },
        holes: Vec::new(),
    }
}

/// The bench's own fixture: NxN grid of 12 mm squares at 10 mm pitch.
fn square_grid(n: i64) -> Vec<ExPolygon> {
    let side = 120_000;
    let pitch = 100_000;
    let mut out = Vec::with_capacity((n * n) as usize);
    for ix in 0..n {
        for iy in 0..n {
            out.push(square(ix * pitch, iy * pitch, side));
        }
    }
    out
}

/// 2 mm outer square with a 0.8 mm square hole (from the polygon_ops tests).
fn annulus() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: 0, y: 0 },
                Point2 { x: 20000, y: 0 },
                Point2 { x: 20000, y: 20000 },
                Point2 { x: 0, y: 20000 },
            ],
        },
        holes: vec![Polygon {
            points: vec![
                Point2 { x: 6000, y: 6000 },
                Point2 { x: 6000, y: 14000 },
                Point2 { x: 14000, y: 14000 },
                Point2 { x: 14000, y: 6000 },
            ],
        }],
    }
}

/// Self-intersecting bowtie (the simplicity validator's own negative fixture).
fn bowtie() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: 0, y: 0 },
                Point2 { x: 10, y: 10 },
                Point2 { x: 10, y: 0 },
                Point2 { x: 0, y: 10 },
            ],
        },
        holes: Vec::new(),
    }
}

struct Dumper {
    dir: PathBuf,
    counts: BTreeMap<String, (usize, usize, usize)>,
    validations: BTreeMap<String, Vec<usize>>,
}

impl Dumper {
    fn new(dir: PathBuf) -> Self {
        fs::create_dir_all(&dir).expect("create out dir");
        Self {
            dir,
            counts: BTreeMap::new(),
            validations: BTreeMap::new(),
        }
    }

    fn dump(&mut self, name: &str, polys: &[ExPolygon]) {
        let json = serde_json::to_string(polys).expect("serialize");
        fs::write(self.dir.join(format!("{name}.json")), &json).expect("write dump");
        let mut holes = 0usize;
        let mut points = 0usize;
        for e in polys {
            holes += e.holes.len();
            points += e.contour.points.len();
            for h in &e.holes {
                points += h.points.len();
            }
        }
        self.counts
            .insert(name.to_string(), (polys.len(), holes, points));
    }

    fn validate(&mut self, name: &str, poly: &ExPolygon) {
        let failing = match validate_polygon_simplicity(poly) {
            Ok(()) => Vec::new(),
            Err(e) => e.contour_indices,
        };
        self.validations.insert(name.to_string(), failing);
    }

    /// Dump an open-path (polyline) result — the `clip_polylines` output shape.
    fn dump_polylines(&mut self, name: &str, polylines: &[Vec<Point2>]) {
        let json = serde_json::to_string(polylines).expect("serialize polylines");
        fs::write(self.dir.join(format!("{name}.json")), &json).expect("write dump");
        let points: usize = polylines.iter().map(|p| p.len()).sum();
        self.counts
            .insert(name.to_string(), (polylines.len(), 0, points));
    }

    fn finish(self) {
        let counts = serde_json::to_string_pretty(&self.counts).expect("serialize counts");
        fs::write(self.dir.join("_counts.json"), counts).expect("write counts");
        let vals = serde_json::to_string_pretty(&self.validations).expect("serialize validations");
        fs::write(self.dir.join("_validations.json"), vals).expect("write validations");
    }
}

fn dump_grids(d: &mut Dumper) {
    for n in [4i64, 8, 16] {
        let subj = square_grid(n);
        let p = n * n;
        // Bench fixtures, exactly as `benches/polygon_ops.rs` builds them.
        d.dump(
            &format!("grid{p}__union_benchclip"),
            &union(&subj, &[square(50_000, 50_000, 1_000_000)]),
        );
        d.dump(
            &format!("grid{p}__intersection_benchclip"),
            &intersection(&subj, &[square(50_000, 50_000, 1_500_000)]),
        );
        d.dump(
            &format!("grid{p}__difference_benchclip"),
            &difference(&subj, &[square(50_000, 50_000, 600_000)]),
        );
        d.dump(
            &format!("grid{p}__offset_bench"),
            &offset(&subj, 0.4, OffsetJoinType::Miter, 0.0),
        );
        // Additional join/sign coverage over the same grids.
        d.dump(&format!("grid{p}__union_ex"), &union_ex(&subj));
        d.dump(
            &format!("grid{p}__xor"),
            &xor(&subj, &[square(50_000, 50_000, 1_000_000)]),
        );
        d.dump(
            &format!("grid{p}__offset_round"),
            &offset(&subj, 0.4, OffsetJoinType::Round, 0.05),
        );
        d.dump(
            &format!("grid{p}__offset_square_neg"),
            &offset(&subj, -0.4, OffsetJoinType::Square, 0.0),
        );
        d.dump(
            &format!("grid{p}__offset2_ex"),
            &offset2_ex(&subj, -0.2, 0.2, OffsetJoinType::Miter, 3.0),
        );
        d.dump(
            &format!("grid{p}__opening_miter"),
            &opening(&subj, 0.1, OffsetJoinType::Miter),
        );
        d.dump(
            &format!("grid{p}__closing_ex_miter"),
            &closing_ex(&subj, 0.1, OffsetJoinType::Miter),
        );
    }
}

fn dump_hole_geometry(d: &mut Dumper) {
    let ann = annulus();
    let shifted = square(10_000, 10_000, 20_000);
    d.dump("annulus__union_ex", &union_ex(std::slice::from_ref(&ann)));
    d.dump(
        "annulus__difference_shifted",
        &difference(&[ann.clone()], &[shifted.clone()]),
    );
    d.dump(
        "annulus__intersection_shifted",
        &intersection(&[ann.clone()], &[shifted]),
    );
    d.dump(
        "annulus__offset_grow",
        &offset(&[ann.clone()], 0.1, OffsetJoinType::Round, 0.0),
    );
    d.dump(
        "annulus__offset_erode",
        &offset(&[ann.clone()], -0.1, OffsetJoinType::Miter, 0.0),
    );
    d.dump(
        "annulus__offset2_ex",
        &offset2_ex(
            std::slice::from_ref(&ann),
            -0.1,
            0.1,
            OffsetJoinType::Miter,
            3.0,
        ),
    );
    d.dump(
        "annulus__opening_miter",
        &opening(std::slice::from_ref(&ann), 0.1, OffsetJoinType::Miter),
    );
    d.dump(
        "annulus__closing_ex_miter",
        &closing_ex(std::slice::from_ref(&ann), 0.1, OffsetJoinType::Miter),
    );
    d.validate("annulus", &ann);

    let bt = bowtie();
    d.dump(
        "bowtie__union_ex_self",
        &union_ex(std::slice::from_ref(&bt)),
    );
    d.validate("bowtie", &bt);

    // Overlapping solid + holed solid: exercises owner/split resolution on
    // polygons whose holes overlap another contour.
    let solid = square(5_000, 5_000, 30_000);
    d.dump(
        "hole_overlap__union",
        &union(&[ann.clone()], &[solid.clone()]),
    );
    d.dump(
        "hole_overlap__difference",
        &difference(&[ann.clone()], &[solid.clone()]),
    );
    d.dump(
        "hole_overlap__intersection",
        &intersection(&[ann], &[solid]),
    );
}

fn dump_wedge(d: &mut Dumper, stl: &std::path::Path) {
    let model = slicer_model_io::load_model(stl).expect("load wedge stl");
    let obj = &model.objects[0];
    let mesh = &obj.mesh;
    let zmin = mesh
        .vertices
        .iter()
        .map(|v| v.z)
        .fold(f32::INFINITY, f32::min);
    let zmax = mesh
        .vertices
        .iter()
        .map(|v| v.z)
        .fold(f32::NEG_INFINITY, f32::max);
    let layers_n = 50usize;
    let zs: Vec<f32> = (0..layers_n)
        .map(|i| zmin + (zmax - zmin) * ((i as f32) + 0.5) / (layers_n as f32))
        .collect();
    let layers = slicer_core::triangle_mesh_slicer::slice_mesh_ex(mesh, &zs);

    let mut all: Vec<ExPolygon> = Vec::new();
    for (i, layer) in layers.iter().enumerate() {
        if layer.is_empty() {
            continue;
        }
        d.dump(&format!("wedge__layer{i:02}_raw"), layer);
        d.dump(
            &format!("wedge__layer{i:02}_offset2_ex"),
            &offset2_ex(layer, -0.2, 0.2, OffsetJoinType::Miter, 3.0),
        );
        d.dump(
            &format!("wedge__layer{i:02}_offset_erode"),
            &offset(layer, -0.2, OffsetJoinType::Miter, 0.0125),
        );
        // Open-path clipping (the `clip_polylines` entry point: lightning
        // infill and infill-linker route through it): hatch a layer with a
        // deterministic raster and clip it back to the layer.
        let hatch = slicer_core::polygon_ops::hatch_areas(layer, 0.5, 45.0);
        let polylines: Vec<Vec<Point2>> = hatch.iter().map(|l| vec![l.start, l.end]).collect();
        let clipped = clip_polylines(&polylines, layer);
        d.dump_polylines(&format!("wedge__layer{i:02}_clip_polylines"), &clipped);
        all.extend(layer.iter().cloned());
    }
    // The whole-slice union (the shape of the removed `region_needs_support`
    // call site named in DEV-173) — the largest single boolean input here.
    d.dump("wedge__slice_all_union_ex", &union_ex(&all));
}

/// Slice a real model and exercise its layer polygons through the clipper
/// entry points the production path uses — the closest thing to a
/// "real-model G-code deltas" proxy that stays a unit-level comparison.
fn dump_model(d: &mut Dumper, stl: &std::path::Path, tag: &str) {
    let model = slicer_model_io::load_model(stl).expect("load model");
    let obj = &model.objects[0];
    let mesh = &obj.mesh;
    let zmin = mesh
        .vertices
        .iter()
        .map(|v| v.z)
        .fold(f32::INFINITY, f32::min);
    let zmax = mesh
        .vertices
        .iter()
        .map(|v| v.z)
        .fold(f32::NEG_INFINITY, f32::max);
    // A sample of layers spread across the model (slicing every benchy layer
    // would be minutes per run; 24 spread layers exercise the real contours).
    let layers_n = 24usize;
    let zs: Vec<f32> = (0..layers_n)
        .map(|i| zmin + (zmax - zmin) * ((i as f32) + 0.5) / (layers_n as f32))
        .collect();
    let layers = slicer_core::triangle_mesh_slicer::slice_mesh_ex(mesh, &zs);

    let mut all: Vec<ExPolygon> = Vec::new();
    let mut dumped = 0usize;
    for (i, layer) in layers.iter().enumerate() {
        if layer.is_empty() {
            continue;
        }
        d.dump(&format!("{tag}__layer{i:02}_raw"), layer);
        d.dump(
            &format!("{tag}__layer{i:02}_offset2_ex"),
            &offset2_ex(layer, -0.2, 0.2, OffsetJoinType::Miter, 3.0),
        );
        d.dump(
            &format!("{tag}__layer{i:02}_opening"),
            &opening(layer, 0.05, OffsetJoinType::Miter),
        );
        d.dump(
            &format!("{tag}__layer{i:02}_closing"),
            &closing_ex(layer, 0.05, OffsetJoinType::Miter),
        );
        let hatch = slicer_core::polygon_ops::hatch_areas(layer, 0.5, 45.0);
        let polylines: Vec<Vec<Point2>> = hatch.iter().map(|l| vec![l.start, l.end]).collect();
        let clipped = clip_polylines(&polylines, layer);
        d.dump_polylines(&format!("{tag}__layer{i:02}_clip_polylines"), &clipped);
        all.extend(layer.iter().cloned());
        dumped += 1;
    }
    eprintln!(
        "{tag}: {dumped} non-empty layers, {} polygons accumulated",
        all.len()
    );
    d.dump(&format!("{tag}__slice_all_union_ex"), &union_ex(&all));
}

fn main() {
    let mut out: Option<PathBuf> = None;
    let mut stl: Option<PathBuf> = None;
    let mut model: Option<PathBuf> = None;
    let mut model_tag = String::from("model");
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--out" => out = args.next().map(PathBuf::from),
            "--stl" => stl = args.next().map(PathBuf::from),
            "--model" => model = args.next().map(PathBuf::from),
            "--tag" => model_tag = args.next().unwrap_or(model_tag),
            other => panic!("unknown arg {other}"),
        }
    }
    let out = out.expect("--out required");
    let stl = stl.expect("--stl required");

    let mut d = Dumper::new(out);
    dump_grids(&mut d);
    dump_hole_geometry(&mut d);
    dump_wedge(&mut d, &stl);
    if let Some(m) = &model {
        dump_model(&mut d, m, &model_tag);
    }
    eprintln!("dumps: {} files", d.counts.len());
    d.finish();
}
