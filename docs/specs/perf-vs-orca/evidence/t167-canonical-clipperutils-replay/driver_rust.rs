//! t167: PnP current Rust Clipper2 driver (in-tree vendor/clipper2-rust + polygon_ops wrapper + mimalloc).
use clipper2_rust::{
    inflate_paths_64, ClipType, Clipper64, ClipperOffset, EndType, FillRule, JoinType, Paths64,
    Point64, PolyTree64,
};
use std::{collections::BTreeMap, time::Instant};

#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

struct Tokens {
    values: Vec<String>,
    pos: usize,
}
impl Tokens {
    fn next(&mut self) -> String {
        let value = self.values.get(self.pos).expect("truncated corpus").clone();
        self.pos += 1;
        value
    }
    fn num<T: std::str::FromStr>(&mut self) -> T
    where
        T::Err: std::fmt::Debug,
    {
        self.next().parse().expect("invalid integer")
    }
    fn paths(&mut self) -> Paths64 {
        let mut paths = Vec::new();
        for _ in 0..self.num::<usize>() {
            for _ in 0..self.num::<usize>() {
                let mut ring = Vec::new();
                for _ in 0..self.num::<usize>() {
                    ring.push(Point64::new(self.num(), self.num()));
                }
                paths.push(ring);
            }
        }
        paths
    }
}

struct Case {
    site: String,
    layer: i64,
    opcode: u64,
    args: BTreeMap<u64, u64>,
    subj: Paths64,
    clip: Paths64,
}

fn signed_area(ring: &[Point64]) -> f64 {
    if ring.len() < 3 {
        return 0.0;
    }
    let mut area: i128 = 0;
    let n = ring.len();
    for i in 0..n {
        let j = (i + 1) % n;
        area += i128::from(ring[i].x) * i128::from(ring[j].y)
            - i128::from(ring[j].x) * i128::from(ring[i].y);
    }
    (area as f64) * 0.5
}

fn oriented_ring(path: &[Point64], want_ccw: bool) -> Vec<Point64> {
    let mut pts = path.to_vec();
    let is_ccw = signed_area(&pts) > 0.0;
    if is_ccw != want_ccw {
        pts.reverse();
    }
    pts
}

fn collect_expolygon_from_tree(tree: &PolyTree64, node: usize, out: &mut Vec<Paths64>) {
    let contour = oriented_ring(tree.nodes[node].polygon(), true);
    let mut poly = Vec::new();
    let mut holes = Vec::new();
    for &hole_idx in tree.nodes[node].children() {
        let hole = oriented_ring(tree.nodes[hole_idx].polygon(), false);
        if hole.len() >= 3 {
            holes.push(hole);
        }
        for &island_idx in tree.nodes[hole_idx].children() {
            collect_expolygon_from_tree(tree, island_idx, out);
        }
    }
    if contour.len() >= 3 {
        poly.push(contour);
        poly.extend(holes);
        out.push(poly);
    }
}

fn expolygons_from_tree(tree: &PolyTree64) -> Vec<Paths64> {
    let mut out = Vec::new();
    for &node in tree.nodes[0].children() {
        collect_expolygon_from_tree(tree, node, &mut out);
    }
    out
}

impl Case {
    fn arg(&self, id: u64) -> u64 {
        *self.args.get(&id).expect("required argument missing")
    }
    fn flag(&self, id: u64, default: u64) -> bool {
        let v = *self.args.get(&id).unwrap_or(&default);
        assert!(v <= 1, "flag must be 0 or 1");
        v == 1
    }
    fn validate(&self) {
        assert!(self.opcode <= 5, "unknown opcode");
        for id in self.args.keys() {
            assert!(
                if self.opcode == 0 || self.opcode == 5 {
                    (1..=5).contains(id) || *id == 7 || *id == 8
                } else {
                    *id == 10 || *id == 7 || *id == 8
                },
                "unexpected argument ID"
            );
        }
        self.flag(7, u64::from(self.opcode >= 1 && self.opcode <= 4));
        self.flag(8, 0);
        if self.opcode == 0 || self.opcode == 5 {
            assert!(self.arg(2) <= 3 && self.arg(3) <= 4, "invalid offset enum");
            for id in [1, 4] {
                assert!(f64::from_bits(self.arg(id)).is_finite());
            }
            assert!(f64::from_bits(*self.args.get(&5).unwrap_or(&0)).is_finite());
        } else {
            assert!(self.arg(10) <= 3, "invalid fill rule");
        }
    }
    fn join(&self) -> JoinType {
        [
            JoinType::Square,
            JoinType::Bevel,
            JoinType::Round,
            JoinType::Miter,
        ][self.arg(2) as usize]
    }
    fn end(&self) -> EndType {
        [
            EndType::Polygon,
            EndType::Joined,
            EndType::Butt,
            EndType::Square,
            EndType::Round,
        ][self.arg(3) as usize]
    }
    /// Matches `crates/slicer-core/src/polygon_ops.rs` (`inflate_once`, `boolean_op_tree_64`, `opening_ex`).
    fn run(&self) -> Vec<Paths64> {
        let mut tree = PolyTree64::new();
        if self.opcode == 0 {
            let delta = f64::from_bits(self.arg(1));
            if delta == 0.0 || self.subj.is_empty() {
                return Vec::new();
            }
            let mut engine = ClipperOffset::new(
                f64::from_bits(self.arg(4)),
                f64::from_bits(*self.args.get(&5).unwrap_or(&0)),
                self.flag(7, 0),
                self.flag(8, 0),
            );
            engine.add_paths(&self.subj, self.join(), self.end());
            engine.execute_tree(delta, &mut tree);
            assert_eq!(engine.error_code(), 0, "offset engine failure");
            expolygons_from_tree(&tree)
        } else if self.opcode == 5 {
            let r = f64::from_bits(self.arg(1));
            if r <= 0.0 || self.subj.is_empty() {
                return Vec::new();
            }
            let miter = f64::from_bits(self.arg(4));
            let arc = f64::from_bits(*self.args.get(&5).unwrap_or(&0));
            let join = self.join();
            let end = self.end();
            let intermediate = inflate_paths_64(&self.subj, -r, join, end, miter, arc);
            if intermediate.is_empty() {
                return Vec::new();
            }
            let mut engine = ClipperOffset::new(miter, arc, false, false);
            engine.add_paths(&intermediate, join, end);
            engine.execute_tree(r, &mut tree);
            assert_eq!(engine.error_code(), 0, "opening pass2 engine failure");
            expolygons_from_tree(&tree)
        } else {
            let mut engine = Clipper64::new();
            engine.set_preserve_collinear(self.flag(7, 1));
            engine.set_reverse_solution(self.flag(8, 0));
            engine.add_subject(&self.subj);
            engine.add_clip(&self.clip);
            let op = [
                ClipType::NoClip,
                ClipType::Difference,
                ClipType::Union,
                ClipType::Intersection,
                ClipType::Xor,
            ][self.opcode as usize];
            let fill = [
                FillRule::EvenOdd,
                FillRule::NonZero,
                FillRule::Positive,
                FillRule::Negative,
            ][self.arg(10) as usize];
            assert!(
                engine.execute_tree(op, fill, &mut tree, &mut Vec::new()),
                "boolean engine failure"
            );
            expolygons_from_tree(&tree)
        }
    }
}

fn summary(polys: &[Paths64]) -> (usize, usize, usize, f64, u64) {
    let mut hash = 14695981039346656037u64;
    let mut rings = 0usize;
    let mut verts = 0usize;
    let mut area = 0.0f64;
    for poly in polys {
        for (ri, ring) in poly.iter().enumerate() {
            rings += 1;
            let a = signed_area(ring).abs();
            if ri == 0 {
                area += a;
            } else {
                area -= a;
            }
            for point in ring {
                verts += 1;
                for value in [point.x, point.y] {
                    for byte in (value as u64).to_le_bytes() {
                        hash = (hash ^ u64::from(byte)).wrapping_mul(1099511628211);
                    }
                }
            }
        }
    }
    (polys.len(), rings, verts, area, hash)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    assert!(!args.is_empty(), "usage: driver corpus.cases ...");
    for file in args {
        let text = std::fs::read_to_string(&file).expect("read corpus");
        let values = text
            .lines()
            .flat_map(|l| {
                l.split('#')
                    .next()
                    .unwrap()
                    .split_whitespace()
                    .map(str::to_owned)
            })
            .collect();
        let mut t = Tokens { values, pos: 0 };
        let mut index = 0;
        while t.pos < t.values.len() {
            assert_eq!(t.next(), "case");
            let mut c = Case {
                site: t.next(),
                layer: t.num(),
                opcode: t.num(),
                args: BTreeMap::new(),
                subj: Vec::new(),
                clip: Vec::new(),
            };
            if t.values
                .get(t.pos)
                .is_some_and(|s| s.parse::<u64>().is_ok())
            {
                let _: u64 = t.num();
            }
            let mut subj = false;
            let mut clip = false;
            loop {
                match t.next().as_str() {
                    "arg" => {
                        let id = t.num();
                        let value = t.num();
                        assert!(c.args.insert(id, value).is_none(), "duplicate arg");
                    }
                    "subj" => {
                        assert!(!subj);
                        subj = true;
                        c.subj = t.paths();
                    }
                    "clip" => {
                        assert!(!clip);
                        clip = true;
                        c.clip = t.paths();
                    }
                    "end" => break,
                    _ => panic!("unknown record"),
                }
            }
            let expect_clip = c.opcode >= 1 && c.opcode <= 4;
            assert!(subj && clip == expect_clip, "incorrect geometry records");
            c.validate();
            let in_verts: usize = c.subj.iter().map(Vec::len).sum::<usize>()
                + c.clip.iter().map(Vec::len).sum::<usize>();
            let expected = summary(&c.run());
            let mut times = Vec::new();
            for _ in 0..5 {
                let start = Instant::now();
                let output = c.run();
                times.push(start.elapsed().as_nanos() as u64);
                let got = summary(&output);
                assert_eq!(
                    (got.0, got.1, got.2, got.4),
                    (expected.0, expected.1, expected.2, expected.4),
                    "nondeterministic output"
                );
                std::hint::black_box(&output);
            }
            println!(
                "{}",
                serde_json::json!({
                    "file": file,
                    "case_index": index,
                    "site": c.site,
                    "layer": c.layer,
                    "opcode": c.opcode,
                    "in_verts": in_verts,
                    "repeat_times_ns": times,
                    "out_paths": expected.0,
                    "out_rings": expected.1,
                    "out_verts": expected.2,
                    "out_area_u2": expected.3,
                    "out_hash": format!("{:016x}", expected.4)
                })
            );
            index += 1;
        }
    }
}
